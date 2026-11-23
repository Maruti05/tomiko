//! Character state machine and animation event generation.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod animation;
pub mod configuration;
pub mod interaction;
pub mod state;

pub use animation::{AnimationClip, clip_for_state, touch_clip};
pub use configuration::CharacterBehaviorConfig;
pub use interaction::{is_affectionate, reaction_for_zone};
pub use state::CharacterStateMachine;

pub const CHARACTER_ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
