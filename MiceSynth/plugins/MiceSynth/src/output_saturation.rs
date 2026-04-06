use crate::eq::Biquad;
use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Sequence)]
pub enum OutputSaturationType {
    Tape,
    Tube,
    Transformer,
}

pub struct OutputSaturation {
    sample_rate: f32,
    tone: [Biquad; 2],
}

impl OutputSaturation {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            tone: [Biquad::new(), Biquad::new()],
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn process_sample(
        &mut self,
        channel: usize,
        input: f32,
        drive: f32,
        mode: OutputSaturationType,
        mix: f32,
    ) -> f32 {
        let drive = (1.0 + drive.clamp(0.0, 1.0) * 8.0).max(1.0);
        let mix = mix.clamp(0.0, 1.0);

        let shaped = match mode {
            OutputSaturationType::Tape => {
                let pushed = input * drive;
                let soft = (pushed * 0.9).tanh();
                self.tone[channel].set_lowpass(self.sample_rate, 12000.0, 0.7);
                self.tone[channel].process(soft)
            }
            OutputSaturationType::Tube => {
                let pushed = input * drive;
                let asym = pushed + 0.2 * pushed * pushed * pushed;
                (asym * 0.8).tanh()
            }
            OutputSaturationType::Transformer => {
                let pushed = input * drive;
                let soft = pushed - 0.15 * pushed * pushed * pushed;
                (soft * 0.9).tanh()
            }
        };

        input * (1.0 - mix) + shaped * mix
    }
}
