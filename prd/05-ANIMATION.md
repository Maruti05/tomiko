# Tomiko — Animation State Machine

> **Superseded (November 2026):** The character state machine now lives in Rust
> and exposes 10 states through Bevy animation events. See
> `docs/ARCHITECTURE.md` and `rust/crates/character_engine`.

## 1. State Machine Overview

```
                    ┌─────────┐
   App Start ──────▶│  IDLE   │◀────────────────────┐
                    └────┬────┘                     │
                         │                          │
                    [Tap Record]                [Replay]
                         │                          │
                    ┌────▼──────┐                   │
                    │ LISTENING │                   │
                    │ (waiting) │                   │
                    └────┬──────┘                   │
                         │                          │
                    [Audio captured]            [Replay pressed]
                         │                          │
                    ┌────▼──────┐              ┌────┴────┐
                    │PROCESSING │              │ PLAYING │
                    │ (loading) │──────────────▶│ (active)│
                    └───────────┘   Effect done └────┬────┘
                                                      │
                                                   [Done]
                                                      │
                                                  ┌───▼────┐
                                                  │ HAPPY  │
                                                  │(idle +)│
                                                  └───┬────┘
                                                      │
                                                  [Timeout 3s]
                                                      │
                                                  ┌────▼────┐
                                                  │  IDLE   │
                                                  └─────────┘
```

## 2. Animation Clips per Character

Each character GLB must contain exactly 4 animation clips:

| Clip Index | Name | Type | Loop | Description |
|---|---|---|---|---|
| 0 | `idle` | Skeletal | Yes | Breathing, slight sway, random blink |
| 1 | `listening` | Skeletal | Yes | Head tilt, ear perk, "I'm listening" pose |
| 2 | `talking` | Morph | No | Driven by lip-sync engine, no body movement |
| 3 | `happy` | Skeletal | No | Jump, spin, celebration (2–3 s) |

**Transition blending** — all transitions use `ModelAnimator.crossfade()` with 250 ms overlap.

---

## 3. AnimationController Implementation

```kotlin
class AnimationController(private val sceneManager: SceneManager) {

    private val transitionDuration = 250L

    fun transitionTo(state: AnimationState) {
        when (state) {
            AnimationState.IDLE -> {
                sceneManager.playAnimation(clipIndex = 0, loop = true, speed = 0.8f)
                sceneManager.startRandomBlink(intervalRange = 2_000L..6_000L)
            }

            AnimationState.LISTENING -> {
                sceneManager.playAnimation(clipIndex = 1, loop = true, speed = 1.0f)
                sceneManager.crossfade(duration = transitionDuration)
            }

            AnimationState.TALKING -> {
                // Morph targets driven externally by LipSyncEngine
                sceneManager.playAnimation(clipIndex = 2, loop = false, speed = 1.0f)
                sceneManager.fadeOutSkeletal(weight = 0.3f) // reduce body movement
            }

            AnimationState.HAPPY -> {
                sceneManager.playAnimation(clipIndex = 3, loop = false, speed = 1.2f)
            }
        }
    }
}
```

---

## 4. Random Idle Behaviors

To make the character feel alive during IDLE state:

| Behavior | Trigger | Implementation |
|---|---|---|
| **Blink** | Random every 2–6 s | Morph target `viseme_BLINK` weight 0→1→0 over 150 ms |
| **Head turn** | Random every 10–15 s | Slight bone rotation, then return |
| **Sway** | Continuous | Subtle sine-wave on root bone Y rotation (±3°) |
| **Look at mic** | On IDLE→LISTENING | Head bone rotates toward recording button position |

---

## 5. Talking Animation — Lip-Sync Integration

During playback, the `TalkViewModel` coordinates `LipSyncEngine` output → `SceneView` morph targets:

```kotlin
// In TalkViewModel
fun playWithLipSync(audioFile: File) {
    viewModelScope.launch {
        val pcm = readWavData(audioFile)
        val visemes = lipSyncEngine.process(pcm)

        // Start playback (coroutine 1)
        launch {
            audioPlayer.play(audioFile, onProgress = { progress ->
                _uiState.update { it.copy(playbackProgress = progress) }
            }, this)
        }

        // Drive morph targets (coroutine 2, synced to audio time)
        val frameDuration = 512f / 44100f // ~11.6 ms per viseme frame
        launch {
            var headIndex = 0
            while (headIndex < visemes.size) {
                val frame = visemes[headIndex]
                val smoothed = lipSyncEngine.smooth(frame)
                sceneManager.applyMorphTarget(smoothed.index, smoothed.weight)
                delay((frameDuration * 1000).toLong())
                headIndex++
            }
            // Clear morphs
            sceneManager.clearMorphTargets()
        }
    }
}
```

---

## 6. Animation State Data Flow

```
TalkViewModel (StateFlow<AnimationState>)
    │
    ├──> CharacterViewport (Composable)
    │       │
    │       ├── collectAsState() — watches animation state
    │       │
    │       └── LaunchedEffect(state) {
    │               animationController.transitionTo(state)
    │           }
    │
    └──> RecordButton / PlaybackControls
            │
            └── onClick → viewModel.onRecordPressed()
                              ↓
                          Phase change
                              ↓
                      _uiState.update { phase = ... }
                              ↓
                      Derive AnimationState from Phase
```

### State Mapping

| TalkUiState.Phase | AnimationState |
|---|---|
| IDLE | IDLE |
| RECORDING | LISTENING |
| PROCESSING | IDLE (brief) |
| PLAYING | TALKING |
| DONE | HAPPY → IDLE (auto after 3s) |
