use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::domain::{LocalChunk, LocalChunkKey, MacroChunk, MacroChunkKey, WorldProfile};
use crate::error::Result;

#[async_trait]
pub trait ChunkStore: Send + Sync {
    async fn save_world_profile(&self, profile: &WorldProfile) -> Result<()>;
    async fn load_world_profile(&self) -> Result<Option<WorldProfile>>;

    async fn save_macro_chunk(&self, chunk: &MacroChunk) -> Result<()>;
    async fn load_macro_chunk(&self, key: MacroChunkKey) -> Result<Option<MacroChunk>>;

    async fn save_local_chunk(&self, chunk: &LocalChunk) -> Result<()>;
    async fn load_local_chunk(&self, key: &LocalChunkKey) -> Result<Option<LocalChunk>>;
    async fn delete_local_chunk(&self, key: &LocalChunkKey) -> Result<()>;
}

pub trait MacroGenerator: Send + Sync {
    fn generate(
        &self,
        key: MacroChunkKey,
        profile: &WorldProfile,
        ct: &CancellationToken,
    ) -> Result<MacroChunk>;
}

pub trait LocalChunkRealizer: Send + Sync {
    fn realize(
        &self,
        key: &LocalChunkKey,
        owning_macro_chunk: &MacroChunk,
        profile: &WorldProfile,
        ct: &CancellationToken,
    ) -> Result<LocalChunk>;
}

pub type SharedChunkStore = Arc<dyn ChunkStore>;
