use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use super::Weapon;

pub const COOLDOWN: f64 = 2.0;
pub const FUEL_COST: f64 = 5.0;
pub const UNLOCK_LEVEL: u32 = 15;

/// Deployable mine that settles into orbit and detonates on proximity.
/// Damage scales with altitude difference between mine and target (0-60 HP).
#[derive(Debug, Clone, Copy)]
pub struct TidalMine;

impl Weapon for TidalMine {
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
        Some(Projectile::new_tidal_mine(origin, ship_velocity, turret_angle, ship_tau, is_player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tidal_mine_specs() {
        let w = TidalMine;
        assert!((w.cooldown() - 2.0).abs() < 1e-10);
        assert!((w.fuel_cost() - 5.0).abs() < 1e-10);
        assert_eq!(w.unlock_level(), 15);
    }

    #[test]
    fn test_tidal_mine_creates_projectile() {
        let w = TidalMine;
        let p = w.create_projectile(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert!(p.is_some());
        let proj = p.unwrap();
        assert!(proj.mine_trigger_radius > 0.0);
    }
}
