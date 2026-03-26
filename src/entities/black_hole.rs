use crate::util::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct BlackHole {
    pub position: Vec2,
    pub mass: f64,
    pub schwarzschild_radius: f64,
    /// Distance from center of mass (for binary systems).
    pub orbital_radius: f64,
    /// Current orbital angle (for binary systems).
    pub orbital_phase: f64,
    /// Angular velocity in radians per coordinate second (0 for static single BH).
    pub orbital_speed: f64,
}

impl BlackHole {
    /// Create a new static (non-orbiting) black hole.
    pub fn new(position: Vec2, mass: f64, schwarzschild_radius: f64) -> Self {
        Self {
            position,
            mass,
            schwarzschild_radius,
            orbital_radius: 0.0,
            orbital_phase: 0.0,
            orbital_speed: 0.0,
        }
    }

    /// Create a black hole that orbits the center of mass (for binary systems).
    pub fn new_binary_member(
        mass: f64,
        schwarzschild_radius: f64,
        orbital_radius: f64,
        initial_phase: f64,
        orbital_speed: f64,
    ) -> Self {
        let position = Vec2::new(
            orbital_radius * initial_phase.cos(),
            orbital_radius * initial_phase.sin(),
        );
        Self {
            position,
            mass,
            schwarzschild_radius,
            orbital_radius,
            orbital_phase: initial_phase,
            orbital_speed,
        }
    }

    /// Update position for binary orbit. `dt_coord` is coordinate-time delta.
    pub fn update(&mut self, dt_coord: f64) {
        if self.orbital_speed.abs() < 1e-12 {
            return;
        }
        self.orbital_phase += self.orbital_speed * dt_coord;
        // Keep phase in [0, 2*pi)
        self.orbital_phase %= std::f64::consts::TAU;
        if self.orbital_phase < 0.0 {
            self.orbital_phase += std::f64::consts::TAU;
        }
        self.position = Vec2::new(
            self.orbital_radius * self.orbital_phase.cos(),
            self.orbital_radius * self.orbital_phase.sin(),
        );
    }

    /// Return the (position, mass) pair used by the gravity system.
    pub fn as_gravity_source(&self) -> (Vec2, f64) {
        (self.position, self.mass)
    }

    /// Return the (position, schwarzschild_radius) pair used by the dilation system.
    pub fn as_dilation_source(&self) -> (Vec2, f64) {
        (self.position, self.schwarzschild_radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_bh_does_not_move() {
        let mut bh = BlackHole::new(Vec2::new(1.0, 0.0), 100.0, 1.0);
        let original_pos = bh.position;
        bh.update(1.0);
        assert!((bh.position.x - original_pos.x).abs() < 1e-12);
        assert!((bh.position.y - original_pos.y).abs() < 1e-12);
    }

    #[test]
    fn test_binary_orbit_updates() {
        let mut bh = BlackHole::new_binary_member(50.0, 0.5, 2.0, 0.0, 1.0);
        assert!((bh.position.x - 2.0).abs() < 1e-10);
        assert!(bh.position.y.abs() < 1e-10);

        bh.update(std::f64::consts::FRAC_PI_2);
        assert!(bh.position.x.abs() < 1e-10);
        assert!((bh.position.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_gravity_source() {
        let bh = BlackHole::new(Vec2::new(3.0, 4.0), 100.0, 1.0);
        let (pos, mass) = bh.as_gravity_source();
        assert!((pos.x - 3.0).abs() < 1e-12);
        assert!((mass - 100.0).abs() < 1e-12);
    }

    #[test]
    fn test_dilation_source() {
        let bh = BlackHole::new(Vec2::new(3.0, 4.0), 100.0, 1.5);
        let (pos, rs) = bh.as_dilation_source();
        assert!((pos.x - 3.0).abs() < 1e-12);
        assert!((rs - 1.5).abs() < 1e-12);
    }
}
