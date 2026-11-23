use shared_types::EngineError;

/// Timings that control idle behavior, blinking, and touch reactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterBehaviorConfig {
    pub sleeping_after_ms: u32,
    pub blink_interval_ms: u32,
    pub blink_duration_ms: u32,
    pub reaction_duration_ms: u32,
    pub talk_fade_ms: u32,
    pub touch_fade_ms: u32,
}

impl Default for CharacterBehaviorConfig {
    fn default() -> Self {
        Self {
            sleeping_after_ms: 25_000,
            blink_interval_ms: 4_500,
            blink_duration_ms: 140,
            reaction_duration_ms: 1_800,
            talk_fade_ms: 150,
            touch_fade_ms: 120,
        }
    }
}

impl CharacterBehaviorConfig {
    pub fn validate(&self) -> Result<(), EngineError> {
        if self.blink_duration_ms == 0 || self.blink_duration_ms > 1_000 {
            return Err(EngineError::InvalidConfig(format!(
                "blink_duration_ms {} is outside 1..=1000",
                self.blink_duration_ms
            )));
        }
        if self.blink_interval_ms <= self.blink_duration_ms {
            return Err(EngineError::InvalidConfig(
                "blink_interval_ms must be greater than blink_duration_ms".to_owned(),
            ));
        }
        if self.reaction_duration_ms == 0 || self.reaction_duration_ms > 10_000 {
            return Err(EngineError::InvalidConfig(format!(
                "reaction_duration_ms {} is outside 1..=10000",
                self.reaction_duration_ms
            )));
        }
        if self.sleeping_after_ms < self.reaction_duration_ms {
            return Err(EngineError::InvalidConfig(
                "sleeping_after_ms must be at least reaction_duration_ms".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert_eq!(CharacterBehaviorConfig::default().validate(), Ok(()));
    }

    #[test]
    fn invalid_config_is_rejected() {
        let defaults = CharacterBehaviorConfig::default();
        let config = CharacterBehaviorConfig {
            blink_interval_ms: defaults.blink_duration_ms,
            ..defaults
        };
        assert!(config.validate().is_err());

        let config = CharacterBehaviorConfig {
            reaction_duration_ms: 0,
            ..CharacterBehaviorConfig::default()
        };
        assert!(config.validate().is_err());
    }
}
