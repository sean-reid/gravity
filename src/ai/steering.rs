use crate::util::Vec2;
use crate::physics::gravity::G;
use crate::physics::orbit::compute_circular_orbit_velocity;

/// Compute a thrust vector to change the bot's orbital altitude toward
/// `target_altitude`. Burns prograde to raise orbit, retrograde to lower.
///
/// Returns a unit-length (or zero) thrust direction vector.
pub fn compute_thrust_for_altitude_change(
    current_pos: Vec2,
    current_vel: Vec2,
    bh_pos: Vec2,
    target_altitude: f64,
    bh_mass: f64,
) -> Vec2 {
    let r_vec = current_pos - bh_pos;
    let r = r_vec.length();
    if r < 1e-6 {
        return Vec2::ZERO;
    }

    // Radial and tangential unit vectors
    let radial_out = r_vec.normalized();
    let prograde = Vec2::new(-radial_out.y, radial_out.x); // perpendicular, in orbital direction

    // Make sure prograde is aligned with current velocity's tangential component
    if prograde.dot(current_vel) < 0.0 {
        // Flip to match actual orbital direction
        let prograde = -prograde;
        return compute_altitude_thrust_inner(r, target_altitude, radial_out, prograde, current_vel, bh_mass);
    }

    compute_altitude_thrust_inner(r, target_altitude, radial_out, prograde, current_vel, bh_mass)
}

fn compute_altitude_thrust_inner(
    r: f64,
    target_altitude: f64,
    radial_out: Vec2,
    prograde: Vec2,
    current_vel: Vec2,
    bh_mass: f64,
) -> Vec2 {
    let altitude_error = target_altitude - r;
    let threshold = 0.5; // deadband to prevent oscillation

    if altitude_error.abs() < threshold {
        // Close enough; no altitude change needed
        return Vec2::ZERO;
    }

    // How fast we should be going for a circular orbit at our current altitude
    let v_circ = compute_circular_orbit_velocity(r, bh_mass, G);
    let current_tangential_speed = current_vel.dot(prograde);

    if altitude_error > 0.0 {
        // Need to raise orbit: burn prograde (increase tangential speed)
        // Also add a small radial-out component to help push outward
        let tangential_deficit = v_circ * 1.05 - current_tangential_speed;
        if tangential_deficit > 0.5 {
            return (prograde * 0.9 + radial_out * 0.1).normalized();
        }
        return prograde.normalized();
    } else {
        // Need to lower orbit: burn retrograde (decrease tangential speed)
        let tangential_excess = current_tangential_speed - v_circ * 0.95;
        if tangential_excess > 0.5 {
            return (-prograde * 0.9 - radial_out * 0.1).normalized();
        }
        return (-prograde).normalized();
    }
}

/// Compute small correction thrust to maintain a stable orbit at `target_altitude`.
///
/// Applies gentle radial and tangential corrections to keep the orbit
/// near-circular at the desired altitude. Returns a thrust direction vector
/// (not necessarily unit length; may be zero if orbit is stable).
pub fn compute_orbit_maintenance_thrust(
    pos: Vec2,
    vel: Vec2,
    bh_pos: Vec2,
    target_altitude: f64,
    bh_mass: f64,
) -> Vec2 {
    let r_vec = pos - bh_pos;
    let r = r_vec.length();
    if r < 1e-6 {
        return Vec2::ZERO;
    }

    let radial_out = r_vec.normalized();
    let mut prograde = Vec2::new(-radial_out.y, radial_out.x);
    if prograde.dot(vel) < 0.0 {
        prograde = -prograde;
    }

    let v_circ = compute_circular_orbit_velocity(r, bh_mass, G);
    let tangential_speed = vel.dot(prograde);
    let radial_speed = vel.dot(radial_out);

    let mut correction = Vec2::ZERO;

    // Altitude correction: gentle radial component
    let altitude_error = target_altitude - r;
    if altitude_error.abs() > 0.3 {
        // Tangential correction to adjust altitude
        let speed_error = tangential_speed - v_circ;
        if altitude_error > 0.0 && speed_error < 0.5 {
            correction = correction + prograde * 0.3;
        } else if altitude_error < 0.0 && speed_error > -0.5 {
            correction = correction - prograde * 0.3;
        }
    }

    // Damp radial oscillation: resist radial velocity
    if radial_speed.abs() > 0.5 {
        correction = correction - radial_out * (radial_speed * 0.2);
    }

    // Speed correction: match circular orbit speed
    let speed_error = tangential_speed - v_circ;
    if speed_error.abs() > 0.3 {
        correction = correction - prograde * (speed_error * 0.15);
    }

    if correction.length_squared() < 1e-6 {
        Vec2::ZERO
    } else {
        correction.normalized()
    }
}

/// Compute thrust to steer a bot toward a formation slot position.
///
/// Used by swarm bots to maintain formation. Returns a thrust direction
/// that blends position-seeking with velocity matching.
pub fn compute_formation_thrust(
    bot_pos: Vec2,
    bot_vel: Vec2,
    target_slot: Vec2,
) -> Vec2 {
    let offset = target_slot - bot_pos;
    let dist = offset.length();

    if dist < 0.3 {
        // Close enough to slot; just damp velocity differences
        if bot_vel.length() > 0.5 {
            return (-bot_vel).normalized();
        }
        return Vec2::ZERO;
    }

    // Proportional-derivative style control:
    // Steer toward target, but also resist velocity that takes us away
    let seek = offset.normalized();
    let brake_factor = if dist < 2.0 { 0.3 } else { 0.1 };
    let combined = seek - bot_vel * brake_factor;

    if combined.length_squared() < 1e-6 {
        Vec2::ZERO
    } else {
        combined.normalized()
    }
}

/// Compute an evasion thrust to dodge incoming threats (projectile positions).
///
/// Steers perpendicular to the average threat direction, choosing the side
/// that requires less turning relative to current velocity.
pub fn compute_evasion_thrust(
    bot_pos: Vec2,
    bot_vel: Vec2,
    threats: &[Vec2],
) -> Vec2 {
    if threats.is_empty() {
        return Vec2::ZERO;
    }

    // Compute average threat direction
    let mut avg_threat_dir = Vec2::ZERO;
    let mut count = 0;
    for &threat_pos in threats {
        let to_bot = bot_pos - threat_pos;
        let dist = to_bot.length();
        if dist < 1e-6 {
            continue;
        }
        // Weight closer threats more heavily
        let weight = 1.0 / (dist + 0.5);
        avg_threat_dir = avg_threat_dir + to_bot.normalized() * weight;
        count += 1;
    }

    if count == 0 {
        return Vec2::ZERO;
    }

    let avg_len = avg_threat_dir.length();
    if avg_len < 1e-6 {
        return Vec2::ZERO;
    }
    avg_threat_dir = avg_threat_dir / avg_len;

    // Dodge perpendicular to the threat direction.
    // Choose the perpendicular that is more aligned with current velocity
    // (requires less course change).
    let perp_a = avg_threat_dir.perpendicular();
    let perp_b = -perp_a;

    let vel_len = bot_vel.length();
    if vel_len < 0.1 {
        // Not moving much; just dodge in any perpendicular direction
        return perp_a;
    }

    let vel_dir = bot_vel / vel_len;
    if vel_dir.dot(perp_a) >= vel_dir.dot(perp_b) {
        perp_a
    } else {
        perp_b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_altitude_change_raise() {
        // Bot at r=5, wants to reach r=10 -> should get prograde thrust
        let pos = Vec2::new(5.0, 0.0);
        let vel = Vec2::new(0.0, 3.0); // orbiting counter-clockwise
        let thrust = compute_thrust_for_altitude_change(pos, vel, Vec2::ZERO, 10.0, 1.0);
        // Prograde for this position/velocity is roughly +y direction
        assert!(thrust.y > 0.0, "Expected prograde thrust, got {:?}", thrust);
    }

    #[test]
    fn test_altitude_change_lower() {
        let pos = Vec2::new(10.0, 0.0);
        let vel = Vec2::new(0.0, 3.0);
        let thrust = compute_thrust_for_altitude_change(pos, vel, Vec2::ZERO, 4.0, 1.0);
        // Retrograde for this position/velocity is roughly -y direction
        assert!(thrust.y < 0.0, "Expected retrograde thrust, got {:?}", thrust);
    }

    #[test]
    fn test_altitude_change_deadband() {
        let pos = Vec2::new(10.0, 0.0);
        let vel = Vec2::new(0.0, 3.0);
        let thrust = compute_thrust_for_altitude_change(pos, vel, Vec2::ZERO, 10.2, 1.0);
        // Within threshold -> zero thrust
        assert!(thrust.length() < 1e-6);
    }

    #[test]
    fn test_orbit_maintenance_stable() {
        // If already at correct altitude and speed, correction should be small
        let r = 10.0;
        let v = compute_circular_orbit_velocity(r, 1.0, G);
        let pos = Vec2::new(r, 0.0);
        let vel = Vec2::new(0.0, v);
        let thrust = compute_orbit_maintenance_thrust(pos, vel, Vec2::ZERO, r, 1.0);
        assert!(
            thrust.length() < 0.5,
            "Expected small correction, got {:?} (len={})",
            thrust,
            thrust.length()
        );
    }

    #[test]
    fn test_formation_thrust_far() {
        let thrust = compute_formation_thrust(
            Vec2::new(0.0, 0.0),
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
        );
        // Should thrust toward the target slot
        assert!(thrust.x > 0.5, "Expected thrust toward target, got {:?}", thrust);
    }

    #[test]
    fn test_formation_thrust_close() {
        let thrust = compute_formation_thrust(
            Vec2::new(0.0, 0.0),
            Vec2::ZERO,
            Vec2::new(0.1, 0.0),
        );
        // Very close, should be near-zero
        assert!(thrust.length() < 0.5);
    }

    #[test]
    fn test_evasion_empty() {
        let thrust = compute_evasion_thrust(Vec2::ZERO, Vec2::ZERO, &[]);
        assert!(thrust.length() < 1e-6);
    }

    #[test]
    fn test_evasion_perpendicular() {
        // Threat coming from the right
        let thrust = compute_evasion_thrust(
            Vec2::new(5.0, 0.0),
            Vec2::new(0.0, 1.0), // moving upward
            &[Vec2::new(8.0, 0.0)],
        );
        // Should dodge perpendicular to the threat direction (roughly +/- y)
        assert!(thrust.y.abs() > 0.5, "Expected perpendicular dodge, got {:?}", thrust);
    }
}
