#[cfg(feature = "web")]
use super::{SaveData, SaveState};

/// localStorage key for save data.
#[cfg(feature = "web")]
const STORAGE_KEY: &str = "gwa_save";

/// Web (WASM) localStorage-based save system.
#[cfg(feature = "web")]
pub struct WebSave;

#[cfg(feature = "web")]
impl WebSave {
    pub fn new() -> Self {
        Self
    }

    fn local_storage(&self) -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }
}

#[cfg(feature = "web")]
impl Default for WebSave {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "web")]
impl SaveData for WebSave {
    fn load(&self) -> Option<SaveState> {
        let storage = self.local_storage()?;
        let json = storage.get_item(STORAGE_KEY).ok()??;
        serde_json::from_str(&json).ok()
    }

    fn save(&self, state: &SaveState) {
        let storage = match self.local_storage() {
            Some(s) => s,
            None => {
                log::error!("localStorage not available");
                return;
            }
        };
        match serde_json::to_string(state) {
            Ok(json) => {
                if let Err(e) = storage.set_item(STORAGE_KEY, &json) {
                    log::error!("Failed to save to localStorage: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize save state: {}", e);
            }
        }
    }

    fn clear(&self) {
        if let Some(storage) = self.local_storage() {
            let _ = storage.remove_item(STORAGE_KEY);
        }
    }
}
