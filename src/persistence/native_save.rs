#[cfg(feature = "native")]
use super::{SaveData, SaveState};

#[cfg(feature = "native")]
use std::path::PathBuf;

/// Native (desktop) file-based save system.
/// Saves to `{data_dir}/gravity-well-arena/save.json` using atomic writes.
#[cfg(feature = "native")]
pub struct NativeSave {
    save_path: PathBuf,
}

#[cfg(feature = "native")]
impl NativeSave {
    /// Create a new NativeSave. The save directory is created if it does not exist.
    pub fn new() -> Self {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let save_dir = base.join("gravity-well-arena");
        let _ = std::fs::create_dir_all(&save_dir);
        Self {
            save_path: save_dir.join("save.json"),
        }
    }

    /// Return the path where save data is stored.
    pub fn save_path(&self) -> &PathBuf {
        &self.save_path
    }

    /// Atomic write: serialize to a temp file, then rename over the target.
    fn atomic_write(&self, data: &[u8]) -> std::io::Result<()> {
        let temp_path = self.save_path.with_extension("json.tmp");
        std::fs::write(&temp_path, data)?;
        std::fs::rename(&temp_path, &self.save_path)?;
        Ok(())
    }
}

#[cfg(feature = "native")]
impl Default for NativeSave {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "native")]
impl SaveData for NativeSave {
    fn load(&self) -> Option<SaveState> {
        let data = std::fs::read_to_string(&self.save_path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save(&self, state: &SaveState) {
        match serde_json::to_string_pretty(state) {
            Ok(json) => {
                if let Err(e) = self.atomic_write(json.as_bytes()) {
                    log::error!("Failed to save game state: {}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize game state: {}", e);
            }
        }
    }

    fn clear(&self) {
        let _ = std::fs::remove_file(&self.save_path);
    }
}
