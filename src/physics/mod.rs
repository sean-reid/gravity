pub mod gravity;
pub mod integrator;
pub mod dilation;
pub mod orbit;
pub mod collision;

pub use gravity::{compute_gravitational_acceleration, compute_gravitational_acceleration_single};
pub use integrator::{VerletState, integrate_step, integrate_step_with_thrust};
pub use dilation::{compute_tau, compute_steps_per_frame};
pub use orbit::{OrbitalParams, compute_orbital_params, compute_circular_orbit_velocity, compute_escape_velocity};
pub use collision::{
    circle_circle, check_event_horizon, check_escape, ray_circle_intersection,
    KILL_FACTOR, MAX_RADIUS, ABYSS_RADIUS_FACTOR, LAST_STABLE_ORBIT,
};
