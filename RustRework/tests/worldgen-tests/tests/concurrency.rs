use std::sync::Arc;

mod rustrework_worldgen {
    pub use worldgen_core::*;
    pub use worldgen_persistence::*;
}

use rustrework_worldgen::{
    ChunkManager, FileChunkStore, LocalRealizer, MacroChunkKey, MacroGenerationPipeline,
    WorldBootstrap,
};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn parallel_macro_generation_completes_without_deadlock_and_is_consistent() {
    let directory = TempDir::new().unwrap();
    let profile = WorldBootstrap::create_default_profile(777);
    let manager = Arc::new(
        ChunkManager::new(
            Arc::new(FileChunkStore::new(directory.path()).await.unwrap()),
            Arc::new(MacroGenerationPipeline),
            Arc::new(LocalRealizer),
            None,
        )
        .await
        .unwrap(),
    );
    manager.register_world_profile(profile);
    manager.persist_world_profile().await.unwrap();
    let token = CancellationToken::new();

    let unique_tasks = (-2..=2)
        .flat_map(|y| (-2..=2).map(move |x| (x, y)))
        .map(|(x, y)| {
            let manager = Arc::clone(&manager);
            let token = token.clone();
            tokio::spawn(async move {
                manager
                    .get_or_create_macro_chunk(MacroChunkKey { x, y }, &token)
                    .await
                    .unwrap()
            })
        });

    let repeated_tasks = (0..10).map(|_| {
        let manager = Arc::clone(&manager);
        let token = token.clone();
        tokio::spawn(async move {
            manager
                .get_or_create_macro_chunk(MacroChunkKey { x: 3, y: 3 }, &token)
                .await
                .unwrap()
        })
    });

    let unique_results = futures::future::join_all(unique_tasks)
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    let repeated_results = futures::future::join_all(repeated_tasks)
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();

    assert_eq!(unique_results.len(), 25);
    assert!(unique_results.iter().all(|chunk| chunk.provenance.canonical_hash != 0));
    assert!(repeated_results
        .iter()
        .all(|chunk| chunk.provenance.canonical_hash == repeated_results[0].provenance.canonical_hash));
}
