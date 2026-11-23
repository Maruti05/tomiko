# Tomiko — Android Compatibility & Adaptive Rules

## 1. Android Version Support Matrix

| API Level | Version | Support | Special Considerations |
|---|---|---|---|
| **30** | **Android 11** | ✅ Target min | Edge-to-edge must be manual (no `enableEdgeToEdge` available — use `WindowCompat.setDecorFitsSystemWindows`) |
| 31 | Android 12 | ✅ Full | Material You dynamic color; `SCHEDULE_EXACT_ALARM` not needed |
| 32 | Android 12L | ✅ Full | Enhanced foldable / tablet support |
| 33 | Android 13 | ✅ Full | Granular media permission (`READ_MEDIA_AUDIO` — not needed since we use app-private dirs) |
| 34 | Android 14 | ✅ Full | Foreground service types (not used); `NO_MANIFEST_COMPAT` for audio focus |
| **35** | **Android 15** | ✅ Target | 16 KB page alignment for native libs; App Functions (not used) |
| **36** | **Android 16** | ✅ Target (compileSdk) | Ensure adaptive layouts with `WindowSizeClass` API |

### Backward Compatibility Checklist for API 30

- [ ] `enableEdgeToEdge()` is API 35+ — use compat fallback for API 30–34
- [ ] `ActivityResultContracts.RequestPermission` works from API 30+
- [ ] `WindowInsetsControllerCompat` for status bar appearance (API 30+)
- [ ] `AudioTrack` with `MODE_STATIC` works on API 30+ (confirmed)
- [ ] `AudioRecord` with `AudioSource.MIC` works on API 30+
- [ ] `Lifecycle` for SceneView (API 30+)
- [ ] GLES 3.0 required for Filament — verify via `PackageManager.FEATURE_OPENGLES_EXTENSION_PACK` or fallback gracefully

---

## 2. Edge-to-Edge Compat Implementation

```kotlin
// MainActivity.kt API 30+ compatible edge-to-edge
@OptIn(ExperimentalComposeApi::class)
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    if (Build.VERSION.SDK_INT >= 35) {
        enableEdgeToEdge() // Native API 35+
    } else {
        // API 30–34 manual approach
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).let { controller ->
            controller.isAppearanceLightStatusBars = false
            controller.isAppearanceLightNavigationBars = false
            controller.hide(WindowInsetsCompat.Type.systemBars())
        }
    }

    setContent { TomikoTheme { TomikoApp() } }
}
```

---

## 3. Display Cutout (Notch) Handling

```kotlin
@Composable
fun SafeSceneView(modifier: Modifier = Modifier) {
    val cutout = LocalWindowInsets.current.displayCutout
    val safeInsets = Modifier.windowInsetsPadding(
        WindowInsets.systemBars.only(WindowInsets.Side.Left or WindowInsets.Side.Right)
    )

    Box(modifier = modifier) {
        // Character renders full-screen behind cutout
        CharacterViewport(modifier = Modifier.fillMaxSize())

        // Controls respect safe area
        RecordButton(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = WindowInsets.systemBars
                    .getBottom(LocalDensity.current).dp)
        )
    }
}
```

---

## 4. Tablet & Foldable Support

### 4.1 Screen Size Buckets

| Category | Width (dp) | Layout | Character size |
|---|---|---|---|
| Small phone | < 400 | Portrait vertical | 60 % height |
| Large phone | 400–600 | Portrait or landscape | 55 % height |
| Small tablet (7″) | 600–720 | Landscape split | 50 % width |
| Large tablet (10″) | 720–840 | Landscape split | 55 % width |
| Foldable unfolded | 840+ | Dual-pane | 50 % width |
| Foldable folded | < 600 | Phone layout | 60 % height |

### 4.2 Foldable Hinge Detection

```kotlin
@Composable
fun rememberHingeBounds(): Rect? {
    val windowInfo = rememberWindowLayoutInfo()
    return windowInfo.windowLayoutInfo.displayFeatures
        .filterIsInstance<FoldingFeature>()
        .firstOrNull()
        ?.bounds
        ?.let { Rect(it.left.toDp(), it.top.toDp(), it.right.toDp(), it.bottom.toDp()) }
}

@Composable
fun TalkScreenWithHinge(characterId: String) {
    val hinge = rememberHingeBounds()

    if (hinge != null) {
        // Render character above hinge, controls below
        Column {
            CharacterViewport(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(hinge.top)
            )
            FoldingFeatureSeparator(hinge)
            TalkControls(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(rest)
            )
        }
    } else {
        NormalTalkLayout(characterId)
    }
}
```

### 4.3 Multi-Window / Freeform

- Activity is `resizeableActivity = true` (required for Android 12L+)
- `android.supports_size_changes = true` in manifest
- Configuration change handling: `onSavedInstanceState` for ongoing recording state

---

## 5. Configuration Changes

```kotlin
// Prevent activity restart on config change (we handle it manually)
// AndroidManifest.xml:
// android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboardHidden"

// In Activity:
override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    // Re-layout Compose tree — composables handle via WindowSizeClass
}
```

---

## 6. Audio Focus Handling

```kotlin
class AudioFocusHelper(private val context: Context) {
    private val focusManager = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

    fun requestFocus(): Boolean {
        val result = focusManager.requestAudioFocus(
            { _, _ -> /* handle focus change */ },
            AudioManager.STREAM_MUSIC,
            AudioManager.AUDIOFOCUS_GAIN_TRANSIENT
        )
        return result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
    }

    fun abandonFocus() {
        focusManager.abandonAudioFocus(null)
    }
}
```

---

## 7. Accessibility

| Requirement | Implementation |
|---|---|
| Content descriptions | All icon buttons have `contentDescription` |
| Touch target size | All interactive elements ≥ 48×48 dp |
| Font scaling | `fontScale` from system; test at 1.5× |
| High contrast | M3 dynamic color ensures ≥ 7:1 contrast |
| TalkBack | `focusable = true` on character viewport; `stateDescription` updates with phase |
| Reduced motion | Animation speed reduced via `animatorDurationScale` check |

### Reduced Motion Detection

```kotlin
@Composable
fun rememberAnimationsEnabled(): Boolean {
    val context = LocalContext.current
    val animatorScale = Settings.Global.getFloat(
        context.contentResolver,
        Settings.Global.ANIMATOR_DURATION_SCALE,
        1f
    )
    return animatorScale > 0f
}
```
