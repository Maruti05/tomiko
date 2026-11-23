# Tomiko — Coding Standards & Conventions

## 1. Kotlin Coding Conventions

### 1.1 Naming Rules

| Element | Convention | Example |
|---|---|---|
| Package | All lowercase, dotted | `com.vedica.labs.ind.app.talkin.tomiko.presentation.talk` |
| Class / Object | PascalCase | `TalkViewModel`, `AudioRecorder` |
| Function | camelCase | `onRecordPressed()`, `playAnimation()` |
| Property (public) | camelCase | `uiState`, `amplitude` |
| Property (private) | _underscore prefix for backing fields | `_uiState: MutableStateFlow<TalkUiState>` |
| Constant | UPPER_SNAKE_CASE | `const val MAX_RECORD_MS = 10_000` |
| Composable | PascalCase (noun) | `CharacterViewport`, `RecordButton` |
| Enum values | UPPER_SNAKE_CASE | `PITCH_HIGH`, `VISEME_REST` |
| Type parameters | Single capital letter | `T`, `R` |

### 1.2 File Organization

Each file contains exactly **one** public class/interface, named the same as the file:

```
Character.kt          → data class Character
CharacterRepository.kt → interface CharacterRepository
CharacterRepositoryImpl.kt → class CharacterRepositoryImpl
```

### 1.3 Maximum Lengths

| Rule | Limit |
|---|---|
| Line length | **120 characters** |
| Function body | **40 lines** |
| Class body | **300 lines** (if exceeded, split) |
| Method parameters | **6** (if exceeded, use parameter object) |
| Nesting depth | **4 levels** max |

---

## 2. Compose Conventions

### 2.1 Composable Function Rules

```kotlin
// 1. Composable name = noun describing UI element
@Composable
fun RecordButton(
    // 2. Modifier is always the first optional parameter with default Modifier
    modifier: Modifier = Modifier,
    // 3. State parameters first, then callbacks
    isRecording: Boolean,
    onRecordStart: () -> Unit,
    onRecordStop: () -> Unit,
    // 4. Content lambda last
    content: @Composable () -> Unit = {},
) { ... }

// 5. All non-interactive elements have contentDescription
Icon(
    imageVector = Icons.Default.Mic,
    contentDescription = stringResource(R.string.btn_record),
    modifier = modifier
)

// 6. Use remember for stable references
val onClick = remember { { viewModel.onRecordPressed() } }

// 7. Never pass ViewModel to child composable — pass primitives/state only
// BAD:  TalkControls(viewModel: TalkViewModel)
// GOOD: TalkControls(phase: Phase, onRecord: () -> Unit)
```

### 2.2 State Hoisting

```kotlin
@Composable
fun RecordButton(
    isRecording: Boolean,
    onRecordStart: () -> Unit,
    onRecordStop: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Button(
        onClick = { if (isRecording) onRecordStop() else onRecordStart() },
        modifier = modifier.size(80.dp),
        shape = CircleShape,
    ) {
        Icon(
            imageVector = if (isRecording) Icons.Default.Stop else Icons.Default.Mic,
            contentDescription = if (isRecording)
                stringResource(R.string.btn_stop_recording)
            else
                stringResource(R.string.btn_start_recording)
        )
    }
}
```

---

## 3. ViewModel Conventions

```kotlin
class TalkViewModel(
    // 1. Constructor injection only (no Hilt)
    private val recordAudio: RecordAudioUseCase,
    private val processAudio: ProcessAudioUseCase,
    private val playback: PlaybackUseCase,
) : ViewModel() {

    // 2. Private backing MutableStateFlow, public StateFlow
    private val _uiState = MutableStateFlow(TalkUiState())
    val uiState: StateFlow<TalkUiState> = _uiState.asStateFlow()

    // 3. One-shot events via Channel (not SharedFlow)
    private val _events = Channel<UiEvent>(Channel.BUFFERED)
    val events: Flow<UiEvent> = _events.receiveAsFlow()

    // 4. All async operations in viewModelScope
    fun onRecordPressed() {
        viewModelScope.launch {
            _uiState.update { it.copy(phase = Phase.RECORDING) }
        }
    }

    // 5. Never expose mutable state directly
}

// 6. UiState is a single data class per screen
data class TalkUiState(
    val phase: Phase = Phase.IDLE,
    val playbackProgress: Float = 0f,
    val error: String? = null,
)

// 7. One-shot events as sealed interface
sealed interface UiEvent {
    data object NavigateToHome : UiEvent
    data class ShowSnackbar(val message: String) : UiEvent
}
```

---

## 4. Architecture Enforcement Rules

| Rule | Enforcement |
|---|---|
| Domain layer imports NOTHING from Android | Compiler check (pure Kotlin module) |
| Presentation imports Domain only | Static analysis or manual review |
| Data layer implements Domain interfaces | Compiler: `: CharacterRepository` |
| ViewModel never exposes mutable state | Code review: all fields are `StateFlow` or `Flow` |
| No `mutableStateOf` outside @Composable | Lint: Kotlin compiler warning |
| No `remember` for ViewModel instances | Use `viewModel()` from `lifecycle-viewmodel-compose` |
| No `LaunchedEffect` with key `Unit` | Use `SideEffect` for non-restartable work |

---

## 5. Error Handling Conventions

```kotlin
// 1. Use sealed class for domain errors
sealed class AppError(val message: String, val cause: Throwable? = null) {
    data class AudioPermissionDenied(cause: Throwable? = null) :
        AppError("Microphone permission required", cause)

    data class RecordingFailed(val reason: String, cause: Throwable? = null) :
        AppError("Recording failed: $reason", cause)
}

// 2. Use Result<T> in use cases
class RecordAudioUseCase(private val repo: AudioRepository) {
    suspend operator fun invoke(maxDurationMs: Long): Result<RecordingResult> {
        return try {
            val result = repo.record(maxDurationMs)
            Result.success(result)
        } catch (e: AudioPermissionException) {
            Result.failure(AppError.AudioPermissionDenied(e))
        } catch (e: Exception) {
            Result.failure(AppError.RecordingFailed(e.message ?: "Unknown", e))
        }
    }
}

// 3. ViewModel maps errors to UI state
viewModelScope.launch {
    recordAudio(10_000)
        .onSuccess { result -> _uiState.update { it.copy(phase = Phase.PROCESSING) } }
        .onFailure { error ->
            _uiState.update { it.copy(error = error.message) }
            _events.send(UiEvent.ShowSnackbar(error.message))
        }
}
```

---

## 6. Resource Naming

```xml
<!-- Drawable icons -->
ic_record_mic.xml
ic_replay.xml
ic_settings.xml
ic_character_tomiko.xml

<!-- String resources (en) -->
strings.xml:
    app_name = Tomiko
    btn_record = Tap to talk
    btn_replay = Listen again
    parental_gate_title = Solve to continue
    error_permission = Please allow microphone access

<!-- Animations -->
animator/record_pulse.xml
animator/happy_jump.xml
```

---

## 7. Git Commit Convention (if used)

```
type(scope): Description

Types: feat | fix | refactor | perf | docs | style | test
Scope: home | talk | settings | audio | scene | build | core

Examples:
feat(talk): implement voice effect selector
fix(audio): prevent AudioTrack leak on stop
perf(scene): reduce morph target update frequency
docs(prd): add performance budget table
```

---

## 8. Lint & Formatting

```kotlin
// .editorconfig
root = true

[*]
indent_style = space
indent_size = 4
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true
max_line_length = 120

[*.{kt,kts}]
ktlint_code_style = android_studio
ktlint_standard_no-wildcard-imports = disabled
```

**Run before every commit:**
```bash
# Format code
./gradlew spotlessApply

# Run lint
./gradlew lint

# Run tests
./gradlew test
```
