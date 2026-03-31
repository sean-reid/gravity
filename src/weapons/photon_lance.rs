use crate::util::Vec2;
use crate::entities::projectile::Projectile;
use crate::physics::gravity::compute_gravitational_acceleration;
use crate::physics::collision::circle_circle;
use super::Weapon;

pub const COOLDOWN: f64 = 0.1;
pub const FUEL_COST: f64 = 5.0; // per second of sustained fire
pub const UNLOCK_LEVEL: u32 = 12;

/// Beam damage per second.
pub const BEAM_DPS: f64 = 15.0;
/// Maximum beam range (arc length).
pub const BEAM_RANGE: f64 = 8.0;
/// Visual beam width.
pub const BEAM_WIDTH: f64 = 0.06;
/// Number of ray-march steps for geodesic tracing.
const GEODESIC_STEPS: u32 = 40;
/// Effective c² in game units. Controls how strongly gravity bends the beam.
/// Derived from architecture §17.5: deflection = accel * step_length / c².
/// Lower values = more bending. Tuned for gameplay-visible curvature.
const C_SQUARED: f64 = 0.667;

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
        // Deflect ray direction by local gravitational curvature (architecture §17.5).
        // deflection = accel * step_length / c², approximating null geodesic bending.
        let accel = compute_gravitational_acceleration(pos, gravity_sources);
        dir = (dir + accel * (step_len / C_SQUARED)).normalized();

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

/// Depth damage bonus factor (architecture §18.2).
const DEPTH_BONUS_FACTOR: f64 = 2.0;

/// Compute damage dealt by the photon lance this frame.
/// Scales with attacker depth: effective_damage = base * (1 + depth_bonus * (1 - τ_attacker)).
/// At the rim (τ ≈ 1) no bonus; in the furnace (τ ≈ 0.6) 80% bonus; at abyss edge (τ ≈ 0.3) 140% bonus.
pub fn compute_beam_damage(dt_proper: f64, attacker_tau: f64) -> f64 {
    let depth_multiplier = 1.0 + DEPTH_BONUS_FACTOR * (1.0 - attacker_tau.clamp(0.01, 1.0));
    BEAM_DPS * dt_proper * depth_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photon_lance_specs() {
        let w = PhotonLance;
        assert!((w.cooldown() - 0.1).abs() < 1e-10);
        assert!((w.fuel_cost() - 5.0).abs() < 1e-10);
        assert_eq!(w.unlock_level(), 12);
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
    fn test_beam_damage_depth_bonus() {
        // Attacker at tau=0.5: multiplier = 1 + 2*(1-0.5) = 2.0
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
