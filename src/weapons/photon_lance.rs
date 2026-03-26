use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use super::Weapon;

pub const COOLDOWN: f64 = 0.1;
pub const FUEL_COST: f64 = 5.0; // per second of sustained fire
pub const UNLOCK_LEVEL: u32 = 6;

/// Beam damage per second.
pub const BEAM_DPS: f64 = 15.0;
/// Maximum beam range.
pub const BEAM_RANGE: f64 = 8.0;
/// Visual beam width.
pub const BEAM_WIDTH: f64 = 0.06;

/// Continuous beam weapon. Deals 15 HP/s, costs 5 fuel/s.
/// Does NOT create a projectile; damage is applied via raycast each frame.
#[derive(Debug, Clone, Copy)]
pub struct PhotonLance;

impl Weapon for PhotonLance {
    fn cooldown(&self) -> f64 {
        COOLDOWN
    }

    fn fuel_cost(&self) -> f64 {
        FUEL_COST
    }

    fn unlock_level(&self) -> u32 {
        UNLOCK_LEVEL
    }

    /// PhotonLance is a beam weapon, so it does not create a projectile.
    /// Returns None; the gameplay system handles beam damage via raycast.
    fn create_projectile(
        &self,
        _origin: Vec2,
        _ship_velocity: Vec2,
        _turret_angle: f64,
        _ship_tau: f64,
        _is_player: bool,
    ) -> Option<Projectile> {
        None
    }
}

/// Compute beam endpoint given origin and turret angle.
pub fn beam_endpoint(origin: Vec2, turret_angle: f64) -> Vec2 {
    origin + Vec2::from_angle(turret_angle) * BEAM_RANGE
}

/// Compute damage dealt by the photon lance this frame.
/// Scales with time dilation: lower tau at target = more damage (blueshift).
pub fn compute_beam_damage(dt_proper: f64, target_tau: f64) -> f64 {
    // Blueshift scaling: damage increases when target is deeper in gravity well
    let shift_factor = if target_tau > 0.01 {
        1.0 / target_tau
    } else {
        100.0
    };
    // Cap the shift bonus at 3x
    let capped_shift = shift_factor.min(3.0);
    BEAM_DPS * dt_proper * capped_shift
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photon_lance_specs() {
        let w = PhotonLance;
        assert!((w.cooldown() - 0.1).abs() < 1e-10);
        assert!((w.fuel_cost() - 5.0).abs() < 1e-10);
        assert_eq!(w.unlock_level(), 6);
    }

    #[test]
    fn test_photon_lance_no_projectile() {
        let w = PhotonLance;
        let p = w.create_projectile(Vec2::new(5.0, 0.0), Vec2::ZERO, 0.0, 1.0, true);
        assert!(p.is_none());
    }

    #[test]
    fn test_beam_endpoint() {
        let ep = beam_endpoint(Vec2::new(5.0, 0.0), 0.0);
        assert!((ep.x - (5.0 + BEAM_RANGE)).abs() < 1e-10);
        assert!(ep.y.abs() < 1e-10);
    }

    #[test]
    fn test_beam_damage_flat_spacetime() {
        let dmg = compute_beam_damage(1.0, 1.0);
        assert!((dmg - BEAM_DPS).abs() < 1e-10);
    }

    #[test]
    fn test_beam_damage_blueshift() {
        let dmg = compute_beam_damage(1.0, 0.5);
        // 1/0.5 = 2x multiplier
        assert!((dmg - BEAM_DPS * 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_beam_damage_capped() {
        let dmg = compute_beam_damage(1.0, 0.1);
        // 1/0.1 = 10, capped at 3
        assert!((dmg - BEAM_DPS * 3.0).abs() < 1e-10);
    }
}
