use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::{
    ChunkConnectivitySummary, ChunkDimensions, LandcoverType, LocalChunk, LocalChunkKey,
    LocalMaterialType, MacroChunk, MacroChunkKey, WorldProfile, ZBounds,
};
use crate::randomization::PhaseSeedDeriver;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorldProfileSummary {
    pub world_id: Uuid,
    pub world_seed: u64,
    pub generation_version: i32,
    pub active_region_id: String,
    pub content_hash: String,
    pub dimensions: ChunkDimensions,
    pub z_bounds: ZBounds,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MacroChunkSummary {
    pub key: MacroChunkKey,
    pub region_id: String,
    pub width: i32,
    pub height: i32,
    pub min_z: i32,
    pub max_z: i32,
    pub stored_parcel_count: usize,
    pub canonical_hash: u64,
    pub settlement_count: i32,
    pub special_count: i32,
    pub surface_landcover_histogram: BTreeMap<LandcoverType, usize>,
    pub connectivity: ChunkConnectivitySummary,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LocalChunkSummary {
    pub key: LocalChunkKey,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub canonical_hash: u64,
    pub is_uniform: bool,
    pub is_dirty: bool,
    pub mutation_count: i32,
    pub source_macro_canonical_hash: u64,
    pub material_histogram: BTreeMap<LocalMaterialType, usize>,
    pub prop_count: usize,
    pub item_count: usize,
    pub field_count: usize,
}

pub fn create_world_profile_summary(profile: &WorldProfile) -> WorldProfileSummary {
    WorldProfileSummary {
        world_id: profile.world_id,
        world_seed: profile.world_seed,
        generation_version: profile.generation_version,
        active_region_id: profile.active_region_id.clone(),
        content_hash: profile.content_hash.clone(),
        dimensions: profile.dimensions,
        z_bounds: profile.z_bounds,
    }
}

pub fn create_macro_summary(chunk: &MacroChunk) -> MacroChunkSummary {
    let mut histogram = BTreeMap::new();
    for cell in chunk.surface_cells() {
        *histogram.entry(cell.landcover).or_insert(0usize) += 1;
    }
    MacroChunkSummary {
        key: chunk.key,
        region_id: chunk.region_id.clone(),
        width: chunk.width,
        height: chunk.height,
        min_z: chunk.min_z(),
        max_z: chunk.max_z(),
        stored_parcel_count: chunk.stored_parcel_count(),
        canonical_hash: chunk.provenance.canonical_hash,
        settlement_count: chunk.feature_summary.settlement_count,
        special_count: chunk.feature_summary.special_count,
        surface_landcover_histogram: histogram,
        connectivity: chunk.connectivity.clone(),
    }
}

pub fn create_local_summary(chunk: &LocalChunk) -> LocalChunkSummary {
    let mut histogram = BTreeMap::new();
    for cell in &chunk.cells {
        *histogram.entry(cell.material).or_insert(0usize) += 1;
    }
    LocalChunkSummary {
        key: chunk.key.clone(),
        width: chunk.width,
        height: chunk.height,
        depth: chunk.depth,
        canonical_hash: chunk.provenance.canonical_hash,
        is_uniform: chunk.is_uniform,
        is_dirty: chunk.is_dirty,
        mutation_count: chunk.provenance.mutation_count,
        source_macro_canonical_hash: chunk.provenance.source_macro_canonical_hash,
        material_histogram: histogram,
        prop_count: chunk.props.len(),
        item_count: chunk.items.len(),
        field_count: chunk.fields.len(),
    }
}

pub fn serialize_macro_chunk_canonical(chunk: &MacroChunk) -> Vec<u8> {
    let features: Vec<String> = chunk
        .feature_summary
        .features
        .iter()
        .map(|feature| {
            format!(
                "{}:{},{},{}->{},{},{}:{}",
                feature.kind as i32,
                feature.start.x,
                feature.start.y,
                feature.start.z,
                feature.end.x,
                feature.end.y,
                feature.end.z,
                feature.elevated
            )
        })
        .collect();

    let mut cells = Vec::new();
    for z in chunk.min_z()..=chunk.max_z() {
        for y in 0..chunk.height {
            for x in 0..chunk.width {
                let cell = chunk.resolve_parcel(x, y, z);
                let layers: Vec<String> = cell
                    .layers
                    .iter()
                    .map(|layer| {
                        format!(
                            "{}:{}:{}:{}:{}:{}:{}",
                            layer.kind as i32,
                            layer.value.kind as i32,
                            layer.value.raw_value,
                            layer.priority,
                            layer.blend_mode as i32,
                            layer.flags.bits(),
                            layer.hint
                        )
                    })
                    .collect();
                cells.push(json!([
                    x,
                    y,
                    z,
                    cell.landcover as i32,
                    cell.infrastructure as i32,
                    cell.parcel as i32,
                    cell.elevation_mode as i32,
                    cell.flags.bits(),
                    cell.settlement_id.unwrap_or(-1),
                    cell.special
                        .as_ref()
                        .map(|special| special.instance_id.value as i64)
                        .unwrap_or(-1),
                    layers
                ]));
            }
        }
    }

    let value = json!({
        "x": chunk.key.x,
        "y": chunk.key.y,
        "width": chunk.width,
        "height": chunk.height,
        "minZ": chunk.z_bounds.min,
        "maxZ": chunk.z_bounds.max,
        "regionId": chunk.region_id,
        "contentHash": chunk.provenance.content_hash,
        "storedParcelCount": chunk.stored_parcel_count(),
        "features": features,
        "cells": cells,
    });
    serde_json::to_vec(&value).expect("canonical macro serialization must succeed")
}

pub fn serialize_local_chunk_canonical(chunk: &LocalChunk) -> Vec<u8> {
    let props: Vec<String> = chunk
        .props
        .iter()
        .map(|prop| format!("{}:{},{},{}", prop.type_ as i32, prop.x, prop.y, prop.z))
        .collect();
    let items: Vec<String> = chunk
        .items
        .iter()
        .map(|item| format!("{}:{}:{},{},{}", item.type_ as i32, item.quantity, item.x, item.y, item.z))
        .collect();
    let fields: Vec<String> = chunk
        .fields
        .iter()
        .map(|field| format!("{}:{}:{},{},{}", field.type_ as i32, field.intensity, field.x, field.y, field.z))
        .collect();

    let mut cells = Vec::new();
    for z in 0..chunk.depth {
        for y in 0..chunk.height {
            for x in 0..chunk.width {
                let cell = chunk.get_cell(x, y, z);
                cells.push(format!(
                    "{},{},{}:{}:{}:{}:{}",
                    x,
                    y,
                    z,
                    cell.material as i32,
                    cell.flags.bits(),
                    cell.elevation,
                    cell.moisture
                ));
            }
        }
    }

    let value = json!({
        "macroX": chunk.key.macro_key.x,
        "macroY": chunk.key.macro_key.y,
        "x": chunk.key.x,
        "y": chunk.key.y,
        "z": chunk.key.z,
        "width": chunk.width,
        "height": chunk.height,
        "depth": chunk.depth,
        "isUniform": chunk.is_uniform,
        "isDirty": chunk.is_dirty,
        "contentHash": chunk.provenance.content_hash,
        "sourceMacroCanonicalHash": chunk.provenance.source_macro_canonical_hash,
        "props": props,
        "items": items,
        "fields": fields,
        "cells": cells,
    });
    serde_json::to_vec(&value).expect("canonical local serialization must succeed")
}

pub fn compute_macro_hash(chunk: &MacroChunk) -> u64 {
    PhaseSeedDeriver::hash_bytes(&serialize_macro_chunk_canonical(chunk))
}

pub fn compute_local_hash(chunk: &LocalChunk) -> u64 {
    PhaseSeedDeriver::hash_bytes(&serialize_local_chunk_canonical(chunk))
}

pub fn to_text_hash(value: u64) -> String {
    format!("{value:016X}")
}

pub fn to_pretty_json(canonical_bytes: &[u8]) -> String {
    let value: Value = serde_json::from_slice(canonical_bytes).expect("canonical JSON is valid");
    serde_json::to_string_pretty(&value).expect("pretty JSON serialization must succeed")
}
