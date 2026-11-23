# Phase 3 Device Test Plan

Runtime verification is **blocked until an arm64 Android device with Vulkan
support is connected via adb**. Emulator results are not sufficient for audio
latency, thermal, or GPU performance validation.

## 1. Prerequisites

- Android 11+ (API 30+), arm64-v8a.
- USB debugging enabled.
- `adb devices` shows the device as `device`, not `unauthorized`.
- App installed: debug package `com.vedica.labs.ind.app.talkin.tomiko.debug`.

## 2. Build and launch

```powershell
.\scripts\verify_environment.ps1 -RequireDevice
.\scripts\run_debug.ps1
```

Or manually:

```powershell
.\scripts\build_android.ps1 -Mode debug
adb install -r app\build\outputs\apk\debug\app-debug.apk
adb shell am start -n com.vedica.labs.ind.app.talkin.tomiko.debug/com.vedica.labs.ind.app.talkin.tomiko.game.CharacterGameActivity
```

## 3. Logcat

```powershell
adb logcat -c
adb logcat -s Tomiko:* bevy:* wgpu:* AndroidRuntime:E
```

Look for:

- `Tomiko native command receiver attached`
- wgpu adapter/adapter limits
- `WorldAssetRoot`/GLB load success
- no `UnsatisfiedLinkError`
- no `VK_ERROR_INITIALIZATION_FAILED`
- no `ANR`/`FATAL EXCEPTION`

## 4. Visual checks

- [ ] Full-screen black background becomes the Bevy scene.
- [ ] Ground plane, rotating cube, and placeholder GLB are visible.
- [ ] No upside-down/inside-out model.
- [ ] No severe z-fighting or missing materials.
- [ ] Touch gestures do not crash the activity.
- [ ] Home/back gestures behave predictably.

## 5. Lifecycle checks

- [ ] Press Home → `onPause` runs; no crash.
- [ ] Return → `onResume` runs; rendering resumes.
- [ ] Rotate/change configuration (if allowed) → no native crash.
- [ ] Open recents and swipe away → `onDestroy` runs; no crash.
- [ ] Incoming call / audio focus change → no crash (audio comes in Phase 5).
- [ ] Repeated launch/close 10 times → no leak or `UnsatisfiedLinkError`.

## 6. JNI checks

```powershell
adb shell run-as com.vedica.labs.ind.app.talkin.tomiko.debug ls app_lib
```

The APK must contain `lib/arm64-v8a/libtomiko_game.so`. The exported symbols were
verified at build time with `llvm-nm`.

Expected JNI behavior:

- `nativeInitialize()` returns `0`.
- `nativeGetAbiVersion()` returns `1`.
- `nativePing()` returns `0x544F4D49`.
- `nativeStartListening()` returns `0` and the command is logged by Bevy.
- `nativeShutdown()` is idempotent.

## 7. Performance capture (baseline only)

```powershell
adb shell dumpsys gfxinfo com.vedica.labs.ind.app.talkin.tomiko.debug
adb shell dumpsys meminfo com.vedica.labs.ind.app.talkin.tomiko.debug
```

Do not claim 60 FPS or low latency from the Phase 3 test scene. It exists to
validate the native foundation, not final performance.

## 8. Known failure modes

| Symptom | Likely cause | Action |
|---|---|---|
| `UnsatisfiedLinkError: dlopen failed` | Wrong ABI or missing `libtomiko_game.so` | Check APK `lib/arm64-v8a`; rebuild release `.so` |
| `UnsatisfiedLinkError: No implementation found` | Kotlin/Rust signature mismatch | Compare `javap` and `llvm-nm` |
| Black screen, no logs | Surface/lifecycle issue | Check GameActivity theme and `android.app.lib_name` |
| `VK_ERROR_INITIALIZATION_FAILED` | Device/driver Vulkan issue | Capture GPU/OS details; test another device |
| GLB not visible | Asset path/case or scale | Check `assets/characters/trex/model.glb`; inspect transform |
| Crash on pause/resume | Native lifecycle double-free | Add logs around `nativePause`/`nativeResume`; check idempotency |
| APK too large / install fails | 98 MB `.so` + assets | Use release build; size optimization in Phase 7 |
