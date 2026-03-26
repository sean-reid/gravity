/// A single frame of recorded input for replay determinism.
#[derive(Debug, Clone)]
pub struct InputFrame {
    /// The simulation frame number this input was recorded at.
    pub frame_number: u64,
    /// Encoded player actions (bitfield: thrust, fire, switch weapon, etc.).
    pub actions: Vec<u8>,
    /// Aim/cursor X position in world coordinates.
    pub aim_x: f32,
    /// Aim/cursor Y position in world coordinates.
    pub aim_y: f32,
}

/// Complete replay data for a level run, sufficient to reproduce the game
/// deterministically given the same seed and level number.
#[derive(Debug, Clone)]
pub struct ReplayData {
    /// The seed used to generate the level.
    pub level_seed: u64,
    /// The level number played.
    pub level_number: u32,
    /// Ordered sequence of input frames.
    pub input_frames: Vec<InputFrame>,
}

/// Records input frames during gameplay for replay storage.
#[derive(Debug)]
pub struct ReplayRecorder {
    level_seed: u64,
    level_number: u32,
    frames: Vec<InputFrame>,
}

impl ReplayRecorder {
    /// Create a new recorder for the given level.
    pub fn new(level_seed: u64, level_number: u32) -> Self {
        Self {
            level_seed,
            level_number,
            frames: Vec::with_capacity(1024),
        }
    }

    /// Record a single frame of input.
    pub fn record_frame(&mut self, frame_number: u64, actions: Vec<u8>, aim_x: f32, aim_y: f32) {
        self.frames.push(InputFrame {
            frame_number,
            actions,
            aim_x,
            aim_y,
        });
    }

    /// Consume the recorder and return the completed replay data.
    pub fn finish(self) -> ReplayData {
        ReplayData {
            level_seed: self.level_seed,
            level_number: self.level_number,
            input_frames: self.frames,
        }
    }

    /// Number of frames recorded so far.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_finish() {
        let mut recorder = ReplayRecorder::new(42, 5);
        recorder.record_frame(0, vec![0x01], 1.0, 2.0);
        recorder.record_frame(1, vec![0x03], 1.5, 2.5);
        assert_eq!(recorder.frame_count(), 2);

        let replay = recorder.finish();
        assert_eq!(replay.level_seed, 42);
        assert_eq!(replay.level_number, 5);
        assert_eq!(replay.input_frames.len(), 2);
        assert_eq!(replay.input_frames[0].frame_number, 0);
        assert_eq!(replay.input_frames[1].aim_x, 1.5);
    }
}
