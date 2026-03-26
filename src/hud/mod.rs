pub mod clocks;
pub mod tempo;
pub mod gauges;
pub mod trajectory;
pub mod radar;
pub mod warnings;

use crate::entities::bot::BotArchetype;
use crate::rendering::hud_render::HudElement;
use crate::util::Color;

/// Snapshot of all data needed to render the HUD for a single frame.
///
/// Built from game state each tick and passed to `build_hud` to produce a
/// flat list of `HudElement` primitives for the renderer.
pub struct HudState {
    /// Coordinate (world) time in seconds.
    pub coordinate_time: f64,
    /// Player proper time in seconds.
    pub proper_time: f64,
    /// World tempo: inverse of player time-dilation factor (1/tau).
    pub world_tempo: f64,
    /// Player health (0 - 100).
    pub health: f64,
    /// Player shields (0 - 100).
    pub shields: f64,
    /// Player fuel (0 - 100).
    pub fuel: f64,
    /// Index of the currently active weapon slot (0-5).
    pub active_weapon: usize,
    /// Per-slot weapon cooldown remaining, normalized to [0, 1].
    pub weapon_cooldowns: [f64; 6],
    /// Per-slot weapon availability (true if unlocked).
    pub weapons_available: [bool; 6],
    /// Living bots with their archetype and relative tau (tau_bot / tau_player).
    pub bot_depths: Vec<(BotArchetype, f64)>,
    /// Whether the player is at or above escape velocity.
    pub escape_velocity_warning: bool,
    /// Whether player fuel is critically low.
    pub low_fuel_warning: bool,
    /// Color for the trajectory preview, based on orbital safety.
    pub trajectory_color: Color,
    /// Viewport width in physical pixels.
    pub viewport_width: f32,
    /// Viewport height in physical pixels.
    pub viewport_height: f32,
    /// DPI scale factor (e.g. 2.0 on Retina). All HUD sizes are multiplied by this.
    pub dpi_scale: f32,
}

/// Assemble all HUD sub-elements into a single flat list of `HudElement` primitives.
///
/// Each sub-module contributes its portion of the HUD:
/// - Clocks (top-left)
/// - Tempo indicator (top-right)
/// - Health / Shields / Fuel gauges (bottom-left)
/// - Weapon hotbar and name (bottom-center)
/// - Bot depth radar (bottom-right)
/// - Warning overlays (escape velocity, low fuel)
pub fn build_hud(state: &HudState) -> Vec<HudElement> {
    let vw = state.viewport_width;
    let vh = state.viewport_height;
    let s = state.dpi_scale;

    let mut elements = Vec::with_capacity(64);

    elements.extend(clocks::build_clock_elements(
        state.coordinate_time,
        state.proper_time,
        vw, vh, s,
    ));

    elements.extend(tempo::build_tempo_elements(state.world_tempo, vw, vh, s));

    elements.extend(gauges::build_gauge_elements(
        state.health, state.shields, state.fuel,
        vw, vh, s,
    ));

    elements.extend(warnings::build_weapon_elements(
        state.active_weapon,
        &state.weapon_cooldowns,
        &state.weapons_available,
        vw, vh, s,
    ));

    elements.extend(radar::build_radar_elements(&state.bot_depths, vw, vh, s));

    elements.extend(warnings::build_warning_elements(
        state.escape_velocity_warning,
        state.low_fuel_warning,
        state.coordinate_time,
        vw, vh, s,
    ));

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_state() -> HudState {
        HudState {
            coordinate_time: 42.0,
            proper_time: 38.5,
            world_tempo: 1.0,
            health: 80.0,
            shields: 60.0,
            fuel: 45.0,
            active_weapon: 0,
            weapon_cooldowns: [0.0; 6],
            weapons_available: [true, true, false, false, false, false],
            bot_depths: vec![
                (BotArchetype::Skirmisher, 0.8),
                (BotArchetype::Diver, 1.2),
            ],
            escape_velocity_warning: false,
            low_fuel_warning: false,
            trajectory_color: Color::GREEN,
            viewport_width: 1280.0,
            viewport_height: 720.0,
            dpi_scale: 1.0,
        }
    }

    #[test]
    fn test_build_hud_produces_elements() {
        let state = default_state();
        let elements = build_hud(&state);
        // Should produce a non-trivial number of elements
        assert!(elements.len() > 10, "Expected >10 elements, got {}", elements.len());
    }

    #[test]
    fn test_build_hud_no_bots() {
        let mut state = default_state();
        state.bot_depths.clear();
        let elements = build_hud(&state);
        // Should still work without bots
        assert!(!elements.is_empty());
    }

    #[test]
    fn test_build_hud_with_warnings() {
        let mut state = default_state();
        state.escape_velocity_warning = true;
        state.low_fuel_warning = true;
        let elements = build_hud(&state);
        // Should have more elements than without warnings
        let base_count = build_hud(&default_state()).len();
        assert!(elements.len() > base_count);
    }

    #[test]
    fn test_build_hud_all_weapons_available() {
        let mut state = default_state();
        state.weapons_available = [true; 6];
        state.active_weapon = 3;
        let elements = build_hud(&state);
        assert!(!elements.is_empty());
    }
}
