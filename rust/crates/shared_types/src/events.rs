use crate::config::QualityTier;

/// High-level audio/engine state exposed to Kotlin and Bevy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum EngineState {
    Idle = 0,
    Listening = 1,
    Capturing = 2,
    Processing = 3,
    Playing = 4,
    Paused = 5,
    Error = 6,
}

impl EngineState {
    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Idle),
            1 => Some(Self::Listening),
            2 => Some(Self::Capturing),
            3 => Some(Self::Processing),
            4 => Some(Self::Playing),
            5 => Some(Self::Paused),
            6 => Some(Self::Error),
            _ => None,
        }
    }
}

/// Character presentation state. The Rust `character_engine` owns transitions;
/// Bevy applies the resulting animation events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CharacterState {
    Idle = 0,
    Listening = 1,
    Talking = 2,
    Happy = 3,
    Surprised = 4,
    Sad = 5,
    Angry = 6,
    Sleeping = 7,
    Blinking = 8,
    TouchReaction = 9,
}

impl CharacterState {
    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Idle),
            1 => Some(Self::Listening),
            2 => Some(Self::Talking),
            3 => Some(Self::Happy),
            4 => Some(Self::Surprised),
            5 => Some(Self::Sad),
            6 => Some(Self::Angry),
            7 => Some(Self::Sleeping),
            8 => Some(Self::Blinking),
            9 => Some(Self::TouchReaction),
            _ => None,
        }
    }
}

/// Touch regions reported by Bevy picking or the native input bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TouchZone {
    Head = 0,
    Body = 1,
    LeftEar = 2,
    RightEar = 3,
    Belly = 4,
    Background = 5,
}

impl TouchZone {
    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Head),
            1 => Some(Self::Body),
            2 => Some(Self::LeftEar),
            3 => Some(Self::RightEar),
            4 => Some(Self::Belly),
            5 => Some(Self::Background),
            _ => None,
        }
    }
}

/// Events consumed by Bevy's animation/facial systems.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationEvent {
    /// Crossfade to the clip associated with `state`.
    Play { state: CharacterState, fade_ms: u32 },
    /// Set the current mouth openness and viseme blend.
    SetMouth {
        openness: f32,
        viseme: u8,
        weight: f32,
    },
    /// Trigger one blink cycle.
    Blink,
    /// Play a zone-specific touch reaction.
    TouchReaction { zone: TouchZone, fade_ms: u32 },
    /// Return all mouth morphs to neutral.
    ResetMouth,
}

pub const FLAG_PERMISSION_REQUIRED: u32 = 1 << 0;
pub const FLAG_REPEATING: u32 = 1 << 1;
pub const FLAG_AUDIO_FOCUS_LOST: u32 = 1 << 2;

/// Compact state snapshot. Kotlin polls this at a low rate for UI state; Bevy
/// reads the underlying atomics directly every frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiStateSnapshot {
    pub engine_state: EngineState,
    pub character_state: CharacterState,
    pub quality_tier: QualityTier,
    pub flags: u32,
    pub amplitude: f32,
    pub progress: f32,
    pub error_code: i32,
    pub mouth_open: f32,
}

impl Default for UiStateSnapshot {
    fn default() -> Self {
        Self {
            engine_state: EngineState::Idle,
            character_state: CharacterState::Idle,
            quality_tier: QualityTier::Medium,
            flags: 0,
            amplitude: 0.0,
            progress: 0.0,
            error_code: 0,
            mouth_open: 0.0,
        }
    }
}

/// FFI-safe snapshot. Keep field order and types stable; Kotlin reads it from
/// a direct `ByteBuffer` in native byte order.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeUiState {
    pub engine_state: i32,
    pub character_state: i32,
    pub flags: u32,
    pub amplitude: f32,
    pub progress: f32,
    pub error_code: i32,
    pub mouth_open: f32,
    pub quality_tier: i32,
}

impl Default for NativeUiState {
    fn default() -> Self {
        Self::from_snapshot(UiStateSnapshot::default())
    }
}

impl NativeUiState {
    #[must_use]
    pub fn from_snapshot(snapshot: UiStateSnapshot) -> Self {
        Self {
            engine_state: snapshot.engine_state as i32,
            character_state: snapshot.character_state as i32,
            flags: snapshot.flags,
            amplitude: snapshot.amplitude.clamp(0.0, 1.0),
            progress: snapshot.progress.clamp(0.0, 1.0),
            error_code: snapshot.error_code,
            mouth_open: snapshot.mouth_open.clamp(0.0, 1.0),
            quality_tier: snapshot.quality_tier as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_state_round_trips() {
        for id in 0..=6 {
            let state = EngineState::from_id(id).expect("valid state");
            assert_eq!(state as i32, id);
        }
        assert_eq!(EngineState::from_id(7), None);
    }

    #[test]
    fn character_state_round_trips() {
        for id in 0..=9 {
            let state = CharacterState::from_id(id).expect("valid state");
            assert_eq!(state as i32, id);
        }
        assert_eq!(CharacterState::from_id(10), None);
    }

    #[test]
    fn touch_zone_round_trips() {
        for id in 0..=5 {
            let zone = TouchZone::from_id(id).expect("valid zone");
            assert_eq!(zone as i32, id);
        }
        assert_eq!(TouchZone::from_id(6), None);
    }

    #[test]
    fn native_snapshot_clamps_values() {
        let snapshot = UiStateSnapshot {
            amplitude: 2.0,
            progress: -1.0,
            mouth_open: 0.5,
            ..UiStateSnapshot::default()
        };
        let native = NativeUiState::from_snapshot(snapshot);
        assert_eq!(native.amplitude, 1.0);
        assert_eq!(native.progress, 0.0);
        assert_eq!(native.mouth_open, 0.5);
    }

    #[test]
    fn native_snapshot_is_stable_size() {
        // 8 x 4-byte fields. If this changes, bump ENGINE_ABI_VERSION and update Kotlin.
        assert_eq!(core::mem::size_of::<NativeUiState>(), 32);
        assert_eq!(core::mem::align_of::<NativeUiState>(), 4);
    }
}
