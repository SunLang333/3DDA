mod rustrework_worldgen {
    pub use worldgen_core::*;
}

use rustrework_worldgen::{serialize_macro_chunk_canonical, MacroChunkKey, MacroGenerationPipeline, WorldBootstrap};
use tokio_util::sync::CancellationToken;

#[test]
fn macro_generation_is_bitwise_stable_for_the_same_seed_and_key() {
    let profile = WorldBootstrap::create_default_profile(123_456);
    let generator = MacroGenerationPipeline;
    let key = MacroChunkKey { x: -2, y: 3 };

    let chunk_a = generator.generate(key, &profile, &CancellationToken::new()).unwrap();
    let chunk_b = generator.generate(key, &profile, &CancellationToken::new()).unwrap();

    assert_eq!(serialize_macro_chunk_canonical(&chunk_a), serialize_macro_chunk_canonical(&chunk_b));
}
