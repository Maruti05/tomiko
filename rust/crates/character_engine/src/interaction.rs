use shared_types::{CharacterState, TouchZone};

/// Emotional reaction associated with a touch region.
#[must_use]
pub fn reaction_for_zone(zone: TouchZone) -> Option<CharacterState> {
    match zone {
        TouchZone::Head | TouchZone::LeftEar | TouchZone::RightEar => {
            Some(CharacterState::Surprised)
        }
        TouchZone::Body | TouchZone::Belly => Some(CharacterState::Happy),
        TouchZone::Background => None,
    }
}

/// Whether a touch should be treated as affectionate feedback.
#[must_use]
pub fn is_affectionate(zone: TouchZone) -> bool {
    matches!(zone, TouchZone::Body | TouchZone::Belly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_is_not_a_reaction() {
        assert_eq!(reaction_for_zone(TouchZone::Background), None);
        assert!(!is_affectionate(TouchZone::Background));
    }

    #[test]
    fn body_is_affectionate() {
        assert!(is_affectionate(TouchZone::Body));
        assert!(is_affectionate(TouchZone::Belly));
        assert!(!is_affectionate(TouchZone::Head));
    }
}
