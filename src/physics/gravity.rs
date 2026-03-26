use crate::util::Vec2;

/// Gravitational constant (tunable for gameplay feel).
pub const G: f64 = 50.0;

/// Softening parameter to prevent singularities at close range.
pub const SOFTENING: f64 = 0.1;

/// Compute the total gravitational acceleration at `position` due to all `bodies`.
///
/// Each body is a (position, mass) pair. Uses a softened inverse-square law:
///   a_i = sum_j [ -G * M_j * (r_i - r_j) / (|r_i - r_j|^2 + eps^2)^(3/2) ]
pub fn compute_gravitational_acceleration(position: Vec2, bodies: &[(Vec2, f64)]) -> Vec2 {
    let mut acc = Vec2::ZERO;
    for &(body_pos, body_mass) in bodies {
        acc += compute_gravitational_acceleration_single(position, body_pos, body_mass);
    }
    acc
}

/// Compute gravitational acceleration at `pos` due to a single body at `body_pos` with `body_mass`.
pub fn compute_gravitational_acceleration_single(pos: Vec2, body_pos: Vec2, body_mass: f64) -> Vec2 {
    let diff = pos - body_pos;
    let dist_sq = diff.length_squared();
    let softened_sq = dist_sq + SOFTENING * SOFTENING;
    let softened_dist = softened_sq.sqrt();
    let inv_cube = 1.0 / (softened_sq * softened_dist);
    diff * (-G * body_mass * inv_cube)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acceleration_points_toward_body() {
        let pos = Vec2::new(5.0, 0.0);
        let body = Vec2::new(0.0, 0.0);
        let acc = compute_gravitational_acceleration_single(pos, body, 1.0);
        // Acceleration should point toward the body (negative x direction)
        assert!(acc.x < 0.0);
        assert!(acc.y.abs() < 1e-12);
    }

    #[test]
    fn test_softening_prevents_singularity() {
        let pos = Vec2::new(0.001, 0.0);
        let body = Vec2::ZERO;
        let acc = compute_gravitational_acceleration_single(pos, body, 1.0);
        // Should be finite even at very close range
        assert!(acc.x.is_finite());
        assert!(acc.y.is_finite());
    }

    #[test]
    fn test_zero_distance_is_finite() {
        let pos = Vec2::ZERO;
        let body = Vec2::ZERO;
        let acc = compute_gravitational_acceleration_single(pos, body, 1.0);
        assert!(acc.x.is_finite());
        assert!(acc.y.is_finite());
    }

    #[test]
    fn test_multiple_bodies() {
        let pos = Vec2::ZERO;
        let bodies = vec![
            (Vec2::new(3.0, 0.0), 1.0),
            (Vec2::new(-3.0, 0.0), 1.0),
        ];
        let acc = compute_gravitational_acceleration(pos, &bodies);
        // Symmetric placement -> acceleration should cancel out
        assert!(acc.x.abs() < 1e-12);
        assert!(acc.y.abs() < 1e-12);
    }

    #[test]
    fn test_inverse_square_falloff() {
        let body = (Vec2::ZERO, 1.0);
        let acc_near = compute_gravitational_acceleration(Vec2::new(2.0, 0.0), &[body]);
        let acc_far = compute_gravitational_acceleration(Vec2::new(4.0, 0.0), &[body]);
        // At large distances (relative to softening), should approximate inverse-square
        // acc_near / acc_far ~ (4/2)^2 = 4
        let ratio = acc_near.length() / acc_far.length();
        assert!((ratio - 4.0).abs() < 0.5); // approximate due to softening
    }
}
