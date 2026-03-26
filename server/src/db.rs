use wasm_bindgen::JsValue;
use worker::*;

use crate::types::*;

/// Insert a new player and return the row.
pub async fn create_player(db: &D1Database, id: &str, display_name: &str) -> Result<PlayerRow> {
    let stmt = db.prepare(
        "INSERT INTO players (id, display_name) VALUES (?1, ?2) RETURNING id, display_name, created_at",
    );
    let row = stmt
        .bind(&[JsValue::from(id), JsValue::from(display_name)])?
        .first::<PlayerRow>(None)
        .await?
        .ok_or_else(|| Error::RustError("Failed to create player".into()))?;
    Ok(row)
}

/// Check if a player exists by id.
pub async fn player_exists(db: &D1Database, player_id: &str) -> Result<bool> {
    let stmt = db.prepare("SELECT COUNT(*) as count FROM players WHERE id = ?1");
    let row = stmt
        .bind(&[JsValue::from(player_id)])?
        .first::<CountRow>(None)
        .await?;
    Ok(row.map(|r| r.count > 0).unwrap_or(false))
}

/// Get a player by id.
pub async fn get_player(db: &D1Database, player_id: &str) -> Result<Option<PlayerRow>> {
    let stmt = db.prepare("SELECT id, display_name, created_at FROM players WHERE id = ?1");
    stmt.bind(&[JsValue::from(player_id)])?
        .first::<PlayerRow>(None)
        .await
}

/// Update a player's display name.
pub async fn update_display_name(
    db: &D1Database,
    player_id: &str,
    display_name: &str,
) -> Result<Option<PlayerRow>> {
    let stmt = db.prepare(
        "UPDATE players SET display_name = ?1 WHERE id = ?2 RETURNING id, display_name, created_at",
    );
    stmt.bind(&[JsValue::from(display_name), JsValue::from(player_id)])?
        .first::<PlayerRow>(None)
        .await
}

/// Insert a score row.
pub async fn insert_score(db: &D1Database, req: &SubmitScoreRequest) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO scores (player_id, level_number, seed, score, proper_time, coordinate_time, \
         accuracy, health_remaining, deepest_altitude, bots_killed, bots_spaghettified, \
         shots_fired, shots_hit, damage_taken, dilation_ratio) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
    );
    stmt.bind(&[
        JsValue::from(req.player_id.as_str()),
        JsValue::from(req.level_number as f64),
        JsValue::from(req.seed as f64),
        JsValue::from(req.score as f64),
        JsValue::from(req.proper_time),
        JsValue::from(req.coordinate_time),
        JsValue::from(req.accuracy),
        JsValue::from(req.health_remaining),
        JsValue::from(req.deepest_altitude),
        JsValue::from(req.bots_killed as f64),
        JsValue::from(req.bots_spaghettified as f64),
        JsValue::from(req.shots_fired as f64),
        JsValue::from(req.shots_hit as f64),
        JsValue::from(req.damage_taken),
        JsValue::from(req.dilation_ratio),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Get the rank of a player's latest score for a given seed, and the total count for that seed.
pub async fn get_rank_and_total(
    db: &D1Database,
    player_id: &str,
    seed: i64,
    score: i64,
) -> Result<(i64, i64)> {
    let _ = player_id;
    // Rank = number of scores strictly higher + 1
    let rank_stmt = db.prepare(
        "SELECT COUNT(*) as count FROM scores WHERE seed = ?1 AND score > ?2",
    );
    let rank_row = rank_stmt
        .bind(&[JsValue::from(seed as f64), JsValue::from(score as f64)])?
        .first::<CountRow>(None)
        .await?;
    let rank = rank_row.map(|r| r.count + 1).unwrap_or(1);

    let total_stmt = db.prepare("SELECT COUNT(*) as count FROM scores WHERE seed = ?1");
    let total_row = total_stmt
        .bind(&[JsValue::from(seed as f64)])?
        .first::<CountRow>(None)
        .await?;
    let total = total_row.map(|r| r.count).unwrap_or(0);

    Ok((rank, total))
}

/// Get leaderboard entries for a seed with limit/offset.
pub async fn get_leaderboard(
    db: &D1Database,
    seed: i64,
    limit: i64,
    offset: i64,
) -> Result<(Vec<LeaderboardEntry>, i64)> {
    // Get total count
    let count_stmt = db.prepare("SELECT COUNT(*) as count FROM scores WHERE seed = ?1");
    let total = count_stmt
        .bind(&[JsValue::from(seed as f64)])?
        .first::<CountRow>(None)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

    // Get entries
    let stmt = db.prepare(
        "SELECT s.id, s.player_id, p.display_name, s.score, s.proper_time, s.accuracy, \
         s.health_remaining, s.deepest_altitude, s.level_number, s.created_at \
         FROM scores s JOIN players p ON s.player_id = p.id \
         WHERE s.seed = ?1 \
         ORDER BY s.score DESC \
         LIMIT ?2 OFFSET ?3",
    );
    let results = stmt
        .bind(&[JsValue::from(seed as f64), JsValue::from(limit as f64), JsValue::from(offset as f64)])?
        .all()
        .await?;

    let rows = results.results::<ScoreRow>()?;
    let entries: Vec<LeaderboardEntry> = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| LeaderboardEntry {
            rank: offset + i as i64 + 1,
            player_id: row.player_id,
            display_name: row.display_name,
            score: row.score,
            proper_time: row.proper_time,
            accuracy: row.accuracy,
            health_remaining: row.health_remaining,
            deepest_altitude: row.deepest_altitude,
            level_number: row.level_number,
            timestamp: row.created_at,
        })
        .collect();

    Ok((entries, total))
}

/// Get scores around a player's best score for a seed.
pub async fn get_leaderboard_around(
    db: &D1Database,
    seed: i64,
    player_id: &str,
    range: i64,
) -> Result<Option<(Vec<LeaderboardEntry>, i64)>> {
    // Get total count
    let count_stmt = db.prepare("SELECT COUNT(*) as count FROM scores WHERE seed = ?1");
    let total = count_stmt
        .bind(&[JsValue::from(seed as f64)])?
        .first::<CountRow>(None)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

    // Find the player's best score for this seed
    let best_stmt = db.prepare(
        "SELECT score FROM scores WHERE seed = ?1 AND player_id = ?2 ORDER BY score DESC LIMIT 1",
    );

    #[derive(serde::Deserialize)]
    struct ScoreOnly {
        score: i64,
    }

    let best = best_stmt
        .bind(&[JsValue::from(seed as f64), JsValue::from(player_id)])?
        .first::<ScoreOnly>(None)
        .await?;

    let player_score = match best {
        Some(b) => b.score,
        None => return Ok(None),
    };

    // Calculate the player's rank (0-indexed position)
    let rank_stmt =
        db.prepare("SELECT COUNT(*) as count FROM scores WHERE seed = ?1 AND score > ?2");
    let player_rank = rank_stmt
        .bind(&[JsValue::from(seed as f64), JsValue::from(player_score as f64)])?
        .first::<CountRow>(None)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

    // Compute offset: range entries above the player
    let offset = (player_rank - range).max(0);
    let limit = range * 2 + 1;

    let stmt = db.prepare(
        "SELECT s.id, s.player_id, p.display_name, s.score, s.proper_time, s.accuracy, \
         s.health_remaining, s.deepest_altitude, s.level_number, s.created_at \
         FROM scores s JOIN players p ON s.player_id = p.id \
         WHERE s.seed = ?1 \
         ORDER BY s.score DESC \
         LIMIT ?2 OFFSET ?3",
    );
    let results = stmt
        .bind(&[JsValue::from(seed as f64), JsValue::from(limit as f64), JsValue::from(offset as f64)])?
        .all()
        .await?;

    let rows = results.results::<ScoreRow>()?;
    let entries: Vec<LeaderboardEntry> = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| LeaderboardEntry {
            rank: offset + i as i64 + 1,
            player_id: row.player_id,
            display_name: row.display_name,
            score: row.score,
            proper_time: row.proper_time,
            accuracy: row.accuracy,
            health_remaining: row.health_remaining,
            deepest_altitude: row.deepest_altitude,
            level_number: row.level_number,
            timestamp: row.created_at,
        })
        .collect();

    Ok(Some((entries, total)))
}

/// Get player stats: total scores, best score, levels completed.
pub async fn get_player_stats(
    db: &D1Database,
    player_id: &str,
) -> Result<Option<PlayerStatsResponse>> {
    let player = get_player(db, player_id).await?;
    let player = match player {
        Some(p) => p,
        None => return Ok(None),
    };

    let stmt = db.prepare(
        "SELECT COUNT(*) as total_scores, MAX(score) as best_score, \
         COUNT(DISTINCT seed) as levels_completed \
         FROM scores WHERE player_id = ?1",
    );
    let stats = stmt
        .bind(&[JsValue::from(player_id)])?
        .first::<PlayerStatsRow>(None)
        .await?;

    match stats {
        Some(s) => Ok(Some(PlayerStatsResponse {
            player_id: player.id,
            display_name: player.display_name,
            total_scores: s.total_scores,
            best_score: s.best_score,
            levels_completed: s.levels_completed,
        })),
        None => Ok(Some(PlayerStatsResponse {
            player_id: player.id,
            display_name: player.display_name,
            total_scores: 0,
            best_score: None,
            levels_completed: 0,
        })),
    }
}
