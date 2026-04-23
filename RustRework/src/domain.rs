use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn utc_now_string() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LandcoverType {
    OpenAir,
    Plains,
    Forest,
    Water,
    Wetland,
    Ravine,
    Settlement,
    Subterranean,
}

impl LandcoverType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAir => "OpenAir",
            Self::Plains => "Plains",
            Self::Forest => "Forest",
            Self::Water => "Water",
            Self::Wetland => "Wetland",
            Self::Ravine => "Ravine",
            Self::Settlement => "Settlement",
            Self::Subterranean => "Subterranean",
        }
    }
}

impl Display for LandcoverType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InfrastructureType {
    None,
    Road,
    Highway,
    Bridge,
    Sewer,
    UtilityTunnel,
    Subway,
}

impl InfrastructureType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Road => "Road",
            Self::Highway => "Highway",
            Self::Bridge => "Bridge",
            Self::Sewer => "Sewer",
            Self::UtilityTunnel => "UtilityTunnel",
            Self::Subway => "Subway",
        }
    }
}

impl Display for InfrastructureType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParcelType {
    Empty,
    Field,
    ForestStand,
    River,
    TownCenter,
    ResidentialBlock,
    ResearchOutpost,
    BridgeSpan,
    UtilityNode,
    Basement,
    Manhole,
    SewerTunnel,
    SubwayTunnel,
    SubwayStation,
    SkyscraperBase,
    SkyscraperFloor,
    BridgeRoof,
    BridgeSupport,
}

impl ParcelType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::Field => "Field",
            Self::ForestStand => "ForestStand",
            Self::River => "River",
            Self::TownCenter => "TownCenter",
            Self::ResidentialBlock => "ResidentialBlock",
            Self::ResearchOutpost => "ResearchOutpost",
            Self::BridgeSpan => "BridgeSpan",
            Self::UtilityNode => "UtilityNode",
            Self::Basement => "Basement",
            Self::Manhole => "Manhole",
            Self::SewerTunnel => "SewerTunnel",
            Self::SubwayTunnel => "SubwayTunnel",
            Self::SubwayStation => "SubwayStation",
            Self::SkyscraperBase => "SkyscraperBase",
            Self::SkyscraperFloor => "SkyscraperFloor",
            Self::BridgeRoof => "BridgeRoof",
            Self::BridgeSupport => "BridgeSupport",
        }
    }
}

impl Display for ParcelType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpecialType {
    None,
    ResearchOutpost,
    BridgeControl,
}

impl SpecialType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::ResearchOutpost => "ResearchOutpost",
            Self::BridgeControl => "BridgeControl",
        }
    }
}

impl Display for SpecialType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureKind {
    River,
    Road,
    Highway,
    Sewer,
    Subway,
}

impl FeatureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::River => "River",
            Self::Road => "Road",
            Self::Highway => "Highway",
            Self::Sewer => "Sewer",
            Self::Subway => "Subway",
        }
    }
}

impl Display for FeatureKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticValueKind {
    None,
    Landcover,
    Infrastructure,
    Parcel,
    Special,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticLayerKind {
    Landform,
    Hydrology,
    Vegetation,
    Infrastructure,
    Parcel,
    PostGeneration,
    Underground,
    Overhead,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerBlendMode {
    Replace,
    Overlay,
    Refine,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Orientation {
    North,
    East,
    South,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElevationMode {
    Surface,
    Elevated,
    Underground,
    Bridge,
    Basin,
}

bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct CellFlags: u32 {
        const NONE = 0;
        const TRAVERSABLE = 1 << 0;
        const WATERY = 1 << 1;
        const HAS_ROAD = 1 << 2;
        const HAS_SPECIAL = 1 << 3;
        const UNDERGROUND_CONNECTION = 1 << 4;
        const OVERHEAD_CONNECTION = 1 << 5;
        const SETTLEMENT = 1 << 6;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct LocalCellFlags: u32 {
        const NONE = 0;
        const SOLID = 1 << 0;
        const LIQUID = 1 << 1;
        const WALKABLE = 1 << 2;
        const STRUCTURAL = 1 << 3;
        const BLOCKS_SIGHT = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LocalMaterialType {
    Air,
    Soil,
    Grass,
    Water,
    Asphalt,
    Concrete,
    Wood,
    Stone,
    Metal,
    Rubble,
}

impl LocalMaterialType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Air => "Air",
            Self::Soil => "Soil",
            Self::Grass => "Grass",
            Self::Water => "Water",
            Self::Asphalt => "Asphalt",
            Self::Concrete => "Concrete",
            Self::Wood => "Wood",
            Self::Stone => "Stone",
            Self::Metal => "Metal",
            Self::Rubble => "Rubble",
        }
    }
}

impl Display for LocalMaterialType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LocalPropType {
    Tree,
    StreetLight,
    SupplyCrate,
    ControlConsole,
    SupportColumn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemKind {
    Food,
    Scrap,
    MedicalSupplies,
    Fuel,
    Weapon,
    Supplement,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Food => "Food",
            Self::Scrap => "Scrap",
            Self::MedicalSupplies => "MedicalSupplies",
            Self::Fuel => "Fuel",
            Self::Weapon => "Weapon",
            Self::Supplement => "Supplement",
        }
    }
}

impl Display for ItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldType {
    Fog,
    Steam,
    Radiation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpawnType {
    Civilian,
    Scavenger,
    AquaticLife,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ZBounds {
    pub min: i32,
    pub max: i32,
}

impl ZBounds {
    pub fn cdda() -> Self {
        Self { min: -10, max: 10 }
    }

    pub fn layer_count(self) -> i32 {
        (self.max - self.min) + 1
    }

    pub fn contains(self, z: i32) -> bool {
        z >= self.min && z <= self.max
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChunkDimensions {
    pub macro_width: i32,
    pub macro_height: i32,
    pub min_z: i32,
    pub max_z: i32,
    pub local_width: usize,
    pub local_height: usize,
    pub local_depth: usize,
}

impl ChunkDimensions {
    pub fn z_layer_count(self) -> i32 {
        (self.max_z - self.min_z) + 1
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MacroChunkKey {
    pub x: i32,
    pub y: i32,
}

impl Display for MacroChunkKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldCellCoordinate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Display for WorldCellCoordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}_{}", self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LocalChunkKey {
    pub macro_key: MacroChunkKey,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Ord for LocalChunkKey {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.macro_key.x, self.macro_key.y, self.z, self.y, self.x)
            .cmp(&(other.macro_key.x, other.macro_key.y, other.z, other.y, other.x))
    }
}

impl PartialOrd for LocalChunkKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl LocalChunkKey {
    pub fn from_world_coordinates(world_x: i32, world_y: i32, z: i32, dimensions: ChunkDimensions) -> Self {
        Self {
            macro_key: MacroChunkKey {
                x: floor_div(world_x, dimensions.macro_width),
                y: floor_div(world_y, dimensions.macro_height),
            },
            x: positive_mod(world_x, dimensions.macro_width),
            y: positive_mod(world_y, dimensions.macro_height),
            z,
        }
    }

    pub fn owning_macro_chunk_key(&self) -> MacroChunkKey {
        self.macro_key
    }

    pub fn local_x_within_macro(&self, _dimensions: ChunkDimensions) -> i32 {
        self.x
    }

    pub fn local_y_within_macro(&self, _dimensions: ChunkDimensions) -> i32 {
        self.y
    }

    pub fn world_x(&self, dimensions: ChunkDimensions) -> i32 {
        (self.macro_key.x * dimensions.macro_width) + self.x
    }

    pub fn world_y(&self, dimensions: ChunkDimensions) -> i32 {
        (self.macro_key.y * dimensions.macro_height) + self.y
    }

    pub fn to_world_cell_coordinate(&self, dimensions: ChunkDimensions) -> WorldCellCoordinate {
        WorldCellCoordinate {
            x: self.world_x(dimensions),
            y: self.world_y(dimensions),
            z: self.z,
        }
    }
}

impl Display for LocalChunkKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}:{}_{}_{}",
            self.macro_key.x, self.macro_key.y, self.x, self.y, self.z
        )
    }
}

pub fn floor_div(value: i32, divisor: i32) -> i32 {
    let quotient = value / divisor;
    let remainder = value % divisor;
    if remainder < 0 { quotient - 1 } else { quotient }
}

pub fn positive_mod(value: i32, divisor: i32) -> i32 {
    let result = value % divisor;
    if result < 0 { result + divisor } else { result }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SemanticValue {
    pub kind: SemanticValueKind,
    pub raw_value: i32,
}

impl SemanticValue {
    pub fn none() -> Self {
        Self {
            kind: SemanticValueKind::None,
            raw_value: 0,
        }
    }

    pub fn for_landcover(value: LandcoverType) -> Self {
        Self {
            kind: SemanticValueKind::Landcover,
            raw_value: value as i32,
        }
    }

    pub fn for_infrastructure(value: InfrastructureType) -> Self {
        Self {
            kind: SemanticValueKind::Infrastructure,
            raw_value: value as i32,
        }
    }

    pub fn for_parcel(value: ParcelType) -> Self {
        Self {
            kind: SemanticValueKind::Parcel,
            raw_value: value as i32,
        }
    }

    pub fn for_special(value: SpecialType) -> Self {
        Self {
            kind: SemanticValueKind::Special,
            raw_value: value as i32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SemanticLayer {
    pub kind: SemanticLayerKind,
    pub value: SemanticValue,
    pub priority: i32,
    pub orientation: Orientation,
    pub blend_mode: LayerBlendMode,
    pub flags: CellFlags,
    pub hint: String,
}

impl SemanticLayer {
    pub fn create(
        kind: SemanticLayerKind,
        value: SemanticValue,
        priority: i32,
        blend_mode: LayerBlendMode,
        flags: CellFlags,
        hint: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            value,
            priority,
            orientation: Orientation::North,
            blend_mode,
            flags,
            hint: hint.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegionProfile {
    pub region_id: String,
    pub default_surface_landcover: LandcoverType,
    pub urban_bias: f64,
    pub forest_bias: f64,
    pub hydrology_bias: f64,
    pub settlement_threshold: f64,
    pub road_density: f64,
    pub special_placement_bias: f64,
    pub regional_road_material: LocalMaterialType,
    pub regional_settlement_floor_material: LocalMaterialType,
    pub regional_bridge_material: LocalMaterialType,
    pub allowed_specials: Vec<SpecialType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecialCounter {
    pub type_: SpecialType,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldProfile {
    pub world_id: Uuid,
    pub world_seed: u64,
    pub generation_version: i32,
    pub active_region_id: String,
    pub overlay_region_ids: Vec<String>,
    pub content_hash: String,
    pub special_counters: Vec<SpecialCounter>,
    pub dimensions: ChunkDimensions,
    pub active_region: RegionProfile,
    pub created_at_utc: String,
    pub z_bounds: ZBounds,
}

impl WorldProfile {
    pub fn min_z(&self) -> i32 {
        self.z_bounds.min
    }

    pub fn max_z(&self) -> i32 {
        self.z_bounds.max
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpecialInstanceId {
    pub value: u64,
}

impl Display for SpecialInstanceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016X}", self.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SpecialInstance {
    pub instance_id: SpecialInstanceId,
    pub type_: SpecialType,
    pub location: WorldCellCoordinate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FeatureSegmentDescriptor {
    pub kind: FeatureKind,
    pub start: WorldCellCoordinate,
    pub end: WorldCellCoordinate,
    pub elevated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkFeatureSummary {
    pub features: Vec<FeatureSegmentDescriptor>,
    pub settlement_count: i32,
    pub special_count: i32,
}

impl Default for ChunkFeatureSummary {
    fn default() -> Self {
        Self {
            features: Vec::new(),
            settlement_count: 0,
            special_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ChunkConnectivitySummary {
    pub north_road_connections: i32,
    pub south_road_connections: i32,
    pub east_road_connections: i32,
    pub west_road_connections: i32,
    pub north_water_connections: i32,
    pub south_water_connections: i32,
    pub east_water_connections: i32,
    pub west_water_connections: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacroChunkProvenance {
    pub schema_version: i32,
    pub generation_version: i32,
    pub content_hash: String,
    pub macro_seed: u64,
    pub canonical_hash: u64,
    pub generated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticParcel {
    pub landcover: LandcoverType,
    pub infrastructure: InfrastructureType,
    pub parcel: ParcelType,
    pub rotation: Orientation,
    pub elevation_mode: ElevationMode,
    pub layers: Vec<SemanticLayer>,
    pub special: Option<SpecialInstance>,
    pub settlement_id: Option<i32>,
    pub flags: CellFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacroChunk {
    pub key: MacroChunkKey,
    pub width: i32,
    pub height: i32,
    pub z_bounds: ZBounds,
    pub parcels: BTreeMap<WorldCellCoordinate, SemanticParcel>,
    pub feature_summary: ChunkFeatureSummary,
    pub connectivity: ChunkConnectivitySummary,
    pub provenance: MacroChunkProvenance,
    pub region_id: String,
    pub surface_default_landcover: LandcoverType,
}

impl MacroChunk {
    pub fn min_z(&self) -> i32 {
        self.z_bounds.min
    }

    pub fn max_z(&self) -> i32 {
        self.z_bounds.max
    }

    pub fn stored_parcel_count(&self) -> usize {
        self.parcels.len()
    }

    pub fn get_parcel(&self, x: i32, y: i32, z: i32) -> Option<&SemanticParcel> {
        self.validate_coordinates(x, y, z);
        self.parcels.get(&WorldCellCoordinate { x, y, z })
    }

    pub fn resolve_parcel(&self, x: i32, y: i32, z: i32) -> SemanticParcel {
        self.get_parcel(x, y, z)
            .cloned()
            .unwrap_or_else(|| Self::create_implicit_parcel(z, self.surface_default_landcover))
    }

    pub fn surface_cells(&self) -> Vec<SemanticParcel> {
        self.enumerate_resolved_cells_at_z(0)
    }

    pub fn enumerate_resolved_cells_at_z(&self, z: i32) -> Vec<SemanticParcel> {
        let mut cells = Vec::with_capacity((self.width * self.height) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                cells.push(self.resolve_parcel(x, y, z));
            }
        }
        cells
    }

    pub fn create_implicit_parcel(z: i32, surface_default_landcover: LandcoverType) -> SemanticParcel {
        let landcover = match z.cmp(&0) {
            Ordering::Less => LandcoverType::Subterranean,
            Ordering::Equal => surface_default_landcover,
            Ordering::Greater => LandcoverType::OpenAir,
        };
        let elevation_mode = match z.cmp(&0) {
            Ordering::Less => ElevationMode::Underground,
            Ordering::Equal => ElevationMode::Surface,
            Ordering::Greater => ElevationMode::Elevated,
        };
        let flags = if z == 0 { CellFlags::TRAVERSABLE } else { CellFlags::NONE };
        SemanticParcel {
            landcover,
            infrastructure: InfrastructureType::None,
            parcel: if z == 0 { ParcelType::Field } else { ParcelType::Empty },
            rotation: Orientation::North,
            elevation_mode,
            layers: vec![SemanticLayer::create(
                SemanticLayerKind::Landform,
                SemanticValue::for_landcover(landcover),
                0,
                LayerBlendMode::Replace,
                flags,
                "implicit",
            )],
            special: None,
            settlement_id: None,
            flags,
        }
    }

    fn validate_coordinates(&self, x: i32, y: i32, z: i32) {
        assert!(
            x >= 0 && x < self.width && y >= 0 && y < self.height && self.z_bounds.contains(z),
            "Invalid macro parcel coordinate ({x}, {y}, {z}) for chunk {}.",
            self.key
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCell {
    pub material: LocalMaterialType,
    pub flags: LocalCellFlags,
    pub elevation: u8,
    pub moisture: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacedProp {
    pub type_: LocalPropType,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacedItem {
    pub type_: ItemKind,
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentalField {
    pub type_: FieldType,
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub intensity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpawnMarker {
    pub type_: SpawnType,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalChunkProvenance {
    pub schema_version: i32,
    pub generation_version: i32,
    pub content_hash: String,
    pub local_seed: u64,
    pub source_macro_canonical_hash: u64,
    pub is_mutated: bool,
    pub mutation_count: i32,
    pub generated_at_utc: String,
    pub last_mutation_at_utc: Option<String>,
    pub canonical_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalChunk {
    pub key: LocalChunkKey,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub cells: Vec<LocalCell>,
    pub props: Vec<PlacedProp>,
    pub items: Vec<PlacedItem>,
    pub fields: Vec<EnvironmentalField>,
    pub spawns: Vec<SpawnMarker>,
    pub is_uniform: bool,
    pub is_dirty: bool,
    pub provenance: LocalChunkProvenance,
}

impl LocalChunk {
    pub fn can_regenerate_deterministically(&self) -> bool {
        !self.is_dirty
    }

    pub fn get_cell(&self, x: usize, y: usize, z: usize) -> &LocalCell {
        self.validate_coordinates(x, y, z);
        &self.cells[self.get_index(x, y, z)]
    }

    pub fn set_cell(&mut self, x: usize, y: usize, z: usize, cell: LocalCell) {
        self.validate_coordinates(x, y, z);
        let index = self.get_index(x, y, z);
        self.cells[index] = cell;
        self.mark_mutated();
    }

    pub fn get_index(&self, x: usize, y: usize, z: usize) -> usize {
        assert!(z < self.depth, "local z index out of bounds");
        (z * self.width * self.height) + (y * self.width) + x
    }

    pub fn add_prop(&mut self, prop: PlacedProp) {
        self.props.push(prop);
        self.mark_mutated();
    }

    pub fn add_generated_prop(&mut self, prop: PlacedProp) {
        self.props.push(prop);
    }

    pub fn add_item(&mut self, item: PlacedItem) {
        self.items.push(item);
        self.mark_mutated();
    }

    pub fn add_generated_item(&mut self, item: PlacedItem) {
        self.items.push(item);
    }

    pub fn add_field(&mut self, field: EnvironmentalField) {
        self.fields.push(field);
        self.mark_mutated();
    }

    pub fn add_generated_field(&mut self, field: EnvironmentalField) {
        self.fields.push(field);
    }

    pub fn add_generated_spawn(&mut self, spawn: SpawnMarker) {
        self.spawns.push(spawn);
    }

    pub fn mark_persisted(&mut self) {
        self.is_dirty = false;
    }

    pub fn mark_mutated(&mut self) {
        self.is_dirty = true;
        self.provenance.is_mutated = true;
        self.provenance.mutation_count += 1;
        self.provenance.last_mutation_at_utc = Some(utc_now_string());
    }

    fn validate_coordinates(&self, x: usize, y: usize, z: usize) {
        assert!(
            x < self.width && y < self.height && z < self.depth,
            "Invalid local cell coordinate ({x}, {y}, {z}) for chunk {}.",
            self.key
        );
    }
}
