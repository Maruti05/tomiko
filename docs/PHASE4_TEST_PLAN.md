# Phase 4 Device Test Plan

Phase 4 validates the original character asset, animation graph, morph targets,
and touch responses. It requires an arm64 Vulkan device.

## Execution record — 22 November 2026

Device: **CPH2423 / OnePlus**, Android 15 (API 35), arm64-v8a, Mali-G610 MC6,
1080×2412. Native release library packaged into a debug APK.

Result: **passed the animation and touch gate** after three fixes:

1. Vendored the wgpu-hal `VK_ERROR_FRAGMENTED_POOL` fix from
   [gfx-rs/wgpu#10264](https://github.com/gfx-rs/wgpu/pull/10264); this stopped
   the startup `SIGABRT` on Mali.
2. Corrected `scripts/build_rust.ps1`, which wrote `libtomiko_game.so` into a
   nested `jniLibs/arm64-v8a/arm64-v8a/` folder and left Gradle packaging a
   stale library.
3. Inverted the touch hit test so the upper screen triggers `touch_head` and
   the lower screen triggers `touch_body`.

Observed log sequence: `Tomiko character scene spawned` →
`Animation graph attached to player entity` → `Playing clip idle`, followed by
`blink` every ~4.50 s and `Playing clip <name>` for every debug key. Upper and
lower taps produced `touch_head` and `touch_body`. Home/return produced
`Native command: Pause` / `Resume` with the same PID.

## 1. Build and launch

```powershell
.\scripts\build_android.ps1 -Mode debug
.\scripts\run_debug.ps1 -SkipBuild
```

Logs:

```powershell
adb logcat -c
adb logcat -s Tomiko:* bevy:* wgpu:* AndroidRuntime:E
```

Look for:

- `Tomiko character scene spawned`
- `Animation graph attached to player entity ...`
- `Playing clip idle`
- no `Failed to load` / `Unsolved` errors
- no `ANR` or `FATAL EXCEPTION`

## 2. Visual checks

- [x] Tomiko is centered and fully visible.
- [x] Character is upright, facing the camera, correct scale.
- [x] Eyes, muzzle, ears, belly, and limbs read clearly. (Tail is not visible
      from the default front camera.)
- [x] Materials are correct; no pink missing-texture surfaces.
- [x] Idle animation moves subtly. (Clip plays; smoothness not measured.)
- [x] Blink occurs roughly every 4.5 seconds.
- [x] No z-fighting or exploded polygons.
- [x] Backdrop/ground look intentional.

## 3. Animation clip checks

Use ADB key events (GameActivity receives them):

```powershell
adb shell input keyevent 37   # I = idle
adb shell input keyevent 40   # L = listening
adb shell input keyevent 48   # T = talking
adb shell input keyevent 36   # H = happy
adb shell input keyevent 49   # U = surprised
adb shell input keyevent 47   # S = sad
adb shell input keyevent 29   # A = angry
adb shell input keyevent 54   # Z = sleeping
adb shell input keyevent 30   # B = blink
adb shell input keyevent 45   # Q = touch_head
adb shell input keyevent 51   # W = touch_body
```

Expected: a `Playing clip <name>` log and a visible crossfade. One-shot clips
return to idle.

## 4. Morph checks

- [ ] Talking shows alternating `viseme_AA`/`viseme_O` mouth shapes. (Not
      visually inspected; placeholder motion is replaced in Phase 5.)
- [x] Blink closes both eyes to a line and opens them. (Closed-eye frame
      captured; cadence confirmed in logs.)
- [ ] Mouth returns to neutral after talking/blink. (Not captured frame-by-frame.)
- [x] No morph target defaults to 1.0 (face must not look collapsed on load).

## 5. Touch checks

- [x] Tap the upper half → head reaction.
- [x] Tap the lower half → body reaction.
- [ ] Repeated taps do not crash or leave the character stuck. (Two taps
      tested; stress test still open.)
- [x] Reaction returns to idle.

## 6. Animation quality gate

- [ ] Crossfades are smooth, not popping. (Not measured; needs frame capture.)
- [x] Limbs do not detach or interpenetrate badly. (Visual inspection.)
- [ ] Head/ear motion reads as intentional. (Not captured frame-by-frame.)
- [ ] Idle loop does not visibly snap. (Not measured.)
- [x] Character remains responsive while animations play. (Keys and taps
      handled during playback.)

## 7. Current known limitations

- The character is a first visual pass; texture/KTX2 and weight cleanup are
  follow-ups.
- Talking morphs are a temporary sine-wave placeholder until the audio engine
  drives playback-timeline visemes in Phase 5.
- Shadows are disabled on Android until the Bevy/Android shadow crash risk is
  re-tested.
- The Mali descriptor-pool fix is carried as a vendored `wgpu-hal` patch until
  a released wgpu-hal includes it.
- Debug APK currently embeds the release native library (~98 MB `.so`).
