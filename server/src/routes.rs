use serde_json::json;
use worker::*;

use crate::db;
use crate::types::*;
use crate::validation::validate_score;

/// Sanitize a display name: trim whitespace, enforce max 30 chars, reject empty.
fn sanitize_display_name(name: &str) -> std::result::Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("display_name must not be empty".into());
    }
    if trimmed.len() > 30 {
        return Err("display_name must be at most 30 characters".into());
    }
    Ok(trimmed.to_string())
}

/// Helper to build a JSON response with CORS headers.
fn json_response(body: &impl serde::Serialize, status: u16) -> Result<Response> {
    let json = serde_json::to_string(body).map_err(|e| Error::RustError(e.to_string()))?;
    let mut resp = Response::ok(json)?;
    let headers = resp.headers_mut();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, POST, PATCH, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type")?;
    // Override status code
    Ok(resp.with_status(status))
}

/// Return an error JSON response.
fn error_response(message: &str, status: u16) -> Result<Response> {
    json_response(&ErrorResponse { error: message.to_string() }, status)
}

/// CORS preflight handler.
pub fn handle_options() -> Result<Response> {
    let mut resp = Response::ok("")?;
    let headers = resp.headers_mut();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, POST, PATCH, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type")?;
    headers.set("Access-Control-Max-Age", "86400")?;
    Ok(resp.with_status(204))
}

/// POST /api/register
pub async fn register(mut req: Request, env: Env) -> Result<Response> {
    let body: RegisterRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid JSON body", 400),
    };

    let display_name = match sanitize_display_name(&body.display_name) {
        Ok(name) => name,
        Err(msg) => return error_response(&msg, 400),
    };

    let player_id = uuid::Uuid::new_v4().to_string();
    let d1 = env.d1("DB")?;

    match db::create_player(&d1, &player_id, &display_name).await {
        Ok(player) => json_response(
            &RegisterResponse {
                player_id: player.id,
                display_name: player.display_name,
            },
            201,
        ),
        Err(e) => error_response(&format!("Failed to create player: {}", e), 500),
    }
}

/// POST /api/scores
pub async fn submit_score(mut req: Request, env: Env) -> Result<Response> {
    let body: SubmitScoreRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid JSON body", 400),
    };

    // Validate score fields
    if let Err(msg) = validate_score(&body) {
        return error_response(&msg, 400);
    }

    let d1 = env.d1("DB")?;

    // Check player exists
    match db::player_exists(&d1, &body.player_id).await {
        Ok(true) => {}
        Ok(false) => return error_response("player_id not found", 404),
        Err(e) => return error_response(&format!("Database error: {}", e), 500),
    }

    let seed = body.seed;
    let score = body.score;
    let player_id = body.player_id.clone();

    // Insert score
    if let Err(e) = db::insert_score(&d1, &body).await {
        return error_response(&format!("Failed to insert score: {}", e), 500);
    }

    // Get rank and total
    match db::get_rank_and_total(&d1, &player_id, seed, score).await {
        Ok((rank, total)) => json_response(&SubmitScoreResponse { rank, total_entries: total }, 201),
        Err(e) => error_response(&format!("Failed to compute rank: {}", e), 500),
    }
}

/// GET /api/leaderboard/:seed?limit=10&offset=0
pub async fn get_leaderboard(req: Request, env: Env, seed: i64) -> Result<Response> {
    let url = req.url()?;
    let params: std::collections::HashMap<String, String> =
        url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(10)
        .min(100)
        .max(1);

    let offset = params
        .get("offset")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);

    let d1 = env.d1("DB")?;
    match db::get_leaderboard(&d1, seed, limit, offset).await {
        Ok((entries, total)) => json_response(&LeaderboardResponse { entries, total }, 200),
        Err(e) => error_response(&format!("Database error: {}", e), 500),
    }
}

/// GET /api/leaderboard/:seed/around/:player_id?range=5
pub async fn get_leaderboard_around(
    req: Request,
    env: Env,
    seed: i64,
    player_id: String,
) -> Result<Response> {
    let url = req.url()?;
    let params: std::collections::HashMap<String, String> =
        url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    let range = params
        .get("range")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(5)
        .min(50)
        .max(1);

    let d1 = env.d1("DB")?;
    match db::get_leaderboard_around(&d1, seed, &player_id, range).await {
        Ok(Some((entries, total))) => {
            json_response(&LeaderboardResponse { entries, total }, 200)
        }
        Ok(None) => error_response("Player has no scores for this seed", 404),
        Err(e) => error_response(&format!("Database error: {}", e), 500),
    }
}

/// GET /api/player/:player_id/stats
pub async fn get_player_stats(env: Env, player_id: String) -> Result<Response> {
    let d1 = env.d1("DB")?;
    match db::get_player_stats(&d1, &player_id).await {
        Ok(Some(stats)) => json_response(&stats, 200),
        Ok(None) => error_response("Player not found", 404),
        Err(e) => error_response(&format!("Database error: {}", e), 500),
    }
}

/// PATCH /api/player/:player_id
pub async fn update_player(mut req: Request, env: Env, player_id: String) -> Result<Response> {
    let body: UpdateDisplayNameRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid JSON body", 400),
    };

    let display_name = match sanitize_display_name(&body.display_name) {
        Ok(name) => name,
        Err(msg) => return error_response(&msg, 400),
    };

    let d1 = env.d1("DB")?;
    match db::update_display_name(&d1, &player_id, &display_name).await {
        Ok(Some(player)) => json_response(
            &json!({
                "player_id": player.id,
                "display_name": player.display_name
            }),
            200,
        ),
        Ok(None) => error_response("Player not found", 404),
        Err(e) => error_response(&format!("Failed to update player: {}", e), 500),
    }
}
