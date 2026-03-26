use crate::types::SubmitScoreRequest;

/// Validate a score submission. Returns Ok(()) if valid, or Err(message) describing
/// the first validation rule that failed.
pub fn validate_score(req: &SubmitScoreRequest) -> Result<(), String> {
    if req.level_number < 1 {
        return Err("level_number must be >= 1".into());
    }

    if req.score <= 0 || req.score >= 10_000_000 {
        return Err("score must be > 0 and < 10,000,000".into());
    }

    if req.accuracy < 0.0 || req.accuracy > 1.0 {
        return Err("accuracy must be in [0.0, 1.0]".into());
    }

    if req.health_remaining < 0.0 || req.health_remaining > 100.0 {
        return Err("health_remaining must be in [0.0, 100.0]".into());
    }

    if req.proper_time <= 0.0 {
        return Err("proper_time must be > 0".into());
    }

    if req.coordinate_time < req.proper_time {
        return Err("coordinate_time must be >= proper_time".into());
    }

    if req.deepest_altitude <= 1.0 {
        return Err("deepest_altitude must be > 1.0 (can't go below event horizon and survive)".into());
    }

    if req.dilation_ratio < 1.0 {
        return Err("dilation_ratio must be >= 1.0".into());
    }

    if req.shots_hit > req.shots_fired {
        return Err("shots_hit must be <= shots_fired".into());
    }

    if req.bots_spaghettified > req.bots_killed {
        return Err("bots_spaghettified must be <= bots_killed".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> SubmitScoreRequest {
        SubmitScoreRequest {
            player_id: "test-uuid".into(),
            level_number: 1,
            seed: 12345,
            score: 1000,
            proper_time: 30.0,
            coordinate_time: 60.0,
            accuracy: 0.75,
            health_remaining: 50.0,
            deepest_altitude: 2.0,
            bots_killed: 10,
            bots_spaghettified: 2,
            shots_fired: 100,
            shots_hit: 75,
            damage_taken: 50.0,
            dilation_ratio: 2.0,
        }
    }

    #[test]
    fn test_valid_score() {
        assert!(validate_score(&valid_request()).is_ok());
    }

    #[test]
    fn test_score_too_high() {
        let mut req = valid_request();
        req.score = 10_000_000;
        assert!(validate_score(&req).is_err());
    }

    #[test]
    fn test_accuracy_out_of_range() {
        let mut req = valid_request();
        req.accuracy = 1.5;
        assert!(validate_score(&req).is_err());
    }

    #[test]
    fn test_coordinate_less_than_proper() {
        let mut req = valid_request();
        req.coordinate_time = 10.0;
        req.proper_time = 30.0;
        assert!(validate_score(&req).is_err());
    }

    #[test]
    fn test_below_event_horizon() {
        let mut req = valid_request();
        req.deepest_altitude = 0.5;
        assert!(validate_score(&req).is_err());
    }

    #[test]
    fn test_shots_hit_exceeds_fired() {
        let mut req = valid_request();
        req.shots_hit = 200;
        req.shots_fired = 100;
        assert!(validate_score(&req).is_err());
    }
}
