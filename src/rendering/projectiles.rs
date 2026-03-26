use crate::entities::projectile::{Projectile, ProjectileType};
use crate::util::Color;
use super::pipelines::sprite::ShipInstance;

/// Build a sprite instance from a Projectile.
/// Applies blueshift/redshift based on tau_at_launch.
pub fn projectile_instance(proj: &Projectile) -> ShipInstance {
    let base_color = match proj.projectile_type {
        ProjectileType::Railgun => Color::CYAN,
        ProjectileType::MassDriver => Color::ORANGE,
        ProjectileType::PhotonLance => Color::WHITE, // rendered as beam, not sprite
        ProjectileType::ImpulseRocket => Color::YELLOW,
        ProjectileType::GravityBomb => {
            if proj.bomb_active {
                Color::MAGENTA
            } else {
                Color::rgb(0.6, 0.2, 0.8)
            }
        }
        ProjectileType::TidalMine => {
            if proj.mine_orbiting {
                Color::RED
            } else {
                Color::DIM_RED
            }
        }
    };

    // Apply relativistic color shift based on tau_at_launch.
    // tau < 1 means time is slower (deeper in gravity well) -> blueshift for outgoing.
    // tau > 1 is not physically possible in our model, but handle gracefully.
    let color = if proj.tau_at_launch < 0.8 {
        let shift = (1.0 - proj.tau_at_launch as f32) * 1.5;
        base_color.blueshift(shift)
    } else if proj.tau_at_launch > 1.2 {
        let shift = (proj.tau_at_launch as f32 - 1.0) * 1.5;
        base_color.redshift(shift)
    } else {
        base_color
    };

    let rotation = proj.velocity.angle() as f32 - std::f32::consts::FRAC_PI_2;

    let scale = match proj.projectile_type {
        ProjectileType::Railgun => 0.1,
        ProjectileType::MassDriver => 0.18,
        ProjectileType::PhotonLance => 0.05,
        ProjectileType::ImpulseRocket => 0.14,
        ProjectileType::GravityBomb => {
            if proj.bomb_active { 0.25 } else { 0.15 }
        }
        ProjectileType::TidalMine => 0.12,
    };

    ShipInstance {
        position: proj.position.as_f32_array(),
        rotation,
        scale,
        color: color.to_array(),
        shield_alpha: 0.0,
        thrust_type: 0,
        thrust_magnitude: match proj.projectile_type {
            ProjectileType::ImpulseRocket => 0.8,
            _ => 0.0,
        },
        turret_angle: 0.0,
    }
}
