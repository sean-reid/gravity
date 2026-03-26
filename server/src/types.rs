use serde::{Deserialize, Serialize};

// ── Request bodies ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct SubmitScoreRequest {
    pub player_id: String,
    pub level_number: i64,
    pub seed: f64, // u64 from client may exceed i64 range; use f64 for JSON compat
    pub score: i64,
    pub proper_time: f64,
    pub coordinate_time: f64,
    pub accuracy: f64,
    pub health_remaining: f64,
    pub deepest_altitude: f64,
    pub bots_killed: i64,
    pub bots_spaghettified: i64,
    pub shots_fired: i64,
    pub shots_hit: i64,
    pub damage_taken: f64,
    pub dilation_ratio: f64,
}

#[derive(Deserialize)]
pub struct UpdateDisplayNameRequest {
    pub display_name: String,
}

// ── Response bodies ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RegisterResponse {
    pub player_id: String,
    pub display_name: String,
}

#[derive(Serialize)]
pub struct SubmitScoreResponse {
    pub rank: i64,
    pub total_entries: i64,
}

#[derive(Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub player_id: String,
    pub display_name: String,
    pub score: i64,
    pub proper_time: f64,
    pub accuracy: f64,
    pub health_remaining: f64,
    pub deepest_altitude: f64,
    pub level_number: i64,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct LeaderboardResponse {
    pub entries: Vec<LeaderboardEntry>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct PlayerStatsResponse {
    pub player_id: String,
    pub display_name: String,
    pub total_scores: i64,
    pub best_score: Option<i64>,
    pub levels_completed: i64,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ── D1 row types (for deserialization from query results) ───────────────────

#[derive(Deserialize)]
pub struct PlayerRow {
    pub id: String,
    pub display_name: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct ScoreRow {
    pub id: Option<i64>,
    pub player_id: String,
    pub display_name: String,
    pub score: i64,
    pub proper_time: f64,
    pub accuracy: f64,
    pub health_remaining: f64,
    pub deepest_altitude: f64,
    pub level_number: i64,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CountRow {
    pub count: i64,
}

#[derive(Deserialize)]
pub struct RankRow {
    pub rank: i64,
}

#[derive(Deserialize)]
pub struct PlayerStatsRow {
    pub total_scores: i64,
    pub best_score: Option<i64>,
    pub levels_completed: i64,
}
