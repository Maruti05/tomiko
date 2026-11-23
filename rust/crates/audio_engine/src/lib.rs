//! Real-time audio engine.
//!
//! Phase 5 contains the allocation-free buffer primitives, RMS/envelope
//! analysis, validated voice presets, the DSP chain, VAD, lip-sync meters, and
//! the AAudio-backed capture/playback runtime on Android.
//!
//! Real-time callbacks only touch preallocated SPSC rings and atomics. All
//! allocation and locking happens on the control/DSP worker threads.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod buffer;
pub mod dsp;
pub mod lipsync;
pub mod presets;
pub mod runtime;
pub mod vad;

mod aaudio;

pub use buffer::{AudioBuffer, EnvelopeFollower, peak_i16, rms_i16};
pub use dsp::{VoiceProcessor, soft_clip};
pub use lipsync::{LipSync, LipSyncFrame};
pub use presets::{VoicePresetParameters, parameters_for};
pub use runtime::{AudioCommand, AudioMeters, AudioRuntime};
pub use vad::{Vad, VadConfig, VadEvent};

pub const AUDIO_ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
