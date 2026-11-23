# Implementation Status

**Last updated:** Phase 5-8 implementation pass, 22 November 2026.

## Phase checklist

| Phase | Status | Evidence / notes |
|---|---|---|
| 1. Audit | ✅ Complete | Repository audit delivered; no files modified |
| 2. Architecture, dependencies, scaffolding | ✅ Complete | Toolchain upgraded; Rust workspace + tests; docs/scripts |
| 3. Native foundation + GameActivity/Bevy spike | ✅ Complete | JNI bridge, Bevy cdylib, GameActivity, arm64 `.so`; launching and rendering on device |
| 4. Character foundation | ✅ Complete (device gate) | Original GLB, animation graph, blink morph, touch, pause/resume verified on device |
| 5. Audio foundation | ✅ Complete (device gate) | AAudio capture/playback, VAD, DSP, presets, lip-sync, focus/lifecycle verified on device |
| 6. Premium UI | ✅ Complete (first pass) | Compose home/hero/settings entry; native-resolution Bevy HUD with Talk/Replay/Voice |
| 7. Performance optimization | 🔄 In progress | 480p 3D target + native UI, HUD throttling, R8 APK 67.8 MB; cool-device 60 FPS gate still open |
| 8. Production readiness | 🔄 In progress | R8/resource shrink, env-based signing, legacy SceneView/C++ removed, release smoke test |

## Phase 2 completed work

### Toolchain

- Gradle wrapper upgraded to **9.7.0**.
- AGP upgraded to **9.3.1** with built-in Kotlin.
- Kotlin upgraded to **2.4.21**.
- Compose BOM upgraded to **2026.11.01** (Compose 1.13.0).
- `compileSdk = 37`, `compileSdkMinor = 1`; `targetSdk = 36`; `minSdk = 30`.
- Android SDK platform **android-37.1** installed.
- NDK pinned to **28.2.13676358**.
- Removed the legacy `org.jetbrains.kotlin.android` plugin.
- Migrated off `android.kotlinOptions`; Java/Kotlin target 17.
- Added `androidResources.noCompress` for GLB/KTX2/WAV/OGG.
- Added `jniLibs.useLegacyPackaging = false`.
- Added DataStore, GameActivity, AppCompat, Material Icons Core, AndroidX test
  runner, AndroidX ext JUnit.
- Added version catalog entries for the new dependencies.
- Fixed a positional-format string warning in `strings.xml`.

### Rust workspace

Created `rust/` with:

- `shared_types`: engine/character states, voice presets, audio/quality config,
  FFI-safe UI snapshot, stable error codes.
- `character_engine`: 10-state machine, touch reactions, idle/blink/sleep timers,
  animation clip mapping, behavior config.
- `audio_engine`: allocation-free PCM buffer, RMS/peak, attack/release envelope,
  six validated voice presets.
- `android_bridge`: bounded command channel, atomic state snapshot, engine
  lifecycle, command helpers.
- `tomiko_bevy` (`rust/crates/bevy_app`): Bevy cdylib, `#[bevy_main]` entry,
  test scene, engine command consumer, JNI exports.
- `rust/.cargo/config.toml`: 16 KB page alignment for `aarch64-linux-android`.

### Tooling

- Installed Rust target `aarch64-linux-android`.
- Installed `cargo-ndk` 4.1.2.
- Added `scripts/verify_environment.ps1`.
- Added `scripts/build_rust.ps1`.
- Added `scripts/build_android.ps1`.
- Added `scripts/clean.ps1`.
- Added repository-level `assets/README.md`.
- Added repository-level assets to the app source set.

### Documentation

- Rewrote `README.md`.
- Added `DEPENDENCIES.md`.
- Added `docs/ARCHITECTURE.md`.
- Added `docs/BUILD.md`.
- Added `docs/THREADING_MODEL.md`.
- Added `docs/AUDIO_PIPELINE.md`.
- Added `docs/ANDROID_INTEGRATION.md`.
- Added `docs/PERFORMANCE.md`.
- Added `docs/CHARACTER_PIPELINE.md`.
- Added `docs/DEPENDENCIES.md`.
- Added `docs/IMPLEMENTATION_STATUS.md`.

## Phase 3 completed work

### Native engine

- `android_bridge::EngineHandle`:
  - bounded command channel (capacity 64),
  - atomics for state/amplitude/progress/mouth/error,
  - idempotent initialization,
  - stable command helpers.
- JNI exports in the cdylib for:
  - initialize/shutdown/pause/resume,
  - start/stop listening, play last,
  - set voice preset/quality/character,
  - engine/character state, amplitude, progress, mouth open, error code,
  - ABI version and ping.
- `tomiko_bevy` with `#[bevy_main]` and Android GameActivity feature.
- Bevy test scene: camera, directional light, ground, rotating cube, and the
  existing `characters/trex/model.glb` loaded through `WorldAssetRoot`.
- `AssetMetaCheck::Never`, MSAA off on Android, shadows off on Android.
- Engine command consumer updates native state for Phase 3 validation.

### Android integration

- `CharacterGameActivity : GameActivity` with:
  - `System.loadLibrary("tomiko_game")`,
  - initialize/resume/pause/shutdown lifecycle calls,
  - immersive system UI.
- Manifest entry with `android.app.lib_name = "tomiko_game"`.
- `Theme.Tomiko.Game` AppCompat theme.
- `NativeEngine` JNI facade with static native methods.
- Repository-level `assets/` and existing app assets both packaged.

### Verification evidence

| Check | Result |
|---|---|
| Baseline build before changes | `BUILD SUCCESSFUL in 1m 22s` |
| AGP 9.3.1 / Kotlin 2.4.21 / Compose 1.13 build | `BUILD SUCCESSFUL in 46s` |
| Rust workspace tests | **42 passed, 0 failed** |
| `cargo check -p tomiko_bevy` (host) | Passed |
| arm64 debug native build | Passed (`libtomiko_game.so`, 1.65 GB unstripped) |
| arm64 release native build | Passed (`libtomiko_game.so`, **97.97 MB**) |
| `libc++_shared.so` packaged | Yes (8.81 MB from NDK r28c) |
| JNI symbols exported | Verified with `llvm-nm` |
| `android_main` exported | Verified |
| Debug APK | `app-debug.apk`, **109.14 MB** |
| Native libraries in APK | `libtomiko_game.so`, `libc++_shared.so` |
| Assets in APK | Existing character GLBs present |
| Runtime on device | ✅ Verified 22 Nov 2026 (see Phase 3/4 device validation) |

## Phase 4 completed work (first pass)

### Original character asset

- Generated with Blender 5.3 via `tools/blender/create_tomiko.py`.
- Output: `assets/characters/tomiko/character.glb` (~2.2 MB).
- One skinned mesh, 5 materials, 27 joints.
- 15 morph targets: 14 visemes + `viseme_BLINK`.
- 11 clips: `idle`, `listening`, `talking`, `happy`, `surprised`, `sad`,
  `angry`, `sleeping`, `blink`, `touch_head`, `touch_body`.
- 512×512 thumbnail and `metadata.json`.
- Character added at the top of `assets/characters/index.json`.

### Bevy character systems

- Loads the GLB asynchronously through `Gltf`/`WorldAssetRoot`.
- Builds an `AnimationGraph` from named clips.
- Attaches `AnimationGraphHandle` and `AnimationTransitions` to the loaded
  `AnimationPlayer`.
- Crossfades between clips.
- Idle blink timer drives the `viseme_BLINK` morph.
- Talking state drives `viseme_AA`/`viseme_O` morphs as a temporary stand-in
  for audio-driven visemes.
- Touch/mouse input selects `touch_head` or `touch_body`.
- Commands map to idle/listening/talking states; pause/resume controls the
  animation player.
- Camera, key/fill lighting, ground, and backdrop are in place.
- Android keeps MSAA and shadow maps off for stability until device validation.

### Verification status

| Check | Result |
|---|---|
| Blender generation | Passed |
| GLB structure | 1 mesh, 5 prims, 27 joints, 15 morphs, 11 animations, default weights 0 |
| `scripts/validate_character.ps1` | Passed |
| Host `cargo check -p tomiko_bevy` | Passed |
| Rust tests | 42 passed, 0 failed |
| `cargo clippy -D warnings` | Passed |
| arm64 release rebuild | Passed in 9m 38s |
| Debug APK rebuild | Passed in 30s |
| Debug APK content | `assets/characters/tomiko/character.glb` + index + thumbnail present |
| Debug APK size | 111.72 MiB (117,131,661 bytes) |
| On-device animation/morph | ✅ Verified 22 Nov 2026 |

## Phase 3/4 device validation (22 November 2026)

Device: **CPH2423 / OnePlus**, Android 15 (API 35), arm64-v8a,
**Mali-G610 MC6**, 1080×2412, 8 cores, 11.2 GiB RAM.

| Check | Result |
|---|---|
| APK install and GameActivity launch | Passed |
| wgpu adapter | Vulkan, `Mali-G610 MC6`, driver `v1.r32p1-01eac0...` |
| CPU fallbacks | GPU clustering + GPU preprocessing unsupported → CPU paths, no crash |
| Tomiko rendered fullscreen | Passed (screenshot at 1080×2412) |
| GLB load + animation graph attach | `Tomiko character scene spawned`, `Animation graph attached to player entity` |
| Idle loop | `Playing clip idle` |
| Blink cadence | `blink` → `idle` every ~4.50 s; closed-eye frame captured |
| All 11 clips | `idle`, `listening`, `talking`, `happy`, `surprised`, `sad`, `angry`, `sleeping`, `blink`, `touch_head`, `touch_body` all logged and crossfaded |
| Touch hit test | Upper tap → `touch_head`, lower tap → `touch_body` |
| Pause / resume | `Pause` on Home, `Resume` on return; same PID, no crash |
| App stability | Process alive after repeated clip/touch/lifecycle exercise |

### Bugs found and fixed during validation

1. **Mali Vulkan descriptor-pool panic.**
   `wgpu-hal 30.0.2` entered `unreachable!()` when the Mali driver returned
   `VK_ERROR_FRAGMENTED_POOL` from `vkAllocateDescriptorSets`. Reproduced as
   `SIGABRT` on `Compute Task Pool` a few seconds after startup.
   Fixed by vendoring `wgpu-hal 30.0.2` with the upstream patch from
   [gfx-rs/wgpu#10264](https://github.com/gfx-rs/wgpu/pull/10264)
   (commit `573944a`), wired through `[patch.crates-io]` in `rust/Cargo.toml`.
   Remove the vendor copy once a released wgpu-hal contains the fix.

2. **Stale native library packaged.**
   `cargo-ndk -o app/src/main/jniLibs/arm64-v8a` created a nested
   `arm64-v8a/arm64-v8a/` directory, while Gradle kept packaging the older
   library at the top level, so the descriptor-pool fix never reached the APK.
   `scripts/build_rust.ps1` now points `-o` at `app/src/main/jniLibs` and
   removes the legacy nested folder.

3. **Inverted touch hit test.**
   The head/body branch compared window Y in the wrong direction. The upper
   screen now triggers `touch_head` and the lower screen `touch_body`.

### Not yet measured

- Frame rate, frame time, and jank (Phase 7).
- Thermal behavior under sustained rendering.
- Activity destroy/recreate and long-running (>minutes) stability.

## Phase 5 audio implementation (22 November 2026)

Current device: **OnePlus EB2101**, board `lito`, Android 15 (API 35),
arm64-v8a, **Adreno 619**, 1080×2412, 8 cores, 11.2 GiB RAM.

> The Adreno 619 exposed a Bevy storage-buffer skinning bug that hid skinned
> characters. The earlier Phase 3/4 clip evidence was collected on a
> Mali-G610-class device; the Adreno path is now fixed with a vendored
> `bevy_render` patch.

### Implemented

- **Rust audio engine modules:** `dsp`, `vad`, `lipsync`, `runtime`, `aaudio`.
- **AAudio:** 48 kHz, mono, PCM16 input/output; low-latency and shared-mode
  fallbacks; state-aware start/stop; error callbacks.
- **Real-time safety:** input callback → SPSC ring; output callback ← SPSC
  ring; no allocation, locks, JNI, or logging in callbacks. Output callback
  computes RMS/ZCR lip-sync and writes atomics.
- **VAD:** 10 ms frames, 300 ms pre-roll, 800 ms silence timeout, hysteresis,
  10 s max duration, 150 ms tail trim.
- **DSP:** DC blocker, 80 Hz high-pass, WSOLA pitch shift + Catmull-Rom
  resampling, spectral-tilt formant approximation, low-pass, robot modulator,
  echo, reverb, soft limiter, gain. Six presets scaled by intensity.
- **Lip-sync:** mouth openness plus `viseme_AA`/`viseme_E`/`viseme_O` weights
  from playback RMS and zero-crossing rate.
- **Android:** runtime `RECORD_AUDIO` permission, `AudioFocusManager`
  (transient exclusive focus, loss/duck/gain handling), immersive edge-to-edge
  UI, stop capture on pause/stop.
- **Lifecycle ownership:** `android_bridge` owns the `AudioRuntime`; JNI pause,
  stop, resume, preset, and replay calls control audio directly instead of
  waiting for Bevy's event loop.
- **Pause safety:** stopping mid-playback discards leftover ring samples so a
  later utterance cannot replay stale audio.
- **Bevy:** audio runtime resource, command forwarding, phase→clip steering,
  meter-driven face morphs, debug keys (`Space` listen toggle, `P` replay,
  `Y` DSP self-test, `G` VAD self-test).
- **Adreno workaround:** vendored `bevy_render 0.20.1` forcing the
  uniform-buffer skinning and image morph-target paths on Android
  ([bevy#24926](https://github.com/bevyengine/bevy/issues/24926)).

### Evidence

| Check | Result |
|---|---|
| Workspace tests | **65 passed, 0 failed** (32 audio) |
| `cargo clippy --workspace --all-targets -D warnings` | Passed |
| Android cross-check / release build | Passed |
| AAudio input + output | Opened at 48 kHz, mono, PCM16, LowLatency, Shared |
| Live capture callback | Active; **747,648 samples**, **0 dropped** |
| Live VAD utterances | **1.33 s** and **0.62 s** captured and replayed |
| VAD synthetic self-test | 134 frames produced → DSP → playback |
| DSP duration | **1.6–4.3 ms** for 1.3 s utterances |
| Playback | **1.33 s** utterance, **0 underruns** |
| Overlapping playback | Two utterances queued sequentially, no `InvalidState` |
| Lip-sync | Mouth visibly opens across playback frames |
| Pause / resume | Audio stops/restarts, capture resumes, same PID |
| Background pause latency | Audio paused **6 ms** after JNI call; Bevy drained 512 ms later |
| Adreno 619 rendering | Character visible after `bevy_render` patch |

### Mic sensitivity fix

The initial VAD threshold (0.025 RMS) missed distant speech. The capture path
now uses:

- `VoiceCommunication` input preset first (platform AGC/noise suppression),
  then `VoicePerformance`, `VoiceRecognition`, and `Generic`.
- Adaptive VAD thresholds (0.0025 on / 0.0012 off) with a tracked noise floor.
- Per-utterance normalization: quiet speech is amplified up to **16×** before
  DSP, while loud speech is not boosted.
- A quiet synthetic VAD test (15% amplitude) exercises the distant-speech path.

Evidence: the CPH2423 captured utterances at **RMS 0.0038**, **0.0026**, and
**0.0033** (all below the old 0.025 threshold), with input gain measured at
**10.7×** and **16×**. The EB2101 captured at RMS 0.0053/0.0072/0.0028.

The higher sensitivity means ambient speech/noise can also trigger capture;
field tuning of the noise floor and a push-to-talk control remain follow-ups.

### Known limitations

- 48 kHz is required; other device-native rates are rejected rather than
  resampled.
- Formant handling is a spectral tilt, not true formant preservation.
- Lip-sync is amplitude/ZCR based, not phoneme-accurate.
- Listening auto-starts on resume; a push-to-talk gesture is a Phase 6 UI
  follow-up.
- Capture-start→playback latency and preset quality have not been measured
  with an external reference.

## Phase 6 premium UI (22 November 2026)

- Compose is now the launcher: `MainActivity` starts at the new Home hero with
  “Start talking”, Settings, and an offline/privacy note.
- “Start talking” launches the native `CharacterGameActivity`.
- Legacy SceneView, Talk, old Game, Kotlin audio, and the C++ DSP UI paths were
  removed from the production app.
- Bevy HUD at native UI resolution: state/preset label, Stop/Talk, Replay, and
  Voice preset cycle. The 3D stage renders off-screen and is upscaled; the HUD
  stays sharp.
- HUD text updates at 4 Hz instead of every frame.
- Camera pulled back so the character fits above the HUD.
- Activity destroy now writes `AppExit` so Bevy stops instead of spinning
  without a surface.

## Phase 7 performance (22 November 2026)

- 3D stage render target reduced from native 1080×2400 to **480×1067**,
  upscaled by the UI camera; HUD text stays native.
- HUD text layout throttled to 4 Hz; FPS is logged every 5 s.
- `strip = "symbols"` reduced the release `.so` from **98.3 MB to 61.2 MB**.
- Removed `basis-universal`, SceneView/Filament, CMake/C++ DSP, and legacy
  assets.
- Release APK with R8/resource shrinking: **67.8 MB** (debug: 102.4 MB).
- Measured on Adreno 619 (EB2101): **37 FPS** at 600p before thermal
  throttling; **18–21 FPS** at 480p while the device reported **thermal status
  3**. The throttled numbers are not a valid 60 FPS gate. A cool-device run is
  still required.
- SurfaceFlinger reported 49–60 FPS during the earlier CPH2423 run.

## Phase 8 production readiness (22 November 2026)

- R8 + resource shrinking enabled for release.
- ProGuard narrowed to manifest entry points, JNI methods, and Kotlin
  serialization instead of keeping the whole app.
- Release signing reads `TOMIKO_KEYSTORE`, `TOMIKO_KEYSTORE_PASSWORD`,
  `TOMIKO_KEY_ALIAS`, and `TOMIKO_KEY_PASSWORD`; without them the release build
  is unsigned.
- Release APK built, signed with the local debug key for smoke testing, and
  installed successfully.
- Legacy native libraries/classes removed from the package.
- Remaining production work: real release keystore, device matrix, Play Console
  data-safety/privacy, cool-device performance gate, and a final Play test.

## Current repository state

- Compose is the launcher/home; the native GameActivity is the character stage.
- Legacy SceneView/Filament, Talk/Game screens, Kotlin audio, and C++ DSP are
  removed from the production app.
- Rust owns AAudio capture/playback, VAD, DSP, normalization, and lip-sync.
- Mic sensitivity is fixed with `VoiceCommunication`, adaptive VAD, and
  per-utterance normalization.
- Bevy HUD provides Talk/Stop, Replay, and Voice preset control.
- Release APK: **67.8 MB**; release `.so`: **61.2 MB**.
- FPS measurements are still affected by device thermal throttling.

## Next tasks

1. Cool the test device and measure the final 480p render target at a stable
   temperature; tune resolution/morphs to reach 60 FPS on Adreno 619.
2. Add push-to-talk and wire Compose settings to voice precedence/presets.
3. Measure capture-to-playback latency, preset quality, and lip-sync accuracy
   with an external reference.
4. Produce a release build with the real keystore, run the full device matrix,
   and prepare Play Console data-safety/privacy.
5. Evaluate true formant-preserving pitch shifting and phoneme-level visemes.
6. Stress-test destroy/recreate and >10-minute sessions.

## Blockers and open items

| Item | Impact | Action |
|---|---|---|
| Device thermally throttled (status 3) | No valid final 60 FPS number | Cool device, re-measure; consider quality tiers |
| Native release `.so` is 61.2 MB | APK 67.8 MB | Evaluate further feature trimming after FPS gate |
| Ambient speech can trigger capture | Unwanted playback in noisy rooms | Add push-to-talk and tune noise floor with field data |
| No external latency measurement | Cannot claim a specific mouth/audio latency | Loopback or hardware latency test |
| Formant handling is approximate | Presets do not truly preserve formants | Evaluate a phase-vocoder/formant tracker |
| Release signing uses a debug key for smoke tests | Not shippable | Provide the real keystore/env vars before publishing |
| Vendored `wgpu-hal` patch | Extra maintenance until upstream release | Drop when a released wgpu-hal includes #10264 |
| Vendored `bevy_render` patch | Extra maintenance until Bevy fixes #24926 | Drop when a released Bevy contains the fix |
