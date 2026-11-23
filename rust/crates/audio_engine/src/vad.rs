//! Frame-based voice activity detection with pre-roll and silence timeout.
//!
//! The unit of processing is a 10 ms frame. The runtime feeds frames from the
//! lock-free capture ring, so no audio-callback work happens here.

use shared_types::AudioConfig;

use crate::buffer::rms_i16;

const FRAME_MS: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadEvent {
    None,
    SpeechStarted,
    UtteranceReady { frames: usize },
    Discarded,
}

#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    pub sample_rate: u32,
    pub frame_frames: usize,
    pub threshold_on: f32,
    pub threshold_off: f32,
    pub silence_frames: usize,
    pub max_frames: usize,
    pub pre_roll_frames: usize,
    pub min_speech_frames: usize,
    pub tail_frames: usize,
}

impl VadConfig {
    #[must_use]
    pub fn from_audio(config: &AudioConfig) -> Self {
        let sample_rate = config.sample_rate.max(8_000);
        let frame_frames = (sample_rate / (1_000 / FRAME_MS)).max(1) as usize;
        Self {
            sample_rate,
            frame_frames,
            threshold_on: 0.0025,
            threshold_off: 0.0012,
            silence_frames: (config.silence_timeout_ms / FRAME_MS).max(1) as usize,
            max_frames: (config.max_recording_ms / FRAME_MS).max(1) as usize,
            pre_roll_frames: (config.pre_roll_ms / FRAME_MS).max(1) as usize,
            min_speech_frames: 15,
            tail_frames: 15,
        }
    }
}

#[derive(Debug)]
pub struct Vad {
    config: VadConfig,
    pre_roll: Vec<i16>,
    pre_roll_write: usize,
    pre_roll_count: usize,
    utterance: Vec<i16>,
    in_speech: bool,
    voiced_frames: usize,
    silence_run: usize,
    total_frames: usize,
    last_voice_frame: usize,
    prefix_frames: usize,
    noise_floor: f32,
    ready: bool,
}

impl Vad {
    #[must_use]
    pub fn new(config: VadConfig) -> Self {
        let frame_frames = config.frame_frames.max(1);
        Self {
            config,
            pre_roll: vec![0; config.pre_roll_frames * frame_frames],
            pre_roll_write: 0,
            pre_roll_count: 0,
            utterance: Vec::with_capacity(config.max_frames * frame_frames),
            in_speech: false,
            voiced_frames: 0,
            silence_run: 0,
            total_frames: 0,
            last_voice_frame: 0,
            prefix_frames: 0,
            noise_floor: 0.001,
            ready: false,
        }
    }

    #[must_use]
    pub fn config(&self) -> VadConfig {
        self.config
    }

    #[must_use]
    pub fn is_speaking(&self) -> bool {
        self.in_speech
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    #[must_use]
    pub fn noise_floor(&self) -> f32 {
        self.noise_floor
    }

    #[must_use]
    pub fn utterance(&self) -> &[i16] {
        &self.utterance
    }

    pub fn clear_utterance(&mut self) {
        self.utterance.clear();
        self.ready = false;
    }

    /// Clears speech/pre-roll state, for example when capture stops.
    pub fn reset(&mut self) {
        self.in_speech = false;
        self.voiced_frames = 0;
        self.silence_run = 0;
        self.total_frames = 0;
        self.last_voice_frame = 0;
        self.prefix_frames = 0;
        self.noise_floor = 0.001;
        self.pre_roll_write = 0;
        self.pre_roll_count = 0;
        self.utterance.clear();
        self.ready = false;
    }

    /// Feeds exactly one frame. Callers should use chunks of
    /// [`VadConfig::frame_frames`] samples; shorter tails are ignored.
    pub fn push_frame(&mut self, frame: &[i16]) -> VadEvent {
        if frame.len() < self.config.frame_frames {
            return VadEvent::None;
        }
        let frame = &frame[..self.config.frame_frames];
        let level = rms_i16(frame);

        if !self.in_speech {
            if level < self.noise_floor {
                self.noise_floor += (level - self.noise_floor) * 0.2;
            } else {
                self.noise_floor += (level - self.noise_floor) * 0.003;
            }
            self.noise_floor = self.noise_floor.clamp(0.0002, 0.08);

            self.push_pre_roll(frame);
            if level < self.threshold_on() {
                return VadEvent::None;
            }

            self.in_speech = true;
            self.utterance.clear();
            self.copy_pre_roll_into_utterance();
            // The pre-roll copy already includes the frame that triggered
            // speech; `total_frames` counts that frame too.
            self.prefix_frames = self.pre_roll_count.saturating_sub(1);
            self.voiced_frames = 1;
            self.silence_run = 0;
            self.total_frames = 1;
            self.last_voice_frame = 0;
            return VadEvent::SpeechStarted;
        }

        let frame_frames = self.config.frame_frames;
        let capacity = self.config.max_frames * frame_frames;
        if self.utterance.len() + frame_frames <= capacity {
            self.utterance.extend_from_slice(frame);
        }
        self.total_frames += 1;

        if level >= self.threshold_off() {
            self.voiced_frames += 1;
            self.silence_run = 0;
            self.last_voice_frame = self.total_frames - 1;
        } else {
            self.silence_run += 1;
        }

        let timed_out = self.silence_run >= self.config.silence_frames;
        let too_long = self.total_frames >= self.config.max_frames;
        if !timed_out && !too_long {
            return VadEvent::None;
        }

        self.finish()
    }

    fn threshold_on(&self) -> f32 {
        self.config.threshold_on.max(self.noise_floor * 2.5)
    }

    fn threshold_off(&self) -> f32 {
        self.config.threshold_off.max(self.noise_floor * 1.5)
    }

    fn finish(&mut self) -> VadEvent {
        let frame_frames = self.config.frame_frames;
        let buffered_frames = self.utterance.len() / frame_frames;
        let keep_frames =
            (self.prefix_frames + self.last_voice_frame + 1 + self.config.tail_frames)
                .min(buffered_frames)
                .max(1);
        self.utterance.truncate(keep_frames * frame_frames);

        let enough_speech = self.voiced_frames >= self.config.min_speech_frames;
        let enough_audio = self.utterance.len() >= frame_frames * self.config.min_speech_frames;
        let accepted = enough_speech && enough_audio;

        self.in_speech = false;
        self.voiced_frames = 0;
        self.silence_run = 0;
        self.total_frames = 0;
        self.last_voice_frame = 0;
        self.prefix_frames = 0;
        self.pre_roll_write = 0;
        self.pre_roll_count = 0;

        if accepted {
            self.ready = true;
            VadEvent::UtteranceReady {
                frames: self.utterance.len() / frame_frames,
            }
        } else {
            self.utterance.clear();
            VadEvent::Discarded
        }
    }

    fn push_pre_roll(&mut self, frame: &[i16]) {
        let frame_frames = self.config.frame_frames;
        let slots = self.config.pre_roll_frames.max(1);
        let slot = self.pre_roll_write % slots;
        let start = slot * frame_frames;
        self.pre_roll[start..start + frame_frames].copy_from_slice(frame);
        self.pre_roll_write = (self.pre_roll_write + 1) % slots;
        self.pre_roll_count = (self.pre_roll_count + 1).min(slots);
    }

    fn copy_pre_roll_into_utterance(&mut self) {
        let frame_frames = self.config.frame_frames;
        let slots = self.config.pre_roll_frames.max(1);
        let start_slot = if self.pre_roll_count >= slots {
            self.pre_roll_write
        } else {
            0
        };
        for offset in 0..self.pre_roll_count {
            let slot = (start_slot + offset) % slots;
            let start = slot * frame_frames;
            self.utterance
                .extend_from_slice(&self.pre_roll[start..start + frame_frames]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> VadConfig {
        VadConfig {
            sample_rate: 48_000,
            frame_frames: 480,
            threshold_on: 0.025,
            threshold_off: 0.015,
            silence_frames: 80,
            max_frames: 1_000,
            pre_roll_frames: 30,
            min_speech_frames: 15,
            tail_frames: 15,
        }
    }

    fn silence_frame() -> Vec<i16> {
        vec![0; 480]
    }

    fn tone_frame(amplitude: f32) -> Vec<i16> {
        (0..480)
            .map(|index| {
                let phase = std::f32::consts::TAU * 220.0 * index as f32 / 48_000.0;
                (phase.sin() * amplitude * f32::from(i16::MAX)) as i16
            })
            .collect()
    }

    fn voice_frame() -> Vec<i16> {
        tone_frame(0.244)
    }

    #[test]
    fn quiet_speech_triggers_after_adaptation() {
        let mut config = config();
        config.threshold_on = 0.0025;
        config.threshold_off = 0.0012;
        let mut vad = Vad::new(config);
        for _ in 0..50 {
            assert_eq!(vad.push_frame(&tone_frame(0.001)), VadEvent::None);
        }
        let mut started = false;
        for _ in 0..20 {
            if vad.push_frame(&tone_frame(0.006)) == VadEvent::SpeechStarted {
                started = true;
                break;
            }
        }
        assert!(started);
    }

    #[test]
    fn steady_low_noise_stays_below_threshold() {
        let mut config = config();
        config.threshold_on = 0.0025;
        config.threshold_off = 0.0012;
        let mut vad = Vad::new(config);
        for _ in 0..200 {
            assert_eq!(vad.push_frame(&tone_frame(0.001)), VadEvent::None);
        }
    }

    #[test]
    fn silence_never_starts_speech() {
        let mut vad = Vad::new(config());
        for _ in 0..100 {
            assert_eq!(vad.push_frame(&silence_frame()), VadEvent::None);
        }
        assert!(!vad.is_speaking());
        assert!(!vad.is_ready());
    }

    #[test]
    fn voice_burst_produces_utterance_with_tail_trimmed() {
        let mut vad = Vad::new(config());
        let mut events = Vec::new();
        for _ in 0..5 {
            events.push(vad.push_frame(&silence_frame()));
        }
        for _ in 0..30 {
            events.push(vad.push_frame(&voice_frame()));
        }
        for _ in 0..120 {
            events.push(vad.push_frame(&silence_frame()));
        }
        assert!(events.contains(&VadEvent::SpeechStarted));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, VadEvent::UtteranceReady { .. }))
        );
        assert!(vad.is_ready());
        // 5 pre-roll + 30 voice + 15 tail frames.
        assert_eq!(vad.utterance().len(), 50 * 480);
        assert!(vad.is_ready());
    }

    #[test]
    fn click_is_discarded() {
        let mut vad = Vad::new(config());
        let mut events = Vec::new();
        for _ in 0..2 {
            events.push(vad.push_frame(&voice_frame()));
        }
        for _ in 0..120 {
            events.push(vad.push_frame(&silence_frame()));
        }
        assert!(events.contains(&VadEvent::Discarded));
        assert!(!vad.is_ready());
    }

    #[test]
    fn pre_roll_is_bounded() {
        let mut vad = Vad::new(config());
        for _ in 0..100 {
            vad.push_frame(&silence_frame());
        }
        for _ in 0..30 {
            vad.push_frame(&voice_frame());
        }
        for _ in 0..120 {
            vad.push_frame(&silence_frame());
        }
        // 30 pre-roll frames (the last one is the first voice frame), the
        // remaining 29 voice frames, and 15 tail frames.
        assert_eq!(vad.utterance().len(), (30 + 29 + 15) * 480);
    }

    #[test]
    fn maximum_duration_ends_utterance() {
        let mut config = config();
        config.max_frames = 100;
        let mut vad = Vad::new(config);
        let mut ready = false;
        for _ in 0..100 {
            if let VadEvent::UtteranceReady { .. } = vad.push_frame(&voice_frame()) {
                ready = true;
                break;
            }
        }
        assert!(ready);
        assert_eq!(vad.utterance().len(), 100 * 480);
    }
}
