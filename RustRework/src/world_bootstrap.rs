use uuid::Uuid;

use crate::domain::{
    ChunkDimensions, LandcoverType, LocalMaterialType, RegionProfile, SpecialCounter, SpecialType,
    WorldProfile, ZBounds,
};
use crate::randomization::PhaseSeedDeriver;

pub struct WorldBootstrap;

impl WorldBootstrap {
    pub fn create_default_profile(seed: u64) -> WorldProfile {
        let region = Self::create_default_region();
        let z_bounds = ZBounds::cdda();
        let content_hash = PhaseSeedDeriver::hash_text(&format!(
            "{}:{}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{}:{}",
            region.region_id,
            region.default_surface_landcover.as_str(),
            region.urban_bias,
            region.forest_bias,
            region.hydrology_bias,
            region.settlement_threshold,
            region.road_density,
            region.special_placement_bias,
            z_bounds.min,
            z_bounds.max,
        ));

        WorldProfile {
            world_id: Uuid::new_v4(),
            world_seed: seed,
            generation_version: 2,
            active_region_id: region.region_id.clone(),
            overlay_region_ids: Vec::new(),
            content_hash: format!("{content_hash:016X}"),
            special_counters: vec![
                SpecialCounter {
                    type_: SpecialType::ResearchOutpost,
                    count: 0,
                },
                SpecialCounter {
                    type_: SpecialType::BridgeControl,
                    count: 0,
                },
            ],
            dimensions: ChunkDimensions {
                macro_width: 16,
                macro_height: 16,
                min_z: z_bounds.min,
                max_z: z_bounds.max,
                local_width: 8,
                local_height: 8,
                local_depth: 4,
            },
            active_region: region,
            created_at_utc: crate::domain::utc_now_string(),
            z_bounds,
        }
    }

    pub fn create_default_region() -> RegionProfile {
        RegionProfile {
            region_id: "temperate-research-frontier".to_string(),
            default_surface_landcover: LandcoverType::Plains,
            urban_bias: 0.56,
            forest_bias: 0.61,
            hydrology_bias: 0.51,
            settlement_threshold: 0.64,
            road_density: 0.47,
            special_placement_bias: 0.39,
            regional_road_material: LocalMaterialType::Asphalt,
            regional_settlement_floor_material: LocalMaterialType::Concrete,
            regional_bridge_material: LocalMaterialType::Metal,
            allowed_specials: vec![SpecialType::ResearchOutpost, SpecialType::BridgeControl],
        }
    }
}
