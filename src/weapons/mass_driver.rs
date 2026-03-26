use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use super::Weapon;

pub const COOLDOWN: f64 = 1.5;
pub const FUEL_COST: f64 = 3.0;
pub const UNLOCK_LEVEL: u32 = 3;

/// Slow, heavy projectile. High damage (30-50), scales with gravitational depth.
#[derive(Debug, Clone, Copy)]
pub struct MassDriver;

impl Weapon for MassDriver {
    fn cooldown(&self) -> f64 {
        COOLDOWN
    }

    fn fuel_cost(&self) -> f64 {
        FUEL_COST
    }

    fn unlock_level(&self) -> u32 {
        UNLOCK_LEVEL
    }

    fn create_projectile(
        &self,
        origin: Vec2,
        ship_velocity: Vec2,
        turret_angle: f64,
        ship_tau: f64,
        is_player: bool,
    ) -> Option<Projectile> {
        Some(Projectile::new_mass_driver(origin, ship_velocity, turret_angle, ship_tau, is_player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_driver_specs() {
        let w = MassDriver;
        assert!((w.cooldown() - 1.5).abs() < 1e-10);
        assert!((w.fuel_cost() - 3.0).abs() < 1e-10);
        assert_eq!(w.unlock_level(), 3);
    }

    #[test]
    fn test_mass_driver_creates_projectile() {
        let w = MassDriver;
        let p = w.create_projectile(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 0.8, true);
        assert!(p.is_some());
    }
}
