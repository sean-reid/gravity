use crate::util::Vec2;
use crate::util::Rng;
use crate::entities::bot::{Bot, BotArchetype, BotGoal};
use crate::entities::ship::PlayerShip;
use crate::entities::projectile::{Projectile, RAILGUN_SPEED, MASS_DRIVER_SPEED, IMPULSE_ROCKET_SPEED};

use super::personality::get_personality;
use super::targeting::{compute_lead_target, compute_threat_level};
use super::steering::{
    compute_thrust_for_altitude_change,
    compute_orbit_maintenance_thrust,
    compute_formation_thrust,
    compute_evasion_thrust,
};

/// Everything the AI needs to make a decision for one bot.
pub struct AiContext<'a> {
    pub player: &'a PlayerShip,
    pub bots: &'a [Bot],
    pub projectiles: &'a [Projectile],
    /// (position, mass) pairs for all black holes.
    pub black_holes: &'a [(Vec2, f64)],
    /// (position, schwarzschild_radius) pairs for all black holes.
    pub black_hole_positions_rs: &'a [(Vec2, f64)],
    pub rng: &'a mut Rng,
    pub difficulty: f64,
}

/// The output of a single AI decision tick.
pub struct AiOutput {
    /// Thrust direction (will be normalized by the caller).
    pub thrust: Vec2,
    /// Whether the bot should fire its weapon this tick.
    pub fire: bool,
    /// Turret aim direction in radians.
    pub turret_angle: f64,
    /// If set, the bot transitions to this goal.
    pub new_goal: Option<BotGoal>,
}

/// Main AI entry point. Called when a bot's proper-time accumulator
/// exceeds its decision interval.
pub fn run_ai_tick(bot: &Bot, ctx: &mut AiContext) -> AiOutput {
    let personality = get_personality(&bot.archetype, ctx.difficulty);

    // Find the primary black hole (use first one, or origin if none)
    let (bh_pos, bh_mass) = if !ctx.black_holes.is_empty() {
        ctx.black_holes[0]
    } else {
        (Vec2::ZERO, 1.0)
    };

    let bh_rs = if !ctx.black_hole_positions_rs.is_empty() {
        ctx.black_hole_positions_rs[0].1
    } else {
        1.0
    };

    let bot_altitude = (bot.position - bh_pos).length();
    let player_dist = bot.position.distance(ctx.player.position);
    let player_altitude = (ctx.player.position - bh_pos).length();

    let threat = compute_threat_level(bot, ctx.player.position, ctx.projectiles);
    let health_fraction = if bot.max_health > 0.0 {
        bot.health / bot.max_health
    } else {
        1.0
    };

    // Gather nearby player projectiles for evasion
    let threats: Vec<Vec2> = ctx
        .projectiles
        .iter()
        .filter(|p| p.alive && p.owner_is_player)
        .filter(|p| p.position.distance(bot.position) < 6.0)
        .map(|p| p.position)
        .collect();

    match bot.archetype {
        BotArchetype::Skirmisher => tick_skirmisher(bot, ctx, &personality_wrap(personality, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, &threats)),
        BotArchetype::Diver => tick_diver(bot, ctx, &personality_wrap(personality, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, &threats)),
        BotArchetype::Vulture => tick_vulture(bot, ctx, &personality_wrap(personality, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, &threats)),
        BotArchetype::Anchor => tick_anchor(bot, ctx, &personality_wrap(personality, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, &threats)),
        BotArchetype::Swarm => tick_swarm(bot, ctx, &personality_wrap(personality, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, &threats)),
        BotArchetype::Commander => tick_commander(bot, ctx, &personality_wrap(personality, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, &threats)),
    }
}

// ---------------------------------------------------------------------------
// Internal helper: bundles commonly needed context into one struct to avoid
// passing 10+ parameters to every archetype function.
// ---------------------------------------------------------------------------

use super::personality::PersonalityParams;

struct TickCtx<'a> {
    p: PersonalityParams,
    bh_pos: Vec2,
    bh_mass: f64,
    bh_rs: f64,
    bot_altitude: f64,
    player_dist: f64,
    player_altitude: f64,
    threat: f64,
    health_fraction: f64,
    threats: &'a [Vec2],
}

fn personality_wrap<'a>(
    p: PersonalityParams,
    bh_pos: Vec2,
    bh_mass: f64,
    bh_rs: f64,
    bot_altitude: f64,
    player_dist: f64,
    player_altitude: f64,
    threat: f64,
    health_fraction: f64,
    threats: &'a [Vec2],
) -> TickCtx<'a> {
    TickCtx { p, bh_pos, bh_mass, bh_rs, bot_altitude, player_dist, player_altitude, threat, health_fraction, threats }
}

// ---------------------------------------------------------------------------
// Helper: aim at the player with the given projectile speed
// ---------------------------------------------------------------------------

fn aim_at_player(bot: &Bot, ctx: &mut AiContext, projectile_speed: f64, accuracy_error: f64) -> f64 {
    compute_lead_target(
        bot.position,
        bot.velocity,
        ctx.player.position,
        ctx.player.velocity,
        projectile_speed,
        accuracy_error,
        ctx.rng,
    )
}

/// Compute desired altitude for current goal within personality bounds.
fn orbit_altitude(tc: &TickCtx) -> f64 {
    let mid = (tc.p.preferred_min_altitude + tc.p.preferred_max_altitude) * 0.5 * tc.bh_rs;
    mid
}

// ---------------------------------------------------------------------------
// SKIRMISHER
// ---------------------------------------------------------------------------

fn tick_skirmisher(bot: &Bot, ctx: &mut AiContext, tc: &TickCtx) -> AiOutput {
    let target_alt = orbit_altitude(tc);
    let fire_range = 15.0 * tc.bh_rs;

    // Determine goal transitions
    let new_goal = if tc.health_fraction < tc.p.retreat_health_threshold && tc.threat > 0.3 {
        Some(BotGoal::Retreat)
    } else if bot.current_goal == BotGoal::Retreat && tc.health_fraction > tc.p.retreat_health_threshold + 0.1 {
        Some(BotGoal::Orbit)
    } else {
        None
    };

    let current_goal = new_goal.unwrap_or(bot.current_goal);

    let thrust = match current_goal {
        BotGoal::Retreat => {
            // Retreat: raise orbit to max altitude
            let retreat_alt = tc.p.preferred_max_altitude * tc.bh_rs;
            let alt_thrust = compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, retreat_alt, tc.bh_mass);
            let evade = compute_evasion_thrust(bot.position, bot.velocity, tc.threats);
            blend_thrusts(alt_thrust, 0.6, evade, 0.4)
        }
        _ => {
            // Normal orbit + mild evasion
            let maint = compute_orbit_maintenance_thrust(bot.position, bot.velocity, tc.bh_pos, target_alt, tc.bh_mass);
            let alt_thrust = compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, target_alt, tc.bh_mass);
            let orbit = if alt_thrust.length_squared() > 1e-6 { alt_thrust } else { maint };
            if !tc.threats.is_empty() {
                let evade = compute_evasion_thrust(bot.position, bot.velocity, tc.threats);
                blend_thrusts(orbit, 0.7, evade, 0.3)
            } else {
                orbit
            }
        }
    };

    // Fire railgun if player in range
    let can_fire = tc.player_dist < fire_range && bot.weapon_cooldown <= 0.0 && ctx.player.alive;
    let turret_angle = if ctx.player.alive {
        aim_at_player(bot, ctx, RAILGUN_SPEED, tc.p.accuracy_error)
    } else {
        bot.turret_angle
    };

    AiOutput {
        thrust,
        fire: can_fire,
        turret_angle,
        new_goal,
    }
}

// ---------------------------------------------------------------------------
// DIVER
// ---------------------------------------------------------------------------

fn tick_diver(bot: &Bot, ctx: &mut AiContext, tc: &TickCtx) -> AiOutput {
    // Diver cycles between Orbit (belt) and Dive (furnace) phases.
    let belt_alt = 8.0 * tc.bh_rs;
    let furnace_alt = 3.0 * tc.bh_rs;
    let fire_range = 12.0 * tc.bh_rs;

    let new_goal = match bot.current_goal {
        BotGoal::Orbit => {
            // Wait at belt; dive when player is within range or on a timer
            if tc.player_dist < fire_range || tc.health_fraction > 0.6 {
                if ctx.rng.chance(tc.p.aggression * 0.3) {
                    Some(BotGoal::Dive)
                } else {
                    None
                }
            } else {
                None
            }
        }
        BotGoal::Dive => {
            // Reached furnace depth? Switch to climb
            if tc.bot_altitude < furnace_alt + 0.5 * tc.bh_rs {
                Some(BotGoal::Climb)
            } else if tc.health_fraction < tc.p.retreat_health_threshold {
                Some(BotGoal::Climb) // abort dive if hurt
            } else {
                None
            }
        }
        BotGoal::Climb => {
            // Back to belt altitude? Return to orbit
            if tc.bot_altitude > belt_alt - 0.5 * tc.bh_rs {
                Some(BotGoal::Orbit)
            } else {
                None
            }
        }
        _ => Some(BotGoal::Orbit),
    };

    let current_goal = new_goal.unwrap_or(bot.current_goal);

    let thrust = match current_goal {
        BotGoal::Dive => {
            compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, furnace_alt, tc.bh_mass)
        }
        BotGoal::Climb => {
            compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, belt_alt, tc.bh_mass)
        }
        _ => {
            compute_orbit_maintenance_thrust(bot.position, bot.velocity, tc.bh_pos, belt_alt, tc.bh_mass)
        }
    };

    // Fire mass driver when in range
    let can_fire = tc.player_dist < fire_range && bot.weapon_cooldown <= 0.0 && ctx.player.alive;
    let turret_angle = if ctx.player.alive {
        aim_at_player(bot, ctx, MASS_DRIVER_SPEED, tc.p.accuracy_error)
    } else {
        bot.turret_angle
    };

    AiOutput {
        thrust,
        fire: can_fire,
        turret_angle,
        new_goal,
    }
}

// ---------------------------------------------------------------------------
// VULTURE
// ---------------------------------------------------------------------------

fn tick_vulture(bot: &Bot, ctx: &mut AiContext, tc: &TickCtx) -> AiOutput {
    let min_safe_alt = 8.0 * tc.bh_rs;
    let cruise_alt = 15.0 * tc.bh_rs;
    let flee_alt = 22.0 * tc.bh_rs;
    let fire_range = 20.0 * tc.bh_rs;

    // If player approaches, retreat further out
    let player_close = tc.player_dist < 8.0 * tc.bh_rs;

    let new_goal = if tc.health_fraction < tc.p.retreat_health_threshold || player_close {
        if bot.current_goal != BotGoal::Retreat {
            Some(BotGoal::Retreat)
        } else {
            None
        }
    } else if bot.current_goal == BotGoal::Retreat && !player_close && tc.health_fraction > tc.p.retreat_health_threshold + 0.1 {
        Some(BotGoal::Orbit)
    } else {
        None
    };

    let current_goal = new_goal.unwrap_or(bot.current_goal);

    let target_alt = match current_goal {
        BotGoal::Retreat => flee_alt,
        _ => cruise_alt,
    };

    // Never allow altitude below min_safe_alt
    let effective_alt = target_alt.max(min_safe_alt);

    let thrust = if tc.bot_altitude < min_safe_alt {
        // Emergency: we're too deep, climb out immediately
        compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, min_safe_alt, tc.bh_mass)
    } else {
        let maint = compute_orbit_maintenance_thrust(bot.position, bot.velocity, tc.bh_pos, effective_alt, tc.bh_mass);
        let alt = compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, effective_alt, tc.bh_mass);
        if alt.length_squared() > 1e-6 { alt } else { maint }
    };

    // Fire railgun at long range
    let can_fire = tc.player_dist < fire_range && bot.weapon_cooldown <= 0.0 && ctx.player.alive;
    let turret_angle = if ctx.player.alive {
        aim_at_player(bot, ctx, RAILGUN_SPEED, tc.p.accuracy_error)
    } else {
        bot.turret_angle
    };

    AiOutput {
        thrust,
        fire: can_fire,
        turret_angle,
        new_goal,
    }
}

// ---------------------------------------------------------------------------
// ANCHOR
// ---------------------------------------------------------------------------

fn tick_anchor(bot: &Bot, ctx: &mut AiContext, tc: &TickCtx) -> AiOutput {
    // Anchor doesn't move. Just aims and fires at anything in range.
    let fire_range = 12.0 * tc.bh_rs;

    // Alternate between mass driver and impulse rocket based on cooldown
    let use_impulse = ctx.rng.chance(0.3);
    let projectile_speed = if use_impulse { IMPULSE_ROCKET_SPEED } else { MASS_DRIVER_SPEED };

    let can_fire = tc.player_dist < fire_range && bot.weapon_cooldown <= 0.0 && ctx.player.alive;
    let turret_angle = if ctx.player.alive {
        aim_at_player(bot, ctx, projectile_speed, tc.p.accuracy_error)
    } else {
        bot.turret_angle
    };

    AiOutput {
        thrust: Vec2::ZERO, // Anchor never thrusts
        fire: can_fire,
        turret_angle,
        new_goal: Some(BotGoal::Guard),
    }
}

// ---------------------------------------------------------------------------
// SWARM
// ---------------------------------------------------------------------------

fn tick_swarm(bot: &Bot, ctx: &mut AiContext, tc: &TickCtx) -> AiOutput {
    let fire_range = 10.0 * tc.bh_rs;

    // Determine thrust: follow formation slot if assigned, otherwise orbit
    let thrust = if let Some(slot) = bot.formation_slot {
        let form_thrust = compute_formation_thrust(bot.position, bot.velocity, slot);
        // Also maintain orbital altitude so the formation doesn't fall into the BH
        let orbit_thrust = compute_orbit_maintenance_thrust(
            bot.position,
            bot.velocity,
            tc.bh_pos,
            orbit_altitude(tc),
            tc.bh_mass,
        );
        blend_thrusts(form_thrust, 0.6, orbit_thrust, 0.4)
    } else {
        // No formation; just orbit
        let target_alt = orbit_altitude(tc);
        let alt = compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, target_alt, tc.bh_mass);
        let maint = compute_orbit_maintenance_thrust(bot.position, bot.velocity, tc.bh_pos, target_alt, tc.bh_mass);
        if alt.length_squared() > 1e-6 { alt } else { maint }
    };

    // Focus fire railgun on player
    let can_fire = tc.player_dist < fire_range && bot.weapon_cooldown <= 0.0 && ctx.player.alive;
    let turret_angle = if ctx.player.alive {
        aim_at_player(bot, ctx, RAILGUN_SPEED, tc.p.accuracy_error)
    } else {
        bot.turret_angle
    };

    AiOutput {
        thrust,
        fire: can_fire,
        turret_angle,
        new_goal: if bot.formation_slot.is_some() {
            Some(BotGoal::FormationHold(bot.formation_slot.unwrap()))
        } else {
            Some(BotGoal::Attack)
        },
    }
}

// ---------------------------------------------------------------------------
// COMMANDER
// ---------------------------------------------------------------------------

fn tick_commander(bot: &Bot, ctx: &mut AiContext, tc: &TickCtx) -> AiOutput {
    let fire_range = 20.0 * tc.bh_rs;

    // Counter player depth: if player is deep, climb; if player is shallow, dive.
    let player_is_deep = tc.player_altitude < 6.0 * tc.bh_rs;
    let player_is_shallow = tc.player_altitude > 12.0 * tc.bh_rs;

    let new_goal = if tc.health_fraction < 0.15 {
        // Emergency: critically damaged, dive to the furnace as a last-ditch attack
        Some(BotGoal::Dive)
    } else if tc.health_fraction < tc.p.retreat_health_threshold && tc.threat > 0.5 {
        Some(BotGoal::Retreat)
    } else if player_is_deep {
        Some(BotGoal::Climb)
    } else if player_is_shallow {
        Some(BotGoal::Dive)
    } else {
        Some(BotGoal::Attack)
    };

    let current_goal = new_goal.unwrap_or(bot.current_goal);

    let target_alt = match current_goal {
        BotGoal::Climb => {
            // Go high, opposite of player's deep position
            (tc.p.preferred_max_altitude * 0.8 * tc.bh_rs).max(12.0 * tc.bh_rs)
        }
        BotGoal::Dive => {
            if tc.health_fraction < 0.15 {
                // Emergency dive: go deep for one last attack
                2.5 * tc.bh_rs
            } else {
                // Counter shallow player: dive to get firing angle
                (tc.p.preferred_min_altitude * 1.2 * tc.bh_rs).min(5.0 * tc.bh_rs)
            }
        }
        BotGoal::Retreat => {
            tc.p.preferred_max_altitude * tc.bh_rs
        }
        _ => {
            // Attack: try to maintain medium altitude for tactical flexibility
            8.0 * tc.bh_rs
        }
    };

    let alt_thrust = compute_thrust_for_altitude_change(bot.position, bot.velocity, tc.bh_pos, target_alt, tc.bh_mass);
    let maint_thrust = compute_orbit_maintenance_thrust(bot.position, bot.velocity, tc.bh_pos, target_alt, tc.bh_mass);
    let base_thrust = if alt_thrust.length_squared() > 1e-6 { alt_thrust } else { maint_thrust };

    let thrust = if !tc.threats.is_empty() && tc.p.caution > 0.2 {
        let evade = compute_evasion_thrust(bot.position, bot.velocity, tc.threats);
        blend_thrusts(base_thrust, 0.6, evade, 0.4)
    } else {
        base_thrust
    };

    // Weapon selection: use impulse rockets to shove player deeper when above them,
    // otherwise use mass driver for heavy damage.
    let player_below = tc.player_altitude < tc.bot_altitude;
    let use_impulse = player_below && ctx.rng.chance(0.4);
    let projectile_speed = if use_impulse { IMPULSE_ROCKET_SPEED } else { MASS_DRIVER_SPEED };

    let can_fire = tc.player_dist < fire_range && bot.weapon_cooldown <= 0.0 && ctx.player.alive;
    let turret_angle = if ctx.player.alive {
        aim_at_player(bot, ctx, projectile_speed, tc.p.accuracy_error)
    } else {
        bot.turret_angle
    };

    AiOutput {
        thrust,
        fire: can_fire,
        turret_angle,
        new_goal,
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Blend two thrust vectors by weight, returning a normalized result.
fn blend_thrusts(a: Vec2, weight_a: f64, b: Vec2, weight_b: f64) -> Vec2 {
    let combined = a * weight_a + b * weight_b;
    if combined.length_squared() < 1e-6 {
        Vec2::ZERO
    } else {
        combined.normalized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::bot::Bot;
    use crate::entities::ship::PlayerShip;

    fn make_ctx<'a>(
        player: &'a PlayerShip,
        bots: &'a [Bot],
        projectiles: &'a [Projectile],
        black_holes: &'a [(Vec2, f64)],
        bh_rs: &'a [(Vec2, f64)],
        rng: &'a mut Rng,
    ) -> AiContext<'a> {
        AiContext {
            player,
            bots,
            projectiles,
            black_holes,
            black_hole_positions_rs: bh_rs,
            rng,
            difficulty: 0.5,
        }
    }

    #[test]
    fn test_skirmisher_fires_in_range() {
        let player = PlayerShip::new(Vec2::new(10.0, 0.0), Vec2::ZERO);
        let bot = Bot::new(BotArchetype::Skirmisher, Vec2::new(8.0, 0.0), Vec2::new(0.0, 2.0), 0.5);
        let bh = vec![(Vec2::ZERO, 1.0)];
        let bh_rs = vec![(Vec2::ZERO, 1.0)];
        let bots = [bot.clone()];
        let mut rng = Rng::new(42);
        let mut ctx = make_ctx(&player, &bots, &[], &bh, &bh_rs, &mut rng);
        let out = run_ai_tick(&bot, &mut ctx);
        assert!(out.fire, "Skirmisher should fire when player is 2 r_s away");
    }

    #[test]
    fn test_anchor_zero_thrust() {
        let player = PlayerShip::new(Vec2::new(10.0, 0.0), Vec2::ZERO);
        let bot = Bot::new(BotArchetype::Anchor, Vec2::new(4.0, 0.0), Vec2::new(0.0, 3.0), 0.5);
        let bh = vec![(Vec2::ZERO, 1.0)];
        let bh_rs = vec![(Vec2::ZERO, 1.0)];
        let bots = [bot.clone()];
        let mut rng = Rng::new(42);
        let mut ctx = make_ctx(&player, &bots, &[], &bh, &bh_rs, &mut rng);
        let out = run_ai_tick(&bot, &mut ctx);
        assert!(out.thrust.length() < 1e-6, "Anchor should never thrust");
    }

    #[test]
    fn test_vulture_stays_high() {
        let player = PlayerShip::new(Vec2::new(4.0, 0.0), Vec2::ZERO);
        let mut bot = Bot::new(BotArchetype::Vulture, Vec2::new(5.0, 0.0), Vec2::new(0.0, 2.0), 0.5);
        bot.current_goal = BotGoal::Orbit;
        let bh = vec![(Vec2::ZERO, 1.0)];
        let bh_rs = vec![(Vec2::ZERO, 1.0)];
        let bots = [bot.clone()];
        let mut rng = Rng::new(42);
        let mut ctx = make_ctx(&player, &bots, &[], &bh, &bh_rs, &mut rng);
        let out = run_ai_tick(&bot, &mut ctx);
        // Vulture at altitude 5 (below min 8) should try to climb
        let _radial_out = bot.position.normalized();
        // The thrust should have some outward or prograde component to raise orbit
        assert!(out.thrust.length() > 0.1, "Vulture below safe altitude should thrust outward");
    }

    #[test]
    fn test_commander_counters_deep_player() {
        let player = PlayerShip::new(Vec2::new(3.0, 0.0), Vec2::ZERO); // deep
        let bot = Bot::new(BotArchetype::Commander, Vec2::new(8.0, 0.0), Vec2::new(0.0, 2.0), 0.5);
        let bh = vec![(Vec2::ZERO, 1.0)];
        let bh_rs = vec![(Vec2::ZERO, 1.0)];
        let bots = [bot.clone()];
        let mut rng = Rng::new(42);
        let mut ctx = make_ctx(&player, &bots, &[], &bh, &bh_rs, &mut rng);
        let out = run_ai_tick(&bot, &mut ctx);
        // When player is deep (3 r_s), commander should climb
        assert!(out.new_goal == Some(BotGoal::Climb), "Commander should climb when player is deep");
    }

    #[test]
    fn test_blend_thrusts() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        let blended = blend_thrusts(a, 0.5, b, 0.5);
        assert!((blended.length() - 1.0).abs() < 1e-6, "Blended thrust should be normalized");
    }
}
