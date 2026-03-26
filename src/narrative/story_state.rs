use std::collections::HashMap;

pub struct StoryState {
    pub flags: HashMap<String, bool>,
    pub current_act: u32,
}

impl StoryState {
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
            current_act: 1,
        }
    }

    pub fn set_flag(&mut self, id: &str) {
        self.flags.insert(id.to_string(), true);
    }

    pub fn has_flag(&self, id: &str) -> bool {
        self.flags.get(id).copied().unwrap_or(false)
    }

    /// Returns the act number for a given level.
    /// Act 1: levels 1-10
    /// Act 2: levels 11-20
    /// Act 3: levels 21-35
    /// Act 4: levels 36+
    pub fn get_act_for_level(level: u32) -> u32 {
        match level {
            1..=10 => 1,
            11..=20 => 2,
            21..=35 => 3,
            _ => 4,
        }
    }
}
