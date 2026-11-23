use shared_types::EngineError;

/// Fixed-capacity PCM buffer. `extend` never allocates and reports overflow,
/// which makes it safe to use from a preallocated capture/pre-roll path.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    samples: Vec<i16>,
    len: usize,
}

impl AudioBuffer {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: vec![0; capacity],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.samples.len()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[i16] {
        &self.samples[..self.len]
    }

    /// Appends samples without allocating. Returns `false` and copies nothing
    /// if the buffer does not have enough remaining capacity.
    pub fn extend(&mut self, input: &[i16]) -> bool {
        let Some(end) = self.len.checked_add(input.len()) else {
            return false;
        };
        if end > self.samples.len() {
            return false;
        }
        self.samples[self.len..end].copy_from_slice(input);
        self.len = end;
        true
    }

    pub fn extend_checked(&mut self, input: &[i16]) -> Result<(), EngineError> {
        if self.extend(input) {
            Ok(())
        } else {
            Err(EngineError::AudioDevice(format!(
                "audio buffer overflow: len={}, input={}, capacity={}",
                self.len,
                input.len(),
                self.samples.len()
            )))
        }
    }
}

/// Root-mean-square of normalized PCM16 samples.
#[must_use]
pub fn rms_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples
        .iter()
        .map(|&sample| {
            let value = f64::from(sample) / f64::from(i16::MAX);
            value * value
        })
        .sum();
    (sum / samples.len() as f64).sqrt() as f32
}

/// Absolute peak of normalized PCM16 samples.
#[must_use]
pub fn peak_i16(samples: &[i16]) -> f32 {
    samples
        .iter()
        .map(|&sample| (f32::from(sample) / f32::from(i16::MAX)).abs())
        .fold(0.0, f32::max)
}

/// Exponential envelope with independent attack and release times.
#[derive(Debug, Clone)]
pub struct EnvelopeFollower {
    attack_coeff: f32,
    release_coeff: f32,
    value: f32,
}

impl EnvelopeFollower {
    #[must_use]
    pub fn new(attack_ms: f32, release_ms: f32, sample_rate: u32) -> Self {
        let sample_rate = sample_rate.max(1) as f32;
        Self {
            attack_coeff: time_coefficient(attack_ms, sample_rate),
            release_coeff: time_coefficient(release_ms, sample_rate),
            value: 0.0,
        }
    }

    /// Processes one control-rate value. `target` should already be normalized.
    pub fn process(&mut self, target: f32) -> f32 {
        let target = target.clamp(0.0, 1.0);
        let coeff = if target > self.value {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.value += coeff * (target - self.value);
        self.value = self.value.clamp(0.0, 1.0);
        self.value
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
    }
}

fn time_coefficient(time_ms: f32, sample_rate: f32) -> f32 {
    if time_ms <= 0.0 {
        return 1.0;
    }
    let samples = (time_ms / 1_000.0) * sample_rate;
    if samples <= 1.0 {
        1.0
    } else {
        1.0 - (-1.0 / samples).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_audio_has_zero_rms() {
        assert_eq!(rms_i16(&[]), 0.0);
        assert_eq!(peak_i16(&[]), 0.0);
    }

    #[test]
    fn full_scale_square_wave_has_expected_rms() {
        let samples = [i16::MAX, i16::MIN, i16::MAX, i16::MIN];
        let rms = rms_i16(&samples);
        assert!((rms - 1.0).abs() < 0.001, "rms={rms}");
        assert!((peak_i16(&samples) - 1.0).abs() < 0.001);
    }

    #[test]
    fn buffer_never_allocates_on_overflow() {
        let mut buffer = AudioBuffer::with_capacity(4);
        assert!(buffer.extend(&[1, 2, 3]));
        assert!(!buffer.extend(&[4, 5]));
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.capacity(), 4);
        assert!(buffer.extend_checked(&[4]).is_ok());
        assert!(buffer.extend_checked(&[5]).is_err());
    }

    #[test]
    fn envelope_attacks_faster_than_it_releases() {
        let mut envelope = EnvelopeFollower::new(10.0, 200.0, 1_000);
        let attack = envelope.process(1.0);
        let mut release_envelope = EnvelopeFollower::new(10.0, 200.0, 1_000);
        release_envelope.process(1.0);
        let release = release_envelope.process(0.0);
        assert!(attack > release, "attack={attack}, release={release}");
    }

    #[test]
    fn envelope_clamps_to_unit_range() {
        let mut envelope = EnvelopeFollower::new(1.0, 1.0, 48_000);
        assert!((0.0..=1.0).contains(&envelope.process(2.0)));
        assert!((0.0..=1.0).contains(&envelope.process(-1.0)));
    }
}
