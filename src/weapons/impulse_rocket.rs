use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use super::Weapon;

pub const COOLDOWN: f64 = 3.0;
pub const FUEL_COST: f64 = 10.0;
pub const UNLOCK_LEVEL: u32 = 12;

/// Slow tracking rocket that delivers an orbital kick on impact.
/// Deals 25 damage and perturbs the target's orbit.
#[derive(Debug, Clone, Copy)]
pub struct ImpulseRocket;

impl Weapon for ImpulseRocket {
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
        Some(Projectile::new_impulse_rocket(origin, ship_velocity, turret_angle, ship_tau, is_player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impulse_rocket_specs() {
        let w = ImpulseRocket;
        assert!((w.cooldown() - 3.0).abs() < 1e-10);
        assert!((w.fuel_cost() - 10.0).abs() < 1e-10);
        assert_eq!(w.unlock_level(), 12);
    }

    #[test]
    fn test_impulse_rocket_creates_projectile() {
        let w = ImpulseRocket;
        let p = w.create_projectile(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert!(p.is_some());
        let proj = p.unwrap();
        assert!(proj.tracking_strength > 0.0);
    }
}
