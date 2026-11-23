//! Audio runtime: stream lifecycle, VAD, DSP worker, playback scheduling, and
//! the atomics read by Bevy/Compose.
//!
//! Threads:
//! - AAudio input callback: copies samples into a preallocated SPSC ring.
//! - AAudio output callback: drains the playback ring, computes lip-sync, and
//!   writes atomics.
//! - `tomiko-audio`: command handling, VAD, playback scheduling.
//! - `tomiko-dsp`: pitch/formant/filter chain (allocation allowed).

use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded};
use ringbuf::traits::{Consumer, Observer, Producer};
use shared_types::{AudioConfig, EngineError, EngineState, ErrorCode, VoicePreset};

use crate::aaudio::{self, OpenedStreams};
use crate::dsp::{VoiceProcessor, synthesize_test_utterance};
use crate::lipsync::LipSyncFrame;
use crate::presets::{VoicePresetParameters, parameters_for};
use crate::vad::{Vad, VadConfig, VadEvent};

const COMMAND_QUEUE: usize = 32;
const DSP_QUEUE: usize = 2;

/// Commands from Bevy/Kotlin into the audio control thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    StartListening,
    StopListening,
    PlayLast,
    Pause,
    Resume,
    SetPreset {
        preset: VoicePreset,
        intensity: f32,
    },
    SelfTest,
    /// Debug path: feed a synthetic utterance through VAD, then DSP/playback.
    SelfTestCapture,
    Shutdown,
}

/// Atomics shared with the renderer/UI. All values are safe to read from any
/// thread; f32 values are stored as bits to avoid torn reads.
#[derive(Debug, Default)]
pub struct AudioMeters {
    phase: AtomicI32,
    amplitude_bits: AtomicU32,
    mouth_open_bits: AtomicU32,
    viseme_aa_bits: AtomicU32,
    viseme_o_bits: AtomicU32,
    viseme_e_bits: AtomicU32,
    capture_frames: AtomicU64,
    capture_dropped: AtomicU64,
    playback_underruns: AtomicU64,
    dsp_micros: AtomicU32,
    error_code: AtomicI32,
}

impl AudioMeters {
    #[must_use]
    pub fn phase(&self) -> EngineState {
        EngineState::from_id(self.phase.load(Ordering::Acquire)).unwrap_or(EngineState::Error)
    }

    pub fn set_phase(&self, phase: EngineState) {
        self.phase.store(phase as i32, Ordering::Release);
    }

    #[must_use]
    pub fn amplitude(&self) -> f32 {
        load_f32(&self.amplitude_bits)
    }

    #[must_use]
    pub fn mouth_open(&self) -> f32 {
        load_f32(&self.mouth_open_bits)
    }

    #[must_use]
    pub fn viseme_aa(&self) -> f32 {
        load_f32(&self.viseme_aa_bits)
    }

    #[must_use]
    pub fn viseme_o(&self) -> f32 {
        load_f32(&self.viseme_o_bits)
    }

    #[must_use]
    pub fn viseme_e(&self) -> f32 {
        load_f32(&self.viseme_e_bits)
    }

    #[must_use]
    pub fn capture_frames(&self) -> u64 {
        self.capture_frames.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn capture_dropped(&self) -> u64 {
        self.capture_dropped.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn playback_underruns(&self) -> u64 {
        self.playback_underruns.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn dsp_micros(&self) -> u32 {
        self.dsp_micros.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn error_code(&self) -> i32 {
        self.error_code.load(Ordering::Acquire)
    }

    pub(crate) fn set_levels(&self, frame: LipSyncFrame) {
        store_f32(&self.amplitude_bits, frame.amplitude);
        store_f32(&self.mouth_open_bits, frame.mouth_open);
        store_f32(&self.viseme_aa_bits, frame.viseme_aa);
        store_f32(&self.viseme_o_bits, frame.viseme_o);
        store_f32(&self.viseme_e_bits, frame.viseme_e);
    }

    pub(crate) fn reset_levels(&self) {
        self.set_levels(LipSyncFrame::default());
    }

    /// Called from the capture callback or the capture test harness.
    pub fn record_capture(&self, captured: usize, dropped: usize) {
        self.capture_frames
            .fetch_add(captured as u64, Ordering::Relaxed);
        if dropped > 0 {
            self.capture_dropped
                .fetch_add(dropped as u64, Ordering::Relaxed);
        }
    }

    /// Called from the output callback when the playback ring underruns.
    pub fn record_underrun(&self) {
        self.playback_underruns.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_error(&self, code: i32) {
        self.error_code.store(code, Ordering::Release);
    }

    pub(crate) fn set_dsp_micros(&self, micros: u64) {
        self.dsp_micros
            .store(micros.min(u64::from(u32::MAX)) as u32, Ordering::Relaxed);
    }
}

fn load_f32(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Acquire))
}

fn store_f32(atomic: &AtomicU32, value: f32) {
    atomic.store(value.clamp(0.0, 1.0).to_bits(), Ordering::Release);
}

/// Handle to the running audio engine.
#[derive(Debug)]
pub struct AudioRuntime {
    command_tx: Sender<AudioCommand>,
    meters: Arc<AudioMeters>,
    control: Mutex<Option<JoinHandle<()>>>,
}

impl AudioRuntime {
    #[must_use]
    pub fn start(config: AudioConfig) -> Self {
        let (command_tx, command_rx) = bounded(COMMAND_QUEUE);
        let (job_tx, job_rx) = bounded(DSP_QUEUE);
        let (result_tx, result_rx) = bounded(DSP_QUEUE);
        let meters = Arc::new(AudioMeters::default());

        let dsp_meters = meters.clone();
        let dsp_handle = thread::Builder::new()
            .name("tomiko-dsp".to_owned())
            .spawn(move || dsp_worker(config.sample_rate, job_rx, result_tx, dsp_meters))
            .ok();

        let control_meters = meters.clone();
        let control = thread::Builder::new()
            .name("tomiko-audio".to_owned())
            .spawn(move || {
                let controller = Controller::new(
                    config,
                    control_meters,
                    command_rx,
                    job_tx,
                    result_rx,
                    dsp_handle,
                );
                controller.run();
            })
            .ok();

        if control.is_none() {
            meters.record_error(ErrorCode::AudioInit as i32);
            meters.set_phase(EngineState::Error);
        }

        Self {
            command_tx,
            meters,
            control: Mutex::new(control),
        }
    }

    #[must_use]
    pub fn meters(&self) -> Arc<AudioMeters> {
        self.meters.clone()
    }

    pub fn send(&self, command: AudioCommand) -> Result<(), EngineError> {
        self.command_tx
            .try_send(command)
            .map_err(|error| match error {
                crossbeam_channel::TrySendError::Full(_) => {
                    EngineError::AudioDevice("audio command queue is full".to_owned())
                }
                crossbeam_channel::TrySendError::Disconnected(_) => EngineError::ChannelClosed,
            })
    }

    pub fn shutdown(&self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
        if let Some(handle) = self
            .control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

enum DspJob {
    Parameters(VoicePresetParameters),
    Process(Vec<i16>),
    Shutdown,
}

enum DspResult {
    Processed {
        samples: Vec<i16>,
        input_gain: f32,
    },
    Failed(i32),
}

struct Controller {
    config: AudioConfig,
    meters: Arc<AudioMeters>,
    commands: Receiver<AudioCommand>,
    jobs: Sender<DspJob>,
    results: Receiver<DspResult>,
    dsp_handle: Option<JoinHandle<()>>,
    streams: Option<OpenedStreams>,
    vad: Vad,
    pending: Vec<i16>,
    frame: Vec<i16>,
    scratch: Vec<i16>,
    last_processed: Vec<i16>,
    remaining: Arc<AtomicU32>,
    output_discard: Arc<AtomicU32>,
    listening: bool,
    paused: bool,
    capture_active: bool,
    capture_seen: bool,
    phase: EngineState,
    sample_rate: u32,
}

impl Controller {
    fn new(
        config: AudioConfig,
        meters: Arc<AudioMeters>,
        commands: Receiver<AudioCommand>,
        jobs: Sender<DspJob>,
        results: Receiver<DspResult>,
        dsp_handle: Option<JoinHandle<()>>,
    ) -> Self {
        let sample_rate = config.sample_rate;
        let vad_config = VadConfig::from_audio(&config);
        let frame_frames = vad_config.frame_frames;
        meters.set_phase(EngineState::Idle);
        Self {
            config,
            meters,
            commands,
            jobs,
            results,
            dsp_handle,
            streams: None,
            vad: Vad::new(vad_config),
            pending: Vec::with_capacity(frame_frames * 4),
            frame: Vec::with_capacity(frame_frames),
            scratch: vec![0; 8_192],
            last_processed: Vec::new(),
            remaining: Arc::new(AtomicU32::new(0)),
            output_discard: Arc::new(AtomicU32::new(0)),
            listening: false,
            paused: false,
            capture_active: false,
            capture_seen: false,
            phase: EngineState::Idle,
            sample_rate,
        }
    }

    fn run(mut self) {
        while self.tick() {}
        self.stop_capture();
        if let Some(streams) = &self.streams {
            streams.stop_output();
        }
        self.streams = None;
        let _ = self.jobs.send(DspJob::Shutdown);
        if let Some(handle) = self.dsp_handle.take() {
            let _ = handle.join();
        }
        self.meters.set_phase(EngineState::Idle);
        self.meters.reset_levels();
    }

    fn tick(&mut self) -> bool {
        loop {
            match self.commands.try_recv() {
                Ok(command) => {
                    if !self.handle_command(command) {
                        return false;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return false,
            }
        }

        while let Ok(result) = self.results.try_recv() {
            self.handle_result(result);
        }

        self.check_playback_finished();
        self.pump_capture();

        let timeout = if matches!(self.phase, EngineState::Listening | EngineState::Capturing) {
            Duration::from_millis(2)
        } else {
            Duration::from_millis(10)
        };
        match self.commands.recv_timeout(timeout) {
            Ok(command) => self.handle_command(command),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => true,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => false,
        }
    }

    fn handle_command(&mut self, command: AudioCommand) -> bool {
        match command {
            AudioCommand::StartListening => self.listen(),
            AudioCommand::StopListening => self.stop_listening(),
            AudioCommand::PlayLast => self.play_last(),
            AudioCommand::Pause => self.pause(),
            AudioCommand::Resume => self.resume(),
            AudioCommand::SetPreset { preset, intensity } => {
                self.set_preset(preset, intensity);
            }
            AudioCommand::SelfTest => self.self_test(),
            AudioCommand::SelfTestCapture => self.self_test_capture(),
            AudioCommand::Shutdown => return false,
        }
        true
    }

    fn listen(&mut self) {
        if let Err(error) = self.config.validate() {
            self.fail(&error);
            return;
        }
        self.listening = true;
        self.paused = false;
        if let Err(error) = self.ensure_streams() {
            self.fail(&error);
            return;
        }
        if matches!(self.phase, EngineState::Playing | EngineState::Processing) {
            log::info!("Audio listening requested; capture resumes after playback");
            return;
        }
        self.vad.reset();
        self.pending.clear();
        if let Err(error) = self.start_capture() {
            self.fail(&error);
            return;
        }
        self.set_phase(EngineState::Listening);
        log::info!("Audio listening started");
    }

    fn stop_listening(&mut self) {
        self.listening = false;
        self.stop_capture();
        self.vad.reset();
        self.pending.clear();
        if !matches!(self.phase, EngineState::Playing | EngineState::Processing) {
            if let Some(streams) = &self.streams {
                streams.stop_output();
            }
            self.set_phase(EngineState::Idle);
        }
        log::info!("Audio listening stopped");
    }

    fn pause(&mut self) {
        self.paused = true;
        self.stop_capture();
        if let Some(streams) = &mut self.streams {
            streams.stop_output();
            let stale = streams.output_producer().occupied_len() as u32;
            self.output_discard.store(stale, Ordering::Release);
        }
        self.remaining.store(0, Ordering::Release);
        self.meters.reset_levels();
        self.set_phase(EngineState::Paused);
        log::info!("Audio paused");
    }

    fn resume(&mut self) {
        self.paused = false;
        if !self.listening {
            self.set_phase(EngineState::Idle);
            return;
        }
        if let Err(error) = self.ensure_streams() {
            self.fail(&error);
            return;
        }
        self.vad.reset();
        self.pending.clear();
        if let Err(error) = self.start_capture() {
            self.fail(&error);
            return;
        }
        self.set_phase(EngineState::Listening);
        log::info!("Audio resumed");
    }

    fn play_last(&mut self) {
        if self.last_processed.is_empty() {
            log::info!("PlayLast ignored: no processed utterance");
            return;
        }
        let samples = self.last_processed.clone();
        self.enqueue_playback(samples);
    }

    fn set_preset(&mut self, preset: VoicePreset, intensity: f32) {
        let parameters = blend_parameters(parameters_for(preset), intensity);
        if self.jobs.try_send(DspJob::Parameters(parameters)).is_err() {
            log::warn!("DSP worker busy; preset will apply to the next utterance");
        }
        log::info!(
            "Voice preset {} at intensity {:.2}",
            preset.display_name(),
            intensity
        );
    }

    fn self_test(&mut self) {
        if let Err(error) = self.ensure_streams() {
            self.fail(&error);
            return;
        }
        self.paused = false;
        self.stop_capture();
        self.vad.reset();
        self.pending.clear();
        let samples = synthesize_test_utterance(self.sample_rate, 1_200);
        self.process_utterance(samples, "self-test");
    }

    fn self_test_capture(&mut self) {
        if let Err(error) = self.ensure_streams() {
            self.fail(&error);
            return;
        }
        self.paused = false;
        self.stop_capture();
        self.vad.reset();
        self.pending.clear();

        let frame_frames = self.vad.config().frame_frames;
        let silence = vec![0i16; frame_frames];
        for _ in 0..5 {
            self.vad.push_frame(&silence);
        }

        let mut speech = synthesize_test_utterance(self.sample_rate, 1_200);
        // Exercise the distant-speech path: low input level, then VAD and the
        // utterance normalization in the DSP worker.
        for sample in &mut speech {
            *sample = (f32::from(*sample) * 0.15) as i16;
        }
        let mut ready = false;
        for chunk in speech.chunks(frame_frames) {
            let mut frame = vec![0i16; frame_frames];
            frame[..chunk.len()].copy_from_slice(chunk);
            if let VadEvent::UtteranceReady { frames } = self.vad.push_frame(&frame) {
                log::info!("VAD self-test produced an utterance of {frames} frames");
                ready = true;
                break;
            }
        }
        if !ready {
            for _ in 0..120 {
                if let VadEvent::UtteranceReady { frames } = self.vad.push_frame(&silence) {
                    log::info!("VAD self-test produced an utterance of {frames} frames");
                    ready = true;
                    break;
                }
            }
        }

        if !ready {
            self.vad.reset();
            self.fail(&EngineError::AudioDevice(
                "VAD self-test did not produce an utterance".to_owned(),
            ));
            return;
        }

        let samples = self.vad.utterance().to_vec();
        self.vad.clear_utterance();
        self.process_utterance(samples, "vad-self-test");
    }

    fn ensure_streams(&mut self) -> Result<(), EngineError> {
        if let Some(streams) = &self.streams {
            // The output stream may have been stopped by pause/stop-listening.
            streams.start_output()?;
            return Ok(());
        }
        let streams = aaudio::open(
            &self.config,
            self.meters.clone(),
            self.remaining.clone(),
            self.output_discard.clone(),
        )?;
        self.sample_rate = streams.sample_rate();
        streams.start_output()?;
        log::info!("Audio streams ready at {} Hz", self.sample_rate);
        self.streams = Some(streams);
        Ok(())
    }

    fn start_capture(&mut self) -> Result<(), EngineError> {
        if self.capture_active {
            return Ok(());
        }
        let streams = self.streams.as_ref().ok_or(EngineError::NotInitialized)?;
        streams.start_input()?;
        self.capture_active = true;
        Ok(())
    }

    fn stop_capture(&mut self) {
        if self.capture_active {
            if let Some(streams) = &self.streams {
                streams.stop_input();
            }
            self.capture_active = false;
        }
    }

    fn pump_capture(&mut self) {
        if !self.capture_active || self.paused {
            return;
        }
        let frame_frames = self.vad.config().frame_frames;
        if frame_frames == 0 {
            return;
        }

        loop {
            let Some(streams) = self.streams.as_mut() else {
                return;
            };
            let read = streams.input_consumer().pop_slice(&mut self.scratch);
            if read == 0 {
                break;
            }
            self.pending.extend_from_slice(&self.scratch[..read]);
        }

        if !self.capture_seen && self.meters.capture_frames() > 0 {
            self.capture_seen = true;
            log::info!(
                "AAudio capture callback active ({} samples)",
                self.meters.capture_frames()
            );
        }

        while self.pending.len() >= frame_frames {
            self.frame.clear();
            self.frame.extend_from_slice(&self.pending[..frame_frames]);
            self.pending.drain(..frame_frames);

            match self.vad.push_frame(&self.frame) {
                VadEvent::None => {}
                VadEvent::SpeechStarted => {
                    self.set_phase(EngineState::Capturing);
                    log::info!(
                        "Speech started (rms {:.4}, noise floor {:.4})",
                        crate::buffer::rms_i16(&self.frame),
                        self.vad.noise_floor()
                    );
                }
                VadEvent::UtteranceReady { frames } => {
                    self.finish_utterance(frames);
                    break;
                }
                VadEvent::Discarded => {
                    if self.listening {
                        self.set_phase(EngineState::Listening);
                    }
                    log::debug!("Discarded short utterance");
                }
            }
        }
    }

    fn finish_utterance(&mut self, frames: usize) {
        self.stop_capture();
        let samples = self.vad.utterance().to_vec();
        self.vad.clear_utterance();
        self.pending.clear();
        log::info!(
            "Utterance captured: {} frames ({:.2}s)",
            frames,
            samples.len() as f32 / self.sample_rate as f32
        );
        self.process_utterance(samples, "capture");
    }

    fn process_utterance(&mut self, samples: Vec<i16>, origin: &str) {
        if samples.is_empty() {
            self.set_phase(if self.listening {
                EngineState::Listening
            } else {
                EngineState::Idle
            });
            return;
        }
        self.set_phase(EngineState::Processing);
        log::info!(
            "DSP start ({origin}, {} samples, {:.2}s)",
            samples.len(),
            samples.len() as f32 / self.sample_rate as f32
        );
        if let Err(error) = self.jobs.try_send(DspJob::Process(samples)) {
            log::error!("DSP queue unavailable: {error}");
            self.fail(&EngineError::AudioDevice("DSP worker is busy".to_owned()));
        }
    }

    fn handle_result(&mut self, result: DspResult) {
        match result {
            DspResult::Processed {
                samples,
                input_gain,
            } => {
                log::info!(
                    "DSP done: {} samples, {} us, input gain {:.1}x",
                    samples.len(),
                    self.meters.dsp_micros(),
                    input_gain
                );
                self.last_processed = samples.clone();
                if self.paused {
                    self.set_phase(EngineState::Paused);
                    return;
                }
                self.enqueue_playback(samples);
            }
            DspResult::Failed(code) => {
                self.meters.record_error(code);
                self.set_phase(EngineState::Error);
            }
        }
    }

    fn enqueue_playback(&mut self, samples: Vec<i16>) {
        if samples.is_empty() {
            return;
        }
        if let Err(error) = self.ensure_streams() {
            self.fail(&error);
            return;
        }
        let Some(streams) = self.streams.as_mut() else {
            self.fail(&EngineError::NotInitialized);
            return;
        };
        let written = streams.output_producer().push_slice(&samples);
        if written != samples.len() {
            log::warn!(
                "Playback ring full: wrote {written}/{} samples",
                samples.len()
            );
        }
        self.remaining.fetch_add(written as u32, Ordering::AcqRel);
        self.set_phase(EngineState::Playing);
        log::info!(
            "Playback started: {:.2}s",
            written as f32 / self.sample_rate as f32
        );
    }

    fn check_playback_finished(&mut self) {
        if self.phase != EngineState::Playing {
            return;
        }
        if self.remaining.load(Ordering::Acquire) != 0 {
            return;
        }
        self.meters.reset_levels();
        self.vad.reset();
        self.pending.clear();

        if !self.listening
            && !self.paused
            && let Some(streams) = &self.streams
        {
            streams.stop_output();
        }

        if self.paused {
            self.set_phase(EngineState::Paused);
        } else if self.listening {
            if let Err(error) = self.start_capture() {
                self.fail(&error);
                return;
            }
            self.set_phase(EngineState::Listening);
        } else {
            self.set_phase(EngineState::Idle);
        }
        log::info!(
            "Playback finished (captured={} samples, dropped={}, underruns={})",
            self.meters.capture_frames(),
            self.meters.capture_dropped(),
            self.meters.playback_underruns()
        );
    }

    fn set_phase(&mut self, phase: EngineState) {
        self.phase = phase;
        self.meters.set_phase(phase);
    }

    fn fail(&mut self, error: &EngineError) {
        log::error!("Audio engine error: {error}");
        self.meters.record_error(ErrorCode::from(error) as i32);
        self.stop_capture();
        self.set_phase(EngineState::Error);
    }
}

fn blend_parameters(base: VoicePresetParameters, intensity: f32) -> VoicePresetParameters {
    let mix = intensity.clamp(0.0, 1.0);
    VoicePresetParameters {
        pitch_ratio: 1.0 + (base.pitch_ratio - 1.0) * mix,
        formant_ratio: 1.0 + (base.formant_ratio - 1.0) * mix,
        reverb_mix: base.reverb_mix * mix,
        robot_mix: base.robot_mix * mix,
        echo_delay_ms: base.echo_delay_ms * mix,
        low_pass_hz: 20_000.0 + (base.low_pass_hz - 20_000.0) * mix,
        gain: 1.0 + (base.gain - 1.0) * mix,
    }
}

fn dsp_worker(
    sample_rate: u32,
    jobs: Receiver<DspJob>,
    results: Sender<DspResult>,
    meters: Arc<AudioMeters>,
) {
    let mut processor = match VoiceProcessor::new(sample_rate, parameters_for(VoicePreset::Normal))
    {
        Ok(processor) => processor,
        Err(error) => {
            meters.record_error(ErrorCode::from(&error) as i32);
            let _ = results.try_send(DspResult::Failed(ErrorCode::from(&error) as i32));
            return;
        }
    };

    while let Ok(job) = jobs.recv() {
        match job {
            DspJob::Parameters(parameters) => {
                if let Err(error) = processor.set_parameters(parameters) {
                    meters.record_error(ErrorCode::from(&error) as i32);
                }
            }
            DspJob::Process(input) => {
                let started = Instant::now();
                let mut output = Vec::with_capacity(input.len());
                processor.process(&input, &mut output);
                meters.set_dsp_micros(started.elapsed().as_micros() as u64);
                let _ = results.try_send(DspResult::Processed {
                    samples: output,
                    input_gain: processor.last_input_gain(),
                });
            }
            DspJob::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_zero_is_neutral() {
        let parameters = blend_parameters(parameters_for(VoicePreset::Cartoon), 0.0);
        assert!(parameters.is_neutral());
    }

    #[test]
    fn intensity_one_matches_preset() {
        let base = parameters_for(VoicePreset::Deep);
        assert_eq!(blend_parameters(base, 1.0), base);
    }

    #[test]
    fn meters_round_trip_phase_and_levels() {
        let meters = AudioMeters::default();
        assert_eq!(meters.phase(), EngineState::Idle);
        meters.set_phase(EngineState::Playing);
        assert_eq!(meters.phase(), EngineState::Playing);
        meters.set_levels(LipSyncFrame {
            amplitude: 0.25,
            mouth_open: 0.5,
            viseme_aa: 0.4,
            viseme_o: 0.2,
            viseme_e: 0.1,
        });
        assert!((meters.amplitude() - 0.25).abs() < f32::EPSILON);
        assert!((meters.mouth_open() - 0.5).abs() < f32::EPSILON);
        assert!((meters.viseme_aa() - 0.4).abs() < f32::EPSILON);
        meters.reset_levels();
        assert_eq!(meters.mouth_open(), 0.0);
    }
}
