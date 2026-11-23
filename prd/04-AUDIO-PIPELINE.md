# Tomiko — Audio Recording, Voice Effects & Lip-Sync Pipeline

> **Superseded (November 2026):** Final audio is owned by Rust using AAudio at
> 48 kHz, with Kotlin owning audio focus. See `docs/AUDIO_PIPELINE.md`. The
> Kotlin AudioRecord/AudioTrack and whole-file C++ DSP approach below is legacy.

## 1. Audio Capture (AudioRecord Wrapper)

### Configuration

| Parameter | Value | Rationale |
|---|---|---|
| Sample rate | **44100 Hz** | CD quality; sufficient for voice |
| Channel | **MONO** | Voice only; no stereo needed |
| Encoding | **PCM 16-bit** | Standard format; low CPU |
| Buffer size | **4096 bytes** | ~23 ms latency; matches DSP frame |
| Source | **MIC** | Default microphone |
| Max duration | **10 seconds** | Age-appropriate limit |

### Implementation Pattern

```kotlin
class AudioRecorder {

    private var recordingJob: Job? = null
    private val _amplitude = MutableStateFlow(0f)
    val amplitude: StateFlow<Float> = _amplitude.asStateFlow()

    fun start(outputFile: File, scope: CoroutineScope): File {
        val buffer = ShortArray(2048)
        recordingJob = scope.launch(Dispatchers.IO) {
            val record = AudioRecord(
                MediaRecorder.AudioSource.MIC,
                44100,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
                4096
            )
            record.startRecording()

            val pcmOut = DataOutputStream(FileOutputStream(outputFile))
            var bytesRead: Int

            while (isActive && recording) {
                bytesRead = record.read(buffer, 0, buffer.size)
                if (bytesRead > 0) {
                    // Convert each short to little-endian bytes
                    for (s in buffer.sliceArray(0 until bytesRead)) {
                        pcmOut.writeShort(s.toInt() and 0xFFFF)
                    }
                    // RMS amplitude for UI wave
                    val rms = sqrt(buffer.sliceArray(0 until bytesRead)
                        .map { it.toDouble() / Short.MAX_VALUE }
                        .sumOf { it * it } / bytesRead)
                    _amplitude.value = rms.toFloat()
                }
            }

            pcmOut.close()
            record.stop()
            record.release()
        }
        return outputFile
    }

    fun stop() {
        recording = false
        recordingJob?.cancel()
    }
}
```

---

## 2. WAV File Writer

A simple helper to wrap raw PCM with a proper WAV header for downstream processing.

```kotlin
class WavFileWriter {
    fun write(data: ShortArray, sampleRate: Int, file: File) {
        val totalDataLen = data.size * 2 // 16-bit = 2 bytes per sample
        val totalFileLen = 36 + totalDataLen

        RandomAccessFile(file, "rw").use { raf ->
            raf.writeBytes("RIFF")
            raf.writeInt(Integer.reverseBytes(totalFileLen))
            raf.writeBytes("WAVE")
            raf.writeBytes("fmt ")
            raf.writeInt(Integer.reverseBytes(16)) // chunk size
            raf.writeShort(Integer.reverseBytes(1)) // PCM format
            raf.writeShort(Integer.reverseBytes(1)) // MONO
            raf.writeInt(Integer.reverseBytes(sampleRate))
            raf.writeInt(Integer.reverseBytes(sampleRate * 2)) // byte rate
            raf.writeShort(Integer.reverseBytes(2)) // block align
            raf.writeShort(Integer.reverseBytes(16)) // bits per sample
            raf.writeBytes("data")
            raf.writeInt(Integer.reverseBytes(totalDataLen))
            data.forEach { raf.writeShort(it.toInt() and 0xFFFF) }
        }
    }
}
```

---

## 3. Voice Effects Pipeline (NDK — libtomic_processor.so)

### 3.1 Architecture

```
Raw PCM → [Pitch Shift] → [Formant Filter] → [Reverb Model] → [Robot Voice] → Output PCM
               ▲                 ▲                  ▲                  ▲
              C++               C++                C++                C++
           (Sonic lib)     (custom IIR)       (freeverb)         (custom)
```

### 3.2 JNI Bridge

```kotlin
// data/audio/AudioProcessor.kt
class AudioProcessor {

    companion object {
        init { System.loadLibrary("tomic_processor") }
    }

    /** Apply a voice effect to a WAV file. Returns processed file path. */
    external fun processEffect(
        inputPath: String,
        outputPath: String,
        effectOrdinal: Int,  // 0=NORMAL, 1=PITCH_HIGH, 2=ROBOT, 3=REVERB
        pitchShift: Float,   // 1.0 = normal, 1.5 = chipmunk, 0.7 = deep
        formantShift: Float, // 1.0 = normal
        reverbMix: Float     // 0.0–1.0
    ): Int // 0 = success, -1 = error
}
```

### 3.3 NDK CMakeLists.txt

```cmake
# lib/tomic_processor/CMakeLists.txt
cmake_minimum_required(VERSION 3.22)
project("tomic_processor")

add_library(tomic_processor SHARED
    src/main/cpp/native_bridge.cpp
    src/main/cpp/pitch_shift.cpp
    src/main/cpp/formant_filter.cpp
    src/main/cpp/reverb_model.cpp
    src/main/cpp/robot_voice.cpp
    src/main/cpp/wav_io.cpp
)

target_include_directories(tomic_processor PRIVATE src/main/cpp/include)
find_library(log-lib log)
target_link_libraries(tomic_processor ${log-lib} android)
```

### 3.4 Effect Presets

| Effect | pitchShift | formantShift | reverbMix | Description |
|---|---|---|---|---|
| **NORMAL** | 1.0 | 1.0 | 0.0 | Raw audio (no effect) |
| **CHIPMUNK** | 1.8 | 1.3 | 0.0 | High-pitched squeaky |
| **DEEP** | 0.6 | 0.7 | 0.05 | Low monster voice |
| **ROBOT** | 1.0 | 1.0 | 0.2 | Staccato robot (custom processing) |
| **ECHO** | 1.0 | 1.0 | 0.5 | Reverb hall effect |
| **ALIEN** | 1.5 | 0.5 | 0.3 | Pitch up + formant down |

---

## 4. Lip-Sync Engine (Energy-Based Viseme Detection)

### 4.1 Algorithm Overview

Since we cannot run ML models (privacy, APK size), Tomiko uses an **energy-based** approach:

```
Audio PCM → Window (512 samples) → RMS Energy → Frequency Bands → Viseme Mapping
                                                 ├── Low (0–300 Hz)   → M, B, P
                                                 ├── Mid (300–2000)   → AA, E, I, O, U
                                                 └── High (2000–4000) → S, SH, F, TH
```

### 4.2 Implementation

```kotlin
class LipSyncEngine(
    private val sampleRate: Int = 44100,
    private val frameSize: Int = 512, // ~11.6 ms
) {
    data class VisemeFrame(val index: Int, val weight: Float)

    fun process(pcm: ShortArray): List<VisemeFrame> {
        val frames = mutableListOf<VisemeFrame>()
        var offset = 0

        while (offset + frameSize <= pcm.size) {
            val frame = pcm.sliceArray(offset until offset + frameSize)
            val viseme = analyzeFrame(frame)
            frames.add(viseme)
            offset += frameSize / 2 // 50% overlap for smoothness
        }
        return frames
    }

    private fun analyzeFrame(frame: ShortArray): VisemeFrame {
        val rms = computeRms(frame)
        if (rms < SILENCE_THRESHOLD) return VisemeFrame(VISEME_REST, 0f)

        val spectrum = fft(frame) // simple FFT (256 bins)
        val lowEnergy = sumBins(spectrum, 0, lowBin)
        val midEnergy = sumBins(spectrum, lowBin, midBin)
        val highEnergy = sumBins(spectrum, midBin, highBin)

        return when {
            lowEnergy > midEnergy && lowEnergy > highEnergy -> VisemeFrame(VISEME_M, rms)
            highEnergy > lowEnergy && highEnergy > midEnergy -> VisemeFrame(VISEME_S, rms)
            midEnergy > lowEnergy && midEnergy > highEnergy -> {
                VisemeFrame(detectVowel(frame), rms)
            }
            else -> VisemeFrame(VISEME_REST, 0f)
        }
    }

    private fun detectVowel(frame: ShortArray): Int {
        // Simple heuristic: F1 / F2 ratio via zero-crossing rate
        val zcr = computeZeroCrossingRate(frame)
        return when {
            zcr < 0.05 -> VISEME_U
            zcr < 0.10 -> VISEME_O
            zcr < 0.15 -> VISEME_AA
            zcr < 0.20 -> VISEME_E
            else -> VISEME_I
        }
    }

    companion object {
        const val VISEME_REST = 13
        const val VISEME_AA = 0
        const val VISEME_E = 1
        const val VISEME_I = 2
        const val VISEME_O = 3
        const val VISEME_U = 4
        const val VISEME_M = 5
        const val VISEME_S = 9
        const val SILENCE_THRESHOLD = 0.02f
    }
}
```

### 4.3 Temporal Smoothing

To avoid "jittery" mouth movements, apply an exponential moving average:

```kotlin
private var previousViseme = VISEME_REST
private var previousWeight = 0f

fun smooth(current: VisemeFrame, alpha: Float = 0.3f): VisemeFrame {
    val smoothedIndex = if (current.weight > 0.1f) current.index else previousViseme
    val smoothedWeight = alpha * current.weight + (1 - alpha) * previousWeight
    previousViseme = smoothedIndex
    previousWeight = smoothedWeight
    return VisemeFrame(smoothedIndex, smoothedWeight)
}
```

---

## 5. Audio Playback (AudioTrack Wrapper)

```kotlin
class AudioPlayer {

    private var track: AudioTrack? = null

    fun play(file: File, onProgress: (Float) -> Unit, scope: CoroutineScope) {
        scope.launch(Dispatchers.IO) {
            val pcm = readWavFile(file)
            val totalFrames = pcm.size

            track = AudioTrack(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build(),
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(44100)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build(),
                pcm.size * 2,
                AudioTrack.MODE_STATIC,
                AudioTrack.AUDIO_SESSION_ID_GENERATE
            )

            track?.write(pcm, 0, pcm.size)
            track?.play()

            // Progress reporting
            while (track?.playState == AudioTrack.PLAYSTATE_PLAYING) {
                val pos = track?.playbackHeadPosition ?: 0
                onProgress(pos.toFloat() / totalFrames)
                delay(50) // 20 FPS progress updates
            }
        }
    }

    fun stop() {
        track?.stop()
        track?.release()
        track = null
    }
}
```

---

## 6. Complete Pipeline Flow

```
┌──────────┐    ┌──────────┐    ┌─────────────┐    ┌──────────┐
│  Record   │───▶│  WAV     │───▶│  AudioProc  │───▶│  Play    │
│  PCM RAW  │    │  Encode  │    │  (NDK C++)  │    │  + Lip-  │
└──────────┘    └──────────┘    └─────────────┘    │  Sync    │
                                                    └──────────┘
       ▲                                                │
       │                        ┌──────────────┐        │
       │                        │  LipSyncEngine│◀───────┘
       │                        │  (viseme map) │
       │                        └──────┬───────┘
       │                               │
       │                        ┌──────▼───────┐
       └────────────────────────│  SceneView   │
                                │  (morph tgt) │
                                └──────────────┘
```
