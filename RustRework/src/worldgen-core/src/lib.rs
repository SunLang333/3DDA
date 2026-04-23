#[path = "../../abstractions.rs"]
pub mod abstractions;
#[path = "../../chunk_manager.rs"]
pub mod chunk_manager;
#[path = "../../data.rs"]
pub mod data;
#[path = "../../diagnostics.rs"]
pub mod diagnostics;
#[path = "../../domain.rs"]
pub mod domain;
#[path = "../../error.rs"]
pub mod error;
#[path = "../../feature_graph.rs"]
pub mod feature_graph;
#[path = "../../local_generation.rs"]
pub mod local_generation;
#[path = "../../lru_cache.rs"]
pub mod lru_cache;
#[path = "../../macro_generation.rs"]
pub mod macro_generation;
#[path = "../../randomization.rs"]
pub mod randomization;
#[path = "../../world_bootstrap.rs"]
pub mod world_bootstrap;

pub use abstractions::{ChunkStore, LocalChunkRealizer, MacroGenerator, SharedChunkStore};
pub use chunk_manager::{ChunkManager, ChunkManagerOptions, SharedLocalChunk};
pub use data::{load_items_csv, CsvItemTemplate};
pub use diagnostics::{
    create_local_summary, create_macro_summary, create_world_profile_summary, compute_local_hash,
    compute_macro_hash, serialize_local_chunk_canonical, serialize_macro_chunk_canonical,
    to_pretty_json, to_text_hash, LocalChunkSummary, MacroChunkSummary, WorldProfileSummary,
};
pub use domain::*;
pub use error::{Result, WorldGenError};
pub use feature_graph::{FeatureGraph, LinearFeatureGraph};
pub use local_generation::{LocalGenerationContext, LocalRealizer};
pub use macro_generation::{
    BuildInfrastructureStage, CalculateDensityMetricsStage, FinalizeOvermapStage,
    InitializeLayersStage, MacroChunkBuilder, MacroGenerationPhase, MacroGenerationPipeline,
    MacroGenerationStage, PlaceHydrologyStage, PlaceNaturalFeaturesStage, PlaceSpecialsStage,
    SeedSettlementsStage, VerticalDerivationStage,
};
pub use randomization::{DeterministicRandomSource, PhaseSeedDeriver};
pub use world_bootstrap::WorldBootstrap;
