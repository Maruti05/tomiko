use crate::errors::EngineError;

/// Device quality tier. The Kotlin layer derives this from RAM, Vulkan support,
/// and GPU class, then passes it to Rust/Bevy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i32)]
pub enum QualityTier {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl QualityTier {
    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Low),
            1 => Some(Self::Medium),
            2 => Some(Self::High),
            _ => None,
        }
    }

    /// Conservative tier selection. GPU capability must still be checked by
    /// the caller; RAM alone is not sufficient.
    #[must_use]
    pub fn from_total_ram_mb(total_ram_mb: u64) -> Self {
        if total_ram_mb < 3_072 {
            Self::Low
        } else if total_ram_mb < 6_144 {
            Self::Medium
        } else {
            Self::High
        }
    }

    #[must_use]
    pub fn render_scale(self) -> f32 {
        match self {
            Self::Low => 0.75,
            Self::Medium => 1.0,
            Self::High => 1.0,
        }
    }

    #[must_use]
    pub fn shadow_map_size(self) -> u32 {
        match self {
            Self::Low => 512,
            Self::Medium => 1_024,
            Self::High => 2_048,
        }
    }

    #[must_use]
    pub fn msaa_samples(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 4,
        }
    }

    #[must_use]
    pub fn target_fps(self) -> u32 {
        match self {
            Self::Low => 30,
            Self::Medium => 60,
            Self::High => 120,
        }
    }

    #[must_use]
    pub fn supports_ssao(self) -> bool {
        self >= Self::Medium
    }
}

/// Real-time audio configuration shared between Kotlin and Rust.
///
/// The defaults target AAudio low-latency operation at the device-native
/// 48 kHz rate. They are validated before any stream is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames_per_burst: u32,
    pub max_recording_ms: u32,
    pub silence_timeout_ms: u32,
    pub pre_roll_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 1,
            frames_per_burst: 192,
            max_recording_ms: 10_000,
            silence_timeout_ms: 800,
            pre_roll_ms: 300,
        }
    }
}

impl AudioConfig {
    pub fn validate(&self) -> Result<(), EngineError> {
        if !(8_000..=192_000).contains(&self.sample_rate) {
            return Err(EngineError::InvalidConfig(format!(
                "sample_rate {} is outside 8000..=192000",
                self.sample_rate
            )));
        }
        if !(1..=2).contains(&self.channels) {
            return Err(EngineError::InvalidConfig(format!(
                "channels {} is outside 1..=2",
                self.channels
            )));
        }
        if !(32..=4_096).contains(&self.frames_per_burst) {
            return Err(EngineError::InvalidConfig(format!(
                "frames_per_burst {} is outside 32..=4096",
                self.frames_per_burst
            )));
        }
        if !(1_000..=60_000).contains(&self.max_recording_ms) {
            return Err(EngineError::InvalidConfig(format!(
                "max_recording_ms {} is outside 1000..=60000",
                self.max_recording_ms
            )));
        }
        if !(100..=5_000).contains(&self.silence_timeout_ms) {
            return Err(EngineError::InvalidConfig(format!(
                "silence_timeout_ms {} is outside 100..=5000",
                self.silence_timeout_ms
            )));
        }
        if self.pre_roll_ms > 2_000 || self.pre_roll_ms >= self.max_recording_ms {
            return Err(EngineError::InvalidConfig(format!(
                "pre_roll_ms {} is invalid for max_recording_ms {}",
                self.pre_roll_ms, self.max_recording_ms
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn bytes_per_frame(&self) -> u32 {
        u32::from(self.channels) * 2
    }

    #[must_use]
    pub fn burst_bytes(&self) -> u32 {
        self.frames_per_burst * self.bytes_per_frame()
    }

    #[must_use]
    pub fn max_recording_frames(&self) -> u32 {
        self.sample_rate * self.max_recording_ms / 1_000
    }
}

/// Stable numeric IDs shared with Kotlin. Do not reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum VoicePreset {
    Normal = 0,
    Cartoon = 1,
    HighPitch = 2,
    Deep = 3,
    Robot = 4,
    Funny = 5,
}

impl VoicePreset {
    pub const ALL: [Self; 6] = [
        Self::Normal,
        Self::Cartoon,
        Self::HighPitch,
        Self::Deep,
        Self::Robot,
        Self::Funny,
    ];

    #[must_use]
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Normal),
            1 => Some(Self::Cartoon),
            2 => Some(Self::HighPitch),
            3 => Some(Self::Deep),
            4 => Some(Self::Robot),
            5 => Some(Self::Funny),
            _ => None,
        }
    }

    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Cartoon => "Cartoon",
            Self::HighPitch => "High Pitch",
            Self::Deep => "Deep Voice",
            Self::Robot => "Robot",
            Self::Funny => "Funny",
        }
    }
}

/// A stable identifier for a voice preset.
pub type VoicePresetId = i32;

/// Character metadata loaded at startup. Animation clip and morph indices are
/// resolved inside Bevy from the GLB, not hardcoded here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDescriptor {
    pub id: String,
    pub display_name: String,
    pub model_path: String,
    pub thumbnail_path: String,
    pub config_path: String,
}

impl CharacterDescriptor {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.id.trim().is_empty() {
            return Err(EngineError::InvalidConfig(
                "character id must not be empty".to_owned(),
            ));
        }
        if self.display_name.trim().is_empty() {
            return Err(EngineError::InvalidConfig(
                "character display_name must not be empty".to_owned(),
            ));
        }
        if !self.model_path.ends_with(".glb") {
            return Err(EngineError::InvalidConfig(format!(
                "character model_path must end in .glb: {}",
                self.model_path
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_tiers_are_monotonic() {
        assert_eq!(QualityTier::from_total_ram_mb(2_048), QualityTier::Low);
        assert_eq!(QualityTier::from_total_ram_mb(4_096), QualityTier::Medium);
        assert_eq!(QualityTier::from_total_ram_mb(8_192), QualityTier::High);
        assert!(QualityTier::High.shadow_map_size() > QualityTier::Low.shadow_map_size());
    }

    #[test]
    fn default_audio_config_is_valid() {
        let config = AudioConfig::default();
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.bytes_per_frame(), 2);
        assert_eq!(config.burst_bytes(), 384);
        assert_eq!(config.max_recording_frames(), 480_000);
    }

    #[test]
    fn invalid_audio_config_is_rejected() {
        let config = AudioConfig {
            sample_rate: 0,
            ..AudioConfig::default()
        };
        assert!(config.validate().is_err());

        let defaults = AudioConfig::default();
        let config = AudioConfig {
            pre_roll_ms: defaults.max_recording_ms,
            ..defaults
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn voice_preset_ids_round_trip() {
        for preset in VoicePreset::ALL {
            assert_eq!(VoicePreset::from_id(preset as i32), Some(preset));
        }
        assert_eq!(VoicePreset::from_id(-1), None);
        assert_eq!(VoicePreset::from_id(6), None);
    }

    #[test]
    fn character_descriptor_validation() {
        let descriptor = CharacterDescriptor {
            id: "tomiko".to_owned(),
            display_name: "Tomiko".to_owned(),
            model_path: "characters/tomiko/character.glb".to_owned(),
            thumbnail_path: "characters/tomiko/thumbnail.webp".to_owned(),
            config_path: "characters/tomiko/metadata.json".to_owned(),
        };
        assert_eq!(descriptor.validate(), Ok(()));

        let mut invalid = descriptor;
        invalid.model_path = "characters/tomiko/character.gltf".to_owned();
        assert!(invalid.validate().is_err());
    }
}
