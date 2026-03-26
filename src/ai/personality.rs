use crate::entities::bot::BotArchetype;

/// Per-archetype behavior parameters that drive AI decision thresholds.
#[derive(Debug, Clone, Copy)]
pub struct PersonalityParams {
    /// 0-1, how readily it engages targets.
    pub aggression: f64,
    /// 0-1, how readily it retreats from danger.
    pub caution: f64,
    /// 0-1, aim error factor (0 = perfect aim).
    pub accuracy_error: f64,
    /// 0-1, willingness to orbit deep near the black hole.
    pub dive_willingness: f64,
    /// Minimum preferred orbital altitude in Schwarzschild radii.
    pub preferred_min_altitude: f64,
    /// Maximum preferred orbital altitude in Schwarzschild radii.
    pub preferred_max_altitude: f64,
    /// Fraction of max health at which the bot considers retreating.
    pub retreat_health_threshold: f64,
}

/// Return personality parameters for the given archetype, scaled by difficulty (0.0-1.0).
///
/// Difficulty primarily affects accuracy (lower error at higher difficulty) and
/// retreat thresholds (bots fight longer at higher difficulty).
pub fn get_personality(archetype: &BotArchetype, difficulty: f64) -> PersonalityParams {
    let d = difficulty.clamp(0.0, 1.0);

    match archetype {
        // Skirmisher: jack-of-all-trades, moderate everything, belt orbiter.
        BotArchetype::Skirmisher => PersonalityParams {
            aggression: lerp(0.2, 0.6, d),
            caution: lerp(0.7, 0.4, d),
            accuracy_error: lerp(0.40, 0.03, d),
            dive_willingness: 0.3,
            preferred_min_altitude: 5.0,
            preferred_max_altitude: 12.0,
            retreat_health_threshold: lerp(0.5, 0.25, d),
        },

        // Diver: aggressive, brave, good aim, alternates belt and furnace.
        BotArchetype::Diver => PersonalityParams {
            aggression: lerp(0.4, 0.85, d),
            caution: lerp(0.4, 0.15, d),
            accuracy_error: lerp(0.30, 0.03, d),
            dive_willingness: lerp(0.5, 0.9, d),
            preferred_min_altitude: 2.0,
            preferred_max_altitude: 12.0,
            retreat_health_threshold: lerp(0.4, 0.15, d),
        },

        // Vulture: cowardly sniper, stays at rim, never goes deep.
        BotArchetype::Vulture => PersonalityParams {
            aggression: lerp(0.15, 0.35, d),
            caution: 0.8,
            accuracy_error: lerp(0.35, 0.05, d),
            dive_willingness: 0.0,
            preferred_min_altitude: 10.0,
            preferred_max_altitude: 25.0,
            retreat_health_threshold: lerp(0.6, 0.4, d),
        },

        // Anchor: immobile turret, very aggressive, doesn't retreat, stays low.
        BotArchetype::Anchor => PersonalityParams {
            aggression: 0.9,
            caution: 0.0,
            accuracy_error: lerp(0.12, 0.05, d),
            dive_willingness: 0.5,
            preferred_min_altitude: 2.5,
            preferred_max_altitude: 5.0,
            retreat_health_threshold: 0.0, // never retreats
        },

        // Swarm: aggressive pack hunter, poor accuracy, expendable.
        BotArchetype::Swarm => PersonalityParams {
            aggression: lerp(0.3, 0.75, d),
            caution: 0.1,
            accuracy_error: lerp(0.45, 0.12, d),
            dive_willingness: 0.4,
            preferred_min_altitude: 5.0,
            preferred_max_altitude: 10.0,
            retreat_health_threshold: 0.1, // almost never retreats
        },

        // Commander: elite, full range, counters player depth, excellent aim.
        BotArchetype::Commander => PersonalityParams {
            aggression: 0.9,
            caution: 0.4,
            accuracy_error: lerp(0.05, 0.02, d),
            dive_willingness: 0.8,
            preferred_min_altitude: 2.5,
            preferred_max_altitude: 25.0,
            retreat_health_threshold: lerp(0.3, 0.15, d),
        },
    }
}

/// Linear interpolation: returns `a` when `t=0`, `b` when `t=1`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skirmisher_easy() {
        let p = get_personality(&BotArchetype::Skirmisher, 0.0);
        assert!((p.aggression - 0.2).abs() < 1e-10);
        assert!((p.accuracy_error - 0.40).abs() < 1e-10);
    }

    #[test]
    fn test_skirmisher_hard() {
        let p = get_personality(&BotArchetype::Skirmisher, 1.0);
        assert!((p.accuracy_error - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_anchor_never_retreats() {
        let p = get_personality(&BotArchetype::Anchor, 0.5);
        assert!((p.retreat_health_threshold).abs() < 1e-10);
        assert!((p.caution).abs() < 1e-10);
    }

    #[test]
    fn test_vulture_stays_high() {
        let p = get_personality(&BotArchetype::Vulture, 0.5);
        assert!(p.preferred_min_altitude >= 10.0);
        assert!(p.dive_willingness < 0.01);
    }

    #[test]
    fn test_commander_excellent_accuracy() {
        let p = get_personality(&BotArchetype::Commander, 1.0);
        assert!(p.accuracy_error <= 0.02 + 1e-10);
    }

    #[test]
    fn test_difficulty_clamped() {
        let p_over = get_personality(&BotArchetype::Diver, 5.0);
        let p_max = get_personality(&BotArchetype::Diver, 1.0);
        assert!((p_over.accuracy_error - p_max.accuracy_error).abs() < 1e-10);
    }
}
