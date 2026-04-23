use std::collections::BTreeMap;

use tokio_util::sync::CancellationToken;

use crate::abstractions::LocalChunkRealizer;
use crate::diagnostics::compute_local_hash;
use crate::domain::{
    ElevationMode, EnvironmentalField, FieldType, InfrastructureType, ItemKind, LandcoverType,
    LocalCell, LocalCellFlags, LocalChunk, LocalChunkKey, LocalChunkProvenance,
    LocalMaterialType, LocalPropType, MacroChunk, ParcelType, PlacedItem, PlacedProp,
    RegionProfile, SemanticLayerKind, SemanticParcel, SpawnMarker, SpawnType, WorldProfile,
};
use crate::error::{Result, WorldGenError};
use crate::randomization::{DeterministicRandomSource, PhaseSeedDeriver};

#[derive(Debug, Clone)]
pub struct LocalGenerationContext {
    pub key: LocalChunkKey,
    pub profile: WorldProfile,
    pub owning_macro_chunk: MacroChunk,
    pub current_cell: SemanticParcel,
    pub neighbors: BTreeMap<String, Option<SemanticParcel>>,
}

#[derive(Debug, Default, Clone)]
pub struct LocalRealizer;

impl LocalRealizer {
    pub fn realize(
        &self,
        key: &LocalChunkKey,
        owning_macro_chunk: &MacroChunk,
        profile: &WorldProfile,
        ct: &CancellationToken,
    ) -> Result<LocalChunk> {
        let local_x = key.local_x_within_macro(profile.dimensions);
        let local_y = key.local_y_within_macro(profile.dimensions);
        let current_cell = owning_macro_chunk.resolve_parcel(local_x, local_y, key.z);
        let context = LocalGenerationContext {
            key: key.clone(),
            profile: profile.clone(),
            owning_macro_chunk: owning_macro_chunk.clone(),
            current_cell,
            neighbors: build_neighbor_map(owning_macro_chunk, local_x, local_y, key.z),
        };
        let mut chunk = realize_internal(&context, ct)?;
        chunk.provenance.canonical_hash = compute_local_hash(&chunk);
        Ok(chunk)
    }
}

impl LocalChunkRealizer for LocalRealizer {
    fn realize(
        &self,
        key: &LocalChunkKey,
        owning_macro_chunk: &MacroChunk,
        profile: &WorldProfile,
        ct: &CancellationToken,
    ) -> Result<LocalChunk> {
        LocalRealizer::realize(self, key, owning_macro_chunk, profile, ct)
    }
}

fn realize_internal(context: &LocalGenerationContext, ct: &CancellationToken) -> Result<LocalChunk> {
    let dimensions = context.profile.dimensions;
    let local_seed = PhaseSeedDeriver::hash64_local(context.profile.world_seed, &context.key, "local:realization");
    let mut rng = DeterministicRandomSource::new(local_seed);
    let mut chunk = LocalChunk {
        key: context.key.clone(),
        width: dimensions.local_width,
        height: dimensions.local_height,
        depth: dimensions.local_depth,
        cells: vec![
            LocalCell {
                material: LocalMaterialType::Air,
                flags: LocalCellFlags::NONE,
                elevation: 0,
                moisture: 0,
            };
            dimensions.local_width * dimensions.local_height * dimensions.local_depth
        ],
        props: Vec::new(),
        items: Vec::new(),
        fields: Vec::new(),
        spawns: Vec::new(),
        is_uniform: false,
        is_dirty: false,
        provenance: LocalChunkProvenance {
            schema_version: 2,
            generation_version: context.profile.generation_version,
            content_hash: context.profile.content_hash.clone(),
            local_seed,
            source_macro_canonical_hash: context.owning_macro_chunk.provenance.canonical_hash,
            is_mutated: false,
            mutation_count: 0,
            generated_at_utc: crate::domain::utc_now_string(),
            last_mutation_at_utc: None,
            canonical_hash: 0,
        },
    };

    initialize_base_cells(&mut chunk, &context.current_cell, ct)?;
    replay_semantic_layers(&mut chunk, context, &mut rng, ct)?;
    apply_neighbor_aware_refinement(&mut chunk, context);
    chunk.is_uniform = determine_uniformity(&chunk);
    Ok(chunk)
}

fn build_neighbor_map(
    owning_macro_chunk: &MacroChunk,
    x: i32,
    y: i32,
    z: i32,
) -> BTreeMap<String, Option<SemanticParcel>> {
    BTreeMap::from([
        (
            "north".to_string(),
            if y > 0 {
                Some(owning_macro_chunk.resolve_parcel(x, y - 1, z))
            } else {
                None
            },
        ),
        (
            "south".to_string(),
            if y < owning_macro_chunk.height - 1 {
                Some(owning_macro_chunk.resolve_parcel(x, y + 1, z))
            } else {
                None
            },
        ),
        (
            "west".to_string(),
            if x > 0 {
                Some(owning_macro_chunk.resolve_parcel(x - 1, y, z))
            } else {
                None
            },
        ),
        (
            "east".to_string(),
            if x < owning_macro_chunk.width - 1 {
                Some(owning_macro_chunk.resolve_parcel(x + 1, y, z))
            } else {
                None
            },
        ),
        (
            "below".to_string(),
            if z > owning_macro_chunk.min_z() {
                Some(owning_macro_chunk.resolve_parcel(x, y, z - 1))
            } else {
                None
            },
        ),
        (
            "above".to_string(),
            if z < owning_macro_chunk.max_z() {
                Some(owning_macro_chunk.resolve_parcel(x, y, z + 1))
            } else {
                None
            },
        ),
    ])
}

fn initialize_base_cells(chunk: &mut LocalChunk, macro_cell: &SemanticParcel, ct: &CancellationToken) -> Result<()> {
    for z in 0..chunk.depth {
        for y in 0..chunk.height {
            for x in 0..chunk.width {
                check_cancelled(ct)?;
                let cell = match macro_cell.elevation_mode {
                    ElevationMode::Underground => LocalCell {
                        material: LocalMaterialType::Stone,
                        flags: LocalCellFlags::SOLID | LocalCellFlags::BLOCKS_SIGHT,
                        elevation: z as u8,
                        moisture: 0,
                    },
                    ElevationMode::Elevated => LocalCell {
                        material: LocalMaterialType::Air,
                        flags: LocalCellFlags::NONE,
                        elevation: z as u8,
                        moisture: 0,
                    },
                    _ => {
                        if z == 0 {
                            LocalCell {
                                material: LocalMaterialType::Soil,
                                flags: LocalCellFlags::SOLID,
                                elevation: 0,
                                moisture: 25,
                            }
                        } else {
                            LocalCell {
                                material: LocalMaterialType::Air,
                                flags: LocalCellFlags::NONE,
                                elevation: z as u8,
                                moisture: 0,
                            }
                        }
                    }
                };
                let index = chunk.get_index(x, y, z);
                chunk.cells[index] = cell;
            }
        }
    }
    Ok(())
}

fn replay_semantic_layers(
    chunk: &mut LocalChunk,
    context: &LocalGenerationContext,
    rng: &mut DeterministicRandomSource,
    ct: &CancellationToken,
) -> Result<()> {
    let mut layers = context.current_cell.layers.clone();
    layers.sort_by_key(|layer| (layer.priority, layer.kind));
    for layer in layers {
        check_cancelled(ct)?;
        match layer.kind {
            SemanticLayerKind::Landform => apply_landform(chunk, &context.current_cell),
            SemanticLayerKind::Hydrology => apply_hydrology(chunk),
            SemanticLayerKind::Vegetation => apply_vegetation(chunk, &context.current_cell, rng),
            SemanticLayerKind::Infrastructure => {
                apply_infrastructure(chunk, &context.current_cell, &context.profile.active_region, rng)
            }
            SemanticLayerKind::Parcel => apply_parcel(chunk, &context.current_cell, &context.profile.active_region, rng),
            SemanticLayerKind::Underground => apply_underground(chunk, &context.current_cell),
            SemanticLayerKind::Overhead => apply_overhead(chunk, &context.current_cell, &context.profile.active_region),
            SemanticLayerKind::PostGeneration => {}
        }
    }
    Ok(())
}

fn apply_landform(chunk: &mut LocalChunk, macro_cell: &SemanticParcel) {
    match macro_cell.landcover {
        LandcoverType::Plains => fill_top_layer(chunk, LocalMaterialType::Grass, LocalCellFlags::WALKABLE, 20),
        LandcoverType::Forest => fill_top_layer(chunk, LocalMaterialType::Grass, LocalCellFlags::WALKABLE, 32),
        LandcoverType::Wetland => fill_top_layer(chunk, LocalMaterialType::Soil, LocalCellFlags::WALKABLE, 80),
        LandcoverType::Ravine => lower_center(chunk, LocalMaterialType::Stone),
        LandcoverType::Settlement => fill_top_layer(chunk, LocalMaterialType::Concrete, LocalCellFlags::WALKABLE, 5),
        LandcoverType::Subterranean => hollow_interior(chunk, LocalMaterialType::Stone, LocalMaterialType::Air),
        LandcoverType::OpenAir => fill_all(chunk, LocalMaterialType::Air, LocalCellFlags::NONE, 0),
        LandcoverType::Water => {}
    }
}

fn apply_hydrology(chunk: &mut LocalChunk) {
    for y in 0..chunk.height {
        for x in 0..chunk.width {
            let index = chunk.get_index(x, y, 0);
            chunk.cells[index] = LocalCell {
                material: LocalMaterialType::Water,
                flags: LocalCellFlags::LIQUID,
                elevation: 0,
                moisture: 100,
            };
        }
    }
}

fn apply_vegetation(
    chunk: &mut LocalChunk,
    macro_cell: &SemanticParcel,
    rng: &mut DeterministicRandomSource,
) {
    if macro_cell.landcover != LandcoverType::Forest {
        return;
    }

    let tree_count = rng.next_int(15, 35);
    for _ in 0..tree_count {
        let x = rng.next_int(1, chunk.width as i32 - 1) as u8;
        let y = rng.next_int(1, chunk.height as i32 - 1) as u8;
        chunk.add_generated_prop(PlacedProp {
            type_: LocalPropType::Tree,
            x,
            y,
            z: 0,
        });
    }

    let supplement_count = rng.next_int(1, 5);
    for _ in 0..supplement_count {
        chunk.add_generated_item(PlacedItem {
            type_: ItemKind::Supplement,
            quantity: rng.next_int(1, 3),
            x: rng.next_int(1, chunk.width as i32 - 1) as u8,
            y: rng.next_int(1, chunk.height as i32 - 1) as u8,
            z: 0,
        });
    }
}

fn apply_infrastructure(
    chunk: &mut LocalChunk,
    macro_cell: &SemanticParcel,
    region: &RegionProfile,
    rng: &mut DeterministicRandomSource,
) {
    match macro_cell.infrastructure {
        InfrastructureType::Road | InfrastructureType::Highway => {
            draw_road_stripe(
                chunk,
                region.regional_road_material,
                macro_cell.infrastructure == InfrastructureType::Highway,
            );
            if rng.next_bool(0.35) {
                chunk.add_generated_prop(PlacedProp {
                    type_: LocalPropType::StreetLight,
                    x: (chunk.width / 2) as u8,
                    y: 1,
                    z: 0,
                });
            }
        }
        InfrastructureType::Bridge => {
            draw_road_stripe(chunk, region.regional_road_material, true);
            draw_bridge_deck(chunk, region.regional_bridge_material);
        }
        InfrastructureType::Sewer | InfrastructureType::UtilityTunnel | InfrastructureType::Subway => {
            hollow_interior(chunk, LocalMaterialType::Stone, LocalMaterialType::Air);
            draw_tunnel_floor(chunk, LocalMaterialType::Concrete);
        }
        InfrastructureType::None => {}
    }
}

fn apply_parcel(
    chunk: &mut LocalChunk,
    macro_cell: &SemanticParcel,
    region: &RegionProfile,
    rng: &mut DeterministicRandomSource,
) {
    match macro_cell.parcel {
        ParcelType::TownCenter
        | ParcelType::ResidentialBlock
        | ParcelType::SkyscraperBase
        | ParcelType::SkyscraperFloor => {
            build_simple_structure(chunk, region.regional_settlement_floor_material, LocalMaterialType::Wood);
            if matches!(macro_cell.parcel, ParcelType::TownCenter | ParcelType::SkyscraperBase) {
                chunk.add_generated_item(PlacedItem {
                    type_: ItemKind::Food,
                    quantity: 4,
                    x: (chunk.width / 2) as u8,
                    y: (chunk.height / 2) as u8,
                    z: 1,
                });
                if rng.next_bool(0.25) {
                    chunk.add_generated_item(PlacedItem {
                        type_: ItemKind::Weapon,
                        quantity: 1,
                        x: (chunk.width / 2 + 1) as u8,
                        y: (chunk.height / 2) as u8,
                        z: 1,
                    });
                }
                if rng.next_bool(0.5) {
                    chunk.add_generated_item(PlacedItem {
                        type_: ItemKind::Supplement,
                        quantity: rng.next_int(1, 3),
                        x: (chunk.width / 2 - 1) as u8,
                        y: (chunk.height / 2) as u8,
                        z: 1,
                    });
                }
            }
        }
        ParcelType::ResearchOutpost => {
            build_simple_structure(chunk, LocalMaterialType::Concrete, LocalMaterialType::Metal);
            chunk.add_generated_prop(PlacedProp {
                type_: LocalPropType::ControlConsole,
                x: (chunk.width / 2) as u8,
                y: 1,
                z: 1,
            });
            chunk.add_generated_prop(PlacedProp {
                type_: LocalPropType::SupplyCrate,
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 1,
            });
            chunk.add_generated_item(PlacedItem {
                type_: ItemKind::MedicalSupplies,
                quantity: 2 + rng.next_int(0, 3),
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 1,
            });
            chunk.add_generated_item(PlacedItem {
                type_: ItemKind::Weapon,
                quantity: rng.next_int(1, 3),
                x: (chunk.width / 2 + 1) as u8,
                y: (chunk.height / 2 + 1) as u8,
                z: 1,
            });
            chunk.add_generated_field(EnvironmentalField {
                type_: FieldType::Fog,
                intensity: 1,
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 1,
            });
        }
        ParcelType::BridgeSpan | ParcelType::BridgeRoof => {
            draw_bridge_deck(chunk, region.regional_bridge_material);
            chunk.add_generated_prop(PlacedProp {
                type_: LocalPropType::SupportColumn,
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 0,
            });
        }
        ParcelType::BridgeSupport => {
            hollow_interior(chunk, LocalMaterialType::Metal, LocalMaterialType::Air);
            chunk.add_generated_prop(PlacedProp {
                type_: LocalPropType::SupportColumn,
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 1,
            });
        }
        ParcelType::Basement
        | ParcelType::UtilityNode
        | ParcelType::SewerTunnel
        | ParcelType::SubwayTunnel
        | ParcelType::SubwayStation => {
            hollow_interior(chunk, LocalMaterialType::Stone, LocalMaterialType::Air);
            draw_tunnel_floor(chunk, LocalMaterialType::Concrete);
            chunk.add_generated_spawn(SpawnMarker {
                type_: SpawnType::Scavenger,
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 1,
            });
        }
        ParcelType::Manhole => {
            draw_road_stripe(chunk, region.regional_road_material, false);
            chunk.add_generated_prop(PlacedProp {
                type_: LocalPropType::SupportColumn,
                x: (chunk.width / 2) as u8,
                y: (chunk.height / 2) as u8,
                z: 0,
            });
        }
        ParcelType::Empty | ParcelType::Field | ParcelType::ForestStand | ParcelType::River => {}
    }
}

fn apply_underground(chunk: &mut LocalChunk, macro_cell: &SemanticParcel) {
    if macro_cell.elevation_mode != ElevationMode::Underground {
        return;
    }
    hollow_interior(chunk, LocalMaterialType::Stone, LocalMaterialType::Air);
    draw_tunnel_floor(chunk, LocalMaterialType::Concrete);
}

fn apply_overhead(chunk: &mut LocalChunk, macro_cell: &SemanticParcel, region: &RegionProfile) {
    if macro_cell.infrastructure != InfrastructureType::Bridge && macro_cell.parcel != ParcelType::BridgeRoof {
        return;
    }
    fill_all(chunk, LocalMaterialType::Air, LocalCellFlags::NONE, 0);
    draw_bridge_deck(chunk, region.regional_bridge_material);
}

fn apply_neighbor_aware_refinement(chunk: &mut LocalChunk, context: &LocalGenerationContext) {
    if let Some(Some(north)) = context.neighbors.get("north") {
        if matches!(north.infrastructure, InfrastructureType::Road | InfrastructureType::Highway) {
            draw_road_continuation_marker(chunk, north.infrastructure);
        }
    }

    if let Some(Some(east)) = context.neighbors.get("east") {
        if east.landcover == LandcoverType::Water {
            chunk.add_generated_field(EnvironmentalField {
                type_: FieldType::Steam,
                intensity: 1,
                x: (chunk.width - 1) as u8,
                y: (chunk.height / 2) as u8,
                z: 0,
            });
        }
    }
}

fn determine_uniformity(chunk: &LocalChunk) -> bool {
    if !chunk.props.is_empty() || !chunk.items.is_empty() || !chunk.fields.is_empty() || !chunk.spawns.is_empty() {
        return false;
    }
    let first = &chunk.cells[0];
    chunk.cells.iter().all(|cell| {
        cell.material == first.material && cell.flags == first.flags && cell.moisture == first.moisture
    })
}

fn fill_top_layer(
    chunk: &mut LocalChunk,
    material: LocalMaterialType,
    flags: LocalCellFlags,
    moisture: u8,
) {
    for y in 0..chunk.height {
        for x in 0..chunk.width {
            let index = chunk.get_index(x, y, 0);
            chunk.cells[index] = LocalCell {
                material,
                flags,
                elevation: 0,
                moisture,
            };
        }
    }
}

fn fill_all(chunk: &mut LocalChunk, material: LocalMaterialType, flags: LocalCellFlags, moisture: u8) {
    for z in 0..chunk.depth {
        for y in 0..chunk.height {
            for x in 0..chunk.width {
                let index = chunk.get_index(x, y, z);
                chunk.cells[index] = LocalCell {
                    material,
                    flags,
                    elevation: z as u8,
                    moisture,
                };
            }
        }
    }
}

fn lower_center(chunk: &mut LocalChunk, material: LocalMaterialType) {
    fill_top_layer(chunk, LocalMaterialType::Grass, LocalCellFlags::WALKABLE, 12);
    let start_x = chunk.width / 4;
    let end_x = chunk.width - start_x;
    let start_y = chunk.height / 4;
    let end_y = chunk.height - start_y;
    for y in start_y..end_y {
        for x in start_x..end_x {
            let index = chunk.get_index(x, y, 0);
            chunk.cells[index] = LocalCell {
                material,
                flags: LocalCellFlags::SOLID,
                elevation: 0,
                moisture: 8,
            };
        }
    }
}

fn draw_road_stripe(chunk: &mut LocalChunk, material: LocalMaterialType, wide: bool) {
    fill_top_layer(chunk, LocalMaterialType::Grass, LocalCellFlags::WALKABLE, 18);
    let start_y = if wide { (chunk.height / 2).saturating_sub(1) } else { chunk.height / 2 };
    let end_y = if wide { start_y + 2 } else { start_y + 1 };
    for y in start_y..end_y.min(chunk.height) {
        for x in 0..chunk.width {
            let index = chunk.get_index(x, y, 0);
            chunk.cells[index] = LocalCell {
                material,
                flags: LocalCellFlags::WALKABLE,
                elevation: 0,
                moisture: 2,
            };
        }
    }
}

fn draw_bridge_deck(chunk: &mut LocalChunk, deck_material: LocalMaterialType) {
    fill_all(chunk, LocalMaterialType::Air, LocalCellFlags::NONE, 0);
    let deck_z = (chunk.depth - 2).max(1);
    for y in 0..chunk.height {
        for x in 0..chunk.width {
            let index = chunk.get_index(x, y, deck_z);
            chunk.cells[index] = LocalCell {
                material: deck_material,
                flags: LocalCellFlags::WALKABLE | LocalCellFlags::STRUCTURAL,
                elevation: deck_z as u8,
                moisture: 0,
            };
        }
    }
}

fn hollow_interior(chunk: &mut LocalChunk, wall_material: LocalMaterialType, interior_material: LocalMaterialType) {
    fill_all(
        chunk,
        wall_material,
        LocalCellFlags::SOLID | LocalCellFlags::BLOCKS_SIGHT | LocalCellFlags::STRUCTURAL,
        0,
    );
    if chunk.depth < 2 || chunk.height < 2 || chunk.width < 2 {
        return;
    }
    for z in 1..chunk.depth - 1 {
        for y in 1..chunk.height - 1 {
            for x in 1..chunk.width - 1 {
                let index = chunk.get_index(x, y, z);
                chunk.cells[index] = LocalCell {
                    material: interior_material,
                    flags: LocalCellFlags::NONE,
                    elevation: z as u8,
                    moisture: 0,
                };
            }
        }
    }
}

fn draw_tunnel_floor(chunk: &mut LocalChunk, material: LocalMaterialType) {
    let floor_z = 1.min(chunk.depth.saturating_sub(1));
    for y in 1..chunk.height.saturating_sub(1) {
        for x in 1..chunk.width.saturating_sub(1) {
            let index = chunk.get_index(x, y, floor_z);
            chunk.cells[index] = LocalCell {
                material,
                flags: LocalCellFlags::WALKABLE,
                elevation: floor_z as u8,
                moisture: 0,
            };
        }
    }
}

fn build_simple_structure(
    chunk: &mut LocalChunk,
    floor_material: LocalMaterialType,
    wall_material: LocalMaterialType,
) {
    fill_top_layer(chunk, LocalMaterialType::Grass, LocalCellFlags::WALKABLE, 10);
    let floor_z = 1;
    for y in 1..chunk.height - 1 {
        for x in 1..chunk.width - 1 {
            let index = chunk.get_index(x, y, floor_z);
            chunk.cells[index] = LocalCell {
                material: floor_material,
                flags: LocalCellFlags::WALKABLE,
                elevation: floor_z as u8,
                moisture: 0,
            };
        }
    }

    let wall_z = floor_z + 1;
    for y in 1..chunk.height - 1 {
        let left_index = chunk.get_index(1, y, wall_z);
        let right_index = chunk.get_index(chunk.width - 2, y, wall_z);
        chunk.cells[left_index] = LocalCell {
            material: wall_material,
            flags: LocalCellFlags::SOLID | LocalCellFlags::BLOCKS_SIGHT | LocalCellFlags::STRUCTURAL,
            elevation: wall_z as u8,
            moisture: 0,
        };
        chunk.cells[right_index] = LocalCell {
            material: wall_material,
            flags: LocalCellFlags::SOLID | LocalCellFlags::BLOCKS_SIGHT | LocalCellFlags::STRUCTURAL,
            elevation: wall_z as u8,
            moisture: 0,
        };
    }
    for x in 1..chunk.width - 1 {
        let top_index = chunk.get_index(x, 1, wall_z);
        let bottom_index = chunk.get_index(x, chunk.height - 2, wall_z);
        chunk.cells[top_index] = LocalCell {
            material: wall_material,
            flags: LocalCellFlags::SOLID | LocalCellFlags::BLOCKS_SIGHT | LocalCellFlags::STRUCTURAL,
            elevation: wall_z as u8,
            moisture: 0,
        };
        chunk.cells[bottom_index] = LocalCell {
            material: wall_material,
            flags: LocalCellFlags::SOLID | LocalCellFlags::BLOCKS_SIGHT | LocalCellFlags::STRUCTURAL,
            elevation: wall_z as u8,
            moisture: 0,
        };
    }
}

fn draw_road_continuation_marker(chunk: &mut LocalChunk, infrastructure: InfrastructureType) {
    chunk.add_generated_prop(PlacedProp {
        type_: if infrastructure == InfrastructureType::Highway {
            LocalPropType::StreetLight
        } else {
            LocalPropType::SupportColumn
        },
        x: (chunk.width / 2) as u8,
        y: 0,
        z: 0,
    });
}

fn check_cancelled(ct: &CancellationToken) -> Result<()> {
    if ct.is_cancelled() {
        Err(WorldGenError::Cancelled)
    } else {
        Ok(())
    }
}
