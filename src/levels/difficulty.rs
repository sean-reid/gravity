/// Sigmoid difficulty curve. Returns a value in (0.0, 1.0) that ramps up
/// around the midpoint level, reaching ~0.5 at level 28 and asymptoting to 1.0.
/// Early levels (1-5) produce very low difficulty values for an approachable start.
pub fn difficulty(level: u32) -> f64 {
    let max_difficulty = 1.0;
    let midpoint = 28.0;
    let steepness = 10.0;
    max_difficulty / (1.0 + (-(level as f64 - midpoint) / steepness).exp())
}

/// Maximum time-dilation factor for a given level.
/// Starts at 1.5 and ramps faster, capped at 6.0.
pub fn max_dilation_for_level(level: u32) -> f64 {
    let raw = 1.5 + level as f64 * 0.15;
    raw.min(6.0)
}

/// Number of bots to spawn for a given level.
/// Slow ramp: 2 bots through level 5, then +1 every 3 levels, capped at 25.
pub fn bot_count_for_level(level: u32) -> u32 {
    let raw = match level {
        1..=3 => 2,
        4..=5 => 3,
        _ => 3 + (level - 5) / 3,
    };
    raw.min(25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_is_low_at_start() {
        assert!(difficulty(1) < 0.1);
    }

    #[test]
    fn difficulty_is_near_half_at_midpoint() {
        let d = difficulty(28);
        assert!((d - 0.5).abs() < 0.01);
    }

    #[test]
    fn difficulty_approaches_one() {
        assert!(difficulty(60) > 0.95);
    }

    #[test]
    fn dilation_caps_at_six() {
        assert!((max_dilation_for_level(200) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn bot_count_caps_at_twenty_five() {
        assert_eq!(bot_count_for_level(100), 25);
    }

    #[test]
    fn bot_count_early() {
        assert_eq!(bot_count_for_level(1), 2);
        assert_eq!(bot_count_for_level(2), 2);
        assert_eq!(bot_count_for_level(3), 2);
        assert_eq!(bot_count_for_level(5), 3);
    }
}
