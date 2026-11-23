# Dependencies (Documentation Index)

The canonical, version-pinned dependency matrix is maintained at:

**[../DEPENDENCIES.md](../DEPENDENCIES.md)**

Layer ownership summary:

| Layer | Main dependencies |
|---|---|
| Kotlin/Android UI | Kotlin 2.4.21, Compose BOM 2026.11.01, Material 3, Navigation 2.10.2, DataStore 1.3.0 |
| Android host | AGP 9.3.1, Gradle 9.7.0, GameActivity 4.4.0, compileSdk 37.1, NDK 28.2 |
| Rust core | Rust 1.100.0, `thiserror` 2.0.21, `crossbeam-channel` 0.5.17, `ringbuf` 0.6.0 |
| Bevy/3D | Bevy 0.20.1, `jni` 0.22.4, `android-activity` 0.6.1 |
| Audio | AAudio via `ndk` 0.9.0; `rubato` 5.1.0 if resampling is required; Oboe 1.11.0 fallback |

Do not add dependencies outside this matrix without updating the canonical file
with version, license, documentation, compatibility notes, and the alternatives
evaluated.
