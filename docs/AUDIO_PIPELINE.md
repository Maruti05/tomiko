# Audio Pipeline

## 1. Ownership model

**Rust owns capture, DSP, playback, and lip-sync analysis.**
**Kotlin owns Android audio focus, session policy, and interruption handling.**

The `AudioRuntime` handle lives in `android_bridge`, not in Bevy. JNI lifecycle
calls (`nativePause`, `nativeStopListening`, etc.) send audio commands directly
from the Android main thread, so capture stops even while Bevy's event loop is
suspended. Bevy reads the shared meter atomics every frame and forwards only
debug commands.

This is the only audio ownership model. There is no second `AudioRecord` or
`AudioTrack` path in production.

## 2. Backend decision

The initial backend is **AAudio in Rust** (`ndk` crate, `audio` feature).
`minSdk 30` guarantees AAudio availability.

Reasons:

- One owner for input and output.
- No per-buffer JNI.
- Playback callback has the exact samples needed for mouth synchronization.
- No duplicate buffers or device state.
- Avoids a C++ layer in the first implementation.

**Oboe 1.11.0 is the documented fallback** if device testing reveals AAudio
failures. It will be added behind the same `AudioBackend` trait, not bolted onto
the Kotlin layer.

## 3. Stream configuration

| Parameter | Initial value |
|---|---|
| Sample rate | 48,000 Hz (device-native; resample only if required) |
| Channels | Mono |
| Format | PCM16 |
| Input burst | 192 frames (~4 ms) |
| Output burst | 192–256 frames |
| Performance mode | `LOW_LATENCY`, fallback `NONE` |
| Sharing mode | `EXCLUSIVE`, fallback `SHARED` |
| Input preset | `VOICE_RECOGNITION`, fallback `GENERIC` |
| Output usage | `MEDIA` / `SPEECH` |
| Max recording | 10,000 ms |
| Silence timeout | 800 ms |
| Pre-roll | 300 ms |

All values are validated by `AudioConfig::validate()` before a stream opens.

## 4. Buffers

```text
AAudio input callback
  → SPSC capture ring (preallocated)
  → capture/VAD worker
  → pre-roll + speech buffer (fixed capacity)
  → DSP worker
  → SPSC playback ring (preallocated)
  → AAudio output callback
```

Rules:

- No allocation in callbacks.
- No locks in callbacks.
- Ring buffers are SPSC and bounded.
- Capture buffer capacity is derived from `max_recording_ms`.
- Pre-roll is a fixed ring of 300 ms.
- If the playback ring underruns, output silence and increment an underrun counter.

## 5. Voice DSP

Planned chain:

```text
DC blocker → high-pass (80 Hz) → pitch shift (formant-aware)
  → formant shift → low-pass → robot/echo (preset-dependent)
  → soft-knee limiter → gain
```

Pitch shifting must preserve duration. Do **not** use naive linear
interpolation; it changes speed and creates aliasing. The implementation target
is WSOLA/PSOLA-style overlap-add with formant preservation. `rubato` is used
only for sample-rate conversion, not as a pitch shifter.

### Presets

| Preset | Pitch | Formant | Reverb | Robot | Echo | Low-pass | Notes |
|---|---:|---:|---:|---:|---:|---:|---|
| Normal | 1.00 | 1.00 | 0.00 | 0.00 | 0 ms | 20 kHz | Neutral |
| Cartoon | 1.45 | 1.25 | 0.05 | 0.00 | 0 ms | 12 kHz | Bright, small-character |
| High Pitch | 1.80 | 1.00 | 0.00 | 0.00 | 0 ms | 18 kHz | Chipmunk-like |
| Deep | 0.65 | 0.75 | 0.05 | 0.00 | 0 ms | 8 kHz | Low, large-character |
| Robot | 1.00 | 1.00 | 0.00 | 0.65 | 0 ms | 6 kHz | Sample-and-hold/ring mod |
| Funny | 1.25 | 0.85 | 0.12 | 0.15 | 120 ms | 10 kHz | Cartoon + echo |

Parameters are defined in `audio_engine::presets` and validated at startup.
Extreme settings are clamped; the DSP must never clip the output.

## 6. VAD and capture

- `VoiceCommunication` input preset first for platform AGC/noise suppression,
  with `VoicePerformance`, `VoiceRecognition`, and `Generic` fallbacks.
- Adaptive energy thresholds (0.0025 on / 0.0012 off) with a tracked noise
  floor. This is sensitive enough for distant speech (validated at RMS 0.0026
  to 0.0038) while a noise floor above the speech level raises the gate.
- Per-utterance normalization amplifies quiet speech up to **16× before DSP**
  so distant utterances are audible; loud speech is not boosted.
- Pre-roll prevents clipped word starts.
- Silence timeout ends the utterance.
- Hard max duration ends the utterance.
- Capture is muted during playback to prevent feedback.
- No continuous capture when the app is backgrounded or not in talk mode.

## 7. Lip-sync

### First release

- Compute RMS from the **playback buffer samples** in the output callback.
- Smooth with independent attack/release envelopes.
- Derive mouth openness plus broad viseme weights from RMS and spectral balance.
- Publish atomics; Bevy reads them every frame.
- Mouth latency target: < 16 ms from sample to morph update.

This is amplitude/spectral viseme approximation, **not** phoneme-accurate
lip-sync. Document the distinction in the UI/README.

### Future

- Phoneme/viseme alignment with a small on-device model or rule-based formant
  tracker.
- Per-character viseme mapping.

## 8. Audio focus and interruptions

Kotlin `AudioFocusManager`:

- Request `AUDIOFOCUS_GAIN_TRANSIENT_EXCLUSIVE`.
- Handle `LOSS`, `LOSS_TRANSIENT`, `LOSS_TRANSIENT_CAN_DUCK`, `GAIN`.
- Pause capture/playback on focus loss.
- Resume only after explicit user action or a safe focus regain.
- Stop capture on `onStop`.
- Release streams on `onDestroy`.
- Mute microphone during playback.
- No foreground service in the first release; the experience is foreground-only.

## 9. Measurement

Instrument:

- Capture callback duration.
- DSP duration per block.
- Output callback duration.
- Underrun count.
- Ring-buffer fill level.
- Capture-start to playback-start latency.
- Playback sample index vs mouth update timestamp.
- Sample-rate conversion cost (if used).

Measure on a real device. Emulator numbers are not acceptable for latency or
glitch claims.

## 10. Current status

Phase 5 is implemented and device-validated on Android 15 / Adreno 619:

- AAudio input/output streams at 48 kHz, mono, PCM16, low-latency/shared.
- Lock-free capture and playback rings with no allocation or locking in
  callbacks.
- Frame-based VAD with pre-roll, hysteresis, silence timeout, and max duration.
- Offline DSP worker: DC blocker, 80 Hz high-pass, WSOLA pitch shift,
  Catmull-Rom resampling, spectral-tilt formant approximation, low-pass,
  robot modulator, echo, reverb, soft limiter, and gain.
- Playback-buffer RMS/ZCR lip-sync published through atomics; Bevy applies
  `viseme_AA`, `viseme_E`, and `viseme_O`.
- Kotlin audio focus, runtime microphone permission, and foreground-only
  lifecycle.
- Underruns, captured frames, dropped frames, and DSP duration are counted.

### Implementation reference

| Module | Responsibility |
|---|---|
| `audio_engine::aaudio` | AAudio builders, callbacks, SPSC rings, state checks |
| `audio_engine::runtime` | Control thread, VAD scheduling, DSP queue, meter atomics |
| `audio_engine::dsp` | WSOLA, filters, effects, limiter, test signal |
| `audio_engine::vad` | Pre-roll + utterance segmentation |
| `audio_engine::lipsync` | RMS/ZCR mouth and viseme approximation |
| `android_bridge` | Runtime ownership, direct JNI audio commands, meter access |
| `AudioFocusManager.kt` | Focus request/interruption handling |
| `CharacterGameActivity.kt` | Permission, focus, immersive UI, lifecycle |

### Device validation (22 November 2026)

- Input and output opened at **48,000 Hz**, mono, PCM16, low-latency, shared.
- Capture callback delivered **747,648 samples** in one session with
  **0 dropped**.
- The live microphone produced VAD utterances of **1.33 s** and **0.62 s**;
  both were processed, played back, and counted **0 underruns**.
- DSP processed a 1.34 s utterance in **1.6–4.3 ms**.
- Two self-tests queued during playback and played sequentially without
  `InvalidState`.
- Lip-sync screenshots show the mouth opening across playback frames.
- `onPause` stopped audio **6 ms** after the JNI call, while Bevy only drained
  the same command 512 ms later; direct bridge ownership prevents background
  capture/playback.

### Known limitations

- 48 kHz is currently required; a device-native rate mismatch is reported as
  an error instead of being resampled.
- Formant handling is a high-shelf tilt approximation, not true
  formant-preserving pitch shifting.
- Lip-sync is amplitude/zero-crossing based, not phoneme-accurate.
- The activity auto-starts listening on resume for the Phase 5 gate. A
  push-to-talk UI lands in Phase 6.
- Continuous capture is used only while the activity is foreground and audio
  focus is held.
- Audio focus loss pauses capture/playback; regain resumes only when the
  activity is still resumed.
- End-to-end latency (capture start to first playback sample) has not been
  measured with an external reference.
