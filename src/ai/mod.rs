pub mod personality;
pub mod targeting;
pub mod steering;
pub mod decision;

pub use personality::{PersonalityParams, get_personality};
pub use targeting::{compute_lead_target, compute_threat_level};
pub use steering::{
    compute_thrust_for_altitude_change,
    compute_orbit_maintenance_thrust,
    compute_formation_thrust,
    compute_evasion_thrust,
};
pub use decision::{AiContext, AiOutput, run_ai_tick};
