# Android Integration

## 1. Activities

| Activity | Base class | Purpose |
|---|---|---|
| `MainActivity` | `ComponentActivity` | Compose shell: onboarding, selection, presets, settings, about, privacy |
| `CharacterGameActivity` | `GameActivity` | Full-screen Bevy character stage, audio, touch, optional overlay |

The onboarding flow launches `CharacterGameActivity` when complete. The game
activity is the primary character experience; Compose remains the UI framework
for non-3D screens.

## 2. Manifest

```xml
<activity
    android:name=".game.CharacterGameActivity"
    android:exported="true"
    android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboardHidden|density|uiMode"
    android:theme="@style/Theme.Tomiko.Game">
    <meta-data
        android:name="android.app.lib_name"
        android:value="tomiko_game" />
</activity>
```

`android.app.lib_name` tells `android-activity` which cdylib to load. The Kotlin
activity also calls `System.loadLibrary("tomiko_game")` in a static initializer.

## 3. Native library

One cdylib:

```text
app/src/main/jniLibs/arm64-v8a/libtomiko_game.so
app/src/main/jniLibs/arm64-v8a/libc++_shared.so   (if required by Bevy/wgpu)
```

The Rust crate is `tomiko_bevy` in `rust/crates/bevy_app` with:

```toml
[package]
name = "tomiko_bevy"

[lib]
name = "tomiko_game"
crate-type = ["cdylib"]
```

## 4. JNI API

Kotlin class: `com.vedica.labs.ind.app.talkin.tomiko.native.NativeEngine`

| Function | Direction | Frequency | Purpose |
|---|---|---|---|
| `nativeInitialize(config: String): Int` | K → R | Once | Create engine, set quality/audio config |
| `nativeStartListening(): Int` | K → R | User action | Begin capture/VAD |
| `nativeStopListening(): Int` | K → R | User action | Stop capture |
| `nativePlayLast(): Int` | K → R | User action | Replay last processed audio |
| `nativeSetVoicePreset(id: Int, intensity: Float): Int` | K → R | User action | Change preset |
| `nativeSetCharacter(id: String): Int` | K → R | User action | Swap character |
| `nativeOnTouch(zone: Int, x: Float, y: Float): Int` | K → R | Touch | Reaction |
| `nativeSetQuality(tier: Int): Int` | K → R | Settings | Quality tier |
| `nativePause(): Int` | K → R | Lifecycle | Pause audio/render |
| `nativeResume(): Int` | K → R | Lifecycle | Resume |
| `nativeGetEngineState(): Int` | K → R | ~10 Hz | Engine state |
| `nativeGetCharacterState(): Int` | K → R | ~10 Hz | Character state |
| `nativeGetAmplitude(): Float` | K → R | ~10 Hz | Input/output level |
| `nativeGetProgress(): Float` | K → R | ~10 Hz | Playback progress |
| `nativeGetMouthOpen(): Float` | K → R | ~10 Hz | Mouth blend |
| `nativeGetErrorCode(): Int` | K → R | Error only | Last native error |
| `nativeGetAbiVersion(): Int` | K → R | Once | Contract version |
| `nativePing(): Int` | K → R | Integration test | Library/JNI check |
| `nativeShutdown(): Int` | K → R | Once | Release |

The individual getters above are the Phase 3 contract. Phase 5 will replace the
UI poll with one packed `NativeUiState` direct-buffer call to reduce JNI crossings
from six calls to one per poll. The final ABI will be versioned.

Rules:

- No per-frame JNI.
- No per-audio-buffer JNI.
- Phase 3 uses bounded individual getters at ~10 Hz; Phase 5 introduces the
  packed 32-byte `#[repr(C)]` `NativeUiState` direct-buffer call.
- Every command function returns an `i32` error code; no Rust panic or raw
  error text crosses JNI.
- Kotlin validates inputs before calling.
- Rust catches panics at the boundary and returns `Panic`.

## 5. Lifecycle

| Android event | Action |
|---|---|
| `onCreate` | Load library, initialize native engine after permission/quality detection |
| `onStart` | Prepare audio focus request |
| `onResume` | `nativeResume()`; request audio focus when entering talk mode |
| `onPause` | `nativePause()`; abandon audio focus |
| `onStop` | Stop capture; release focus |
| `onDestroy` | `nativeShutdown()`; release JNI references |
| `onWindowFocusChanged` | Hide system bars only when focused |

`nativeShutdown` is idempotent. Do not call native functions after shutdown.

## 6. GameActivity + Compose overlay

GameActivity creates a `FrameLayout` containing an `InputEnabledSurfaceView`.
A Compose overlay can be added as a sibling above the surface.

Phase 3 spike:

1. Subclass `GameActivity`.
2. Override `onCreateSurfaceView()`.
3. After `super`, add a `ComposeView`/native overlay to the content `FrameLayout`.
4. Keep the overlay static; update only on state changes.
5. Measure frame time with and without the overlay.

Fallback if overlay is unstable or too expensive:

- Native Android Views for the HUD.
- Bevy UI for the HUD.
- Separate Compose activities for settings/presets.

Do not assume the overlay is free; SurfaceView overlays can trigger a
full-screen alpha composite.

## 7. Assets

- Gradle adds `assets/` to the app source set.
- Bevy uses `AndroidAssetReader` backed by `AssetManager`.
- `AssetMetaCheck::Never`.
- GLB/KTX2/WAV/OGG are `noCompress`.
- No `.meta` files in the APK.
- Asset paths are lowercase and stable.

## 8. Packaging and R8

- `packaging.jniLibs.useLegacyPackaging = false`.
- 16 KB page alignment via Rust linker flag.
- Keep rules for:
  - `com.vedica.labs.ind.app.talkin.tomiko.native.**`
  - `com.google.androidgamesdk.**`
  - JNI-called classes/methods.
  - Bevy/wgpu JNI entry points as required.
- Test release builds, not only debug.
- Add `consumer-rules.pro` if a `:game` module is introduced later.

## 9. Debug vs release

| Setting | Debug | Release |
|---|---|---|
| Minify | Off | On |
| Resource shrink | Off | On |
| Diagnostics overlay | Available | Off by default |
| Logging | Verbose, sanitized | Error only |
| ABI | arm64 (+ optional x86_64 for emulator) | arm64 |
| Signing | Debug | User keystore |

## 10. Verification checklist

- App launches to the character stage.
- Library loads on arm64.
- No `UnsatisfiedLinkError`.
- `onPause`/`onResume` do not leak or double-open audio.
- `onDestroy` releases native resources.
- Incoming call pauses capture/playback.
- Permission denial and permanent denial are handled.
- Release APK/AAB installs and runs.
- 16 KB page device runs without linker errors.
