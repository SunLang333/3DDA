use crate::domain::{LocalChunkKey, MacroChunkKey, WorldProfile};

#[derive(Debug, Clone)]
pub struct DeterministicRandomSource {
    root_seed: u64,
    state: u64,
    increment: u64,
}

pub struct PhaseSeedDeriver;

impl PhaseSeedDeriver {
    pub const OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
    pub const PRIME: u64 = 1_099_511_628_211;

    pub fn hash64(world_seed: u64, key: MacroChunkKey, phase_name: &str) -> u64 {
        let mut hash = Self::OFFSET_BASIS;
        Self::add_u64(&mut hash, world_seed);
        Self::add_i32(&mut hash, key.x);
        Self::add_i32(&mut hash, key.y);
        Self::add_string(&mut hash, phase_name);
        hash
    }

    pub fn hash64_local(world_seed: u64, key: &LocalChunkKey, phase_name: &str) -> u64 {
        let mut hash = Self::OFFSET_BASIS;
        Self::add_u64(&mut hash, world_seed);
        Self::add_i32(&mut hash, key.macro_key.x);
        Self::add_i32(&mut hash, key.macro_key.y);
        Self::add_i32(&mut hash, key.x);
        Self::add_i32(&mut hash, key.y);
        Self::add_i32(&mut hash, key.z);
        Self::add_string(&mut hash, phase_name);
        hash
    }

    pub fn hash_seed(seed: u64, phase_name: &str) -> u64 {
        let mut hash = Self::OFFSET_BASIS;
        Self::add_u64(&mut hash, seed);
        Self::add_string(&mut hash, phase_name);
        hash
    }

    pub fn hash_coordinate(seed: u64, x: i32, y: i32, z: i32) -> u64 {
        let mut hash = Self::OFFSET_BASIS;
        Self::add_u64(&mut hash, seed);
        Self::add_i32(&mut hash, x);
        Self::add_i32(&mut hash, y);
        Self::add_i32(&mut hash, z);
        hash
    }

    pub fn hash_text(text: &str) -> u64 {
        let mut hash = Self::OFFSET_BASIS;
        Self::add_string(&mut hash, text);
        hash
    }

    pub fn hash_bytes(bytes: &[u8]) -> u64 {
        let mut hash = Self::OFFSET_BASIS;
        for value in bytes {
            hash ^= u64::from(*value);
            hash = hash.wrapping_mul(Self::PRIME);
        }
        hash
    }

    pub fn to_unit_double(hash: u64) -> f64 {
        ((hash >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    fn add_u64(hash: &mut u64, value: u64) {
        Self::add_bytes(hash, &value.to_le_bytes());
    }

    fn add_i32(hash: &mut u64, value: i32) {
        Self::add_bytes(hash, &value.to_le_bytes());
    }

    fn add_string(hash: &mut u64, value: &str) {
        Self::add_bytes(hash, value.as_bytes());
    }

    fn add_bytes(hash: &mut u64, bytes: &[u8]) {
        for value in bytes {
            *hash ^= u64::from(*value);
            *hash = hash.wrapping_mul(Self::PRIME);
        }
    }
}

impl DeterministicRandomSource {
    pub fn new(seed: u64) -> Self {
        let mut rng = Self {
            root_seed: seed,
            state: 0,
            increment: (Self::split_mix64(seed) << 1) | 1,
        };
        let _ = rng.next_u32();
        rng.state = rng
            .state
            .wrapping_add(Self::split_mix64(seed ^ 0x9E37_79B9_7F4A_7C15));
        let _ = rng.next_u32();
        rng
    }

    pub fn next_int(&mut self, min_inclusive: i32, max_exclusive: i32) -> i32 {
        assert!(max_exclusive > min_inclusive, "max_exclusive must be greater than min_inclusive");
        let range = (max_exclusive - min_inclusive) as u32;
        min_inclusive + (self.next_u32() % range) as i32
    }

    pub fn next_double(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    pub fn next_bool(&mut self, probability: f64) -> bool {
        assert!((0.0..=1.0).contains(&probability), "probability must be within 0..=1");
        self.next_double() <= probability
    }

    pub fn next_u64(&mut self) -> u64 {
        (u64::from(self.next_u32()) << 32) | u64::from(self.next_u32())
    }

    pub fn fork(&self, stream_name: &str) -> Self {
        Self::new(PhaseSeedDeriver::hash_seed(self.root_seed, stream_name))
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            let swap_index = self.next_int(0, (index + 1) as i32) as usize;
            items.swap(index, swap_index);
        }
    }

    pub fn for_macro_phase(profile: &WorldProfile, key: MacroChunkKey, phase_name: &str) -> Self {
        Self::new(PhaseSeedDeriver::hash64(profile.world_seed, key, phase_name))
    }

    pub fn for_local_phase(profile: &WorldProfile, key: &LocalChunkKey, phase_name: &str) -> Self {
        Self::new(PhaseSeedDeriver::hash64_local(profile.world_seed, key, phase_name))
    }

    fn next_u32(&mut self) -> u32 {
        let previous_state = self.state;
        self.state = previous_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.increment);
        let xorshifted = (((previous_state >> 18) ^ previous_state) >> 27) as u32;
        let rotation = (previous_state >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    fn split_mix64(mut value: u64) -> u64 {
        value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}
