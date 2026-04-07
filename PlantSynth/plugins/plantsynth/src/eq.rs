use std::f32::consts::PI;

#[derive(Clone, Copy, Debug)]
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
    pub fn new() -> Self {
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

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }

    fn set_coeffs(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        let inv_a0 = 1.0 / a0.max(1.0e-12);
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    pub fn set_lowpass(&mut self, sample_rate: f32, freq: f32, q: f32) {
        let w0 = 2.0 * PI * freq / sample_rate.max(1.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q.max(0.001));

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_highpass(&mut self, sample_rate: f32, freq: f32, q: f32) {
        let w0 = 2.0 * PI * freq / sample_rate.max(1.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q.max(0.001));

        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;
        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_peaking(&mut self, sample_rate: f32, freq: f32, q: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate.max(1.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q.max(0.001));

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_low_shelf(&mut self, sample_rate: f32, freq: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate.max(1.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let sqrt_a = a.sqrt();
        let alpha = sin_w0 / 2.0 * (2.0_f32).sqrt();

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }

    pub fn set_high_shelf(&mut self, sample_rate: f32, freq: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate.max(1.0);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let sqrt_a = a.sqrt();
        let alpha = sin_w0 / 2.0 * (2.0_f32).sqrt();

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
        self.set_coeffs(b0, b1, b2, a0, a1, a2);
    }
}

pub struct ThreeBandEq {
    sample_rate: f32,
    low: [Biquad; 2],
    mid: [Biquad; 2],
    high: [Biquad; 2],
}

impl ThreeBandEq {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            low: [Biquad::new(), Biquad::new()],
            mid: [Biquad::new(), Biquad::new()],
            high: [Biquad::new(), Biquad::new()],
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn set_params(&mut self, low_gain_db: f32, mid_gain_db: f32, mid_freq: f32, mid_q: f32, high_gain_db: f32) {
        let low_freq = 120.0;
        let high_freq = 4200.0;
        for channel in 0..2 {
            self.low[channel].set_low_shelf(self.sample_rate, low_freq, low_gain_db);
            self.mid[channel].set_peaking(self.sample_rate, mid_freq, mid_q, mid_gain_db);
            self.high[channel].set_high_shelf(self.sample_rate, high_freq, high_gain_db);
        }
    }

    pub fn process_sample(&mut self, channel: usize, input: f32) -> f32 {
        let mut sample = self.low[channel].process(input);
        sample = self.mid[channel].process(sample);
        self.high[channel].process(sample)
    }
}
