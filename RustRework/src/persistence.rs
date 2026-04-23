use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::fs;
use worldgen_core::{
    create_local_summary, create_macro_summary, create_world_profile_summary, to_text_hash,
    ChunkStore, LocalChunk, LocalChunkKey, MacroChunk, MacroChunkKey, PhaseSeedDeriver,
    Result, WorldGenError, WorldProfile,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoredChunkKind {
    WorldProfile = 1,
    MacroChunk = 2,
    LocalChunk = 3,
}

impl StoredChunkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorldProfile => "WorldProfile",
            Self::MacroChunk => "MacroChunk",
            Self::LocalChunk => "LocalChunk",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkCompressionMode {
    None = 0,
    Lz4 = 1,
}

impl ChunkCompressionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Lz4 => "Lz4",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileChunkStoreOptions {
    pub root_path: PathBuf,
    pub schema_version: i32,
    pub compression_mode: ChunkCompressionMode,
}

impl FileChunkStoreOptions {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            schema_version: 2,
            compression_mode: ChunkCompressionMode::Lz4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkMetadataSidecar {
    pub schema_version: i32,
    pub chunk_kind: String,
    pub key: String,
    pub compression: String,
    pub content_hash: String,
    pub canonical_hash: String,
    pub is_mutated: bool,
    pub is_uniform: bool,
    pub persisted_at_utc: String,
    pub summary: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct FileChunkStore {
    options: FileChunkStoreOptions,
}

impl FileChunkStore {
    pub async fn new(root_path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_options(FileChunkStoreOptions::new(root_path)).await
    }

    pub async fn with_options(options: FileChunkStoreOptions) -> Result<Self> {
        fs::create_dir_all(&options.root_path).await?;
        fs::create_dir_all(options.root_path.join("world")).await?;
        fs::create_dir_all(options.root_path.join("macro")).await?;
        fs::create_dir_all(options.root_path.join("local")).await?;
        Ok(Self { options })
    }

    pub fn root_path(&self) -> &Path {
        &self.options.root_path
    }

    async fn write_payload<T: Serialize>(
        &self,
        expected_kind: StoredChunkKind,
        base_path: &Path,
        payload: &T,
        sidecar: &ChunkMetadataSidecar,
    ) -> Result<()> {
        if let Some(parent) = base_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let encoded = rmp_serde::to_vec(payload)?;
        let encoded = self.compress(&encoded)?;
        let header = self.create_header(expected_kind, encoded.len() as u32, self.options.compression_mode);
        let mut output = Vec::with_capacity(header.len() + encoded.len());
        output.extend_from_slice(&header);
        output.extend_from_slice(&encoded);
        fs::write(self.binary_path(base_path), output).await?;
        fs::write(self.json_path(base_path), serde_json::to_string_pretty(sidecar)?).await?;
        Ok(())
    }

    async fn read_payload<T: DeserializeOwned>(
        &self,
        expected_kind: StoredChunkKind,
        base_path: &Path,
    ) -> Result<T> {
        let bytes = fs::read(self.binary_path(base_path)).await?;
        let (payload_offset, payload_length, compression_mode) =
            self.validate_header(&bytes, expected_kind)?;
        let payload = &bytes[payload_offset..payload_offset + payload_length];
        let payload = self.decompress(payload, compression_mode)?;
        Ok(rmp_serde::from_slice(&payload)?)
    }

    fn compress(&self, payload: &[u8]) -> Result<Vec<u8>> {
        Ok(match self.options.compression_mode {
            ChunkCompressionMode::None => payload.to_vec(),
            ChunkCompressionMode::Lz4 => lz4_flex::compress_prepend_size(payload),
        })
    }

    fn decompress(
        &self,
        payload: &[u8],
        compression_mode: ChunkCompressionMode,
    ) -> Result<Vec<u8>> {
        match compression_mode {
            ChunkCompressionMode::None => Ok(payload.to_vec()),
            ChunkCompressionMode::Lz4 => lz4_flex::decompress_size_prepended(payload)
                .map_err(|error| WorldGenError::invalid_data(format!("LZ4 decompression failed: {error}"))),
        }
    }

    fn create_header(
        &self,
        kind: StoredChunkKind,
        payload_length: u32,
        compression_mode: ChunkCompressionMode,
    ) -> [u8; 14] {
        let mut header = [0u8; 14];
        header[0..4].copy_from_slice(b"WGEN");
        header[4..8].copy_from_slice(&self.options.schema_version.to_le_bytes());
        header[8] = kind as u8;
        header[9] = compression_mode as u8;
        header[10..14].copy_from_slice(&payload_length.to_le_bytes());
        header
    }

    fn validate_header(
        &self,
        data: &[u8],
        expected_kind: StoredChunkKind,
    ) -> Result<(usize, usize, ChunkCompressionMode)> {
        let payload_offset = 14usize;
        if data.len() < payload_offset {
            return Err(WorldGenError::invalid_data(
                "Chunk payload is truncated before the header completed.",
            ));
        }

        if &data[0..4] != b"WGEN" {
            return Err(WorldGenError::invalid_data(
                "Chunk payload magic header does not match WorldGen format.",
            ));
        }

        let schema_version = i32::from_le_bytes(data[4..8].try_into().expect("schema version bytes"));
        if schema_version != self.options.schema_version {
            return Err(WorldGenError::UnsupportedSchemaVersion {
                actual: schema_version,
                expected: self.options.schema_version,
            });
        }

        let kind = match data[8] {
            1 => StoredChunkKind::WorldProfile,
            2 => StoredChunkKind::MacroChunk,
            3 => StoredChunkKind::LocalChunk,
            other => {
                return Err(WorldGenError::invalid_data(format!(
                    "Chunk payload kind {other} does not match expected {}.",
                    expected_kind.as_str()
                )))
            }
        };

        if kind != expected_kind {
            return Err(WorldGenError::invalid_data(format!(
                "Chunk payload kind {} does not match expected {}.",
                kind.as_str(),
                expected_kind.as_str()
            )));
        }

        let compression_mode = match data[9] {
            0 => ChunkCompressionMode::None,
            1 => ChunkCompressionMode::Lz4,
            other => {
                return Err(WorldGenError::invalid_data(format!(
                    "Unsupported compression mode {other}."
                )))
            }
        };

        let payload_length =
            u32::from_le_bytes(data[10..14].try_into().expect("payload length bytes")) as usize;
        if data.len() != payload_offset + payload_length {
            return Err(WorldGenError::invalid_data(
                "Chunk payload length does not match the header.",
            ));
        }

        Ok((payload_offset, payload_length, compression_mode))
    }

    fn world_profile_base_path(&self) -> PathBuf {
        self.options.root_path.join("world").join("world-profile")
    }

    fn macro_base_path(&self, key: MacroChunkKey) -> PathBuf {
        self.options
            .root_path
            .join("macro")
            .join(format!("{}_{}", key.x, key.y))
    }

    fn local_base_path(&self, key: &LocalChunkKey) -> PathBuf {
        self.options
            .root_path
            .join("local")
            .join(format!("{}_{}", key.macro_key.x, key.macro_key.y))
            .join(format!("{}_{}_{}", key.x, key.y, key.z))
    }

    fn binary_path(&self, base_path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.bin", base_path.display()))
    }

    fn json_path(&self, base_path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.json", base_path.display()))
    }
}

#[async_trait]
impl ChunkStore for FileChunkStore {
    async fn save_world_profile(&self, profile: &WorldProfile) -> Result<()> {
        let summary = create_world_profile_summary(profile);
        let canonical_hash = PhaseSeedDeriver::hash_text(&format!(
            "{}:{}:{}:{}:{}",
            summary.world_id.as_simple(),
            summary.world_seed,
            summary.generation_version,
            summary.active_region_id,
            summary.content_hash,
        ));
        let sidecar = ChunkMetadataSidecar {
            schema_version: self.options.schema_version,
            chunk_kind: StoredChunkKind::WorldProfile.as_str().to_string(),
            key: summary.world_id.as_simple().to_string(),
            compression: self.options.compression_mode.as_str().to_string(),
            content_hash: summary.content_hash.clone(),
            canonical_hash: to_text_hash(canonical_hash),
            is_mutated: false,
            is_uniform: false,
            persisted_at_utc: worldgen_core::utc_now_string(),
            summary: BTreeMap::from([
                ("WorldId".to_string(), summary.world_id.as_simple().to_string()),
                ("WorldSeed".to_string(), summary.world_seed.to_string()),
                (
                    "GenerationVersion".to_string(),
                    summary.generation_version.to_string(),
                ),
                ("ActiveRegionId".to_string(), summary.active_region_id.clone()),
                (
                    "Dimensions".to_string(),
                    format!(
                        "macro={}x{};local={}x{}x{};z={}..{}",
                        summary.dimensions.macro_width,
                        summary.dimensions.macro_height,
                        summary.dimensions.local_width,
                        summary.dimensions.local_height,
                        summary.dimensions.local_depth,
                        summary.z_bounds.min,
                        summary.z_bounds.max,
                    ),
                ),
            ]),
        };
        self.write_payload(
            StoredChunkKind::WorldProfile,
            &self.world_profile_base_path(),
            profile,
            &sidecar,
        )
        .await
    }

    async fn load_world_profile(&self) -> Result<Option<WorldProfile>> {
        let binary_path = self.binary_path(&self.world_profile_base_path());
        if !binary_path.exists() {
            return Ok(None);
        }

        self.read_payload(StoredChunkKind::WorldProfile, &self.world_profile_base_path())
            .await
            .map(Some)
    }

    async fn save_macro_chunk(&self, chunk: &MacroChunk) -> Result<()> {
        let summary = create_macro_summary(chunk);
        let surface_histogram = summary
            .surface_landcover_histogram
            .iter()
            .map(|(kind, count)| format!("{}:{}", kind.as_str(), count))
            .collect::<Vec<_>>()
            .join(", ");
        let sidecar = ChunkMetadataSidecar {
            schema_version: self.options.schema_version,
            chunk_kind: StoredChunkKind::MacroChunk.as_str().to_string(),
            key: chunk.key.to_string(),
            compression: self.options.compression_mode.as_str().to_string(),
            content_hash: chunk.provenance.content_hash.clone(),
            canonical_hash: to_text_hash(summary.canonical_hash),
            is_mutated: false,
            is_uniform: false,
            persisted_at_utc: worldgen_core::utc_now_string(),
            summary: BTreeMap::from([
                ("RegionId".to_string(), summary.region_id.clone()),
                ("SurfaceHistogram".to_string(), surface_histogram),
                (
                    "SettlementCount".to_string(),
                    summary.settlement_count.to_string(),
                ),
                ("SpecialCount".to_string(), summary.special_count.to_string()),
                (
                    "RoadConnections".to_string(),
                    format!(
                        "N:{} S:{} E:{} W:{}",
                        summary.connectivity.north_road_connections,
                        summary.connectivity.south_road_connections,
                        summary.connectivity.east_road_connections,
                        summary.connectivity.west_road_connections,
                    ),
                ),
                (
                    "WaterConnections".to_string(),
                    format!(
                        "N:{} S:{} E:{} W:{}",
                        summary.connectivity.north_water_connections,
                        summary.connectivity.south_water_connections,
                        summary.connectivity.east_water_connections,
                        summary.connectivity.west_water_connections,
                    ),
                ),
            ]),
        };
        self.write_payload(
            StoredChunkKind::MacroChunk,
            &self.macro_base_path(chunk.key),
            chunk,
            &sidecar,
        )
        .await
    }

    async fn load_macro_chunk(&self, key: MacroChunkKey) -> Result<Option<MacroChunk>> {
        let base = self.macro_base_path(key);
        let binary_path = self.binary_path(&base);
        if !binary_path.exists() {
            return Ok(None);
        }

        self.read_payload(StoredChunkKind::MacroChunk, &base)
            .await
            .map(Some)
    }

    async fn save_local_chunk(&self, chunk: &LocalChunk) -> Result<()> {
        let summary = create_local_summary(chunk);
        let material_histogram = summary
            .material_histogram
            .iter()
            .map(|(kind, count)| format!("{}:{}", kind.as_str(), count))
            .collect::<Vec<_>>()
            .join(", ");
        let sidecar = ChunkMetadataSidecar {
            schema_version: self.options.schema_version,
            chunk_kind: StoredChunkKind::LocalChunk.as_str().to_string(),
            key: chunk.key.to_string(),
            compression: self.options.compression_mode.as_str().to_string(),
            content_hash: chunk.provenance.content_hash.clone(),
            canonical_hash: to_text_hash(summary.canonical_hash),
            is_mutated: chunk.provenance.is_mutated,
            is_uniform: chunk.is_uniform,
            persisted_at_utc: worldgen_core::utc_now_string(),
            summary: BTreeMap::from([
                ("MacroChunk".to_string(), chunk.key.macro_key.to_string()),
                (
                    "LocalCell".to_string(),
                    format!("{},{},{}", chunk.key.x, chunk.key.y, chunk.key.z),
                ),
                ("Materials".to_string(), material_histogram),
                ("Props".to_string(), summary.prop_count.to_string()),
                ("Items".to_string(), summary.item_count.to_string()),
                ("Fields".to_string(), summary.field_count.to_string()),
                (
                    "MutationCount".to_string(),
                    summary.mutation_count.to_string(),
                ),
                (
                    "SourceMacroCanonicalHash".to_string(),
                    to_text_hash(summary.source_macro_canonical_hash),
                ),
            ]),
        };
        self.write_payload(
            StoredChunkKind::LocalChunk,
            &self.local_base_path(&chunk.key),
            chunk,
            &sidecar,
        )
        .await
    }

    async fn load_local_chunk(&self, key: &LocalChunkKey) -> Result<Option<LocalChunk>> {
        let base = self.local_base_path(key);
        let binary_path = self.binary_path(&base);
        if !binary_path.exists() {
            return Ok(None);
        }

        self.read_payload(StoredChunkKind::LocalChunk, &base)
            .await
            .map(Some)
    }

    async fn delete_local_chunk(&self, key: &LocalChunkKey) -> Result<()> {
        let base = self.local_base_path(key);
        let binary_path = self.binary_path(&base);
        let json_path = self.json_path(&base);
        if binary_path.exists() {
            fs::remove_file(binary_path).await?;
        }
        if json_path.exists() {
            fs::remove_file(json_path).await?;
        }
        Ok(())
    }
}
