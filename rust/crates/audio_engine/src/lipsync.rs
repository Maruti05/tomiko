//! Amplitude/spectral lip-sync approximation for the first release.
//!
//! This is deliberately not phoneme-accurate. It derives mouth openness from
//! the playback RMS and a broad rounded/bright split from the zero-crossing
//! rate. A real viseme tracker is a future phase.

use crate::buffer::rms_i16;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LipSyncFrame {
    pub amplitude: f32,
    pub mouth_open: f32,
    pub viseme_aa: f32,
    pub viseme_o: f32,
    pub viseme_e: f32,
}

#[derive(Debug)]
pub struct LipSync {
    sample_rate: u32,
    envelope: f32,
    brightness: f32,
}

impl LipSync {
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate.max(8_000),
            envelope: 0.0,
            brightness: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.envelope = 0.0;
        self.brightness = 0.0;
    }

    /// Processes one playback callback block.
    pub fn process(&mut self, samples: &[i16]) -> LipSyncFrame {
        if samples.is_empty() {
            return LipSyncFrame::default();
        }

        let rms = rms_i16(samples).clamp(0.0, 1.0);
        let duration = samples.len() as f32 / self.sample_rate as f32;
        let coefficient = if rms > self.envelope {
            1.0 - (-duration / 0.012).exp()
        } else {
            1.0 - (-duration / 0.090).exp()
        };
        self.envelope += coefficient * (rms - self.envelope);
        self.envelope = self.envelope.clamp(0.0, 1.0);

        let mut crossings = 0usize;
        for pair in samples.windows(2) {
            if (pair[0] >= 0) != (pair[1] >= 0) {
                crossings += 1;
            }
        }
        let zcr = crossings as f32 / samples.len() as f32;
        let target_brightness = (zcr / 0.18).clamp(0.0, 1.0);
        self.brightness += (target_brightness - self.brightness) * 0.25;
        self.brightness = self.brightness.clamp(0.0, 1.0);

        let mouth_open = (self.envelope * 3.0).clamp(0.0, 1.0).powf(0.75);
        let amplitude = self.envelope;

        if mouth_open < 0.001 {
            return LipSyncFrame {
                amplitude,
                mouth_open: 0.0,
                viseme_aa: 0.0,
                viseme_o: 0.0,
                viseme_e: 0.0,
            };
        }

        let viseme_aa = mouth_open * (0.35 + 0.50 * self.brightness);
        let viseme_o = mouth_open * (0.55 * (1.0 - self.brightness));
        let viseme_e = mouth_open * (0.35 * self.brightness);

        LipSyncFrame {
            amplitude,
            mouth_open,
            viseme_aa: viseme_aa.clamp(0.0, 1.0),
            viseme_o: viseme_o.clamp(0.0, 1.0),
            viseme_e: viseme_e.clamp(0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn tone(frequency: f32, amplitude: f32) -> Vec<i16> {
        (0..960)
            .map(|index| {
                let phase = TAU * frequency * index as f32 / 48_000.0;
                (phase.sin() * amplitude * f32::from(i16::MAX)) as i16
            })
            .collect()
    }

    #[test]
    fn silence_closes_the_mouth() {
        let mut lip_sync = LipSync::new(48_000);
        let mut frame = LipSyncFrame::default();
        for _ in 0..20 {
            frame = lip_sync.process(&[0; 480]);
        }
        assert!(frame.mouth_open < 0.01);
        assert_eq!(frame.viseme_aa, 0.0);
    }

    #[test]
    fn voice_opens_the_mouth() {
        let mut lip_sync = LipSync::new(48_000);
        let samples = tone(200.0, 0.3);
        let mut frame = LipSyncFrame::default();
        for _ in 0..10 {
            frame = lip_sync.process(&samples);
        }
        assert!(frame.mouth_open > 0.2, "mouth={}", frame.mouth_open);
        assert!(frame.viseme_aa > 0.0);
    }

    #[test]
    fn high_frequency_is_brighter_than_low_frequency() {
        let mut low = LipSync::new(48_000);
        let mut high = LipSync::new(48_000);
        let low_samples = tone(150.0, 0.3);
        let high_samples = tone(2_000.0, 0.3);
        let mut low_frame = LipSyncFrame::default();
        let mut high_frame = LipSyncFrame::default();
        for _ in 0..10 {
            low_frame = low.process(&low_samples);
            high_frame = high.process(&high_samples);
        }
        assert!(high_frame.viseme_e > low_frame.viseme_e);
        assert!(low_frame.viseme_o > high_frame.viseme_o);
    }

    #[test]
    fn envelope_releases_after_sound_stops() {
        let mut lip_sync = LipSync::new(48_000);
        let samples = tone(200.0, 0.3);
        let mut frame = LipSyncFrame::default();
        for _ in 0..10 {
            frame = lip_sync.process(&samples);
        }
        let open = frame.mouth_open;
        for _ in 0..40 {
            frame = lip_sync.process(&[0; 480]);
        }
        assert!(frame.mouth_open < open * 0.2);
    }
}
