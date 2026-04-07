use std::f32::consts::PI;

const MAX_DELAY_SAMPLES: usize = 8192;

#[derive(Debug, Clone)]
pub struct Chorus {
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    write_index: usize,
    lfo_phase: f32,
    sample_rate: f32,
    enabled: bool,
}

impl Chorus {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            buffer_left: vec![0.0; MAX_DELAY_SAMPLES],
            buffer_right: vec![0.0; MAX_DELAY_SAMPLES],
            write_index: 0,
            lfo_phase: 0.0,
            sample_rate,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn reset(&mut self) {
        self.buffer_left.fill(0.0);
        self.buffer_right.fill(0.0);
        self.write_index = 0;
        self.lfo_phase = 0.0;
    }

    pub fn process(
        &mut self,
        input_left: f32,
        input_right: f32,
        rate_hz: f32,
        depth_ms: f32,
        mix: f32,
    ) -> (f32, f32) {
        if !self.enabled {
            return (input_left, input_right);
        }

        // Write input to delay buffer
        self.buffer_left[self.write_index] = input_left;
        self.buffer_right[self.write_index] = input_right;

        // Calculate LFO modulation
        let lfo = (self.lfo_phase * 2.0 * PI).sin();
        
        // Convert depth from milliseconds to samples
        let depth_samples = (depth_ms / 1000.0) * self.sample_rate;
        
        // Calculate base delay (center delay time in samples)
        let base_delay_samples = depth_samples;
        
        // Modulate delay time with LFO
        let modulated_delay = base_delay_samples + (lfo * depth_samples * 0.5);
        let delay_samples = modulated_delay.max(1.0).min((MAX_DELAY_SAMPLES - 1) as f32);
        
        // Calculate read position with fractional delay (linear interpolation)
        let read_pos = (self.write_index as f32 - delay_samples + MAX_DELAY_SAMPLES as f32) % MAX_DELAY_SAMPLES as f32;
        let read_index_1 = read_pos.floor() as usize;
        let read_index_2 = (read_index_1 + 1) % MAX_DELAY_SAMPLES;
        let frac = read_pos - read_pos.floor();
        
        // Linear interpolation for smooth delay
        let delayed_left = self.buffer_left[read_index_1] * (1.0 - frac) 
                         + self.buffer_left[read_index_2] * frac;
        let delayed_right = self.buffer_right[read_index_1] * (1.0 - frac) 
                          + self.buffer_right[read_index_2] * frac;
        
        // Mix dry and wet signals
        let output_left = input_left * (1.0 - mix) + delayed_left * mix;
        let output_right = input_right * (1.0 - mix) + delayed_right * mix;
        
        // Advance write index
        self.write_index = (self.write_index + 1) % MAX_DELAY_SAMPLES;
        
        // Advance LFO phase
        self.lfo_phase += rate_hz / self.sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        
        (output_left, output_right)
    }
}
