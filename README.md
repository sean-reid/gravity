# Gravity Well Arena

A single-player, top-down, real-time combat game built around orbital mechanics and relativistic time dilation. Pilot a ship in orbit around black holes, fighting AI-controlled bots where diving deep into a gravity well causes the universe to speed up around you.

## The Core Mechanic

The game simulation runs on the player's **proper time**. Diving into a gravity well causes time dilation — the external universe speeds up while your controls stay responsive. Climbing out slows it back down. This creates asymmetric combat: going deep powers up your weapons but makes enemies appear faster and more dangerous.

## Features

- **Schwarzschild time dilation** as a core gameplay mechanic, not a cosmetic effect
- **6 weapon types** — railgun, mass driver, photon lance, gravity bomb, impulse rocket, tidal mine
- **6 bot archetypes** — skirmisher, diver, vulture, anchor, swarm, commander — each with distinct AI
- **Procedural level generation** from deterministic seeds
- **Binary black hole systems** with chaotic transfer orbits
- **Full procedural audio** — no audio files, everything synthesized in real-time
- **4-act narrative** delivered through mission briefings and radio chatter
- **GPU-accelerated rendering** via wgpu with gravitational lensing, bloom, and accretion disk shaders
- **Cross-platform** — native (Windows, macOS, Linux) and web (WASM)
- **Online leaderboards** — global rankings via Cloudflare Workers + D1, with lightweight anti-cheat validation
- **Local leaderboards** — offline-first with automatic sync when connected

## Building

### Native (recommended)

```bash
cargo run --release
```

### WebAssembly

```bash
# Install wasm-pack if you haven't
cargo install wasm-pack

# Build WASM package
wasm-pack build --target web --no-default-features --features web

# Copy output to web directory and serve
cp -r pkg/ web/pkg/
# Use any static file server, e.g.:
python3 -m http.server -d web 8080
```

Then open `http://localhost:8080` in a browser with WebGPU support.

## Controls

| Input | Action |
|---|---|
| W | Prograde thrust (accelerate along orbit) |
| S | Retrograde thrust (decelerate, lower orbit) |
| A | Radial-in thrust (toward black hole) |
| D | Radial-out thrust (away from black hole) |
| Mouse | Aim turret |
| Left click | Fire active weapon |
| 1-6 | Select weapon |
| Scroll wheel | Zoom in/out |
| Space | Orbit Anchor ability (when unlocked) |
| Q | Tidal Flare ability (when unlocked) |
| Escape | Pause |

## Depth Zones

| Zone | Feel | Weapons | Danger |
|---|---|---|---|
| **Rim** | Slow, safe, time to plan | Weak | Low |
| **Belt** | Normal combat pacing | Standard | Medium |
| **Furnace** | Fast, intense | Powerful | High |
| **Abyss** | Survival horror | Devastating | Extreme |

## Online Leaderboard

The game includes a global online leaderboard powered by a Cloudflare Worker (Rust compiled to WASM) with a D1 SQLite database.

### Architecture

```
Game Client (native/WASM)
    │
    ├── Local leaderboard (always available, offline-first)
    │
    └── Online leaderboard (background HTTP, non-blocking)
            │
            ▼
    Cloudflare Worker (Rust → WASM)
            │
            ▼
    Cloudflare D1 (SQLite edge database)
```

- Scores are submitted with lightweight server-side validation (plausibility checks on accuracy, time, altitude, etc.) rather than full replay verification
- The game client uses `ureq` (native) or `fetch` (WASM) on background threads, never blocking the game loop
- Scores queue locally when offline and sync automatically when connectivity returns
- Player identity is a simple UUID generated on first launch — no account or login required

### Deploying the Server

See [server/README.md](server/README.md) for full deployment instructions. Quick start:

```bash
cd server
npm install -g wrangler
wrangler login
wrangler d1 create gravity-leaderboard
# Update database_id in wrangler.toml
wrangler d1 migrations apply gravity-leaderboard
wrangler deploy
```

## Project Structure

```
gravity/
├── src/                    # Game source (Rust)
│   ├── main.rs             # Native entry point
│   ├── lib.rs              # Library root + WASM entry point
│   ├── game.rs             # Game state machine
│   ├── physics/            # Gravity, Verlet integration, time dilation, orbits
│   ├── entities/           # Player ship, bots, projectiles, black holes
│   ├── weapons/            # 6 weapon implementations
│   ├── ai/                 # Bot AI decision loop, steering, targeting
│   ├── rendering/          # wgpu renderer, pipelines, text
│   ├── shaders/            # WGSL shaders (8 files)
│   ├── audio/              # Procedural audio synthesis (native + web)
│   ├── input/              # Keyboard/mouse + touch input abstraction
│   ├── camera/             # Player-tracking camera with zoom
│   ├── hud/                # Heads-up display elements
│   ├── narrative/          # Story script, radio chatter, briefings
│   ├── levels/             # Procedural level generation, difficulty curve
│   ├── leaderboard/        # Score computation, local + online leaderboards
│   ├── persistence/        # Save/load (native JSON file, web localStorage)
│   ├── platform/           # Platform runners (native, web)
│   └── util/               # Vec2, RNG, color utilities
├── server/                 # Leaderboard server (Cloudflare Worker)
│   ├── src/                # Worker source (Rust → WASM)
│   ├── migrations/         # D1 database schema
│   └── wrangler.toml       # Cloudflare config
├── web/                    # Web build assets (HTML, CSS, JS loader)
├── ARCHITECTURE.md         # Full technical specification
├── Cargo.toml              # Game dependencies
└── README.md
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full technical specification covering physics, weapons, AI, rendering, audio, and narrative systems.

## License

MIT — see [LICENSE](LICENSE).
