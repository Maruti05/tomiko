# Architecture

## 1. Goals

Tomiko is an offline Android talking-character experience with:

- Immediate 3D character rendering and smooth animation.
- Low-latency microphone capture, local voice transformation, and playback.
- Mouth movement synchronized to the actual playback timeline.
- Responsive touch/idle behavior.
- Premium Compose UI for everything outside the 3D stage.
- No network, accounts, uploads, analytics, or cloud processing.

## 2. Non-goals for the first release

- Live full-duplex voice changing while speaking.
- Phoneme/viseme alignment from an ML model.
- Multiple characters with unique rigs/voices at launch.
- Background microphone capture.
- Cross-platform builds beyond Android arm64.

## 3. Layers

```text
┌─────────────────────────────────────────────────────────────────────┐
│ KOTLIN / ANDROID                                                    │
│  MainActivity (Compose shell)                                       │
│   ├── onboarding, character selection, voice presets, settings      │
│   └── launches CharacterGameActivity after onboarding               │
│  CharacterGameActivity : GameActivity                               │
│   ├── Bevy SurfaceView + optional Compose/native overlay            │
│   ├── audio focus/session policy                                    │
│   ├── permission + lifecycle coordination                           │
│   └── NativeEngine JNI facade                                       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ rare commands + ~10 Hz UI state poll
┌───────────────────────────────▼─────────────────────────────────────┐
│ RUST CORE (libtomiko_game.so)                                       │
│                                                                     │
│  android_bridge                                                     │
│   ├── JNI exports, panic containment, error codes                   │
│   ├── bounded command channel                                       │
│   └── atomic UI snapshot                                            │
│                                                                     │
│  audio_engine                                                       │
│   ├── AAudio capture/playback                                       │
│   ├── lock-free rings, VAD, pre-roll                                │
│   ├── DSP presets, limiter, filters                                 │
│   └── playback-timeline RMS/viseme analysis                         │
│                                                                     │
│  character_engine                                                   │
│   ├── 10-state character machine                                    │
│   ├── touch/idle/animation events                                   │
│   └── behavior configuration                                        │
│                                                                     │
│  tomiko_bevy                                                        │
│   ├── GLTF loading, skeleton, animation graph                       │
│   ├── morph targets / facial expressions                            │
│   ├── camera, lighting, materials, quality tiers                    │
│   ├── touch picking and reactions                                   │
│   └── diagnostics                                                   │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                        assets/characters/<id>/character.glb
                        assets/environments/studio.ktx2
```

## 4. Module responsibilities

### Kotlin

- Application lifecycle, Activity lifecycle, permissions.
- Compose UI, navigation, design system, accessibility.
- Audio focus, audio session policy, interruption handling.
- Settings persistence via DataStore.
- Character selection and voice preset selection.
- JNI facade, error mapping, UI state polling.
- Device capability detection (RAM, Vulkan, GPU class).
- Launching and managing `CharacterGameActivity`.

### Rust `shared_types`

- Stable enums (`EngineState`, `CharacterState`, `VoicePreset`, `TouchZone`).
- Audio/quality/character configuration.
- FFI-safe UI snapshot and error codes.
- No Android/Bevy/audio-device dependencies.

### Rust `audio_engine`

- AAudio capture and playback (Phase 5).
- Lock-free capture/playback rings.
- VAD, pre-roll, max-duration enforcement.
- Pitch/formant/filter/robot/echo/limiter DSP.
- Playback-timeline RMS and viseme estimation.
- Atomics consumed by Bevy; no JNI in the audio path.

### Rust `character_engine`

- Character state machine.
- Touch reactions, idle/blink/sleep timers.
- Animation event generation.
- Character behavior configuration.
- No rendering code.

### Rust `android_bridge`

- Thread-safe engine handle and bounded command channel.
- Atomic UI state snapshot.
- Error propagation and command helpers.
- Native resource lifecycle.

JNI shims live in `tomiko_bevy` (the cdylib crate) and call `android_bridge`.
Keeping the exported symbols in the final cdylib prevents the linker from
dropping them from a dependency archive.

### Rust `tomiko_bevy` (`rust/crates/bevy_app`)

- Bevy `App` and `#[bevy_main] android_main`.
- GLTF/asset loading.
- Skeleton and animation graph.
- Morph targets/visemes and facial expressions.
- Camera, lights, shadows, materials.
- Touch picking and reaction playback.
- Audio-sync systems reading `audio_engine` atomics.
- Performance diagnostics.

## 5. Key decisions

### 5.1 Single Android module

The project stays as `:app` for now. A single module keeps the Kotlin/Compose
surface simple and avoids duplicating Gradle/native configuration. Package-level
separation is enforced by convention and documented here. If GameActivity needs
isolation later, it can move to a `:game` module without changing the architecture.

### 5.2 One native cdylib

`libtomiko_game.so` contains Bevy + audio + character + JNI bridge. One library
means one global engine handle, one Rust allocator, one Bevy world, and no
cross-library state duplication. The Rust crates remain independently testable.

### 5.3 Rust owns audio

Kotlin owns Android audio focus/session policy. Rust owns the device streams,
buffers, DSP, and playback timeline. This avoids per-buffer JNI, duplicate
ownership, and feedback/lifecycle conflicts. See `AUDIO_PIPELINE.md`.

### 5.4 GameActivity is the character stage

The character screen is a dedicated `CharacterGameActivity` subclass of
GameActivity. Bevy renders to the GameActivity SurfaceView. Compose/native
overlays are evaluated in Phase 3; if the overlay is too expensive or unstable,
the HUD falls back to native Views or Bevy UI. Compose remains the UI framework
for onboarding, selection, presets, settings, about, and privacy.

### 5.5 No per-frame JNI

Audio-to-Bevy synchronization stays inside Rust. Kotlin polls a compact
`NativeUiState` at ~10 Hz for high-level UI state. User actions cross JNI only
when they occur.

### 5.6 Assets are APK assets

Bevy 0.20.1 has an Android `AssetReader` backed by `AssetManager`. Character
GLBs and KTX2 textures are packaged under `assets/` and loaded with
`AssetMetaCheck::Never`; no `.meta` files or hot reload on Android.

## 6. State model

```text
EngineState:
  Idle -> Listening -> Capturing -> Processing -> Playing -> Listening
  Any -> Paused (lifecycle/audio focus)
  Any -> Error (recoverable via retry)

CharacterState:
  Idle, Listening, Talking, Happy, Surprised, Sad, Angry,
  Sleeping, Blinking, TouchReaction
```

`character_engine` owns transitions and emits `AnimationEvent`s. Bevy applies
them; Bevy never invents character state. `EngineState` is owned by the audio
engine/bridge, not by the character machine.

## 7. Error model

- Rust returns `EngineError`; JNI maps it to stable `i32` codes.
- No Rust panic or raw error text crosses JNI.
- Kotlin maps codes to user-facing messages.
- Asset failures are reported with the asset path and a retry action.
- Audio device failures fall back to shared mode, then to Oboe if necessary.
- Release builds log sanitized diagnostics; no audio content is logged.

## 8. Privacy

- Microphone audio stays in RAM.
- No recordings are persisted by default.
- No network permission is required for the core experience.
- No analytics/ads/crash SDKs.
- Debug diagnostics are local and opt-in.
- Permission rationale explains microphone use before requesting access.

## 9. Migration strategy

1. Keep the current SceneView screen until the Bevy Phase 3 gate passes.
2. Add the Bevy `CharacterGameActivity` alongside the existing screen.
3. Route the main experience to Bevy once rendering, lifecycle, and JNI are stable.
4. Remove SceneView, the parallel TalkScreen implementation, and the old C++ DSP.
5. Keep the old screen only in Git history, not in the shipping APK.
