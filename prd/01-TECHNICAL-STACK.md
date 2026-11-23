# Tomiko — Technical Stack Specification

> **Superseded (November 2026):** See `DEPENDENCIES.md` and
> `docs/ARCHITECTURE.md`. The app now uses AGP 9.3.1, Kotlin 2.4.21,
> Compose BOM 2026.11.01, compileSdk 37.1, Rust 1.100.0, Bevy 0.20.1, and
> GameActivity 4.4.0. SceneView/Filament is a temporary migration fallback only.

## 1. Language & Runtime

| Property | Choice | Rationale |
|---|---|---|
| Language | **Kotlin 2.1.x** | First-class Android support, coroutines, null safety |
| JVM target | **JVM 17** | Required by latest AGP + Compose compiler |
| API level min/target | **30 / 36** | Covers 95 %+ of active devices; grants full edge-to-edge support |

---

## 2. Build System

| Tool | Version | Purpose |
|---|---|---|
| Android Gradle Plugin | **8.7.x** | Modern build pipeline, R8 full mode |
| Gradle | **8.11.x** | Kotlin DSL scripts; version catalog for dependency management |
| Kotlin Compose Compiler | **2.1.x** (bundled with Kotlin) | No separate `compose-compiler` plugin needed |
| version catalog | `libs.versions.toml` | Single source of truth for all dependency coordinates |

---

## 3. UI Framework

| Library | Version | Rationale |
|---|---|---|
| Jetpack Compose BOM | **2025.06.00+** | Latest stable Compose tooling |
| Material 3 | via BOM | M3 design system, dynamic color, adaptive layouts |
| Compose Foundation | via BOM | `pointerInput`, `gestures`, `layout` primitives |
| Activity Compose | **1.10.x** | `setContent` with edge-to-edge `enableEdgeToEdge()` |
| Window Manager | **1.4.x** | Foldable posture detection, `FoldingFeature` |
| Navigation Compose | **2.9.x** | Type-safe navigation (Kotlin serialization) |

---

## 4. 3D Rendering

| Library | Version | Rationale |
|---|---|---|
| **SceneView** | **0.15.0+** | Android-first Filament wrapper; Compose `View` interop |
| Filament (transitive) | **1.58.x** | PBR rendering, IBL, morph targets for lip-sync |
| `gltfio` (bundled) | — | glTF 2.0 loader + KTX2 texture support |

**Why not alternatives?**
| Alternative | Rejected because |
|---|---|
| **Unity / Unreal** | APK size > 200 MB; overkill for single character |
| **OpenGL ES directly** | No animation/morph-target helpers; too much boilerplate |
| **Rajawali** | Unmaintained; no Compose interop |
| **Godot Mobile** | Heavy runtime; not designed for 2D+3D hybrid Compose apps |

**SceneView Compose interop pattern:**
```kotlin
@Composable
fun SceneViewComposable(modifier: Modifier = Modifier) {
    AndroidView(
        factory = { ctx ->
            SceneView(ctx).apply {
                // Configure renderer, load glTF
            }
        },
        modifier = modifier
    )
}
```

---

## 5. Audio Pipeline

| Library | Purpose | Why not alternatives |
|---|---|---|
| **AudioRecord** (android.media) | PCM capture | Zero dependencies; lowest latency |
| **AudioTrack** (android.media) | PCM playback | Zero dependencies; direct buffer write |
| **SoundPool** | SFX (button clicks, UI) | Pre-loaded, low-latency for small clips |
| **Ringdroid / Sonic** (NDK port) | Time-stretching / pitch-shift | Native C++ speed; no Java heap churn |

**Voice effect pipeline (native C++ via JNI):**
- `libtomic_processor.so` — custom shared library
- Sub-modules: `pitch_shift.c`, `formant_filter.c`, `reverb_model.c`, `robot_voice.c`
- Built via **CMake** + **NDK r27+**

---

## 6. Concurrency

| Technology | Usage |
|---|---|
| **Kotlin Coroutines** | Async operations: asset loading, audio recording, file I/O |
| **Flow (cold)** | Reactive state: recording amplitude, playback progress, animation triggers |
| **Dispatchers.IO** | File writes, audio buffer reads |
| **Dispatchers.Default** | DSP processor computations |
| **Main (immediate)** | Compose state mutations |

**Explicitly avoided:** RxJava, LiveData, AsyncTask, Thread pools.

---

## 7. File System (Zero-DB Strategy)

| Directory (Context.filesDir) | Purpose |
|---|---|
| `characters/` | Extracted glTF assets per character |
| `recordings/` | Raw PCM + processed WAV files |
| `config/` | JSON preference files (selected character, volume, etc.) |
| `cache/` | Temporary processed audio; cleared on app restart |

**No Room, SQLite, or Realm.** All state is read/written via `kotlinx.serialization` JSON.

---

## 8. Dependency Summary (libs.versions.toml)

```toml
[versions]
agp = "8.7.0"
kotlin = "2.1.0"
compose-bom = "2025.06.00"
activity-compose = "1.10.0"
window = "1.4.0"
navigation = "2.9.0"
sceneview = "0.15.0"
coroutines = "1.9.0"
serialization = "1.7.0"
ksp = "2.1.0-1.0.25"
junit = "4.13.2"
turbine = "1.2.0"

[libraries]
compose-bom = { group = "androidx.compose", name = "compose-bom", version.ref = "compose-bom" }
compose-material3 = { group = "androidx.compose.material3", name = "material3" }
compose-ui = { group = "androidx.compose.ui", name = "ui" }
compose-foundation = { group = "androidx.compose.foundation", name = "foundation" }
activity-compose = { group = "androidx.activity", name = "activity-compose", version.ref = "activity-compose" }
window = { group = "androidx.window", name = "window", version.ref = "window" }
navigation-compose = { group = "androidx.navigation", name = "navigation-compose", version.ref = "navigation" }
sceneview = { group = "io.github.sceneview", name = "sceneview", version.ref = "sceneview" }
coroutines-core = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-core", version.ref = "coroutines" }
coroutines-android = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-android", version.ref = "coroutines" }
serialization-json = { group = "org.jetbrains.kotlinx", name = "kotlinx-serialization-json", version.ref = "serialization" }
junit = { group = "junit", name = "junit", version.ref = "junit" }
turbine = { group = "app.cash.turbine", name = "turbine", version.ref = "turbine" }
coroutines-test = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-test", version.ref = "coroutines" }
```

---

## 9. Gradle Module Structure

```
Tomiko/
├── app/                          # :app — Application module (thin shell)
│   ├── src/main/
│   │   ├── java/                 # Kotlin source root
│   │   ├── res/                  # Drawables, layouts, M3 theme XML
│   │   ├── assets/               # Bundled glTF characters
│   │   └── jniLibs/              # Pre-built .so files (arm64-v8a, armeabi-v7a)
│   ├── src/test/
│   └── src/androidTest/
├── core/
│   ├── ui/                       # :core:ui — Shared composables, M3 theme
│   ├── audio/                    # :core:audio — AudioRecord/AudioTrack wrappers
│   ├── model/                    # :core:model — Domain entities, data classes
│   └── util/                     # :core:util — Extensions, constants, file helpers
├── feature/
│   ├── home/                     # :feature:home — Character select screen
│   ├── talk/                     # :feature:talk — Main recording/playback screen
│   └── settings/                 # :feature:settings — Volume, reset (parental gate)
├── lib/
│   └── tomic-processor/          # NDK module (C++): pitch, formant, reverb
├── build.gradle.kts              # Root build file
├── settings.gradle.kts           # Module registry
├── gradle/
│   └── libs.versions.toml        # Version catalog
└── gradle.properties             # JVM / Android / Compose flags
```
