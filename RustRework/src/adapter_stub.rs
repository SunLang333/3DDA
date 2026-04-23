use serde::{Deserialize, Serialize};

use worldgen_core::LocalChunk;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GodotChunkSceneBlueprint {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub cell_count: usize,
    pub prop_count: usize,
    pub item_count: usize,
    pub field_count: usize,
    pub spawn_count: usize,
}

pub fn project_local_chunk_to_blueprint(chunk: &LocalChunk) -> GodotChunkSceneBlueprint {
    GodotChunkSceneBlueprint {
        width: chunk.width,
        height: chunk.height,
        depth: chunk.depth,
        cell_count: chunk.cells.len(),
        prop_count: chunk.props.len(),
        item_count: chunk.items.len(),
        field_count: chunk.fields.len(),
        spawn_count: chunk.spawns.len(),
    }
}
