# Threading Model

## 1. Thread ownership

| Thread | Owner | Responsibilities | Real-time rules |
|---|---|---|---|
| Android main | Kotlin | Compose composition, navigation, permissions, activity lifecycle | No blocking I/O or DSP |
| GameActivity UI | Kotlin | Surface lifecycle, overlay, audio focus, JNI commands | Keep work short; no per-frame allocations |
| Bevy main/update | Rust/Bevy | ECS, animation graph, character events, touch, meter reads | No blocking I/O; no per-frame JNI |
| Bevy render | Rust/Bevy/wgpu | GPU submission, uploads, present | No locks shared with audio |
| AAudio input callback | Rust | Read mic, write capture ring, timestamp | No alloc, no lock, no JNI, no logging |
| Audio control | Rust | Commands, VAD, state machine, playback scheduling | Bounded channels; no callback work |
| DSP worker | Rust | Pitch/filter/effects chain, offline per utterance | Allocation allowed; never in a callback |
| AAudio output callback | Rust | Read playback ring, write device, compute RMS/visemes, update atomics | No alloc, no lock, no JNI |
| Bevy asset IO | Rust/Bevy | GLB/texture load | Async; never block update/render |
| JNI caller | Kotlin → Rust | User actions | Bounded channel; return immediately |

## 2. Data flow

```text
Kotlin lifecycle/preset/replay
  → JNI
  → android_bridge
  → bounded audio command channel (direct; does not wait for Bevy)

Kotlin animation commands
  → JNI
  → bounded crossbeam channel
  → Bevy / character_engine

Mic
  → AAudio input callback
  → SPSC capture ring
  → audio control thread (VAD)
  → DSP worker
  → SPSC playback ring
  → AAudio output callback
  → atomics (RMS, viseme, progress)
  → Bevy Update
  → MorphWeights / AnimationGraph
  → GPU

Kotlin UI
  ← nativeGetUiState() poll at ~10 Hz
```

## 3. Real-time rules

- No heap allocation in audio callbacks.
- No mutex/rwlock in audio callbacks.
- No JNI in audio callbacks.
- No file I/O, logging, or formatting in audio callbacks.
- No unbounded channels. Command/event queues are bounded; overflow returns a
  typed error instead of blocking.
- Use `AtomicU32`/`AtomicF32`-style bit-casts for shared scalar state; do not
  share `f32` across threads through unsynchronized memory.
- Preallocate capture buffers, playback buffers, DSP scratch buffers, and viseme
  arrays.
- Clamp/validate all Kotlin-provided values at the JNI boundary.

## 4. JNI crossings

Allowed:

- Initialization/shutdown.
- Start/stop listening.
- Play/replay.
- Voice preset/character change.
- Touch events (if routed through Kotlin; Bevy can handle touch directly).
- Pause/resume.
- ~10 Hz UI state poll.
- Debug diagnostics poll.

Not allowed:

- Per audio sample or per audio buffer.
- Per rendered frame.
- Per animation event.
- Any blocking call from Kotlin main thread.

## 5. Shutdown sequence

1. Kotlin calls `nativePause()` on `onPause`.
2. Kotlin calls `nativeShutdown()` on `onDestroy`.
3. Rust stops accepting commands.
4. Audio streams are stopped and closed; callbacks no longer run.
5. DSP worker is joined.
6. Bevy app exits its event loop and drops the world.
7. JNI global references are released.
8. Kotlin unloads the library only after the activity is destroyed.

`nativeShutdown()` must be idempotent. A second call returns `Ok` without
touching freed resources.

## 6. Performance instrumentation

- Frame time/FPS: Bevy diagnostics.
- Audio callback duration: Rust timestamps, histograms.
- DSP duration: Rust timestamps.
- Underruns: AAudio callback counters.
- JNI calls: counters incremented at the boundary.
- UI poll duration: Kotlin monotonic clock.
- Memory: `dumpsys meminfo`, Android Studio Profiler.
- Startup: `reportFullyDrawn`, Macrobenchmark.
- GPU: GPUWatch/AGI where available.

Do not claim a target is met without a measurement from a real device.
