use super::synth::{Envelope, LowPassFilter, Oscillator, Waveform};

// ---------------------------------------------------------------------------
// AmbientDrone
// ---------------------------------------------------------------------------

/// Continuous low-frequency drone whose pitch and timbre shift with gravity
/// well depth. At the rim the tone is 60 Hz and relatively bright; near the
/// abyss it drops to 20 Hz and becomes heavily filtered.
pub struct AmbientDrone {
    oscillator: Oscillator,
    sub_oscillator: Oscillator,
    filter: LowPassFilter,
    depth_factor: f64, // 0 = rim, 1 = abyss
    volume: f32,
}

impl AmbientDrone {
    pub fn new() -> Self {
        let sr = 44100.0;
        Self {
            oscillator: Oscillator::new(60.0, Waveform::Sawtooth),
            sub_oscillator: Oscillator::new(30.0, Waveform::Sine),
            filter: LowPassFilter::new(200.0, 0.7, sr),
            depth_factor: 0.0,
            volume: 0.15,
        }
    }

    /// Set the depth factor. 0 = rim (safe), 1 = abyss (dangerous).
    pub fn set_depth(&mut self, depth_factor: f64) {
        self.depth_factor = depth_factor.clamp(0.0, 1.0);

        // Frequency: 60 Hz at rim, 20 Hz at abyss
        let freq = 60.0 - 40.0 * self.depth_factor;
        self.oscillator.frequency = freq;
        self.sub_oscillator.frequency = freq * 0.5;

        // Filter cutoff narrows as we go deeper: 200 Hz -> 60 Hz
        let cutoff = 200.0 - 140.0 * self.depth_factor;
        self.filter.set_cutoff(cutoff);

        // Volume increases slightly with depth for menace
        self.volume = (0.10 + 0.15 * self.depth_factor as f32).min(0.25);
    }

    pub fn sample(&mut self, sample_rate: f64) -> f32 {
        let main = self.oscillator.sample(sample_rate);
        let sub = self.sub_oscillator.sample(sample_rate);
        let mixed = main * 0.6 + sub * 0.4;
        self.filter.process(mixed) * self.volume
    }
}

impl Default for AmbientDrone {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// HeartbeatGenerator
// ---------------------------------------------------------------------------

/// Rhythmic heartbeat sound that activates when the player is deep in the
/// gravity well. Rate (BPM) increases as the player nears the event horizon.
pub struct HeartbeatGenerator {
    active: bool,
    bpm: f64,
    phase: f64, // 0..1 within one beat cycle
    oscillator: Oscillator,
    envelope: Envelope,
    beat_triggered: bool,
}

impl HeartbeatGenerator {
    pub fn new() -> Self {
        Self {
            active: false,
            bpm: 0.0,
            phase: 0.0,
            oscillator: Oscillator::new(45.0, Waveform::Sine),
            envelope: Envelope::decay_only(0.12),
            beat_triggered: false,
        }
    }

    /// Set the heartbeat rate. 0 = deactivate.
    pub fn set_bpm(&mut self, bpm: f64) {
        if bpm <= 0.0 {
            self.active = false;
            self.bpm = 0.0;
        } else {
            self.active = true;
            self.bpm = bpm.clamp(20.0, 240.0);
        }
    }

    pub fn sample(&mut self, sample_rate: f64) -> f32 {
        if !self.active {
            return 0.0;
        }

        let dt = 1.0 / sample_rate;
        let beats_per_sec = self.bpm / 60.0;
        let prev_phase = self.phase;

        self.phase += beats_per_sec * dt;

        // Detect beat boundary (phase wraps past 1.0)
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Trigger on the downbeat (phase crosses 0) or on the "lub-dub"
        // second hit at phase ~0.15
        let trigger_first = prev_phase > self.phase; // wrapped
        let trigger_second = prev_phase < 0.15 && self.phase >= 0.15;

        if trigger_first || trigger_second {
            self.envelope = Envelope::decay_only(0.08);
            self.envelope.trigger();
            self.beat_triggered = true;
        }

        let env = self.envelope.process(dt);
        let osc = self.oscillator.sample(sample_rate);

        osc * env * 0.5
    }
}

impl Default for HeartbeatGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Pitch shifting concept
// ---------------------------------------------------------------------------

/// Compute the relativistic pitch shift factor for a sound emitted by `source`
/// as perceived by `player`.
///
/// `tau_source` is the proper-time rate of the emitter (in the gravity well).
/// `tau_player` is the proper-time rate of the receiver.
///
/// A source deeper in the well (lower tau) sounds *lower-pitched* to an
/// observer further out, and vice versa.
pub fn gravitational_pitch_factor(tau_source: f64, tau_player: f64) -> f64 {
    if tau_player <= 0.0 {
        return 1.0;
    }
    (tau_source / tau_player).clamp(0.1, 10.0)
}
