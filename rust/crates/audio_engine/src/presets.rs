use shared_types::{EngineError, VoicePreset};

/// Parameters consumed by the Rust DSP chain. Ratios are linear; 1.0 is neutral.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoicePresetParameters {
    pub pitch_ratio: f32,
    pub formant_ratio: f32,
    pub reverb_mix: f32,
    pub robot_mix: f32,
    pub echo_delay_ms: f32,
    pub low_pass_hz: f32,
    pub gain: f32,
}

impl VoicePresetParameters {
    pub fn validate(&self) -> Result<(), EngineError> {
        if !(0.5..=2.0).contains(&self.pitch_ratio) {
            return Err(EngineError::InvalidConfig(format!(
                "pitch_ratio {} is outside 0.5..=2.0",
                self.pitch_ratio
            )));
        }
        if !(0.5..=2.0).contains(&self.formant_ratio) {
            return Err(EngineError::InvalidConfig(format!(
                "formant_ratio {} is outside 0.5..=2.0",
                self.formant_ratio
            )));
        }
        if !(0.0..=1.0).contains(&self.reverb_mix) {
            return Err(EngineError::InvalidConfig(format!(
                "reverb_mix {} is outside 0.0..=1.0",
                self.reverb_mix
            )));
        }
        if !(0.0..=1.0).contains(&self.robot_mix) {
            return Err(EngineError::InvalidConfig(format!(
                "robot_mix {} is outside 0.0..=1.0",
                self.robot_mix
            )));
        }
        if !(0.0..=500.0).contains(&self.echo_delay_ms) {
            return Err(EngineError::InvalidConfig(format!(
                "echo_delay_ms {} is outside 0..=500",
                self.echo_delay_ms
            )));
        }
        if !(2_000.0..=20_000.0).contains(&self.low_pass_hz) {
            return Err(EngineError::InvalidConfig(format!(
                "low_pass_hz {} is outside 2000..=20000",
                self.low_pass_hz
            )));
        }
        if !(0.0..=2.0).contains(&self.gain) {
            return Err(EngineError::InvalidConfig(format!(
                "gain {} is outside 0.0..=2.0",
                self.gain
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn is_neutral(&self) -> bool {
        (self.pitch_ratio - 1.0).abs() < f32::EPSILON
            && (self.formant_ratio - 1.0).abs() < f32::EPSILON
            && self.reverb_mix == 0.0
            && self.robot_mix == 0.0
            && self.echo_delay_ms == 0.0
            && (self.low_pass_hz - 20_000.0).abs() < f32::EPSILON
            && (self.gain - 1.0).abs() < f32::EPSILON
    }
}

/// Preset values are intentionally conservative. Extreme pitch/formant shifts
/// degrade intelligibility and produce aliasing on mobile speakers.
#[must_use]
pub fn parameters_for(preset: VoicePreset) -> VoicePresetParameters {
    match preset {
        VoicePreset::Normal => VoicePresetParameters {
            pitch_ratio: 1.0,
            formant_ratio: 1.0,
            reverb_mix: 0.0,
            robot_mix: 0.0,
            echo_delay_ms: 0.0,
            low_pass_hz: 20_000.0,
            gain: 1.0,
        },
        VoicePreset::Cartoon => VoicePresetParameters {
            pitch_ratio: 1.45,
            formant_ratio: 1.25,
            reverb_mix: 0.05,
            robot_mix: 0.0,
            echo_delay_ms: 0.0,
            low_pass_hz: 12_000.0,
            gain: 0.95,
        },
        VoicePreset::HighPitch => VoicePresetParameters {
            pitch_ratio: 1.8,
            formant_ratio: 1.0,
            reverb_mix: 0.0,
            robot_mix: 0.0,
            echo_delay_ms: 0.0,
            low_pass_hz: 18_000.0,
            gain: 0.9,
        },
        VoicePreset::Deep => VoicePresetParameters {
            pitch_ratio: 0.65,
            formant_ratio: 0.75,
            reverb_mix: 0.05,
            robot_mix: 0.0,
            echo_delay_ms: 0.0,
            low_pass_hz: 8_000.0,
            gain: 1.0,
        },
        VoicePreset::Robot => VoicePresetParameters {
            pitch_ratio: 1.0,
            formant_ratio: 1.0,
            reverb_mix: 0.0,
            robot_mix: 0.65,
            echo_delay_ms: 0.0,
            low_pass_hz: 6_000.0,
            gain: 0.95,
        },
        VoicePreset::Funny => VoicePresetParameters {
            pitch_ratio: 1.25,
            formant_ratio: 0.85,
            reverb_mix: 0.12,
            robot_mix: 0.15,
            echo_delay_ms: 120.0,
            low_pass_hz: 10_000.0,
            gain: 0.95,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_valid() {
        for preset in VoicePreset::ALL {
            let parameters = parameters_for(preset);
            assert_eq!(
                parameters.validate(),
                Ok(()),
                "invalid preset {}",
                preset.display_name()
            );
        }
    }

    #[test]
    fn normal_preset_is_neutral() {
        assert!(parameters_for(VoicePreset::Normal).is_neutral());
    }

    #[test]
    fn presets_are_distinct() {
        let mut seen = Vec::new();
        for preset in VoicePreset::ALL {
            let parameters = parameters_for(preset);
            assert!(
                !seen.contains(&parameters),
                "duplicate parameters for {}",
                preset.display_name()
            );
            seen.push(parameters);
        }
    }

    #[test]
    fn extreme_pitch_is_rejected() {
        let parameters = VoicePresetParameters {
            pitch_ratio: 3.0,
            ..parameters_for(VoicePreset::Normal)
        };
        assert!(parameters.validate().is_err());
    }
}
