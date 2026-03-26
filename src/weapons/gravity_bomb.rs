use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use super::Weapon;

pub const COOLDOWN: f64 = 4.0;
pub const FUEL_COST: f64 = 8.0;
pub const UNLOCK_LEVEL: u32 = 9;

/// Deployable gravity source. After arming, creates a temporary gravitational
/// pull that disrupts enemy orbits. Deals 20 damage on direct hit.
#[derive(Debug, Clone, Copy)]
pub struct GravityBomb;

impl Weapon for GravityBomb {
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
        Some(Projectile::new_gravity_bomb(origin, ship_velocity, turret_angle, ship_tau, is_player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gravity_bomb_specs() {
        let w = GravityBomb;
        assert!((w.cooldown() - 4.0).abs() < 1e-10);
        assert!((w.fuel_cost() - 8.0).abs() < 1e-10);
        assert_eq!(w.unlock_level(), 9);
    }

    #[test]
    fn test_gravity_bomb_creates_projectile() {
        let w = GravityBomb;
        let p = w.create_projectile(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert!(p.is_some());
        let proj = p.unwrap();
        assert!(proj.bomb_mass > 0.0);
    }
}
