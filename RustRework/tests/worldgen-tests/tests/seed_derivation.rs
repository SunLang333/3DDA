mod rustrework_worldgen {
    pub use worldgen_core::*;
}

use rustrework_worldgen::{LocalChunkKey, MacroChunkKey, PhaseSeedDeriver, WorldBootstrap};

#[test]
fn phase_seed_derivation_is_stable_and_phase_partitioned() {
    let macro_key = MacroChunkKey { x: 12, y: -8 };
    let profile = WorldBootstrap::create_default_profile(123_456_789);
    let local_key = LocalChunkKey::from_world_coordinates(7, -3, 1, profile.dimensions);

    let macro_hydrology_a = PhaseSeedDeriver::hash64(123_456_789, macro_key, "macro:hydrology");
    let macro_hydrology_b = PhaseSeedDeriver::hash64(123_456_789, macro_key, "macro:hydrology");
    let macro_settlement = PhaseSeedDeriver::hash64(123_456_789, macro_key, "macro:settlement");
    let local_realization_a = PhaseSeedDeriver::hash64_local(123_456_789, &local_key, "local:realization");
    let local_realization_b = PhaseSeedDeriver::hash64_local(123_456_789, &local_key, "local:realization");

    assert_eq!(macro_hydrology_a, macro_hydrology_b);
    assert_eq!(local_realization_a, local_realization_b);
    assert_ne!(macro_hydrology_a, macro_settlement);
    assert_ne!(macro_hydrology_a, local_realization_a);
    assert_ne!(macro_hydrology_a, 0);
    assert_ne!(local_realization_a, 0);
}
