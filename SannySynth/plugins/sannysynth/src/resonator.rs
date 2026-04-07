use std::f32::consts::PI;
use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;

const NUM_BANDS: usize = 12;
const MIN_FREQ: f32 = 20.0;
const MAX_FREQ_FRACTION: f32 = 0.49;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum ResonatorTimbre {
    Balanced,
    Bright,
    Warm,
    Sitar,
    Metallic,
    Hollow,
}

#[derive(Debug, Clone)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn set_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) {
        self.b0 = b0;
        self.b1 = b1;
        self.b2 = b2;
        self.a1 = a1;
        self.a2 = a2;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input + self.z2 - self.a1 * output;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

#[derive(Debug, Clone)]
struct ResonatorBand {
    freq: f32,
    q: f32,
    sample_rate: f32,
    biquad: Biquad,
    dirty: bool,
}

impl ResonatorBand {
    fn new(freq: f32, q: f32, sample_rate: f32) -> Self {
        Self {
            freq,
            q,
            sample_rate,
            biquad: Biquad::new(),
            dirty: true,
        }
    }

    fn set_freq(&mut self, freq: f32) {
        if (freq - self.freq).abs() > 0.01 {
            self.freq = freq;
            self.dirty = true;
        }
    }

    fn set_q(&mut self, q: f32) {
        if (q - self.q).abs() > 0.01 {
            self.q = q;
            self.dirty = true;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        if (sample_rate - self.sample_rate).abs() > f32::EPSILON {
            self.sample_rate = sample_rate;
            self.dirty = true;
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            self.update_coeffs();
        }
        self.biquad.process(input)
    }

    fn update_coeffs(&mut self) {
        let cutoff = clamp_freq(self.freq, self.sample_rate);
        let w0 = 2.0 * PI * cutoff / self.sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * self.q.max(0.1));

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        let inv_a0 = 1.0 / a0.max(0.0001);
        self.biquad
            .set_coefficients(b0 * inv_a0, b1 * inv_a0, b2 * inv_a0, a1 * inv_a0, a2 * inv_a0);
        self.dirty = false;
    }
}

fn clamp_freq(freq: f32, sample_rate: f32) -> f32 {
    freq.clamp(MIN_FREQ, sample_rate * MAX_FREQ_FRACTION)
}

#[derive(Debug, Clone)]
pub struct ResonatorBank {
    sample_rate: f32,
    base_freq: f32,
    tone: f32,
    shape: f32,
    timbre: ResonatorTimbre,
    damping: f32,
    bands: Vec<ResonatorBand>,
}

impl ResonatorBank {
    pub fn new(sample_rate: f32, base_freq: f32) -> Self {
        let mut bank = Self {
            sample_rate,
            base_freq,
            tone: 0.5,
            shape: 0.5,
            timbre: ResonatorTimbre::Balanced,
            damping: 0.0,
            bands: Vec::with_capacity(NUM_BANDS),
        };
        bank.rebuild_bands();
        bank
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (sample_rate - self.sample_rate).abs() < f32::EPSILON {
            return;
        }
        self.sample_rate = sample_rate;
        for band in &mut self.bands {
            band.set_sample_rate(sample_rate);
        }
        self.update_band_freqs();
    }

    pub fn set_base_freq(&mut self, base_freq: f32) {
        if (base_freq - self.base_freq).abs() < 0.01 {
            return;
        }
        self.base_freq = base_freq.max(MIN_FREQ);
        self.update_band_freqs();
    }

    pub fn set_tone(&mut self, tone: f32) {
        let tone = tone.clamp(0.0, 1.0);
        if (tone - self.tone).abs() < 0.001 {
            return;
        }
        self.tone = tone;
        let q = q_for(self.tone, self.damping);
        for band in &mut self.bands {
            band.set_q(q);
        }
    }

    pub fn set_shape(&mut self, shape: f32) {
        self.shape = shape.clamp(0.0, 1.0);
    }

    pub fn set_timbre(&mut self, timbre: ResonatorTimbre) {
        self.timbre = timbre;
    }

    pub fn set_damping(&mut self, damping: f32) {
        let damping = damping.clamp(0.0, 1.0);
        if (damping - self.damping).abs() < 0.001 {
            return;
        }
        self.damping = damping;
        let q = q_for(self.tone, self.damping);
        for band in &mut self.bands {
            band.set_q(q);
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        if self.bands.is_empty() {
            return input;
        }

        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        for (index, band) in self.bands.iter_mut().enumerate() {
            let harmonic = (index + 1) as f32;
            let weight = band_weight(harmonic, self.shape, self.timbre);
            sum += band.process(input) * weight;
            weight_sum += weight.abs();
        }

        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            sum
        }
    }

    fn rebuild_bands(&mut self) {
        let q = q_for(self.tone, self.damping);
        self.bands.clear();
        for band_idx in 0..NUM_BANDS {
            let harmonic = (band_idx + 1) as f32;
            let freq = clamp_freq(self.base_freq * harmonic, self.sample_rate);
            self.bands.push(ResonatorBand::new(freq, q, self.sample_rate));
        }
    }

    fn update_band_freqs(&mut self) {
        for (band_idx, band) in self.bands.iter_mut().enumerate() {
            let harmonic = (band_idx + 1) as f32;
            let freq = clamp_freq(self.base_freq * harmonic, self.sample_rate);
            band.set_freq(freq);
        }
    }
}

fn band_weight(harmonic: f32, shape: f32, timbre: ResonatorTimbre) -> f32 {
    let tilt = (shape * 2.0) - 1.0;
    let decay = harmonic.powf(-tilt);
    let base = match timbre {
        ResonatorTimbre::Balanced => decay,
        ResonatorTimbre::Bright => decay * (1.0 + (harmonic / 6.0).min(2.0)),
        ResonatorTimbre::Warm => decay * (1.0 / (1.0 + (harmonic / 3.5))),
        ResonatorTimbre::Sitar => {
            let odd_boost = if (harmonic as i32) % 2 == 1 { 1.2 } else { 0.55 };
            let high_tilt = 1.0 + (harmonic / 5.0).min(1.5);
            decay * odd_boost * high_tilt
        }
        ResonatorTimbre::Metallic => decay * (1.0 + (harmonic / 2.5).min(3.0)),
        ResonatorTimbre::Hollow => decay * if harmonic <= 3.0 { 1.4 } else { 0.6 },
    };

    base.clamp(0.2, 6.0)
}

fn q_for(tone: f32, damping: f32) -> f32 {
    let tone = tone.clamp(0.0, 1.0);
    let damping = damping.clamp(0.0, 1.0);
    let base_q = 2.0 + tone * 18.0;
    let damp_scale = 1.0 - damping * 0.65;
    (base_q * damp_scale).max(0.6)
}
