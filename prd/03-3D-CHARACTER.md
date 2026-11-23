# Tomiko — 3D Character & SceneView Integration

> **Superseded (November 2026):** Final rendering is Bevy 0.20.1 in a
> GameActivity. See `docs/CHARACTER_PIPELINE.md` and
> `docs/ANDROID_INTEGRATION.md`. The polygon/joint budget, clip count, and
> texture rules below are replaced by the newer character pipeline document.

## 1. Rendering Engine: SceneView (Filament for Android)

SceneView is an open-source Android library wrapping Google Filament, providing:

- glTF 2.0 / GLB model loading via `filament-gltfio`
- PBR (Physically Based Rendering) with IBL (Image-Based Lighting)
- Morph-target animation (blendshapes) for lip-sync
- Skeletal animation for body movements
- KTX2 texture compression (basis universal)
- Shadow mapping, ambient occlusion
- View/Director API for camera control

### SceneView Version

- **Library:** `io.github.sceneview:sceneview:0.15.0+`
- **Transitive Filament version:** `1.58.x`

### Compose Integration

```kotlin
@Composable
fun CharacterViewport(
    characterId: String,
    animationState: StateFlow<AnimationState>,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current

    AndroidView(
        factory = { ctx ->
            SceneView(ctx).apply {
                lifecycleOwner.lifecycle.addObserver(this)
                // Render configuration
                renderEngine?.configure {
                    transparencyOrder = TransparencyOrder.INDEPENDENT
                    ambientOcclusion?.enabled = true
                    shadowOptions?.enabled = true
                }
                // Load character glTF
                loadGlbModel(
                    assetLocation = "characters/$characterId/model.glb",
                    onLoaded = { model -> /* attach to director */ },
                    onError = { e -> /* handle */ }
                )
                // Animation controller
                ModelAnimator(modelInstance).also { animator ->
                    // Start idle animation loop
                }
            }
        },
        modifier = modifier.fillMaxSize()
    )
}
```

---

## 2. Character Asset Specification

### 2.1 glTF 2.0 Requirements

| Property | Specification |
|---|---|
| Format | **GLB** (binary glTF) — single file |
| Textures | **KTX2** with Basis Universal compression |
| Max textures | 1× diffuse, 1× ORM (occlusion-roughness-metalness), 1× emissive |
| Texture resolution | **512×512** max for mobile performance |
| Polygon budget | **≤ 5,000 triangles** per character |
| Skeleton | Max **24 joints** |
| Morph targets | **15 viseme shapes** (see §4 below) |
| Animations | **4 clips**: idle (loop), listening (loop), talking (driven), happy (once) |
| PBR | metallic-roughness workflow |

### 2.2 Character File Structure (bundled in assets/)

```
assets/characters/
├── index.json                              # Character manifest
├── character_tomiko/
│   ├── model.glb                           # Main GLB file
│   ├── thumbnail.webp                      # 256×256 preview
│   └── config.json                         # Per-character config
└── character_rabbit/
    ├── model.glb
    ├── thumbnail.webp
    └── config.json
```

### 2.3 index.json Format

```json
{
  "characters": [
    {
      "id": "tomiko",
      "name": "Tomiko",
      "thumbnail": "characters/tomiko/thumbnail.webp",
      "modelPath": "characters/tomiko/model.glb",
      "configPath": "characters/tomiko/config.json"
    },
    {
      "id": "rabbit",
      "name": "Bunny",
      "thumbnail": "characters/rabbit/thumbnail.webp",
      "modelPath": "characters/rabbit/model.glb",
      "configPath": "characters/rabbit/config.json"
    }
  ]
}
```

### 2.4 character config.json

```json
{
  "scale": 1.0,
  "initialRotation": [0.0, 180.0, 0.0],
  "camera": {
    "distance": 2.5,
    "angleX": 15.0,
    "angleY": 0.0
  },
  "animations": {
    "idle": { "clip": 0, "speed": 0.8, "loop": true },
    "listening": { "clip": 1, "speed": 1.0, "loop": true },
    "talking": { "clip": 2, "speed": 1.0, "loop": false },
    "happy": { "clip": 3, "speed": 1.2, "loop": false }
  },
  "morphTargets": {
    "viseme_AA": 0,
    "viseme_E": 1,
    "viseme_I": 2,
    "viseme_O": 3,
    "viseme_U": 4,
    "viseme_M": 5,
    "viseme_F": 6,
    "viseme_L": 7,
    "viseme_R": 8,
    "viseme_S": 9,
    "viseme_SH": 10,
    "viseme_TH": 11,
    "viseme_W": 12,
    "viseme_REST": 13,
    "viseme_BLINK": 14
  }
}
```

---

## 3. SceneManager — Lifecycle-Aware Controller

```kotlin
// data/scene/SceneManager.kt
class SceneManager(private val sceneView: SceneView) {

    private var currentModel: ModelInstance? = null
    private var animator: ModelAnimator? = null

    suspend fun loadCharacter(context: Context, config: CharacterConfig): Result<Unit> {
        return withContext(Dispatchers.Main) {
            try {
                val model = sceneView.loadGlbModel(
                    assetLocation = config.modelPath,
                    lifecycle = /* lifecycle owner */
                ).await()
                currentModel = model
                animator = ModelAnimator(model)
                applyInitialPose(config)
                Result.success(Unit)
            } catch (e: Exception) {
                Result.failure(e)
            }
        }
    }

    fun playAnimation(clipIndex: Int, loop: Boolean, speed: Float = 1f) {
        animator?.play(clipIndex, loop = loop, speed = speed)
    }

    fun applyMorphTarget(targetIndex: Int, weight: Float) {
        currentModel?.morphTargets?.forEachIndexed { i, target ->
            target.weight = if (i == targetIndex) weight else 0f
        }
    }

    fun clearMorphTargets() {
        currentModel?.morphTargets?.forEach { it.weight = 0f }
    }

    fun release() {
        currentModel?.destroy()
        sceneView.destroy()
    }
}
```

---

## 4. Lip-Sync Morph Target Specification

The character model **must** contain **15 morph targets** (blendshapes) named after visemes:

| Index | Name | Mouth Shape |
|---|---|---|
| 0 | `viseme_AA` | Wide open "ah" |
| 1 | `viseme_E` | Spread "eh/ee" |
| 2 | `viseme_I` | Slight spread "ih" |
| 3 | `viseme_O` | Rounded "oh" |
| 4 | `viseme_U` | Pursed "oo" |
| 5 | `viseme_M` | Lips closed "m/b/p" |
| 6 | `viseme_F` | Upper teeth on lower lip "f/v" |
| 7 | `viseme_L` | Tongue up "l" |
| 8 | `viseme_R` | Tongue curled "r" |
| 9 | `viseme_S` | Teeth together "s/z" |
| 10 | `viseme_SH` | Protruded "sh/ch/j" |
| 11 | `viseme_TH` | Tongue out "th" |
| 12 | `viseme_W` | Rounded "w" |
| 13 | `viseme_REST` | Neutral mouth |
| 14 | `viseme_BLINK` | Eyes closed (blink) |

**Modeling guidelines for Blender / Maya exporters:**
- Export as **glTF 2.0** with `MORPH_TARGET` extension
- All morph targets must share the same vertex count and topology (no re-meshing)
- Weights must be in range `[0.0, 1.0]`
- The GLB file **must not** exceed **3 MB** with KTX2 textures

---

## 5. Lighting & Environment

| Property | Setting |
|---|---|
| IBL | Pre-baked HDR environment (512×512 KTX2) bundled in assets |
| Main light | Directional, angle 45°, intensity 50,000 lux |
| Ambient occlusion | SSAO, enabled, medium quality |
| Shadows | PCF (percentage-closer filtering), 1024×1024 map |
| Tone-mapping | Filmic (ACES) |

**IBL file location:** `assets/environment/env_playground.ktx2`

---

## 6. Performance Tuning for Filament/SceneView

| Tuning | Setting | Rationale |
|---|---|---|
| Render resolution | 720p on < 4 GB, 1080p on 4 GB+ | Balance quality vs fill rate |
| MSAA | 2× on 2 GB, 4× on 4 GB+ | Reduce aliasing without heavy cost |
| Texture filtering | Trilinear (no anisotropy) | Good quality, low cost |
| Viewport clear | Skip clear if full-screen | Avoids unnecessary GPU work |
| Culling | Frustum + occlusion | Filament auto-handles |
| Animated morph targets | Update only on weight change | Avoid per-frame uniform upload |
| Garbage | Pool morph weight arrays | Prevent JNI array allocation spam |

---

## 7. Asset Pipeline for Developers

```
Blender / Maya
    │
    ├── Export glTF 2.0 (separate .bin + .gltf + textures)
    │
    ├── gltf-transform (CLI):
    │   │   gltf-transform optimize model.glb model_opt.glb
    │   │   gltf-transform draco model_opt.glb model_draco.glb
    │   │   gltf-transform resize --width 512 --height 512 model_draco.glb model_final.glb
    │   │
    │   └── Result: KTX2 textures embedded, Draco mesh compression
    │
    └── Place in assets/characters/<id>/model.glb
```

**Tools:**
- **Blender 4.x** with glTF 2.0 exporter
- **glTF Transform** (`npm install -g @gltf-transform/cli`)
- **KTX2** via `toktx` (Basis Universal encoder from Khronos)
