pub mod util;
pub mod physics;
pub mod entities;
pub mod weapons;
pub mod ai;
pub mod camera;
pub mod input;
pub mod audio;
pub mod narrative;
pub mod levels;
pub mod persistence;
pub mod leaderboard;
pub mod rendering;
pub mod hud;
pub mod game;
pub mod platform;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    platform::web::run_web();
}
