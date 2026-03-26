-- Migration: Initial schema for Gravity Well Arena leaderboard

CREATE TABLE players (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    player_id TEXT NOT NULL REFERENCES players(id),
    level_number INTEGER NOT NULL,
    seed INTEGER NOT NULL,
    score INTEGER NOT NULL,
    proper_time REAL NOT NULL,
    coordinate_time REAL NOT NULL,
    accuracy REAL NOT NULL,
    health_remaining REAL NOT NULL,
    deepest_altitude REAL NOT NULL,
    bots_killed INTEGER NOT NULL,
    bots_spaghettified INTEGER NOT NULL,
    shots_fired INTEGER NOT NULL,
    shots_hit INTEGER NOT NULL,
    damage_taken REAL NOT NULL,
    dilation_ratio REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_scores_seed_score ON scores(seed, score DESC);
CREATE INDEX idx_scores_player ON scores(player_id);
