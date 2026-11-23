use shared_types::{AnimationEvent, CharacterState, EngineError, TouchZone};

use crate::configuration::CharacterBehaviorConfig;

/// Owns the character presentation state. This is intentionally independent of
/// Bevy: the renderer consumes `AnimationEvent`s and never decides state itself.
#[derive(Debug, Clone)]
pub struct CharacterStateMachine {
    state: CharacterState,
    config: CharacterBehaviorConfig,
    idle_elapsed_ms: u32,
    blink_elapsed_ms: u32,
    blink_active_ms: u32,
    reaction_elapsed_ms: u32,
}

impl Default for CharacterStateMachine {
    fn default() -> Self {
        Self::new(CharacterBehaviorConfig::default()).expect("default character config is valid")
    }
}

impl CharacterStateMachine {
    pub fn new(config: CharacterBehaviorConfig) -> Result<Self, EngineError> {
        config.validate()?;
        Ok(Self {
            state: CharacterState::Idle,
            config,
            idle_elapsed_ms: 0,
            blink_elapsed_ms: 0,
            blink_active_ms: 0,
            reaction_elapsed_ms: 0,
        })
    }

    #[must_use]
    pub fn state(&self) -> CharacterState {
        self.state
    }

    #[must_use]
    pub fn config(&self) -> CharacterBehaviorConfig {
        self.config
    }

    /// Returns true when the state machine can move from `from` to `to`.
    #[must_use]
    pub fn is_valid_transition(from: CharacterState, to: CharacterState) -> bool {
        if from == to {
            return true;
        }

        // Core states must always be reachable so the character responds
        // immediately to speech, playback, and touch.
        if matches!(
            to,
            CharacterState::Idle
                | CharacterState::Listening
                | CharacterState::Talking
                | CharacterState::TouchReaction
        ) {
            return true;
        }

        matches!(
            (from, to),
            (
                CharacterState::Idle,
                CharacterState::Sleeping
                    | CharacterState::Blinking
                    | CharacterState::Happy
                    | CharacterState::Surprised
                    | CharacterState::Sad
                    | CharacterState::Angry
            ) | (
                CharacterState::Listening,
                CharacterState::Happy
                    | CharacterState::Surprised
                    | CharacterState::Sad
                    | CharacterState::Angry
            ) | (
                CharacterState::Talking,
                CharacterState::Happy
                    | CharacterState::Surprised
                    | CharacterState::Sad
                    | CharacterState::Angry
            ) | (CharacterState::Happy, CharacterState::Surprised)
                | (CharacterState::Surprised, CharacterState::Happy)
                | (CharacterState::Sad, CharacterState::Happy)
                | (CharacterState::Sleeping, CharacterState::Surprised)
                | (
                    CharacterState::TouchReaction,
                    CharacterState::Happy | CharacterState::Surprised | CharacterState::Angry
                )
        )
    }

    pub fn transition_to(
        &mut self,
        next: CharacterState,
        fade_ms: u32,
    ) -> Result<Option<AnimationEvent>, EngineError> {
        if next == self.state {
            return Ok(None);
        }
        if !Self::is_valid_transition(self.state, next) {
            return Err(EngineError::InvalidTransition(
                "target state is not reachable from the current state",
            ));
        }
        self.enter(next);
        Ok(Some(AnimationEvent::Play {
            state: next,
            fade_ms,
        }))
    }

    pub fn on_speech_start(&mut self) -> Result<Option<AnimationEvent>, EngineError> {
        self.transition_to(CharacterState::Listening, 150)
    }

    pub fn on_playback_start(&mut self) -> Result<Option<AnimationEvent>, EngineError> {
        self.transition_to(CharacterState::Talking, self.config.talk_fade_ms)
    }

    pub fn on_playback_end(&mut self) -> Result<Option<AnimationEvent>, EngineError> {
        self.transition_to(CharacterState::Happy, 200)
    }

    pub fn on_sleep(&mut self) -> Result<Option<AnimationEvent>, EngineError> {
        self.transition_to(CharacterState::Sleeping, 300)
    }

    /// Touch reactions are deliberately immediate. The zone is carried in the
    /// event so Bevy can choose a head/body reaction clip.
    pub fn on_touch(&mut self, zone: TouchZone) -> Option<AnimationEvent> {
        if zone == TouchZone::Background {
            return None;
        }
        self.enter(CharacterState::TouchReaction);
        Some(AnimationEvent::TouchReaction {
            zone,
            fade_ms: self.config.touch_fade_ms,
        })
    }

    /// Called by Bevy when a one-shot reaction clip finishes.
    pub fn on_animation_finished(&mut self) -> Result<Option<AnimationEvent>, EngineError> {
        if matches!(
            self.state,
            CharacterState::Blinking
                | CharacterState::Happy
                | CharacterState::Surprised
                | CharacterState::Sad
                | CharacterState::Angry
                | CharacterState::TouchReaction
        ) {
            self.transition_to(CharacterState::Idle, 180)
        } else {
            Ok(None)
        }
    }

    /// Advances timers. Call once per rendered frame with a clamped delta.
    pub fn tick(&mut self, delta_ms: u32) -> Result<Option<AnimationEvent>, EngineError> {
        if delta_ms == 0 {
            return Ok(None);
        }

        match self.state {
            CharacterState::Idle => {
                self.idle_elapsed_ms = self.idle_elapsed_ms.saturating_add(delta_ms);
                if self.idle_elapsed_ms >= self.config.sleeping_after_ms {
                    self.enter(CharacterState::Sleeping);
                    return Ok(Some(AnimationEvent::Play {
                        state: CharacterState::Sleeping,
                        fade_ms: 300,
                    }));
                }

                self.blink_elapsed_ms = self.blink_elapsed_ms.saturating_add(delta_ms);
                if self.blink_elapsed_ms >= self.config.blink_interval_ms {
                    self.enter(CharacterState::Blinking);
                    return Ok(Some(AnimationEvent::Blink));
                }

                Ok(None)
            }
            CharacterState::Blinking => {
                self.blink_active_ms = self.blink_active_ms.saturating_add(delta_ms);
                if self.blink_active_ms >= self.config.blink_duration_ms {
                    self.enter(CharacterState::Idle);
                    Ok(Some(AnimationEvent::Play {
                        state: CharacterState::Idle,
                        fade_ms: 80,
                    }))
                } else {
                    Ok(None)
                }
            }
            CharacterState::Happy
            | CharacterState::Surprised
            | CharacterState::Sad
            | CharacterState::Angry
            | CharacterState::TouchReaction => {
                self.reaction_elapsed_ms = self.reaction_elapsed_ms.saturating_add(delta_ms);
                if self.reaction_elapsed_ms >= self.config.reaction_duration_ms {
                    self.enter(CharacterState::Idle);
                    Ok(Some(AnimationEvent::Play {
                        state: CharacterState::Idle,
                        fade_ms: 180,
                    }))
                } else {
                    Ok(None)
                }
            }
            CharacterState::Listening | CharacterState::Talking | CharacterState::Sleeping => {
                Ok(None)
            }
        }
    }

    fn enter(&mut self, next: CharacterState) {
        self.state = next;
        self.idle_elapsed_ms = 0;
        self.blink_elapsed_ms = 0;
        self.blink_active_ms = 0;
        self.reaction_elapsed_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short_config() -> CharacterBehaviorConfig {
        CharacterBehaviorConfig {
            sleeping_after_ms: 1_000,
            blink_interval_ms: 100,
            blink_duration_ms: 20,
            reaction_duration_ms: 50,
            talk_fade_ms: 150,
            touch_fade_ms: 120,
        }
    }

    fn machine() -> CharacterStateMachine {
        CharacterStateMachine::new(short_config()).expect("valid config")
    }

    #[test]
    fn starts_idle() {
        assert_eq!(machine().state(), CharacterState::Idle);
    }

    #[test]
    fn speech_and_playback_flow() {
        let mut machine = machine();
        assert!(matches!(
            machine.on_speech_start().unwrap(),
            Some(AnimationEvent::Play {
                state: CharacterState::Listening,
                ..
            })
        ));
        assert!(matches!(
            machine.on_playback_start().unwrap(),
            Some(AnimationEvent::Play {
                state: CharacterState::Talking,
                ..
            })
        ));
        assert!(matches!(
            machine.on_playback_end().unwrap(),
            Some(AnimationEvent::Play {
                state: CharacterState::Happy,
                ..
            })
        ));
        assert_eq!(machine.state(), CharacterState::Happy);
        machine.tick(50).unwrap();
        assert_eq!(machine.state(), CharacterState::Idle);
    }

    #[test]
    fn touch_reaction_is_immediate_and_returns_to_idle() {
        let mut machine = machine();
        assert!(matches!(
            machine.on_touch(TouchZone::Head),
            Some(AnimationEvent::TouchReaction {
                zone: TouchZone::Head,
                ..
            })
        ));
        assert_eq!(machine.state(), CharacterState::TouchReaction);
        machine.tick(50).unwrap();
        assert_eq!(machine.state(), CharacterState::Idle);
    }

    #[test]
    fn background_touch_is_ignored() {
        let mut machine = machine();
        assert_eq!(machine.on_touch(TouchZone::Background), None);
        assert_eq!(machine.state(), CharacterState::Idle);
    }

    #[test]
    fn blink_cycle_runs() {
        let mut machine = machine();
        assert_eq!(machine.tick(99).unwrap(), None);
        assert!(matches!(
            machine.tick(1).unwrap(),
            Some(AnimationEvent::Blink)
        ));
        assert_eq!(machine.state(), CharacterState::Blinking);

        assert_eq!(machine.tick(19).unwrap(), None);
        assert!(matches!(
            machine.tick(1).unwrap(),
            Some(AnimationEvent::Play {
                state: CharacterState::Idle,
                ..
            })
        ));
        assert_eq!(machine.state(), CharacterState::Idle);
    }

    #[test]
    fn sleeps_after_idle_timeout() {
        let mut machine = machine();
        machine.tick(1_000).unwrap();
        assert_eq!(machine.state(), CharacterState::Sleeping);

        // Touching a sleeping character wakes it immediately.
        assert!(machine.on_touch(TouchZone::Body).is_some());
        assert_eq!(machine.state(), CharacterState::TouchReaction);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut machine = machine();
        machine
            .transition_to(CharacterState::Happy, 100)
            .expect("idle -> happy is valid");
        let error = machine
            .transition_to(CharacterState::Sleeping, 100)
            .expect_err("happy -> sleeping is invalid");
        assert!(matches!(error, EngineError::InvalidTransition(_)));
    }

    #[test]
    fn same_state_is_a_no_op() {
        let mut machine = machine();
        assert_eq!(
            machine.transition_to(CharacterState::Idle, 100).unwrap(),
            None
        );
    }

    #[test]
    fn animation_finished_returns_reaction_to_idle() {
        let mut machine = machine();
        machine
            .transition_to(CharacterState::Surprised, 100)
            .unwrap();
        assert!(matches!(
            machine.on_animation_finished().unwrap(),
            Some(AnimationEvent::Play {
                state: CharacterState::Idle,
                ..
            })
        ));
    }
}
