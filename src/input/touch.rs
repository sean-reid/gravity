use std::collections::HashMap;

use crate::camera::Camera;
use crate::util::Vec2;

use super::{InputAction, InputProvider};

/// Touch point state tracked by finger id.
#[derive(Debug, Clone, Copy)]
struct TouchPoint {
    position: (f32, f32),
}

/// Which zone a touch belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TouchZone {
    Joystick,
    AimFire,
}

pub struct TouchInput {
    /// Active touch points indexed by finger id.
    touches: HashMap<u64, TouchPoint>,
    /// Assignment of each finger to a zone.
    zone_assignment: HashMap<u64, TouchZone>,
    /// Joystick center (set on touch start in left zone).
    joystick_origin: Option<(f32, f32)>,
    /// Finger id currently driving the joystick.
    joystick_finger: Option<u64>,
    /// Screen dimensions (needed to determine zones).
    screen_width: f32,
    screen_height: f32,
    /// Previous distance between two fingers for pinch zoom.
    pinch_base_distance: Option<f32>,
    /// Accumulated zoom factor from pinch (positive = zoom in, negative = zoom out).
    pinch_zoom_accum: f64,
}

impl TouchInput {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            touches: HashMap::new(),
            zone_assignment: HashMap::new(),
            joystick_origin: None,
            joystick_finger: None,
            screen_width,
            screen_height,
            pinch_base_distance: None,
            pinch_zoom_accum: 0.0,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// Determine which zone a screen position falls in.
    fn classify_zone(&self, x: f32, y: f32) -> TouchZone {
        let half_w = self.screen_width / 2.0;
        let half_h = self.screen_height / 2.0;
        // Left side, bottom half = joystick zone
        if x < half_w && y > half_h {
            TouchZone::Joystick
        } else {
            TouchZone::AimFire
        }
    }

    /// Compute joystick deflection as a normalized Vec2 (magnitude clamped to 1.0).
    fn joystick_deflection(&self) -> Option<Vec2> {
        let finger_id = self.joystick_finger?;
        let origin = self.joystick_origin?;
        let tp = self.touches.get(&finger_id)?;

        let dx = (tp.position.0 - origin.0) as f64;
        let dy = (tp.position.1 - origin.1) as f64;

        // Joystick radius in screen pixels
        let radius = (self.screen_width.min(self.screen_height) * 0.12) as f64;
        if radius < 1.0 {
            return None;
        }

        let deflection = Vec2::new(dx / radius, dy / radius);
        let len = deflection.length();
        if len > 1.0 {
            Some(deflection / len)
        } else if len < 0.15 {
            // Dead zone
            None
        } else {
            Some(deflection)
        }
    }

    /// Update pinch zoom state based on current touches.
    fn update_pinch(&mut self) {
        if self.touches.len() == 2 {
            let mut iter = self.touches.values();
            let a = iter.next().unwrap();
            let b = iter.next().unwrap();
            let dx = a.position.0 - b.position.0;
            let dy = a.position.1 - b.position.1;
            let dist = (dx * dx + dy * dy).sqrt();

            if let Some(base) = self.pinch_base_distance {
                if base > 1.0 {
                    let ratio = dist / base;
                    if ratio > 1.15 {
                        self.pinch_zoom_accum += 1.0;
                        self.pinch_base_distance = Some(dist);
                    } else if ratio < 0.87 {
                        self.pinch_zoom_accum -= 1.0;
                        self.pinch_base_distance = Some(dist);
                    }
                }
            } else {
                self.pinch_base_distance = Some(dist);
            }
        } else {
            self.pinch_base_distance = None;
        }
    }

    /// Reset per-frame accumulators. Call after poll() each frame.
    pub fn consume_frame(&mut self) {
        self.pinch_zoom_accum = 0.0;
    }
}

impl InputProvider for TouchInput {
    fn handle_window_event(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::Touch(touch) => {
                let id = touch.id;
                let pos = (touch.location.x as f32, touch.location.y as f32);

                match touch.phase {
                    winit::event::TouchPhase::Started => {
                        let tp = TouchPoint { position: pos };
                        self.touches.insert(id, tp);

                        let zone = self.classify_zone(pos.0, pos.1);
                        self.zone_assignment.insert(id, zone);

                        if zone == TouchZone::Joystick && self.joystick_finger.is_none() {
                            self.joystick_finger = Some(id);
                            self.joystick_origin = Some(pos);
                        }
                    }
                    winit::event::TouchPhase::Moved => {
                        if let Some(tp) = self.touches.get_mut(&id) {
                            tp.position = pos;
                        }
                        self.update_pinch();
                    }
                    winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                        self.touches.remove(&id);
                        self.zone_assignment.remove(&id);

                        if self.joystick_finger == Some(id) {
                            self.joystick_finger = None;
                            self.joystick_origin = None;
                        }

                        if self.touches.len() < 2 {
                            self.pinch_base_distance = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn poll(&mut self, camera: &Camera) -> Vec<InputAction> {
        let mut actions = Vec::new();

        // Joystick -> thrust actions
        if let Some(deflection) = self.joystick_deflection() {
            // In screen space: up is negative Y, but we map up = prograde.
            // Joystick: up (negative dy) = prograde, down (positive dy) = retrograde,
            //           left (negative dx) = radial in, right (positive dx) = radial out.
            let threshold = 0.3;

            if deflection.y < -threshold {
                actions.push(InputAction::ThrustPrograde);
            }
            if deflection.y > threshold {
                actions.push(InputAction::ThrustRetrograde);
            }
            if deflection.x < -threshold {
                actions.push(InputAction::ThrustRadialIn);
            }
            if deflection.x > threshold {
                actions.push(InputAction::ThrustRadialOut);
            }
        }

        // Right-side touch -> aim and fire
        for (&id, &zone) in &self.zone_assignment {
            if zone == TouchZone::AimFire {
                if let Some(tp) = self.touches.get(&id) {
                    let world_pos = camera.screen_to_world(tp.position.0, tp.position.1);
                    actions.push(InputAction::AimAt(world_pos));
                    actions.push(InputAction::Fire);
                    break; // Only one aim target at a time
                }
            }
        }

        // Pinch zoom
        if self.pinch_zoom_accum > 0.5 {
            actions.push(InputAction::ZoomIn);
        } else if self.pinch_zoom_accum < -0.5 {
            actions.push(InputAction::ZoomOut);
        }

        actions
    }

    fn needs_virtual_controls(&self) -> bool {
        true
    }
}
