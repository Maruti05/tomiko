//! Voice DSP chain.
//!
//! `VoiceProcessor` is used by the offline DSP worker, never by an AAudio
//! callback. Buffers are preallocated per processor and reused between
//! utterances.
//!
//! Pitch shifting uses WSOLA (waveform-similarity overlap-add) followed by
//! Catmull-Rom resampling. Formant handling is currently a spectral tilt
//! approximation, not true formant-preserving pitch shifting; see
//! `docs/AUDIO_PIPELINE.md`.

use std::f32::consts::{PI, TAU};

use shared_types::EngineError;

use crate::presets::VoicePresetParameters;

const MIN_PITCH: f32 = 0.5;
const MAX_PITCH: f32 = 2.0;

/// Direct-form II transposed biquad.
#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from_coefficients(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv = if a0.abs() > f32::EPSILON {
            1.0 / a0
        } else {
            0.0
        };
        Self {
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            a1: a1 * inv,
            a2: a2 * inv,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[must_use]
    pub fn low_pass(sample_rate: f32, cutoff_hz: f32, q: f32) -> Self {
        let (cos_w0, alpha) = rbj_terms(sample_rate, cutoff_hz, q);
        let b1 = 1.0 - cos_w0;
        let b0 = b1 * 0.5;
        let b2 = b0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        Self::from_coefficients(b0, b1, b2, a0, a1, a2)
    }

    #[must_use]
    pub fn high_pass(sample_rate: f32, cutoff_hz: f32, q: f32) -> Self {
        let (cos_w0, alpha) = rbj_terms(sample_rate, cutoff_hz, q);
        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = b0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        Self::from_coefficients(b0, b1, b2, a0, a1, a2)
    }

    #[must_use]
    pub fn high_shelf(sample_rate: f32, cutoff_hz: f32, gain_db: f32) -> Self {
        let a = 10.0f32.powf(gain_db / 40.0);
        let (cos_w0, alpha) = rbj_terms(sample_rate, cutoff_hz, 0.707);
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
        Self::from_coefficients(b0, b1, b2, a0, a1, a2)
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

fn rbj_terms(sample_rate: f32, cutoff_hz: f32, q: f32) -> (f32, f32) {
    let nyquist = (sample_rate * 0.5).max(1.0);
    let cutoff = cutoff_hz.clamp(10.0, nyquist * 0.95);
    let w0 = TAU * cutoff / sample_rate.max(1.0);
    let cos_w0 = w0.cos();
    let alpha = w0.sin() / (2.0 * q.max(0.1));
    (cos_w0, alpha)
}

/// One-pole DC blocker.
#[derive(Debug, Clone, Copy)]
pub struct DcBlocker {
    x1: f32,
    y1: f32,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self { x1: 0.0, y1: 0.0 }
    }
}

impl DcBlocker {
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + 0.995 * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

/// Soft-knee limiter. The output is always inside `[-0.999, 0.999]`.
#[must_use]
pub fn soft_clip(x: f32) -> f32 {
    const KNEE: f32 = 0.7;
    let magnitude = x.abs();
    let limited = if magnitude <= KNEE {
        magnitude
    } else {
        let excess = (magnitude - KNEE) / (1.0 - KNEE);
        KNEE + (1.0 - KNEE) * excess.tanh()
    };
    limited.copysign(x).clamp(-0.999, 0.999)
}

/// Feedback echo with a fixed 500 ms delay line.
#[derive(Debug)]
pub struct Echo {
    buffer: Vec<f32>,
    index: usize,
    delay_samples: usize,
    mix: f32,
}

impl Echo {
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let capacity = (sample_rate as usize / 2).max(1);
        Self {
            buffer: vec![0.0; capacity],
            index: 0,
            delay_samples: 0,
            mix: 0.0,
        }
    }

    pub fn configure(&mut self, sample_rate: u32, delay_ms: f32, mix: f32) {
        let delay = (delay_ms.max(0.0) * sample_rate as f32 / 1_000.0).round() as usize;
        self.delay_samples = delay.min(self.buffer.len().saturating_sub(1));
        self.mix = mix.clamp(0.0, 1.0);
        self.buffer.fill(0.0);
        self.index = 0;
    }

    pub fn process(&mut self, x: f32) -> f32 {
        if self.delay_samples == 0 || self.mix <= 0.0 {
            return x;
        }
        let delayed = self.buffer[self.index];
        self.buffer[self.index] = x + delayed * 0.35;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        x + delayed * self.mix
    }
}

#[derive(Debug)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    damp: f32,
    store: f32,
}

impl Comb {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            index: 0,
            feedback: 0.72,
            damp: 0.25,
            store: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let out = self.buffer[self.index];
        self.store = out * (1.0 - self.damp) + self.store * self.damp;
        self.buffer[self.index] = x + self.store * self.feedback;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        out
    }
}

#[derive(Debug)]
struct AllPass {
    buffer: Vec<f32>,
    index: usize,
}

impl AllPass {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            index: 0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let buffered = self.buffer[self.index];
        let out = -x + buffered;
        self.buffer[self.index] = x + buffered * 0.5;
        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
        out
    }
}

/// Small Schroeder reverb (four combs, two all-passes).
#[derive(Debug)]
pub struct Reverb {
    combs: [Comb; 4],
    all_passes: [AllPass; 2],
    mix: f32,
}

impl Reverb {
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let scale = sample_rate as f32 / 44_100.0;
        let comb_lengths = [1_116.0f32, 1_188.0, 1_277.0, 1_356.0];
        let all_pass_lengths = [556.0f32, 441.0];
        Self {
            combs: [
                Comb::new((comb_lengths[0] * scale) as usize),
                Comb::new((comb_lengths[1] * scale) as usize),
                Comb::new((comb_lengths[2] * scale) as usize),
                Comb::new((comb_lengths[3] * scale) as usize),
            ],
            all_passes: [
                AllPass::new((all_pass_lengths[0] * scale) as usize),
                AllPass::new((all_pass_lengths[1] * scale) as usize),
            ],
            mix: 0.0,
        }
    }

    pub fn configure(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
        if self.mix <= 0.0 {
            for comb in &mut self.combs {
                comb.buffer.fill(0.0);
                comb.store = 0.0;
            }
            for pass in &mut self.all_passes {
                pass.buffer.fill(0.0);
            }
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        if self.mix <= 0.0 {
            return x;
        }
        let mut wet = 0.0;
        for comb in &mut self.combs {
            wet += comb.process(x);
        }
        wet *= 0.25;
        for pass in &mut self.all_passes {
            wet = pass.process(wet);
        }
        x * (1.0 - self.mix) + wet * self.mix
    }
}

/// Ring modulation plus a 70 Hz sample-and-hold for the robot preset.
#[derive(Debug)]
pub struct RobotModulator {
    phase: f32,
    phase_increment: f32,
    held: f32,
    hold_counter: u32,
    hold_period: u32,
    mix: f32,
}

impl RobotModulator {
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(1);
        Self {
            phase: 0.0,
            phase_increment: TAU * 55.0 / rate as f32,
            held: 0.0,
            hold_counter: 0,
            hold_period: (rate / 70).max(1),
            mix: 0.0,
        }
    }

    pub fn configure(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn process(&mut self, x: f32) -> f32 {
        if self.mix <= 0.0 {
            return x;
        }
        self.hold_counter += 1;
        if self.hold_counter >= self.hold_period {
            self.hold_counter = 0;
            self.held = x;
        }
        self.phase += self.phase_increment;
        if self.phase >= TAU {
            self.phase -= TAU;
        }
        let wet = self.held * self.phase.sin();
        x * (1.0 - self.mix) + wet * self.mix
    }
}

/// WSOLA time-stretch plus resampling pitch shifter.
#[derive(Debug)]
pub struct Wsola {
    sample_rate: f32,
    window: usize,
    hop: usize,
    search_radius: usize,
    correlation_len: usize,
    window_table: Vec<f32>,
    stretched: Vec<f32>,
    filtered: Vec<f32>,
    anti_alias: Biquad,
    anti_alias_ratio: f32,
}

impl Wsola {
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(8_000);
        let window = (rate as usize * 30 / 1_000).max(64);
        let hop = window / 2;
        Self {
            sample_rate: rate as f32,
            window,
            hop,
            search_radius: (rate as usize * 5 / 1_000).max(8),
            correlation_len: (rate as usize * 5 / 1_000).max(16),
            window_table: hann_window(window),
            stretched: Vec::new(),
            filtered: Vec::new(),
            anti_alias: Biquad::low_pass(rate as f32, 18_000.0, 0.707),
            anti_alias_ratio: 1.0,
        }
    }

    /// Stretches `input` by `pitch_ratio`, then resamples back to the input
    /// length so playback duration is unchanged.
    pub fn process(&mut self, input: &[f32], pitch_ratio: f32, output: &mut Vec<f32>) {
        output.clear();
        if input.is_empty() {
            return;
        }
        let ratio = pitch_ratio.clamp(MIN_PITCH, MAX_PITCH);
        let target_len = ((input.len() as f32) * ratio).round().max(1.0) as usize;
        let total = target_len + self.window;
        self.stretched.clear();
        self.stretched.resize(total, 0.0);

        let analysis_hop = self.hop as f32 / ratio;
        let mut previous_position = 0usize;
        let mut frame = 0usize;

        while frame * self.hop < target_len + self.window {
            let nominal = (frame as f32 * analysis_hop).round().max(0.0) as usize;
            let position = if frame == 0 {
                0
            } else {
                self.search_position(input, nominal, previous_position)
            };
            let copy_len = self.window.min(input.len().saturating_sub(position));
            if copy_len == 0 {
                break;
            }

            let out_pos = frame * self.hop;
            let (window_table, stretched) = (&self.window_table, &mut self.stretched);
            for (offset, &weight) in window_table.iter().enumerate().take(copy_len) {
                if out_pos + offset >= stretched.len() {
                    break;
                }
                stretched[out_pos + offset] += input[position + offset] * weight;
            }

            previous_position = position;
            frame += 1;
        }

        let stretched_len = target_len.min(self.stretched.len());
        let source = &self.stretched[..stretched_len];

        let cutoff = 0.45 * self.sample_rate / ratio;
        if (ratio - self.anti_alias_ratio).abs() > 0.01 {
            self.anti_alias = Biquad::low_pass(self.sample_rate, cutoff, 0.707);
            self.anti_alias_ratio = ratio;
        }

        self.filtered.clear();
        self.filtered.reserve(stretched_len);
        for &sample in source {
            self.filtered.push(self.anti_alias.process(sample));
        }

        resample_catmull_rom(&self.filtered, input.len(), output);
    }

    fn search_position(&self, input: &[f32], nominal: usize, previous: usize) -> usize {
        let max_position = input.len().saturating_sub(self.window);
        let template_start = (previous + self.hop).min(input.len());
        let template_len = self
            .correlation_len
            .min(input.len().saturating_sub(template_start));
        if template_len < 16 {
            return nominal.min(max_position);
        }
        let template = &input[template_start..template_start + template_len];

        let low = nominal.saturating_sub(self.search_radius).min(max_position);
        let high = (nominal + self.search_radius).min(max_position);
        if low >= high {
            return low;
        }

        let mut best_position = low;
        let mut best_score = f32::NEG_INFINITY;
        let coarse = 4;
        let mut candidate = low;
        while candidate <= high {
            let len = template_len.min(input.len().saturating_sub(candidate));
            if len >= 16 {
                let score = normalized_correlation(template, &input[candidate..candidate + len]);
                if score > best_score {
                    best_score = score;
                    best_position = candidate;
                }
            }
            candidate += coarse;
        }

        let refine_low = best_position.saturating_sub(coarse);
        let refine_high = (best_position + coarse).min(high);
        for candidate in refine_low..=refine_high {
            let len = template_len.min(input.len().saturating_sub(candidate));
            if len >= 16 {
                let score = normalized_correlation(template, &input[candidate..candidate + len]);
                if score > best_score {
                    best_score = score;
                    best_position = candidate;
                }
            }
        }
        best_position
    }
}

fn normalized_correlation(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut energy = 1e-9;
    for (&left, &right) in a.iter().zip(b) {
        dot += left * right;
        energy += right * right;
    }
    dot / energy.sqrt()
}

fn hann_window(length: usize) -> Vec<f32> {
    if length == 0 {
        return Vec::new();
    }
    (0..length)
        .map(|index| 0.5 - 0.5 * (TAU * index as f32 / length as f32).cos())
        .collect()
}

fn resample_catmull_rom(input: &[f32], output_len: usize, output: &mut Vec<f32>) {
    output.clear();
    if input.is_empty() || output_len == 0 {
        return;
    }
    output.reserve(output_len);
    if input.len() == 1 {
        output.resize(output_len, input[0]);
        return;
    }

    let step = (input.len() - 1) as f32 / (output_len.max(2) - 1) as f32;
    let last = input.len() - 1;
    let sample = |index: isize| -> f32 { input[index.clamp(0, last as isize) as usize] };

    for index in 0..output_len {
        let source = index as f32 * step;
        let base = source.floor() as isize;
        let fraction = source - base as f32;
        let p0 = sample(base - 1);
        let p1 = sample(base);
        let p2 = sample(base + 1);
        let p3 = sample(base + 2);
        let value = 0.5
            * ((2.0 * p1)
                + (-p0 + p2) * fraction
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * fraction * fraction
                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * fraction * fraction * fraction);
        output.push(value);
    }
}

/// Complete offline voice chain.
#[derive(Debug)]
pub struct VoiceProcessor {
    sample_rate: u32,
    parameters: VoicePresetParameters,
    dc_blocker: DcBlocker,
    high_pass: Biquad,
    low_pass: Biquad,
    formant_tilt: Biquad,
    echo: Echo,
    reverb: Reverb,
    robot: RobotModulator,
    wsola: Wsola,
    work_a: Vec<f32>,
    work_b: Vec<f32>,
    last_input_gain: f32,
}

impl VoiceProcessor {
    pub fn new(sample_rate: u32, parameters: VoicePresetParameters) -> Result<Self, EngineError> {
        if sample_rate < 8_000 {
            return Err(EngineError::InvalidConfig(format!(
                "audio sample rate {sample_rate} is too low"
            )));
        }
        parameters.validate()?;
        let rate = sample_rate as f32;
        let mut processor = Self {
            sample_rate,
            parameters,
            dc_blocker: DcBlocker::default(),
            high_pass: Biquad::high_pass(rate, 80.0, 0.707),
            low_pass: Biquad::low_pass(rate, parameters.low_pass_hz, 0.707),
            formant_tilt: formant_tilt(rate, parameters.formant_ratio),
            echo: Echo::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            robot: RobotModulator::new(sample_rate),
            wsola: Wsola::new(sample_rate),
            work_a: Vec::new(),
            work_b: Vec::new(),
            last_input_gain: 1.0,
        };
        processor.configure_effects();
        Ok(processor)
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Gain applied to normalize the most recent utterance. Values above 1.0
    /// mean the microphone input was quiet (for example, distant speech).
    #[must_use]
    pub fn last_input_gain(&self) -> f32 {
        self.last_input_gain
    }

    pub fn set_parameters(&mut self, parameters: VoicePresetParameters) -> Result<(), EngineError> {
        parameters.validate()?;
        self.parameters = parameters;
        let rate = self.sample_rate as f32;
        self.low_pass = Biquad::low_pass(rate, parameters.low_pass_hz, 0.707);
        self.formant_tilt = formant_tilt(rate, parameters.formant_ratio);
        self.configure_effects();
        Ok(())
    }

    fn configure_effects(&mut self) {
        self.echo.configure(
            self.sample_rate,
            self.parameters.echo_delay_ms,
            if self.parameters.echo_delay_ms > 0.0 {
                0.45
            } else {
                0.0
            },
        );
        self.reverb.configure(self.parameters.reverb_mix);
        self.robot.configure(self.parameters.robot_mix);
        self.dc_blocker.reset();
        self.high_pass.reset();
        self.low_pass.reset();
        self.formant_tilt.reset();
    }

    /// Processes one utterance. No allocation happens in an audio callback;
    /// this method runs on the DSP worker only.
    pub fn process(&mut self, input: &[i16], output: &mut Vec<i16>) {
        output.clear();
        if input.is_empty() {
            return;
        }

        // Normalize quiet utterances before filtering. Distant speech is often
        // 10-20 dB below close speech; this makes playback and lip-sync level
        // consistent without amplifying the VAD or background noise.
        let input_rms = crate::buffer::rms_i16(input);
        self.last_input_gain = if input_rms > 0.0004 {
            (0.10 / input_rms).clamp(1.0, 16.0)
        } else {
            1.0
        };

        self.work_a.clear();
        self.work_a.reserve(input.len());
        for &sample in input {
            let normalized =
                f32::from(sample) / f32::from(i16::MAX) * self.last_input_gain;
            self.work_a.push(normalized);
        }

        for sample in &mut self.work_a {
            let filtered = self.dc_blocker.process(*sample);
            *sample = self.high_pass.process(filtered);
        }

        if (self.parameters.pitch_ratio - 1.0).abs() < 0.001 {
            self.work_b.clear();
            self.work_b.extend_from_slice(&self.work_a);
        } else {
            self.wsola
                .process(&self.work_a, self.parameters.pitch_ratio, &mut self.work_b);
        }

        let output_len = self.work_b.len().min(input.len());
        output.reserve(output_len);
        let fade_samples = (self.sample_rate as usize / 200).max(1);
        let fade_out_start = output_len.saturating_sub(fade_samples);
        let gain = self.parameters.gain;

        for (index, &sample) in self.work_b.iter().take(output_len).enumerate() {
            let mut value = self.formant_tilt.process(sample);
            value = self.low_pass.process(value);
            value = self.robot.process(value);
            value = self.echo.process(value);
            value = self.reverb.process(value);
            value = soft_clip(value * gain);

            if index < fade_samples {
                value *= index as f32 / fade_samples as f32;
            } else if index >= fade_out_start && fade_samples > 0 {
                value *= (output_len - index) as f32 / fade_samples as f32;
            }

            output.push((value.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16);
        }
    }
}

fn formant_tilt(sample_rate: f32, formant_ratio: f32) -> Biquad {
    // Approximation only: shifts spectral brightness instead of moving
    // formants. Kept deliberately gentle to avoid artefacts.
    let gain_db = ((formant_ratio - 1.0) * 12.0).clamp(-9.0, 9.0);
    Biquad::high_shelf(sample_rate, 1_800.0, gain_db)
}

/// A short synthetic voice-like signal used by the device self-test.
#[must_use]
pub fn synthesize_test_utterance(sample_rate: u32, duration_ms: u32) -> Vec<i16> {
    let rate = sample_rate.max(8_000) as f32;
    let frames = (rate * duration_ms as f32 / 1_000.0) as usize;
    let mut output = Vec::with_capacity(frames);
    for index in 0..frames {
        let time = index as f32 / rate;
        let syllable = (TAU * 3.2 * time).sin().mul_add(0.35, 0.65).max(0.0);
        let envelope = (PI * (index as f32 / frames as f32)).sin();
        let voice = (TAU * 185.0 * time).sin() * 0.55
            + (TAU * 370.0 * time).sin() * 0.25
            + (TAU * 740.0 * time).sin() * 0.12
            + (TAU * 1_480.0 * time).sin() * 0.05;
        let sample = voice * syllable * envelope * 0.6;
        output.push((sample * f32::from(i16::MAX)) as i16);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::parameters_for;
    use shared_types::VoicePreset;

    fn sine(frequency: f32, sample_rate: u32, duration_ms: u32) -> Vec<i16> {
        let frames = (sample_rate as f32 * duration_ms as f32 / 1_000.0) as usize;
        (0..frames)
            .map(|index| {
                let time = index as f32 / sample_rate as f32;
                ((TAU * frequency * time).sin() * 0.5 * f32::from(i16::MAX)) as i16
            })
            .collect()
    }

    fn dominant_frequency(samples: &[i16], sample_rate: u32) -> f32 {
        let middle = samples.len() / 4;
        let window = &samples[middle..middle + samples.len() / 2];
        let min_lag = sample_rate as usize / 2_000;
        let max_lag = sample_rate as usize / 80;
        let mut best_lag = min_lag;
        let mut best = f32::NEG_INFINITY;
        for lag in min_lag..max_lag.min(window.len() / 2) {
            let mut dot = 0.0;
            let mut energy = 1e-9;
            for (&a, &b) in window[..window.len() - lag].iter().zip(&window[lag..]) {
                let a = f32::from(a);
                let b = f32::from(b);
                dot += a * b;
                energy += b * b;
            }
            let score = dot / energy.sqrt();
            if score > best {
                best = score;
                best_lag = lag;
            }
        }
        sample_rate as f32 / best_lag as f32
    }

    #[test]
    fn biquad_stays_finite() {
        let mut filter = Biquad::low_pass(48_000.0, 1_000.0, 0.707);
        let mut value = 1.0;
        for _ in 0..10_000 {
            value = filter.process(value);
        }
        assert!(value.is_finite());
    }

    #[test]
    fn soft_clip_is_bounded() {
        for input in [-4.0, -1.0, -0.5, 0.0, 0.5, 1.0, 4.0] {
            let output = soft_clip(input);
            assert!(output.is_finite());
            assert!((-1.0..=1.0).contains(&output));
        }
        assert_eq!(soft_clip(0.25), 0.25);
    }

    #[test]
    fn wsola_preserves_duration() {
        let mut wsola = Wsola::new(48_000);
        let input = sine(440.0, 48_000, 500);
        let floats: Vec<f32> = input
            .iter()
            .map(|&value| f32::from(value) / f32::from(i16::MAX))
            .collect();
        let mut output = Vec::new();
        wsola.process(&floats, 1.5, &mut output);
        assert_eq!(output.len(), floats.len());
    }

    #[test]
    fn wsola_shifts_pitch_up() {
        let sample_rate = 48_000;
        let mut processor = VoiceProcessor::new(
            sample_rate,
            VoicePresetParameters {
                pitch_ratio: 1.5,
                low_pass_hz: 20_000.0,
                ..parameters_for(VoicePreset::Normal)
            },
        )
        .expect("processor");
        let input = sine(440.0, sample_rate, 600);
        let mut output = Vec::new();
        processor.process(&input, &mut output);
        let frequency = dominant_frequency(&output, sample_rate);
        assert!(
            (frequency - 660.0).abs() < 45.0,
            "expected ~660 Hz, got {frequency}"
        );
    }

    #[test]
    fn wsola_shifts_pitch_down() {
        let sample_rate = 48_000;
        let mut processor = VoiceProcessor::new(
            sample_rate,
            VoicePresetParameters {
                pitch_ratio: 0.75,
                low_pass_hz: 20_000.0,
                ..parameters_for(VoicePreset::Normal)
            },
        )
        .expect("processor");
        let input = sine(440.0, sample_rate, 600);
        let mut output = Vec::new();
        processor.process(&input, &mut output);
        let frequency = dominant_frequency(&output, sample_rate);
        assert!(
            (frequency - 330.0).abs() < 30.0,
            "expected ~330 Hz, got {frequency}"
        );
    }

    #[test]
    fn every_preset_produces_bounded_output() {
        for preset in VoicePreset::ALL {
            let mut processor = VoiceProcessor::new(48_000, parameters_for(preset))
                .expect("processor construction");
            let input = sine(220.0, 48_000, 250);
            let mut output = Vec::new();
            processor.process(&input, &mut output);
            assert_eq!(output.len(), input.len());
            assert!(output.iter().any(|&sample| sample != 0));
        }
    }

    #[test]
    fn quiet_input_is_normalized() {
        let mut processor =
            VoiceProcessor::new(48_000, parameters_for(VoicePreset::Normal)).expect("processor");
        let input = sine(220.0, 48_000, 300);
        let quiet: Vec<i16> = input.iter().map(|sample| sample / 20).collect();
        let mut output = Vec::new();
        processor.process(&quiet, &mut output);
        assert!(processor.last_input_gain() > 5.0);
        assert!(crate::buffer::peak_i16(&output) > crate::buffer::peak_i16(&quiet) * 2.0);
    }

    #[test]
    fn loud_input_is_not_boosted() {
        let mut processor =
            VoiceProcessor::new(48_000, parameters_for(VoicePreset::Normal)).expect("processor");
        let input = sine(220.0, 48_000, 300);
        let mut output = Vec::new();
        processor.process(&input, &mut output);
        assert!((processor.last_input_gain() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_utterance_is_audible() {
        let samples = synthesize_test_utterance(48_000, 500);
        assert!(!samples.is_empty());
        assert!(crate::buffer::peak_i16(&samples) > 0.2);
    }
}
