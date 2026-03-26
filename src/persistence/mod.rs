pub mod native_save;
pub mod web_save;

use std::collections::HashMap;

/// Trait for loading and saving game state.
pub trait SaveData {
    fn load(&self) -> Option<SaveState>;
    fn save(&self, state: &SaveState);
    fn clear(&self);
}

/// Complete serializable game save state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveState {
    pub highest_level: u32,
    pub base_seed: u64,
    pub unlocked_weapons: Vec<String>,
    pub unlocked_orbit_anchor: bool,
    pub unlocked_tidal_flare: bool,
    pub story_flags: HashMap<String, bool>,
    pub settings: GameSettings,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub online_player_id: String,
}

/// Player-configurable game settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameSettings {
    pub master_volume: f64,
    pub mouse_sensitivity: f64,
    pub bloom_enabled: bool,
    pub post_process_scale: f64,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            mouse_sensitivity: 1.0,
            bloom_enabled: true,
            post_process_scale: 1.0,
        }
    }
}

impl Default for SaveState {
    fn default() -> Self {
        Self {
            highest_level: 0,
            base_seed: 0,
            unlocked_weapons: vec!["Railgun".to_string()],
            unlocked_orbit_anchor: false,
            unlocked_tidal_flare: false,
            story_flags: HashMap::new(),
            settings: GameSettings::default(),
            display_name: String::new(),
            online_player_id: String::new(),
        }
    }
}
