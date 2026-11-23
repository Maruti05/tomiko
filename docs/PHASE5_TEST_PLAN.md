# Phase 5 Device Test Plan

Phase 5 validates AAudio capture/playback, VAD, DSP, presets, lip-sync, audio
focus, and lifecycle behavior on a real device.

## 1. Build and install

```powershell
.\scripts\build_rust.ps1 -Profile release
.\scripts\build_android.ps1 -Mode debug -SkipRust
adb install -r app\build\outputs\apk\debug\app-debug.apk
adb shell pm grant com.vedica.labs.ind.app.talkin.tomiko.debug android.permission.RECORD_AUDIO
```

If `pm grant` is rejected, accept the microphone dialog on the device.

## 2. Launch and stream checks

```powershell
adb logcat -c
adb shell am start -n com.vedica.labs.ind.app.talkin.tomiko.debug/com.vedica.labs.ind.app.talkin.tomiko.game.CharacterGameActivity
adb logcat -s RustStdoutStderr:I
```

Expected:

- `AAudio input opened: 48000 Hz, 1 ch, PCM_I16, perf LowLatency`
- `AAudio output opened: 48000 Hz, 1 ch, PCM_I16, perf LowLatency`
- `Audio streams ready at 48000 Hz`
- `Audio listening started`
- `AAudio capture callback active (...)`
- Tomiko visible and `Playing clip listening`

## 3. Synthetic DSP / playback / lip-sync

```powershell
adb shell input keyevent 53   # Y = DSP self-test
```

Expected:

- `DSP start (self-test, ...)`
- `DSP done: ... us` (well below real time)
- `Playback started: ...`
- Mouth opens during playback and closes afterwards
- `Playback finished (captured=..., dropped=0, underruns=0)`

## 4. VAD self-test

```powershell
adb shell input keyevent 35   # G = synthetic utterance through VAD
```

Expected: `VAD self-test produced an utterance`, then DSP/playback logs.

## 5. Live microphone

Speak near the device while it is listening. Expected:

- `Speech started (rms 0.0xxx, noise floor 0.0xxx)` at low levels for distant
  speech (validated down to RMS ~0.0026).
- `Utterance captured: ... frames (...)`
- `DSP start (capture, ...)`
- `DSP done: ..., input gain N×` where N is above 1 for quiet input
- Playback of the processed voice
- Mouth motion follows the playback

If the device's microphone is picking up ambient speech, the logs may show
utterances without user action; this is expected while auto-listening is on.
Use the HUD Stop button when testing in a noisy room.

## 6. Overlapping playback

Send `G` twice within one second. Expected: two `Playback started` entries,
one final `Playback finished`, and no `InvalidState`.

## 7. Audio focus and lifecycle

- Press Home → `Pause`/`StopListening`; audio stops immediately (measured
  6 ms from JNI call, independent of the Bevy event loop).
- Return to the app → `Resume`/`StartListening`; capture restarts.
- Trigger a focus loss (another app plays media) → capture/playback pauses.
- Regain focus → capture resumes if the activity is still resumed.
- Verify the process/PID survives.

## 8. Metrics

Record from logs/atomics:

- Capture callback frames and dropped count.
- DSP duration per utterance.
- Playback underruns.
- Capture-start to playback-start latency (not yet externally measured).

## 9. Pass criteria

- [x] Input/output open at 48 kHz, mono, PCM16.
- [x] Capture callback delivers samples with zero drops in the test run.
- [x] Live VAD utterance captured and replayed.
- [x] Synthetic VAD → DSP → playback path passes.
- [x] DSP faster than real time.
- [x] Zero playback underruns in the test run.
- [x] Lip-sync visibly moves the mouth.
- [x] Overlapping playback does not error.
- [x] Pause/resume restarts capture.
- [ ] Capture-to-playback latency measured with an external reference.
- [ ] Preset quality/audibility assessed by a human listener.
