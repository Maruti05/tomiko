# Build Guide

## 1. Required versions

| Tool | Version |
|---|---|
| JDK | 21 (Temurin 21.0.13.9 used here) |
| Gradle | 9.7.0 |
| AGP | 9.3.1 |
| Kotlin | 2.4.21 |
| Android SDK platform | **android-37.1** |
| Android build-tools | 36.0.0+ |
| Android NDK | **28.2.13676358** |
| Rust | 1.100.0 |
| Rust target | `aarch64-linux-android` |
| cargo-ndk | 4.1.2 |

`compileSdk = 37` with `compileSdkMinor = 1` is required by Compose 1.13.0.
`targetSdk` remains 36.

## 2. Install Android SDK components

```powershell
$sdk = "$env:LOCALAPPDATA\Android\Sdk"
& "$sdk\cmdline-tools\bin\sdkmanager.bat" --sdk_root="$sdk" "platforms;android-37.1"
& "$sdk\cmdline-tools\bin\sdkmanager.bat" --sdk_root="$sdk" "ndk;28.2.13676358"
```

Verify:

```powershell
Get-ChildItem "$env:LOCALAPPDATA\Android\Sdk\platforms"
Get-ChildItem "$env:LOCALAPPDATA\Android\Sdk\ndk"
```

## 3. Install Rust Android support

```powershell
rustup target add aarch64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
```

Set the NDK for native builds:

```powershell
$env:ANDROID_NDK_HOME = "$env:LOCALAPPDATA\Android\Sdk\ndk\28.2.13676358"
$env:ANDROID_NDK_ROOT = $env:ANDROID_NDK_HOME
```

## 4. Verify the environment

```powershell
.\scripts\verify_environment.ps1
```

The script checks Java, Rust, cargo-ndk, adb, SDK/NDK, the Rust target, Gradle
wrapper, disk space, and connected devices. It exits non-zero if a required
component is missing.

## 5. Current build (Phase 2)

The existing Compose/SceneView app still builds:

```powershell
.\gradlew.bat :app:assembleDebug
```

Rust core tests:

```powershell
Push-Location .\rust
cargo test --workspace
Pop-Location
```

Expected: 37 tests pass (9 audio + 15 character + 13 shared types).

## 6. Native + Android build (Phase 3+)

The Bevy crate (`tomiko_bevy`) lives at `rust/crates/bevy_app`:

```powershell
# Debug
.\scripts\build_android.ps1 -Mode debug

# Release APK
.\scripts\build_android.ps1 -Mode release

# Release App Bundle
.\scripts\build_android.ps1 -Mode bundle

# Install and launch the debug GameActivity on a connected device
.\scripts\run_debug.ps1

# With more than one device connected, select one
.\scripts\run_debug.ps1 -Serial 927e5fce

# Validate the character GLB structure
.\scripts\validate_character.ps1

# Regenerate the character in Blender 5.3 (requires Blender)
.\scripts\build_character.ps1
```

Release signing is configured from environment variables. Without them the
release APK is unsigned:

```powershell
$env:TOMIKO_KEYSTORE = "C:\keys\tomiko-release.jks"
$env:TOMIKO_KEYSTORE_PASSWORD = "..."
$env:TOMIKO_KEY_ALIAS = "tomiko"
$env:TOMIKO_KEY_PASSWORD = "..."
.\gradlew.bat :app:assembleRelease
```

Release builds enable R8 and resource shrinking; keep rules are limited to
manifest/JNI/serialization entry points in `app/proguard-rules.pro`.

The Rust script runs:

```powershell
$env:ANDROID_NDK_HOME = "$env:LOCALAPPDATA\Android\Sdk\ndk\28.2.13676358"
cargo ndk -t arm64-v8a -P 30 -o app\src\main\jniLibs build -p tomiko_bevy --release
```

`-P 30` pins the link platform to `minSdk`; without it, the default API 21
sysroot has no `libaaudio.so` and the link fails with
`unable to find library -laaudio`.

`cargo-ndk` inspects the highest installed NDK by default and may select r30,
which this project has not validated. Always set `ANDROID_NDK_HOME` to r28c
(the build script does this).

It copies `libc++_shared.so` from the pinned NDK if required by Bevy/wgpu.
The output is `app/src/main/jniLibs/arm64-v8a/libtomiko_game.so`. Do not point
`-o` at the `arm64-v8a` directory itself; cargo-ndk appends the ABI directory
and would leave Gradle packaging a stale library.

Observed Phase 7/8 sizes:

| Artifact | Size |
|---|---:|
| arm64 release `libtomiko_game.so` (`strip=symbols`) | 61.2 MB |
| `libc++_shared.so` | 8.81 MB |
| debug APK | 102.4 MB |
| signed release APK (R8 + resource shrink) | 67.8 MB |

## 7. AGP 9 notes

AGP 9 enables built-in Kotlin. Do **not** apply `org.jetbrains.kotlin.android`.
The project keeps:

- `com.android.application`
- `org.jetbrains.kotlin.plugin.compose`
- `org.jetbrains.kotlin.plugin.serialization`

Kotlin compiler version is selected by the KGP classpath entry in the root
`build.gradle.kts`. `android.kotlinOptions` is removed; `jvmTarget` defaults to
`compileOptions.targetCompatibility` (Java 17).

## 8. 16 KB page size

`rust/.cargo/config.toml` adds:

```toml
[target.aarch64-linux-android]
rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]
```

This is required for Android 15+ devices with 16 KB pages. Verify the resulting
`.so` with `llvm-readelf -l` (align 0x4000) before release.

## 9. Asset rules

Gradle points the app's assets at the repository-level `assets/` directory.
Do not duplicate assets under `app/src/main/assets`.

- GLB, KTX2, WAV, and OGG are marked `noCompress` to avoid double compression.
- `AssetMetaCheck::Never` is used on Android.
- No `.meta` files are required in the APK.

## 10. Troubleshooting

### `Failed to find target with hash string 'android-37.1'`

Install the platform:

```powershell
sdkmanager --sdk_root="$env:LOCALAPPDATA\Android\Sdk" "platforms;android-37.1"
```

### `Dependency ... requires compileSdk 37.1`

Keep `compileSdk = 37` and `compileSdkMinor = 1` in `app/build.gradle.kts`.

### `Cannot add extension with name 'kotlin'`

The legacy `org.jetbrains.kotlin.android` plugin is being applied. Remove it from
the root and app build files and from the version catalog.

### `cargo ndk` cannot find the NDK

Set `ANDROID_NDK_HOME` to the exact NDK directory. Do not point it at the SDK root.

### Build fails because of spaces in the path

This repository is under `C:\flutter apps\tomiko`. Quote every path in scripts.
If a third-party build script fails on spaces, move the repository to a path
without spaces and update `local.properties`.

### Missing `libc++_shared.so`

Copy it from:

```text
<NDK>/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so
```

### No device

`adb devices` must list an arm64 device with Vulkan support. Emulator-only
testing is not sufficient for audio latency, thermal, or GPU validation.

## 11. Clean build

```powershell
.\scripts\clean.ps1
.\scripts\verify_environment.ps1
.\scripts\build_android.ps1 -Mode debug
```

A clean checkout must be buildable by following this document; update it whenever
the toolchain changes.
