use crate::util::Vec2;

/// Minimum time dilation factor to prevent division by zero or negative square roots.
const TAU_MIN: f64 = 0.01;
/// Maximum time dilation factor (flat spacetime).
const TAU_MAX: f64 = 1.0;

/// Compute the Schwarzschild time dilation factor at `position` due to multiple black holes.
///
/// For multiple BHs: tau = sqrt(1 - sum_j(r_s_j / |r - r_j|))
///
/// `black_holes` is a slice of (position, schwarzschild_radius) pairs.
/// The result is clamped to [0.01, 1.0].
pub fn compute_tau(position: Vec2, black_holes: &[(Vec2, f64)]) -> f64 {
    let mut potential_sum = 0.0;
    for &(bh_pos, bh_rs) in black_holes {
        let dist = position.distance(bh_pos);
        if dist > 1e-12 {
            potential_sum += bh_rs / dist;
        } else {
            // Essentially at the singularity; return minimum dilation
            return TAU_MIN;
        }
    }

    let tau_squared = 1.0 - potential_sum;
    if tau_squared <= 0.0 {
        TAU_MIN
    } else {
        tau_squared.sqrt().clamp(TAU_MIN, TAU_MAX)
    }
}

/// Compute how many simulation sub-steps to run this frame, and the effective dt per step.
///
/// When a player experiences strong time dilation (low tau), their local clock runs
/// slower relative to coordinate time. We scale the number of physics steps accordingly
/// so that the player perceives smooth gameplay regardless of dilation.
///
/// - `dt_wall`: wall-clock time elapsed this frame (e.g., 1/60)
/// - `tau_player`: the player's time dilation factor (0.01..1.0)
/// - `fixed_dt_coord`: the desired coordinate-time step size (e.g., 1/120)
/// - `max_steps`: upper bound on steps per frame to prevent spiral of death
///
/// Returns (num_steps, effective_dt_per_step) where:
///   num_steps * effective_dt_per_step = dt_wall * tau_player (proper time elapsed)
///   but measured in coordinate time: total_coord_dt = dt_wall
pub fn compute_steps_per_frame(
    dt_wall: f64,
    tau_player: f64,
    fixed_dt_coord: f64,
    max_steps: u32,
) -> (u32, f64) {
    // The coordinate time that needs to elapse this frame
    let total_coord_dt = dt_wall;

    // Number of fixed-size steps needed to cover the coordinate time
    let ideal_steps = (total_coord_dt / fixed_dt_coord).ceil() as u32;

    // Scale steps by inverse of tau: slower proper time means we need fewer coordinate steps
    // from the player's perspective, but the simulation still runs at coordinate time.
    // More dilation (lower tau) = time passes slower for the player = fewer effective steps.
    let scaled_steps = ((ideal_steps as f64) * tau_player).ceil() as u32;

    let num_steps = scaled_steps.max(1).min(max_steps);
    let effective_dt = total_coord_dt / num_steps as f64;

    (num_steps, effective_dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tau_far_from_bh() {
        let pos = Vec2::new(100.0, 0.0);
        let bhs = vec![(Vec2::ZERO, 1.0)];
        let tau = compute_tau(pos, &bhs);
        // Far away, tau should be close to 1.0
        assert!((tau - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_tau_at_event_horizon() {
        let pos = Vec2::new(1.0, 0.0);
        let bhs = vec![(Vec2::ZERO, 1.0)];
        let tau = compute_tau(pos, &bhs);
        // At r = r_s, tau = sqrt(1 - 1) = 0 -> clamped to TAU_MIN
        assert!((tau - TAU_MIN).abs() < 1e-10);
    }

    #[test]
    fn test_tau_inside_event_horizon() {
        let pos = Vec2::new(0.5, 0.0);
        let bhs = vec![(Vec2::ZERO, 1.0)];
        let tau = compute_tau(pos, &bhs);
        assert!((tau - TAU_MIN).abs() < 1e-10);
    }

    #[test]
    fn test_tau_multiple_bhs() {
        let pos = Vec2::new(5.0, 0.0);
        let bhs = vec![
            (Vec2::new(0.0, 0.0), 1.0),
            (Vec2::new(10.0, 0.0), 1.0),
        ];
        let tau = compute_tau(pos, &bhs);
        // Both at distance 5, each contributes 1/5 = 0.2, sum = 0.4
        // tau = sqrt(1 - 0.4) = sqrt(0.6) ~ 0.7746
        assert!((tau - 0.6_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_tau_no_black_holes() {
        let pos = Vec2::new(5.0, 0.0);
        let bhs: &[(Vec2, f64)] = &[];
        let tau = compute_tau(pos, bhs);
        assert!((tau - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_steps_per_frame_no_dilation() {
        let (steps, dt) = compute_steps_per_frame(1.0 / 60.0, 1.0, 1.0 / 120.0, 10);
        assert_eq!(steps, 2);
        assert!((dt - 1.0 / 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_steps_per_frame_heavy_dilation() {
        let (steps, _dt) = compute_steps_per_frame(1.0 / 60.0, 0.1, 1.0 / 120.0, 10);
        // With tau=0.1, scaled_steps = ceil(2 * 0.1) = 1
        assert_eq!(steps, 1);
    }

    #[test]
    fn test_steps_per_frame_max_cap() {
        let (steps, _dt) = compute_steps_per_frame(1.0, 1.0, 0.001, 10);
        assert_eq!(steps, 10);
    }
}
