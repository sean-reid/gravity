use crate::entities::ship::{PlayerShip, ThrustDirection};
use crate::entities::bot::{Bot, BotArchetype};
use crate::util::Color;
use super::pipelines::sprite::ShipInstance;

/// Build a ShipInstance from a PlayerShip.
pub fn ship_instance_from_player(ship: &PlayerShip) -> ShipInstance {
    // The chevron model points in +Y; velocity.angle() returns the +X direction.
    // Subtract PI/2 so the nose aligns with the velocity vector.
    let rotation = ship.velocity.angle() as f32 - std::f32::consts::FRAC_PI_2;
    let (thrust_type, thrust_magnitude) = thrust_params(ship.thrust_direction);

    ShipInstance {
        position: ship.position.as_f32_array(),
        rotation,
        scale: 0.3,
        color: Color::player().to_array(),
        shield_alpha: (ship.shields / 100.0) as f32,
        thrust_type,
        thrust_magnitude,
        turret_angle: (ship.turret_angle - ship.velocity.angle()) as f32,
    }
}

/// Build a ShipInstance from a Bot.
pub fn ship_instance_from_bot(bot: &Bot) -> ShipInstance {
    let rotation = bot.velocity.angle() as f32 - std::f32::consts::FRAC_PI_2;

    let color = match bot.archetype {
        BotArchetype::Skirmisher => Color::skirmisher(),
        BotArchetype::Diver => Color::diver(),
        BotArchetype::Vulture => Color::vulture(),
        BotArchetype::Anchor => Color::anchor(),
        BotArchetype::Swarm => Color::swarm(),
        BotArchetype::Commander => Color::commander(),
    };

    let scale = match bot.archetype {
        BotArchetype::Swarm => 0.2,
        BotArchetype::Commander => 0.4,
        _ => 0.3,
    };

    let shield_alpha = if bot.max_shields > 0.0 {
        (bot.shields / bot.max_shields) as f32
    } else {
        0.0
    };

    ShipInstance {
        position: bot.position.as_f32_array(),
        rotation,
        scale,
        color: color.to_array(),
        shield_alpha,
        thrust_type: 0, // bots use chemical thrust visuals
        thrust_magnitude: if bot.acceleration.length() > 0.1 { 0.6 } else { 0.0 },
        turret_angle: (bot.turret_angle - bot.velocity.angle()) as f32,
    }
}

/// Convert a ThrustDirection into (thrust_type, thrust_magnitude).
fn thrust_params(dir: ThrustDirection) -> (u32, f32) {
    match dir {
        ThrustDirection::None => (0, 0.0),
        ThrustDirection::Prograde => (0, 1.0),
        ThrustDirection::Retrograde => (1, 1.0),
        ThrustDirection::RadialIn => (0, 0.7),
        ThrustDirection::RadialOut => (0, 0.7),
    }
}
