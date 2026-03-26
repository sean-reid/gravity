#[cfg(feature = "web")]
mod inner {
    use wasm_bindgen::JsValue;
    use web_sys::{AudioContext, BiquadFilterNode, BiquadFilterType, GainNode, OscillatorNode, OscillatorType};

    use crate::audio::mod_types::{AmbientParams, AudioBackend, SoundEvent};

    // -----------------------------------------------------------------------
    // Helper: create an AudioContext (handling browser prefix)
    // -----------------------------------------------------------------------
    fn create_audio_context() -> Result<AudioContext, JsValue> {
        AudioContext::new()
    }

    // -----------------------------------------------------------------------
    // Helper: schedule a simple tone
    // -----------------------------------------------------------------------
    fn play_tone(
        ctx: &AudioContext,
        freq_start: f32,
        freq_end: Option<f32>,
        osc_type: OscillatorType,
        duration: f64,
        gain_val: f32,
        filter_freq: Option<f32>,
    ) -> Result<(), JsValue> {
        let current_time = ctx.current_time();

        // Oscillator
        let osc = ctx.create_oscillator()?;
        osc.set_type(osc_type);
        osc.frequency().set_value(freq_start);

        if let Some(end_freq) = freq_end {
            osc.frequency()
                .linear_ramp_to_value_at_time(end_freq, current_time + duration)?;
        }

        // Gain envelope
        let gain = ctx.create_gain()?;
        gain.gain().set_value(gain_val);
        gain.gain()
            .linear_ramp_to_value_at_time(0.0, current_time + duration)?;

        // Optional filter
        if let Some(cutoff) = filter_freq {
            let filter = ctx.create_biquad_filter()?;
            filter.set_type(BiquadFilterType::Lowpass);
            filter.frequency().set_value(cutoff);
            filter.q().set_value(1.0);

            osc.connect_with_audio_node(&filter)?;
            filter.connect_with_audio_node(&gain)?;
        } else {
            osc.connect_with_audio_node(&gain)?;
        }

        gain.connect_with_audio_node(&ctx.destination())?;

        osc.start()?;
        osc.stop_with_when(current_time + duration)?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // WebAudio backend
    // -----------------------------------------------------------------------
    pub struct WebAudio {
        ctx: AudioContext,
        drone_osc: Option<OscillatorNode>,
        drone_gain: Option<GainNode>,
        drone_filter: Option<BiquadFilterNode>,
        master_volume: f64,
    }

    impl WebAudio {
        pub fn try_new() -> Option<Self> {
            let ctx = create_audio_context().ok()?;
            let mut wa = WebAudio {
                ctx,
                drone_osc: None,
                drone_gain: None,
                drone_filter: None,
                master_volume: 0.8,
            };
            wa.init_drone();
            Some(wa)
        }

        fn init_drone(&mut self) {
            let ctx = &self.ctx;

            let osc = match ctx.create_oscillator() {
                Ok(o) => o,
                Err(_) => return,
            };
            osc.set_type(OscillatorType::Sawtooth);
            osc.frequency().set_value(60.0);

            let filter = match ctx.create_biquad_filter() {
                Ok(f) => f,
                Err(_) => return,
            };
            filter.set_type(BiquadFilterType::Lowpass);
            filter.frequency().set_value(200.0);
            filter.q().set_value(0.7);

            let gain = match ctx.create_gain() {
                Ok(g) => g,
                Err(_) => return,
            };
            gain.gain().set_value(0.1);

            if osc.connect_with_audio_node(&filter).is_err() {
                return;
            }
            if filter.connect_with_audio_node(&gain).is_err() {
                return;
            }
            if gain.connect_with_audio_node(&ctx.destination()).is_err() {
                return;
            }
            let _ = osc.start();

            self.drone_osc = Some(osc);
            self.drone_gain = Some(gain);
            self.drone_filter = Some(filter);
        }

        fn play_event(&self, event: &SoundEvent) {
            let ctx = &self.ctx;
            let vol = self.master_volume as f32;

            let result = match event {
                SoundEvent::RailgunFire => {
                    // Bright snap - high frequency noise approximated with square wave
                    play_tone(ctx, 4000.0, Some(8000.0), OscillatorType::Square, 0.1, 0.7 * vol, Some(8000.0))
                }
                SoundEvent::MassDriverFire => {
                    play_tone(ctx, 50.0, None, OscillatorType::Sine, 0.3, 0.8 * vol, Some(200.0))
                }
                SoundEvent::PhotonLanceStart => {
                    play_tone(ctx, 400.0, Some(1200.0), OscillatorType::Sine, 2.0, 0.5 * vol, None)
                }
                SoundEvent::PhotonLanceStop => Ok(()),
                SoundEvent::GravityBombDeploy => {
                    play_tone(ctx, 400.0, Some(80.0), OscillatorType::Sine, 0.5, 0.7 * vol, Some(600.0))
                }
                SoundEvent::ImpulseRocketFire => {
                    play_tone(ctx, 120.0, Some(400.0), OscillatorType::Sawtooth, 0.8, 0.6 * vol, Some(1500.0))
                }
                SoundEvent::TidalMineDeploy => {
                    play_tone(ctx, 2000.0, None, OscillatorType::Sine, 0.15, 0.5 * vol, None)
                }
                SoundEvent::ShieldHit => {
                    play_tone(ctx, 1800.0, None, OscillatorType::Sine, 0.2, 0.5 * vol, None)
                }
                SoundEvent::HullHit => {
                    play_tone(ctx, 60.0, None, OscillatorType::Sine, 0.15, 0.7 * vol, Some(300.0))
                }
                SoundEvent::Explosion => {
                    play_tone(ctx, 40.0, None, OscillatorType::Sawtooth, 0.5, 0.9 * vol, Some(500.0))
                }
                SoundEvent::Spaghettification => {
                    play_tone(ctx, 600.0, Some(20.0), OscillatorType::Sawtooth, 1.0, 0.9 * vol, Some(400.0))
                }
                SoundEvent::RadioOpen => {
                    play_tone(ctx, 600.0, Some(1200.0), OscillatorType::Sine, 0.1, 0.4 * vol, None)
                }
                SoundEvent::RadioClose => {
                    play_tone(ctx, 1200.0, Some(600.0), OscillatorType::Sine, 0.1, 0.4 * vol, None)
                }
                SoundEvent::WarningEscapeVelocity => {
                    play_tone(ctx, 1000.0, None, OscillatorType::Square, 0.2, 0.5 * vol, Some(3000.0))
                }
                SoundEvent::WarningLowFuel => {
                    play_tone(ctx, 800.0, Some(500.0), OscillatorType::Sine, 0.2, 0.45 * vol, None)
                }
                SoundEvent::UIConfirm => {
                    play_tone(ctx, 880.0, Some(1320.0), OscillatorType::Sine, 0.15, 0.4 * vol, None)
                }
                SoundEvent::UISelect => {
                    play_tone(ctx, 1200.0, None, OscillatorType::Sine, 0.05, 0.35 * vol, None)
                }
            };

            if let Err(e) = result {
                log::warn!("WebAudio play_event error: {:?}", e);
            }
        }
    }

    impl AudioBackend for WebAudio {
        fn play_sound(&mut self, sound: SoundEvent) {
            self.play_event(&sound);
        }

        fn set_ambient_params(&mut self, params: AmbientParams) {
            // Adjust drone frequency and filter based on depth
            let freq = 60.0 - 40.0 * params.depth_factor;
            let cutoff = 200.0 - 140.0 * params.depth_factor;
            let gain = 0.10 + 0.15 * params.depth_factor;

            if let Some(ref osc) = self.drone_osc {
                osc.frequency().set_value(freq as f32);
            }
            if let Some(ref filter) = self.drone_filter {
                filter.frequency().set_value(cutoff as f32);
            }
            if let Some(ref g) = self.drone_gain {
                g.gain().set_value(gain as f32 * self.master_volume as f32);
            }
        }

        fn set_heartbeat_rate(&mut self, _bpm: f64) {
            // Heartbeat on web is simplified: the game loop would need to
            // call play_sound with a heartbeat event at the right rate.
            // For a full implementation, use setInterval via js_sys.
        }

        fn set_master_volume(&mut self, volume: f64) {
            self.master_volume = volume.clamp(0.0, 1.0);
        }

        fn update(&mut self, _dt: f64) {
            // Web Audio API is self-timed; nothing needed here.
        }
    }
}

#[cfg(feature = "web")]
pub use inner::WebAudio;
