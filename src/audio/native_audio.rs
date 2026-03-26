#[cfg(feature = "native")]
mod inner {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    use crate::audio::depth_audio::{AmbientDrone, HeartbeatGenerator};
    use crate::audio::mod_types::{AmbientParams, AudioBackend, SoundEvent};
    use crate::audio::sfx::{self, SoundGenerator};

    // -----------------------------------------------------------------------
    // Commands from game thread -> audio thread
    // -----------------------------------------------------------------------
    pub(crate) enum AudioCommand {
        PlaySound(SoundEvent),
        SetAmbient(AmbientParams),
        SetHeartbeat(f64),
        SetVolume(f64),
    }

    // -----------------------------------------------------------------------
    // Shared audio state (lives on the audio thread, behind Arc<Mutex<>>)
    // -----------------------------------------------------------------------
    struct AudioState {
        active_sounds: Vec<SoundGenerator>,
        ambient_drone: AmbientDrone,
        heartbeat: HeartbeatGenerator,
        master_volume: f64,
        sample_rate: f64,
    }

    impl AudioState {
        fn new(sample_rate: f64) -> Self {
            Self {
                active_sounds: Vec::with_capacity(16),
                ambient_drone: AmbientDrone::new(),
                heartbeat: HeartbeatGenerator::new(),
                master_volume: 0.8,
                sample_rate,
            }
        }

        fn process_commands(&mut self, commands: &[AudioCommand]) {
            for cmd in commands {
                match cmd {
                    AudioCommand::PlaySound(event) => {
                        // PhotonLanceStop: stop any sustaining lance sounds
                        if matches!(event, SoundEvent::PhotonLanceStop) {
                            for snd in &mut self.active_sounds {
                                if snd.is_sustaining() {
                                    snd.stop();
                                }
                            }
                        } else {
                            let gen = sfx::create_sound(event);
                            // Cap simultaneous sounds at 8; evict oldest
                            if self.active_sounds.len() >= 8 {
                                // Remove the first non-sustaining finished or oldest
                                if let Some(idx) = self
                                    .active_sounds
                                    .iter()
                                    .position(|s| s.is_finished())
                                {
                                    self.active_sounds.remove(idx);
                                } else {
                                    self.active_sounds.remove(0);
                                }
                            }
                            self.active_sounds.push(gen);
                        }
                    }
                    AudioCommand::SetAmbient(params) => {
                        self.ambient_drone.set_depth(params.depth_factor);
                    }
                    AudioCommand::SetHeartbeat(bpm) => {
                        self.heartbeat.set_bpm(*bpm);
                    }
                    AudioCommand::SetVolume(vol) => {
                        self.master_volume = vol.clamp(0.0, 1.0);
                    }
                }
            }
        }

        fn render_sample(&mut self) -> f32 {
            let sr = self.sample_rate;

            // Ambient layers
            let ambient = self.ambient_drone.sample(sr);
            let heartbeat = self.heartbeat.sample(sr);

            // Active SFX
            let mut sfx_sum = 0.0f32;
            for snd in &mut self.active_sounds {
                sfx_sum += snd.sample(sr);
            }

            // Remove finished sounds
            self.active_sounds.retain(|s| !s.is_finished());

            let mixed = ambient + heartbeat + sfx_sum;
            (mixed * self.master_volume as f32).clamp(-1.0, 1.0)
        }
    }

    // -----------------------------------------------------------------------
    // Public backend
    // -----------------------------------------------------------------------

    pub struct NativeAudio {
        sound_sender: mpsc::Sender<AudioCommand>,
        /// The cpal Stream is !Send, so we keep it alive on a dedicated
        /// thread and hold the JoinHandle here (which IS Send).
        _stream_thread: Option<std::thread::JoinHandle<()>>,
        /// Shared flag to signal the stream thread to exit on drop.
        _keep_alive: Arc<Mutex<bool>>,
        master_volume: f64,
    }

    impl Drop for NativeAudio {
        fn drop(&mut self) {
            if let Ok(mut alive) = self._keep_alive.lock() {
                *alive = false;
            }
            if let Some(handle) = self._stream_thread.take() {
                handle.thread().unpark();
                let _ = handle.join();
            }
        }
    }

    impl NativeAudio {
        /// Try to open the default audio output. Returns `None` on failure so
        /// the caller can fall back to `NullAudio`.
        pub fn try_new() -> Option<Self> {
            let host = cpal::default_host();
            let device = host.default_output_device()?;
            let config = device.default_output_config().ok()?;
            let sample_rate = config.sample_rate().0 as f64;
            let channels = config.channels() as usize;

            let (tx, rx) = mpsc::channel::<AudioCommand>();

            let state = Arc::new(Mutex::new(AudioState::new(sample_rate)));
            let state_clone = Arc::clone(&state);

            // Buffer for draining commands without holding the channel lock
            let cmd_buf: Arc<Mutex<Vec<AudioCommand>>> =
                Arc::new(Mutex::new(Vec::with_capacity(32)));
            let cmd_buf_clone = Arc::clone(&cmd_buf);

            // A signal so the stream thread knows when to exit.
            let keep_alive = Arc::new(Mutex::new(true));
            let keep_alive_clone = Arc::clone(&keep_alive);

            // Spawn a lightweight helper that moves commands from the mpsc
            // channel into the shared buffer. This avoids calling recv inside
            // the real-time audio callback.
            std::thread::Builder::new()
                .name("audio-cmd-relay".into())
                .spawn(move || {
                    while let Ok(cmd) = rx.recv() {
                        if let Ok(mut buf) = cmd_buf_clone.lock() {
                            buf.push(cmd);
                        }
                    }
                })
                .ok()?;

            // Build and play the stream on a dedicated thread so the !Send
            // cpal::Stream never needs to cross thread boundaries.
            let stream_config: cpal::StreamConfig = config.into();

            let stream_thread = std::thread::Builder::new()
                .name("audio-stream".into())
                .spawn(move || {
                    let stream = match device.build_output_stream(
                        &stream_config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            // Drain pending commands
                            {
                                let mut buf = cmd_buf.lock().unwrap();
                                if !buf.is_empty() {
                                    let cmds: Vec<AudioCommand> = buf.drain(..).collect();
                                    drop(buf);
                                    let mut st = state_clone.lock().unwrap();
                                    st.process_commands(&cmds);
                                }
                            }

                            let mut st = state_clone.lock().unwrap();
                            for frame in data.chunks_mut(channels) {
                                let sample = st.render_sample();
                                for s in frame.iter_mut() {
                                    *s = sample;
                                }
                            }
                        },
                        |err| {
                            log::error!("Audio stream error: {}", err);
                        },
                        None,
                    ) {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Failed to build audio stream: {}", e);
                            return;
                        }
                    };

                    if let Err(e) = stream.play() {
                        log::error!("Failed to play audio stream: {}", e);
                        return;
                    }

                    // Park this thread, keeping the stream alive, until the
                    // NativeAudio handle is dropped (keep_alive set to false).
                    loop {
                        std::thread::park_timeout(std::time::Duration::from_millis(500));
                        if let Ok(alive) = keep_alive_clone.lock() {
                            if !*alive {
                                break;
                            }
                        }
                    }
                    // `stream` drops here, stopping playback.
                })
                .ok()?;

            // Give the stream thread a moment to initialise.
            std::thread::sleep(std::time::Duration::from_millis(50));

            Some(NativeAudio {
                sound_sender: tx,
                _stream_thread: Some(stream_thread),
                _keep_alive: keep_alive,
                master_volume: 0.8,
            })
        }
    }

    impl AudioBackend for NativeAudio {
        fn play_sound(&mut self, sound: SoundEvent) {
            let _ = self.sound_sender.send(AudioCommand::PlaySound(sound));
        }

        fn set_ambient_params(&mut self, params: AmbientParams) {
            let _ = self.sound_sender.send(AudioCommand::SetAmbient(params));
        }

        fn set_heartbeat_rate(&mut self, bpm: f64) {
            let _ = self.sound_sender.send(AudioCommand::SetHeartbeat(bpm));
        }

        fn set_master_volume(&mut self, volume: f64) {
            self.master_volume = volume;
            let _ = self.sound_sender.send(AudioCommand::SetVolume(volume));
        }

        fn update(&mut self, _dt: f64) {
            // Audio rendering happens on the cpal callback thread;
            // nothing to do here.
        }
    }
}

#[cfg(feature = "native")]
pub use inner::NativeAudio;
