//! Thread-safe JNI-facing engine state and bounded command channel.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod engine;

pub use engine::{
    EngineCommand, EngineHandle, audio_meters, ensure_audio_runtime, ensure_initialized, global,
    initialize, pause, play_last, resume, send_audio, send_command, set_character, set_quality,
    set_voice_preset, shutdown, snapshot, start_listening, stop_listening, take_receiver,
};

pub const BRIDGE_VERSION: &str = env!("CARGO_PKG_VERSION");
