# Tomiko — LLM Prompting Instructions for Module-by-Module Generation

## Purpose

This document instructs how to feed the PRD documents to an LLM (GPT-4, Claude, Gemini, etc.) to generate the complete Android project module-by-module without breaking the architectural integrity.

---

## 1. Ground Rules for the LLM

```
You are generating a production-quality Android application called "Tomiko" for kids.
Package name: com.vedica.labs.ind.app.talkin.tomiko
Min SDK: 30, Target SDK: 36, Language: Kotlin 2.1.x

CRITICAL RULES:
1. NEVER add INTERNET permission or any network calls.
2. NEVER add Room, SQLite, Hilt, Dagger, or any third-party DI framework.
3. NEVER add analytics, crash reporting, or ad SDKs.
4. Code must compile for API 30 (Android 11) through API 36 (Android 16).
5. ALL file operations use app-private directories (Context.filesDir).
6. Use Kotlin Coroutines + Flow for async. NO RxJava, LiveData, or AsyncTask.
7. ALL Compose state must use StateFlow from ViewModels (no mutableStateOf in VM).
8. 3D rendering uses SceneView (io.github.sceneview:sceneview:0.15.0).
9. Material 3 only. No Material 2 imports.
10. Each response must include the FULL file content, never "rest of code unchanged".
11. Follow the exact package structure defined in the Architecture document.
```

---

## 2. Module Generation Sequence

Generate modules in this **strict order** to satisfy dependencies:

```
Phase 1: Foundation (no Android framework dep)
  Step 1  → Domain models + Enums
  Step 2  → Repository interfaces
  Step 3  → Use cases

Phase 2: Data layer (Android framework dep)
  Step 4  → FileManager + JsonConfigStore
  Step 5  → AudioRecorder + AudioPlayer + WavFileWriter
  Step 6  → AudioProcessor (JNI bridge)
  Step 7  → LipSyncEngine
  Step 8  → SceneManager + CharacterLoader + AnimationController
  Step 9  → Repository implementations

Phase 3: Presentation layer
  Step 10 → Theme (Color, Type, Shape)
  Step 11 → Navigation (Screen, NavHost)
  Step 12 → HomeScreen + HomeViewModel
  Step 13 → TalkScreen + TalkViewModel + all components
  Step 14 → SettingsScreen + SettingsViewModel + ParentalGate

Phase 4: DI & Application
  Step 15 → AppModule + CharacterModule
  Step 16 → TomikoApplication
  Step 17 → MainActivity

Phase 5: Build & Config
  Step 18 → build.gradle.kts (root + app)
  Step 19 → settings.gradle.kts
  Step 20 → libs.versions.toml
  Step 21 → gradle.properties
  Step 22 → proguard-rules.pro
  Step 23 → AndroidManifest.xml
```

For each step, provide the complete file path and contents. Each step depends on the previous steps being complete.

---

## 3. Per-Step Prompt Template

```
=== STEP {N}: {MODULE NAME} ===

Context: Previous step generated {previous files}.
Now generate the following file(s) with COMPLETE implementation:

File: {full.qualified.path.ClassName.kt}

Requirements:
- Package: com.vedica.labs.ind.app.talkin.tomiko.{layer}.{submodule}
- Imports all types from previous steps
- Follows Clean Architecture (Domain has zero Android imports)
- Kotlin Coroutines + Flow for async
- Error handling with Result<T> or sealed class AppError
- Compose Material 3 components with contentDescription for accessibility
- Complete implementation (no TODOs, no placeholders)

[Optional: additional requirements specific to this file]

Provide ONLY the code in a single code block. No explanations needed.
```

---

## 4. Cross-Module Type Reference

When the LLM needs to know types from OTHER modules, reference the following type table:

| Type | Defined In | Used By |
|---|---|---|
| `Character` (data class) | `domain/model/Character.kt` | All layers |
| `RecordingResult` (sealed) | `domain/model/RecordingResult.kt` | Use cases, ViewModels |
| `VoiceEffect` (enum) | `domain/model/VoiceEffect.kt` | Audio pipeline, UI |
| `AnimationState` (enum) | `domain/model/AnimationState.kt` | Scene, UI |
| `AppError` (sealed) | `domain/model/AppError.kt` | All error paths |
| `TalkUiState` (data class) | `presentation/talk/TalkViewModel.kt` | TalkScreen |
| `Phase` (enum) | `presentation/talk/TalkViewModel.kt` | TalkScreen |
| `CharacterConfig` (serializable) | `data/scene/CharacterLoader.kt` | SceneManager |
| `VisemeFrame` (data class) | `data/audio/LipSyncEngine.kt` | AnimationController |

---

## 5. Validation Checklist (LLM Self-Check)

After generating each module, the LLM should verify:

- [ ] File path matches the architecture tree exactly
- [ ] Package name is `com.vedica.labs.ind.app.talkin.tomiko.{...}`
- [ ] No Kotlin compiler warnings (unused imports, unused variables)
- [ ] No `null` used where a default value works
- [ ] No `!!` operator (use `?:`, `.orEmpty()`, or safe cast)
- [ ] All `@Composable` functions have `contentDescription` on icon-only elements
- [ ] All ViewModels use `viewModelScope.launch` for coroutines
- [ ] No internet/network imports (`java.net.*`, `okhttp*`, `retrofit*`)
- [ ] No database imports (`Room`, `SQLite`)
- [ ] No DI framework annotations (`@Inject`, `@Module`, `@Provides`)
- [ ] All file writes go to `Context.filesDir` or `Context.cacheDir`
- [ ] `Modifier` parameters have a default value where sensible

---

## 6. Example Complete Step

```
=== STEP 1: Domain Models ===

Context: Starting from empty project.
Generate the following files with COMPLETE implementation:

File: com/vedica/labs/ind/app/talkin/tomiko/domain/model/Character.kt
File: com/vedica/labs/ind/app/talkin/tomiko/domain/model/VoiceEffect.kt
File: com/vedica/labs/ind/app/talkin/tomiko/domain/model/AnimationState.kt
File: com/vedica/labs/ind/app/talkin/tomiko/domain/model/RecordingResult.kt
File: com/vedica/labs/ind/app/talkin/tomiko/domain/model/AppError.kt

Requirements:
- Pure Kotlin (no Android imports)
- data classes / sealed classes / enum classes
- All types will be shared across layers

Provide ONLY the code in code blocks.
```

---

## 7. Troubleshooting Common Issues

| Issue | Resolution |
|---|---|
| `SceneView` not found | Add `implementation(libs.sceneview)` and ensure `minSdk = 30` |
| Compose BOM conflict | Use single BOM version; do not mix material2/material3 |
| JNI error loading `.so` | Verify ABI filter: `ndk { abiFilters "arm64-v8a" }` |
| Edge-to-edge on API 30 | Use `WindowCompat.setDecorFitsSystemWindows` fallback |
| `AudioRecord` permission | Single runtime request via `ActivityResultContracts.RequestPermission` |
| `MODE_STATIC` buffer too large | Split into chunks if audio > 5 s; or use `MODE_STREAM` |
| Foldable not detected | Add `implementation(libs.window)` and query `WindowLayoutInfo` |
```

