use crate::util::Vec2;

/// Multiplier for event horizon kill radius (slightly outside r_s for gameplay).
pub const KILL_FACTOR: f64 = 1.05;

/// Maximum allowed distance from center of mass before an entity is considered escaped.
pub const MAX_RADIUS: f64 = 50.0;

/// Visual radius factor for the abyss/accretion disk effect around black holes.
pub const ABYSS_RADIUS_FACTOR: f64 = 3.0;

/// Innermost stable circular orbit in Schwarzschild radii.
pub const LAST_STABLE_ORBIT: f64 = 3.0;

/// Test whether two circles overlap.
pub fn circle_circle(pos_a: Vec2, radius_a: f64, pos_b: Vec2, radius_b: f64) -> bool {
    let dist_sq = pos_a.distance_squared(pos_b);
    let radii_sum = radius_a + radius_b;
    dist_sq <= radii_sum * radii_sum
}

/// Check if an entity has crossed (or is inside) the event horizon kill boundary.
///
/// The kill boundary is `bh_rs * kill_factor` from the black hole center.
/// Returns true if the entity should be destroyed.
pub fn check_event_horizon(entity_pos: Vec2, bh_pos: Vec2, bh_rs: f64, kill_factor: f64) -> bool {
    let dist = entity_pos.distance(bh_pos);
    dist <= bh_rs * kill_factor
}

/// Check if an entity has escaped beyond the arena boundary.
///
/// Returns true if the entity is farther than `max_radius` from the center of mass.
pub fn check_escape(entity_pos: Vec2, center_of_mass: Vec2, max_radius: f64) -> bool {
    let dist = entity_pos.distance(center_of_mass);
    dist >= max_radius
}

/// Compute the intersection of a ray with a circle.
///
/// Ray is defined by `ray_origin + t * ray_dir` for t >= 0.
/// `ray_dir` should be normalized.
///
/// Returns `Some(t)` with the nearest intersection distance, or `None` if no intersection.
pub fn ray_circle_intersection(
    ray_origin: Vec2,
    ray_dir: Vec2,
    circle_center: Vec2,
    circle_radius: f64,
) -> Option<f64> {
    // Vector from ray origin to circle center
    let oc = ray_origin - circle_center;

    // Quadratic coefficients: |ray_origin + t*ray_dir - circle_center|^2 = r^2
    // a*t^2 + b*t + c = 0
    let a = ray_dir.dot(ray_dir);
    let b = 2.0 * oc.dot(ray_dir);
    let c = oc.dot(oc) - circle_radius * circle_radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let inv_2a = 1.0 / (2.0 * a);

    // Try the nearer intersection first
    let t1 = (-b - sqrt_disc) * inv_2a;
    if t1 >= 0.0 {
        return Some(t1);
    }

    // If the nearer intersection is behind the ray, try the farther one
    // (ray origin is inside the circle)
    let t2 = (-b + sqrt_disc) * inv_2a;
    if t2 >= 0.0 {
        return Some(t2);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_circle_overlap() {
        assert!(circle_circle(Vec2::ZERO, 1.0, Vec2::new(1.5, 0.0), 1.0));
    }

    #[test]
    fn test_circle_circle_no_overlap() {
        assert!(!circle_circle(Vec2::ZERO, 1.0, Vec2::new(3.0, 0.0), 1.0));
    }

    #[test]
    fn test_circle_circle_touching() {
        assert!(circle_circle(Vec2::ZERO, 1.0, Vec2::new(2.0, 0.0), 1.0));
    }

    #[test]
    fn test_event_horizon_inside() {
        assert!(check_event_horizon(
            Vec2::new(0.5, 0.0), Vec2::ZERO, 1.0, KILL_FACTOR
        ));
    }

    #[test]
    fn test_event_horizon_outside() {
        assert!(!check_event_horizon(
            Vec2::new(2.0, 0.0), Vec2::ZERO, 1.0, KILL_FACTOR
        ));
    }

    #[test]
    fn test_event_horizon_at_boundary() {
        // At exactly kill_factor * r_s, should be on the boundary (inside)
        assert!(check_event_horizon(
            Vec2::new(KILL_FACTOR, 0.0), Vec2::ZERO, 1.0, KILL_FACTOR
        ));
    }

    #[test]
    fn test_escape_outside() {
        assert!(check_escape(Vec2::new(60.0, 0.0), Vec2::ZERO, MAX_RADIUS));
    }

    #[test]
    fn test_escape_inside() {
        assert!(!check_escape(Vec2::new(10.0, 0.0), Vec2::ZERO, MAX_RADIUS));
    }

    #[test]
    fn test_ray_circle_hit() {
        let t = ray_circle_intersection(
            Vec2::new(-5.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
            1.0,
        );
        assert!(t.is_some());
        let t = t.unwrap();
        assert!((t - 4.0).abs() < 1e-10); // hits at x = -1
    }

    #[test]
    fn test_ray_circle_miss() {
        let t = ray_circle_intersection(
            Vec2::new(-5.0, 5.0),
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
            1.0,
        );
        assert!(t.is_none());
    }

    #[test]
    fn test_ray_circle_origin_inside() {
        let t = ray_circle_intersection(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
            2.0,
        );
        assert!(t.is_some());
        let t = t.unwrap();
        assert!((t - 2.0).abs() < 1e-10); // exits at x = 2
    }

    #[test]
    fn test_ray_circle_behind() {
        // Circle is behind the ray
        let t = ray_circle_intersection(
            Vec2::new(5.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::ZERO,
            1.0,
        );
        assert!(t.is_none());
    }
}
