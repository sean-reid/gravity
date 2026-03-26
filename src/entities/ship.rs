use crate::util::Vec2;
use crate::physics::orbit::OrbitalParams;
use super::EntityId;
use super::next_entity_id;

/// Rate at which shields regenerate per proper-time second.
pub const SHIELD_REGEN_RATE: f64 = 5.0;
/// Rate at which fuel regenerates per proper-time second.
pub const FUEL_REGEN_RATE: f64 = 2.0;
/// Seconds of proper time after taking damage before shields begin regenerating.
pub const SHIELD_REGEN_DELAY: f64 = 2.0;
/// Magnitude of thrust acceleration (in coordinate-space units/s^2).
pub const THRUST_MAGNITUDE: f64 = 8.0;
/// Fuel consumed per second while thrusting.
pub const FUEL_THRUST_COST: f64 = 15.0;
/// Maximum number of trail positions stored.
pub const MAX_TRAIL_LENGTH: usize = 120;
/// Collision radius of the player ship.
pub const SHIP_RADIUS: f64 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrustDirection {
    None,
    Prograde,
    Retrograde,
    RadialIn,
    RadialOut,
}

impl Default for ThrustDirection {
    fn default() -> Self {
        ThrustDirection::None
    }
}

#[derive(Debug, Clone)]
pub struct PlayerShip {
    pub id: EntityId,
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub health: f64,
    pub shields: f64,
    pub fuel: f64,
    pub tau: f64,
    pub proper_time: f64,
    pub active_weapon: usize,
    pub weapon_cooldowns: [f64; 6],
    pub turret_angle: f64,
    pub orbital_params: OrbitalParams,
    pub shield_regen_delay_timer: f64,
    pub orbit_anchor_active: bool,
    pub orbit_anchor_timer: f64,
    pub orbit_anchor_cooldown: f64,
    pub tidal_flare_cooldown: f64,
    pub trail: Vec<Vec2>,
    pub alive: bool,
    pub thrust_direction: ThrustDirection,
}

impl PlayerShip {
    pub fn new(position: Vec2, velocity: Vec2) -> Self {
        Self {
            id: next_entity_id(),
            position,
            velocity,
            acceleration: Vec2::ZERO,
            health: 100.0,
            shields: 100.0,
            fuel: 100.0,
            tau: 1.0,
            proper_time: 0.0,
            active_weapon: 0,
            weapon_cooldowns: [0.0; 6],
            turret_angle: 0.0,
            orbital_params: OrbitalParams::default(),
            shield_regen_delay_timer: 0.0,
            orbit_anchor_active: false,
            orbit_anchor_timer: 0.0,
            orbit_anchor_cooldown: 0.0,
            tidal_flare_cooldown: 0.0,
            trail: Vec::with_capacity(MAX_TRAIL_LENGTH),
            alive: true,
            thrust_direction: ThrustDirection::None,
        }
    }

    /// Apply damage, absorbed first by shields then health.
    pub fn apply_damage(&mut self, amount: f64) {
        // Reset shield regen delay
        self.shield_regen_delay_timer = 0.0;

        if self.shields > 0.0 {
            let shield_absorbed = amount.min(self.shields);
            self.shields -= shield_absorbed;
            let remaining = amount - shield_absorbed;
            if remaining > 0.0 {
                self.health = (self.health - remaining).max(0.0);
            }
        } else {
            self.health = (self.health - amount).max(0.0);
        }

        if self.health <= 0.0 {
            self.alive = false;
        }
    }

    /// Regenerate shields and fuel over proper time. Called each tick with dt_proper.
    pub fn regenerate(&mut self, dt_proper: f64) {
        // Fuel always regenerates
        self.fuel = (self.fuel + FUEL_REGEN_RATE * dt_proper).min(100.0);

        // Shields regenerate only after delay since last hit
        self.shield_regen_delay_timer += dt_proper;
        if self.shield_regen_delay_timer >= SHIELD_REGEN_DELAY {
            self.shields = (self.shields + SHIELD_REGEN_RATE * dt_proper).min(100.0);
        }
    }

    /// Try to consume fuel. Returns true if enough fuel was available.
    pub fn consume_fuel(&mut self, amount: f64) -> bool {
        if self.fuel >= amount {
            self.fuel -= amount;
            true
        } else {
            false
        }
    }

    /// Whether the ship is dead.
    pub fn is_dead(&self) -> bool {
        !self.alive || self.health <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ship_defaults() {
        let ship = PlayerShip::new(Vec2::new(5.0, 0.0), Vec2::new(0.0, 2.0));
        assert_eq!(ship.health, 100.0);
        assert_eq!(ship.shields, 100.0);
        assert_eq!(ship.fuel, 100.0);
        assert!(ship.alive);
    }

    #[test]
    fn test_damage_shields_first() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.apply_damage(30.0);
        assert_eq!(ship.shields, 70.0);
        assert_eq!(ship.health, 100.0);
        assert!(ship.alive);
    }

    #[test]
    fn test_damage_overflow_to_health() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.shields = 10.0;
        ship.apply_damage(25.0);
        assert_eq!(ship.shields, 0.0);
        assert_eq!(ship.health, 85.0);
    }

    #[test]
    fn test_lethal_damage() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.shields = 0.0;
        ship.apply_damage(150.0);
        assert_eq!(ship.health, 0.0);
        assert!(!ship.alive);
        assert!(ship.is_dead());
    }

    #[test]
    fn test_regenerate_fuel() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.fuel = 50.0;
        ship.shield_regen_delay_timer = 0.0;
        ship.regenerate(1.0);
        assert!((ship.fuel - 52.0).abs() < 1e-10);
    }

    #[test]
    fn test_shield_regen_delay() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.shields = 50.0;
        ship.apply_damage(0.0); // resets delay timer
        ship.regenerate(1.0);
        // Not enough time has passed; shields should not regenerate
        assert!((ship.shields - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_shield_regen_after_delay() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.shields = 50.0;
        ship.shield_regen_delay_timer = SHIELD_REGEN_DELAY;
        ship.regenerate(1.0);
        assert!((ship.shields - 55.0).abs() < 1e-10);
    }

    #[test]
    fn test_consume_fuel_success() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        assert!(ship.consume_fuel(30.0));
        assert!((ship.fuel - 70.0).abs() < 1e-10);
    }

    #[test]
    fn test_consume_fuel_insufficient() {
        let mut ship = PlayerShip::new(Vec2::ZERO, Vec2::ZERO);
        ship.fuel = 5.0;
        assert!(!ship.consume_fuel(10.0));
        assert!((ship.fuel - 5.0).abs() < 1e-10);
    }
}
