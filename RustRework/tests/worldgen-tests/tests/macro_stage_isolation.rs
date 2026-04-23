mod rustrework_worldgen {
    pub use worldgen_core::*;
}

use rustrework_worldgen::{
    CalculateDensityMetricsStage, CellFlags, InitializeLayersStage, MacroChunkBuilder,
    MacroChunkKey, MacroGenerationStage, PlaceHydrologyStage, WorldBootstrap,
};
use tokio_util::sync::CancellationToken;

#[test]
fn density_stage_populates_metric_arrays_without_creating_features() {
    let profile = WorldBootstrap::create_default_profile(42);
    let mut builder = MacroChunkBuilder::new(
        MacroChunkKey {
            x: profile.dimensions.macro_width / 2,
            y: 0,
        },
        profile,
    );
    let stage = CalculateDensityMetricsStage;

    stage.execute(&mut builder, &CancellationToken::new()).unwrap();

    assert!(builder.urbanity.iter().all(|value| (0.0..=1.0).contains(value)));
    assert!(builder.forestosity.iter().all(|value| (0.0..=1.0).contains(value)));
    assert!(builder.hydrology.iter().all(|value| (0.0..=1.0).contains(value)));
    assert!(builder.features.is_empty());
}

#[test]
fn hydrology_stage_marks_watery_cells_and_emits_feature_segments() {
    let mut profile = WorldBootstrap::create_default_profile(99);
    profile.active_region.hydrology_bias = 1.0;
    let mut builder = MacroChunkBuilder::new(MacroChunkKey { x: 0, y: 0 }, profile);
    InitializeLayersStage
        .execute(&mut builder, &CancellationToken::new())
        .unwrap();
    CalculateDensityMetricsStage
        .execute(&mut builder, &CancellationToken::new())
        .unwrap();

    PlaceHydrologyStage
        .execute(&mut builder, &CancellationToken::new())
        .unwrap();

    assert!(!builder.features.is_empty());
    assert!(builder.cells().any(|cell| cell.flags.contains(CellFlags::WATERY)));
}
