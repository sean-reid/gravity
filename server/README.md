# Gravity Well Arena - Leaderboard Server

A Cloudflare Worker leaderboard API for Gravity Well Arena, built with Rust compiled to WASM. Uses Cloudflare D1 (SQLite) for storage.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Node.js](https://nodejs.org/) (for wrangler)
- [wrangler](https://developers.cloudflare.com/workers/wrangler/) CLI

## Setup

### 1. Install wrangler

```bash
npm install -g wrangler
```

### 2. Login to Cloudflare

```bash
wrangler login
```

### 3. Create the D1 database

```bash
wrangler d1 create gravity-leaderboard
```

This will output a `database_id`. Copy it.

### 4. Update wrangler.toml

Replace `placeholder-replace-with-actual-id` in `wrangler.toml` with your actual `database_id`:

```toml
[[d1_databases]]
binding = "DB"
database_name = "gravity-leaderboard"
database_id = "your-actual-database-id-here"
```

### 5. Run the database migration

```bash
wrangler d1 migrations apply gravity-leaderboard
```

This creates the `players` and `scores` tables along with indexes.

### 6. Deploy

```bash
wrangler deploy
```

The first deploy will compile the Rust code to WASM (this takes a minute or two). Subsequent deploys are faster.

### 7. Local development

```bash
wrangler dev
```

This starts a local dev server (typically at `http://localhost:8787`) with a local D1 database. Run the migration for local dev too:

```bash
wrangler d1 migrations apply gravity-leaderboard --local
```

## API Endpoints

### POST /api/register

Create a new player.

**Request:**
```json
{ "display_name": "PlayerOne" }
```

**Response (201):**
```json
{ "player_id": "550e8400-e29b-41d4-a716-446655440000", "display_name": "PlayerOne" }
```

### POST /api/scores

Submit a score for a level seed.

**Request:**
```json
{
    "player_id": "550e8400-e29b-41d4-a716-446655440000",
    "level_number": 42,
    "seed": 12345678,
    "score": 98765,
    "proper_time": 45.2,
    "coordinate_time": 120.5,
    "accuracy": 0.73,
    "health_remaining": 55.0,
    "deepest_altitude": 2.1,
    "bots_killed": 8,
    "bots_spaghettified": 2,
    "shots_fired": 120,
    "shots_hit": 88,
    "damage_taken": 45.0,
    "dilation_ratio": 2.67
}
```

**Response (201):**
```json
{ "rank": 5, "total_entries": 230 }
```

### GET /api/leaderboard/:seed?limit=10&offset=0

Get top scores for a level seed.

**Response (200):**
```json
{
    "entries": [
        {
            "rank": 1,
            "player_id": "uuid",
            "display_name": "PlayerOne",
            "score": 98765,
            "proper_time": 45.2,
            "accuracy": 0.73,
            "health_remaining": 55.0,
            "deepest_altitude": 2.1,
            "level_number": 42,
            "timestamp": "2026-03-25T12:00:00Z"
        }
    ],
    "total": 230
}
```

### GET /api/leaderboard/:seed/around/:player_id?range=5

Get scores around a player's rank (N above and N below).

### GET /api/player/:player_id/stats

Get player summary statistics.

**Response (200):**
```json
{
    "player_id": "uuid",
    "display_name": "PlayerOne",
    "total_scores": 47,
    "best_score": 98765,
    "levels_completed": 12
}
```

### PATCH /api/player/:player_id

Update a player's display name.

**Request:**
```json
{ "display_name": "NewName" }
```

**Response (200):**
```json
{ "player_id": "uuid", "display_name": "NewName" }
```

## Score Validation

Submitted scores are checked against these rules to reject obviously fake data:

- `score` must be > 0 and < 10,000,000
- `accuracy` must be in [0.0, 1.0]
- `health_remaining` must be in [0.0, 100.0]
- `proper_time` must be > 0
- `coordinate_time` must be >= `proper_time`
- `deepest_altitude` must be > 1.0 (event horizon constraint)
- `dilation_ratio` must be >= 1.0
- `shots_hit` must be <= `shots_fired`
- `bots_spaghettified` must be <= `bots_killed`
- `level_number` must be >= 1
- `player_id` must exist in the players table

## Project Structure

```
server/
├── Cargo.toml              # Rust dependencies
├── wrangler.toml            # Cloudflare Worker configuration
├── src/
│   ├── lib.rs               # Worker entry point, router setup
│   ├── routes.rs            # Route handler functions
│   ├── validation.rs        # Score validation logic
│   ├── db.rs                # D1 database query functions
│   └── types.rs             # Request/response serde types
├── migrations/
│   └── 0001_initial.sql     # D1 schema migration
└── README.md
```
