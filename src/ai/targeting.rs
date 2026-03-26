use crate::util::{Vec2, Rng};
use crate::entities::bot::Bot;
use crate::entities::projectile::Projectile;

/// Compute the turret angle a shooter should use to hit a moving target,
/// accounting for projectile travel time. Applies accuracy error as a
/// random angular offset.
///
/// Uses iterative lead prediction: estimate travel time, predict where the
/// target will be at that time, re-estimate, repeat.
pub fn compute_lead_target(
    shooter_pos: Vec2,
    _shooter_vel: Vec2,
    target_pos: Vec2,
    target_vel: Vec2,
    projectile_speed: f64,
    accuracy_error: f64,
    rng: &mut Rng,
) -> f64 {
    // Start with the straight-line estimate
    let mut predicted_pos = target_pos;

    // Iterative refinement (3 iterations is sufficient for convergence)
    for _ in 0..3 {
        let to_target = predicted_pos - shooter_pos;
        let dist = to_target.length();
        if dist < 1e-6 {
            // Target is on top of us; just aim at them
            break;
        }

        // Time for the projectile to reach predicted position.
        // The projectile inherits the shooter's velocity, so effective speed
        // relative to the world is (shooter_vel + projectile_dir * speed).
        // For a first-order approximation, use raw projectile speed for the
        // travel-time estimate and compensate via iteration.
        let travel_time = dist / projectile_speed;

        // Predict target position at travel_time, assuming constant velocity.
        predicted_pos = target_pos + target_vel * travel_time;
    }

    // Aim direction from shooter to predicted intercept
    let aim_dir = predicted_pos - shooter_pos;
    let base_angle = aim_dir.angle();

    // Apply accuracy error: random angular offset scaled by accuracy_error
    // The multiplier converts accuracy_error into radians of aim spread
    let error_offset = accuracy_error * rng.range_f64(-1.0, 1.0) * 0.35;

    base_angle + error_offset
}

/// Compute a threat level in [0, 1] for a bot, considering how close the player
/// is and how many hostile projectiles are nearby.
///
/// Higher values mean more danger. Used by the decision layer to trigger
/// retreat or evasion behaviors.
pub fn compute_threat_level(
    bot: &Bot,
    player_pos: Vec2,
    projectiles: &[Projectile],
) -> f64 {
    let bot_pos = bot.position;

    // --- Distance threat from the player ---
    // At distance 3 r_s -> threat ~1.0, at distance 20 r_s -> threat ~0.0
    let player_dist = bot_pos.distance(player_pos);
    let distance_threat = (1.0 - (player_dist - 3.0) / 17.0).clamp(0.0, 1.0);

    // --- Incoming projectile threat ---
    // Count player-owned projectiles within 5 r_s that are roughly heading our way
    let mut projectile_threat_count = 0;
    for proj in projectiles {
        if !proj.alive || !proj.owner_is_player {
            continue;
        }
        let to_bot = bot_pos - proj.position;
        let dist = to_bot.length();
        if dist > 8.0 {
            continue;
        }
        // Check if projectile is heading roughly toward the bot
        let proj_dir = proj.velocity.normalized();
        let toward_bot = to_bot.normalized();
        let dot = proj_dir.dot(toward_bot);
        if dot > 0.3 {
            projectile_threat_count += 1;
        }
    }
    // Each incoming projectile adds 0.15 threat, capped at 0.6
    let projectile_threat = (projectile_threat_count as f64 * 0.15).min(0.6);

    // --- Health-based vulnerability ---
    let health_fraction = if bot.max_health > 0.0 {
        bot.health / bot.max_health
    } else {
        1.0
    };
    // Low health amplifies threat perception
    let vulnerability = 1.0 + (1.0 - health_fraction) * 0.5;

    // Combine and clamp
    ((distance_threat * 0.5 + projectile_threat) * vulnerability).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::bot::{Bot, BotArchetype};

    #[test]
    fn test_lead_target_stationary() {
        let mut rng = Rng::new(42);
        let angle = compute_lead_target(
            Vec2::new(0.0, 0.0),
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::ZERO,
            40.0,
            0.0, // perfect accuracy
            &mut rng,
        );
        // Should aim roughly at angle 0 (to the right)
        assert!(angle.abs() < 0.01);
    }

    #[test]
    fn test_lead_target_moving() {
        let mut rng = Rng::new(42);
        let angle = compute_lead_target(
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 5.0), // target moving upward
            40.0,
            0.0,
            &mut rng,
        );
        // Should aim slightly above horizontal
        assert!(angle > 0.0);
        assert!(angle < std::f64::consts::FRAC_PI_4);
    }

    #[test]
    fn test_lead_target_with_error() {
        let mut rng = Rng::new(42);
        let angles: Vec<f64> = (0..100)
            .map(|_| {
                compute_lead_target(
                    Vec2::ZERO,
                    Vec2::ZERO,
                    Vec2::new(10.0, 0.0),
                    Vec2::ZERO,
                    40.0,
                    1.0, // maximum error
                    &mut rng,
                )
            })
            .collect();
        // With error, we should see variation in angles
        let min = angles.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = angles.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max - min > 0.01, "Expected spread in angles, got {}", max - min);
    }

    #[test]
    fn test_threat_level_far_away() {
        let bot = Bot::new(BotArchetype::Skirmisher, Vec2::new(25.0, 0.0), Vec2::ZERO, 0.0);
        let threat = compute_threat_level(&bot, Vec2::new(0.0, 0.0), &[]);
        // Player is far away, no projectiles
        assert!(threat < 0.3, "Expected low threat for far player, got {}", threat);
    }

    #[test]
    fn test_threat_level_close() {
        let bot = Bot::new(BotArchetype::Skirmisher, Vec2::new(3.0, 0.0), Vec2::ZERO, 0.0);
        let threat = compute_threat_level(&bot, Vec2::new(0.0, 0.0), &[]);
        // Player is very close
        assert!(threat > 0.3, "Expected elevated threat for close player, got {}", threat);
    }
}
