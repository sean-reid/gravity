use crate::util::Vec2;

/// Orbital parameters for a body orbiting a single attractor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OrbitalParams {
    /// Distance from the body to the attractor surface (center-to-center distance).
    pub altitude: f64,
    /// Current orbital speed (magnitude of velocity).
    pub orbital_velocity: f64,
    /// Eccentricity of the orbit (0 = circular, 0..1 = elliptical, 1 = parabolic, >1 = hyperbolic).
    pub eccentricity: f64,
    /// Closest approach distance (center-to-center).
    pub periapsis: f64,
    /// Farthest distance (center-to-center). Infinite for parabolic/hyperbolic orbits, stored as f64::INFINITY.
    pub apoapsis: f64,
    /// Semi-major axis. Negative for hyperbolic orbits.
    pub semi_major_axis: f64,
}

/// Compute orbital parameters for a body at `pos` with velocity `vel` orbiting a
/// black hole at `bh_pos` with `bh_mass`, using gravitational constant `g`.
///
/// Uses the vis-viva equation and standard Keplerian orbital mechanics.
pub fn compute_orbital_params(
    pos: Vec2,
    vel: Vec2,
    bh_pos: Vec2,
    bh_mass: f64,
    g: f64,
) -> OrbitalParams {
    let r_vec = pos - bh_pos;
    let r = r_vec.length();
    let v = vel.length();
    let mu = g * bh_mass; // standard gravitational parameter

    // Specific orbital energy: E = v^2/2 - mu/r
    let specific_energy = 0.5 * v * v - mu / r;

    // Specific angular momentum (scalar, 2D cross product): h = r x v
    let h = r_vec.cross(vel);

    // Semi-major axis from vis-viva: a = -mu / (2*E)
    let semi_major_axis = if specific_energy.abs() < 1e-12 {
        // Parabolic orbit (E ~ 0), semi-major axis is effectively infinite
        f64::INFINITY
    } else {
        -mu / (2.0 * specific_energy)
    };

    // Eccentricity: e = sqrt(1 + 2*E*h^2 / mu^2)
    let ecc_squared = 1.0 + 2.0 * specific_energy * h * h / (mu * mu);
    let eccentricity = if ecc_squared < 0.0 {
        0.0 // numerical protection
    } else {
        ecc_squared.sqrt()
    };

    // Periapsis and apoapsis
    let periapsis;
    let apoapsis;

    if specific_energy.abs() < 1e-12 {
        // Parabolic: periapsis = h^2 / (2*mu)
        periapsis = h * h / (2.0 * mu);
        apoapsis = f64::INFINITY;
    } else if specific_energy < 0.0 {
        // Bound orbit (elliptical or circular)
        periapsis = semi_major_axis * (1.0 - eccentricity);
        apoapsis = semi_major_axis * (1.0 + eccentricity);
    } else {
        // Hyperbolic orbit
        periapsis = semi_major_axis * (1.0 - eccentricity); // a is negative, so this works out
        apoapsis = f64::INFINITY;
    }

    OrbitalParams {
        altitude: r,
        orbital_velocity: v,
        eccentricity,
        periapsis: periapsis.abs(), // ensure positive for hyperbolic case
        apoapsis,
        semi_major_axis,
    }
}

/// Compute the velocity needed for a circular orbit at `radius` around a body
/// with `bh_mass`, using gravitational constant `g`.
///
/// v_circ = sqrt(G * M / r)
pub fn compute_circular_orbit_velocity(radius: f64, bh_mass: f64, g: f64) -> f64 {
    (g * bh_mass / radius).sqrt()
}

/// Compute the escape velocity at `radius` from a body with `bh_mass`,
/// using gravitational constant `g`.
///
/// v_esc = sqrt(2 * G * M / r)
pub fn compute_escape_velocity(radius: f64, bh_mass: f64, g: f64) -> f64 {
    (2.0 * g * bh_mass / radius).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Vec2;

    const G_TEST: f64 = 50.0;

    #[test]
    fn test_circular_orbit_eccentricity() {
        let radius = 10.0;
        let bh_mass = 1.0;
        let v_circ = compute_circular_orbit_velocity(radius, bh_mass, G_TEST);

        let params = compute_orbital_params(
            Vec2::new(radius, 0.0),
            Vec2::new(0.0, v_circ),
            Vec2::ZERO,
            bh_mass,
            G_TEST,
        );

        assert!(params.eccentricity < 0.01, "Circular orbit should have ~0 eccentricity, got {}", params.eccentricity);
        assert!((params.altitude - radius).abs() < 1e-10);
        assert!((params.periapsis - radius).abs() < 0.1);
        assert!((params.apoapsis - radius).abs() < 0.1);
    }

    #[test]
    fn test_escape_velocity_gives_parabolic() {
        let radius = 10.0;
        let bh_mass = 1.0;
        let v_esc = compute_escape_velocity(radius, bh_mass, G_TEST);

        let params = compute_orbital_params(
            Vec2::new(radius, 0.0),
            Vec2::new(0.0, v_esc),
            Vec2::ZERO,
            bh_mass,
            G_TEST,
        );

        // Should be approximately parabolic (e ~ 1.0)
        assert!((params.eccentricity - 1.0).abs() < 0.01,
            "Escape velocity should give e~1, got {}", params.eccentricity);
    }

    #[test]
    fn test_escape_velocity_is_sqrt2_times_circular() {
        let radius = 10.0;
        let bh_mass = 1.0;
        let v_circ = compute_circular_orbit_velocity(radius, bh_mass, G_TEST);
        let v_esc = compute_escape_velocity(radius, bh_mass, G_TEST);
        assert!((v_esc / v_circ - std::f64::consts::SQRT_2).abs() < 1e-10);
    }

    #[test]
    fn test_hyperbolic_orbit() {
        let radius = 10.0;
        let bh_mass = 1.0;
        let v_hyp = compute_escape_velocity(radius, bh_mass, G_TEST) * 1.5;

        let params = compute_orbital_params(
            Vec2::new(radius, 0.0),
            Vec2::new(0.0, v_hyp),
            Vec2::ZERO,
            bh_mass,
            G_TEST,
        );

        assert!(params.eccentricity > 1.0, "Hyperbolic orbit should have e>1, got {}", params.eccentricity);
        assert!(params.apoapsis.is_infinite());
        assert!(params.semi_major_axis < 0.0, "Hyperbolic orbit should have negative semi-major axis");
    }
}
