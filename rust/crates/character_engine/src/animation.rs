use shared_types::{CharacterState, TouchZone};

/// Animation clips expected in a character GLB. The Bevy loader resolves these
/// by name; missing clips are reported as asset errors rather than silently
/// falling back to arbitrary clip indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationClip {
    Idle,
    Listening,
    Talking,
    Happy,
    Surprised,
    Sad,
    Angry,
    Sleeping,
    Blink,
    TouchHead,
    TouchBody,
}

impl AnimationClip {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Listening => "listening",
            Self::Talking => "talking",
            Self::Happy => "happy",
            Self::Surprised => "surprised",
            Self::Sad => "sad",
            Self::Angry => "angry",
            Self::Sleeping => "sleeping",
            Self::Blink => "blink",
            Self::TouchHead => "touch_head",
            Self::TouchBody => "touch_body",
        }
    }
}

#[must_use]
pub fn clip_for_state(state: CharacterState) -> AnimationClip {
    match state {
        CharacterState::Idle => AnimationClip::Idle,
        CharacterState::Listening => AnimationClip::Listening,
        CharacterState::Talking => AnimationClip::Talking,
        CharacterState::Happy => AnimationClip::Happy,
        CharacterState::Surprised => AnimationClip::Surprised,
        CharacterState::Sad => AnimationClip::Sad,
        CharacterState::Angry => AnimationClip::Angry,
        CharacterState::Sleeping => AnimationClip::Sleeping,
        CharacterState::Blinking => AnimationClip::Blink,
        CharacterState::TouchReaction => AnimationClip::TouchBody,
    }
}

#[must_use]
pub fn touch_clip(zone: TouchZone) -> Option<AnimationClip> {
    match zone {
        TouchZone::Head | TouchZone::LeftEar | TouchZone::RightEar => {
            Some(AnimationClip::TouchHead)
        }
        TouchZone::Body | TouchZone::Belly => Some(AnimationClip::TouchBody),
        TouchZone::Background => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_state_has_a_clip() {
        for id in 0..=9 {
            let state = CharacterState::from_id(id).expect("state");
            assert!(!clip_for_state(state).name().is_empty());
        }
    }

    #[test]
    fn touch_zones_map_to_clips() {
        assert_eq!(touch_clip(TouchZone::Head), Some(AnimationClip::TouchHead));
        assert_eq!(touch_clip(TouchZone::Belly), Some(AnimationClip::TouchBody));
        assert_eq!(touch_clip(TouchZone::Background), None);
    }
}
