pub mod railgun;
pub mod mass_driver;
pub mod photon_lance;
pub mod gravity_bomb;
pub mod impulse_rocket;
pub mod tidal_mine;

pub use railgun::Railgun;
pub use mass_driver::MassDriver;
pub use photon_lance::PhotonLance;
pub use gravity_bomb::GravityBomb;
pub use impulse_rocket::ImpulseRocket;
pub use tidal_mine::TidalMine;

use crate::util::Vec2;
use crate::entities::projectile::Projectile;

/// Weapon type enum matching ProjectileType.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    Railgun,
    MassDriver,
    PhotonLance,
    GravityBomb,
    ImpulseRocket,
    TidalMine,
}

/// Info about a weapon slot in the player's loadout.
#[derive(Debug, Clone)]
pub struct WeaponSlotInfo {
    pub weapon_type: WeaponType,
    pub name: &'static str,
    pub unlock_level: u32,
    pub cooldown: f64,
    pub fuel_cost: f64,
}

/// Return weapon slot info for all six weapon slots, in order.
pub fn weapon_slots() -> [WeaponSlotInfo; 6] {
    [
        WeaponSlotInfo {
            weapon_type: WeaponType::Railgun,
            name: "Railgun",
            unlock_level: 1,
            cooldown: railgun::COOLDOWN,
            fuel_cost: railgun::FUEL_COST,
        },
        WeaponSlotInfo {
            weapon_type: WeaponType::MassDriver,
            name: "Mass Driver",
            unlock_level: 3,
            cooldown: mass_driver::COOLDOWN,
            fuel_cost: mass_driver::FUEL_COST,
        },
        WeaponSlotInfo {
            weapon_type: WeaponType::PhotonLance,
            name: "Photon Lance",
            unlock_level: 6,
            cooldown: photon_lance::COOLDOWN,
            fuel_cost: photon_lance::FUEL_COST,
        },
        WeaponSlotInfo {
            weapon_type: WeaponType::GravityBomb,
            name: "Gravity Bomb",
            unlock_level: 9,
            cooldown: gravity_bomb::COOLDOWN,
            fuel_cost: gravity_bomb::FUEL_COST,
        },
        WeaponSlotInfo {
            weapon_type: WeaponType::ImpulseRocket,
            name: "Impulse Rocket",
            unlock_level: 12,
            cooldown: impulse_rocket::COOLDOWN,
            fuel_cost: impulse_rocket::FUEL_COST,
        },
        WeaponSlotInfo {
            weapon_type: WeaponType::TidalMine,
            name: "Tidal Mine",
            unlock_level: 15,
            cooldown: tidal_mine::COOLDOWN,
            fuel_cost: tidal_mine::FUEL_COST,
        },
    ]
}

/// Trait for weapon behavior.
pub trait Weapon {
    /// Cooldown time in proper-time seconds.
    fn cooldown(&self) -> f64;
    /// Fuel cost per shot (or per second for continuous weapons).
    fn fuel_cost(&self) -> f64;
    /// Level at which this weapon unlocks.
    fn unlock_level(&self) -> u32;
    /// Create a projectile. Returns None for beam weapons (PhotonLance).
    fn create_projectile(
        &self,
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        is_player: bool,
    ) -> Option<Projectile>;
}
