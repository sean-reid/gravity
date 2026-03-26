use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use super::Weapon;

pub const COOLDOWN: f64 = 0.4;
pub const FUEL_COST: f64 = 0.0;
pub const UNLOCK_LEVEL: u32 = 1;

/// Fast projectile weapon. Low damage, no fuel cost, rapid fire.
/// Damage: 8-12 (base 10 with tau-based blueshift/redshift scaling).
#[derive(Debug, Clone, Copy)]
pub struct Railgun;

impl Weapon for Railgun {
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
        Some(Projectile::new_railgun(origin, ship_velocity, turret_angle, ship_tau, is_player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_railgun_specs() {
        let gun = Railgun;
        assert!((gun.cooldown() - 0.4).abs() < 1e-10);
        assert!((gun.fuel_cost() - 0.0).abs() < 1e-10);
        assert_eq!(gun.unlock_level(), 1);
    }

    #[test]
    fn test_railgun_creates_projectile() {
        let gun = Railgun;
        let p = gun.create_projectile(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert!(p.is_some());
    }
}
