use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use crate::physics::gravity::compute_gravitational_acceleration;
use crate::physics::collision::circle_circle;
use super::Weapon;

pub const COOLDOWN: f64 = 0.1;
pub const FUEL_COST: f64 = 5.0; // per second of sustained fire
pub const UNLOCK_LEVEL: u32 = 6;

/// Beam damage per second.
pub const BEAM_DPS: f64 = 15.0;
/// Maximum beam range (arc length).
pub const BEAM_RANGE: f64 = 8.0;
/// Visual beam width.
pub const BEAM_WIDTH: f64 = 0.06;
/// Number of ray-march steps for geodesic tracing.
const GEODESIC_STEPS: u32 = 40;

/// Continuous beam weapon. Deals 15 HP/s, costs 5 fuel/s.
/// Does NOT create a projectile; damage is applied via geodesic raycast each frame.
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

/// A single segment of the curved beam path.
#[derive(Debug, Clone, Copy)]
pub struct BeamPoint {
    pub position: Vec2,
}

/// Trace the beam along a geodesic (photon path through gravitational field).
/// Returns the series of points along the beam, and optionally the index of the
/// first bot hit along with the hit position.
///
/// `gravity_sources` is &[(position, mass)] for all black holes.
/// `bots` is &[(position, radius)] for hit testing.
pub fn trace_beam_geodesic(
    origin: Vec2,
    turret_angle: f64,
    gravity_sources: &[(Vec2, f64)],
    bots: &[(Vec2, f64)],
    bot_alive: &[bool],
) -> (Vec<BeamPoint>, Option<(usize, Vec2)>) {
    let step_len = BEAM_RANGE / GEODESIC_STEPS as f64;
    let mut pos = origin;
    let mut dir = Vec2::from_angle(turret_angle);
    let mut points = Vec::with_capacity(GEODESIC_STEPS as usize + 1);
    points.push(BeamPoint { position: pos });

    for _ in 0..GEODESIC_STEPS {
        // Compute gravitational deflection at current position.
        // Photons travel at c (infinite speed in our game), but gravity bends their path.
        // We use the gravitational acceleration to deflect the direction vector.
        let accel = compute_gravitational_acceleration(pos, gravity_sources);
        // Deflection: bend the direction toward the gravity source.
        // The factor controls how strongly gravity bends the beam.
        // A real photon near a Schwarzschild BH deflects by ~2*r_s/b (b=impact parameter).
        // We scale it for gameplay visibility.
        let deflection_strength = 1.5; // tunable
        dir = (dir + accel * (step_len * deflection_strength / (accel.length() + 1e-6).max(1e-6)
            * accel.length()))
            .normalized();
        // Simplified: just add accel * step_len * strength as a velocity-like nudge
        dir = (dir + accel * step_len * deflection_strength).normalized();

        pos = pos + dir * step_len;
        points.push(BeamPoint { position: pos });

        // Hit test against bots at this step
        for (i, &(bot_pos, bot_radius)) in bots.iter().enumerate() {
            if !bot_alive[i] {
                continue;
            }
            if circle_circle(pos, BEAM_WIDTH * 0.5, bot_pos, bot_radius) {
                return (points, Some((i, pos)));
            }
        }
    }

    (points, None)
}

/// Simple straight-line beam endpoint (fallback).
pub fn beam_endpoint(origin: Vec2, turret_angle: f64) -> Vec2 {
    origin + Vec2::from_angle(turret_angle) * BEAM_RANGE
}

/// Compute damage dealt by the photon lance this frame.
/// Scales with time dilation: lower tau at target = more damage (blueshift).
pub fn compute_beam_damage(dt_proper: f64, target_tau: f64) -> f64 {
    let shift_factor = if target_tau > 0.01 {
        1.0 / target_tau
    } else {
        100.0
    };
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
    fn test_beam_damage_flat_spacetime() {
        let dmg = compute_beam_damage(1.0, 1.0);
        assert!((dmg - BEAM_DPS).abs() < 1e-10);
    }

    #[test]
    fn test_beam_damage_blueshift() {
        let dmg = compute_beam_damage(1.0, 0.5);
        assert!((dmg - BEAM_DPS * 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_geodesic_no_gravity() {
        // No gravity sources: beam should go straight
        let (points, hit) = trace_beam_geodesic(
            Vec2::new(5.0, 0.0), 0.0, &[], &[], &[],
        );
        assert!(hit.is_none());
        assert!(points.len() > 2);
        let last = points.last().unwrap().position;
        assert!((last.x - (5.0 + BEAM_RANGE)).abs() < 0.1);
        assert!(last.y.abs() < 0.1);
    }

    #[test]
    fn test_geodesic_curves_toward_mass() {
        // Black hole at origin: beam fired tangentially should curve inward
        let gravity = vec![(Vec2::ZERO, 1.0)];
        let (points, _) = trace_beam_geodesic(
            Vec2::new(5.0, 0.0), std::f64::consts::FRAC_PI_2, // fire upward
            &gravity, &[], &[],
        );
        let last = points.last().unwrap().position;
        // Should curve inward (x should decrease from 5.0)
        assert!(last.x < 5.0, "Beam should curve toward the mass, got x={}", last.x);
    }
}
