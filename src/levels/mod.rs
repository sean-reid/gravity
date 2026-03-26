pub mod config;
pub mod difficulty;
pub mod generator;
pub mod progression;

pub use config::{LevelConfig, BlackHoleConfig, BotSpawn};
pub use difficulty::{difficulty, max_dilation_for_level, bot_count_for_level};
pub use generator::generate_level;
pub use progression::Progression;
