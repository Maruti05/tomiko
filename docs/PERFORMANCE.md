# Performance Plan

## 1. Engineering targets

These are targets, not measured results. They must be verified on real hardware.

| Metric | Target | Notes |
|---|---|---|
| Frame rate | 60 FPS mid-range; 120 Hz where supported | Quality-tier dependent |
| Frame time p95 | < 16.6 ms @ 60 FPS | Include UI overlay |
| Audio callback | < 2 ms per burst | No allocation/locks |
| DSP | < 50% of real-time budget | Room for device variance |
| Record-to-playback start | < 250 ms after speech end | Target |
| Mouth latency | < 16 ms sample-to-morph | Playback timeline |
| JNI calls | 0/frame, 0/audio buffer; ~10 Hz UI poll | Counters |
| Cold start to first frame | < 2 s | `reportFullyDrawn` |
| Interactive ready | < 3 s | Permission/audio ready |
| Memory | < 150 MB typical | `dumpsys meminfo` |
| Idle battery | < 3%/h screen-on idle | Battery historian |

## 2. Quality tiers

| Tier | RAM signal | Render scale | Shadows | SSAO | MSAA | FPS cap |
|---|---|---:|---|---:|---:|---:|
| Low | < 3 GB or weak GPU | 0.75 | 512 map, off/low | Off | 1x | 30 |
| Medium | 3–6 GB | 1.0 | 1024 map | Low | 2x | 60 |
| High | > 6 GB + Vulkan | 1.0 | 2048 map | Medium | 4x | 120 |

Tier selection must also check Vulkan availability and GPU class. RAM alone is
not sufficient.

## 3. Rendering budget

- One skinned character mesh, 20–30k triangles, 2–3 materials.
- One main directional light + ambient/IBL.
- Shadow map 512/1024/2048 by tier.
- No bloom on Low; subtle bloom on High only.
- No depth of field, motion blur, or expensive post-processing.
- 1–2 draw calls for the character; background as a simple plane/gradient.
- KTX2/Basis textures, 512/1024, mipmapped.
- Frustum culling; no unnecessary shadow casters.
- Avoid per-frame material/mesh re-creation.
- Update morph weights only when values change.

## 4. Audio budget

- No allocation or locks in AAudio callbacks.
- Preallocated rings and DSP scratch buffers.
- DSP work bounded per block; drop/skip gracefully on overload.
- Underrun counter and callback duration histogram.
- No logging/formatting in the audio path.
- Mute capture during playback.

## 5. Instrumentation

### Bevy/Rust

- Frame time and FPS.
- Draw calls/entities.
- Asset load durations.
- Audio callback duration, DSP duration, underruns.
- Ring fill level.
- JNI call count.
- State transition timings.

### Kotlin

- Startup timings (`reportFullyDrawn`, process start).
- Permission flow duration.
- UI state poll duration.
- Audio focus transitions.
- Memory snapshots.

### External

- Android Studio Profiler.
- GPUWatch / Android GPU Inspector.
- `dumpsys meminfo`, `dumpsys gfxinfo`.
- Perfetto traces.
- Macrobenchmark (Phase 7).
- OboeTester if Oboe fallback is used.

## 6. Measurement protocol

1. Use a physical arm64 device with Vulkan.
2. Cold start 5 times; record median/p95.
3. Idle 5 minutes; record FPS/frame time/battery.
4. Speak/replay 20 times; record audio latency/underruns.
5. Touch/idle/animation loop 100 times; record frame drops.
6. Background/foreground and audio interruption tests.
7. Release build, not debug.
8. Store raw results and device details in `docs/PERFORMANCE_RESULTS.md` when available.

Do not claim targets are met without this evidence.

## 7. Optimization checklist

- [ ] No per-frame allocations in Bevy systems.
- [ ] No per-frame JNI.
- [ ] No locks in audio callbacks.
- [ ] Bounded queues.
- [ ] Textures compressed and mipmapped.
- [ ] Shadow map quality tiered.
- [ ] MSAA tiered.
- [ ] Morph updates change-driven.
- [ ] Animation clips loaded once.
- [ ] Audio streams released on background.
- [ ] Rendering paused on background.
- [ ] Baseline profile added.
- [ ] Release R8 rules validated.
- [ ] 16 KB page alignment verified.

## 8. Current status

Phase 2 has no performance measurements. Instrumentation and device testing
begin in Phase 3/5 and are reported in Phase 7. The machine currently has no
connected Android device.
