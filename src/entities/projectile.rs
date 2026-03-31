use crate::util::Vec2;
use super::{EntityId, next_entity_id};

// -- Railgun constants --
pub const RAILGUN_SPEED: f64 = 40.0;
pub const RAILGUN_DAMAGE: f64 = 10.0;
pub const RAILGUN_RADIUS: f64 = 0.05;
pub const RAILGUN_LIFETIME: f64 = 2.0;

// -- Mass driver constants --
// Slow heavy slug (~0.8x orbital speed) that curves dramatically in gravity fields.
// Mastering trick shots around the black hole is the skill ceiling.
pub const MASS_DRIVER_SPEED: f64 = 3.0;
pub const MASS_DRIVER_DAMAGE: f64 = 40.0;
pub const MASS_DRIVER_RADIUS: f64 = 0.15;
pub const MASS_DRIVER_LIFETIME: f64 = 8.0;

// -- Impulse rocket constants --
// Slow tracking rocket (~0.5x orbital speed) — the kill comes from the orbital kick, not damage
pub const IMPULSE_ROCKET_SPEED: f64 = 2.0;
pub const IMPULSE_ROCKET_DAMAGE: f64 = 5.0;
pub const IMPULSE_ROCKET_RADIUS: f64 = 0.1;
pub const IMPULSE_ROCKET_LIFETIME: f64 = 6.0;
pub const IMPULSE_ROCKET_TRACKING: f64 = 5.5;

// -- Gravity bomb constants --
pub const GRAVITY_BOMB_SPEED: f64 = 8.0;
pub const GRAVITY_BOMB_DAMAGE: f64 = 20.0;
pub const GRAVITY_BOMB_RADIUS: f64 = 0.15;
pub const GRAVITY_BOMB_LIFETIME: f64 = 10.0;
pub const GRAVITY_BOMB_MASS: f64 = 0.3;
pub const GRAVITY_BOMB_ARM_TIME: f64 = 1.5;

// -- Tidal mine constants --
pub const TIDAL_MINE_SPEED: f64 = 6.0;
pub const TIDAL_MINE_BASE_DAMAGE: f64 = 20.0;
pub const TIDAL_MINE_MAX_DAMAGE: f64 = 60.0;
/// Altitude difference (in r_s) at which damage reaches max.
pub const TIDAL_MINE_SCALE_FACTOR: f64 = 5.0;
pub const TIDAL_MINE_RADIUS: f64 = 0.1;
pub const TIDAL_MINE_LIFETIME: f64 = 20.0;
pub const TIDAL_MINE_TRIGGER_RADIUS: f64 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileType {
    Railgun,
    MassDriver,
    PhotonLance,
    ImpulseRocket,
    GravityBomb,
    TidalMine,
}

#[derive(Debug, Clone)]
pub struct Projectile {
    pub id: EntityId,
    pub projectile_type: ProjectileType,
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub radius: f64,
    pub damage: f64,
    pub owner_is_player: bool,
    pub tau_at_launch: f64,
    pub lifetime: f64,
    pub alive: bool,
    // Impulse rocket tracking
    pub tracking_strength: f64,
    // Gravity bomb fields
    pub bomb_active: bool,
    pub bomb_mass: f64,
    pub bomb_timer: f64,
    // Tidal mine fields
    pub mine_orbiting: bool,
    pub mine_altitude: f64,
    pub mine_trigger_radius: f64,
}

impl Projectile {
    /// Create a railgun projectile. Fast, lightweight, low damage.
    pub fn new_railgun(
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        owner_is_player: bool,
    ) -> Self {
        let dir = Vec2::from_angle(turret_angle);
        let velocity = ship_velocity + dir * RAILGUN_SPEED;
        Self {
            id: next_entity_id(),
            projectile_type: ProjectileType::Railgun,
            position: origin + dir * 0.3,
            velocity,
            acceleration: Vec2::ZERO,
            radius: RAILGUN_RADIUS,
            damage: RAILGUN_DAMAGE,
            owner_is_player,
            tau_at_launch: ship_tau,
            lifetime: RAILGUN_LIFETIME,
            alive: true,
            tracking_strength: 0.0,
            bomb_active: false,
            bomb_mass: 0.0,
            bomb_timer: 0.0,
            mine_orbiting: false,
            mine_altitude: 0.0,
            mine_trigger_radius: 0.0,
        }
    }

    /// Create a mass driver projectile. Slow, heavy, high damage, scales with depth.
    pub fn new_mass_driver(
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        owner_is_player: bool,
    ) -> Self {
        let dir = Vec2::from_angle(turret_angle);
        let velocity = ship_velocity + dir * MASS_DRIVER_SPEED;
        Self {
            id: next_entity_id(),
            projectile_type: ProjectileType::MassDriver,
            position: origin + dir * 0.3,
            velocity,
            acceleration: Vec2::ZERO,
            radius: MASS_DRIVER_RADIUS,
            damage: MASS_DRIVER_DAMAGE,
            owner_is_player,
            tau_at_launch: ship_tau,
            lifetime: MASS_DRIVER_LIFETIME,
            alive: true,
            tracking_strength: 0.0,
            bomb_active: false,
            bomb_mass: 0.0,
            bomb_timer: 0.0,
            mine_orbiting: false,
            mine_altitude: 0.0,
            mine_trigger_radius: 0.0,
        }
    }

    /// Create an impulse rocket. Slow, tracking, delivers orbital kick.
    pub fn new_impulse_rocket(
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        owner_is_player: bool,
    ) -> Self {
        let dir = Vec2::from_angle(turret_angle);
        let velocity = ship_velocity + dir * IMPULSE_ROCKET_SPEED;
        Self {
            id: next_entity_id(),
            projectile_type: ProjectileType::ImpulseRocket,
            position: origin + dir * 0.3,
            velocity,
            acceleration: Vec2::ZERO,
            radius: IMPULSE_ROCKET_RADIUS,
            damage: IMPULSE_ROCKET_DAMAGE,
            owner_is_player,
            tau_at_launch: ship_tau,
            lifetime: IMPULSE_ROCKET_LIFETIME,
            alive: true,
            tracking_strength: IMPULSE_ROCKET_TRACKING,
            bomb_active: false,
            bomb_mass: 0.0,
            bomb_timer: 0.0,
            mine_orbiting: false,
            mine_altitude: 0.0,
            mine_trigger_radius: 0.0,
        }
    }

    /// Create a gravity bomb. Deploys into a temporary gravity source.
    pub fn new_gravity_bomb(
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        owner_is_player: bool,
    ) -> Self {
        let dir = Vec2::from_angle(turret_angle);
        let velocity = ship_velocity + dir * GRAVITY_BOMB_SPEED;
        Self {
            id: next_entity_id(),
            projectile_type: ProjectileType::GravityBomb,
            position: origin + dir * 0.3,
            velocity,
            acceleration: Vec2::ZERO,
            radius: GRAVITY_BOMB_RADIUS,
            damage: GRAVITY_BOMB_DAMAGE,
            owner_is_player,
            tau_at_launch: ship_tau,
            lifetime: GRAVITY_BOMB_LIFETIME,
            alive: true,
            tracking_strength: 0.0,
            bomb_active: false,
            bomb_mass: GRAVITY_BOMB_MASS,
            bomb_timer: GRAVITY_BOMB_ARM_TIME,
            mine_orbiting: false,
            mine_altitude: 0.0,
            mine_trigger_radius: 0.0,
        }
    }

    /// Create a tidal mine. Deploys into orbit and triggers on proximity.
    pub fn new_tidal_mine(
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        owner_is_player: bool,
    ) -> Self {
        let dir = Vec2::from_angle(turret_angle);
        let velocity = ship_velocity + dir * TIDAL_MINE_SPEED;
        let altitude = origin.length();
        Self {
            id: next_entity_id(),
            projectile_type: ProjectileType::TidalMine,
            position: origin + dir * 0.3,
            velocity,
            acceleration: Vec2::ZERO,
            radius: TIDAL_MINE_RADIUS,
            damage: TIDAL_MINE_BASE_DAMAGE,
            owner_is_player,
            tau_at_launch: ship_tau,
            lifetime: TIDAL_MINE_LIFETIME,
            alive: true,
            tracking_strength: 0.0,
            bomb_active: false,
            bomb_mass: 0.0,
            bomb_timer: 0.0,
            mine_orbiting: false,
            mine_altitude: altitude,
            mine_trigger_radius: TIDAL_MINE_TRIGGER_RADIUS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_railgun_properties() {
        let p = Projectile::new_railgun(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert_eq!(p.projectile_type, ProjectileType::Railgun);
        assert!(p.alive);
        assert!(p.owner_is_player);
        assert!((p.damage - RAILGUN_DAMAGE).abs() < 1e-10);
        assert!(p.velocity.length() > RAILGUN_SPEED - 1.0);
    }

    #[test]
    fn test_mass_driver_properties() {
        let p = Projectile::new_mass_driver(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 0.8, false);
        assert_eq!(p.projectile_type, ProjectileType::MassDriver);
        assert!(!p.owner_is_player);
        assert!((p.damage - MASS_DRIVER_DAMAGE).abs() < 1e-10);
    }

    #[test]
    fn test_impulse_rocket_tracking() {
        let p = Projectile::new_impulse_rocket(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert_eq!(p.projectile_type, ProjectileType::ImpulseRocket);
        assert!(p.tracking_strength > 0.0);
    }

    #[test]
    fn test_gravity_bomb_fields() {
        let p = Projectile::new_gravity_bomb(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert_eq!(p.projectile_type, ProjectileType::GravityBomb);
        assert!(!p.bomb_active);
        assert!(p.bomb_mass > 0.0);
        assert!(p.bomb_timer > 0.0);
    }

    #[test]
    fn test_tidal_mine_fields() {
        let p = Projectile::new_tidal_mine(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert_eq!(p.projectile_type, ProjectileType::TidalMine);
        assert!(p.mine_trigger_radius > 0.0);
        assert!(p.mine_altitude > 0.0);
    }

    #[test]
    fn test_projectile_inherits_ship_velocity() {
        let ship_vel = Vec2::new(0.0, 3.0);
        let p = Projectile::new_railgun(Vec2::new(5.0, 0.0), ship_vel, 0.0, 1.0, true);
        // Velocity should include the ship's velocity component
        assert!(p.velocity.y > 2.0);
    }
}
