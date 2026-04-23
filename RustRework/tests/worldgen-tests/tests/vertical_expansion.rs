mod rustrework_worldgen {
    pub use worldgen_core::*;
}

use rustrework_worldgen::{
    CellFlags, FeatureKind, InfrastructureType, LandcoverType, MacroChunkBuilder, MacroChunkKey,
    MacroGenerationStage, ParcelType, VerticalDerivationStage, WorldBootstrap, ZBounds,
};
use tokio_util::sync::CancellationToken;

#[test]
fn vertical_derivation_extends_surface_triggers_across_full_cdda_z_bounds() {
    let profile = WorldBootstrap::create_default_profile(20_260_409);
    let mut builder = MacroChunkBuilder::new(MacroChunkKey { x: 0, y: 0 }, profile.clone());

    let bridge = builder.get_or_create_parcel(1, 1, 0);
    bridge.landcover = LandcoverType::Water;
    bridge.infrastructure = InfrastructureType::Bridge;
    bridge.parcel = ParcelType::BridgeSpan;
    bridge.flags.insert(CellFlags::WATERY | CellFlags::HAS_ROAD);

    let manhole = builder.get_or_create_parcel(2, 2, 0);
    manhole.infrastructure = InfrastructureType::Road;
    manhole.parcel = ParcelType::Manhole;
    manhole
        .flags
        .insert(CellFlags::HAS_ROAD | CellFlags::TRAVERSABLE | CellFlags::UNDERGROUND_CONNECTION);

    let house = builder.get_or_create_parcel(3, 3, 0);
    house.landcover = LandcoverType::Settlement;
    house.parcel = ParcelType::ResidentialBlock;
    house.flags.insert(CellFlags::SETTLEMENT | CellFlags::TRAVERSABLE);
    house.settlement_id = Some(1);

    let tower = builder.get_or_create_parcel(4, 4, 0);
    tower.landcover = LandcoverType::Settlement;
    tower.parcel = ParcelType::SkyscraperBase;
    tower.flags.insert(CellFlags::SETTLEMENT | CellFlags::TRAVERSABLE);
    tower.settlement_id = Some(1);
    let tower_metric_index = builder.get_metric_index(4, 4);
    builder.urbanity[tower_metric_index] = 1.0;

    VerticalDerivationStage
        .execute(&mut builder, &CancellationToken::new())
        .unwrap();
    let chunk = builder.build();

    assert_eq!(chunk.min_z(), ZBounds::cdda().min);
    assert_eq!(chunk.max_z(), ZBounds::cdda().max);
    assert_eq!(chunk.resolve_parcel(1, 1, 1).parcel, ParcelType::BridgeRoof);
    assert_eq!(chunk.resolve_parcel(1, 1, -1).parcel, ParcelType::BridgeSupport);
    assert_eq!(chunk.resolve_parcel(2, 2, -1).parcel, ParcelType::SewerTunnel);
    assert_eq!(chunk.resolve_parcel(3, 3, -1).parcel, ParcelType::Basement);
    assert_eq!(chunk.resolve_parcel(4, 4, 10).parcel, ParcelType::SkyscraperFloor);
    assert_eq!(chunk.resolve_parcel(4, 4, -2).parcel, ParcelType::SubwayStation);
    assert!(chunk.feature_summary.features.iter().any(|feature| feature.kind == FeatureKind::Sewer));
    assert!(chunk.feature_summary.features.iter().any(|feature| feature.kind == FeatureKind::Subway));
    assert!(chunk.stored_parcel_count() >= 6);
    assert!(chunk.stored_parcel_count() <= (profile.dimensions.macro_width * profile.dimensions.macro_height * profile.z_bounds.layer_count()) as usize);
}
