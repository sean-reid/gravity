use super::mod_types::SoundEvent;
use super::synth::{Envelope, LowPassFilter, Oscillator, Waveform, WhiteNoise};

// ---------------------------------------------------------------------------
// SoundGenerator
// ---------------------------------------------------------------------------

/// A self-contained procedural sound that can be sampled until finished.
pub struct SoundGenerator {
    oscillators: Vec<Oscillator>,
    noise: Option<WhiteNoise>,
    envelope: Envelope,
    filter: Option<LowPassFilter>,
    duration: f64,
    time: f64,
    /// For sounds that sweep frequency over time.
    freq_start: f64,
    freq_end: f64,
    use_freq_sweep: bool,
    /// Overall volume scale for this sound.
    volume: f32,
    /// Whether this sound sustains until explicitly stopped.
    sustaining: bool,
    /// Set to true to begin release / stop.
    releasing: bool,
}

impl SoundGenerator {
    /// Advance by one sample and return the value in approximately [-1, 1].
    pub fn sample(&mut self, sample_rate: f64) -> f32 {
        if self.is_finished() {
            return 0.0;
        }

        let dt = 1.0 / sample_rate;
        self.time += dt;

        // Frequency sweep
        if self.use_freq_sweep && !self.oscillators.is_empty() {
            let t = (self.time / self.duration).min(1.0);
            let freq = self.freq_start + (self.freq_end - self.freq_start) * t;
            for osc in &mut self.oscillators {
                osc.frequency = freq;
            }
        }

        // If non-sustaining and past duration, start release
        if !self.sustaining && self.time >= self.duration && !self.releasing {
            self.releasing = true;
            self.envelope.release_note();
        }

        // Envelope
        let env_gain = self.envelope.process(dt);

        // Mix oscillators
        let mut out = 0.0f32;
        for osc in &mut self.oscillators {
            out += osc.sample(sample_rate);
        }

        // Add noise if present
        if let Some(ref mut noise) = self.noise {
            out += noise.sample();
        }

        // Normalize by source count
        let source_count = self.oscillators.len() + if self.noise.is_some() { 1 } else { 0 };
        if source_count > 1 {
            out /= source_count as f32;
        }

        // Filter
        if let Some(ref mut filter) = self.filter {
            out = filter.process(out);
        }

        out * env_gain * self.volume
    }

    pub fn is_finished(&self) -> bool {
        self.envelope.is_finished()
    }

    /// For sustaining sounds, call this to begin the release phase.
    pub fn stop(&mut self) {
        self.releasing = true;
        self.envelope.release_note();
    }

    pub fn is_sustaining(&self) -> bool {
        self.sustaining
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create a procedural sound generator for the given event.
pub fn create_sound(event: &SoundEvent) -> SoundGenerator {
    // Default sample rate for filter init. The actual sample rate is passed
    // into `sample()`. We pick 44100 as a reasonable default for coefficient
    // calculation; for native audio the real rate may differ slightly but the
    // perceptual difference is negligible.
    let sr = 44100.0;

    match event {
        // ---- Weapons -------------------------------------------------------
        SoundEvent::RailgunFire => {
            // Short burst of filtered noise (softer snap)
            SoundGenerator {
                oscillators: vec![],
                noise: Some(WhiteNoise::new()),
                envelope: Envelope::new(0.001, 0.09, 0.0, 0.01),
                filter: Some(LowPassFilter::new(4000.0, 1.0, sr)),
                duration: 0.1,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.3,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::MassDriverFire => {
            // Low-frequency thump (50Hz sine burst) + rumble tail
            SoundGenerator {
                oscillators: vec![
                    Oscillator::new(50.0, Waveform::Sine),
                    Oscillator::new(35.0, Waveform::Sine),
                ],
                noise: Some(WhiteNoise::new()),
                envelope: Envelope::new(0.005, 0.25, 0.0, 0.05),
                filter: Some(LowPassFilter::new(200.0, 0.7, sr)),
                duration: 0.3,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.55,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::PhotonLanceStart => {
            // Rising sine tone that sustains
            SoundGenerator {
                oscillators: vec![
                    Oscillator::new(800.0, Waveform::Sine),
                    Oscillator::new(803.0, Waveform::Sine), // slight detune for shimmer
                ],
                noise: None,
                envelope: Envelope::new(0.15, 0.1, 0.7, 0.3),
                filter: None,
                duration: 10.0, // long – sustains until PhotonLanceStop
                time: 0.0,
                freq_start: 400.0,
                freq_end: 1200.0,
                use_freq_sweep: true,
                volume: 0.35,
                sustaining: true,
                releasing: false,
            }
        }

        SoundEvent::PhotonLanceStop => {
            // Silence marker – the mixer handles stopping the lance sound.
            // Return a very short silent generator.
            SoundGenerator {
                oscillators: vec![],
                noise: None,
                envelope: Envelope::decay_only(0.01),
                filter: None,
                duration: 0.01,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.0,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::GravityBombDeploy => {
            // Descending pitch sweep 400Hz -> 80Hz, 0.5s
            SoundGenerator {
                oscillators: vec![Oscillator::new(400.0, Waveform::Sine)],
                noise: None,
                envelope: Envelope::new(0.01, 0.4, 0.0, 0.1),
                filter: Some(LowPassFilter::new(600.0, 1.2, sr)),
                duration: 0.5,
                time: 0.0,
                freq_start: 400.0,
                freq_end: 80.0,
                use_freq_sweep: true,
                volume: 0.45,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::ImpulseRocketFire => {
            // Rising pitch sweep + sustained burn noise
            SoundGenerator {
                oscillators: vec![Oscillator::new(120.0, Waveform::Sawtooth)],
                noise: Some(WhiteNoise::new()),
                envelope: Envelope::new(0.05, 0.1, 0.6, 0.2),
                filter: Some(LowPassFilter::new(1500.0, 0.7, sr)),
                duration: 0.8,
                time: 0.0,
                freq_start: 120.0,
                freq_end: 400.0,
                use_freq_sweep: true,
                volume: 0.4,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::TidalMineDeploy => {
            // Metallic ping – lower sine with fast decay, 0.15s
            SoundGenerator {
                oscillators: vec![
                    Oscillator::new(1200.0, Waveform::Sine),
                    Oscillator::new(1900.0, Waveform::Sine), // inharmonic partial for metallic quality
                ],
                noise: None,
                envelope: Envelope::new(0.001, 0.14, 0.0, 0.01),
                filter: None,
                duration: 0.15,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.3,
                sustaining: false,
                releasing: false,
            }
        }

        // ---- Damage --------------------------------------------------------
        SoundEvent::ShieldHit => {
            // Softer metallic ring – lower sine with medium decay, 0.2s
            SoundGenerator {
                oscillators: vec![
                    Oscillator::new(1200.0, Waveform::Sine),
                    Oscillator::new(1600.0, Waveform::Sine),
                ],
                noise: None,
                envelope: Envelope::new(0.002, 0.18, 0.0, 0.02),
                filter: None,
                duration: 0.2,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.25,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::HullHit => {
            // Dull thud – low noise burst through LPF, 0.15s
            SoundGenerator {
                oscillators: vec![Oscillator::new(60.0, Waveform::Sine)],
                noise: Some(WhiteNoise::new()),
                envelope: Envelope::new(0.002, 0.13, 0.0, 0.02),
                filter: Some(LowPassFilter::new(300.0, 0.7, sr)),
                duration: 0.15,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.7,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::Explosion => {
            // Noise burst through LPF with long decay, 0.5s
            SoundGenerator {
                oscillators: vec![Oscillator::new(40.0, Waveform::Sine)],
                noise: Some(WhiteNoise::new()),
                envelope: Envelope::new(0.005, 0.45, 0.0, 0.05),
                filter: Some(LowPassFilter::new(500.0, 0.5, sr)),
                duration: 0.5,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.9,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::Spaghettification => {
            // Dramatic descending sweep with noise, long duration
            SoundGenerator {
                oscillators: vec![
                    Oscillator::new(600.0, Waveform::Sawtooth),
                    Oscillator::new(300.0, Waveform::Sine),
                ],
                noise: Some(WhiteNoise::new()),
                envelope: Envelope::new(0.1, 0.8, 0.0, 0.1),
                filter: Some(LowPassFilter::new(400.0, 1.5, sr)),
                duration: 1.0,
                time: 0.0,
                freq_start: 600.0,
                freq_end: 20.0,
                use_freq_sweep: true,
                volume: 0.9,
                sustaining: false,
                releasing: false,
            }
        }

        // ---- Radio / UI ----------------------------------------------------
        SoundEvent::RadioOpen => {
            // Quick ascending two-tone blip
            SoundGenerator {
                oscillators: vec![Oscillator::new(600.0, Waveform::Sine)],
                noise: None,
                envelope: Envelope::new(0.005, 0.08, 0.0, 0.02),
                filter: None,
                duration: 0.1,
                time: 0.0,
                freq_start: 600.0,
                freq_end: 1200.0,
                use_freq_sweep: true,
                volume: 0.4,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::RadioClose => {
            // Quick descending blip
            SoundGenerator {
                oscillators: vec![Oscillator::new(1200.0, Waveform::Sine)],
                noise: None,
                envelope: Envelope::new(0.005, 0.08, 0.0, 0.02),
                filter: None,
                duration: 0.1,
                time: 0.0,
                freq_start: 1200.0,
                freq_end: 600.0,
                use_freq_sweep: true,
                volume: 0.4,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::WarningEscapeVelocity => {
            // Urgent repeating high tone (single burst here; the game layer
            // would re-trigger for repeats)
            SoundGenerator {
                oscillators: vec![Oscillator::new(1000.0, Waveform::Square)],
                noise: None,
                envelope: Envelope::new(0.005, 0.15, 0.0, 0.05),
                filter: Some(LowPassFilter::new(3000.0, 0.7, sr)),
                duration: 0.2,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.5,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::WarningLowFuel => {
            // Two-tone beep
            SoundGenerator {
                oscillators: vec![Oscillator::new(800.0, Waveform::Sine)],
                noise: None,
                envelope: Envelope::new(0.01, 0.15, 0.0, 0.05),
                filter: None,
                duration: 0.2,
                time: 0.0,
                freq_start: 800.0,
                freq_end: 500.0,
                use_freq_sweep: true,
                volume: 0.45,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::UIConfirm => {
            // Bright ascending ding
            SoundGenerator {
                oscillators: vec![Oscillator::new(880.0, Waveform::Sine)],
                noise: None,
                envelope: Envelope::new(0.005, 0.12, 0.0, 0.03),
                filter: None,
                duration: 0.15,
                time: 0.0,
                freq_start: 880.0,
                freq_end: 1320.0,
                use_freq_sweep: true,
                volume: 0.4,
                sustaining: false,
                releasing: false,
            }
        }

        SoundEvent::UISelect => {
            // Soft tick
            SoundGenerator {
                oscillators: vec![Oscillator::new(1200.0, Waveform::Sine)],
                noise: None,
                envelope: Envelope::new(0.002, 0.04, 0.0, 0.01),
                filter: None,
                duration: 0.05,
                time: 0.0,
                freq_start: 0.0,
                freq_end: 0.0,
                use_freq_sweep: false,
                volume: 0.35,
                sustaining: false,
                releasing: false,
            }
        }
    }
}
