//! AAudio capture and playback streams.
//!
//! Real-time callbacks only move samples through lock-free SPSC rings and
//! update atomics. Stream creation, VAD, DSP, and playback scheduling all run
//! on the audio control thread.

#![allow(unsafe_code)]

#[cfg(target_os = "android")]
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
#[cfg(target_os = "android")]
use std::sync::atomic::Ordering;

use ringbuf::traits::Split;
#[cfg(target_os = "android")]
use ringbuf::traits::{Consumer, Producer};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use shared_types::{AudioConfig, EngineError};

#[cfg(target_os = "android")]
use crate::lipsync::LipSync;
use crate::runtime::AudioMeters;

const INPUT_RING_SAMPLES: usize = 16_384;

#[cfg(target_os = "android")]
const OUTPUT_RING_SAMPLES: usize = 524_288;

/// Owns the AAudio streams and the control-thread ends of both rings.
pub struct OpenedStreams {
    #[cfg(target_os = "android")]
    input: ndk::audio::AudioStream,
    #[cfg(target_os = "android")]
    output: ndk::audio::AudioStream,
    input_consumer: HeapCons<i16>,
    output_producer: HeapProd<i16>,
    sample_rate: u32,
}

impl std::fmt::Debug for OpenedStreams {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenedStreams")
            .field("sample_rate", &self.sample_rate)
            .finish_non_exhaustive()
    }
}

impl OpenedStreams {
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn start_input(&self) -> Result<(), EngineError> {
        #[cfg(target_os = "android")]
        {
            use ndk::audio::AudioStreamState;

            let state = self.input.state();
            if matches!(
                state,
                AudioStreamState::Started | AudioStreamState::Starting
            ) {
                return Ok(());
            }
            if matches!(
                state,
                AudioStreamState::Stopping | AudioStreamState::Pausing | AudioStreamState::Flushing
            ) {
                let _ = self.input.wait_for_state_change(state, 20_000_000);
            }
            self.input
                .request_start()
                .map_err(|error| audio_error("starting input stream", error))
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(())
        }
    }

    pub fn stop_input(&self) {
        #[cfg(target_os = "android")]
        {
            let _ = self.input.request_stop();
        }
    }

    pub fn start_output(&self) -> Result<(), EngineError> {
        #[cfg(target_os = "android")]
        {
            use ndk::audio::AudioStreamState;

            let state = self.output.state();
            if matches!(
                state,
                AudioStreamState::Started | AudioStreamState::Starting
            ) {
                return Ok(());
            }
            if matches!(
                state,
                AudioStreamState::Stopping | AudioStreamState::Pausing | AudioStreamState::Flushing
            ) {
                let _ = self.output.wait_for_state_change(state, 20_000_000);
            }
            self.output
                .request_start()
                .map_err(|error| audio_error("starting output stream", error))
        }
        #[cfg(not(target_os = "android"))]
        {
            Ok(())
        }
    }

    pub fn stop_output(&self) {
        #[cfg(target_os = "android")]
        {
            let _ = self.output.request_stop();
        }
    }

    pub fn input_consumer(&mut self) -> &mut HeapCons<i16> {
        &mut self.input_consumer
    }

    pub fn output_producer(&mut self) -> &mut HeapProd<i16> {
        &mut self.output_producer
    }
}

#[cfg(target_os = "android")]
pub fn open(
    config: &AudioConfig,
    meters: Arc<AudioMeters>,
    remaining: Arc<AtomicU32>,
    discard: Arc<AtomicU32>,
) -> Result<OpenedStreams, EngineError> {
    config.validate()?;
    let (input, input_consumer) = open_input(config, meters.clone())?;
    let sample_rate = input.sample_rate();
    if sample_rate != config.sample_rate as i32 {
        return Err(EngineError::AudioInit(format!(
            "device opened input at {sample_rate} Hz, but the DSP expects {} Hz",
            config.sample_rate
        )));
    }
    if input.channel_count() != i32::from(config.channels) {
        return Err(EngineError::AudioInit(format!(
            "input stream opened with {} channels, expected {}",
            input.channel_count(),
            config.channels
        )));
    }
    if input.format() != ndk::audio::AudioFormat::PCM_I16 {
        return Err(EngineError::AudioInit(format!(
            "input stream opened with {:?}, expected PCM_I16",
            input.format()
        )));
    }
    log::info!(
        "AAudio input opened: {} Hz, {} ch, {:?}, perf {:?}, sharing {:?}, preset {:?}",
        sample_rate,
        input.channel_count(),
        input.format(),
        input.performance_mode(),
        input.sharing_mode(),
        input.input_preset()
    );

    let (output, output_producer) =
        open_output(sample_rate as u32, config, meters, remaining, discard)?;
    if output.channel_count() != i32::from(config.channels) {
        return Err(EngineError::AudioInit(format!(
            "output stream opened with {} channels, expected {}",
            output.channel_count(),
            config.channels
        )));
    }
    if output.sample_rate() != sample_rate {
        return Err(EngineError::AudioInit(format!(
            "input/output sample rate mismatch: {sample_rate} vs {}",
            output.sample_rate()
        )));
    }
    log::info!(
        "AAudio output opened: {} Hz, {} ch, {:?}, perf {:?}, sharing {:?}",
        output.sample_rate(),
        output.channel_count(),
        output.format(),
        output.performance_mode(),
        output.sharing_mode()
    );

    Ok(OpenedStreams {
        input,
        output,
        input_consumer,
        output_producer,
        sample_rate: sample_rate as u32,
    })
}

#[cfg(not(target_os = "android"))]
pub fn open(
    config: &AudioConfig,
    _meters: Arc<AudioMeters>,
    _remaining: Arc<AtomicU32>,
    _discard: Arc<AtomicU32>,
) -> Result<OpenedStreams, EngineError> {
    config.validate()?;
    let input_ring = HeapRb::<i16>::new(INPUT_RING_SAMPLES);
    let (_producer, input_consumer) = input_ring.split();
    let output_ring = HeapRb::<i16>::new(INPUT_RING_SAMPLES);
    let (output_producer, _consumer) = output_ring.split();
    Ok(OpenedStreams {
        input_consumer,
        output_producer,
        sample_rate: config.sample_rate,
    })
}

#[cfg(target_os = "android")]
fn open_input(
    config: &AudioConfig,
    meters: Arc<AudioMeters>,
) -> Result<(ndk::audio::AudioStream, HeapCons<i16>), EngineError> {
    use ndk::audio::{
        AudioCallbackResult, AudioDirection, AudioFormat, AudioInputPreset, AudioPerformanceMode,
        AudioSharingMode, AudioStreamBuilder,
    };

    // VoiceCommunication first: it enables the platform AGC/noise suppression
    // that improves distant-speech pickup. VoicePerformance and
    // VoiceRecognition are quieter fallbacks; Generic is the last resort.
    let attempts = [
        (
            AudioSharingMode::Exclusive,
            AudioPerformanceMode::LowLatency,
            AudioInputPreset::VoiceCommunication,
        ),
        (
            AudioSharingMode::Shared,
            AudioPerformanceMode::LowLatency,
            AudioInputPreset::VoiceCommunication,
        ),
        (
            AudioSharingMode::Shared,
            AudioPerformanceMode::None,
            AudioInputPreset::VoiceCommunication,
        ),
        (
            AudioSharingMode::Shared,
            AudioPerformanceMode::None,
            AudioInputPreset::VoicePerformance,
        ),
        (
            AudioSharingMode::Shared,
            AudioPerformanceMode::None,
            AudioInputPreset::VoiceRecognition,
        ),
        (
            AudioSharingMode::Shared,
            AudioPerformanceMode::None,
            AudioInputPreset::Generic,
        ),
    ];

    let mut last_error = None;
    for (sharing, performance, preset) in attempts {
        let ring = HeapRb::<i16>::new(INPUT_RING_SAMPLES);
        let (mut producer, consumer) = ring.split();
        let callback_meters = meters.clone();
        let error_meters = meters.clone();
        let builder = AudioStreamBuilder::new().map_err(|error| {
            EngineError::AudioInit(format!("AAudio builder creation failed: {error}"))
        })?;
        let result = builder
            .direction(AudioDirection::Input)
            .performance_mode(performance)
            .sharing_mode(sharing)
            .input_preset(preset)
            .sample_rate(config.sample_rate as i32)
            .channel_count(i32::from(config.channels))
            .format(AudioFormat::PCM_I16)
            .buffer_capacity_in_frames((config.frames_per_burst * 4) as i32)
            .data_callback(Box::new(move |_stream, data: *mut c_void, frames: i32| {
                if data.is_null() || frames <= 0 {
                    return AudioCallbackResult::Continue;
                }
                // SAFETY: AAudio provides `frames` interleaved i16 samples.
                let samples =
                    unsafe { std::slice::from_raw_parts(data.cast::<i16>(), frames as usize) };
                let pushed = producer.push_slice(samples);
                callback_meters.record_capture(pushed, samples.len() - pushed);
                AudioCallbackResult::Continue
            }))
            .error_callback(Box::new(move |_stream, error| {
                log::error!("AAudio input error: {error}");
                error_meters.record_error(shared_types::ErrorCode::AudioDevice as i32);
            }))
            .open_stream();

        match result {
            Ok(stream) => return Ok((stream, consumer)),
            Err(error) => {
                log::warn!("AAudio input open failed ({sharing:?}/{performance:?}): {error}");
                last_error = Some(error);
            }
        }
    }

    Err(EngineError::AudioInit(format!(
        "could not open AAudio input: {}",
        last_error.map_or_else(|| "unknown error".to_owned(), |error| error.to_string())
    )))
}

#[cfg(target_os = "android")]
fn open_output(
    sample_rate: u32,
    config: &AudioConfig,
    meters: Arc<AudioMeters>,
    remaining: Arc<AtomicU32>,
    discard: Arc<AtomicU32>,
) -> Result<(ndk::audio::AudioStream, HeapProd<i16>), EngineError> {
    use ndk::audio::{
        AudioCallbackResult, AudioContentType, AudioDirection, AudioFormat, AudioPerformanceMode,
        AudioSharingMode, AudioStreamBuilder, AudioUsage,
    };

    let attempts = [
        (
            AudioSharingMode::Exclusive,
            AudioPerformanceMode::LowLatency,
        ),
        (AudioSharingMode::Shared, AudioPerformanceMode::LowLatency),
        (AudioSharingMode::Shared, AudioPerformanceMode::None),
    ];

    let mut last_error = None;
    for (sharing, performance) in attempts {
        let ring = HeapRb::<i16>::new(OUTPUT_RING_SAMPLES);
        let (producer, mut consumer) = ring.split();
        let callback_meters = meters.clone();
        let error_meters = meters.clone();
        let callback_remaining = remaining.clone();
        let callback_discard = discard.clone();
        let mut lip_sync = LipSync::new(sample_rate);
        let builder = AudioStreamBuilder::new().map_err(|error| {
            EngineError::AudioInit(format!("AAudio builder creation failed: {error}"))
        })?;
        let result = builder
            .direction(AudioDirection::Output)
            .performance_mode(performance)
            .sharing_mode(sharing)
            .usage(AudioUsage::Media)
            .content_type(AudioContentType::Speech)
            .sample_rate(sample_rate as i32)
            .channel_count(i32::from(config.channels))
            .format(AudioFormat::PCM_I16)
            .buffer_capacity_in_frames((config.frames_per_burst * 4) as i32)
            .data_callback(Box::new(move |_stream, data: *mut c_void, frames: i32| {
                if data.is_null() || frames <= 0 {
                    return AudioCallbackResult::Continue;
                }
                let wanted = frames as usize;
                // SAFETY: AAudio provides a writable buffer of `frames` i16 samples.
                let output = unsafe { std::slice::from_raw_parts_mut(data.cast::<i16>(), wanted) };

                // Samples left over from a paused playback are discarded
                // before any new utterance starts.
                let mut leftover = callback_discard.load(Ordering::Acquire);
                if leftover > 0 {
                    let mut scratch = [0i16; 512];
                    while leftover > 0 {
                        let chunk = (leftover as usize).min(scratch.len());
                        let discarded = consumer.pop_slice(&mut scratch[..chunk]);
                        if discarded == 0 {
                            break;
                        }
                        leftover = leftover.saturating_sub(discarded as u32);
                        callback_discard.store(leftover, Ordering::Release);
                    }
                }

                let queued = callback_remaining.load(Ordering::Acquire);
                let played = consumer.pop_slice(output);
                if played < wanted {
                    output[played..].fill(0);
                    if queued > 0 {
                        callback_meters.record_underrun();
                    }
                }

                if played > 0 && queued > 0 {
                    let played = (played as u32).min(queued);
                    let mut current = callback_remaining.load(Ordering::Acquire);
                    loop {
                        let next = current.saturating_sub(played);
                        match callback_remaining.compare_exchange_weak(
                            current,
                            next,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => break,
                            Err(actual) => current = actual,
                        }
                    }
                }

                let frame = lip_sync.process(output);
                callback_meters.set_levels(frame);
                AudioCallbackResult::Continue
            }))
            .error_callback(Box::new(move |_stream, error| {
                log::error!("AAudio output error: {error}");
                error_meters.record_error(shared_types::ErrorCode::AudioDevice as i32);
            }))
            .open_stream();

        match result {
            Ok(stream) => return Ok((stream, producer)),
            Err(error) => {
                log::warn!("AAudio output open failed ({sharing:?}/{performance:?}): {error}");
                last_error = Some(error);
            }
        }
    }

    Err(EngineError::AudioInit(format!(
        "could not open AAudio output: {}",
        last_error.map_or_else(|| "unknown error".to_owned(), |error| error.to_string())
    )))
}

#[cfg(target_os = "android")]
fn audio_error(context: &str, error: ndk::audio::AudioError) -> EngineError {
    EngineError::AudioDevice(format!("{context}: {error}"))
}
