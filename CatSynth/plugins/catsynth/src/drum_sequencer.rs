use crate::drum_params::DRUM_STEPS;

pub struct DrumSequencer {
    step_index: usize,
    phase: f32,
}

impl DrumSequencer {
    pub fn new() -> Self {
        Self {
            step_index: 0,
            phase: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.step_index = 0;
        self.phase = 0.0;
    }

    pub fn advance(
        &mut self,
        sample_rate: f32,
        bpm: f32,
        steps_per_beat: f32,
        swing: f32,
    ) -> Option<usize> {
        let beats_per_second = bpm.max(1.0) / 60.0;
        let base_steps_per_second = beats_per_second * steps_per_beat.max(0.01);
        let swing = swing.clamp(0.0, 0.75);
        let step_divisor = if self.step_index % 2 == 0 {
            1.0 + swing
        } else {
            1.0 - swing
        };
        let steps_per_second = base_steps_per_second / step_divisor.max(0.01);
        let step_inc = steps_per_second / sample_rate.max(1.0);
        self.phase += step_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            let step = self.step_index;
            self.step_index = (self.step_index + 1) % DRUM_STEPS;
            return Some(step);
        }
        None
    }
}
