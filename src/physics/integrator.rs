use crate::util::Vec2;
use super::gravity::compute_gravitational_acceleration;

/// State for velocity Verlet integration of a single body.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VerletState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
}

impl VerletState {
    pub fn new(position: Vec2, velocity: Vec2) -> Self {
        Self {
            position,
            velocity,
            acceleration: Vec2::ZERO,
        }
    }

    /// Initialize acceleration from the current gravitational field.
    pub fn initialize_acceleration(&mut self, bodies: &[(Vec2, f64)]) {
        self.acceleration = compute_gravitational_acceleration(self.position, bodies);
    }
}

/// Advance the state by one velocity Verlet step:
///   1. r(t+dt) = r(t) + v(t)*dt + 0.5*a(t)*dt^2
///   2. a(t+dt) = gravity(r(t+dt))
///   3. v(t+dt) = v(t) + 0.5*(a(t) + a(t+dt))*dt
pub fn integrate_step(state: &mut VerletState, dt: f64, bodies: &[(Vec2, f64)]) {
    // Step 1: update position
    state.position = state.position + state.velocity * dt + state.acceleration * (0.5 * dt * dt);

    // Step 2: compute new acceleration
    let new_acc = compute_gravitational_acceleration(state.position, bodies);

    // Step 3: update velocity using average of old and new acceleration
    state.velocity = state.velocity + (state.acceleration + new_acc) * (0.5 * dt);

    // Store new acceleration for next step
    state.acceleration = new_acc;
}

/// Advance the state by one velocity Verlet step with an additional thrust force.
/// Thrust is treated as a constant external acceleration over the timestep.
pub fn integrate_step_with_thrust(
    state: &mut VerletState,
    dt: f64,
    bodies: &[(Vec2, f64)],
    thrust: Vec2,
) {
    let total_acc = state.acceleration + thrust;

    // Step 1: update position using total acceleration (gravity + thrust)
    state.position = state.position + state.velocity * dt + total_acc * (0.5 * dt * dt);

    // Step 2: compute new gravitational acceleration at new position, add thrust
    let new_grav_acc = compute_gravitational_acceleration(state.position, bodies);
    let new_total_acc = new_grav_acc + thrust;

    // Step 3: update velocity using average of old and new total acceleration
    state.velocity = state.velocity + (total_acc + new_total_acc) * (0.5 * dt);

    // Store new gravitational acceleration (without thrust) for next step
    state.acceleration = new_grav_acc;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_free_body_constant_velocity() {
        let mut state = VerletState::new(Vec2::ZERO, Vec2::new(1.0, 0.0));
        let bodies: &[(Vec2, f64)] = &[];
        for _ in 0..100 {
            integrate_step(&mut state, 0.01, bodies);
        }
        // Should travel ~1.0 units in x
        assert!((state.position.x - 1.0).abs() < 1e-10);
        assert!(state.position.y.abs() < 1e-10);
    }

    #[test]
    fn test_thrust_accelerates() {
        let mut state = VerletState::new(Vec2::ZERO, Vec2::ZERO);
        let bodies: &[(Vec2, f64)] = &[];
        let thrust = Vec2::new(1.0, 0.0);
        for _ in 0..100 {
            integrate_step_with_thrust(&mut state, 0.01, bodies, thrust);
        }
        // After 1s of constant acceleration a=1, x = 0.5*a*t^2 = 0.5
        assert!((state.position.x - 0.5).abs() < 1e-4);
        // Velocity should be ~1.0
        assert!((state.velocity.x - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_circular_orbit_stability() {
        // Set up a circular orbit around a central body
        let bh_mass = 1.0;
        let radius = 10.0;
        let v_circ = (super::super::gravity::G * bh_mass / radius).sqrt();
        let bodies = vec![(Vec2::ZERO, bh_mass)];

        let mut state = VerletState::new(
            Vec2::new(radius, 0.0),
            Vec2::new(0.0, v_circ),
        );
        state.initialize_acceleration(&bodies);

        let dt = 0.001;
        let initial_radius = state.position.length();
        for _ in 0..10000 {
            integrate_step(&mut state, dt, &bodies);
        }
        let final_radius = state.position.length();
        // Radius should stay approximately constant for a circular orbit
        assert!((final_radius - initial_radius).abs() / initial_radius < 0.01);
    }
}
