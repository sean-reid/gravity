use crate::util::{Vec2, Color};

/// Compute a trajectory preview by simulating forward physics with Velocity Verlet integration.
///
/// `position` and `velocity` are the starting state. `black_holes` provides
/// `(position, mass)` for each gravitational attractor. The simulation runs
/// `steps` iterations with timestep `dt`.
///
/// Returns a polyline of world-space positions suitable for rendering as a
/// trajectory overlay.
pub fn compute_trajectory_preview(
    position: Vec2,
    velocity: Vec2,
    black_holes: &[(Vec2, f64)],
    dt: f64,
    steps: u32,
) -> Vec<Vec2> {
    // Gravitational constant matching physics::gravity::G
    const G: f64 = 50.0;
    const SOFTENING: f64 = 0.1;

    let mut points = Vec::with_capacity(steps as usize + 1);
    let mut pos = position;
    let mut vel = velocity;

    points.push(pos);

    // Compute gravitational acceleration at a point
    let accel_at = |p: Vec2| -> Vec2 {
        let mut acc = Vec2::ZERO;
        for &(bh_pos, bh_mass) in black_holes {
            let r = p - bh_pos;
            let dist_sq = r.length_squared() + SOFTENING * SOFTENING;
            let dist = dist_sq.sqrt();
            let force_mag = G * bh_mass / dist_sq;
            acc = acc - r.normalized() * force_mag;
            let _ = dist; // used via dist_sq
        }
        acc
    };

    let mut acc = accel_at(pos);

    for _ in 0..steps {
        // Velocity Verlet integration
        let half_vel = vel + acc * (dt * 0.5);
        pos = pos + half_vel * dt;
        let new_acc = accel_at(pos);
        vel = half_vel + new_acc * (dt * 0.5);
        acc = new_acc;

        points.push(pos);

        // Early termination if trajectory reaches a very small radius (swallowed)
        let mut swallowed = false;
        for &(bh_pos, _) in black_holes {
            if pos.distance(bh_pos) < SOFTENING * 0.5 {
                swallowed = true;
                break;
            }
        }
        if swallowed {
            break;
        }
    }

    points
}

/// Determine the trajectory safety color based on periapsis distance relative
/// to Schwarzschild radii.
///
/// - Green if periapsis > 5 r_s (safe orbit)
/// - Yellow if periapsis is between 3 and 5 r_s (caution)
/// - Red if periapsis < 3 r_s (danger, near last stable orbit)
pub fn trajectory_safety_color(periapsis: f64, schwarzschild_radius: f64) -> Color {
    let ratio = periapsis / schwarzschild_radius;
    if ratio > 5.0 {
        Color::GREEN
    } else if ratio > 3.0 {
        let t = ((5.0 - ratio) / 2.0) as f32;
        Color::GREEN.lerp(Color::YELLOW, t)
    } else {
        let t = ((3.0 - ratio) / 3.0).clamp(0.0, 1.0) as f32;
        Color::YELLOW.lerp(Color::RED, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_preview_length() {
        let points = compute_trajectory_preview(
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 2.0),
            &[(Vec2::ZERO, 1.0)],
            0.016,
            100,
        );
        // Should have initial point + up to 100 steps
        assert!(points.len() >= 2);
        assert!(points.len() <= 101);
    }

    #[test]
    fn test_trajectory_preview_no_black_holes() {
        // Straight line with no gravity
        let points = compute_trajectory_preview(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            &[],
            1.0,
            10,
        );
        assert_eq!(points.len(), 11);
        // Final position should be approximately (10, 0)
        let last = points.last().unwrap();
        assert!((last.x - 10.0).abs() < 1e-6);
        assert!(last.y.abs() < 1e-6);
    }

    #[test]
    fn test_safety_color_safe() {
        let c = trajectory_safety_color(6.0, 1.0);
        // Should be green
        assert!(c.g > c.r);
    }

    #[test]
    fn test_safety_color_caution() {
        let c = trajectory_safety_color(4.0, 1.0);
        // Should be between green and yellow
        assert!(c.g > 0.5);
    }

    #[test]
    fn test_safety_color_danger() {
        let c = trajectory_safety_color(1.0, 1.0);
        // Should be red-ish
        assert!(c.r > c.g);
    }
}
