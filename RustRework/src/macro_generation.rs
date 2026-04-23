use crate::abstractions::MacroGenerator;
use std::collections::BTreeSet;

use tokio_util::sync::CancellationToken;

use crate::diagnostics::compute_macro_hash;
use crate::domain::{
    positive_mod, CellFlags, ChunkConnectivitySummary, ChunkFeatureSummary, ChunkDimensions,
    ElevationMode, FeatureKind, InfrastructureType, LandcoverType, LayerBlendMode,
    MacroChunk, MacroChunkKey, MacroChunkProvenance, ParcelType, SemanticLayer,
    SemanticLayerKind, SemanticParcel, SemanticValue, SpecialInstance, SpecialInstanceId,
    SpecialType, WorldCellCoordinate, WorldProfile,
};
use crate::error::{Result, WorldGenError};
use crate::feature_graph::{FeatureGraph, LinearFeatureGraph};
use crate::randomization::{DeterministicRandomSource, PhaseSeedDeriver};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MacroGenerationPhase {
    InitializeLayers = 0,
    CalculateDensityMetrics = 1,
    PlaceHydrology = 2,
    PlaceNaturalFeatures = 3,
    SeedSettlements = 4,
    BuildInfrastructure = 5,
    PlaceSpecials = 6,
    VerticalDerivation = 7,
    FinalizeOvermap = 8,
}

pub trait MacroGenerationStage {
    fn phase(&self) -> MacroGenerationPhase;
    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()>;
}

#[derive(Debug, Default, Clone)]
pub struct MacroGenerationPipeline;

impl MacroGenerationPipeline {
    pub fn generate(
        &self,
        key: MacroChunkKey,
        profile: &WorldProfile,
        ct: &CancellationToken,
    ) -> Result<MacroChunk> {
        let mut builder = MacroChunkBuilder::new(key, profile.clone());
        InitializeLayersStage.execute(&mut builder, ct)?;
        CalculateDensityMetricsStage.execute(&mut builder, ct)?;
        PlaceHydrologyStage.execute(&mut builder, ct)?;
        PlaceNaturalFeaturesStage.execute(&mut builder, ct)?;
        SeedSettlementsStage.execute(&mut builder, ct)?;
        BuildInfrastructureStage.execute(&mut builder, ct)?;
        PlaceSpecialsStage.execute(&mut builder, ct)?;
        VerticalDerivationStage.execute(&mut builder, ct)?;
        FinalizeOvermapStage.execute(&mut builder, ct)?;
        let mut chunk = builder.build();
        chunk.provenance.canonical_hash = compute_macro_hash(&chunk);
        Ok(chunk)
    }
}

impl MacroGenerator for MacroGenerationPipeline {
    fn generate(
        &self,
        key: MacroChunkKey,
        profile: &WorldProfile,
        ct: &CancellationToken,
    ) -> Result<MacroChunk> {
        MacroGenerationPipeline::generate(self, key, profile, ct)
    }
}

#[derive(Debug, Clone)]
pub struct MacroChunkBuilder {
    dimensions: ChunkDimensions,
    parcels: std::collections::BTreeMap<WorldCellCoordinate, SemanticParcel>,
    pub key: MacroChunkKey,
    pub profile: WorldProfile,
    pub urbanity: Vec<f64>,
    pub forestosity: Vec<f64>,
    pub hydrology: Vec<f64>,
    pub features: Vec<crate::domain::FeatureSegmentDescriptor>,
    pub settlement_count: i32,
    pub special_count: i32,
}

impl MacroChunkBuilder {
    pub fn new(key: MacroChunkKey, profile: WorldProfile) -> Self {
        let dimensions = profile.dimensions;
        Self {
            dimensions,
            parcels: std::collections::BTreeMap::new(),
            key,
            profile,
            urbanity: vec![0.0; (dimensions.macro_width * dimensions.macro_height) as usize],
            forestosity: vec![0.0; (dimensions.macro_width * dimensions.macro_height) as usize],
            hydrology: vec![0.0; (dimensions.macro_width * dimensions.macro_height) as usize],
            features: Vec::new(),
            settlement_count: 0,
            special_count: 0,
        }
    }

    pub fn width(&self) -> i32 {
        self.dimensions.macro_width
    }

    pub fn height(&self) -> i32 {
        self.dimensions.macro_height
    }

    pub fn min_z(&self) -> i32 {
        self.profile.z_bounds.min
    }

    pub fn max_z(&self) -> i32 {
        self.profile.z_bounds.max
    }

    pub fn cells(&self) -> impl Iterator<Item = &SemanticParcel> {
        self.parcels.values()
    }

    pub fn is_within_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < self.width() && y >= 0 && y < self.height() && self.profile.z_bounds.contains(z)
    }

    pub fn get_parcel(&self, x: i32, y: i32, z: i32) -> Option<&SemanticParcel> {
        self.validate_coordinates(x, y, z);
        self.parcels.get(&WorldCellCoordinate { x, y, z })
    }

    pub fn resolve_parcel(&self, x: i32, y: i32, z: i32) -> SemanticParcel {
        self.get_parcel(x, y, z)
            .cloned()
            .unwrap_or_else(|| MacroChunk::create_implicit_parcel(z, self.profile.active_region.default_surface_landcover))
    }

    pub fn get_or_create_parcel(&mut self, x: i32, y: i32, z: i32) -> &mut SemanticParcel {
        self.validate_coordinates(x, y, z);
        self.parcels
            .entry(WorldCellCoordinate { x, y, z })
            .or_insert_with(|| MacroChunk::create_implicit_parcel(z, self.profile.active_region.default_surface_landcover))
    }

    pub fn set_parcel(&mut self, x: i32, y: i32, z: i32, parcel: SemanticParcel) {
        self.validate_coordinates(x, y, z);
        self.parcels.insert(WorldCellCoordinate { x, y, z }, parcel);
    }

    pub fn get_metric_index(&self, x: i32, y: i32) -> usize {
        ((y * self.width()) + x) as usize
    }

    pub fn to_world_coordinate(&self, local_x: i32, local_y: i32, z: i32) -> WorldCellCoordinate {
        WorldCellCoordinate {
            x: (self.key.x * self.width()) + local_x,
            y: (self.key.y * self.height()) + local_y,
            z,
        }
    }

    pub fn append_layer(parcel: &mut SemanticParcel, layer: SemanticLayer) {
        parcel.layers.push(layer);
    }

    pub fn build(&self) -> MacroChunk {
        let mut features = self.features.clone();
        features.sort_by_key(|feature| {
            (
                feature.kind,
                feature.start.z,
                feature.start.y,
                feature.start.x,
                feature.end.z,
                feature.end.y,
                feature.end.x,
            )
        });

        MacroChunk {
            key: self.key,
            width: self.width(),
            height: self.height(),
            z_bounds: self.profile.z_bounds,
            parcels: self.parcels.clone(),
            region_id: self.profile.active_region_id.clone(),
            surface_default_landcover: self.profile.active_region.default_surface_landcover,
            feature_summary: ChunkFeatureSummary {
                features,
                settlement_count: self.settlement_count,
                special_count: self.special_count,
            },
            connectivity: self.build_connectivity_summary(),
            provenance: MacroChunkProvenance {
                schema_version: 2,
                generation_version: self.profile.generation_version,
                content_hash: self.profile.content_hash.clone(),
                macro_seed: PhaseSeedDeriver::hash64(self.profile.world_seed, self.key, "macro:chunk"),
                canonical_hash: 0,
                generated_at_utc: crate::domain::utc_now_string(),
            },
        }
    }

    fn build_connectivity_summary(&self) -> ChunkConnectivitySummary {
        let mut summary = ChunkConnectivitySummary::default();
        for x in 0..self.width() {
            Self::accumulate_boundary(&mut summary, &self.resolve_parcel(x, 0, 0), true, false, false, false);
            Self::accumulate_boundary(
                &mut summary,
                &self.resolve_parcel(x, self.height() - 1, 0),
                false,
                true,
                false,
                false,
            );
        }
        for y in 0..self.height() {
            Self::accumulate_boundary(&mut summary, &self.resolve_parcel(0, y, 0), false, false, false, true);
            Self::accumulate_boundary(
                &mut summary,
                &self.resolve_parcel(self.width() - 1, y, 0),
                false,
                false,
                true,
                false,
            );
        }
        summary
    }

    fn accumulate_boundary(
        summary: &mut ChunkConnectivitySummary,
        parcel: &SemanticParcel,
        north: bool,
        south: bool,
        east: bool,
        west: bool,
    ) {
        if parcel.flags.contains(CellFlags::HAS_ROAD) {
            if north {
                summary.north_road_connections += 1;
            }
            if south {
                summary.south_road_connections += 1;
            }
            if east {
                summary.east_road_connections += 1;
            }
            if west {
                summary.west_road_connections += 1;
            }
        }

        if parcel.flags.contains(CellFlags::WATERY) {
            if north {
                summary.north_water_connections += 1;
            }
            if south {
                summary.south_water_connections += 1;
            }
            if east {
                summary.east_water_connections += 1;
            }
            if west {
                summary.west_water_connections += 1;
            }
        }
    }

    fn validate_coordinates(&self, x: i32, y: i32, z: i32) {
        assert!(
            self.is_within_bounds(x, y, z),
            "Invalid macro parcel coordinate ({x}, {y}, {z}) for chunk {}.",
            self.key
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InitializeLayersStage;

impl MacroGenerationStage for InitializeLayersStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::InitializeLayers
    }

    fn execute(&self, _builder: &mut MacroChunkBuilder, _ct: &CancellationToken) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CalculateDensityMetricsStage;

impl MacroGenerationStage for CalculateDensityMetricsStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::CalculateDensityMetrics
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        let urban_seed = PhaseSeedDeriver::hash64(builder.profile.world_seed, builder.key, "macro:metrics:urbanity");
        let forest_seed = PhaseSeedDeriver::hash64(builder.profile.world_seed, builder.key, "macro:metrics:forestosity");
        let hydrology_seed = PhaseSeedDeriver::hash64(builder.profile.world_seed, builder.key, "macro:metrics:hydrology");

        for y in 0..builder.height() {
            for x in 0..builder.width() {
                check_cancelled(ct)?;
                let index = builder.get_metric_index(x, y);
                builder.urbanity[index] = PhaseSeedDeriver::to_unit_double(PhaseSeedDeriver::hash_coordinate(urban_seed, x, y, 0));
                builder.forestosity[index] = PhaseSeedDeriver::to_unit_double(PhaseSeedDeriver::hash_coordinate(forest_seed, x, y, 0));
                builder.hydrology[index] = PhaseSeedDeriver::to_unit_double(PhaseSeedDeriver::hash_coordinate(hydrology_seed, x, y, 0));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PlaceHydrologyStage;

impl MacroGenerationStage for PlaceHydrologyStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::PlaceHydrology
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        let mut rng = DeterministicRandomSource::for_macro_phase(&builder.profile, builder.key, "macro:hydrology");
        if rng.next_bool(builder.profile.active_region.hydrology_bias) {
            let horizontal = rng.next_bool(0.5);
            let offset = if horizontal {
                rng.next_int(2, builder.height() - 2)
            } else {
                rng.next_int(2, builder.width() - 2)
            };
            let start = if horizontal {
                builder.to_world_coordinate(0, offset, 0)
            } else {
                builder.to_world_coordinate(offset, 0, 0)
            };
            let end = if horizontal {
                builder.to_world_coordinate(builder.width() - 1, offset, 0)
            } else {
                builder.to_world_coordinate(offset, builder.height() - 1, 0)
            };
            let river = LinearFeatureGraph::create_line(FeatureKind::River, start, end, false);
            builder.features.extend(river.segments.clone());
            for point in river.rasterize() {
                check_cancelled(ct)?;
                let local_x = positive_mod(point.x, builder.width());
                let local_y = positive_mod(point.y, builder.height());
                let parcel = builder.get_or_create_parcel(local_x, local_y, 0);
                parcel.landcover = LandcoverType::Water;
                parcel.parcel = ParcelType::River;
                parcel.elevation_mode = ElevationMode::Basin;
                parcel.flags.insert(CellFlags::WATERY);
                MacroChunkBuilder::append_layer(
                    parcel,
                    SemanticLayer::create(
                        SemanticLayerKind::Hydrology,
                        SemanticValue::for_landcover(LandcoverType::Water),
                        10,
                        LayerBlendMode::Overlay,
                        CellFlags::WATERY,
                        "river",
                    ),
                );
            }
        }

        if rng.next_bool(0.25) {
            let center_x = rng.next_int(3, builder.width() - 3);
            let center_y = rng.next_int(3, builder.height() - 3);
            let radius = rng.next_int(1, 3);
            for y in (center_y - radius)..=(center_y + radius) {
                for x in (center_x - radius)..=(center_x + radius) {
                    if x < 0 || x >= builder.width() || y < 0 || y >= builder.height() {
                        continue;
                    }
                    if (x - center_x).abs() + (y - center_y).abs() > radius + 1 {
                        continue;
                    }
                    let parcel = builder.get_or_create_parcel(x, y, 0);
                    parcel.landcover = LandcoverType::Water;
                    parcel.parcel = ParcelType::River;
                    parcel.elevation_mode = ElevationMode::Basin;
                    parcel.flags.insert(CellFlags::WATERY);
                    MacroChunkBuilder::append_layer(
                        parcel,
                        SemanticLayer::create(
                            SemanticLayerKind::Hydrology,
                            SemanticValue::for_landcover(LandcoverType::Water),
                            11,
                            LayerBlendMode::Overlay,
                            CellFlags::WATERY,
                            "lake",
                        ),
                    );
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PlaceNaturalFeaturesStage;

impl MacroGenerationStage for PlaceNaturalFeaturesStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::PlaceNaturalFeatures
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        for y in 0..builder.height() {
            for x in 0..builder.width() {
                check_cancelled(ct)?;
                let current = builder.resolve_parcel(x, y, 0);
                if current.flags.contains(CellFlags::WATERY) {
                    continue;
                }
                let index = builder.get_metric_index(x, y);
                let forestosity = builder.forestosity[index];
                let hydrology = builder.hydrology[index];
                if forestosity >= builder.profile.active_region.forest_bias {
                    let parcel = builder.get_or_create_parcel(x, y, 0);
                    parcel.landcover = LandcoverType::Forest;
                    parcel.parcel = ParcelType::ForestStand;
                    MacroChunkBuilder::append_layer(
                        parcel,
                        SemanticLayer::create(
                            SemanticLayerKind::Vegetation,
                            SemanticValue::for_landcover(LandcoverType::Forest),
                            20,
                            LayerBlendMode::Refine,
                            CellFlags::NONE,
                            "forest",
                        ),
                    );
                } else if hydrology >= 0.82 {
                    let parcel = builder.get_or_create_parcel(x, y, 0);
                    parcel.landcover = LandcoverType::Wetland;
                    parcel.parcel = ParcelType::Field;
                    MacroChunkBuilder::append_layer(
                        parcel,
                        SemanticLayer::create(
                            SemanticLayerKind::Vegetation,
                            SemanticValue::for_landcover(LandcoverType::Wetland),
                            21,
                            LayerBlendMode::Refine,
                            CellFlags::NONE,
                            "wetland",
                        ),
                    );
                } else if (x == 0 || x == builder.width() - 1 || y == 0 || y == builder.height() - 1)
                    && forestosity <= 0.1
                {
                    let parcel = builder.get_or_create_parcel(x, y, 0);
                    parcel.landcover = LandcoverType::Ravine;
                    parcel.elevation_mode = ElevationMode::Basin;
                    MacroChunkBuilder::append_layer(
                        parcel,
                        SemanticLayer::create(
                            SemanticLayerKind::Vegetation,
                            SemanticValue::for_landcover(LandcoverType::Ravine),
                            22,
                            LayerBlendMode::Refine,
                            CellFlags::NONE,
                            "ravine-edge",
                        ),
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SeedSettlementsStage;

impl MacroGenerationStage for SeedSettlementsStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::SeedSettlements
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        let mut best_score = f64::MIN;
        let mut best_x = -1;
        let mut best_y = -1;
        for y in 0..builder.height() {
            for x in 0..builder.width() {
                let parcel = builder.resolve_parcel(x, y, 0);
                if parcel.flags.contains(CellFlags::WATERY) {
                    continue;
                }
                let index = builder.get_metric_index(x, y);
                let score = builder.urbanity[index]
                    - (builder.hydrology[index] * 0.35)
                    - ((x as f64 - builder.width() as f64 / 2.0).abs()
                        + (y as f64 - builder.height() as f64 / 2.0).abs())
                        * 0.01;
                if score > best_score {
                    best_score = score;
                    best_x = x;
                    best_y = y;
                }
            }
        }

        if best_score < builder.profile.active_region.settlement_threshold {
            return Ok(());
        }

        builder.settlement_count = 1;
        let promote_to_skyscraper = best_score >= builder.profile.active_region.settlement_threshold + 0.08;
        for y in (best_y - 2).max(0)..=(best_y + 2).min(builder.height() - 1) {
            for x in (best_x - 2).max(0)..=(best_x + 2).min(builder.width() - 1) {
                check_cancelled(ct)?;
                let distance = (x - best_x).abs() + (y - best_y).abs();
                if distance > 3 {
                    continue;
                }
                let current = builder.resolve_parcel(x, y, 0);
                if current.flags.contains(CellFlags::WATERY) {
                    continue;
                }
                let parcel = builder.get_or_create_parcel(x, y, 0);
                parcel.landcover = LandcoverType::Settlement;
                parcel.parcel = match distance {
                    0 if promote_to_skyscraper => ParcelType::SkyscraperBase,
                    0 => ParcelType::TownCenter,
                    _ => ParcelType::ResidentialBlock,
                };
                parcel.settlement_id = Some(1);
                parcel.flags.insert(CellFlags::SETTLEMENT | CellFlags::TRAVERSABLE);
                MacroChunkBuilder::append_layer(
                    parcel,
                    SemanticLayer::create(
                        SemanticLayerKind::Parcel,
                        SemanticValue::for_parcel(parcel.parcel),
                        30,
                        LayerBlendMode::Overlay,
                        CellFlags::SETTLEMENT,
                        if distance == 0 {
                            if promote_to_skyscraper {
                                "settlement-skyscraper-core"
                            } else {
                                "settlement-core"
                            }
                        } else {
                            "settlement-ring"
                        },
                    ),
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BuildInfrastructureStage;

impl MacroGenerationStage for BuildInfrastructureStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::BuildInfrastructure
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        let mut rng = DeterministicRandomSource::for_macro_phase(&builder.profile, builder.key, "macro:infrastructure");
        let primary_road_y = find_primary_settlement_row(builder).unwrap_or(builder.height() / 2);
        add_linear_infrastructure(builder, FeatureKind::Road, 0, primary_road_y, builder.width() - 1, primary_road_y, ct)?;
        if rng.next_bool(builder.profile.active_region.road_density) {
            let highway_x = find_primary_settlement_column(builder).unwrap_or_else(|| rng.next_int(2, builder.width() - 2));
            add_linear_infrastructure(builder, FeatureKind::Highway, highway_x, 0, highway_x, builder.height() - 1, ct)?;
        }
        place_manholes(builder);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PlaceSpecialsStage;

impl MacroGenerationStage for PlaceSpecialsStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::PlaceSpecials
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        let mut candidates = Vec::new();
        for y in 1..builder.height() - 1 {
            for x in 1..builder.width() - 1 {
                let parcel = builder.resolve_parcel(x, y, 0);
                if parcel.flags.contains(CellFlags::WATERY) || parcel.special.is_some() {
                    continue;
                }
                let adjacent_road = has_adjacent(builder, x, y, |candidate| candidate.flags.contains(CellFlags::HAS_ROAD));
                let adjacent_forest = has_adjacent(builder, x, y, |candidate| candidate.landcover == LandcoverType::Forest);
                if adjacent_road && adjacent_forest {
                    let score = (parcel.settlement_id.unwrap_or(0) * 5)
                        + if parcel.landcover == LandcoverType::Forest { 3 } else { 1 };
                    candidates.push((x, y, score));
                }
            }
        }

        let mut selected = candidates
            .into_iter()
            .max_by(|left, right| left.2.cmp(&right.2).then_with(|| right.1.cmp(&left.1)).then_with(|| right.0.cmp(&left.0)));

        if selected.is_none() {
            if let Some(bridge_feature) = builder
                .features
                .iter()
                .find(|feature| feature.kind == FeatureKind::Highway && feature.elevated)
                .cloned()
            {
                selected = Some((
                    positive_mod(bridge_feature.start.x, builder.width()),
                    positive_mod(bridge_feature.start.y, builder.height()),
                    1,
                ));
            }
        }

        let Some((selected_x, selected_y, _)) = selected else {
            return Ok(());
        };

        check_cancelled(ct)?;
        let world_coordinate = builder.to_world_coordinate(selected_x, selected_y, 0);
        let world_seed = builder.profile.world_seed;
        let special_instance_id = SpecialInstanceId {
            value: PhaseSeedDeriver::hash_coordinate(
                world_seed,
                world_coordinate.x,
                world_coordinate.y,
                world_coordinate.z,
            ),
        };
        let parcel = builder.get_or_create_parcel(selected_x, selected_y, 0);
        parcel.parcel = ParcelType::ResearchOutpost;
        parcel.landcover = LandcoverType::Settlement;
        parcel.flags.insert(CellFlags::HAS_SPECIAL | CellFlags::TRAVERSABLE);
        parcel.special = Some(SpecialInstance {
            instance_id: special_instance_id,
            type_: SpecialType::ResearchOutpost,
            location: world_coordinate,
        });
        MacroChunkBuilder::append_layer(
            parcel,
            SemanticLayer::create(
                SemanticLayerKind::Parcel,
                SemanticValue::for_special(SpecialType::ResearchOutpost),
                50,
                LayerBlendMode::Overlay,
                CellFlags::HAS_SPECIAL,
                "research-outpost",
            ),
        );
        builder.special_count += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VerticalDerivationStage;

impl MacroGenerationStage for VerticalDerivationStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::VerticalDerivation
    }

    fn execute(&self, builder: &mut MacroChunkBuilder, ct: &CancellationToken) -> Result<()> {
        let mut sewer_anchors = BTreeSet::new();
        let mut subway_anchors = BTreeSet::new();

        for y in 0..builder.height() {
            for x in 0..builder.width() {
                check_cancelled(ct)?;
                let surface = builder.resolve_parcel(x, y, 0);
                let Some(trigger_omt_id) = VerticalSemanticCatalog::trigger_omt_id(&surface) else {
                    continue;
                };
                for rule in default_vertical_extension_rules()
                    .into_iter()
                    .filter(|rule| rule.trigger_omt_id == trigger_omt_id)
                {
                    if let Some(condition) = rule.condition {
                        if !condition(builder, x, y) {
                            continue;
                        }
                    }

                    if trigger_omt_id == VerticalSemanticCatalog::SKYSCRAPER_TRIGGER_ID {
                        let top_z = determine_skyscraper_top_z(builder, x, y);
                        for target_z in 1..=top_z {
                            place_vertical_parcel(builder, x, y, target_z, rule.placed_omt_id, &surface);
                        }
                        if builder.min_z() <= -2 {
                            let station = VerticalSemanticCatalog::create_placed_parcel(
                                "subway_station",
                                &surface,
                                -2,
                                builder.profile.active_region.default_surface_landcover,
                            );
                            builder.set_parcel(x, y, -2, station);
                            subway_anchors.insert(builder.to_world_coordinate(x, y, -2));
                        }
                        continue;
                    }

                    let destination_z = rule.delta_z;
                    if !builder.is_within_bounds(x, y, destination_z) {
                        continue;
                    }
                    place_vertical_parcel(builder, x, y, destination_z, rule.placed_omt_id, &surface);
                    if rule.placed_omt_id == "sewer_tunnel" {
                        sewer_anchors.insert(builder.to_world_coordinate(x, y, destination_z));
                    }
                }
            }
        }

        let phase_rng = DeterministicRandomSource::for_macro_phase(&builder.profile, builder.key, "macro:vertical");
        generate_subsurface_network(
            builder,
            &sewer_anchors,
            -1,
            FeatureKind::Sewer,
            "sewer_tunnel",
            "sewer_tunnel",
            phase_rng.fork("sewer_network"),
            ct,
        )?;
        generate_subsurface_network(
            builder,
            &subway_anchors,
            -2,
            FeatureKind::Subway,
            "subway_tunnel",
            "subway_station",
            phase_rng.fork("subway_network"),
            ct,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FinalizeOvermapStage;

impl MacroGenerationStage for FinalizeOvermapStage {
    fn phase(&self) -> MacroGenerationPhase {
        MacroGenerationPhase::FinalizeOvermap
    }

    fn execute(&self, _builder: &mut MacroChunkBuilder, _ct: &CancellationToken) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct VerticalExtensionRule {
    trigger_omt_id: &'static str,
    delta_z: i32,
    placed_omt_id: &'static str,
    condition: Option<fn(&MacroChunkBuilder, i32, i32) -> bool>,
}

fn default_vertical_extension_rules() -> Vec<VerticalExtensionRule> {
    vec![
        VerticalExtensionRule {
            trigger_omt_id: VerticalSemanticCatalog::BRIDGE_TRIGGER_ID,
            delta_z: 1,
            placed_omt_id: "bridgepaved_top",
            condition: None,
        },
        VerticalExtensionRule {
            trigger_omt_id: VerticalSemanticCatalog::BRIDGE_TRIGGER_ID,
            delta_z: -1,
            placed_omt_id: "bridge_support",
            condition: Some(|builder, x, y| {
                let surface = builder.resolve_parcel(x, y, 0);
                surface.landcover == LandcoverType::Water || surface.flags.contains(CellFlags::WATERY)
            }),
        },
        VerticalExtensionRule {
            trigger_omt_id: VerticalSemanticCatalog::MANHOLE_TRIGGER_ID,
            delta_z: -1,
            placed_omt_id: "sewer_tunnel",
            condition: None,
        },
        VerticalExtensionRule {
            trigger_omt_id: VerticalSemanticCatalog::HOUSE_TRIGGER_ID,
            delta_z: -1,
            placed_omt_id: "basement",
            condition: None,
        },
        VerticalExtensionRule {
            trigger_omt_id: VerticalSemanticCatalog::SKYSCRAPER_TRIGGER_ID,
            delta_z: 1,
            placed_omt_id: "skyscraper_floor",
            condition: None,
        },
    ]
}

struct VerticalSemanticCatalog;

impl VerticalSemanticCatalog {
    const BRIDGE_TRIGGER_ID: &'static str = "road_bridge";
    const MANHOLE_TRIGGER_ID: &'static str = "manhole_surface";
    const HOUSE_TRIGGER_ID: &'static str = "house";
    const SKYSCRAPER_TRIGGER_ID: &'static str = "skyscraper";

    fn trigger_omt_id(parcel: &SemanticParcel) -> Option<&'static str> {
        if parcel.infrastructure == InfrastructureType::Bridge {
            return Some(Self::BRIDGE_TRIGGER_ID);
        }
        match parcel.parcel {
            ParcelType::Manhole => Some(Self::MANHOLE_TRIGGER_ID),
            ParcelType::ResidentialBlock => Some(Self::HOUSE_TRIGGER_ID),
            ParcelType::SkyscraperBase => Some(Self::SKYSCRAPER_TRIGGER_ID),
            _ => None,
        }
    }

    fn create_placed_parcel(
        omt_id: &str,
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        match omt_id {
            "bridgepaved_top" => Self::create_bridge_roof(source, target_z, surface_default_landcover),
            "bridge_support" => Self::create_bridge_support(source, target_z, surface_default_landcover),
            "sewer_tunnel" => Self::create_sewer_tunnel(source, target_z, surface_default_landcover),
            "basement" => Self::create_basement(source, target_z, surface_default_landcover),
            "skyscraper_floor" => Self::create_skyscraper_floor(source, target_z, surface_default_landcover),
            "subway_tunnel" => Self::create_subway_tunnel(source, target_z, surface_default_landcover),
            "subway_station" => Self::create_subway_station(source, target_z, surface_default_landcover),
            other => panic!("Unknown vertical placement OMT id '{other}'."),
        }
    }

    fn create_bridge_roof(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = MacroChunk::create_implicit_parcel(target_z, surface_default_landcover);
        parcel.landcover = LandcoverType::OpenAir;
        parcel.infrastructure = InfrastructureType::Bridge;
        parcel.parcel = ParcelType::BridgeRoof;
        parcel.elevation_mode = ElevationMode::Bridge;
        parcel.flags.insert(CellFlags::HAS_ROAD | CellFlags::OVERHEAD_CONNECTION | CellFlags::TRAVERSABLE);
        parcel.settlement_id = source.settlement_id;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Overhead,
            SemanticValue::for_parcel(ParcelType::BridgeRoof),
            60,
            LayerBlendMode::Overlay,
            CellFlags::HAS_ROAD | CellFlags::OVERHEAD_CONNECTION,
            "bridge-roof",
        ));
        parcel
    }

    fn create_bridge_support(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = MacroChunk::create_implicit_parcel(target_z, surface_default_landcover);
        parcel.landcover = LandcoverType::Subterranean;
        parcel.infrastructure = InfrastructureType::Bridge;
        parcel.parcel = ParcelType::BridgeSupport;
        parcel.elevation_mode = ElevationMode::Underground;
        parcel.flags.insert(CellFlags::UNDERGROUND_CONNECTION);
        parcel.settlement_id = source.settlement_id;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Underground,
            SemanticValue::for_parcel(ParcelType::BridgeSupport),
            61,
            LayerBlendMode::Overlay,
            CellFlags::UNDERGROUND_CONNECTION,
            "bridge-support",
        ));
        parcel
    }

    fn create_sewer_tunnel(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = MacroChunk::create_implicit_parcel(target_z, surface_default_landcover);
        parcel.landcover = LandcoverType::Subterranean;
        parcel.infrastructure = InfrastructureType::Sewer;
        parcel.parcel = ParcelType::SewerTunnel;
        parcel.elevation_mode = ElevationMode::Underground;
        parcel.flags.insert(CellFlags::UNDERGROUND_CONNECTION | CellFlags::TRAVERSABLE);
        parcel.settlement_id = source.settlement_id;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Underground,
            SemanticValue::for_infrastructure(InfrastructureType::Sewer),
            62,
            LayerBlendMode::Overlay,
            CellFlags::UNDERGROUND_CONNECTION,
            "sewer-tunnel",
        ));
        parcel
    }

    fn create_basement(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = MacroChunk::create_implicit_parcel(target_z, surface_default_landcover);
        parcel.landcover = LandcoverType::Subterranean;
        parcel.infrastructure = InfrastructureType::UtilityTunnel;
        parcel.parcel = ParcelType::Basement;
        parcel.elevation_mode = ElevationMode::Underground;
        parcel.flags.insert(CellFlags::UNDERGROUND_CONNECTION | CellFlags::TRAVERSABLE);
        parcel.settlement_id = source.settlement_id;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Underground,
            SemanticValue::for_parcel(ParcelType::Basement),
            63,
            LayerBlendMode::Overlay,
            CellFlags::UNDERGROUND_CONNECTION,
            "basement",
        ));
        parcel
    }

    fn create_skyscraper_floor(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = MacroChunk::create_implicit_parcel(target_z, surface_default_landcover);
        parcel.landcover = LandcoverType::Settlement;
        parcel.infrastructure = InfrastructureType::None;
        parcel.parcel = ParcelType::SkyscraperFloor;
        parcel.elevation_mode = ElevationMode::Elevated;
        parcel.flags.insert(CellFlags::SETTLEMENT | CellFlags::TRAVERSABLE | CellFlags::OVERHEAD_CONNECTION);
        parcel.settlement_id = source.settlement_id;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Overhead,
            SemanticValue::for_parcel(ParcelType::SkyscraperFloor),
            64,
            LayerBlendMode::Overlay,
            CellFlags::SETTLEMENT | CellFlags::OVERHEAD_CONNECTION,
            "skyscraper-floor",
        ));
        parcel
    }

    fn create_subway_tunnel(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = MacroChunk::create_implicit_parcel(target_z, surface_default_landcover);
        parcel.landcover = LandcoverType::Subterranean;
        parcel.infrastructure = InfrastructureType::Subway;
        parcel.parcel = ParcelType::SubwayTunnel;
        parcel.elevation_mode = ElevationMode::Underground;
        parcel.flags.insert(CellFlags::UNDERGROUND_CONNECTION | CellFlags::TRAVERSABLE);
        parcel.settlement_id = source.settlement_id;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Underground,
            SemanticValue::for_infrastructure(InfrastructureType::Subway),
            65,
            LayerBlendMode::Overlay,
            CellFlags::UNDERGROUND_CONNECTION,
            "subway-tunnel",
        ));
        parcel
    }

    fn create_subway_station(
        source: &SemanticParcel,
        target_z: i32,
        surface_default_landcover: LandcoverType,
    ) -> SemanticParcel {
        let mut parcel = Self::create_subway_tunnel(source, target_z, surface_default_landcover);
        parcel.parcel = ParcelType::SubwayStation;
        parcel.layers.push(SemanticLayer::create(
            SemanticLayerKind::Underground,
            SemanticValue::for_parcel(ParcelType::SubwayStation),
            66,
            LayerBlendMode::Overlay,
            CellFlags::UNDERGROUND_CONNECTION,
            "subway-station",
        ));
        parcel
    }
}

fn determine_skyscraper_top_z(builder: &MacroChunkBuilder, x: i32, y: i32) -> i32 {
    if builder.max_z() <= 0 {
        return 0;
    }
    let urbanity = builder.urbanity[builder.get_metric_index(x, y)];
    let desired_floors = 3 + (urbanity * f64::from(builder.max_z() - 2)).round() as i32;
    desired_floors.clamp(1, builder.max_z())
}

fn place_vertical_parcel(
    builder: &mut MacroChunkBuilder,
    x: i32,
    y: i32,
    z: i32,
    placed_omt_id: &str,
    source: &SemanticParcel,
) {
    let parcel = VerticalSemanticCatalog::create_placed_parcel(
        placed_omt_id,
        source,
        z,
        builder.profile.active_region.default_surface_landcover,
    );
    builder.set_parcel(x, y, z, parcel);
}

fn generate_subsurface_network(
    builder: &mut MacroChunkBuilder,
    anchors: &BTreeSet<WorldCellCoordinate>,
    z: i32,
    feature_kind: FeatureKind,
    tunnel_omt_id: &str,
    anchor_omt_id: &str,
    mut rng: DeterministicRandomSource,
    ct: &CancellationToken,
) -> Result<()> {
    if anchors.is_empty() || z < builder.min_z() || z > builder.max_z() {
        return Ok(());
    }

    let mut nodes: Vec<_> = anchors.iter().copied().collect();
    if nodes.len() == 1 {
        let boundary = create_boundary_node(builder, nodes[0], z, &mut rng);
        nodes.push(boundary);
    }

    let mut frontier = vec![nodes[0]];
    for node in nodes.iter().skip(1).copied() {
        check_cancelled(ct)?;
        let parent = frontier
            .iter()
            .copied()
            .min_by_key(|candidate| (manhattan_distance(*candidate, node), candidate.y, candidate.x))
            .expect("frontier always has a seed node");
        add_network_line(
            builder,
            parent,
            node,
            z,
            feature_kind,
            tunnel_omt_id,
            anchor_omt_id,
            anchors,
            ct,
        )?;
        frontier.push(node);
    }

    if nodes.len() >= 3 && rng.next_bool(0.45) {
        let extra_start = nodes[0];
        let extra_end = nodes[nodes.len() - 1];
        add_network_line(
            builder,
            extra_start,
            extra_end,
            z,
            feature_kind,
            tunnel_omt_id,
            anchor_omt_id,
            anchors,
            ct,
        )?;
    }

    for anchor in anchors {
        let local_x = positive_mod(anchor.x, builder.width());
        let local_y = positive_mod(anchor.y, builder.height());
        let source = builder.resolve_parcel(local_x, local_y, 0);
        let parcel = VerticalSemanticCatalog::create_placed_parcel(
            anchor_omt_id,
            &source,
            z,
            builder.profile.active_region.default_surface_landcover,
        );
        builder.set_parcel(local_x, local_y, z, parcel);
    }

    Ok(())
}

fn add_network_line(
    builder: &mut MacroChunkBuilder,
    start: WorldCellCoordinate,
    end: WorldCellCoordinate,
    z: i32,
    feature_kind: FeatureKind,
    tunnel_omt_id: &str,
    anchor_omt_id: &str,
    anchors: &BTreeSet<WorldCellCoordinate>,
    ct: &CancellationToken,
) -> Result<()> {
    let graph = LinearFeatureGraph::create_line(
        feature_kind,
        WorldCellCoordinate { z, ..start },
        WorldCellCoordinate { z, ..end },
        false,
    );
    builder.features.extend(graph.segments.clone());
    for point in graph.rasterize() {
        check_cancelled(ct)?;
        let local_x = positive_mod(point.x, builder.width());
        let local_y = positive_mod(point.y, builder.height());
        let omt_id = if anchors.contains(&point) { anchor_omt_id } else { tunnel_omt_id };
        let source = builder.resolve_parcel(local_x, local_y, 0);
        let parcel = VerticalSemanticCatalog::create_placed_parcel(
            omt_id,
            &source,
            z,
            builder.profile.active_region.default_surface_landcover,
        );
        builder.set_parcel(local_x, local_y, z, parcel);
    }
    Ok(())
}

fn create_boundary_node(
    builder: &MacroChunkBuilder,
    anchor: WorldCellCoordinate,
    z: i32,
    rng: &mut DeterministicRandomSource,
) -> WorldCellCoordinate {
    match rng.next_int(0, 4) {
        0 => builder.to_world_coordinate(anchor.x % builder.width(), 0, z),
        1 => builder.to_world_coordinate(anchor.x % builder.width(), builder.height() - 1, z),
        2 => builder.to_world_coordinate(0, anchor.y % builder.height(), z),
        _ => builder.to_world_coordinate(builder.width() - 1, anchor.y % builder.height(), z),
    }
}

fn manhattan_distance(a: WorldCellCoordinate, b: WorldCellCoordinate) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()
}

fn find_primary_settlement_row(builder: &MacroChunkBuilder) -> Option<i32> {
    for y in 0..builder.height() {
        for x in 0..builder.width() {
            let parcel = builder.resolve_parcel(x, y, 0);
            if matches!(parcel.parcel, ParcelType::TownCenter | ParcelType::SkyscraperBase) {
                return Some(y);
            }
        }
    }
    None
}

fn find_primary_settlement_column(builder: &MacroChunkBuilder) -> Option<i32> {
    for y in 0..builder.height() {
        for x in 0..builder.width() {
            let parcel = builder.resolve_parcel(x, y, 0);
            if matches!(parcel.parcel, ParcelType::TownCenter | ParcelType::SkyscraperBase) {
                return Some(x);
            }
        }
    }
    None
}

fn add_linear_infrastructure(
    builder: &mut MacroChunkBuilder,
    kind: FeatureKind,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    ct: &CancellationToken,
) -> Result<()> {
    let graph = LinearFeatureGraph::create_line(
        kind,
        builder.to_world_coordinate(start_x, start_y, 0),
        builder.to_world_coordinate(end_x, end_y, 0),
        kind == FeatureKind::Highway,
    );
    builder.features.extend(graph.segments.clone());
    for point in graph.rasterize() {
        check_cancelled(ct)?;
        let local_x = positive_mod(point.x, builder.width());
        let local_y = positive_mod(point.y, builder.height());
        let parcel = builder.get_or_create_parcel(local_x, local_y, 0);
        let mut infrastructure = if kind == FeatureKind::Highway {
            InfrastructureType::Highway
        } else {
            InfrastructureType::Road
        };
        if parcel.flags.contains(CellFlags::WATERY) {
            infrastructure = InfrastructureType::Bridge;
            parcel.parcel = ParcelType::BridgeSpan;
            parcel.flags.insert(CellFlags::OVERHEAD_CONNECTION);
        }
        parcel.infrastructure = infrastructure;
        parcel.flags.insert(CellFlags::HAS_ROAD | CellFlags::TRAVERSABLE);
        MacroChunkBuilder::append_layer(
            parcel,
            SemanticLayer::create(
                SemanticLayerKind::Infrastructure,
                SemanticValue::for_infrastructure(infrastructure),
                40,
                LayerBlendMode::Overlay,
                CellFlags::HAS_ROAD,
                kind.as_str().to_ascii_lowercase(),
            ),
        );
    }
    Ok(())
}

fn place_manholes(builder: &mut MacroChunkBuilder) {
    let mut candidates = Vec::new();
    for y in 1..builder.height() - 1 {
        for x in 1..builder.width() - 1 {
            let parcel = builder.resolve_parcel(x, y, 0);
            if !matches!(parcel.infrastructure, InfrastructureType::Road | InfrastructureType::Highway) {
                continue;
            }
            if parcel.infrastructure == InfrastructureType::Bridge || parcel.flags.contains(CellFlags::WATERY) {
                continue;
            }
            let mut score = count_adjacent_roads(builder, x, y) * 10;
            if parcel.flags.contains(CellFlags::SETTLEMENT) {
                score += 5;
            }
            candidates.push((x, y, score));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| {
                PhaseSeedDeriver::hash_coordinate(builder.profile.world_seed, left.0, left.1, 0)
                    .cmp(&PhaseSeedDeriver::hash_coordinate(builder.profile.world_seed, right.0, right.1, 0))
            })
    });

    for (x, y, _) in candidates.into_iter().take(3) {
        let parcel = builder.get_or_create_parcel(x, y, 0);
        parcel.parcel = ParcelType::Manhole;
        parcel.flags.insert(CellFlags::UNDERGROUND_CONNECTION | CellFlags::TRAVERSABLE);
        MacroChunkBuilder::append_layer(
            parcel,
            SemanticLayer::create(
                SemanticLayerKind::Parcel,
                SemanticValue::for_parcel(ParcelType::Manhole),
                41,
                LayerBlendMode::Overlay,
                CellFlags::UNDERGROUND_CONNECTION,
                "manhole-surface",
            ),
        );
    }
}

fn count_adjacent_roads(builder: &MacroChunkBuilder, x: i32, y: i32) -> i32 {
    let mut count = 0;
    for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
        let neighbor = builder.resolve_parcel(x + dx, y + dy, 0);
        if neighbor.flags.contains(CellFlags::HAS_ROAD) {
            count += 1;
        }
    }
    count
}

fn has_adjacent(
    builder: &MacroChunkBuilder,
    x: i32,
    y: i32,
    predicate: impl Fn(&SemanticParcel) -> bool,
) -> bool {
    for delta_y in -1..=1 {
        for delta_x in -1..=1 {
            if delta_x == 0 && delta_y == 0 {
                continue;
            }
            if !builder.is_within_bounds(x + delta_x, y + delta_y, 0) {
                continue;
            }
            let candidate = builder.resolve_parcel(x + delta_x, y + delta_y, 0);
            if predicate(&candidate) {
                return true;
            }
        }
    }
    false
}

fn check_cancelled(ct: &CancellationToken) -> Result<()> {
    if ct.is_cancelled() {
        Err(WorldGenError::Cancelled)
    } else {
        Ok(())
    }
}
