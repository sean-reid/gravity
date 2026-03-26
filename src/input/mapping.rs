use std::collections::HashMap;
use winit::keyboard::KeyCode;

/// Simplified action identifiers for key binding (no associated data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoundAction {
    ThrustPrograde,
    ThrustRetrograde,
    ThrustRadialIn,
    ThrustRadialOut,
    Fire,
    SelectWeapon1,
    SelectWeapon2,
    SelectWeapon3,
    SelectWeapon4,
    SelectWeapon5,
    SelectWeapon6,
    ActivateOrbitAnchor,
    ActivateTidalFlare,
    ZoomIn,
    ZoomOut,
    Pause,
    Confirm,
}

pub struct KeyMapping {
    /// Maps a bound action to its key.
    pub bindings: HashMap<BoundAction, KeyCode>,
    /// Reverse map: key -> action.
    reverse: HashMap<KeyCode, BoundAction>,
}

impl KeyMapping {
    pub fn new() -> Self {
        let mut mapping = Self {
            bindings: HashMap::new(),
            reverse: HashMap::new(),
        };

        // Default bindings
        mapping.set(BoundAction::ThrustPrograde, KeyCode::KeyW);
        mapping.set(BoundAction::ThrustRetrograde, KeyCode::KeyS);
        mapping.set(BoundAction::ThrustRadialIn, KeyCode::KeyA);
        mapping.set(BoundAction::ThrustRadialOut, KeyCode::KeyD);
        mapping.set(BoundAction::SelectWeapon1, KeyCode::Digit1);
        mapping.set(BoundAction::SelectWeapon2, KeyCode::Digit2);
        mapping.set(BoundAction::SelectWeapon3, KeyCode::Digit3);
        mapping.set(BoundAction::SelectWeapon4, KeyCode::Digit4);
        mapping.set(BoundAction::SelectWeapon5, KeyCode::Digit5);
        mapping.set(BoundAction::SelectWeapon6, KeyCode::Digit6);
        mapping.set(BoundAction::ActivateOrbitAnchor, KeyCode::Space);
        mapping.set(BoundAction::ActivateTidalFlare, KeyCode::KeyQ);
        mapping.set(BoundAction::Pause, KeyCode::Escape);
        mapping.set(BoundAction::Confirm, KeyCode::Enter);

        mapping
    }

    /// Bind a key to an action, replacing any previous binding for that action.
    pub fn set(&mut self, action: BoundAction, key: KeyCode) {
        // Remove old key for this action if it existed.
        if let Some(old_key) = self.bindings.insert(action, key) {
            self.reverse.remove(&old_key);
        }
        // Remove any other action that was on this key.
        if let Some(old_action) = self.reverse.insert(key, action) {
            if old_action != action {
                self.bindings.remove(&old_action);
            }
        }
    }

    /// Look up which action (if any) is bound to the given key.
    pub fn get_action_for_key(&self, key: KeyCode) -> Option<BoundAction> {
        self.reverse.get(&key).copied()
    }

    /// Look up which key (if any) is bound to the given action.
    pub fn get_key_for_action(&self, action: BoundAction) -> Option<KeyCode> {
        self.bindings.get(&action).copied()
    }
}

impl Default for KeyMapping {
    fn default() -> Self {
        Self::new()
    }
}
