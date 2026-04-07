use std::f32::consts::PI;

use crate::eq::Biquad;

pub struct Distortion {
    sample_rate: f32,
    pre_emphasis: [Biquad; 2],
    post_tone: [Biquad; 2],
    sizzle_lp: [Biquad; 2],
    sizzle_enabled: bool,
    sizzle_cutoff: f32,
}

impl Distortion {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            pre_emphasis: [Biquad::new(), Biquad::new()],
            post_tone: [Biquad::new(), Biquad::new()],
            sizzle_lp: [Biquad::new(), Biquad::new()],
            sizzle_enabled: false,
            sizzle_cutoff: 12000.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        if self.sizzle_enabled {
            let cutoff = self.sizzle_cutoff.min(self.sample_rate * 0.45);
            for channel in 0..2 {
                self.sizzle_lp[channel].set_lowpass(self.sample_rate, cutoff, 0.7);
            }
        }
    }

    pub fn set_sizzle_guard(&mut self, enabled: bool, cutoff: f32) {
        self.sizzle_enabled = enabled;
        self.sizzle_cutoff = cutoff.max(1000.0).min(self.sample_rate * 0.45);
        if self.sizzle_enabled {
            for channel in 0..2 {
                self.sizzle_lp[channel].set_lowpass(self.sample_rate, self.sizzle_cutoff, 0.7);
            }
        }
    }

    pub fn set_tone(&mut self, tone: f32) {
        let tone = tone.clamp(0.0, 1.0);
        let pre_gain_db = -3.0 + tone * 9.0;
        let post_cutoff = 2000.0 + tone * 10000.0;

        for channel in 0..2 {
            self.pre_emphasis[channel].set_high_shelf(self.sample_rate, 2500.0, pre_gain_db);
            self.post_tone[channel].set_lowpass(self.sample_rate, post_cutoff, 0.7);
        }
    }

    pub fn process_sample(
        &mut self,
        channel: usize,
        input: f32,
        drive: f32,
        magic: f32,
        mix: f32,
    ) -> f32 {
        let drive_amount = drive.clamp(0.0, 1.0);
        let drive = 1.0 + drive_amount * 8.0;
        let post_gain = 1.0 / (1.0 + drive_amount * 2.5);
        let magic = magic.clamp(0.0, 1.0);
        let mix = mix.clamp(0.0, 1.0);

        let pre = self.pre_emphasis[channel].process(input);
        let pushed = pre * drive;
        let soft = pushed.tanh();
        let shimmer = (soft * PI).sin();
        let mut shaped = soft + magic * 0.35 * (shimmer - soft);
        shaped += magic * 0.15 * shaped * shaped * shaped;
        let mut post = self.post_tone[channel].process(shaped) * post_gain;
        if self.sizzle_enabled {
            post = self.sizzle_lp[channel].process(post);
        }

        input * (1.0 - mix) + post * mix
    }
}
