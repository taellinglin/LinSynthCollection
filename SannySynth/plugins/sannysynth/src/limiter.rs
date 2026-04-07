// Basic Limiter implementation for SannySynth
pub struct Limiter {
    pub enabled: bool,
    pub threshold: f32,
    pub release_ms: f32,
    gain: f32,
}

impl Limiter {
    pub fn new() -> Self {
        Self {
            enabled: false,
            threshold: 0.95,
            release_ms: 50.0,
            gain: 1.0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        if !self.enabled {
            return input;
        }
        let abs_in = input.abs();
        if abs_in > self.threshold {
            self.gain *= 0.9;
        } else {
            self.gain += (1.0 - self.gain) * (1.0 - (-1.0 / (self.release_ms * 0.001)).exp());
        }
        self.gain = self.gain.clamp(0.0, 1.0);
        input * self.gain
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold;
    }
    pub fn set_release(&mut self, release_ms: f32) {
        self.release_ms = release_ms;
    }
}
