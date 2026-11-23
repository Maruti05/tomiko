use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use audio_engine::{AudioCommand, AudioMeters, AudioRuntime};
use crossbeam_channel::{Receiver, Sender, bounded};
use shared_types::{
    AudioConfig, CharacterState, EngineError, EngineState, NativeUiState, QualityTier,
    UiStateSnapshot, VoicePreset,
};

/// Commands crossing from Kotlin into the Rust/Bevy world. The channel is
/// bounded so a misbehaving caller cannot grow memory without limit.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineCommand {
    StartListening,
    StopListening,
    PlayLast,
    SetVoicePreset { preset: VoicePreset, intensity: f32 },
    SetQuality(QualityTier),
    SetCharacter(String),
    Pause,
    Resume,
    Shutdown,
}

/// Thread-safe native engine handle. Audio/rendering code reads atomics;
/// Kotlin commands go through the bounded channel.
#[derive(Debug)]
pub struct EngineHandle {
    command_tx: Sender<EngineCommand>,
    receiver: Mutex<Option<Receiver<EngineCommand>>>,
    initialized: AtomicBool,
    engine_state: AtomicI32,
    character_state: AtomicI32,
    quality_tier: AtomicI32,
    flags: AtomicU32,
    amplitude_bits: AtomicU32,
    progress_bits: AtomicU32,
    mouth_open_bits: AtomicU32,
    error_code: AtomicI32,
}

impl EngineHandle {
    #[must_use]
    pub fn new() -> Self {
        let (command_tx, receiver) = bounded(64);
        Self {
            command_tx,
            receiver: Mutex::new(Some(receiver)),
            initialized: AtomicBool::new(false),
            engine_state: AtomicI32::new(EngineState::Idle as i32),
            character_state: AtomicI32::new(CharacterState::Idle as i32),
            quality_tier: AtomicI32::new(QualityTier::Medium as i32),
            flags: AtomicU32::new(0),
            amplitude_bits: AtomicU32::new(0.0f32.to_bits()),
            progress_bits: AtomicU32::new(0.0f32.to_bits()),
            mouth_open_bits: AtomicU32::new(0.0f32.to_bits()),
            error_code: AtomicI32::new(0),
        }
    }

    pub fn send(&self, command: EngineCommand) -> Result<(), EngineError> {
        self.command_tx
            .try_send(command)
            .map_err(|_| EngineError::ChannelClosed)
    }

    /// Takes the single consumer. Bevy calls this during startup.
    pub fn take_receiver(&self) -> Option<Receiver<EngineCommand>> {
        self.receiver
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }

    pub fn set_initialized(&self, initialized: bool) {
        self.initialized.store(initialized, Ordering::Release);
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    pub fn set_engine_state(&self, state: EngineState) {
        self.engine_state.store(state as i32, Ordering::Release);
    }

    pub fn set_character_state(&self, state: CharacterState) {
        self.character_state.store(state as i32, Ordering::Release);
    }

    pub fn set_quality_tier(&self, tier: QualityTier) {
        self.quality_tier.store(tier as i32, Ordering::Release);
    }

    pub fn set_flags(&self, flags: u32) {
        self.flags.store(flags, Ordering::Release);
    }

    pub fn set_amplitude(&self, amplitude: f32) {
        self.amplitude_bits
            .store(amplitude.clamp(0.0, 1.0).to_bits(), Ordering::Release);
    }

    pub fn set_progress(&self, progress: f32) {
        self.progress_bits
            .store(progress.clamp(0.0, 1.0).to_bits(), Ordering::Release);
    }

    pub fn set_mouth_open(&self, mouth_open: f32) {
        self.mouth_open_bits
            .store(mouth_open.clamp(0.0, 1.0).to_bits(), Ordering::Release);
    }

    pub fn set_error_code(&self, code: i32) {
        self.error_code.store(code, Ordering::Release);
    }

    #[must_use]
    pub fn snapshot(&self) -> NativeUiState {
        NativeUiState {
            engine_state: self.engine_state.load(Ordering::Acquire),
            character_state: self.character_state.load(Ordering::Acquire),
            flags: self.flags.load(Ordering::Acquire),
            amplitude: f32::from_bits(self.amplitude_bits.load(Ordering::Acquire)),
            progress: f32::from_bits(self.progress_bits.load(Ordering::Acquire)),
            error_code: self.error_code.load(Ordering::Acquire),
            mouth_open: f32::from_bits(self.mouth_open_bits.load(Ordering::Acquire)),
            quality_tier: self.quality_tier.load(Ordering::Acquire),
        }
    }

    #[must_use]
    pub fn snapshot_typed(&self) -> UiStateSnapshot {
        let native = self.snapshot();
        UiStateSnapshot {
            engine_state: EngineState::from_id(native.engine_state).unwrap_or(EngineState::Error),
            character_state: CharacterState::from_id(native.character_state)
                .unwrap_or(CharacterState::Idle),
            quality_tier: QualityTier::from_id(native.quality_tier).unwrap_or(QualityTier::Medium),
            flags: native.flags,
            amplitude: native.amplitude,
            progress: native.progress,
            error_code: native.error_code,
            mouth_open: native.mouth_open,
        }
    }
}

impl Default for EngineHandle {
    fn default() -> Self {
        Self::new()
    }
}

static ENGINE: OnceLock<EngineHandle> = OnceLock::new();
static AUDIO: OnceLock<Mutex<Option<AudioRuntime>>> = OnceLock::new();

/// Idempotent initialization. The Bevy thread can call this before Kotlin does.
#[must_use]
pub fn ensure_initialized() -> &'static EngineHandle {
    ENGINE.get_or_init(EngineHandle::new)
}

#[must_use]
pub fn global() -> Option<&'static EngineHandle> {
    ENGINE.get()
}

/// Takes the command receiver once. Bevy calls this at startup.
#[must_use]
pub fn take_receiver() -> Option<Receiver<EngineCommand>> {
    global().and_then(EngineHandle::take_receiver)
}

pub fn initialize() -> Result<(), EngineError> {
    let handle = ensure_initialized();
    handle.set_initialized(true);
    handle.set_engine_state(EngineState::Idle);
    ensure_audio_runtime().map(|_| ())
}

/// Starts the audio runtime once and returns its meter atomics. Safe to call
/// from Kotlin's `onCreate` and again from Bevy's startup.
pub fn ensure_audio_runtime() -> Result<Arc<AudioMeters>, EngineError> {
    let slot = AUDIO.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_none() {
        *guard = Some(AudioRuntime::start(AudioConfig::default()));
    }
    guard
        .as_ref()
        .map(AudioRuntime::meters)
        .ok_or_else(|| EngineError::AudioInit("audio runtime did not start".to_owned()))
}

#[must_use]
pub fn audio_meters() -> Option<Arc<AudioMeters>> {
    let slot = AUDIO.get()?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.as_ref().map(AudioRuntime::meters)
}

/// Sends an audio command directly. This path deliberately bypasses Bevy so
/// `onPause`/`onStop` stop capture even while the Bevy event loop is suspended.
pub fn send_audio(command: AudioCommand) -> Result<(), EngineError> {
    let slot = AUDIO
        .get()
        .ok_or_else(|| EngineError::AudioInit("audio runtime is not initialized".to_owned()))?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .as_ref()
        .ok_or(EngineError::ChannelClosed)?
        .send(command)
}

pub fn shutdown() -> Result<(), EngineError> {
    if let Some(slot) = AUDIO.get()
        && let Some(runtime) = slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    {
        runtime.shutdown();
    }
    if let Some(handle) = global() {
        handle.set_initialized(false);
        handle.set_engine_state(EngineState::Idle);
        handle.set_amplitude(0.0);
        handle.set_progress(0.0);
        handle.set_mouth_open(0.0);
        let _ = handle.send(EngineCommand::Shutdown);
    }
    Ok(())
}

pub fn send_command(command: EngineCommand) -> Result<(), EngineError> {
    ensure_initialized().send(command)
}

pub fn start_listening() -> Result<(), EngineError> {
    send_audio(AudioCommand::StartListening)?;
    send_command(EngineCommand::StartListening)
}

pub fn stop_listening() -> Result<(), EngineError> {
    send_audio(AudioCommand::StopListening)?;
    send_command(EngineCommand::StopListening)
}

pub fn play_last() -> Result<(), EngineError> {
    send_audio(AudioCommand::PlayLast)?;
    send_command(EngineCommand::PlayLast)
}

pub fn pause() -> Result<(), EngineError> {
    send_audio(AudioCommand::Pause)?;
    send_command(EngineCommand::Pause)
}

pub fn resume() -> Result<(), EngineError> {
    send_audio(AudioCommand::Resume)?;
    send_command(EngineCommand::Resume)
}

pub fn set_voice_preset(id: i32, intensity: f32) -> Result<(), EngineError> {
    let preset = VoicePreset::from_id(id)
        .ok_or_else(|| EngineError::InvalidConfig(format!("unknown voice preset id {id}")))?;
    let intensity = intensity.clamp(0.0, 1.0);
    send_audio(AudioCommand::SetPreset { preset, intensity })?;
    send_command(EngineCommand::SetVoicePreset { preset, intensity })
}

pub fn set_quality(tier_id: i32) -> Result<(), EngineError> {
    let tier = match tier_id {
        0 => QualityTier::Low,
        1 => QualityTier::Medium,
        2 => QualityTier::High,
        _ => {
            return Err(EngineError::InvalidConfig(format!(
                "unknown quality tier id {tier_id}"
            )));
        }
    };
    send_command(EngineCommand::SetQuality(tier))
}

pub fn set_character(id: String) -> Result<(), EngineError> {
    if id.trim().is_empty() {
        return Err(EngineError::InvalidConfig(
            "character id must not be empty".to_owned(),
        ));
    }
    send_command(EngineCommand::SetCharacter(id))
}

#[must_use]
pub fn snapshot() -> NativeUiState {
    global().map_or_else(NativeUiState::default, EngineHandle::snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_sends_and_receives_commands() {
        let handle = EngineHandle::new();
        let receiver = handle.take_receiver().expect("receiver available");
        handle.send(EngineCommand::StartListening).unwrap();
        assert_eq!(receiver.try_recv().unwrap(), EngineCommand::StartListening);
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn receiver_can_only_be_taken_once() {
        let handle = EngineHandle::new();
        assert!(handle.take_receiver().is_some());
        assert!(handle.take_receiver().is_none());
    }

    #[test]
    fn snapshot_reflects_atomic_updates() {
        let handle = EngineHandle::new();
        handle.set_engine_state(EngineState::Playing);
        handle.set_character_state(CharacterState::Talking);
        handle.set_amplitude(0.75);
        handle.set_progress(0.5);
        handle.set_mouth_open(0.4);
        handle.set_error_code(7);

        let snapshot = handle.snapshot();
        assert_eq!(snapshot.engine_state, EngineState::Playing as i32);
        assert_eq!(snapshot.character_state, CharacterState::Talking as i32);
        assert!((snapshot.amplitude - 0.75).abs() < f32::EPSILON);
        assert!((snapshot.progress - 0.5).abs() < f32::EPSILON);
        assert!((snapshot.mouth_open - 0.4).abs() < f32::EPSILON);
        assert_eq!(snapshot.error_code, 7);
    }

    #[test]
    fn values_are_clamped() {
        let handle = EngineHandle::new();
        handle.set_amplitude(2.0);
        handle.set_progress(-1.0);
        handle.set_mouth_open(1.5);
        let snapshot = handle.snapshot();
        assert_eq!(snapshot.amplitude, 1.0);
        assert_eq!(snapshot.progress, 0.0);
        assert_eq!(snapshot.mouth_open, 1.0);
    }

    #[test]
    fn invalid_preset_and_quality_are_rejected() {
        assert!(set_voice_preset(-1, 0.5).is_err());
        assert!(set_quality(9).is_err());
    }
}
