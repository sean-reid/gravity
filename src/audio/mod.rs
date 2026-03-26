// ---------------------------------------------------------------------------
// Audio subsystem for Gravity Well Arena
//
// ALL audio is procedurally generated – no audio file assets.
// Feature flags: "native" enables cpal, "web" enables web-sys AudioContext.
// ---------------------------------------------------------------------------

pub mod synth;
pub mod sfx;
pub mod depth_audio;

#[cfg(feature = "native")]
pub mod native_audio;

#[cfg(feature = "web")]
pub mod web_audio;

// Re-export the core types under a private name so sibling modules can use
// them without circular `use crate::audio::...` that names the enum before
// the module is fully defined.  Public re-exports below give callers the
// clean path `crate::audio::SoundEvent` etc.
mod mod_types {
    // -----------------------------------------------------------------------
    // Sound events
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SoundEvent {
        RailgunFire,
        MassDriverFire,
        PhotonLanceStart,
        PhotonLanceStop,
        GravityBombDeploy,
        ImpulseRocketFire,
        TidalMineDeploy,
        ShieldHit,
        HullHit,
        Explosion,
        Spaghettification,
        RadioOpen,
        RadioClose,
        WarningEscapeVelocity,
        WarningLowFuel,
        UIConfirm,
        UISelect,
    }

    // -----------------------------------------------------------------------
    // Ambient parameters
    // -----------------------------------------------------------------------

    #[derive(Debug, Clone, Copy)]
    pub struct AmbientParams {
        /// 0 = rim (safe), 1 = abyss (dangerous)
        pub depth_factor: f64,
        /// Proper-time rate of the player (for relativistic pitch shifting)
        pub player_tau: f64,
    }

    // -----------------------------------------------------------------------
    // Audio backend trait
    // -----------------------------------------------------------------------

    pub trait AudioBackend: Send {
        /// Trigger a one-shot (or start/stop) sound effect.
        fn play_sound(&mut self, sound: SoundEvent);

        /// Update the ambient audio parameters (depth, time dilation, etc.).
        fn set_ambient_params(&mut self, params: AmbientParams);

        /// Set the heartbeat rate in BPM. 0 = off.
        fn set_heartbeat_rate(&mut self, bpm: f64);

        /// Set the master output volume in [0, 1].
        fn set_master_volume(&mut self, volume: f64);

        /// Called once per game frame with the frame delta time in seconds.
        fn update(&mut self, dt: f64);
    }

    // -----------------------------------------------------------------------
    // Null backend (fallback – does nothing)
    // -----------------------------------------------------------------------

    pub struct NullAudio;

    impl AudioBackend for NullAudio {
        fn play_sound(&mut self, _sound: SoundEvent) {}
        fn set_ambient_params(&mut self, _params: AmbientParams) {}
        fn set_heartbeat_rate(&mut self, _bpm: f64) {}
        fn set_master_volume(&mut self, _volume: f64) {}
        fn update(&mut self, _dt: f64) {}
    }
}

// ---------------------------------------------------------------------------
// Public re-exports
// ---------------------------------------------------------------------------

pub use mod_types::{AmbientParams, AudioBackend, NullAudio, SoundEvent};

#[cfg(feature = "native")]
pub use native_audio::NativeAudio;

#[cfg(feature = "web")]
pub use web_audio::WebAudio;

// ---------------------------------------------------------------------------
// Convenience constructor
// ---------------------------------------------------------------------------

/// Create the best available audio backend for the current platform.
/// Falls back to `NullAudio` if hardware initialisation fails.
pub fn create_audio_backend() -> Box<dyn AudioBackend> {
    #[cfg(feature = "native")]
    {
        if let Some(native) = NativeAudio::try_new() {
            log::info!("Audio: using native (cpal) backend");
            return Box::new(native);
        }
        log::warn!("Audio: native backend failed to initialise, falling back to NullAudio");
    }

    #[cfg(feature = "web")]
    {
        if let Some(web) = WebAudio::try_new() {
            log::info!("Audio: using Web Audio API backend");
            return Box::new(web);
        }
        log::warn!("Audio: Web Audio backend failed to initialise, falling back to NullAudio");
    }

    log::info!("Audio: using NullAudio (silent)");
    Box::new(NullAudio)
}
