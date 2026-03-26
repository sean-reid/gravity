/// Base score awarded for completing a level before multipliers.
const BASE_SCORE: u64 = 1000;

/// Raw statistics collected during a level for score computation.
#[derive(Debug, Clone)]
pub struct LevelScore {
    /// Total proper time elapsed (player's frame).
    pub proper_time_elapsed: f64,
    /// Total coordinate time elapsed (external frame).
    pub coordinate_time_elapsed: f64,
    /// Total damage taken by the player.
    pub total_damage_taken: f64,
    /// Number of shots the player fired.
    pub shots_fired: u32,
    /// Number of shots that hit an enemy.
    pub shots_hit: u32,
    /// Deepest altitude reached (smallest distance to a black hole center).
    pub deepest_altitude: f64,
    /// Number of bots destroyed by spaghettification (falling into a black hole).
    pub bots_spaghettified: u32,
    /// Shot accuracy (0.0 - 1.0). Computed from shots_fired and shots_hit.
    pub accuracy: f64,
    /// Ratio of coordinate time to proper time (time dilation experienced).
    pub dilation_ratio: f64,
    /// Fraction of health remaining at level end (0.0 - 1.0).
    pub health_remaining: f64,
}

impl LevelScore {
    /// Compute accuracy and dilation ratio from raw values.
    pub fn finalize(&mut self) {
        self.accuracy = if self.shots_fired > 0 {
            self.shots_hit as f64 / self.shots_fired as f64
        } else {
            0.0
        };
        self.dilation_ratio = if self.proper_time_elapsed > 0.0 {
            self.coordinate_time_elapsed / self.proper_time_elapsed
        } else {
            1.0
        };
    }
}

/// Compute the final score for a level.
///
/// Formula:
///   score = BASE_SCORE
///         * level_multiplier
///         * accuracy_bonus
///         * dilation_bonus
///         * health_bonus
///         + spaghettification_bonus
///
/// Where:
///   level_multiplier = 1 + (level_number - 1) * 0.15
///   accuracy_bonus = 1.0 + accuracy * 0.5  (up to 1.5x for perfect aim)
///   dilation_bonus = 1.0 + (dilation_ratio - 1.0) * 0.2  (reward playing deep in the well)
///   health_bonus = 0.5 + health_remaining * 0.5  (50%-100% based on health)
///   spaghettification_bonus = bots_spaghettified * 250 * level_number
pub fn compute_score(stats: &LevelScore, level_number: u32) -> u64 {
    let level_multiplier = 1.0 + (level_number as f64 - 1.0) * 0.15;
    let accuracy_bonus = 1.0 + stats.accuracy * 0.5;
    let dilation_bonus = 1.0 + (stats.dilation_ratio - 1.0).max(0.0) * 0.2;
    let health_bonus = 0.5 + stats.health_remaining.clamp(0.0, 1.0) * 0.5;

    let base = BASE_SCORE as f64 * level_multiplier * accuracy_bonus * dilation_bonus * health_bonus;
    let spaghetti_bonus = stats.bots_spaghettified as f64 * 250.0 * level_number as f64;

    (base + spaghetti_bonus).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats() -> LevelScore {
        LevelScore {
            proper_time_elapsed: 60.0,
            coordinate_time_elapsed: 90.0,
            total_damage_taken: 20.0,
            shots_fired: 100,
            shots_hit: 80,
            deepest_altitude: 3.0,
            bots_spaghettified: 0,
            accuracy: 0.8,
            dilation_ratio: 1.5,
            health_remaining: 1.0,
        }
    }

    #[test]
    fn base_score_level_1() {
        let stats = LevelScore {
            accuracy: 0.0,
            dilation_ratio: 1.0,
            health_remaining: 1.0,
            bots_spaghettified: 0,
            ..make_stats()
        };
        let score = compute_score(&stats, 1);
        // 1000 * 1.0 * 1.0 * 1.0 * 1.0 = 1000
        assert_eq!(score, 1000);
    }

    #[test]
    fn accuracy_increases_score() {
        let low = LevelScore { accuracy: 0.0, ..make_stats() };
        let high = LevelScore { accuracy: 1.0, ..make_stats() };
        assert!(compute_score(&high, 5) > compute_score(&low, 5));
    }

    #[test]
    fn spaghettification_bonus() {
        let mut stats = make_stats();
        stats.bots_spaghettified = 2;
        let with_bonus = compute_score(&stats, 5);
        stats.bots_spaghettified = 0;
        let without = compute_score(&stats, 5);
        assert_eq!(with_bonus - without, 2500); // 2 * 250 * 5
    }

    #[test]
    fn finalize_computes_fields() {
        let mut stats = LevelScore {
            proper_time_elapsed: 50.0,
            coordinate_time_elapsed: 100.0,
            total_damage_taken: 0.0,
            shots_fired: 10,
            shots_hit: 7,
            deepest_altitude: 5.0,
            bots_spaghettified: 0,
            accuracy: 0.0,
            dilation_ratio: 0.0,
            health_remaining: 1.0,
        };
        stats.finalize();
        assert!((stats.accuracy - 0.7).abs() < 1e-9);
        assert!((stats.dilation_ratio - 2.0).abs() < 1e-9);
    }
}
