use crate::util::Vec2;
use crate::entities::bot::BotArchetype;
use crate::weapons::WeaponType;

#[derive(Debug, Clone)]
pub struct LevelConfig {
    pub seed: u64,
    pub level_number: u32,
    pub act: u32,
    pub black_holes: Vec<BlackHoleConfig>,
    pub max_dilation_factor: f64,
    pub abyss_radius_factor: f64,
    pub bot_spawns: Vec<BotSpawn>,
    pub weapons_available: Vec<WeaponType>,
    pub player_start_altitude: f64,
    pub player_start_phase: f64,
}

#[derive(Debug, Clone)]
pub struct BlackHoleConfig {
    pub mass: f64,
    pub position: Vec2,
    pub schwarzschild_radius: f64,
    pub orbital_radius: f64,
    pub orbital_phase: f64,
    pub orbital_speed: f64,
}

#[derive(Debug, Clone)]
pub struct BotSpawn {
    pub archetype: BotArchetype,
    pub altitude: f64,
    pub phase: f64,
    pub swarm_group: Option<u32>,
}
