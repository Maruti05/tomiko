# Tomiko

An offline, privacy-first Android talking-character app built with Kotlin,
Jetpack Compose, Rust, and Bevy.

**Current status:** Phases 1–6 are implemented; Phase 7/8 remain. An original
rigged Tomiko GLB (27 joints, 15 visemes, 11 clips) runs in a Bevy GameActivity
with animation transitions, blink/talk morphs, AAudio capture/playback, VAD,
voice DSP, and audio-driven lip-sync. Compose provides the launcher/home and
settings; Bevy provides the in-stage HUD. Release builds use R8 and shrink to
about **68 MB**; final 60 FPS/thermal validation is still open.

## Stack

| Layer | Technology |
|---|---|
| Android UI | Kotlin 2.4.21, Jetpack Compose (BOM 2026.11.01), Material 3 |
| Build | Gradle 9.7.0, AGP 9.3.1, JDK 21, compileSdk 37.1, minSdk 30 |
| Native core | Rust 1.100.0, JNI 0.22.4, AAudio |
| 3D | Bevy 0.20.1, GameActivity 4.4.0 |
| Audio | Rust DSP + AAudio; Oboe 1.11.0 is the documented fallback |

## Quick start

```powershell
# 1. Verify the machine
.\scripts\verify_environment.ps1

# 2. Build the debug Android app
.\gradlew.bat :app:assembleDebug

# 3. Run Rust unit tests
Push-Location .\rust
cargo test --workspace
Pop-Location

# 4. Build Rust + Android
.\scripts\build_android.ps1 -Mode debug

# 5. Install and launch the Bevy GameActivity on a device
.\scripts\run_debug.ps1

# 6. Validate the character GLB
.\scripts\validate_character.ps1

# 7. Build a signed release (set these first)
$env:TOMIKO_KEYSTORE = "C:\path\to\release.jks"
$env:TOMIKO_KEYSTORE_PASSWORD = "..."
$env:TOMIKO_KEY_ALIAS = "..."
$env:TOMIKO_KEY_PASSWORD = "..."
.\gradlew.bat :app:assembleRelease
```

## Required environment

- JDK 21
- Android SDK platform **android-37.1**
- Android NDK **28.2.13676358**
- Rust target `aarch64-linux-android`
- `cargo-ndk` 4.1.2
- An arm64 Android device with Vulkan support for real verification

See [docs/BUILD.md](docs/BUILD.md) for full setup and troubleshooting.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Dependencies](DEPENDENCIES.md)
- [Build guide](docs/BUILD.md)
- [Threading model](docs/THREADING_MODEL.md)
- [Audio pipeline](docs/AUDIO_PIPELINE.md)
- [Android integration](docs/ANDROID_INTEGRATION.md)
- [Performance plan](docs/PERFORMANCE.md)
- [Character pipeline](docs/CHARACTER_PIPELINE.md)
- [Phase 3 device test plan](docs/PHASE3_TEST_PLAN.md)
- [Phase 4 device test plan](docs/PHASE4_TEST_PLAN.md)
- [Implementation status](docs/IMPLEMENTATION_STATUS.md)

## Known blockers

- **No Android device is currently connected.** Audio, animation sync, FPS,
  lifecycle, and thermal behavior cannot be verified until one is available.
- The first original character asset is generated; visual refinement and
  on-device animation/morph validation remain.
- The original PRD’s SceneView/Filament, 4-clip, 5k-triangle, and 44.1 kHz
  assumptions are superseded by the documents above.

## Privacy

The app is offline-only: no accounts, no network, no audio uploads, no
analytics. Microphone buffers stay in memory and are discarded after playback
unless a debug build explicitly enables diagnostics.
