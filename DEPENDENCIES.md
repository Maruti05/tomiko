# Dependency Matrix

**Status:** Phase 5 implementation. Versions were checked against official
release metadata on **22 November 2026**.

## Version policy

- Use the latest **stable** versions that are mutually compatible.
- Do not use alpha/beta APIs in production-critical paths.
- Pin native toolchains (NDK, Rust target, cargo-ndk) because cross-compilation
  behavior is part of the build contract.
- AGP 9.5.2 and Gradle 9.9.2 are newer than the selected AGP/Gradle versions,
  but Kotlin 2.4.21 is officially tested only through **AGP 9.3.1** and
  **Gradle 9.7.0**. The selected set is the newest fully supported combination.
  The documented fallback is AGP 8.13.2 + Gradle 8.14.3 + Kotlin 2.4.21.

## Build toolchain

| Tool | Version | Purpose | Layer | License | Official documentation | Notes |
|---|---|---|---|---|---|---|
| JDK | 21.0.13.9 (Temurin) | Build/runtime | Toolchain | GPLv2+CE | https://adoptium.net/ | AGP 9.3 requires JDK 17+ |
| Gradle | 9.7.0 | Build system | Toolchain | Apache-2.0 | https://docs.gradle.org/9.7.0/ | Latest inside Kotlin 2.4.21 tested range |
| Android Gradle Plugin | 9.3.1 | Android build | Toolchain | Apache-2.0 | https://developer.android.com/build/releases/gradle-plugin | Built-in Kotlin enabled |
| Kotlin | 2.4.21 | Kotlin compiler | Toolchain | Apache-2.0 | https://kotlinlang.org/docs/releases.html | Latest stable |
| Compose compiler plugin | 2.4.21 | Compose compiler | Toolchain | Apache-2.0 | https://developer.android.com/develop/ui/compose/compiler | Must match Kotlin |
| Android SDK platform | 37.1 | Compile SDK | Toolchain | Android SDK license | https://developer.android.com/studio/releases/platforms | Required by Compose 1.13.0 |
| Android NDK | 28.2.13676358 (r28c) | Native toolchain | Toolchain | Apache-2.0 | https://developer.android.com/ndk/downloads | AGP 9.3 default; 16 KB page ready |
| Rust | 1.100.0 | Native compiler | Toolchain | MIT/Apache-2.0 | https://www.rust-lang.org/ | Bevy 0.20.1 MSRV is 1.97.1 |
| cargo-ndk | 4.1.2 | Android cross-build | Toolchain | Apache-2.0/MIT | https://github.com/bbqsrc/cargo-ndk | Installed |

## Android/Kotlin libraries

| Library | Version | Purpose | Layer | License | Official documentation | Notes |
|---|---|---|---|---|---|---|
| Compose BOM | 2026.11.01 | Compose alignment | UI | Apache-2.0 | https://developer.android.com/develop/ui/compose/bom/bom-mapping | Maps Compose UI/Foundation/Animation to 1.13.0 |
| Compose Material 3 | via BOM | Design system | UI | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/compose-material3 | |
| Compose UI/Foundation/Animation | via BOM | UI primitives | UI | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/compose | |
| Material Icons Core | via BOM | Small icon set | UI | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/compose-material | Avoid `material-icons-extended` |
| Activity Compose | 1.13.0 | Compose activity | UI | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/activity | |
| Core KTX | 1.19.1 | Android extensions | Core | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/core | |
| Lifecycle | 2.11.1 | ViewModel/lifecycle | Core | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/lifecycle | |
| Navigation Compose | 2.10.2 | Navigation | UI | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/navigation | |
| DataStore Preferences | 1.3.0 | Preferences | Data | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/datastore | Replaces JSON preferences |
| Window | 1.5.1 | Adaptive layouts | UI | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/window | |
| Kotlin Coroutines | 1.11.0 | Async/concurrency | Core | Apache-2.0 | https://github.com/Kotlin/kotlinx.coroutines/releases | |
| Kotlin Serialization JSON | 1.13.0 | Metadata/config | Data | Apache-2.0 | https://github.com/Kotlin/kotlinx.serialization/releases | Startup only, not hot paths |
| GameActivity | 4.4.0 | Native activity host | Android | Apache-2.0 | https://developer.android.com/games/agdk/game-activity | Must match `android-activity` 0.6.1 |
| SceneView | 0.10.2 | Removed | 3D (legacy) | Apache-2.0 | https://github.com/SceneView/sceneview-android | Removed in Phase 8; the Bevy GameActivity is the only 3D path |
| JUnit | 4.13.2 | Unit tests | Test | EPL-1.0 | https://junit.org/junit4/ | |
| Turbine | 1.2.1 | Flow tests | Test | Apache-2.0 | https://github.com/cashapp/turbine | |
| AndroidX Test Runner | 1.7.0 | Instrumentation | Test | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/test | |
| AndroidX Test Ext JUnit | 1.3.0 | Instrumentation | Test | Apache-2.0 | https://developer.android.com/jetpack/androidx/releases/test | |

## Rust/Bevy crates

| Crate | Version | Purpose | Layer | License | Official documentation | Notes |
|---|---|---|---|---|---|---|
| Bevy | 0.20.1 | 3D rendering/animation/ECS | Bevy | MIT/Apache-2.0 | https://docs.rs/bevy/0.20.1 | Latest stable; Android GameActivity support |
| `android-activity` | 0.6.1 | GameActivity glue | Native | MIT/Apache-2.0 | https://github.com/rust-mobile/android-activity | Transitive via Bevy; pin in lockfile |
| `jni` | 0.22.4 | JNI bridge | Bridge | MIT/Apache-2.0 | https://docs.rs/jni/0.22.4 | Matches `android-activity` 0.6.1 |
| `wgpu-hal` | 30.0.2 (vendored) | Vulkan/GLES HAL | Render | MIT/Apache-2.0 | https://github.com/gfx-rs/wgpu/pull/10264 | Local patch for Mali `VK_ERROR_FRAGMENTED_POOL` panic; drop when a release includes it |
| `bevy_render` | 0.20.1 (vendored) | Render core | Render | MIT/Apache-2.0 | https://github.com/bevyengine/bevy/issues/24926 | Forces uniform-buffer skinning + image morph targets on Android for Adreno; drop when fixed upstream |
| `log` | 0.4.35 | Structured logging | Core | MIT/Apache-2.0 | https://docs.rs/log | Shared with Bevy's logger; no separate initialization |
| `ndk` | 0.9.0 | NDK/AAudio bindings | Audio/Android | MIT/Apache-2.0 | https://docs.rs/ndk/0.9.0 | `audio`, `api-level-28`, `api-level-30` features; Android-only dependency |
| `rubato` | 5.1.0 | Resampling | Audio | MIT/Apache-2.0 | https://github.com/HEnquist/rubato | Only if device rate differs |
| `ringbuf` | 0.6.0 | Lock-free SPSC buffers | Audio | MIT/Apache-2.0 | https://docs.rs/ringbuf/0.6.0 | Capture/playback rings |
| `crossbeam-channel` | 0.5.17 | Bounded commands/events | Core | MIT/Apache-2.0 | https://docs.rs/crossbeam-channel/0.5.17 | No unbounded queues |
| `thiserror` | 2.0.21 | Error types | Core | MIT/Apache-2.0 | https://docs.rs/thiserror/2.0.21 | |
| `serde` | 1.0.229 | Config deserialization | Config | MIT/Apache-2.0 | https://docs.rs/serde/1.0.229 | Startup only |
| `serde_json` | 1.0.154 | Config JSON | Config | MIT/Apache-2.0 | https://docs.rs/serde_json/1.0.154 | Startup only |
| Oboe | 1.11.0 | Audio fallback | Audio | Apache-2.0 | https://github.com/google/oboe/releases/tag/1.11.0 | Not linked unless AAudio fails device validation |

## Bevy feature set

```toml
bevy = { version = "0.20.1", default-features = false, features = [
  "3d", "ui",
  "bevy_winit", "android-game-activity",
  "multi_threaded", "std", "default_font",
] }
```

- `3d` includes `bevy_gltf`, `bevy_pbr`, `gltf_animation`, `morph_animation`,
  `ktx2`, `zstd_rust`, `bevy_animation`, and `bevy_light`.
- `bevy_audio` is intentionally disabled; audio is owned by `audio_engine`.
- `x11`, `wayland`, `bevy_gilrs`, `file_watcher`, and HTTP features are disabled.
- `basis-universal` was removed in Phase 7; the character pipeline does not use
  Basis/KTX2 yet.
- Android builds link at platform 30 so `-laaudio` resolves (AAudio is API 26+).

## Alternatives evaluated

| Alternative | Decision | Reason |
|---|---|---|
| SceneView/Filament | Rejected for final | Existing animation/morph pipeline is stubbed; brief mandates Bevy |
| Unity/Unreal | Rejected | APK size, runtime, licensing |
| Godot | Rejected | Heavy runtime; wrong architecture |
| Hand-written GLES | Rejected | No animation/morph pipeline; high risk |
| SoundTouch C++ | Rejected initially | Adds C++ DSP dependency; Rust DSP is cleaner |
| Oboe | Fallback | Strong library, but AAudio is directly available at minSdk 30; add only if needed |
| Kotlin AudioRecord/AudioTrack | Rejected as owner | Duplicate ownership/per-buffer JNI |
| Hilt/Dagger | Rejected | Build/APK overhead |
| Room | Rejected | No relational data |
| Coil/Glide | Rejected | Local thumbnails; unnecessary dependency |

## Security and licensing rules

- All runtime dependencies must be offline-compatible.
- No analytics, ad, crash-reporting, or cloud SDKs.
- Do not ship assets with unverified licensing.
- Record exact versions in `gradle/libs.versions.toml` and `rust/Cargo.lock`.
- Run `cargo deny`/license review before release (Phase 8).
