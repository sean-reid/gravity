pub mod keyboard_mouse;
pub mod touch;
pub mod mapping;

pub use keyboard_mouse::KeyboardMouseInput;
pub use touch::TouchInput;
pub use mapping::KeyMapping;

use crate::util::Vec2;

#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    ThrustPrograde,
    ThrustRetrograde,
    ThrustRadialIn,
    ThrustRadialOut,
    AimAt(Vec2),
    Fire,
    SelectWeapon(u8), // 1-6
    ActivateOrbitAnchor,
    ActivateTidalFlare,
    ZoomIn,
    ZoomOut,
    Pause,
    Confirm,    // for menus
    NewGame,        // reset progress (title screen only)
    ChangeCallsign, // re-enter callsign (title screen only)
}

pub trait InputProvider {
    fn handle_window_event(&mut self, event: &winit::event::WindowEvent);
    fn poll(&mut self, camera: &crate::camera::Camera) -> Vec<InputAction>;
    fn needs_virtual_controls(&self) -> bool;
}
