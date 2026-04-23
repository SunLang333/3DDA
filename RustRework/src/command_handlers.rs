use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use worldgen_core::{
    create_local_summary, create_macro_summary, create_world_profile_summary, to_text_hash,
    ChunkManager, ItemKind, LocalChunkKey, LocalRealizer, MacroChunkKey,
    MacroGenerationPipeline, Result, WorldBootstrap, WorldGenError, WorldProfile,
};
use worldgen_persistence::FileChunkStore;

#[derive(Debug, Default, Clone)]
pub struct CommandHandlers;

impl CommandHandlers {
    pub async fn generate_world(
        &self,
        seed: u64,
        output_path: String,
        radius: i32,
        output: &mut impl Write,
        ct: &CancellationToken,
    ) -> Result<i32> {
        let resolved_output_path = resolve_path(&output_path, false);
        fs::create_dir_all(&resolved_output_path)?;
        let profile = WorldBootstrap::create_default_profile(seed);
        let manager = create_manager(&resolved_output_path, profile.clone()).await?;
        manager.persist_world_profile().await?;

        for y in -radius..=radius {
            for x in -radius..=radius {
                check_cancelled(ct)?;
                let chunk = manager
                    .get_or_create_macro_chunk(MacroChunkKey { x, y }, ct)
                    .await?;
                writeln!(
                    output,
                    "Generated macro chunk {} ({})",
                    chunk.key,
                    to_text_hash(chunk.provenance.canonical_hash)
                )?;
            }
        }

        let macro_chunk = manager
            .get_or_create_macro_chunk(MacroChunkKey { x: 0, y: 0 }, ct)
            .await?;
        let macro_summary = create_macro_summary(&macro_chunk);
        let local_key = LocalChunkKey::from_world_coordinates(0, 0, 0, profile.dimensions);
        let local_chunk = manager.get_or_create_local_chunk(&local_key, ct).await?;
        let local_summary = create_local_summary(&local_chunk.lock());
        fs::write(
            resolved_output_path.join("world-summary.json"),
            serde_json::to_string_pretty(&create_world_profile_summary(&profile))?,
        )?;
        fs::write(
            resolved_output_path.join("macro-0_0-summary.json"),
            serde_json::to_string_pretty(&macro_summary)?,
        )?;
        fs::write(
            resolved_output_path.join("local-0_0_0-summary.json"),
            serde_json::to_string_pretty(&local_summary)?,
        )?;
        writeln!(output, "Wrote summaries to {}", resolved_output_path.display())?;
        Ok(0)
    }

    pub async fn dump_macro_chunk(
        &self,
        world_path: String,
        x: i32,
        y: i32,
        export_path: Option<String>,
        output: &mut impl Write,
        ct: &CancellationToken,
    ) -> Result<i32> {
        let (_profile, manager) = load_manager(&world_path).await?;
        let chunk = manager
            .get_or_create_macro_chunk(MacroChunkKey { x, y }, ct)
            .await?;
        let payload = serde_json::to_string_pretty(&create_macro_summary(&chunk))?;
        if let Some(export_path) = export_path.filter(|value| !value.trim().is_empty()) {
            let resolved = resolve_path(&export_path, false);
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(resolved, &payload)?;
        }
        writeln!(output, "{payload}")?;
        Ok(0)
    }

    pub async fn dump_local_chunk(
        &self,
        world_path: String,
        x: i32,
        y: i32,
        z: i32,
        full: bool,
        export_path: Option<String>,
        output: &mut impl Write,
        ct: &CancellationToken,
    ) -> Result<i32> {
        let (profile, manager) = load_manager(&world_path).await?;
        let local_key = LocalChunkKey::from_world_coordinates(x, y, z, profile.dimensions);
        let chunk = manager.get_or_create_local_chunk(&local_key, ct).await?;
        let payload = {
            let chunk = chunk.lock();
            if full {
                serde_json::to_string_pretty(&*chunk)?
            } else {
                serde_json::to_string_pretty(&create_local_summary(&chunk))?
            }
        };
        if let Some(export_path) = export_path.filter(|value| !value.trim().is_empty()) {
            let resolved = resolve_path(&export_path, false);
            if let Some(parent) = resolved.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(resolved, &payload)?;
        }
        writeln!(output, "{payload}")?;
        Ok(0)
    }

    pub async fn validate_determinism(
        &self,
        world_path: String,
        start_x: i32,
        start_y: i32,
        width: i32,
        height: i32,
        output: &mut impl Write,
        ct: &CancellationToken,
    ) -> Result<i32> {
        let (profile, manager) = load_manager(&world_path).await?;
        let generator = MacroGenerationPipeline;
        let mut mismatches = Vec::new();
        for y in start_y..start_y + height {
            for x in start_x..start_x + width {
                check_cancelled(ct)?;
                let key = MacroChunkKey { x, y };
                let generated_a = generator.generate(key, &profile, ct)?;
                let generated_b = generator.generate(key, &profile, ct)?;
                let persisted = manager.get_or_create_macro_chunk(key, ct).await?;
                let bytes_a = worldgen_core::serialize_macro_chunk_canonical(&generated_a);
                let bytes_b = worldgen_core::serialize_macro_chunk_canonical(&generated_b);
                let bytes_persisted = worldgen_core::serialize_macro_chunk_canonical(&persisted);
                let hashes_match = generated_a.provenance.canonical_hash
                    == generated_b.provenance.canonical_hash
                    && generated_a.provenance.canonical_hash == persisted.provenance.canonical_hash;
                if !(bytes_a == bytes_b && bytes_a == bytes_persisted && hashes_match) {
                    mismatches.push(key.to_string());
                }
            }
        }

        if mismatches.is_empty() {
            writeln!(output, "Determinism validation passed.")?;
            Ok(0)
        } else {
            writeln!(
                output,
                "Determinism validation failed for: {}",
                mismatches.join(", ")
            )?;
            Ok(1)
        }
    }

    pub async fn count_items(
        &self,
        world_path: String,
        surface_only: bool,
        output: &mut impl Write,
        ct: &CancellationToken,
    ) -> Result<i32> {
        let resolved_world_path = resolve_path(&world_path, true);
        let (profile, manager) = load_manager(&resolved_world_path.to_string_lossy()).await?;
        let macro_dir = resolved_world_path.join("macro");
        if !macro_dir.exists() {
            writeln!(output, "No macro directory found.")?;
            return Ok(1);
        }

        let macro_files: Vec<_> = fs::read_dir(&macro_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("bin"))
            .collect();
        if macro_files.is_empty() {
            writeln!(output, "No macro chunks found.")?;
            return Ok(1);
        }

        let dimensions = profile.dimensions;
        let min_z = if surface_only { 0 } else { dimensions.min_z };
        let max_z = if surface_only { 0 } else { dimensions.max_z };
        let mut item_entries: BTreeMap<ItemKind, i64> = BTreeMap::new();
        let mut item_quantities: BTreeMap<ItemKind, i64> = BTreeMap::new();
        let mut total_entries = 0i64;
        let mut total_quantity = 0i64;

        for file in macro_files {
            let Some(stem) = file.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let parts: Vec<_> = stem.split('_').collect();
            if parts.len() != 2 {
                continue;
            }
            let Ok(mx) = parts[0].parse::<i32>() else {
                continue;
            };
            let Ok(my) = parts[1].parse::<i32>() else {
                continue;
            };
            let macro_key = MacroChunkKey { x: mx, y: my };
            for local_x in 0..dimensions.macro_width {
                for local_y in 0..dimensions.macro_height {
                    for z in min_z..=max_z {
                        check_cancelled(ct)?;
                        let local_key = LocalChunkKey {
                            macro_key,
                            x: local_x,
                            y: local_y,
                            z,
                        };
                        let chunk = manager.get_or_create_local_chunk(&local_key, ct).await?;
                        let chunk = chunk.lock();
                        for item in &chunk.items {
                            *item_entries.entry(item.type_).or_insert(0) += 1;
                            *item_quantities.entry(item.type_).or_insert(0) += i64::from(item.quantity);
                            total_entries += 1;
                            total_quantity += i64::from(item.quantity);
                        }
                    }
                }
            }
        }

        writeln!(output, "Total placed item entries: {total_entries}")?;
        writeln!(output, "Total item quantity: {total_quantity}")?;
        for (kind, entries) in item_entries {
            let quantity = item_quantities.get(&kind).copied().unwrap_or_default();
            writeln!(
                output,
                "  {}: entries={}, quantity={}",
                kind.as_str(),
                entries,
                quantity
            )?;
        }
        Ok(0)
    }
}

async fn create_manager(output_path: &Path, profile: WorldProfile) -> Result<ChunkManager> {
    let store = Arc::new(FileChunkStore::new(output_path.to_path_buf()).await?);
    let manager = ChunkManager::new(
        store,
        Arc::new(MacroGenerationPipeline),
        Arc::new(LocalRealizer),
        None,
    )
    .await?;
    manager.register_world_profile(profile);
    Ok(manager)
}

async fn load_manager(world_path: &str) -> Result<(WorldProfile, ChunkManager)> {
    let resolved_world_path = resolve_path(world_path, true);
    let store = Arc::new(FileChunkStore::new(resolved_world_path.clone()).await?);
    let manager = ChunkManager::new(
        store,
        Arc::new(MacroGenerationPipeline),
        Arc::new(LocalRealizer),
        None,
    )
    .await?;
    let Some(profile) = manager.load_registered_world_profile().await? else {
        return Err(WorldGenError::state(build_missing_world_message(&resolved_world_path)));
    };
    Ok((profile, manager))
}

fn resolve_path(path: &str, must_exist: bool) -> PathBuf {
    assert!(!path.trim().is_empty(), "path must not be empty");
    let input = PathBuf::from(path);
    if input.is_absolute() {
        return input;
    }

    let relative_segments: Vec<String> = path
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect();

    let anchored_workspace_candidate = find_ancestor_anchored_candidate(&relative_segments, path);
    if !must_exist {
        if let Some(candidate) = &anchored_workspace_candidate {
            return candidate.clone();
        }
    }

    let mut candidates = Vec::new();
    let mut best_candidate = None;
    let mut best_prefix_depth = -1isize;
    let mut current = Some(std::env::current_dir().expect("current directory must resolve"));
    while let Some(directory) = current {
        let candidate = directory.join(path);
        candidates.push(candidate.clone());
        if !must_exist {
            let prefix_depth = count_existing_prefix_segments(&directory, &relative_segments) as isize;
            if prefix_depth > best_prefix_depth {
                best_prefix_depth = prefix_depth;
                best_candidate = Some(candidate.clone());
            }
        }
        current = directory.parent().map(Path::to_path_buf);
    }

    if must_exist {
        if let Some(candidate) = anchored_workspace_candidate {
            if candidate.exists() {
                return candidate;
            }
        }
    }

    for candidate in &candidates {
        if must_exist && candidate.exists() {
            return candidate.clone();
        }
        if !must_exist {
            if let Some(parent) = candidate.parent() {
                if parent.exists() {
                    return candidate.clone();
                }
            }
        }
    }

    if !must_exist {
        if let Some(candidate) = best_candidate {
            return candidate;
        }
    }

    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(path))
}

fn build_missing_world_message(resolved_world_path: &Path) -> String {
    let available_worlds = find_nearby_worlds(resolved_world_path);
    if available_worlds.is_empty() {
        format!(
            "No world profile was found under {}. Run generate-world first.",
            resolved_world_path.display()
        )
    } else {
        format!(
            "No world profile was found under {}. Run generate-world first. Nearby world folders: {}",
            resolved_world_path.display(),
            available_worlds.join(", ")
        )
    }
}

fn find_nearby_worlds(resolved_world_path: &Path) -> Vec<String> {
    let mut result = Vec::new();
    let Some(world_parent) = resolved_world_path.parent() else {
        return result;
    };
    let parents_to_scan = [
        Some(world_parent.to_path_buf()),
        world_parent.parent().map(Path::to_path_buf),
    ];
    for scan_root in parents_to_scan.into_iter().flatten() {
        if !scan_root.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(scan_root) {
            for entry in entries.flatten() {
                let candidate = entry.path();
                if candidate.join("world").join("world-profile.bin").exists() {
                    result.push(candidate.display().to_string());
                }
            }
        }
    }
    result.sort();
    result.truncate(5);
    result
}

fn count_existing_prefix_segments(base_directory: &Path, relative_segments: &[String]) -> usize {
    let mut prefix_depth = 0usize;
    let mut current = base_directory.to_path_buf();
    for segment in relative_segments {
        current = current.join(segment);
        if !current.exists() {
            break;
        }
        prefix_depth += 1;
    }
    prefix_depth
}

fn find_ancestor_anchored_candidate(
    relative_segments: &[String],
    original_path: &str,
) -> Option<PathBuf> {
    let first_segment = relative_segments.first()?;
    let mut current = Some(std::env::current_dir().ok()?);
    while let Some(directory) = current {
        if directory.file_name().and_then(|name| name.to_str()) == Some(first_segment.as_str()) {
            if let Some(parent) = directory.parent() {
                return Some(parent.join(original_path));
            }
        }
        current = directory.parent().map(Path::to_path_buf);
    }
    None
}

fn check_cancelled(ct: &CancellationToken) -> Result<()> {
    if ct.is_cancelled() {
        Err(WorldGenError::Cancelled)
    } else {
        Ok(())
    }
}
