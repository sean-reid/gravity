// ---------------------------------------------------------------------------
// Gravity Well Arena -- Game State Machine
// ---------------------------------------------------------------------------

use crate::util::{Vec2, Rng, Color};
use crate::physics::gravity::{compute_gravitational_acceleration, G};
use crate::physics::integrator::{VerletState, integrate_step, integrate_step_with_thrust};
use crate::physics::dilation::{compute_tau, compute_steps_per_frame};
use crate::physics::orbit::{compute_orbital_params, compute_circular_orbit_velocity};
use crate::physics::collision::{
    circle_circle, check_event_horizon, check_escape, ray_circle_intersection,
    KILL_FACTOR, MAX_RADIUS,
};
// Entity types
use crate::entities::black_hole::BlackHole;
use crate::entities::ship::{
    PlayerShip, ThrustDirection, SHIP_RADIUS, THRUST_MAGNITUDE, FUEL_THRUST_COST, MAX_TRAIL_LENGTH,
};
use crate::entities::bot::{Bot, BotArchetype, BOT_RADIUS, BOT_MAX_TRAIL_LENGTH};
use crate::entities::projectile::{Projectile, ProjectileType};
use crate::entities::effects::{
    Explosion, ParticleEffect, spawn_explosion_particles, spawn_thrust_particle,
    spawn_spaghettification_particles,
};
use crate::weapons::{WeaponType, weapon_slots};
use crate::weapons::photon_lance::{
    beam_endpoint, compute_beam_damage, BEAM_RANGE, BEAM_WIDTH, FUEL_COST as PHOTON_FUEL_COST,
};
use crate::ai::decision::{run_ai_tick, AiContext};
use crate::input::InputAction;
use crate::camera::Camera;
use crate::audio::{AudioBackend, SoundEvent, AmbientParams};
use crate::narrative::{
    build_script, NarrativeEvent, NarrativeTrigger, NarrativeContent, RadioChatterData,
    StoryState, RadioSystem, BriefingState, DialogueLine,
};
use crate::levels::generator::generate_level;
use crate::levels::config::LevelConfig;
use crate::levels::difficulty::difficulty;
use crate::levels::progression::Progression;
use crate::leaderboard::score::{LevelScore, compute_score};
use crate::leaderboard::replay::ReplayRecorder;
use crate::leaderboard::local::{LocalLeaderboard, LeaderboardEntry};
use crate::leaderboard::online::{OnlineLeaderboard, ScoreSubmission};
use crate::persistence::{SaveData, SaveState};
use crate::rendering::{
    RenderScene, CameraUniform, StarInstance, BlackHoleData, TrailData, TrailVertex,
    ShipInstance, ParticleInstance, BeamSegment, HudElement,
};
use crate::rendering::pipelines::starfield::StarfieldPipeline;
use crate::rendering::ships::{ship_instance_from_player, ship_instance_from_bot};
use crate::rendering::projectiles::projectile_instance;
use crate::hud::{HudState, build_hud};
use crate::hud::trajectory::trajectory_safety_color;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FIXED_DT_COORD: f64 = 1.0 / 240.0;
const MAX_STEPS_PER_FRAME: u32 = 20;
const MAX_PLAYER_MINES: u32 = 5;
const STAR_COUNT: u32 = 2048;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    NameEntry,
    Title,
    Briefing,
    Playing,
    Death { cause: DeathCause, stats: LevelStats },
    LevelClear { stats: LevelStats, score: u64 },
    Debrief,
    Paused,
}

pub struct NameEntryState {
    pub chars: Vec<char>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub max_len: usize,
    pub confirmed: bool,
    pub input_cooldown: f64,
}

impl NameEntryState {
    pub fn new() -> Self {
        Self {
            chars: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            max_len: 12,
            confirmed: false,
            input_cooldown: 0.0,
        }
    }

    /// Returns the number of columns for the given row.
    pub fn cols_in_row(row: usize) -> usize {
        match row {
            0 | 1 | 2 => 13,
            3 => 2, // DEL, CONFIRM
            _ => 13,
        }
    }

    /// Returns the character grid layout.
    pub fn grid() -> [&'static [char]; 3] {
        const ROW0: &[char] = &['A','B','C','D','E','F','G','H','I','J','K','L','M'];
        const ROW1: &[char] = &['N','O','P','Q','R','S','T','U','V','W','X','Y','Z'];
        const ROW2: &[char] = &['0','1','2','3','4','5','6','7','8','9','-','_','.'];
        [ROW0, ROW1, ROW2]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeathCause {
    Weapon(ProjectileType),
    Spaghettified,
    LostToVoid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelStats {
    pub proper_time: f64,
    pub coordinate_time: f64,
    pub bots_killed: u32,
    pub bots_spaghettified: u32,
    pub shots_fired: u32,
    pub shots_hit: u32,
    pub damage_taken: f64,
    pub deepest_altitude: f64,
    pub health_remaining: f64,
}

impl Default for LevelStats {
    fn default() -> Self {
        Self {
            proper_time: 0.0,
            coordinate_time: 0.0,
            bots_killed: 0,
            bots_spaghettified: 0,
            shots_fired: 0,
            shots_hit: 0,
            damage_taken: 0.0,
            deepest_altitude: f64::MAX,
            health_remaining: 100.0,
        }
    }
}

pub struct Game {
    pub state: GameState,
    // Current level
    pub level_config: Option<LevelConfig>,
    pub level_number: u32,
    pub base_seed: u64,
    // Entities
    pub player: PlayerShip,
    pub bots: Vec<Bot>,
    pub projectiles: Vec<Projectile>,
    pub black_holes: Vec<BlackHole>,
    pub particles: Vec<ParticleEffect>,
    pub explosions: Vec<Explosion>,
    // Systems
    pub camera: Camera,
    pub rng: Rng,
    pub progression: Progression,
    pub story_state: StoryState,
    pub narrative_script: Vec<NarrativeEvent>,
    pub radio: RadioSystem,
    pub briefing: Option<BriefingState>,
    pub replay_recorder: ReplayRecorder,
    pub leaderboard: LocalLeaderboard,
    pub online: OnlineLeaderboard,
    pub level_stats: LevelStats,
    // Timing
    pub coordinate_time: f64,
    pub proper_time: f64,
    pub time_accumulator: f64,
    // Stars (generated once per level)
    pub stars: Vec<StarInstance>,
    // Active mines tracking
    pub player_mine_count: u32,
    // Beam state
    pub beam_active: bool,
    pub beam_segments: Vec<BeamSegment>,
    // State for retry
    pub is_retry: bool,
    // Title screen: which level's leaderboard to show
    pub title_leaderboard_level: u32,
    pub title_leaderboard_fetched: bool,
    // Cooldown timer to prevent held inputs from skipping screens
    pub screen_cooldown: f64,
    // DPI scale factor for HUD sizing
    pub dpi_scale: f32,
    // Player callsign
    pub display_name: String,
    pub name_entry: Option<NameEntryState>,
    // Frame counter for replay
    frame_number: u64,
    // Fired narrative event IDs (to handle once_only)
    fired_narrative_ids: Vec<String>,
    // Set to true when state should be persisted; platform layer checks and clears this
    pub needs_save: bool,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl Game {
    pub fn new(camera_width: f32, camera_height: f32) -> Self {
        let base_seed = 0xDEAD_BEEF_CAFE_u64;
        Self {
            state: GameState::Title,
            level_config: None,
            level_number: 0,
            base_seed,
            player: PlayerShip::new(Vec2::new(8.0, 0.0), Vec2::ZERO),
            bots: Vec::new(),
            projectiles: Vec::new(),
            black_holes: Vec::new(),
            particles: Vec::new(),
            explosions: Vec::new(),
            camera: Camera::new(camera_width, camera_height),
            rng: Rng::new(base_seed),
            progression: Progression::new(),
            story_state: StoryState::new(),
            narrative_script: build_script(),
            radio: RadioSystem::new(),
            briefing: None,
            replay_recorder: ReplayRecorder::new(0, 0),
            leaderboard: LocalLeaderboard::new(),
            online: OnlineLeaderboard::new(
                crate::leaderboard::online::DEFAULT_BASE_URL.to_string(),
                String::new(),
            ),
            level_stats: LevelStats::default(),
            coordinate_time: 0.0,
            proper_time: 0.0,
            time_accumulator: 0.0,
            stars: Vec::new(),
            player_mine_count: 0,
            beam_active: false,
            beam_segments: Vec::new(),
            is_retry: false,
            title_leaderboard_level: 1,
            title_leaderboard_fetched: false,
            screen_cooldown: 0.0,
            dpi_scale: 1.0,
            display_name: String::new(),
            name_entry: None,
            frame_number: 0,
            fired_narrative_ids: Vec::new(),
            needs_save: false,
        }
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    /// Load saved state and restore progression, story, seed. Call after new().
    pub fn load_save(&mut self, backend: &dyn SaveData) {
        if let Some(save) = backend.load() {
            self.base_seed = save.base_seed;
            self.progression.highest_level = save.highest_level;
            self.progression.unlocked_orbit_anchor = save.unlocked_orbit_anchor;
            self.progression.unlocked_tidal_flare = save.unlocked_tidal_flare;
            self.progression.unlocked_weapons.clear();
            for name in &save.unlocked_weapons {
                match name.as_str() {
                    "Railgun" => self.progression.unlocked_weapons.push(WeaponType::Railgun),
                    "MassDriver" => self.progression.unlocked_weapons.push(WeaponType::MassDriver),
                    "PhotonLance" => self.progression.unlocked_weapons.push(WeaponType::PhotonLance),
                    "GravityBomb" => self.progression.unlocked_weapons.push(WeaponType::GravityBomb),
                    "ImpulseRocket" => self.progression.unlocked_weapons.push(WeaponType::ImpulseRocket),
                    "TidalMine" => self.progression.unlocked_weapons.push(WeaponType::TidalMine),
                    _ => {}
                }
            }
            self.story_state.flags = save.story_flags;
            self.story_state.current_act = StoryState::get_act_for_level(save.highest_level);
            self.rng = Rng::new(self.base_seed);
            self.display_name = save.display_name.clone();

            // Restore or register online player
            if !save.online_player_id.is_empty() {
                self.online.set_player_id(save.online_player_id);
                self.online.set_display_name(save.display_name);
            } else if !save.display_name.is_empty() {
                // Have a name but no online ID — register
                self.online.set_display_name(save.display_name);
                self.online.register();
            }

            log::info!(
                "Loaded save: highest_level={}, weapons={}",
                save.highest_level,
                self.progression.unlocked_weapons.len()
            );
        }
    }

    /// Save current state. Call on level clear and periodically.
    pub fn save_game(&self, backend: &dyn SaveData) {
        let weapon_names: Vec<String> = self.progression.unlocked_weapons.iter().map(|w| {
            match w {
                WeaponType::Railgun => "Railgun",
                WeaponType::MassDriver => "MassDriver",
                WeaponType::PhotonLance => "PhotonLance",
                WeaponType::GravityBomb => "GravityBomb",
                WeaponType::ImpulseRocket => "ImpulseRocket",
                WeaponType::TidalMine => "TidalMine",
            }.to_string()
        }).collect();

        let save = SaveState {
            highest_level: self.progression.highest_level,
            base_seed: self.base_seed,
            unlocked_weapons: weapon_names,
            unlocked_orbit_anchor: self.progression.unlocked_orbit_anchor,
            unlocked_tidal_flare: self.progression.unlocked_tidal_flare,
            story_flags: self.story_state.flags.clone(),
            settings: crate::persistence::GameSettings::default(),
            display_name: self.display_name.clone(),
            online_player_id: self.online.player_id().unwrap_or_default().to_string(),
        };

        backend.save(&save);
        log::info!("Game saved: highest_level={}", save.highest_level);
    }

    // -----------------------------------------------------------------------
    // Level lifecycle
    // -----------------------------------------------------------------------

    pub fn start_level(&mut self, level_number: u32) {
        self.level_number = level_number;

        // Generate level config
        let config = generate_level(level_number, self.base_seed);

        // Unlock weapons for this level (don't advance highest_level yet — that happens on clear)
        self.progression.unlock_for_level(level_number);

        // Update story act
        self.story_state.current_act = StoryState::get_act_for_level(level_number);

        // Spawn black holes
        self.black_holes.clear();
        for bhc in &config.black_holes {
            if bhc.orbital_speed.abs() > 1e-12 {
                self.black_holes.push(BlackHole::new_binary_member(
                    bhc.mass,
                    bhc.schwarzschild_radius,
                    bhc.orbital_radius,
                    bhc.orbital_phase,
                    bhc.orbital_speed,
                ));
            } else {
                self.black_holes.push(BlackHole::new(
                    bhc.position,
                    bhc.mass,
                    bhc.schwarzschild_radius,
                ));
            }
        }

        // Gravity sources for orbital velocity computation
        let gravity_sources: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_gravity_source()).collect();

        // Total mass for circular orbit computation (simplification: use total)
        let total_mass: f64 = self.black_holes.iter().map(|bh| bh.mass).sum();

        // Spawn player in circular orbit
        let player_alt = config.player_start_altitude;
        let player_phase = config.player_start_phase;
        let player_pos = Vec2::new(
            player_alt * player_phase.cos(),
            player_alt * player_phase.sin(),
        );
        let v_circ = compute_circular_orbit_velocity(player_alt, total_mass, G);
        // Velocity is perpendicular to radial direction (prograde)
        let radial = player_pos.normalized();
        let prograde = radial.perpendicular();
        let player_vel = prograde * v_circ;
        self.player = PlayerShip::new(player_pos, player_vel);

        // Initialize player acceleration
        let player_acc = compute_gravitational_acceleration(player_pos, &gravity_sources);
        self.player.acceleration = player_acc;

        // Spawn bots
        self.bots.clear();
        let diff = difficulty(level_number);
        for spawn in &config.bot_spawns {
            let bot_pos = Vec2::new(
                spawn.altitude * spawn.phase.cos(),
                spawn.altitude * spawn.phase.sin(),
            );
            let bot_v_circ = compute_circular_orbit_velocity(spawn.altitude, total_mass, G);
            let bot_radial = bot_pos.normalized();
            let bot_prograde = bot_radial.perpendicular();
            let bot_vel = bot_prograde * bot_v_circ;

            let mut bot = Bot::new(spawn.archetype, bot_pos, bot_vel, diff);
            bot.acceleration = compute_gravitational_acceleration(bot_pos, &gravity_sources);
            if let Some(group) = spawn.swarm_group {
                bot.swarm_group_id = Some(group);
            }
            self.bots.push(bot);
        }

        // Clear projectiles and effects
        self.projectiles.clear();
        self.particles.clear();
        self.explosions.clear();
        self.beam_segments.clear();
        self.beam_active = false;
        self.player_mine_count = 0;

        // Timing
        self.coordinate_time = 0.0;
        self.proper_time = 0.0;
        self.time_accumulator = 0.0;
        self.frame_number = 0;

        // Stats
        self.level_stats = LevelStats::default();

        // Generate star field
        self.stars = StarfieldPipeline::generate_star_field(&mut self.rng, STAR_COUNT);

        // Replay recorder
        self.replay_recorder = ReplayRecorder::new(config.seed, level_number);

        // Radio system reset
        self.radio = RadioSystem::new();

        // Narrative: check for briefing and radio chatter for this level
        let mut briefing_lines: Option<Vec<DialogueLine>> = None;
        for event in &self.narrative_script {
            if self.fired_narrative_ids.contains(&event.id) && event.once_only {
                continue;
            }
            // Check prerequisites
            let prereqs_met = event.prerequisites.iter().all(|p| self.story_state.has_flag(p));
            if !prereqs_met {
                continue;
            }
            match &event.trigger {
                NarrativeTrigger::LevelStart(lvl) if *lvl == level_number => {
                    match &event.content {
                        NarrativeContent::Briefing(lines) => {
                            briefing_lines = Some(lines.clone());
                        }
                        NarrativeContent::RadioChatter(data) => {
                            self.radio.load_chatter(data.clone());
                        }
                        _ => {}
                    }
                    self.fired_narrative_ids.push(event.id.clone());
                    self.story_state.set_flag(&event.id);
                }
                NarrativeTrigger::LevelRange(lo, hi)
                    if level_number >= *lo && level_number <= *hi =>
                {
                    match &event.content {
                        NarrativeContent::RadioChatter(data) => {
                            self.radio.load_chatter(data.clone());
                        }
                        _ => {}
                    }
                    self.fired_narrative_ids.push(event.id.clone());
                    self.story_state.set_flag(&event.id);
                }
                NarrativeTrigger::ActTransition(act)
                    if *act == self.story_state.current_act
                        && !self.story_state.has_flag(&format!("act_transition_{}", act)) =>
                {
                    match &event.content {
                        NarrativeContent::Briefing(lines) => {
                            if briefing_lines.is_none() {
                                briefing_lines = Some(lines.clone());
                            }
                        }
                        NarrativeContent::RadioChatter(data) => {
                            self.radio.load_chatter(data.clone());
                        }
                        _ => {}
                    }
                    self.fired_narrative_ids.push(event.id.clone());
                    self.story_state.set_flag(&event.id);
                    self.story_state.set_flag(&format!("act_transition_{}", act));
                }
                _ => {}
            }
        }

        // If we have a briefing, go to Briefing state; otherwise straight to Playing
        if let Some(lines) = briefing_lines {
            self.briefing = Some(BriefingState::new(lines));
            self.state = GameState::Briefing;
        } else {
            self.briefing = None;
            self.state = GameState::Playing;
        }

        self.level_config = Some(config);
    }

    // -----------------------------------------------------------------------
    // Main update
    // -----------------------------------------------------------------------

    pub fn update(&mut self, dt_wall: f64, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        // Poll online leaderboard for async results
        self.online.poll();

        // Clamp large dt to prevent spiral of death
        let dt_wall = dt_wall.min(0.1);

        // Tick down screen cooldown
        if self.screen_cooldown > 0.0 {
            self.screen_cooldown -= dt_wall;
        }

        match self.state.clone() {
            GameState::NameEntry => {
                self.update_name_entry(dt_wall, actions, audio);
            }
            GameState::Title => {
                self.update_title(actions, audio);
            }
            GameState::Briefing => {
                self.update_briefing(dt_wall, actions, audio);
            }
            GameState::Playing => {
                self.update_playing(dt_wall, actions, audio);
            }
            GameState::Death { .. } => {
                self.update_death(actions, audio);
            }
            GameState::LevelClear { .. } => {
                self.update_level_clear(actions, audio);
            }
            GameState::Debrief => {
                self.update_debrief(actions, audio);
            }
            GameState::Paused => {
                self.update_paused(actions, audio);
            }
        }
    }

    // -----------------------------------------------------------------------
    // State handlers
    // -----------------------------------------------------------------------

    fn update_name_entry(&mut self, dt: f64, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        let s = self.dpi_scale;
        let vw = self.camera.viewport_width;
        let vh = self.camera.viewport_height;

        let entry = match self.name_entry.as_mut() {
            Some(e) => e,
            None => return,
        };

        // Tick down cooldown
        entry.input_cooldown -= dt;
        if entry.input_cooldown < 0.0 {
            entry.input_cooldown = 0.0;
        }

        // Grid layout constants (must match build_name_entry_hud)
        let char_scale = 2.0 * s;
        let char_w = 8.0 * char_scale;
        let cell_w = char_w * 2.2;
        let row_h = 14.0 * char_scale + 8.0 * s;
        let grid_cols = 13;
        let grid_total_w = grid_cols as f32 * cell_w;
        let grid_left = (vw - grid_total_w) * 0.5;
        let grid_top = vh * 0.3;
        let cell_h = 14.0 * char_scale + 4.0 * s;

        // Button layout for row 3
        let btn_scale = 2.0 * s;
        let btn_h = 14.0 * btn_scale + 8.0 * s;
        let del_w = 3.0 * 8.0 * btn_scale + 16.0 * s;
        let btn_gap = 24.0 * s;
        let confirm_w = 7.0 * 8.0 * btn_scale + 16.0 * s;
        let total_btn_w = del_w + btn_gap + confirm_w;
        let btn_left = (vw - total_btn_w) * 0.5;
        let row3_y = grid_top + 3.0 * row_h;

        // Process mouse aim → move cursor to grid cell under mouse
        for action in actions.iter() {
            if let InputAction::AimAt(world_pos) = action {
                // AimAt gives world coords, but we need screen coords for HUD
                // Convert world→screen
                let (sx, sy) = self.camera.world_to_screen(*world_pos);

                // Check character rows 0-2
                for row in 0..3usize {
                    let ry = grid_top + row as f32 * row_h;
                    if sy >= ry && sy < ry + cell_h {
                        for col in 0..13usize {
                            let cx = grid_left + col as f32 * cell_w;
                            if sx >= cx - 2.0 * s && sx < cx + char_w + 4.0 * s {
                                entry.cursor_row = row;
                                entry.cursor_col = col;
                            }
                        }
                    }
                }

                // Check row 3 buttons
                if sy >= row3_y && sy < row3_y + btn_h {
                    // DEL button
                    if sx >= btn_left && sx < btn_left + del_w {
                        entry.cursor_row = 3;
                        entry.cursor_col = 0;
                    }
                    // CONFIRM button
                    let confirm_x = btn_left + del_w + btn_gap;
                    if sx >= confirm_x && sx < confirm_x + confirm_w {
                        entry.cursor_row = 3;
                        entry.cursor_col = 1;
                    }
                }
            }
        }

        if entry.input_cooldown > 0.0 {
            return;
        }

        let cooldown = 0.15;

        for action in actions {
            match action {
                InputAction::ThrustRadialOut => {
                    let cols = NameEntryState::cols_in_row(entry.cursor_row);
                    entry.cursor_col = (entry.cursor_col + 1) % cols;
                    entry.input_cooldown = cooldown;
                    return;
                }
                InputAction::ThrustRadialIn => {
                    let cols = NameEntryState::cols_in_row(entry.cursor_row);
                    entry.cursor_col = if entry.cursor_col == 0 { cols - 1 } else { entry.cursor_col - 1 };
                    entry.input_cooldown = cooldown;
                    return;
                }
                InputAction::ThrustPrograde => {
                    entry.cursor_row = if entry.cursor_row == 0 { 3 } else { entry.cursor_row - 1 };
                    let cols = NameEntryState::cols_in_row(entry.cursor_row);
                    if entry.cursor_col >= cols { entry.cursor_col = cols - 1; }
                    entry.input_cooldown = cooldown;
                    return;
                }
                InputAction::ThrustRetrograde => {
                    entry.cursor_row = (entry.cursor_row + 1) % 4;
                    let cols = NameEntryState::cols_in_row(entry.cursor_row);
                    if entry.cursor_col >= cols { entry.cursor_col = cols - 1; }
                    entry.input_cooldown = cooldown;
                    return;
                }
                InputAction::Fire | InputAction::Confirm => {
                    if entry.cursor_row < 3 {
                        let grid = NameEntryState::grid();
                        let ch = grid[entry.cursor_row][entry.cursor_col];
                        if entry.chars.len() < entry.max_len {
                            entry.chars.push(ch);
                            audio.play_sound(SoundEvent::UIConfirm);
                        }
                    } else {
                        if entry.cursor_col == 0 {
                            entry.chars.pop();
                            audio.play_sound(SoundEvent::UIConfirm);
                        } else {
                            if !entry.chars.is_empty() {
                                entry.confirmed = true;
                                audio.play_sound(SoundEvent::UIConfirm);
                            }
                        }
                    }
                    entry.input_cooldown = cooldown;

                    if entry.confirmed {
                        let name: String = entry.chars.iter().collect();
                        self.display_name = name.clone();
                        self.name_entry = None;
                        self.needs_save = true;
                        self.state = GameState::Title;

                        if self.online.is_registered() {
                            // Already registered — update name on server
                            self.online.set_display_name(name.clone());
                            self.online.update_display_name(name);
                        } else {
                            // First time — register with this name
                            self.online.set_display_name(name);
                            self.online.register();
                        }
                    }
                    return;
                }
                _ => {}
            }
        }
    }

    fn update_title(&mut self, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        // Fetch leaderboard for the displayed level on first frame
        if !self.title_leaderboard_fetched {
            self.title_leaderboard_level = (self.progression.highest_level + 1).max(1);
            let config = generate_level(self.title_leaderboard_level, self.base_seed);
            self.online.fetch_leaderboard(config.seed, 10);
            self.title_leaderboard_fetched = true;
        }

        for action in actions {
            match action {
                InputAction::Confirm | InputAction::Fire => {
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.title_leaderboard_fetched = false;
                    let next_level = self.progression.highest_level + 1;
                    self.start_level(next_level.max(1));
                    return;
                }
                InputAction::NewGame => {
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.title_leaderboard_fetched = false;
                    self.new_game();
                    return;
                }
                InputAction::ChangeCallsign => {
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.title_leaderboard_fetched = false;
                    self.name_entry = Some(NameEntryState::new());
                    self.state = GameState::NameEntry;
                    return;
                }
                _ => {}
            }
        }
    }

    /// Reset all progress and start from level 1.
    pub fn new_game(&mut self) {
        self.progression = Progression::new();
        self.story_state = StoryState::new();
        self.fired_narrative_ids.clear();
        self.base_seed = 0xDEAD_BEEF_CAFE_u64;
        self.rng = Rng::new(self.base_seed);
        self.needs_save = true;
        self.start_level(1);
    }

    fn update_briefing(&mut self, dt: f64, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        if let Some(ref mut briefing) = self.briefing {
            briefing.update(dt);

            for action in actions {
                match action {
                    InputAction::Confirm | InputAction::Fire => {
                        briefing.advance();
                        audio.play_sound(SoundEvent::UIConfirm);
                    }
                    InputAction::ThrustPrograde => {
                        briefing.set_fast_forward(true);
                    }
                    InputAction::Pause => {
                        // Escape = skip briefing entirely
                        self.briefing = None;
                        self.state = GameState::Playing;
                        return;
                    }
                    _ => {}
                }
            }

            if briefing.is_all_done() {
                self.briefing = None;
                self.state = GameState::Playing;
            }
        } else {
            self.state = GameState::Playing;
        }
    }

    fn update_playing(
        &mut self,
        dt_wall: f64,
        actions: &[InputAction],
        audio: &mut dyn AudioBackend,
    ) {
        // Check for pause
        for action in actions {
            if *action == InputAction::Pause {
                self.state = GameState::Paused;
                return;
            }
        }

        // -------------------------------------------------------------------
        // 1. Process input actions
        // -------------------------------------------------------------------
        let mut thrust_dir = ThrustDirection::None;
        let mut wants_fire = false;
        let mut aim_world: Option<Vec2> = None;

        for action in actions {
            match action {
                InputAction::ThrustPrograde => thrust_dir = ThrustDirection::Prograde,
                InputAction::ThrustRetrograde => thrust_dir = ThrustDirection::Retrograde,
                InputAction::ThrustRadialIn => thrust_dir = ThrustDirection::RadialIn,
                InputAction::ThrustRadialOut => thrust_dir = ThrustDirection::RadialOut,
                InputAction::Fire => wants_fire = true,
                InputAction::SelectWeapon(slot) => {
                    let idx = (*slot as usize).saturating_sub(1);
                    if idx < 6 {
                        // Check if weapon is unlocked
                        let slots = weapon_slots();
                        if self.progression.has_weapon(slots[idx].weapon_type) {
                            self.player.active_weapon = idx;
                            audio.play_sound(SoundEvent::UISelect);
                        }
                    }
                }
                InputAction::ZoomIn => self.camera.zoom_in(),
                InputAction::ZoomOut => self.camera.zoom_out(),
                InputAction::AimAt(world_pos) => {
                    aim_world = Some(*world_pos);
                }
                InputAction::ActivateOrbitAnchor => {
                    if self.progression.unlocked_orbit_anchor
                        && self.player.orbit_anchor_cooldown <= 0.0
                        && !self.player.orbit_anchor_active
                    {
                        self.player.orbit_anchor_active = true;
                        self.player.orbit_anchor_timer = 3.0; // 3s proper time
                        self.player.orbit_anchor_cooldown = 15.0;
                    }
                }
                InputAction::ActivateTidalFlare => {
                    // Tidal flare: damage all nearby enemies based on dilation gradient
                    if self.progression.unlocked_tidal_flare
                        && self.player.tidal_flare_cooldown <= 0.0
                    {
                        self.player.tidal_flare_cooldown = 30.0;
                        self.activate_tidal_flare(audio);
                    }
                }
                _ => {}
            }
        }

        // Update turret angle from aim position
        if let Some(aim_pos) = aim_world {
            let diff = aim_pos - self.player.position;
            if diff.length_squared() > 1e-6 {
                self.player.turret_angle = diff.y.atan2(diff.x);
            }
        }

        // -------------------------------------------------------------------
        // 2. Compute player tau and steps_per_frame
        // -------------------------------------------------------------------
        let dilation_sources: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_dilation_source()).collect();
        let player_tau = compute_tau(self.player.position, &dilation_sources);
        self.player.tau = player_tau;

        let (num_steps, dt_step) =
            compute_steps_per_frame(dt_wall, player_tau, FIXED_DT_COORD, MAX_STEPS_PER_FRAME);

        // -------------------------------------------------------------------
        // 3. Physics sub-steps
        // -------------------------------------------------------------------
        for _ in 0..num_steps {
            self.physics_step(dt_step, thrust_dir, wants_fire, audio);

            // Early exit if player died
            if self.player.is_dead() {
                break;
            }
        }

        // If player died during physics, stop beam and transition to Death state
        if self.player.is_dead() {
            if self.beam_active {
                self.beam_active = false;
                self.beam_segments.clear();
                audio.play_sound(SoundEvent::PhotonLanceStop);
            }
            return; // state already set in physics_step
        }

        // Check level clear: all bots dead
        let all_bots_dead = self.bots.iter().all(|b| b.is_dead());
        if all_bots_dead && !self.bots.is_empty() {
            self.on_level_clear(audio);
            return;
        }
        // Also handle edge case: level started with no bots
        if self.bots.is_empty() && self.coordinate_time > 1.0 {
            self.on_level_clear(audio);
            return;
        }

        // -------------------------------------------------------------------
        // 5. Update proper time and coordinate time
        // -------------------------------------------------------------------
        let dt_proper = dt_wall * player_tau;
        self.proper_time += dt_proper;
        self.coordinate_time += dt_wall;
        self.player.proper_time += dt_proper;
        self.level_stats.proper_time = self.proper_time;
        self.level_stats.coordinate_time = self.coordinate_time;

        // -------------------------------------------------------------------
        // 6. Regenerate player shields/fuel
        // -------------------------------------------------------------------
        self.player.regenerate(dt_proper);

        // Tick ability cooldowns in proper time
        self.player.orbit_anchor_cooldown = (self.player.orbit_anchor_cooldown - dt_proper).max(0.0);
        self.player.tidal_flare_cooldown = (self.player.tidal_flare_cooldown - dt_proper).max(0.0);

        // Orbit anchor timer
        if self.player.orbit_anchor_active {
            self.player.orbit_anchor_timer -= dt_proper;
            if self.player.orbit_anchor_timer <= 0.0 {
                self.player.orbit_anchor_active = false;
            }
        }

        // Tick weapon cooldowns in proper time
        for cd in self.player.weapon_cooldowns.iter_mut() {
            *cd = (*cd - dt_proper).max(0.0);
        }

        // -------------------------------------------------------------------
        // 7. Update camera
        // -------------------------------------------------------------------
        let nearest_bh_pos = self
            .black_holes
            .iter()
            .min_by(|a, b| {
                let da = a.position.distance(self.player.position);
                let db = b.position.distance(self.player.position);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bh| bh.position)
            .unwrap_or(Vec2::ZERO);
        self.camera.update(self.player.position, nearest_bh_pos, dt_wall);

        // -------------------------------------------------------------------
        // 8. Update particles and explosions
        // -------------------------------------------------------------------
        for p in &mut self.particles {
            p.position = p.position + p.velocity * dt_wall;
            p.age += dt_wall;
        }
        self.particles.retain(|p| p.is_alive());

        for exp in &mut self.explosions {
            exp.timer += dt_wall;
        }
        self.explosions.retain(|e| !e.is_finished());

        // -------------------------------------------------------------------
        // 9. Update trails
        // -------------------------------------------------------------------
        if self.player.alive {
            self.player.trail.push(self.player.position);
            if self.player.trail.len() > MAX_TRAIL_LENGTH {
                self.player.trail.remove(0);
            }
        }
        for bot in &mut self.bots {
            if bot.alive {
                bot.trail.push(bot.position);
                if bot.trail.len() > BOT_MAX_TRAIL_LENGTH {
                    bot.trail.remove(0);
                }
            }
        }

        // -------------------------------------------------------------------
        // 10. Update audio params
        // -------------------------------------------------------------------
        let nearest_bh_dist = self
            .black_holes
            .iter()
            .map(|bh| self.player.position.distance(bh.position))
            .fold(f64::MAX, f64::min);
        let nearest_rs = self
            .black_holes
            .iter()
            .min_by(|a, b| {
                let da = a.position.distance(self.player.position);
                let db = b.position.distance(self.player.position);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bh| bh.schwarzschild_radius)
            .unwrap_or(1.0);

        let depth_factor = if nearest_bh_dist > 1e-6 {
            (1.0 - nearest_bh_dist / (MAX_RADIUS * 0.5)).clamp(0.0, 1.0)
        } else {
            1.0
        };

        audio.set_ambient_params(AmbientParams {
            depth_factor,
            player_tau,
        });

        // Heartbeat: BPM increases as altitude decreases
        let heartbeat_bpm = if nearest_bh_dist < nearest_rs * 6.0 {
            let ratio = (nearest_rs * 6.0 - nearest_bh_dist) / (nearest_rs * 6.0);
            60.0 + ratio * 120.0 // 60-180 BPM
        } else {
            0.0
        };
        audio.set_heartbeat_rate(heartbeat_bpm);
        audio.update(dt_wall);

        // -------------------------------------------------------------------
        // 11. Tick narrative radio
        // -------------------------------------------------------------------
        self.radio.update(dt_proper);

        // -------------------------------------------------------------------
        // 12. Check narrative triggers
        // -------------------------------------------------------------------
        self.check_narrative_triggers();

        // -------------------------------------------------------------------
        // 13. Record replay frame
        // -------------------------------------------------------------------
        self.replay_recorder.record_frame(
            self.frame_number,
            Vec::new(), // simplified: not encoding full actions
            aim_world.map(|v| v.x as f32).unwrap_or(0.0),
            aim_world.map(|v| v.y as f32).unwrap_or(0.0),
        );
        self.frame_number += 1;

        // Track deepest altitude
        let player_alt = self.player.position.length();
        if player_alt < self.level_stats.deepest_altitude {
            self.level_stats.deepest_altitude = player_alt;
        }
        self.level_stats.health_remaining = self.player.health;
    }

    fn update_death(&mut self, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        if self.screen_cooldown > 0.0 { return; }
        for action in actions {
            match action {
                InputAction::Confirm | InputAction::Fire => {
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.is_retry = true;
                    self.start_level(self.level_number);
                    return;
                }
                InputAction::Pause => {
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.beam_active = false;
                    self.beam_segments.clear();
                    self.title_leaderboard_fetched = false;
                    self.state = GameState::Title;
                    return;
                }
                _ => {}
            }
        }
    }

    fn update_level_clear(&mut self, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        if self.screen_cooldown > 0.0 { return; }
        for action in actions {
            if *action == InputAction::Confirm || *action == InputAction::Fire {
                audio.play_sound(SoundEvent::UIConfirm);
                // Check for debrief narrative
                let has_debrief = self.narrative_script.iter().any(|e| {
                    matches!(&e.trigger, NarrativeTrigger::LevelClear(l) if *l == self.level_number)
                        && matches!(&e.content, NarrativeContent::Debrief(_))
                });
                if has_debrief {
                    self.state = GameState::Debrief;
                } else {
                    // Advance to next level
                    self.is_retry = false;
                    self.start_level(self.level_number + 1);
                }
                return;
            }
        }
    }

    fn update_debrief(&mut self, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        for action in actions {
            if *action == InputAction::Confirm || *action == InputAction::Fire {
                audio.play_sound(SoundEvent::UIConfirm);
                self.is_retry = false;
                self.start_level(self.level_number + 1);
                return;
            }
        }
    }

    fn update_paused(&mut self, actions: &[InputAction], audio: &mut dyn AudioBackend) {
        for action in actions {
            match action {
                InputAction::Pause => {
                    // Escape again = resume
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.state = GameState::Playing;
                    return;
                }
                InputAction::Confirm | InputAction::Fire => {
                    // Enter = resume
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.state = GameState::Playing;
                    return;
                }
                InputAction::SelectWeapon(1) => {
                    // 1 = restart level
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.is_retry = true;
                    self.start_level(self.level_number);
                    return;
                }
                InputAction::SelectWeapon(2) => {
                    // 2 = quit to title
                    audio.play_sound(SoundEvent::UIConfirm);
                    self.beam_active = false;
                    self.beam_segments.clear();
                    self.title_leaderboard_fetched = false;
                    self.state = GameState::Title;
                    return;
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Physics sub-step (one fixed step of the simulation)
    // -----------------------------------------------------------------------

    fn physics_step(
        &mut self,
        dt: f64,
        thrust_dir: ThrustDirection,
        wants_fire: bool,
        audio: &mut dyn AudioBackend,
    ) {
        let gravity_sources: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_gravity_source()).collect();
        let dilation_sources: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_dilation_source()).collect();

        // Update black hole positions (binary systems)
        for bh in &mut self.black_holes {
            bh.update(dt);
        }

        // ---- Player thrust ----
        let thrust_vec = self.compute_player_thrust(thrust_dir, dt);
        self.player.thrust_direction = thrust_dir;

        // Spawn thrust particles
        if thrust_vec.length_squared() > 1e-6 {
            let p = spawn_thrust_particle(self.player.position, self.player.velocity, thrust_vec);
            self.particles.push(p);
        }

        // ---- Integrate player ----
        if self.player.alive {
            let mut vs = VerletState {
                position: self.player.position,
                velocity: self.player.velocity,
                acceleration: self.player.acceleration,
            };

            if self.player.orbit_anchor_active {
                // Orbit anchor: freeze velocity adjustments, just integrate gravity
                integrate_step(&mut vs, dt, &gravity_sources);
            } else if thrust_vec.length_squared() > 1e-12 {
                integrate_step_with_thrust(&mut vs, dt, &gravity_sources, thrust_vec);
            } else {
                integrate_step(&mut vs, dt, &gravity_sources);
            }

            self.player.position = vs.position;
            self.player.velocity = vs.velocity;
            self.player.acceleration = vs.acceleration;

            // Update player tau
            self.player.tau = compute_tau(self.player.position, &dilation_sources);
        }

        // ---- Integrate bots (index-based to satisfy borrow checker) ----
        let bot_count = self.bots.len();
        let diff_scale = difficulty(self.level_number);

        // Collect AI decisions separately so we can borrow self immutably for AiContext
        struct BotDecision {
            thrust: Vec2,
            fire: bool,
            turret_angle: f64,
            new_goal: Option<crate::entities::bot::BotGoal>,
        }
        let mut decisions: Vec<Option<BotDecision>> = Vec::with_capacity(bot_count);

        for i in 0..bot_count {
            if self.bots[i].is_dead() {
                decisions.push(None);
                continue;
            }

            let bot = &mut self.bots[i];
            bot.tau = compute_tau(bot.position, &dilation_sources);
            let dt_proper_bot = dt * bot.tau;

            bot.time_since_last_decision += dt_proper_bot;
            bot.proper_time += dt_proper_bot;
            bot.regenerate(dt_proper_bot);
            bot.weapon_cooldown = (bot.weapon_cooldown - dt_proper_bot).max(0.0);

            let should_decide = bot.time_since_last_decision >= bot.decision_interval;
            if should_decide {
                bot.time_since_last_decision = 0.0;
            }

            decisions.push(if should_decide { Some(BotDecision {
                thrust: Vec2::ZERO,
                fire: false,
                turret_angle: 0.0,
                new_goal: None,
            }) } else { None });
        }

        // Run AI decisions (needs shared borrow of self.bots, self.player, etc.)
        let bh_masses: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_gravity_source()).collect();
        let bh_rs_vec: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_dilation_source()).collect();

        for i in 0..bot_count {
            if decisions[i].is_none() {
                continue;
            }
            // We need to call run_ai_tick with a reference to the bot and an AiContext
            // that borrows other fields. Use unsafe-free approach: clone the bot for the call.
            let bot_snapshot = self.bots[i].clone();

            let mut ctx = AiContext {
                player: &self.player,
                bots: &self.bots,
                projectiles: &self.projectiles,
                black_holes: &bh_masses,
                black_hole_positions_rs: &bh_rs_vec,
                rng: &mut self.rng,
                difficulty: diff_scale,
            };

            let output = run_ai_tick(&bot_snapshot, &mut ctx);
            decisions[i] = Some(BotDecision {
                thrust: output.thrust,
                fire: output.fire,
                turret_angle: output.turret_angle,
                new_goal: output.new_goal,
            });
        }

        // Apply decisions and integrate physics
        let mut fire_requests: Vec<usize> = Vec::new();

        for i in 0..bot_count {
            if self.bots[i].is_dead() {
                continue;
            }

            let mut bot_thrust = Vec2::ZERO;
            let mut bot_wants_fire = false;

            if let Some(decision) = &decisions[i] {
                bot_thrust = decision.thrust;
                bot_wants_fire = decision.fire;
                self.bots[i].turret_angle = decision.turret_angle;
                if let Some(ref new_goal) = decision.new_goal {
                    self.bots[i].current_goal = new_goal.clone();
                }
            }

            // Integrate bot physics
            let bot = &self.bots[i];
            let mut vs = VerletState {
                position: bot.position,
                velocity: bot.velocity,
                acceleration: bot.acceleration,
            };

            let thrust_magnitude = THRUST_MAGNITUDE * 0.8;
            let effective_thrust = if bot_thrust.length_squared() > 1e-6 {
                bot_thrust.normalized() * thrust_magnitude
            } else {
                Vec2::ZERO
            };

            if effective_thrust.length_squared() > 1e-12 {
                integrate_step_with_thrust(&mut vs, dt, &gravity_sources, effective_thrust);
            } else {
                integrate_step(&mut vs, dt, &gravity_sources);
            }

            self.bots[i].position = vs.position;
            self.bots[i].velocity = vs.velocity;
            self.bots[i].acceleration = vs.acceleration;

            if bot_wants_fire && self.bots[i].weapon_cooldown <= 0.0 {
                fire_requests.push(i);
            }
        }

        // Execute bot fire requests
        for &bi in &fire_requests {
            self.spawn_bot_projectile_at(bi, audio);
        }

        // ---- Update projectiles ----
        self.update_projectiles(dt, &gravity_sources, &dilation_sources, audio);

        // ---- Player weapon firing ----
        if wants_fire && self.player.alive {
            self.handle_player_fire(dt, audio);
        } else {
            // Stop beam if not firing
            if self.beam_active {
                self.beam_active = false;
                self.beam_segments.clear();
                audio.play_sound(SoundEvent::PhotonLanceStop);
            }
        }

        // ---- Collision detection ----
        self.check_collisions(dt, audio);

        // ---- Update orbital params for player ----
        if !self.black_holes.is_empty() {
            let primary_bh = &self.black_holes[0];
            self.player.orbital_params = compute_orbital_params(
                self.player.position,
                self.player.velocity,
                primary_bh.position,
                primary_bh.mass,
                G,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Player thrust computation
    // -----------------------------------------------------------------------

    fn compute_player_thrust(&mut self, dir: ThrustDirection, dt: f64) -> Vec2 {
        if dir == ThrustDirection::None || !self.player.alive {
            return Vec2::ZERO;
        }

        let fuel_cost = FUEL_THRUST_COST * dt;
        if !self.player.consume_fuel(fuel_cost) {
            return Vec2::ZERO;
        }

        // Compute thrust direction relative to orbit
        let radial = self.player.position.normalized();
        let prograde = radial.perpendicular(); // counterclockwise tangent

        let direction = match dir {
            ThrustDirection::Prograde => prograde,
            ThrustDirection::Retrograde => -prograde,
            ThrustDirection::RadialIn => -radial,
            ThrustDirection::RadialOut => radial,
            ThrustDirection::None => return Vec2::ZERO,
        };

        direction * THRUST_MAGNITUDE
    }

    // -----------------------------------------------------------------------
    // Player fire / beam weapon
    // -----------------------------------------------------------------------

    fn handle_player_fire(&mut self, dt: f64, audio: &mut dyn AudioBackend) {
        let slots = weapon_slots();
        let weapon_idx = self.player.active_weapon;

        if weapon_idx >= 6 {
            return;
        }

        let slot = &slots[weapon_idx];

        // Check unlock (use current level, not highest_level, since weapons unlock on start)
        if !self.progression.has_weapon(slot.weapon_type) {
            return;
        }

        // Check cooldown
        if self.player.weapon_cooldowns[weapon_idx] > 0.0 {
            // Beam weapon: still active even on "cooldown" (it's per-tick)
            if slot.weapon_type != WeaponType::PhotonLance {
                return;
            }
        }

        match slot.weapon_type {
            WeaponType::PhotonLance => {
                self.fire_photon_lance(dt, audio);
            }
            _ => {
                self.fire_weapon(weapon_idx, audio);
            }
        }
    }

    fn fire_weapon(&mut self, weapon_idx: usize, audio: &mut dyn AudioBackend) {
        let slots = weapon_slots();
        let slot = &slots[weapon_idx];

        // Check fuel cost
        if slot.fuel_cost > 0.0 && !self.player.consume_fuel(slot.fuel_cost) {
            return;
        }

        // Mine limit
        if slot.weapon_type == WeaponType::TidalMine && self.player_mine_count >= MAX_PLAYER_MINES {
            return;
        }

        let proj = match slot.weapon_type {
            WeaponType::Railgun => {
                audio.play_sound(SoundEvent::RailgunFire);
                Projectile::new_railgun(
                    self.player.position,
                    self.player.velocity,
                    self.player.turret_angle,
                    self.player.tau,
                    true,
                )
            }
            WeaponType::MassDriver => {
                audio.play_sound(SoundEvent::MassDriverFire);
                Projectile::new_mass_driver(
                    self.player.position,
                    self.player.velocity,
                    self.player.turret_angle,
                    self.player.tau,
                    true,
                )
            }
            WeaponType::ImpulseRocket => {
                audio.play_sound(SoundEvent::ImpulseRocketFire);
                Projectile::new_impulse_rocket(
                    self.player.position,
                    self.player.velocity,
                    self.player.turret_angle,
                    self.player.tau,
                    true,
                )
            }
            WeaponType::GravityBomb => {
                audio.play_sound(SoundEvent::GravityBombDeploy);
                Projectile::new_gravity_bomb(
                    self.player.position,
                    self.player.velocity,
                    self.player.turret_angle,
                    self.player.tau,
                    true,
                )
            }
            WeaponType::TidalMine => {
                audio.play_sound(SoundEvent::TidalMineDeploy);
                self.player_mine_count += 1;
                Projectile::new_tidal_mine(
                    self.player.position,
                    self.player.velocity,
                    self.player.turret_angle,
                    self.player.tau,
                    true,
                )
            }
            WeaponType::PhotonLance => unreachable!(),
        };

        self.projectiles.push(proj);
        self.player.weapon_cooldowns[weapon_idx] = slot.cooldown;
        self.level_stats.shots_fired += 1;
    }

    fn fire_photon_lance(&mut self, dt: f64, audio: &mut dyn AudioBackend) {
        let dt_proper = dt * self.player.tau;

        // Consume fuel per-second
        let fuel_cost = PHOTON_FUEL_COST * dt_proper;
        if !self.player.consume_fuel(fuel_cost) {
            if self.beam_active {
                self.beam_active = false;
                self.beam_segments.clear();
                audio.play_sound(SoundEvent::PhotonLanceStop);
            }
            return;
        }

        if !self.beam_active {
            self.beam_active = true;
            // Stop any lingering lance sound before starting a new one
            audio.play_sound(SoundEvent::PhotonLanceStop);
            audio.play_sound(SoundEvent::PhotonLanceStart);
        }

        // Raycast beam
        let origin = self.player.position;
        let dir = Vec2::from_angle(self.player.turret_angle);
        let end_point = beam_endpoint(origin, self.player.turret_angle);

        // Check beam hits against bots
        let mut closest_hit: Option<(usize, f64)> = None;
        for (i, bot) in self.bots.iter().enumerate() {
            if bot.is_dead() {
                continue;
            }
            if let Some(t) = ray_circle_intersection(origin, dir, bot.position, BOT_RADIUS) {
                if t > 0.0 && t < BEAM_RANGE {
                    if closest_hit.is_none() || t < closest_hit.unwrap().1 {
                        closest_hit = Some((i, t));
                    }
                }
            }
        }

        // Apply beam damage
        if let Some((bot_idx, t)) = closest_hit {
            let target_tau = self.bots[bot_idx].tau;
            let dmg = compute_beam_damage(dt_proper, target_tau);
            self.bots[bot_idx].apply_damage(dmg);
            self.level_stats.damage_taken += 0.0; // beam hits are shots_hit
            self.level_stats.shots_hit += 1;
            self.level_stats.shots_fired += 1;

            if self.bots[bot_idx].is_dead() {
                self.on_bot_killed(bot_idx, false, audio);
            }

            // Visual: beam ends at hit point
            let hit_point = origin + dir * t;
            let color = Color::WHITE;
            self.beam_segments = vec![BeamSegment {
                start_pos: origin.as_f32_array(),
                end_pos: hit_point.as_f32_array(),
                width: BEAM_WIDTH as f32,
                color: color.to_array(),
                _pad: [0.0; 3],
            }];
        } else {
            // Beam goes full range
            let color = Color::WHITE;
            self.beam_segments = vec![BeamSegment {
                start_pos: origin.as_f32_array(),
                end_pos: end_point.as_f32_array(),
                width: BEAM_WIDTH as f32,
                color: color.to_array(),
                _pad: [0.0; 3],
            }];
        }
    }

    // -----------------------------------------------------------------------
    // Projectile update
    // -----------------------------------------------------------------------

    fn update_projectiles(
        &mut self,
        dt: f64,
        gravity_sources: &[(Vec2, f64)],
        dilation_sources: &[(Vec2, f64)],
        _audio: &mut dyn AudioBackend,
    ) {
        // Build gravity sources including active gravity bombs
        let mut all_gravity = gravity_sources.to_vec();
        for proj in &self.projectiles {
            if proj.alive && proj.bomb_active && proj.projectile_type == ProjectileType::GravityBomb
            {
                all_gravity.push((proj.position, proj.bomb_mass));
            }
        }

        for proj in &mut self.projectiles {
            if !proj.alive {
                continue;
            }

            let _proj_tau = compute_tau(proj.position, dilation_sources);

            // Tick lifetime in coordinate time
            proj.lifetime -= dt;
            if proj.lifetime <= 0.0 {
                proj.alive = false;
                // Reclaim mine count
                if proj.projectile_type == ProjectileType::TidalMine && proj.owner_is_player {
                    self.player_mine_count = self.player_mine_count.saturating_sub(1);
                }
                continue;
            }

            // Special: gravity bomb arming
            if proj.projectile_type == ProjectileType::GravityBomb && !proj.bomb_active {
                proj.bomb_timer -= dt;
                if proj.bomb_timer <= 0.0 {
                    proj.bomb_active = true;
                    // Gravity bomb decelerates to near-stop when active
                    proj.velocity = proj.velocity * 0.1;
                }
            }

            // Special: tidal mine settling into orbit
            if proj.projectile_type == ProjectileType::TidalMine && !proj.mine_orbiting {
                // After initial travel, try to stabilize at current altitude
                let alt = proj.position.length();
                if alt > 0.5 {
                    proj.mine_altitude = alt;
                    // Check if velocity is roughly circular (within 30%)
                    let needed_v = if !gravity_sources.is_empty() {
                        let total_mass: f64 = gravity_sources.iter().map(|g| g.1).sum();
                        compute_circular_orbit_velocity(alt, total_mass, G)
                    } else {
                        1.0
                    };
                    let current_v = proj.velocity.length();
                    if (current_v - needed_v).abs() / needed_v < 0.3 {
                        proj.mine_orbiting = true;
                    }
                }
            }

            // Special: impulse rocket tracking
            if proj.projectile_type == ProjectileType::ImpulseRocket
                && proj.tracking_strength > 0.0
            {
                // Track nearest enemy (if player-owned) or player (if bot-owned)
                let target_pos = if proj.owner_is_player {
                    self.bots
                        .iter()
                        .filter(|b| b.alive)
                        .min_by(|a, b| {
                            let da = a.position.distance_squared(proj.position);
                            let db = b.position.distance_squared(proj.position);
                            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|b| b.position)
                } else if self.player.alive {
                    Some(self.player.position)
                } else {
                    None
                };

                if let Some(target) = target_pos {
                    let to_target = (target - proj.position).normalized();
                    let current_dir = proj.velocity.normalized();
                    let steer = to_target - current_dir;
                    let speed = proj.velocity.length();
                    proj.velocity = (proj.velocity + steer * proj.tracking_strength * dt)
                        .normalized()
                        * speed;
                }
            }

            // Integrate projectile physics
            let mut vs = VerletState {
                position: proj.position,
                velocity: proj.velocity,
                acceleration: proj.acceleration,
            };
            integrate_step(&mut vs, dt, &all_gravity);
            proj.position = vs.position;
            proj.velocity = vs.velocity;
            proj.acceleration = vs.acceleration;
        }
    }

    // -----------------------------------------------------------------------
    // Collision detection
    // -----------------------------------------------------------------------

    fn check_collisions(&mut self, _dt: f64, audio: &mut dyn AudioBackend) {
        let center_of_mass = Vec2::ZERO; // black holes orbit around origin

        // ---- Event horizon checks ----
        for bh in &self.black_holes {
            // Player vs event horizon
            if self.player.alive
                && check_event_horizon(self.player.position, bh.position, bh.schwarzschild_radius, KILL_FACTOR)
            {
                // Spaghettification death
                let particles = spawn_spaghettification_particles(
                    self.player.position,
                    bh.position,
                    20,
                );
                self.particles.extend(particles);
                audio.play_sound(SoundEvent::Spaghettification);
                self.player.alive = false;

                let stats = self.build_current_stats();
                self.screen_cooldown = 1.5;
                self.state = GameState::Death {
                    cause: DeathCause::Spaghettified,
                    stats,
                };
                return;
            }

            // Bots vs event horizon
            for i in 0..self.bots.len() {
                if self.bots[i].is_dead() {
                    continue;
                }
                if check_event_horizon(
                    self.bots[i].position,
                    bh.position,
                    bh.schwarzschild_radius,
                    KILL_FACTOR,
                ) {
                    let particles = spawn_spaghettification_particles(
                        self.bots[i].position,
                        bh.position,
                        12,
                    );
                    self.particles.extend(particles);
                    audio.play_sound(SoundEvent::Spaghettification);
                    self.bots[i].alive = false;
                    self.level_stats.bots_spaghettified += 1;
                    self.level_stats.bots_killed += 1;
                }
            }

            // Projectiles vs event horizon
            for proj in &mut self.projectiles {
                if !proj.alive {
                    continue;
                }
                if check_event_horizon(
                    proj.position,
                    bh.position,
                    bh.schwarzschild_radius,
                    KILL_FACTOR,
                ) {
                    proj.alive = false;
                    if proj.projectile_type == ProjectileType::TidalMine && proj.owner_is_player {
                        self.player_mine_count = self.player_mine_count.saturating_sub(1);
                    }
                }
            }
        }

        // ---- Escape checks ----
        if self.player.alive && check_escape(self.player.position, center_of_mass, MAX_RADIUS) {
            self.player.alive = false;
            audio.play_sound(SoundEvent::WarningEscapeVelocity);
            let stats = self.build_current_stats();
            self.screen_cooldown = 1.5;
            self.state = GameState::Death {
                cause: DeathCause::LostToVoid,
                stats,
            };
            return;
        }

        // Bots that escape just die silently
        for bot in &mut self.bots {
            if !bot.is_dead() && check_escape(bot.position, center_of_mass, MAX_RADIUS) {
                bot.alive = false;
            }
        }

        // Projectiles that escape
        for proj in &mut self.projectiles {
            if proj.alive && check_escape(proj.position, center_of_mass, MAX_RADIUS) {
                proj.alive = false;
                if proj.projectile_type == ProjectileType::TidalMine && proj.owner_is_player {
                    self.player_mine_count = self.player_mine_count.saturating_sub(1);
                }
            }
        }

        // ---- Projectile-ship collisions ----
        // We collect hits first, then apply, to avoid borrow issues
        let mut player_hits: Vec<(usize, f64)> = Vec::new(); // (proj_idx, damage)
        let mut bot_hits: Vec<(usize, usize, f64)> = Vec::new(); // (proj_idx, bot_idx, damage)
        let mut tidal_mine_triggers: Vec<usize> = Vec::new();

        for (pi, proj) in self.projectiles.iter().enumerate() {
            if !proj.alive {
                continue;
            }

            // Tidal mine proximity trigger (against non-owners)
            if proj.projectile_type == ProjectileType::TidalMine && proj.mine_orbiting {
                if proj.owner_is_player {
                    // Check against all bots
                    for (bi, bot) in self.bots.iter().enumerate() {
                        if bot.is_dead() {
                            continue;
                        }
                        let dist = proj.position.distance(bot.position);
                        if dist < proj.mine_trigger_radius {
                            tidal_mine_triggers.push(pi);
                            bot_hits.push((pi, bi, proj.damage));
                            break;
                        }
                    }
                } else {
                    // Bot mine triggers on player
                    if self.player.alive {
                        let dist = proj.position.distance(self.player.position);
                        if dist < proj.mine_trigger_radius {
                            tidal_mine_triggers.push(pi);
                            player_hits.push((pi, proj.damage));
                        }
                    }
                }
                continue; // tidal mines don't do normal collision
            }

            // Normal projectile-vs-ship collision
            if proj.owner_is_player {
                // Check vs bots
                for (bi, bot) in self.bots.iter().enumerate() {
                    if bot.is_dead() {
                        continue;
                    }
                    if circle_circle(proj.position, proj.radius, bot.position, BOT_RADIUS) {
                        bot_hits.push((pi, bi, proj.damage));
                        break;
                    }
                }
            } else {
                // Check vs player
                if self.player.alive
                    && circle_circle(
                        proj.position,
                        proj.radius,
                        self.player.position,
                        SHIP_RADIUS,
                    )
                {
                    player_hits.push((pi, proj.damage));
                }
            }
        }

        // Apply bot hits
        for &(pi, bi, damage) in &bot_hits {
            if pi < self.projectiles.len() {
                self.projectiles[pi].alive = false;
            }
            if bi < self.bots.len() && self.bots[bi].alive {
                self.bots[bi].apply_damage(damage);
                audio.play_sound(SoundEvent::ShieldHit);

                // Explosion at hit point
                let hit_pos = self.bots[bi].position;
                self.explosions.push(Explosion::new(hit_pos, 0.3, Color::ORANGE));
                let hit_particles =
                    spawn_explosion_particles(hit_pos, Vec2::ZERO, Color::ORANGE, 8);
                self.particles.extend(hit_particles);

                self.level_stats.shots_hit += 1;

                if self.bots[bi].is_dead() {
                    self.on_bot_killed(bi, false, audio);
                }
            }
        }

        // Apply player hits
        for &(pi, damage) in &player_hits {
            if pi < self.projectiles.len() {
                let proj_type = self.projectiles[pi].projectile_type;
                self.projectiles[pi].alive = false;

                let old_health = self.player.health;
                self.player.apply_damage(damage);
                let actual_damage = old_health - self.player.health;
                self.level_stats.damage_taken += actual_damage;

                if self.player.shields > 0.0 || actual_damage < damage {
                    audio.play_sound(SoundEvent::ShieldHit);
                } else {
                    audio.play_sound(SoundEvent::HullHit);
                }

                // Explosion
                let hit_pos = self.player.position;
                self.explosions.push(Explosion::new(hit_pos, 0.3, Color::RED));
                let hit_particles =
                    spawn_explosion_particles(hit_pos, Vec2::ZERO, Color::RED, 8);
                self.particles.extend(hit_particles);

                if self.player.is_dead() {
                    let stats = self.build_current_stats();
                    self.screen_cooldown = 1.5;
                    self.state = GameState::Death {
                        cause: DeathCause::Weapon(proj_type),
                        stats,
                    };
                    return;
                }
            }
        }

        // Handle tidal mine triggers (create explosions)
        for &pi in &tidal_mine_triggers {
            if pi < self.projectiles.len() {
                let mine_pos = self.projectiles[pi].position;
                if self.projectiles[pi].owner_is_player {
                    self.player_mine_count = self.player_mine_count.saturating_sub(1);
                }
                self.projectiles[pi].alive = false;
                self.explosions.push(Explosion::new(mine_pos, 1.0, Color::RED));
                let mine_particles =
                    spawn_explosion_particles(mine_pos, Vec2::ZERO, Color::RED, 16);
                self.particles.extend(mine_particles);
                audio.play_sound(SoundEvent::Explosion);
            }
        }

        // Clean up dead projectiles
        self.projectiles.retain(|p| p.alive);
    }

    // -----------------------------------------------------------------------
    // Bot projectile spawning
    // -----------------------------------------------------------------------

    fn spawn_bot_projectile_at(&mut self, bot_idx: usize, audio: &mut dyn AudioBackend) {
        let bot = &self.bots[bot_idx];
        // Bots fire railgun by default, some archetypes fire mass driver or impulse rocket
        let proj = match bot.archetype {
            BotArchetype::Diver | BotArchetype::Commander | BotArchetype::Anchor => {
                audio.play_sound(SoundEvent::MassDriverFire);
                Projectile::new_mass_driver(
                    bot.position,
                    bot.velocity,
                    bot.turret_angle,
                    bot.tau,
                    false,
                )
            }
            _ => {
                audio.play_sound(SoundEvent::RailgunFire);
                Projectile::new_railgun(
                    bot.position,
                    bot.velocity,
                    bot.turret_angle,
                    bot.tau,
                    false,
                )
            }
        };

        let cooldown = match bot.archetype {
            BotArchetype::Skirmisher => 0.6,
            BotArchetype::Diver => 1.2,
            BotArchetype::Vulture => 0.8,
            BotArchetype::Anchor => 1.0,
            BotArchetype::Swarm => 0.4,
            BotArchetype::Commander => 0.8,
        };

        self.projectiles.push(proj);
        self.bots[bot_idx].weapon_cooldown = cooldown;
    }

    // -----------------------------------------------------------------------
    // Narrative triggers
    // -----------------------------------------------------------------------

    pub fn check_narrative_triggers(&mut self) {
        let player_alt = self.player.position.length();
        let proper_time = self.proper_time;
        let _level_number = self.level_number;

        let mut chatter_to_load: Vec<RadioChatterData> = Vec::new();

        for event in &self.narrative_script {
            if self.fired_narrative_ids.contains(&event.id) && event.once_only {
                continue;
            }
            let prereqs_met = event
                .prerequisites
                .iter()
                .all(|p| self.story_state.has_flag(p));
            if !prereqs_met {
                continue;
            }

            let triggered = match &event.trigger {
                NarrativeTrigger::DepthReached(alt) => player_alt <= *alt,
                NarrativeTrigger::ProperTimeSurvived(t) => proper_time >= *t,
                NarrativeTrigger::BotTypeSeen(archetype) => {
                    self.bots.iter().any(|b| b.archetype == *archetype && b.alive)
                }
                NarrativeTrigger::CommanderDefeated => {
                    self.bots
                        .iter()
                        .any(|b| b.archetype == BotArchetype::Commander && b.is_dead())
                }
                _ => false, // LevelStart/LevelClear/LevelRange/ActTransition handled in start_level
            };

            if triggered {
                match &event.content {
                    NarrativeContent::RadioChatter(data) => {
                        chatter_to_load.push(data.clone());
                    }
                    _ => {}
                }
                self.fired_narrative_ids.push(event.id.clone());
                self.story_state.set_flag(&event.id);
            }
        }

        for data in chatter_to_load {
            self.radio.load_chatter(data);
        }
    }

    // -----------------------------------------------------------------------
    // Tidal flare ability
    // -----------------------------------------------------------------------

    fn activate_tidal_flare(&mut self, audio: &mut dyn AudioBackend) {
        let dilation_sources: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_dilation_source()).collect();
        let player_tau = self.player.tau;

        let flare_radius = 8.0;
        for bot in &mut self.bots {
            if bot.is_dead() {
                continue;
            }
            let dist = bot.position.distance(self.player.position);
            if dist < flare_radius {
                let bot_tau = compute_tau(bot.position, &dilation_sources);
                // Damage scales with dilation gradient
                let tau_diff = (player_tau - bot_tau).abs();
                let damage = 20.0 + tau_diff * 50.0;
                bot.apply_damage(damage);

                if bot.is_dead() {
                    self.level_stats.bots_killed += 1;
                    let exp = Explosion::new(bot.position, 0.8, Color::MAGENTA);
                    self.explosions.push(exp);
                    let particles =
                        spawn_explosion_particles(bot.position, Vec2::ZERO, Color::MAGENTA, 12);
                    self.particles.extend(particles);
                    audio.play_sound(SoundEvent::Explosion);
                }
            }
        }

        // Visual: burst of particles from the player
        let particles =
            spawn_explosion_particles(self.player.position, Vec2::ZERO, Color::MAGENTA, 24);
        self.particles.extend(particles);
        audio.play_sound(SoundEvent::Explosion);
    }

    // -----------------------------------------------------------------------
    // Bot death handling
    // -----------------------------------------------------------------------

    fn on_bot_killed(
        &mut self,
        bot_idx: usize,
        spaghettified: bool,
        audio: &mut dyn AudioBackend,
    ) {
        self.level_stats.bots_killed += 1;
        if spaghettified {
            self.level_stats.bots_spaghettified += 1;
        }

        let bot_pos = self.bots[bot_idx].position;
        let exp = Explosion::new(bot_pos, 0.6, Color::ORANGE);
        self.explosions.push(exp);
        let particles = spawn_explosion_particles(bot_pos, Vec2::ZERO, Color::ORANGE, 12);
        self.particles.extend(particles);
        audio.play_sound(SoundEvent::Explosion);
    }

    // -----------------------------------------------------------------------
    // Level clear
    // -----------------------------------------------------------------------

    fn on_level_clear(&mut self, _audio: &mut dyn AudioBackend) {
        let stats = self.build_current_stats();

        // Compute score
        let mut level_score = LevelScore {
            proper_time_elapsed: stats.proper_time,
            coordinate_time_elapsed: stats.coordinate_time,
            total_damage_taken: stats.damage_taken,
            shots_fired: stats.shots_fired,
            shots_hit: stats.shots_hit,
            deepest_altitude: stats.deepest_altitude,
            bots_spaghettified: stats.bots_spaghettified,
            accuracy: 0.0,
            dilation_ratio: 0.0,
            health_remaining: stats.health_remaining / 100.0,
        };
        level_score.finalize();

        let score = compute_score(&level_score, self.level_number);

        // Submit to leaderboard
        let seed = self.level_config.as_ref().map(|c| c.seed).unwrap_or(0);
        let entry = LeaderboardEntry {
            level_number: self.level_number,
            seed,
            score,
            proper_time: stats.proper_time,
            accuracy: level_score.accuracy,
            health_remaining: level_score.health_remaining,
            timestamp: platform_timestamp(),
        };
        self.leaderboard.submit(entry);

        // Submit score online
        log::info!("Level clear: online registered={}, player_id={:?}",
            self.online.is_registered(), self.online.player_id());
        if self.online.is_registered() {
            let seed = self.level_config.as_ref().map(|c| c.seed).unwrap_or(0);
            self.online.submit_score(ScoreSubmission {
                player_id: self.online.player_id().unwrap_or_default().to_string(),
                level_number: self.level_number,
                seed,
                score,
                proper_time: stats.proper_time,
                coordinate_time: stats.coordinate_time,
                accuracy: if stats.shots_fired > 0 {
                    stats.shots_hit as f64 / stats.shots_fired as f64
                } else {
                    0.0
                },
                health_remaining: stats.health_remaining,
                deepest_altitude: stats.deepest_altitude,
                bots_killed: stats.bots_killed,
                bots_spaghettified: stats.bots_spaghettified,
                shots_fired: stats.shots_fired,
                shots_hit: stats.shots_hit,
                damage_taken: stats.damage_taken,
                dilation_ratio: if stats.proper_time > 0.0 {
                    stats.coordinate_time / stats.proper_time
                } else {
                    1.0
                },
            });
        }

        // Advance progression (sets highest_level) and persist
        self.progression.advance_to_level(self.level_number);
        self.needs_save = true;

        // Check for level-clear narrative
        for event in &self.narrative_script {
            if self.fired_narrative_ids.contains(&event.id) && event.once_only {
                continue;
            }
            if let NarrativeTrigger::LevelClear(lvl) = &event.trigger {
                if *lvl == self.level_number {
                    self.fired_narrative_ids.push(event.id.clone());
                    self.story_state.set_flag(&event.id);
                }
            }
        }

        // Leaderboard will auto-fetch after score submission completes (see online.rs poll)

        self.screen_cooldown = 1.5; // prevent held fire from skipping
        self.state = GameState::LevelClear {
            stats,
            score,
        };
    }

    fn build_current_stats(&self) -> LevelStats {
        LevelStats {
            proper_time: self.proper_time,
            coordinate_time: self.coordinate_time,
            bots_killed: self.level_stats.bots_killed,
            bots_spaghettified: self.level_stats.bots_spaghettified,
            shots_fired: self.level_stats.shots_fired,
            shots_hit: self.level_stats.shots_hit,
            damage_taken: self.level_stats.damage_taken,
            deepest_altitude: if self.level_stats.deepest_altitude == f64::MAX {
                self.player.position.length()
            } else {
                self.level_stats.deepest_altitude
            },
            health_remaining: self.player.health,
        }
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Screen HUD builders for non-Playing states
    // -----------------------------------------------------------------------

    fn build_name_entry_hud(&self, vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
        let mut els = Vec::new();

        // Dark overlay
        els.push(HudElement::Rect {
            x: 0.0, y: 0.0, w: vw, h: vh,
            color: [0.0, 0.0, 0.05, 0.92],
        });

        let entry = match &self.name_entry {
            Some(e) => e,
            None => return els,
        };

        // Title
        let title = "ENTER YOUR CALLSIGN";
        let title_scale = 3.0 * s;
        let title_w = title.len() as f32 * 8.0 * title_scale;
        els.push(HudElement::Text {
            x: (vw - title_w) * 0.5,
            y: vh * 0.12,
            text: title.to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
            scale: title_scale,
        });

        // Character grid
        let grid = NameEntryState::grid();
        let char_scale = 2.0 * s;
        let char_w = 8.0 * char_scale; // width of one character glyph
        let cell_w = char_w * 2.2; // spacing between cells
        let row_h = 14.0 * char_scale + 8.0 * s; // row height

        let grid_cols = 13;
        let grid_total_w = grid_cols as f32 * cell_w;
        let grid_left = (vw - grid_total_w) * 0.5;
        let grid_top = vh * 0.3;

        let highlight_color: [f32; 4] = [0.0, 0.8, 1.0, 0.3]; // cyan highlight
        let text_color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let dim_color: [f32; 4] = [0.6, 0.6, 0.6, 1.0];

        // Rows 0-2: character grid
        for row in 0..3 {
            let chars = grid[row];
            let y = grid_top + row as f32 * row_h;
            for (col, &ch) in chars.iter().enumerate() {
                let x = grid_left + col as f32 * cell_w;

                // Highlight background if cursor is here
                if entry.cursor_row == row && entry.cursor_col == col {
                    els.push(HudElement::Rect {
                        x: x - 2.0 * s,
                        y: y - 2.0 * s,
                        w: char_w + 4.0 * s,
                        h: 14.0 * char_scale + 4.0 * s,
                        color: highlight_color,
                    });
                }

                els.push(HudElement::Text {
                    x,
                    y,
                    text: ch.to_string(),
                    color: if entry.cursor_row == row && entry.cursor_col == col {
                        [0.0, 1.0, 1.0, 1.0]
                    } else {
                        text_color
                    },
                    scale: char_scale,
                });
            }
        }

        // Row 3: [DEL] and [CONFIRM]
        let row3_y = grid_top + 3.0 * row_h;
        let btn_scale = 2.0 * s;
        let btn_h = 14.0 * btn_scale + 8.0 * s;

        // DEL button
        let del_text = "DEL";
        let del_w = del_text.len() as f32 * 8.0 * btn_scale + 16.0 * s;
        let btn_gap = 24.0 * s;
        let confirm_text = "CONFIRM";
        let confirm_w = confirm_text.len() as f32 * 8.0 * btn_scale + 16.0 * s;
        let total_btn_w = del_w + btn_gap + confirm_w;
        let btn_left = (vw - total_btn_w) * 0.5;

        // DEL rect + text
        let del_selected = entry.cursor_row == 3 && entry.cursor_col == 0;
        els.push(HudElement::Rect {
            x: btn_left,
            y: row3_y,
            w: del_w,
            h: btn_h,
            color: if del_selected { highlight_color } else { [0.15, 0.15, 0.2, 0.8] },
        });
        els.push(HudElement::Text {
            x: btn_left + 8.0 * s,
            y: row3_y + 4.0 * s,
            text: del_text.to_string(),
            color: if del_selected { [0.0, 1.0, 1.0, 1.0] } else { dim_color },
            scale: btn_scale,
        });

        // CONFIRM rect + text
        let confirm_selected = entry.cursor_row == 3 && entry.cursor_col == 1;
        let confirm_x = btn_left + del_w + btn_gap;
        els.push(HudElement::Rect {
            x: confirm_x,
            y: row3_y,
            w: confirm_w,
            h: btn_h,
            color: if confirm_selected { highlight_color } else { [0.15, 0.15, 0.2, 0.8] },
        });
        els.push(HudElement::Text {
            x: confirm_x + 8.0 * s,
            y: row3_y + 4.0 * s,
            text: confirm_text.to_string(),
            color: if confirm_selected { [0.0, 1.0, 1.0, 1.0] } else { dim_color },
            scale: btn_scale,
        });

        // Current name display
        let name_str: String = entry.chars.iter().collect();
        // Blinking cursor: use coordinate_time for animation
        let blink = (self.coordinate_time * 3.0) as u64 % 2 == 0;
        let cursor_ch = if blink { "_" } else { " " };
        let display = format!("> {}{}", name_str, cursor_ch);
        let name_scale = 2.2 * s;
        let name_w = display.len() as f32 * 8.0 * name_scale;
        els.push(HudElement::Text {
            x: (vw - name_w) * 0.5,
            y: row3_y + btn_h + 24.0 * s,
            text: display,
            color: [0.0, 1.0, 1.0, 1.0],
            scale: name_scale,
        });

        els
    }

    fn build_pause_hud(&self, vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
        let mut els = Vec::new();

        els.push(HudElement::Rect {
            x: 0.0, y: 0.0, w: vw, h: vh,
            color: [0.0, 0.0, 0.05, 0.75],
        });

        let title = "PAUSED";
        let title_scale = 3.5 * s;
        let title_w = title.len() as f32 * 8.0 * title_scale;
        els.push(HudElement::Text {
            x: (vw - title_w) * 0.5,
            y: vh * 0.25,
            text: title.to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
            scale: title_scale,
        });

        let opt_scale = 2.0 * s;
        let line_h = 14.0 * opt_scale + 10.0 * s;
        let mut y = vh * 0.42;

        let options = [
            ("ESC / ENTER", "RESUME", [0.8, 0.8, 0.8, 1.0]),
            ("1", "RESTART LEVEL", [1.0, 0.7, 0.3, 1.0]),
            ("2", "QUIT TO TITLE", [1.0, 0.4, 0.4, 1.0]),
        ];

        for (key, label, color) in &options {
            let text = format!("{} - {}", key, label);
            let text_w = text.len() as f32 * 8.0 * opt_scale;
            els.push(HudElement::Text {
                x: (vw - text_w) * 0.5,
                y,
                text,
                color: *color,
                scale: opt_scale,
            });
            y += line_h;
        }

        els
    }

    fn build_title_hud(&self, vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
        let mut els = Vec::new();

        // Dark overlay
        els.push(HudElement::Rect {
            x: 0.0, y: 0.0, w: vw, h: vh,
            color: [0.0, 0.0, 0.05, 0.85],
        });

        // Title (centered)
        let title = "GRAVITY WELL ARENA";
        let title_scale = 3.5 * s;
        let title_w = title.len() as f32 * 8.0 * title_scale;
        els.push(HudElement::Text {
            x: (vw - title_w) * 0.5,
            y: 40.0 * s,
            text: title.to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
            scale: title_scale,
        });

        // Left side: menu options
        let left_x = 60.0 * s;
        let menu_scale = 1.8 * s;
        let menu_line_h = 14.0 * menu_scale + 10.0 * s;
        let mut my = vh * 0.35;

        // Callsign
        let callsign_text = format!("CALLSIGN: {}", self.display_name);
        els.push(HudElement::Text {
            x: left_x, y: my,
            text: callsign_text,
            color: [0.2, 0.9, 1.0, 1.0],
            scale: menu_scale,
        });
        my += menu_line_h + 8.0 * s;

        // Continue/start
        let prompt = if self.progression.highest_level > 0 {
            format!("ENTER - CONTINUE (LEVEL {})", self.progression.highest_level + 1)
        } else {
            "ENTER - START".to_string()
        };
        els.push(HudElement::Text {
            x: left_x, y: my,
            text: prompt,
            color: [0.8, 0.8, 0.8, 1.0],
            scale: menu_scale,
        });
        my += menu_line_h;

        let opt_scale = 1.5 * s;
        let opt_line_h = 14.0 * opt_scale + 6.0 * s;

        els.push(HudElement::Text {
            x: left_x, y: my,
            text: "C - CHANGE CALLSIGN".to_string(),
            color: [0.5, 0.5, 0.5, 0.7],
            scale: opt_scale,
        });
        my += opt_line_h;

        els.push(HudElement::Text {
            x: left_x, y: my,
            text: "N - NEW GAME".to_string(),
            color: [0.5, 0.5, 0.5, 0.7],
            scale: opt_scale,
        });

        // Right side: leaderboard
        let lb_x = vw * 0.55;
        let lb_scale = 1.5 * s;
        let lb_line_h = 14.0 * lb_scale + 3.0 * s;
        let mut lb_y = vh * 0.35;

        let lb_title = format!("LEADERBOARD - LEVEL {}", self.title_leaderboard_level);
        els.push(HudElement::Text {
            x: lb_x, y: lb_y - lb_line_h,
            text: lb_title,
            color: [0.6, 0.6, 0.6, 1.0],
            scale: lb_scale,
        });

        let entries = &self.online.cached_leaderboard;
        if !entries.is_empty() {
            for entry in entries.iter().take(10) {
                let is_me = self.online.player_id()
                    .map(|id| id == entry.player_id)
                    .unwrap_or(false);

                let line = format!(
                    "#{:<3} {:>7}  {}",
                    entry.rank,
                    entry.score,
                    truncate_name(&entry.display_name, 12),
                );

                let color = if is_me {
                    [0.0, 1.0, 1.0, 1.0]
                } else {
                    [0.7, 0.7, 0.7, 1.0]
                };

                els.push(HudElement::Text {
                    x: lb_x, y: lb_y,
                    text: line,
                    color,
                    scale: lb_scale,
                });
                lb_y += lb_line_h;
            }
        } else {
            els.push(HudElement::Text {
                x: lb_x, y: lb_y,
                text: "NO SCORES YET".to_string(),
                color: [0.4, 0.4, 0.4, 1.0],
                scale: lb_scale,
            });
        }

        els
    }

    fn build_briefing_hud(&self, vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
        let mut els = Vec::new();

        // Dark overlay
        els.push(HudElement::Rect {
            x: 0.0, y: 0.0, w: vw, h: vh,
            color: [0.0, 0.0, 0.05, 0.8],
        });

        // Level header
        let header = format!("LEVEL {}", self.level_number);
        let header_scale = 3.0 * s;
        let header_w = header.len() as f32 * 8.0 * header_scale;
        els.push(HudElement::Text {
            x: (vw - header_w) * 0.5,
            y: 60.0 * s,
            text: header,
            color: [1.0, 1.0, 1.0, 1.0],
            scale: header_scale,
        });

        // Briefing text from BriefingState
        if let Some(ref briefing) = self.briefing {
            if let Some((speaker, text)) = briefing.get_current_display() {
                let speaker_name = speaker.name();
                let speaker_color = speaker.color();

                // Speaker name
                let name_scale = 2.2 * s;
                let margin = 80.0 * s;
                els.push(HudElement::Text {
                    x: margin,
                    y: vh * 0.35,
                    text: format!("[{}]", speaker_name),
                    color: speaker_color,
                    scale: name_scale,
                });

                // Dialogue text — wrap at ~50 chars per line
                let text_scale = 2.0 * s;
                let max_chars = ((vw - margin * 2.0) / (8.0 * text_scale)) as usize;
                let max_chars = max_chars.max(20);
                let mut y = vh * 0.35 + 14.0 * name_scale + 8.0 * s;
                let line_h = 14.0 * text_scale;

                for line in wrap_text(text, max_chars) {
                    els.push(HudElement::Text {
                        x: margin,
                        y,
                        text: line,
                        color: [0.85, 0.85, 0.85, 1.0],
                        scale: text_scale,
                    });
                    y += line_h;
                }
            }

            // Advance prompt
            let prompt = if briefing.is_all_done() {
                "PRESS ENTER TO LAUNCH"
            } else {
                "PRESS ENTER TO CONTINUE"
            };
            let prompt_scale = 1.8 * s;
            let prompt_w = prompt.len() as f32 * 8.0 * prompt_scale;
            els.push(HudElement::Text {
                x: (vw - prompt_w) * 0.5,
                y: vh - 60.0 * s,
                text: prompt.to_string(),
                color: [0.5, 0.5, 0.5, 1.0],
                scale: prompt_scale,
            });

            // Skip hint
            let skip = "ESC - SKIP";
            let skip_scale = 1.4 * s;
            let skip_w = skip.len() as f32 * 8.0 * skip_scale;
            els.push(HudElement::Text {
                x: (vw - skip_w) * 0.5,
                y: vh - 60.0 * s + 14.0 * prompt_scale + 4.0 * s,
                text: skip.to_string(),
                color: [0.4, 0.4, 0.4, 0.6],
                scale: skip_scale,
            });
        }

        els
    }

    fn build_death_hud(&self, cause: &DeathCause, stats: &LevelStats, vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
        let mut els = Vec::new();

        els.push(HudElement::Rect {
            x: 0.0, y: 0.0, w: vw, h: vh,
            color: [0.1, 0.0, 0.0, 0.8],
        });

        let cause_text = match cause {
            DeathCause::Weapon(_) => "DESTROYED",
            DeathCause::Spaghettified => "SPAGHETTIFIED",
            DeathCause::LostToVoid => "LOST TO THE VOID",
        };

        let title_scale = 3.0 * s;
        let title_w = cause_text.len() as f32 * 8.0 * title_scale;
        els.push(HudElement::Text {
            x: (vw - title_w) * 0.5,
            y: vh * 0.25,
            text: cause_text.to_string(),
            color: [1.0, 0.2, 0.2, 1.0],
            scale: title_scale,
        });

        // Stats
        let stat_scale = 1.8 * s;
        let margin = 80.0 * s;
        let line_h = 16.0 * stat_scale;
        let mut y = vh * 0.4;

        let stat_lines = [
            format!("TIME SURVIVED: {:.1}s", stats.proper_time),
            format!("BOTS KILLED: {}", stats.bots_killed),
            format!("SHOTS FIRED: {}", stats.shots_fired),
            format!("ACCURACY: {:.0}%", if stats.shots_fired > 0 { stats.shots_hit as f64 / stats.shots_fired as f64 * 100.0 } else { 0.0 }),
        ];

        for line in &stat_lines {
            els.push(HudElement::Text {
                x: margin, y,
                text: line.clone(),
                color: [0.8, 0.8, 0.8, 1.0],
                scale: stat_scale,
            });
            y += line_h;
        }

        let prompt_scale = 2.0 * s;
        let opt_scale = 1.6 * s;
        let mut py = vh - 80.0 * s;

        let retry = "ENTER - RETRY";
        let retry_w = retry.len() as f32 * 8.0 * prompt_scale;
        els.push(HudElement::Text {
            x: (vw - retry_w) * 0.5, y: py,
            text: retry.to_string(),
            color: [0.7, 0.7, 0.7, 1.0],
            scale: prompt_scale,
        });
        py += 14.0 * prompt_scale + 6.0 * s;

        let quit = "ESC - QUIT TO TITLE";
        let quit_w = quit.len() as f32 * 8.0 * opt_scale;
        els.push(HudElement::Text {
            x: (vw - quit_w) * 0.5, y: py,
            text: quit.to_string(),
            color: [0.5, 0.5, 0.5, 0.7],
            scale: opt_scale,
        });

        els
    }

    fn build_level_clear_hud(&self, stats: &LevelStats, score: u64, vw: f32, vh: f32, s: f32) -> Vec<HudElement> {
        let mut els = Vec::new();

        els.push(HudElement::Rect {
            x: 0.0, y: 0.0, w: vw, h: vh,
            color: [0.0, 0.02, 0.05, 0.8],
        });

        // Title
        let title = "LEVEL CLEAR";
        let title_scale = 3.0 * s;
        let title_w = title.len() as f32 * 8.0 * title_scale;
        els.push(HudElement::Text {
            x: (vw - title_w) * 0.5,
            y: 30.0 * s,
            text: title.to_string(),
            color: [0.2, 1.0, 0.4, 1.0],
            scale: title_scale,
        });

        // Score
        let score_text = format!("SCORE: {}", score);
        let score_scale = 2.5 * s;
        let score_w = score_text.len() as f32 * 8.0 * score_scale;
        els.push(HudElement::Text {
            x: (vw - score_w) * 0.5,
            y: 30.0 * s + 14.0 * title_scale + 10.0 * s,
            text: score_text,
            color: [1.0, 0.9, 0.2, 1.0],
            scale: score_scale,
        });

        // Rank from online submission
        if let Some((rank, total)) = self.online.last_rank {
            let rank_text = format!("RANK: #{} of {}", rank, total);
            let rank_scale = 1.8 * s;
            let rank_w = rank_text.len() as f32 * 8.0 * rank_scale;
            els.push(HudElement::Text {
                x: (vw - rank_w) * 0.5,
                y: 30.0 * s + 14.0 * title_scale + 14.0 * score_scale + 16.0 * s,
                text: rank_text,
                color: [0.2, 0.9, 1.0, 1.0],
                scale: rank_scale,
            });
        }

        // Left column: stats
        let stat_scale = 1.5 * s;
        let left_margin = 40.0 * s;
        let line_h = 14.0 * stat_scale + 4.0 * s;
        let mut y = vh * 0.38;

        let dilation = if stats.proper_time > 0.0 {
            stats.coordinate_time / stats.proper_time
        } else {
            1.0
        };

        let stat_lines = [
            format!("PROPER TIME: {:.1}s", stats.proper_time),
            format!("COORD TIME:  {:.1}s ({:.1}x)", stats.coordinate_time, dilation),
            format!("BOTS KILLED: {}", stats.bots_killed),
            format!("SPAGHETTIFIED: {}", stats.bots_spaghettified),
            format!("ACCURACY: {:.0}%", if stats.shots_fired > 0 { stats.shots_hit as f64 / stats.shots_fired as f64 * 100.0 } else { 0.0 }),
            format!("HEALTH: {:.0}", stats.health_remaining),
            format!("DEEPEST: {:.1} Rs", stats.deepest_altitude),
        ];

        els.push(HudElement::Text {
            x: left_margin, y: y - line_h,
            text: "YOUR STATS".to_string(),
            color: [0.6, 0.6, 0.6, 1.0],
            scale: stat_scale,
        });

        for line in &stat_lines {
            els.push(HudElement::Text {
                x: left_margin, y,
                text: line.clone(),
                color: [0.8, 0.8, 0.8, 1.0],
                scale: stat_scale,
            });
            y += line_h;
        }

        // Right column: online leaderboard
        let lb_x = vw * 0.55;
        let lb_scale = 1.5 * s;
        let lb_line_h = 14.0 * lb_scale + 3.0 * s;
        let mut lb_y = vh * 0.38;

        let entries = &self.online.cached_leaderboard;
        if !entries.is_empty() {
            els.push(HudElement::Text {
                x: lb_x, y: lb_y - lb_line_h,
                text: "LEADERBOARD".to_string(),
                color: [0.6, 0.6, 0.6, 1.0],
                scale: lb_scale,
            });

            for entry in entries.iter().take(10) {
                let is_me = self.online.player_id()
                    .map(|id| id == entry.player_id)
                    .unwrap_or(false);

                let line = format!(
                    "#{:<3} {:>7}  {}",
                    entry.rank,
                    entry.score,
                    truncate_name(&entry.display_name, 12),
                );

                let color = if is_me {
                    [0.0, 1.0, 1.0, 1.0] // cyan highlight for your entry
                } else {
                    [0.7, 0.7, 0.7, 1.0]
                };

                els.push(HudElement::Text {
                    x: lb_x, y: lb_y,
                    text: line,
                    color,
                    scale: lb_scale,
                });
                lb_y += lb_line_h;
            }
        } else {
            els.push(HudElement::Text {
                x: lb_x, y: lb_y,
                text: "LOADING LEADERBOARD...".to_string(),
                color: [0.4, 0.4, 0.4, 1.0],
                scale: lb_scale,
            });
        }

        // Bottom prompts
        let prompt_scale = 1.8 * s;
        let prompt = "ENTER - NEXT LEVEL";
        let prompt_w = prompt.len() as f32 * 8.0 * prompt_scale;
        els.push(HudElement::Text {
            x: (vw - prompt_w) * 0.5,
            y: vh - 50.0 * s,
            text: prompt.to_string(),
            color: [0.6, 0.6, 0.6, 1.0],
            scale: prompt_scale,
        });

        els
    }

    pub fn build_render_scene(&self, time: f32) -> RenderScene {
        let camera_uniform = CameraUniform::from_camera(&self.camera);

        // Black holes
        let black_hole_data: Vec<BlackHoleData> = self
            .black_holes
            .iter()
            .map(|bh| BlackHoleData {
                position: bh.position.as_f32_array(),
                radius: bh.schwarzschild_radius as f32,
                time,
            })
            .collect();

        // Trails
        let mut trails: Vec<TrailData> = Vec::new();

        // Player trail
        if self.player.alive && !self.player.trail.is_empty() {
            let len = self.player.trail.len();
            let vertices: Vec<TrailVertex> = self
                .player
                .trail
                .iter()
                .enumerate()
                .map(|(i, pos)| {
                    let alpha = (i as f32 + 1.0) / len as f32;
                    TrailVertex {
                        position: pos.as_f32_array(),
                        alpha,
                        color: [Color::player().r, Color::player().g, Color::player().b],
                    }
                })
                .collect();
            trails.push(TrailData { vertices });
        }

        // Bot trails
        for bot in &self.bots {
            if bot.is_dead() || bot.trail.is_empty() {
                continue;
            }
            let len = bot.trail.len();
            let bot_color = match bot.archetype {
                BotArchetype::Skirmisher => Color::skirmisher(),
                BotArchetype::Diver => Color::diver(),
                BotArchetype::Vulture => Color::vulture(),
                BotArchetype::Anchor => Color::anchor(),
                BotArchetype::Swarm => Color::swarm(),
                BotArchetype::Commander => Color::commander(),
            };
            let vertices: Vec<TrailVertex> = bot
                .trail
                .iter()
                .enumerate()
                .map(|(i, pos)| {
                    let alpha = (i as f32 + 1.0) / len as f32;
                    TrailVertex {
                        position: pos.as_f32_array(),
                        alpha,
                        color: [bot_color.r, bot_color.g, bot_color.b],
                    }
                })
                .collect();
            trails.push(TrailData { vertices });
        }

        // Ship instances
        let mut ship_instances: Vec<ShipInstance> = Vec::new();
        if self.player.alive {
            ship_instances.push(ship_instance_from_player(&self.player));
        }
        for bot in &self.bots {
            if !bot.is_dead() {
                ship_instances.push(ship_instance_from_bot(bot));
            }
        }

        // Projectile instances
        let projectile_instances: Vec<ShipInstance> = self
            .projectiles
            .iter()
            .filter(|p| p.alive && p.projectile_type != ProjectileType::PhotonLance)
            .map(|p| projectile_instance(p))
            .collect();

        // Beam segments
        let beam_segments = self.beam_segments.clone();

        // Particles
        let particle_instances: Vec<ParticleInstance> = self
            .particles
            .iter()
            .filter(|p| p.is_alive())
            .map(|p| ParticleInstance {
                position: p.position.as_f32_array(),
                velocity: p.velocity.as_f32_array(),
                size: p.size,
                color: p.color.to_array(),
                age: p.age as f32,
                _pad: [0.0; 2],
            })
            .collect();

        // HUD
        let hud_elements = self.build_hud_elements();

        // Depth factor for post-processing
        let nearest_bh_dist = self
            .black_holes
            .iter()
            .map(|bh| self.player.position.distance(bh.position))
            .fold(f64::MAX, f64::min);
        let depth_factor = if nearest_bh_dist < f64::MAX {
            (1.0 - nearest_bh_dist / (MAX_RADIUS * 0.5)).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        RenderScene {
            camera: camera_uniform,
            stars: self.stars.clone(),
            black_holes: black_hole_data,
            trails,
            ship_instances,
            projectile_instances,
            beam_segments,
            particles: particle_instances,
            hud_elements,
            time,
            depth_factor,
        }
    }

    fn build_hud_elements(&self) -> Vec<HudElement> {
        let s = self.dpi_scale;
        let vw = self.camera.viewport_width;
        let vh = self.camera.viewport_height;

        match &self.state {
            GameState::NameEntry => {
                return self.build_name_entry_hud(vw, vh, s);
            }
            GameState::Title => {
                return self.build_title_hud(vw, vh, s);
            }
            GameState::Briefing => {
                return self.build_briefing_hud(vw, vh, s);
            }
            GameState::Death { cause, stats } => {
                return self.build_death_hud(cause, stats, vw, vh, s);
            }
            GameState::LevelClear { stats, score } => {
                return self.build_level_clear_hud(stats, *score, vw, vh, s);
            }
            GameState::Debrief => {
                return self.build_briefing_hud(vw, vh, s);
            }
            GameState::Paused => {
                return self.build_pause_hud(vw, vh, s);
            }
            GameState::Playing => {}
        }

        let dilation_sources: Vec<(Vec2, f64)> =
            self.black_holes.iter().map(|bh| bh.as_dilation_source()).collect();

        // Weapon availability
        let slots = weapon_slots();
        let mut weapons_available = [false; 6];
        for (i, slot) in slots.iter().enumerate() {
            weapons_available[i] = self.progression.has_weapon(slot.weapon_type);
        }

        // Weapon cooldowns normalized to [0, 1]
        let mut weapon_cooldowns = [0.0_f64; 6];
        for (i, slot) in slots.iter().enumerate() {
            if slot.cooldown > 0.0 {
                weapon_cooldowns[i] = self.player.weapon_cooldowns[i] / slot.cooldown;
            }
        }

        // Bot depths relative to player
        let bot_depths: Vec<(BotArchetype, f64)> = self
            .bots
            .iter()
            .filter(|b| b.alive)
            .map(|b| {
                let bot_tau = compute_tau(b.position, &dilation_sources);
                let relative_tau = if self.player.tau > 0.01 {
                    bot_tau / self.player.tau
                } else {
                    bot_tau
                };
                (b.archetype, relative_tau)
            })
            .collect();

        // Escape velocity warning
        let escape_warning = if !self.black_holes.is_empty() {
            let bh = &self.black_holes[0];
            let v_esc = crate::physics::orbit::compute_escape_velocity(
                self.player.position.distance(bh.position).max(0.01),
                bh.mass,
                G,
            );
            self.player.velocity.length() >= v_esc * 0.95
        } else {
            false
        };

        // Low fuel warning
        let low_fuel = self.player.fuel < 15.0;

        // Trajectory preview color
        let trajectory_color = if !self.black_holes.is_empty() {
            let periapsis = self.player.orbital_params.periapsis;
            let rs = self.black_holes[0].schwarzschild_radius;
            trajectory_safety_color(periapsis, rs)
        } else {
            Color::GREEN
        };

        let hud_state = HudState {
            coordinate_time: self.coordinate_time,
            proper_time: self.proper_time,
            world_tempo: if self.player.tau > 0.01 {
                1.0 / self.player.tau
            } else {
                100.0
            },
            health: self.player.health,
            shields: self.player.shields,
            fuel: self.player.fuel,
            active_weapon: self.player.active_weapon,
            weapon_cooldowns,
            weapons_available,
            bot_depths,
            escape_velocity_warning: escape_warning,
            low_fuel_warning: low_fuel,
            trajectory_color,
            viewport_width: self.camera.viewport_width,
            viewport_height: self.camera.viewport_height,
            dpi_scale: self.dpi_scale,
        };

        let mut elements = build_hud(&hud_state);

        // Radio chatter overlay (top-center during gameplay)
        if let Some((speaker, text)) = self.radio.get_display_text() {
            if !text.is_empty() {
                let s = self.dpi_scale;
                let vw = self.camera.viewport_width;
                let radio_scale = 1.8 * s;
                let char_w = 8.0 * radio_scale;

                // Speaker tag
                let speaker_str = format!("[{}]", speaker.name());
                let speaker_color = speaker.color();
                let speaker_w = speaker_str.len() as f32 * char_w;
                let x = (vw - speaker_w) * 0.5;
                let y = 60.0 * s;

                // Dark background behind radio text
                let max_line_chars = ((vw * 0.7) / char_w) as usize;
                let max_line_chars = max_line_chars.max(20);
                let lines = wrap_text(text, max_line_chars);
                let line_h = 14.0 * radio_scale;
                let total_h = line_h * (lines.len() + 1) as f32 + 16.0 * s;
                let bg_w = vw * 0.75;
                let bg_x = (vw - bg_w) * 0.5;

                elements.push(HudElement::Rect {
                    x: bg_x, y: y - 8.0 * s,
                    w: bg_w, h: total_h,
                    color: [0.0, 0.0, 0.0, 0.6],
                });

                elements.push(HudElement::Text {
                    x, y,
                    text: speaker_str,
                    color: speaker_color,
                    scale: radio_scale,
                });

                let text_x = bg_x + 12.0 * s;
                let mut text_y = y + line_h;
                for line in lines {
                    elements.push(HudElement::Text {
                        x: text_x, y: text_y,
                        text: line,
                        color: [0.85, 0.85, 0.85, 1.0],
                        scale: radio_scale,
                    });
                    text_y += line_h;
                }
            }
        }

        elements
    }
}

/// Simple word-wrap: break `text` into lines of at most `max_chars` characters,
/// splitting at spaces when possible.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Get a Unix timestamp (seconds since epoch), cross-platform.
fn platform_timestamp() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
}

/// Truncate a name to max_len characters, appending ".." if truncated.
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}..", &name[..max_len.saturating_sub(2)])
    }
}
