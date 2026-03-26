#[derive(Debug, Clone)]
pub struct DialogueLine {
    pub speaker: Speaker,
    pub text: String,
    pub typing_speed: f64, // chars per second
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Speaker {
    Control, // CONTROL - handler, cyan colored
    Pilot,   // Player character (rare)
    Unknown, // Fragmented transmissions, dim red
    Signal,  // From inside event horizon, flickering white
}

impl Speaker {
    pub fn name(&self) -> &str {
        match self {
            Speaker::Control => "CONTROL",
            Speaker::Pilot => "PILOT",
            Speaker::Unknown => "UNKNOWN",
            Speaker::Signal => "SIGNAL",
        }
    }

    pub fn color(&self) -> [f32; 4] {
        match self {
            Speaker::Control => [0.0, 0.85, 0.9, 1.0],   // cyan
            Speaker::Pilot => [0.8, 0.8, 0.7, 1.0],       // warm off-white
            Speaker::Unknown => [0.6, 0.15, 0.1, 0.85],    // dim red
            Speaker::Signal => [0.95, 0.95, 0.95, 0.7],    // flickering white
        }
    }
}

impl DialogueLine {
    pub fn control(text: &str) -> Self {
        Self {
            speaker: Speaker::Control,
            text: text.to_string(),
            typing_speed: 40.0,
        }
    }

    pub fn pilot(text: &str) -> Self {
        Self {
            speaker: Speaker::Pilot,
            text: text.to_string(),
            typing_speed: 40.0,
        }
    }

    pub fn unknown(text: &str) -> Self {
        Self {
            speaker: Speaker::Unknown,
            text: text.to_string(),
            typing_speed: 25.0, // slower, more unsettling
        }
    }

    pub fn signal(text: &str) -> Self {
        Self {
            speaker: Speaker::Signal,
            text: text.to_string(),
            typing_speed: 18.0, // slowest, deliberate
        }
    }
}
