# Tomiko — Clean Architecture & MVVM Design

## 1. Layer Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    PRESENTATION LAYER                    │
│  (Compose Screens, ViewModels, UI State)                 │
│  :feature:home  :feature:talk  :feature:settings        │
├──────────────────────────────────────────────────────────┤
│                      DOMAIN LAYER                        │
│  (Use Cases, Domain Models, Repository Interfaces)       │
│  :core:model                                              │
├──────────────────────────────────────────────────────────┤
│                       DATA LAYER                         │
│  (Repository Implementations, File I/O, Audio Drivers)   │
│  :core:audio  :core:util                                  │
└──────────────────────────────────────────────────────────┘
```

### Dependency Rule
- **Presentation** depends on **Domain** only (via interfaces).
- **Domain** depends on **nothing** (pure Kotlin module — no Android imports).
- **Data** depends on **Domain** (implements repository interfaces).
- Data never references Presentation types.

---

## 2. Detailed Package Structure

```
com.vedica.labs.ind.app.talkin.tomiko/
│
├── TomikoApplication.kt                  # Application class, initializes scene
│
├── di/                                    # Manual dependency injection (no Hilt/Dagger)
│   ├── AppModule.kt                       # Provides repository, audio engine, dispatchers
│   └── CharacterModule.kt                 # Provides SceneView config per character
│
├── presentation/
│   ├── navigation/
│   │   ├── TomikoNavHost.kt              # NavHost with route definitions
│   │   ├── Screen.kt                     # Sealed class of routes
│   │   └── NavTransitions.kt             # Shared element / fade transitions
│   │
│   ├── theme/
│   │   ├── TomikoTheme.kt               # Material 3 theme (dynamic color optional)
│   │   ├── Color.kt                      # Light & dark palette
│   │   ├── Type.kt                       # Typography (kid-friendly large fonts)
│   │   └── Shape.kt                      # Rounded corner shapes
│   │
│   ├── home/
│   │   ├── HomeScreen.kt                 # Character selection grid
│   │   ├── HomeViewModel.kt              # Loads character list from repository
│   │   └── components/
│   │       ├── CharacterCard.kt          # 3D character preview thumbnail
│   │       └── AppTitle.kt               # Animated app logo
│   │
│   ├── talk/
│   │   ├── TalkScreen.kt                 # Main talking UI
│   │   ├── TalkViewModel.kt              # Orchestrates record → process → play
│   │   └── components/
│   │       ├── CharacterViewport.kt      # SceneView composable wrapper
│   │       ├── RecordButton.kt           # Animated push-to-talk button
│   │       ├── WaveformIndicator.kt      # Live audio level bar
│   │       └── PlaybackControls.kt       # Replay / back / share (local) buttons
│   │
│   └── settings/
│       ├── SettingsScreen.kt             # Volume, character reset, about
│       ├── SettingsViewModel.kt
│       └── components/
│           └── ParentalGate.kt           # Simple math challenge composable
│
├── domain/
│   ├── model/
│   │   ├── Character.kt                  # Domain entity: id, name, gltf path, thumbnail
│   │   ├── RecordingResult.kt            # Sealed: Success(audioFile, duration) | Error
│   │   ├── VoiceEffect.kt                # Enum: NORMAL, PITCH_HIGH, ROBOT, REVERB
│   │   └── AnimationState.kt             # Enum: IDLE, LISTENING, TALKING, HAPPY
│   │
│   ├── repository/
│   │   ├── CharacterRepository.kt        # Interface: listCharacters(), getById()
│   │   └── AudioRepository.kt            # Interface: save(), load(), delete()
│   │
│   └── usecase/
│       ├── GetCharactersUseCase.kt       # Returns flow of available characters
│       ├── RecordAudioUseCase.kt         # Captures PCM → saves WAV
│       ├── ProcessAudioUseCase.kt        # Applies voice effect via NDK
│       └── PlaybackUseCase.kt            # Plays processed audio + drives lip-sync
│
├── data/
│   ├── repository/
│   │   ├── CharacterRepositoryImpl.kt    # Reads from assets/characters/index.json
│   │   └── AudioRepositoryImpl.kt        # Writes/reads recordings/ directory
│   │
│   ├── audio/
│   │   ├── AudioRecorder.kt              # AudioRecord wrapper, raw PCM capture
│   │   ├── AudioPlayer.kt                # AudioTrack wrapper, non-blocking playback
│   │   ├── AudioProcessor.kt             # JNI bridge to libtomic_processor.so
│   │   ├── WavFileWriter.kt              # PCM → WAV header encoder
│   │   └── LipSyncEngine.kt              # Energy-based viseme detection
│   │
│   ├── scene/
│   │   ├── SceneManager.kt               # Filament SceneView lifecycle manager
│   │   ├── CharacterLoader.kt            # glTF async loader with progress
│   │   └── AnimationController.kt        # Morph target / skeleton animation driver
│   │
│   └── file/
│       ├── FileManager.kt                # Context.filesDir abstraction
│       └── JsonConfigStore.kt            # kotlinx.serialization JSON reader/writer
│
└── util/
    ├── Constants.kt                      # Timeouts, file size limits, paths
    ├── Extensions.kt                     # Kotlin extension helpers
    └── PermissionHelper.kt               # Record audio permission request (one-time)
```

---

## 3. Dependency Injection (Manual — No Hilt / Dagger)

Using a simple `AppModule` singleton pattern to avoid annotation-processing overhead and APK size increase.

```kotlin
// di/AppModule.kt
object AppModule {
    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    fun provideAudioRecorder(app: Context): AudioRecorder = AudioRecorder()
    fun provideAudioPlayer(app: Context): AudioPlayer = AudioPlayer()
    fun provideFileManager(app: Context): FileManager = FileManager(app.filesDir)
    fun provideJsonConfigStore(fm: FileManager): JsonConfigStore = JsonConfigStore(fm)
    fun provideCharacterRepository(app: Context): CharacterRepository =
        CharacterRepositoryImpl(app.assets)
    fun provideAudioRepository(fm: FileManager): AudioRepository =
        AudioRepositoryImpl(fm)
}
```

**Why not Hilt/Dagger?**
- APK size savings (~800 KB+)
- Faster build times
- Less complexity for a single-developer / LLM-generated project

---

## 4. State Management (ViewModel + StateFlow)

```kotlin
// TalkViewModel.kt — example state machine
class TalkViewModel(
    private val recordAudio: RecordAudioUseCase,
    private val processAudio: ProcessAudioUseCase,
    private val playback: PlaybackUseCase,
) : ViewModel() {

    private val _uiState = MutableStateFlow(TalkUiState())
    val uiState: StateFlow<TalkUiState> = _uiState.asStateFlow()

    fun onRecordPressed() {
        viewModelScope.launch {
            _uiState.update { it.copy(phase = Phase.RECORDING) }
            val result = recordAudio(maxDurationMs = 10_000)
            _uiState.update { it.copy(phase = Phase.PROCESSING) }
            val processed = processAudio(result.rawFile, effect = VoiceEffect.PITCH_HIGH)
            _uiState.update { it.copy(phase = Phase.PLAYING, audioFile = processed) }
            playback.play(processed) { progress ->
                _uiState.update { it.copy(playbackProgress = progress) }
            }
            _uiState.update { it.copy(phase = Phase.DONE) }
        }
    }
}

data class TalkUiState(
    val phase: Phase = Phase.IDLE,
    val playbackProgress: Float = 0f,
    val error: String? = null,
)

enum class Phase { IDLE, RECORDING, PROCESSING, PLAYING, DONE }
```

---

## 5. Navigation Graph

```kotlin
// Screen.kt
sealed class Screen(val route: String) {
    data object Home : Screen("home")
    data object Talk : Screen("talk/{characterId}") {
        fun route(characterId: String) = "talk/$characterId"
    }
    data object Settings : Screen("settings")
}

// TomikoNavHost.kt
@Composable
fun TomikoNavHost(navController: NavHostController) {
    NavHost(navController, startDestination = Screen.Home.route) {
        composable(Screen.Home.route) { HomeScreen(navController) }
        composable(
            route = Screen.Talk.route,
            arguments = listOf(navArgument("characterId") { type = NavType.StringType })
        ) { entry ->
            TalkScreen(characterId = entry.arguments?.getString("characterId")!!, navController)
        }
        composable(Screen.Settings.route) { SettingsScreen(navController) }
    }
}
```

---

## 6. Error Handling Strategy

| Layer | Approach |
|---|---|
| **Presentation** | `_uiState.update { it.copy(error = msg) }` → Snackbar / dialog |
| **Domain** | Use `Result<T>` or sealed `Result<T, AppError>` for use cases |
| **Data** | Try-catch at boundary; map exceptions to domain errors |
| **Audio native** | Return error codes from JNI; map in `AudioProcessor` |

```kotlin
sealed class AppError(val message: String) {
    data class AudioPermissionDenied : AppError("Microphone permission required")
    data class RecordingFailed(val reason: String) : AppError("Recording failed: $reason")
    data class ProcessingFailed(val reason: String) : AppError("Processing error: $reason")
    data object StorageFull : AppError("Device storage is full")
}
```
