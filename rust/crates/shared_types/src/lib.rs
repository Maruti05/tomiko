//! Shared types used by the Kotlin bridge, audio engine, character engine,
//! and Bevy renderer.
//!
//! This crate intentionally contains no Android, JNI, Bevy, or audio-device
//! dependencies so it can be unit tested on the host.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod config;
pub mod errors;
pub mod events;

pub use config::{AudioConfig, CharacterDescriptor, QualityTier, VoicePreset, VoicePresetId};
pub use errors::{EngineError, ErrorCode};
pub use events::{
    AnimationEvent, CharacterState, EngineState, FLAG_AUDIO_FOCUS_LOST, FLAG_PERMISSION_REQUIRED,
    FLAG_REPEATING, NativeUiState, TouchZone, UiStateSnapshot,
};

/// Incremented whenever the Kotlin/Rust JNI contract changes.
pub const ENGINE_ABI_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_is_stable() {
        assert_eq!(ENGINE_ABI_VERSION, 1);
    }
}
