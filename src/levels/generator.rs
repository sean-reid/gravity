use crate::util::{Vec2, Rng};
use crate::util::rng::hash_seeds;
use crate::entities::bot::BotArchetype;
use crate::weapons::WeaponType;

use super::config::{LevelConfig, BlackHoleConfig, BotSpawn};
use super::difficulty::{max_dilation_for_level, bot_count_for_level};

/// Primary Schwarzschild radius. Larger values create stronger dilation
/// at gameplay distances. At 3.0, a player at r=8 experiences tau=0.79 (1.27x),
/// at r=5 experiences tau=0.63 (1.58x), at r=4 experiences tau=0.50 (2.0x).
const PRIMARY_RS: f64 = 3.0;

/// Generate a complete level configuration from a level number and base seed.
pub fn generate_level(level_number: u32, base_seed: u64) -> LevelConfig {
    let level_seed = hash_seeds(base_seed, level_number as u64);
    let mut rng = Rng::new(level_seed);

    let act = determine_act(level_number);
    let black_holes = generate_black_holes(level_number, &mut rng);
    let max_dilation_factor = max_dilation_for_level(level_number);

    // Abyss radius shrinks as difficulty increases (deeper wells)
    let abyss_radius_factor = 2.0 + (1.0 - super::difficulty::difficulty(level_number)) * 1.5;

    let bot_spawns = generate_bot_spawns(level_number, &mut rng, &black_holes);
    let weapons_available = weapons_for_level(level_number);

    // Player start: altitude 8-10 r_s, random phase, at least 3 r_s from nearest bot
    let (player_start_altitude, player_start_phase) =
        generate_player_start(&mut rng, &bot_spawns);

    LevelConfig {
        seed: level_seed,
        level_number,
        act,
        black_holes,
        max_dilation_factor,
        abyss_radius_factor,
        bot_spawns,
        weapons_available,
        player_start_altitude,
        player_start_phase,
    }
}

/// Determine which act this level belongs to.
fn determine_act(level: u32) -> u32 {
    match level {
        1..=10 => 1,
        11..=20 => 2,
        21..=35 => 3,
        _ => 4,
    }
}

/// Generate black hole configurations based on level range.
fn generate_black_holes(level: u32, rng: &mut Rng) -> Vec<BlackHoleConfig> {
    let binary = should_be_binary(level, rng);

    if binary {
        generate_binary_bh(rng)
    } else {
        generate_single_bh()
    }
}

/// Determine whether the level should have a binary black hole system.
fn should_be_binary(level: u32, rng: &mut Rng) -> bool {
    let binary_chance = match level {
        1..=10 => 0.0,
        11..=20 => 0.3,
        21..=35 => 0.5,
        _ => 0.7,
    };
    rng.chance(binary_chance)
}

/// Generate a single static black hole at origin.
fn generate_single_bh() -> Vec<BlackHoleConfig> {
    vec![BlackHoleConfig {
        mass: 1.0,
        position: Vec2::new(0.0, 0.0),
        schwarzschild_radius: PRIMARY_RS,
        orbital_radius: 0.0,
        orbital_phase: 0.0,
        orbital_speed: 0.0,
    }]
}

/// Generate a binary black hole system orbiting a common center of mass.
fn generate_binary_bh(rng: &mut Rng) -> Vec<BlackHoleConfig> {
    let primary_mass = 1.0;
    let secondary_mass = rng.range_f64(0.3, 1.0);
    let total_mass = primary_mass + secondary_mass;

    // Separation distance in units of r_s (8-20)
    let separation = rng.range_f64(8.0, 20.0) * PRIMARY_RS;

    // Orbital radii from center of mass (r1 * m1 = r2 * m2)
    let r_primary = separation * secondary_mass / total_mass;
    let r_secondary = separation * primary_mass / total_mass;

    // Schwarzschild radii proportional to mass
    let rs_primary = PRIMARY_RS;
    let rs_secondary = PRIMARY_RS * secondary_mass;

    // Orbital speed: v = sqrt(G * M / r) scaled for game units
    // Using simplified Keplerian: omega = sqrt(total_mass / separation^3)
    let omega = (total_mass / (separation * separation * separation)).sqrt();

    let initial_phase = rng.angle();

    vec![
        BlackHoleConfig {
            mass: primary_mass,
            position: Vec2::new(
                r_primary * initial_phase.cos(),
                r_primary * initial_phase.sin(),
            ),
            schwarzschild_radius: rs_primary,
            orbital_radius: r_primary,
            orbital_phase: initial_phase,
            orbital_speed: omega,
        },
        BlackHoleConfig {
            mass: secondary_mass,
            position: Vec2::new(
                -r_secondary * (initial_phase).cos(),
                -r_secondary * (initial_phase).sin(),
            ),
            schwarzschild_radius: rs_secondary,
            orbital_radius: r_secondary,
            orbital_phase: initial_phase + std::f64::consts::PI,
            orbital_speed: omega,
        },
    ]
}

/// Generate bot spawn configurations for the level.
fn generate_bot_spawns(
    level: u32,
    rng: &mut Rng,
    black_holes: &[BlackHoleConfig],
) -> Vec<BotSpawn> {
    let count = bot_count_for_level(level) as usize;
    let mut spawns = Vec::with_capacity(count);

    // Determine if this level gets a Commander
    let has_commander = level >= 18 && {
        // First Commander at 18, then every 8-12 levels
        if level == 18 {
            true
        } else {
            let since_18 = level - 18;
            // Check if we're in a Commander window (every 8-12 levels)
            since_18 % 10 <= 2 && since_18 >= 8
        }
    };

    let mut swarm_group_counter = 0u32;

    for i in 0..count {
        let archetype = if has_commander && i == count - 1 {
            // Commander is always the last spawn (boss-like)
            BotArchetype::Commander
        } else {
            pick_archetype(level, rng)
        };

        // Altitude: bots orbit between 4 and 12 r_s from the primary
        let min_alt = 4.0 * PRIMARY_RS;
        let max_alt = 12.0 * PRIMARY_RS;
        let altitude = rng.range_f64(min_alt, max_alt);

        let phase = rng.angle();

        // Group Swarm bots together
        let swarm_group = if archetype == BotArchetype::Swarm {
            let group = swarm_group_counter;
            // Every 3 swarm bots share a group
            if spawns
                .iter()
                .filter(|s: &&BotSpawn| {
                    s.archetype == BotArchetype::Swarm && s.swarm_group == Some(group)
                })
                .count()
                >= 2
            {
                swarm_group_counter += 1;
            }
            Some(swarm_group_counter)
        } else {
            None
        };

        spawns.push(BotSpawn {
            archetype,
            altitude,
            phase,
            swarm_group,
        });
    }

    // Ignore black_holes for now in spawn placement; they are at/near origin
    let _ = black_holes;

    spawns
}

/// Pick a bot archetype based on weighted selection that varies with level.
fn pick_archetype(level: u32, rng: &mut Rng) -> BotArchetype {
    // Weights: [Skirmisher, Diver, Vulture, Anchor, Swarm]
    // Early levels: heavy Skirmisher bias. Later: more diverse.
    let weights = match level {
        1..=3 => [100, 0, 0, 0, 0],
        4..=5 => [70, 20, 5, 5, 0],
        6..=10 => [40, 20, 15, 15, 10],
        11..=20 => [25, 20, 20, 15, 20],
        21..=35 => [15, 20, 20, 20, 25],
        _ => [10, 20, 20, 25, 25],
    };

    let total: u32 = weights.iter().sum();
    let roll = rng.range_u32(total);

    let mut cumulative = 0;
    let archetypes = [
        BotArchetype::Skirmisher,
        BotArchetype::Diver,
        BotArchetype::Vulture,
        BotArchetype::Anchor,
        BotArchetype::Swarm,
    ];

    for (i, &w) in weights.iter().enumerate() {
        cumulative += w;
        if roll < cumulative {
            return archetypes[i];
        }
    }

    BotArchetype::Skirmisher
}

/// Return the list of weapons available at a given level.
fn weapons_for_level(level: u32) -> Vec<WeaponType> {
    let mut weapons = vec![WeaponType::Railgun];
    if level >= 3 {
        weapons.push(WeaponType::MassDriver);
    }
    if level >= 6 {
        weapons.push(WeaponType::PhotonLance);
    }
    if level >= 9 {
        weapons.push(WeaponType::GravityBomb);
    }
    if level >= 12 {
        weapons.push(WeaponType::ImpulseRocket);
    }
    if level >= 15 {
        weapons.push(WeaponType::TidalMine);
    }
    weapons
}

/// Generate player start position. Altitude 8-10 r_s, random phase,
/// ensuring at least 3 r_s from the nearest bot.
fn generate_player_start(rng: &mut Rng, bots: &[BotSpawn]) -> (f64, f64) {
    let min_separation = 3.0 * PRIMARY_RS;

    for _ in 0..100 {
        let altitude = rng.range_f64(8.0, 10.0) * PRIMARY_RS;
        let phase = rng.angle();

        let player_pos = Vec2::new(altitude * phase.cos(), altitude * phase.sin());

        let too_close = bots.iter().any(|bot| {
            let bot_pos = Vec2::new(
                bot.altitude * bot.phase.cos(),
                bot.altitude * bot.phase.sin(),
            );
            let dx = player_pos.x - bot_pos.x;
            let dy = player_pos.y - bot_pos.y;
            (dx * dx + dy * dy).sqrt() < min_separation
        });

        if !too_close {
            return (altitude, phase);
        }
    }

    // Fallback: just use a safe default
    let altitude = 9.0 * PRIMARY_RS;
    let phase = rng.angle();
    (altitude, phase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_level_1_is_single_bh() {
        let config = generate_level(1, 42);
        assert_eq!(config.level_number, 1);
        assert_eq!(config.act, 1);
        assert_eq!(config.black_holes.len(), 1);
        assert!(config.weapons_available.contains(&WeaponType::Railgun));
        assert!(!config.weapons_available.contains(&WeaponType::MassDriver));
    }

    #[test]
    fn generate_level_is_deterministic() {
        let a = generate_level(10, 12345);
        let b = generate_level(10, 12345);
        assert_eq!(a.seed, b.seed);
        assert_eq!(a.bot_spawns.len(), b.bot_spawns.len());
        assert!((a.player_start_altitude - b.player_start_altitude).abs() < 1e-12);
        assert!((a.player_start_phase - b.player_start_phase).abs() < 1e-12);
    }

    #[test]
    fn weapons_unlock_progression() {
        assert_eq!(weapons_for_level(1).len(), 1);
        assert_eq!(weapons_for_level(3).len(), 2);
        assert_eq!(weapons_for_level(6).len(), 3);
        assert_eq!(weapons_for_level(9).len(), 4);
        assert_eq!(weapons_for_level(12).len(), 5);
        assert_eq!(weapons_for_level(15).len(), 6);
    }

    #[test]
    fn act_assignment() {
        assert_eq!(determine_act(1), 1);
        assert_eq!(determine_act(10), 1);
        assert_eq!(determine_act(11), 2);
        assert_eq!(determine_act(20), 2);
        assert_eq!(determine_act(21), 3);
        assert_eq!(determine_act(35), 3);
        assert_eq!(determine_act(36), 4);
    }
}
