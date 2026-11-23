# Tomiko — Testing Strategy

## 1. Test Pyramid

```
         ╱╲
        ╱ E2E ╲          ← Manual QA + Espresso (critical paths)
       ╱────────╲
      ╱ Integration ╲    ← Audio pipeline, SceneView lifecycle
     ╱──────────────╲
    ╱   Unit Tests    ╲  ← ViewModels, Use Cases, LipSync, File I/O
   ╱────────────────────╲
```

| Layer | Tool | Count (target) |
|---|---|---|
| Unit tests | JUnit 4 + Turbine + MockK (optional) | ≥ 30 tests |
| Integration | JUnit 4 + coroutines-test | ≥ 10 tests |
| UI / E2E | Manual test pass (see checklist) | Full pass per release |

---

## 2. Unit Testing

### 2.1 ViewModel Tests

```kotlin
// TalkViewModelTest.kt
class TalkViewModelTest {

    private val recorder = FakeAudioRecorder()
    private val processor = FakeAudioProcessor()
    private val player = FakeAudioPlayer()
    private lateinit var viewModel: TalkViewModel

    @Before
    fun setup() {
        viewModel = TalkViewModel(recorder, processor, player)
    }

    @Test
    fun `record button starts recording phase`() = runTest {
        viewModel.onRecordPressed()
        val state = viewModel.uiState.first()
        assertEquals(Phase.RECORDING, state.phase)
    }

    @Test
    fun `full pipeline completes successfully`() = runTest {
        viewModel.onRecordPressed()
        // Wait for pipeline to complete
        val state = viewModel.uiState
            .first { it.phase == Phase.DONE }
        assertNull(state.error)
        assertNotNull(state.audioFile)
    }

    @Test
    fun `recording error sets error state`() = runTest {
        recorder.shouldFail = true
        viewModel.onRecordPressed()
        val state = viewModel.uiState
            .first { it.error != null }
        assertTrue(state.error!!.contains("Recording failed"))
    }
}
```

### 2.2 Use Case Tests

```kotlin
class GetCharactersUseCaseTest {
    private val repo = FakeCharacterRepository()
    private val useCase = GetCharactersUseCase(repo)

    @Test
    fun `returns list of characters from repo`() = runTest {
        repo.addCharacter(Character(id = "tomiko", name = "Tomiko"))
        val characters = useCase().first()
        assertEquals(1, characters.size)
        assertEquals("Tomiko", characters[0].name)
    }

    @Test
    fun `returns empty list when no characters`() = runTest {
        val characters = useCase().first()
        assertTrue(characters.isEmpty())
    }
}
```

### 2.3 LipSyncEngine Tests

```kotlin
class LipSyncEngineTest {

    private val engine = LipSyncEngine()

    @Test
    fun `silence returns rest viseme`() {
        val silence = ShortArray(512) { 0 }
        val result = engine.process(silence)
        assertEquals(LipSyncEngine.VISEME_REST, result.first().index)
    }

    @Test
    fun `high frequency content returns s-viseme`() {
        val frame = ShortArray(512) { i ->
            (sin(i * 2 * PI * 3000 / 44100) * Short.MAX_VALUE).toInt().toShort()
        }
        val result = engine.process(frame)
        assertEquals(LipSyncEngine.VISEME_S, result.first().index)
    }

    @Test
    fun `smoothing reduces viseme flicker`() {
        val frames = listOf(
            LipSyncEngine.VisemeFrame(0, 0.9f),
            LipSyncEngine.VisemeFrame(0, 0.1f),
            LipSyncEngine.VisemeFrame(0, 0.8f),
        )
        val smoothed = frames.map { engine.smooth(it, alpha = 0.5f) }
        assertTrue(smoothed.all { it.index == 0 })
    }
}
```

---

## 3. Integration Testing

### 3.1 Audio File I/O

```kotlin
class AudioFileIOTest {
    private val tempDir = File(System.getProperty("java.io.tmpdir"))

    @Test
    fun `wav roundtrip preserves data`() {
        val original = ShortArray(44100) // 1 second
        Random.nextBytes(original.toByteArray())
        val wavFile = File(tempDir, "test.wav")

        WavFileWriter().write(original, 44100, wavFile)
        val loaded = AudioFileReader().readPCM(wavFile)

        assertArrayEquals(original, loaded)
    }
}
```

### 3.2 Character Config Parsing

```kotlin
class CharacterConfigTest {
    @Test
    fun `parse character config json`() {
        val json = """
        {
            "scale": 1.0,
            "camera": { "distance": 2.5, "angleX": 15.0, "angleY": 0.0 },
            "morphTargets": { "viseme_AA": 0 }
        }
        """.trimIndent()
        val config = Json.decodeFromString<CharacterConfig>(json)
        assertEquals(2.5, config.camera.distance)
    }
}
```

---

## 4. Manual QA Checklist

### 4.1 Functional Test Pass

| # | Test Case | Expected Result | Pass/Fail |
|---|---|---|---|
| 1 | Launch app on API 30 emulator | Splash shows → Home screen | ☐ |
| 2 | Tap a character | TalkScreen loads with 3D model visible | ☐ |
| 3 | Tap Record button | Recording starts, waveform animates | ☐ |
| 4 | Speak into mic | Character shows LISTENING animation | ☐ |
| 5 | Release / auto-stop | Processing indicator shown (< 500 ms) | ☐ |
| 6 | Playback starts | Character lip-syncs to recorded audio | ☐ |
| 7 | Voice effect applied | Chipmunk / Robot effect audible | ☐ |
| 8 | Tap Replay | Audio + animation replay correctly | ☐ |
| 9 | Tap Repeat (new recording) | Returns to IDLE, ready to record | ☐ |
| 10 | Settings → Parental Gate | Math challenge appears before settings | ☐ |

### 4.2 Performance Test Pass

| # | Test Case | Criteria |
|---|---|---|
| 1 | Frame rate monitor (30 min) | Avg ≥ 45 FPS on API 30 emulator (2 GB RAM) |
| 2 | Memory heap monitor | Peak ≤ 100 MB, no steady growth over 15 min |
| 3 | APK size | ≤ 25 MB (arm64-v8a) |
| 4 | Battery historian | ≤ 5 % per 10 min continuous use |

### 4.3 Compatibility Test Pass

| # | Device | Form Factor | OS | Result |
|---|---|---|---|---|
| 1 | Pixel 4a | Phone | Android 11 | ☐ |
| 2 | Pixel 7 | Phone | Android 14 | ☐ |
| 3 | Samsung Tab S7 | Tablet (11″) | Android 13 | ☐ |
| 4 | Samsung Galaxy Z Fold 4 | Foldable | Android 14 | ☐ |
| 5 | Lenovo Tab M10 | Budget tablet | Android 12 | ☐ |

---

## 5. Test Fixtures

```kotlin
// Fake implementations for unit testing
class FakeAudioRecorder : AudioRecorder() {
    var shouldFail = false
    override fun start(outputFile: File, scope: CoroutineScope): File {
        if (shouldFail) throw RuntimeException("Recording failed")
        outputFile.writeText("fake pcm data")
        return outputFile
    }
}

class FakeAudioProcessor : AudioProcessor() {
    override fun processEffect(inputPath: String, outputPath: String, ...): Int {
        // Copy input to output (pass-through)
        File(inputPath).copyTo(File(outputPath))
        return 0
    }
}

class FakeAudioPlayer : AudioPlayer() {
    override fun play(file: File, onProgress: (Float) -> Unit, scope: CoroutineScope) {
        scope.launch {
            for (i in 0..100) {
                onProgress(i / 100f)
                delay(10)
            }
        }
    }
}
```
