use std::collections::HashSet;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::camera::Camera;
use super::mapping::{BoundAction, KeyMapping};
use super::{InputAction, InputProvider};

pub struct KeyboardMouseInput {
    keys_held: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    mouse_position: (f32, f32),
    left_mouse_held: bool,
    scroll_accumulator: f64,
    mapping: KeyMapping,
}

impl KeyboardMouseInput {
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            mouse_position: (0.0, 0.0),
            left_mouse_held: false,
            scroll_accumulator: 0.0,
            mapping: KeyMapping::new(),
        }
    }

    pub fn with_mapping(mapping: KeyMapping) -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            mouse_position: (0.0, 0.0),
            left_mouse_held: false,
            scroll_accumulator: 0.0,
            mapping,
        }
    }

    pub fn mapping(&self) -> &KeyMapping {
        &self.mapping
    }

    pub fn mapping_mut(&mut self) -> &mut KeyMapping {
        &mut self.mapping
    }
}

impl Default for KeyboardMouseInput {
    fn default() -> Self {
        Self::new()
    }
}

impl InputProvider for KeyboardMouseInput {
    fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !self.keys_held.contains(&key_code) {
                                self.keys_just_pressed.insert(key_code);
                            }
                            self.keys_held.insert(key_code);
                        }
                        ElementState::Released => {
                            self.keys_held.remove(&key_code);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    self.left_mouse_held = *state == ElementState::Pressed;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y as f64,
                    MouseScrollDelta::PixelDelta(pos) => pos.y / 50.0,
                };
                self.scroll_accumulator += y;
            }
            _ => {}
        }
    }

    fn poll(&mut self, camera: &Camera) -> Vec<InputAction> {
        let mut actions = Vec::new();

        // Held keys: continuous actions (thrust, fire via hold)
        for &key in &self.keys_held {
            if let Some(bound) = self.mapping.get_action_for_key(key) {
                match bound {
                    BoundAction::ThrustPrograde => actions.push(InputAction::ThrustPrograde),
                    BoundAction::ThrustRetrograde => actions.push(InputAction::ThrustRetrograde),
                    BoundAction::ThrustRadialIn => actions.push(InputAction::ThrustRadialIn),
                    BoundAction::ThrustRadialOut => actions.push(InputAction::ThrustRadialOut),
                    BoundAction::Fire => actions.push(InputAction::Fire),
                    BoundAction::ActivateOrbitAnchor => {
                        actions.push(InputAction::ActivateOrbitAnchor)
                    }
                    BoundAction::ActivateTidalFlare => {
                        actions.push(InputAction::ActivateTidalFlare)
                    }
                    _ => {} // one-shot actions handled below via just_pressed
                }
            }
        }

        // Just-pressed keys: one-shot actions (pause, confirm, weapon select, menu)
        for &key in &self.keys_just_pressed {
            if let Some(bound) = self.mapping.get_action_for_key(key) {
                match bound {
                    BoundAction::Pause => actions.push(InputAction::Pause),
                    BoundAction::Confirm => actions.push(InputAction::Confirm),
                    BoundAction::SelectWeapon1 => actions.push(InputAction::SelectWeapon(1)),
                    BoundAction::SelectWeapon2 => actions.push(InputAction::SelectWeapon(2)),
                    BoundAction::SelectWeapon3 => actions.push(InputAction::SelectWeapon(3)),
                    BoundAction::SelectWeapon4 => actions.push(InputAction::SelectWeapon(4)),
                    BoundAction::SelectWeapon5 => actions.push(InputAction::SelectWeapon(5)),
                    BoundAction::SelectWeapon6 => actions.push(InputAction::SelectWeapon(6)),
                    _ => {}
                }
            }
            // Menu-only keys
            if key == KeyCode::KeyN {
                actions.push(InputAction::NewGame);
            }
            if key == KeyCode::KeyC {
                actions.push(InputAction::ChangeCallsign);
            }
        }
        self.keys_just_pressed.clear();

        // Mouse position -> AimAt
        let world_pos =
            camera.screen_to_world(self.mouse_position.0, self.mouse_position.1);
        actions.push(InputAction::AimAt(world_pos));

        // Left mouse button -> Fire
        if self.left_mouse_held {
            actions.push(InputAction::Fire);
        }

        // Scroll wheel -> ZoomIn / ZoomOut (emit one event per 0.15 accumulated)
        while self.scroll_accumulator > 0.15 {
            actions.push(InputAction::ZoomIn);
            self.scroll_accumulator -= 0.15;
        }
        while self.scroll_accumulator < -0.15 {
            actions.push(InputAction::ZoomOut);
            self.scroll_accumulator += 0.15;
        }
        // Keep small remainder for next frame (smooth accumulation)


        actions
    }

    fn needs_virtual_controls(&self) -> bool {
        false
    }
}

impl KeyboardMouseInput {
    /// Reset the scroll accumulator. Call after poll() each frame.
    pub fn consume_scroll(&mut self) {
        self.scroll_accumulator = 0.0;
    }

    /// Set mouse position directly (in CSS/logical pixels).
    /// Used on web to bypass winit's physical pixel coordinates.
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }
}
