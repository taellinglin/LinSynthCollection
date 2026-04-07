const MAX_DELAY_SECONDS: f32 = 4.0;

pub struct StereoDelay {
    sample_rate: f32,
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    write_index: usize,
}

impl StereoDelay {
    pub fn new(sample_rate: f32) -> Self {
        let max_samples = (sample_rate * MAX_DELAY_SECONDS).ceil() as usize + 1;
        Self {
            sample_rate,
            buffer_left: vec![0.0; max_samples],
            buffer_right: vec![0.0; max_samples],
            write_index: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() < f32::EPSILON {
            return;
        }

        self.sample_rate = sample_rate;
        let max_samples = (sample_rate * MAX_DELAY_SECONDS).ceil() as usize + 1;
        self.buffer_left.resize(max_samples, 0.0);
        self.buffer_right.resize(max_samples, 0.0);
        self.write_index %= max_samples.max(1);
    }

    pub fn process(
        &mut self,
        left: f32,
        right: f32,
        time_ms: f32,
        feedback: f32,
        mix: f32,
    ) -> (f32, f32) {
        let buffer_len = self.buffer_left.len().min(self.buffer_right.len());
        if buffer_len == 0 {
            return (left, right);
        }
        if self.write_index >= buffer_len {
            self.write_index %= buffer_len;
        }

        let time_ms = time_ms.max(1.0).min(MAX_DELAY_SECONDS * 1000.0);
        let delay_samples = (time_ms / 1000.0) * self.sample_rate;
        let feedback = feedback.clamp(0.0, 0.95);
        let mix = mix.clamp(0.0, 1.0);

        let delayed_left = self.read_delay(&self.buffer_left, delay_samples);
        let delayed_right = self.read_delay(&self.buffer_right, delay_samples);

        let write_left = left + delayed_left * feedback;
        let write_right = right + delayed_right * feedback;

        let write_index = self.write_index;
        if let Some(slot) = self.buffer_left.get_mut(write_index) {
            *slot = write_left;
        }
        if let Some(slot) = self.buffer_right.get_mut(write_index) {
            *slot = write_right;
        }

        self.write_index = (write_index + 1) % buffer_len.max(1);

        let wet_left = left * (1.0 - mix) + delayed_left * mix;
        let wet_right = right * (1.0 - mix) + delayed_right * mix;

        (wet_left, wet_right)
    }

    fn read_delay(&self, buffer: &[f32], delay_samples: f32) -> f32 {
        let len = buffer.len() as f32;
        if len <= 1.0 {
            return 0.0;
        }

        let read_pos = (self.write_index as f32 - delay_samples).rem_euclid(len);
        let idx_a = read_pos.floor() as usize;
        let idx_b = (idx_a + 1) % buffer.len();
        let frac = read_pos - idx_a as f32;

        let a = buffer[idx_a];
        let b = buffer[idx_b];

        a + (b - a) * frac
    }
}
