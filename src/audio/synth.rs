use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Waveform
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Sawtooth,
    Noise,
}

// ---------------------------------------------------------------------------
// Oscillator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Oscillator {
    pub phase: f64,
    pub frequency: f64,
    pub waveform: Waveform,
    noise_state: u32, // simple xorshift state for noise
}

impl Oscillator {
    pub fn new(frequency: f64, waveform: Waveform) -> Self {
        Self {
            phase: 0.0,
            frequency,
            waveform,
            noise_state: 0xDEAD_BEEF,
        }
    }

    /// Advance the oscillator by one sample and return the value in [-1, 1].
    pub fn sample(&mut self, sample_rate: f64) -> f32 {
        let value = match self.waveform {
            Waveform::Sine => (self.phase * 2.0 * PI).sin(),
            Waveform::Triangle => {
                let t = self.phase;
                if t < 0.25 {
                    4.0 * t
                } else if t < 0.75 {
                    2.0 - 4.0 * t
                } else {
                    -4.0 + 4.0 * t
                }
            }
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Sawtooth => 2.0 * self.phase - 1.0,
            Waveform::Noise => {
                // Xorshift32
                let mut s = self.noise_state;
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                self.noise_state = s;
                // Map to [-1, 1]
                (s as f64 / u32::MAX as f64) * 2.0 - 1.0
            }
        };

        // Advance phase
        if self.waveform != Waveform::Noise {
            self.phase += self.frequency / sample_rate;
            self.phase -= self.phase.floor();
        }

        value as f32
    }
}

// ---------------------------------------------------------------------------
// White Noise Generator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WhiteNoise {
    state: u32,
}

impl WhiteNoise {
    pub fn new() -> Self {
        Self { state: 0xCAFE_BABE }
    }

    pub fn sample(&mut self) -> f32 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        self.state = s;
        (s as f64 / u32::MAX as f64) as f32 * 2.0 - 1.0
    }
}

impl Default for WhiteNoise {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ADSR Envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
    Finished,
}

#[derive(Debug, Clone)]
pub struct Envelope {
    pub attack: f64,
    pub decay: f64,
    pub sustain_level: f64,
    pub release: f64,
    pub state: EnvelopeState,
    pub time: f64,
    pub gain: f64,
    release_start_gain: f64,
}

impl Envelope {
    pub fn new(attack: f64, decay: f64, sustain_level: f64, release: f64) -> Self {
        Self {
            attack,
            decay,
            sustain_level,
            release,
            state: EnvelopeState::Attack,
            time: 0.0,
            gain: 0.0,
            release_start_gain: 0.0,
        }
    }

    /// Create an envelope that is just a simple decay from 1.0 to 0.0.
    pub fn decay_only(duration: f64) -> Self {
        Self::new(0.001, duration, 0.0, 0.001)
    }

    pub fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
        self.time = 0.0;
        self.gain = 0.0;
    }

    pub fn release_note(&mut self) {
        if self.state != EnvelopeState::Finished && self.state != EnvelopeState::Idle {
            self.release_start_gain = self.gain;
            self.state = EnvelopeState::Release;
            self.time = 0.0;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Finished
    }

    /// Advance the envelope by `dt` seconds and return the current gain [0, 1].
    pub fn process(&mut self, dt: f64) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.gain = 0.0;
            }
            EnvelopeState::Attack => {
                self.time += dt;
                if self.attack <= 0.0 || self.time >= self.attack {
                    self.gain = 1.0;
                    self.state = EnvelopeState::Decay;
                    self.time = 0.0;
                } else {
                    self.gain = self.time / self.attack;
                }
            }
            EnvelopeState::Decay => {
                self.time += dt;
                if self.decay <= 0.0 || self.time >= self.decay {
                    self.gain = self.sustain_level;
                    if self.sustain_level > 0.0 {
                        self.state = EnvelopeState::Sustain;
                    } else {
                        self.state = EnvelopeState::Finished;
                    }
                    self.time = 0.0;
                } else {
                    let t = self.time / self.decay;
                    self.gain = 1.0 - t * (1.0 - self.sustain_level);
                }
            }
            EnvelopeState::Sustain => {
                self.gain = self.sustain_level;
            }
            EnvelopeState::Release => {
                self.time += dt;
                if self.release <= 0.0 || self.time >= self.release {
                    self.gain = 0.0;
                    self.state = EnvelopeState::Finished;
                } else {
                    let t = self.time / self.release;
                    self.gain = self.release_start_gain * (1.0 - t);
                }
            }
            EnvelopeState::Finished => {
                self.gain = 0.0;
            }
        }
        self.gain as f32
    }
}

// ---------------------------------------------------------------------------
// Biquad Low-Pass Filter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LowPassFilter {
    pub cutoff: f64,
    pub q: f64,
    // Filter state (Direct Form I)
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    sample_rate: f64,
}

impl LowPassFilter {
    pub fn new(cutoff: f64, q: f64, sample_rate: f64) -> Self {
        let mut f = Self {
            cutoff,
            q,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            sample_rate,
        };
        f.compute_coefficients();
        f
    }

    pub fn set_cutoff(&mut self, cutoff: f64) {
        self.cutoff = cutoff;
        self.compute_coefficients();
    }

    fn compute_coefficients(&mut self) {
        let w0 = 2.0 * PI * self.cutoff / self.sample_rate;
        let alpha = w0.sin() / (2.0 * self.q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let x0 = sample as f64;
        let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = y0;

        y0 as f32
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Mix multiple samples together (sum and clamp).
pub fn mix(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().sum();
    sum.clamp(-1.0, 1.0)
}
