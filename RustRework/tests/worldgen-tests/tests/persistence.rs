use std::sync::Arc;

mod rustrework_worldgen {
    pub use worldgen_core::*;
    pub use worldgen_persistence::*;
}

use rustrework_worldgen::{
    ChunkManager, ChunkStore, FileChunkStore, ItemKind, LocalChunkKey, LocalRealizer,
    MacroChunkKey, MacroGenerationPipeline, PlacedItem, WorldBootstrap,
};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn macro_chunk_persistence_roundtrips_canonical_state() {
    let directory = TempDir::new().unwrap();
    let store = Arc::new(FileChunkStore::new(directory.path()).await.unwrap());
    let profile = WorldBootstrap::create_default_profile(8_675_309);
    store.save_world_profile(&profile).await.unwrap();

    let generator = MacroGenerationPipeline;
    let chunk = generator
        .generate(MacroChunkKey { x: 1, y: 1 }, &profile, &CancellationToken::new())
        .unwrap();
    store.save_macro_chunk(&chunk).await.unwrap();
    let loaded = store
        .load_macro_chunk(MacroChunkKey { x: 1, y: 1 })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        rustrework_worldgen::serialize_macro_chunk_canonical(&chunk),
        rustrework_worldgen::serialize_macro_chunk_canonical(&loaded)
    );
}

#[tokio::test]
async fn local_chunk_mutation_roundtrips_without_modifying_macro_state() {
    let directory = TempDir::new().unwrap();
    let store = Arc::new(FileChunkStore::new(directory.path()).await.unwrap());
    let profile = WorldBootstrap::create_default_profile(20_260_409);
    let manager = ChunkManager::new(
        store,
        Arc::new(MacroGenerationPipeline),
        Arc::new(LocalRealizer),
        None,
    )
    .await
    .unwrap();
    manager.register_world_profile(profile.clone());
    manager.persist_world_profile().await.unwrap();

    let local_key = LocalChunkKey::from_world_coordinates(0, 0, 0, profile.dimensions);
    let macro_key = local_key.owning_macro_chunk_key();
    let macro_chunk = manager
        .get_or_create_macro_chunk(macro_key, &CancellationToken::new())
        .await
        .unwrap();
    let before_macro_hash = macro_chunk.provenance.canonical_hash;
    let local_chunk = manager
        .get_or_create_local_chunk(&local_key, &CancellationToken::new())
        .await
        .unwrap();
    {
        let mut chunk = local_chunk.lock();
        chunk.add_item(PlacedItem {
            type_: ItemKind::Fuel,
            quantity: 7,
            x: 2,
            y: 2,
            z: 1,
        });
    }
    manager.persist_local_chunk(&local_chunk).await.unwrap();
    manager.shutdown().await.unwrap();

    let reload_store = Arc::new(FileChunkStore::new(directory.path()).await.unwrap());
    let reload_manager = ChunkManager::new(
        reload_store,
        Arc::new(MacroGenerationPipeline),
        Arc::new(LocalRealizer),
        None,
    )
    .await
    .unwrap();
    reload_manager.register_world_profile(profile.clone());
    let reloaded_local = reload_manager
        .get_or_create_local_chunk(&local_key, &CancellationToken::new())
        .await
        .unwrap();
    let reloaded_macro = reload_manager
        .get_or_create_macro_chunk(macro_key, &CancellationToken::new())
        .await
        .unwrap();

    {
        let chunk = reloaded_local.lock();
        assert!(chunk.items.iter().any(|item| item.type_ == ItemKind::Fuel && item.quantity == 7));
        assert!(!chunk.is_dirty);
        assert!(chunk.provenance.is_mutated);
    }
    assert_eq!(before_macro_hash, reloaded_macro.provenance.canonical_hash);
}
