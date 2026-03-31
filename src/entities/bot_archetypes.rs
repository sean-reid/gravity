use super::bot::BotArchetype;

/// Holds the base stats for a bot archetype, after difficulty scaling.
#[derive(Debug, Clone, Copy)]
pub struct ArchetypeStats {
    pub health: f64,
    pub shields: f64,
    pub fuel: f64,
    pub decision_interval: f64,
    pub shield_regen_rate: f64,
    pub preferred_altitude: f64,
}

/// Return archetype stats scaled by difficulty (0.0 = easiest, 1.0 = hardest).
/// Difficulty scales HP and shields up to +50%, and makes decision intervals faster.
/// At low difficulty, bots think more slowly and have less health/shields.
pub fn get_archetype_stats(archetype: &BotArchetype, difficulty_scale: f64) -> ArchetypeStats {
    let difficulty = difficulty_scale.clamp(0.0, 1.0);
    // At difficulty 0, bots have 70% base health. At difficulty 1, +50%.
    let hp_shield_multiplier = 0.7 + 0.8 * difficulty;
    // At difficulty 0, decision intervals are 3.5x longer (much slower reactions)
    let interval_multiplier = 3.5 - 2.5 * difficulty; // 3.5 at d=0, 1.0 at d=1

    let (base_hp, base_sh, fuel, interval, shield_regen, altitude) = match archetype {
        BotArchetype::Skirmisher => (40.0, 30.0, 60.0, 0.3, 3.0, 8.0),
        BotArchetype::Diver      => (50.0, 40.0, 80.0, 0.25, 4.0, 4.0),
        BotArchetype::Vulture    => (30.0, 20.0, 100.0, 0.2, 5.0, 6.0),
        BotArchetype::Anchor     => (80.0, 60.0, 30.0, 0.5, 2.0, 12.0),
        BotArchetype::Swarm      => (15.0, 0.0, 20.0, 0.15, 0.0, 7.0),
        BotArchetype::Commander  => (200.0, 150.0, 200.0, 0.15, 6.0, 10.0),
    };

    ArchetypeStats {
        health: base_hp * hp_shield_multiplier,
        shields: base_sh * hp_shield_multiplier,
        fuel,
        decision_interval: interval * interval_multiplier,
        shield_regen_rate: shield_regen,
        preferred_altitude: altitude,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_stats_no_difficulty() {
        let stats = get_archetype_stats(&BotArchetype::Skirmisher, 0.0);
        assert!((stats.health - 28.0).abs() < 1e-10); // 40 * 0.7
        assert!((stats.shields - 21.0).abs() < 1e-10); // 30 * 0.7
        assert!((stats.fuel - 60.0).abs() < 1e-10);
        // At difficulty 0, decision interval is 3.5x base (0.3 * 3.5 = 1.05)
        assert!((stats.decision_interval - 1.05).abs() < 1e-10);
    }

    #[test]
    fn test_max_difficulty_scaling() {
        let stats = get_archetype_stats(&BotArchetype::Skirmisher, 1.0);
        assert!((stats.health - 60.0).abs() < 1e-10); // 40 * 1.5
        assert!((stats.shields - 45.0).abs() < 1e-10); // 30 * 1.5
        // Fuel is not scaled
        assert!((stats.fuel - 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_swarm_no_shields() {
        let stats = get_archetype_stats(&BotArchetype::Swarm, 1.0);
        assert!((stats.shields - 0.0).abs() < 1e-10);
        assert!((stats.shield_regen_rate - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_commander_stats() {
        let stats = get_archetype_stats(&BotArchetype::Commander, 0.5);
        // 200 * (0.7 + 0.8*0.5) = 200 * 1.1 = 220
        assert!((stats.health - 220.0).abs() < 1e-10);
        assert!((stats.shields - 165.0).abs() < 1e-10); // 150 * 1.1
    }

    #[test]
    fn test_difficulty_clamped() {
        let stats_over = get_archetype_stats(&BotArchetype::Diver, 5.0);
        let stats_max = get_archetype_stats(&BotArchetype::Diver, 1.0);
        assert!((stats_over.health - stats_max.health).abs() < 1e-10);
    }
}
