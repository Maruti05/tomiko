//! Bevy Android application entry point.
//!
//! The cdylib is loaded by `CharacterGameActivity`. `#[bevy_main]` generates
//! the Android `android_main` boilerplate expected by `android-activity`.

mod app;
mod jni;

use bevy::prelude::*;

#[bevy_main]
pub fn main() {
    // Kotlin may call nativeInitialize() before or after the Bevy thread
    // starts. Initialization is idempotent so the command receiver is always
    // available by the time Bevy's startup systems run.
    android_bridge::ensure_initialized();
    app::run();
}
