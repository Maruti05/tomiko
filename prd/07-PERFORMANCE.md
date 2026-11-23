# Tomiko — Performance Optimization Guidelines

> **Superseded (November 2026):** See `docs/PERFORMANCE.md` for the Bevy/Rust
> performance plan. SceneView/Filament-specific tuning below no longer applies.
> No target is considered achieved without real-device measurements.

## 1. Frame Rate Budget

Target: **45–60 FPS** on 2 GB devices, **stable 60 FPS** on 4 GB+ devices.

| Frame Budget (16.6 ms @ 60 FPS) | Allocation |
|---|---|
| Compose UI rendering | ≤ 2 ms |
| ViewModel / State updates | ≤ 1 ms |
| SceneView (Filament) GPU | ≤ 8 ms |
| Audio buffer processing | ≤ 2 ms |
| Lip-sync engine | ≤ 1 ms |
| Buffer / overhead | ≤ 2.6 ms |

---

## 2. GPU Optimization

### 2.1 Filament / SceneView Tuning

```kotlin
// Performance-optimized SceneView configuration
sceneView.renderEngine?.configure {
    // Resolution scaling
    renderQuality = when (totalMemoryGB) {
        < 3 -> RenderQuality.MEDIUM    // 0.75× scale
        < 4 -> RenderQuality.HIGH      // 1.0× scale
        else -> RenderQuality.ULTRA    // 1.0× scale + MSAA 4×
    }

    // Shadows
    shadowOptions = ShadowOptions.Builder()
        .mapSize = 512                 // 1024 on 4 GB+
        .vsmShadowSampling = false     // (VSM is more expensive)
        .build()

    // Ambient occlusion
    ambientOcclusion = AmbientOcclusionOptions.Builder()
        .enabled = totalMemoryGB >= 3
        .quality = AmbientOcclusionQuality.LOW
        .build()

    // Post-processing
    bloomEnabled = false
    dithering = Dithering.NONE
    antiAliasing = AntiAliasing.NONE  // we use MSAA instead
}
```

### 2.2 Runtime Device Detection

```kotlin
// Util: detect device class at startup
object DeviceClassifier {
    val totalMemoryGB: Int
        get() {
            val am = context.getSystemService(Context.ACTIVITY_SERVICE)
                    as ActivityManager
            val mi = ActivityManager.MemoryInfo()
            am.getMemoryInfo(mi)
            return (mi.totalMem / (1024 * 1024 * 1024)).toInt()
        }

    val isLowEnd: Boolean get() = totalMemoryGB < 3 || isSlowGpu()

    private fun isSlowGpu(): Boolean {
        // Check GPU renderer string for known slow chips
        val renderer = GLES20.glGetString(GLES20.GL_RENDERER)
        return renderer?.contains("Mali-400") == true ||
               renderer?.contains("Adreno 3") == true
    }
}
```

---

## 3. Memory Optimization

### 3.1 Heap Budget

| Category | Budget (2 GB device) | Budget (4 GB+ device) |
|---|---|---|
| Compose UI | 8 MB | 16 MB |
| SceneView (Filament) | 48 MB | 64 MB |
| Audio buffers | 4 MB | 8 MB |
| Asset cache | 8 MB | 24 MB |
| Kotlin runtime | 8 MB | 12 MB |
| **Total peak** | **≤ 80 MB** | **≤ 128 MB** |

### 3.2 Memory-Saving Techniques

```kotlin
// 1. Pool audio buffers to avoid GC churn
object AudioBufferPool {
    private val pool = ObjectPool<ShortArray>(
        factory = { ShortArray(4096) },
        maxSize = 4
    )

    fun obtain(): ShortArray = pool.acquire()
    fun recycle(array: ShortArray) { pool.release(array) }
}

// 2. Release SceneView when not visible
@Composable
fun CharacterViewport(...) {
    val isVisible by remember { mutableStateOf(true) }
    DisposableEffect(isVisible) {
        if (!isVisible) sceneManager.pauseRendering()
        onDispose { sceneManager.release() }
    }
}

// 3. Compose: use derivedStateOf to reduce recomposition
val animationState by viewModel.uiState
    .map { it.phase.toAnimationState() }
    .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), AnimationState.IDLE)

val characterState by remember { derivedStateOf { animationState } }

// 4. Avoid lambda allocations in composable calls
// BAD:  Button(onClick = { viewModel.onClick() }) ← allocates per recomposition
// GOOD: val onClick = remember { { viewModel.onClick() } }
//        Button(onClick = onClick)
```

---

## 4. Battery Optimization

| Technique | Impact |
|---|---|
| Reduce rendering to 30 FPS when no animation plays (5 s idle) | −40 % GPU power |
| Release audio wakelock immediately after playback | −15 % CPU |
| Batch file writes; avoid per-byte writes | −5 % CPU |
| Use `AudioTrack.MODE_STATIC` instead of `MODE_STREAM` | −10 % CPU |
| Disable shadow maps when character is very small on screen | −8 % GPU |

### Frame Rate Governor

```kotlin
class FrameRateGovernor(private val sceneView: SceneView) {
    private var lastActiveTime = 0L
    private var isThrottled = false

    fun onUserInteraction() {
        lastActiveTime = SystemClock.uptimeMillis()
        if (isThrottled) {
            sceneView.renderEngine?.framerateOptions = FramerateOptions(60f)
            isThrottled = false
        }
    }

    fun checkThrottle() {
        val idleTime = SystemClock.uptimeMillis() - lastActiveTime
        if (idleTime > 5_000 && !isThrottled) {
            sceneView.renderEngine?.framerateOptions = FramerateOptions(30f)
            isThrottled = true
        }
    }
}
```

---

## 5. APK Size Budget

| Component | Target Size |
|---|---|
| Kotlin + Compose runtime | ~4 MB |
| SceneView / Filament libs | ~6 MB (arm64) |
| 1 character (GLB + KTX2) | ~3 MB |
| NDK shared lib (.so) | ~1 MB |
| IBL environment | ~0.5 MB |
| Sound effects | ~0.5 MB |
| App code + resources | ~5 MB |
| **Total base APK** | **≤ 20 MB** |
| **Target AAB (Play Store)** | **≤ 40 MB** (with per-density splits) |

### Size-Saving Techniques

| Technique | Savings |
|---|---|
| R8 full mode (minifyEnabled = true) | −30 % code size |
| Android App Bundle (not APK) | −40 % download size |
| KTX2 textures (instead of PNG) | −60 % texture size |
| Draco mesh compression | −50 % geometry size |
| arm64-v8a only (drop armeabi-v7a if min API 30) | −30 % native lib size |
| Remove unused resources (lint check) | −5 % |

### R8 Rules

```
# proguard-rules.pro
-keep class com.vedica.labs.ind.app.talkin.tomiko.** { *; }
-dontwarn io.github.sceneview.**
-keep class io.github.sceneview.** { *; }
-keep class com.google.android.filament.** { *; }
```

---

## 6. Startup Time Optimization

| Phase | Target | Technique |
|---|---|---|
| Cold start (splash visible) | < 1 s | Baseline profile |
| Character load | < 2 s | Async glTF load with progress |
| First record ready | < 3 s | Pre-allocate AudioRecord |

### Baseline Profile (baseline-prof.txt)

Include a baseline profile in `src/main/baseline-prof.txt` to pre-compile critical paths:

```
HSPL com/vedica/labs/ind/app/talkin/tomiko/MainActivity
HSPL com/vedica/labs/ind/app/talkin/tomiko/presentation/home/HomeScreen
HSPL com/vedica/labs/ind/app/tomiko/presentation/talk/TalkScreen
HSPL io/github/sceneview/SceneView
HSPL android/media/AudioRecord
```

---

## 7. Performance Monitoring (Self-Contained)

No external APM tools. Use debug-only logging:

```kotlin
object PerformanceMonitor {
    private val timers = mutableMapOf<String, Long>()

    fun startTrace(key: String) { timers[key] = SystemClock.elapsedRealtimeNanos() }

    fun endTrace(key: String) {
        val start = timers.remove(key) ?: return
        val elapsed = (SystemClock.elapsedRealtimeNanos() - start) / 1_000_000
        if (elapsed > SLOW_THRESHOLD_MS) {
            Log.w("TomikoPerf", "$key took ${elapsed}ms (threshold: ${SLOW_THRESHOLD_MS}ms)")
        }
    }

    // Frame time callback (SceneView provides this via Renderer)
    fun onFrameTime(frameTimeMs: Float) {
        frameTimes.add(frameTimeMs)
        if (frameTimeMs > 33f) { // < 30 FPS
            Log.w("TomikoPerf", "Jank: ${frameTimeMs}ms frame time")
        }
    }
}
```
