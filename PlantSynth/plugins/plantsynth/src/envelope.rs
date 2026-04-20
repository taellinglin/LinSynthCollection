use nih_plug::prelude::Enum;

pub trait Envelope {
    fn get_value(&mut self) -> f32;
    fn trigger(&mut self);
    fn release(&mut self);
    fn set_envelope_stage(&mut self, stage: ADSREnvelopeState);
    fn get_envelope_stage(&self) -> ADSREnvelopeState;
    fn set_scale(&mut self, envelope_levels: f32);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ADSREnvelope {
    attack: f32,
    hold: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    state: ADSREnvelopeState,
    time: f32,
    delta_time_per_sample: f32,
    sample_rate: f32,
    velocity: f32,
    is_sustained: bool,
    scale: f32,
    tension: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Enum)]
pub enum ADSREnvelopeState {
    Idle,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

impl ADSREnvelope {
    pub fn new(
        attack: f32,
        hold: f32,
        decay: f32,
        sustain: f32,
        release: f32,
        sample_rate: f32,
        velocity: f32,
        tension: f32,
    ) -> Self {
        ADSREnvelope {
            attack,
            hold,
            decay,
            sustain,
            release,
            state: ADSREnvelopeState::Attack,
            time: 0.0,
            sample_rate,
            delta_time_per_sample: 1.0 / sample_rate,
            velocity,
            is_sustained: false,
            scale: 1.0,
            tension: tension.clamp(-1.0, 1.0),
        }
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
        // Velocity is already stored and can be used for velocity-sensitive scaling
        // Don't modify the envelope time parameters - they should remain constant
    }

    pub fn get_time(&mut self) -> f32 {
        self.time
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack;
    }

    pub fn set_decay(&mut self, decay: f32) {
        self.decay = decay;
    }

    pub fn set_sustain(&mut self, sustain: f32) {
        self.sustain = sustain;
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release;
    }

    pub fn get_state(&self) -> ADSREnvelopeState {
        self.state
    }

    pub fn previous_value(&self) -> f32 {
        match self.state {
            ADSREnvelopeState::Idle => 0.0,
            ADSREnvelopeState::Attack => self.time / self.attack,
            ADSREnvelopeState::Hold => self.sustain,
            ADSREnvelopeState::Decay => 1.0 - (1.0 - self.sustain) * (self.time / self.decay),
            ADSREnvelopeState::Sustain => self.sustain,
            ADSREnvelopeState::Release => self.sustain * (1.0 - (self.time / self.release)),
        }
    }

    pub fn advance(&mut self) {
        self.time += self.delta_time_per_sample;

        // Note: sustain is a level, not a time duration
        // The sustain stage continues indefinitely until note is released
        match self.state {
            ADSREnvelopeState::Attack if self.time >= self.attack => {
                self.state = ADSREnvelopeState::Hold;
                self.time = 0.0;
            }
            ADSREnvelopeState::Hold if self.time >= self.hold => {
                self.state = ADSREnvelopeState::Decay;
                self.time = 0.0;
            }
            ADSREnvelopeState::Decay if self.time >= self.decay => {
                self.state = ADSREnvelopeState::Sustain;
                self.time = 0.0;
            }
            // Sustain stage stays until note released (set_envelope_stage called)
            ADSREnvelopeState::Sustain => {
                // Remain in sustain until explicitly released
            }
            ADSREnvelopeState::Release if self.time >= self.release => {
                self.state = ADSREnvelopeState::Idle;
                self.time = 0.0;
            }
            _ => {}
        }
    }

    pub fn get_attack(&self) -> f32 {
        self.attack
    }

    pub fn get_decay(&self) -> f32 {
        self.decay
    }

    pub fn get_sustain(&self) -> f32 {
        self.sustain
    }

    pub fn get_release(&self) -> f32 {
        self.release
    }

    pub fn get_envelope_stage(&self) -> ADSREnvelopeState {
        self.state
    }

    // Setter for envelope stage
    pub fn set_envelope_stage(&mut self, stage: ADSREnvelopeState) {
        self.state = stage;
    }
    pub fn set_scale(&mut self, envelope_levels: f32) {
        self.scale = envelope_levels;
        // Scale is applied in get_value(), not to the time parameters
    }
    pub fn set_hold(&mut self, hold: f32) {
        self.hold = hold;
    }

    pub fn next_sample(&mut self) -> f32 {
        let val = self.get_value();
        self.advance();
        val
    }

    pub fn set_tension(&mut self, tension: f32) {
        self.tension = tension.clamp(-1.0, 1.0);
    }

    pub fn get_tension(&self) -> f32 {
        self.tension
    }

    fn apply_tension(&self, linear_value: f32) -> f32 {
        if self.tension == 0.0 {
            return linear_value;
        }
        
        // Tension affects the curve:
        // tension > 0: exponential (fast start, slow end)
        // tension < 0: logarithmic (slow start, fast end)
        // tension = 0: linear
        if self.tension > 0.0 {
            // Exponential curve
            linear_value.powf(1.0 + self.tension * 3.0)
        } else {
            // Logarithmic curve
            1.0 - (1.0 - linear_value).powf(1.0 - self.tension * 3.0)
        }
    }
}

impl Envelope for ADSREnvelope {
    fn get_value(&mut self) -> f32 {
        let base_value = match self.state {
            ADSREnvelopeState::Idle => 0.0,
            ADSREnvelopeState::Attack => {
                if self.attack <= 0.0 {
                    1.0 // Instant attack
                } else {
                    let linear = (self.time / self.attack).min(1.0);
                    self.apply_tension(linear)
                }
            }
            ADSREnvelopeState::Hold => {
                1.0 // Hold at peak
            }
            ADSREnvelopeState::Decay => {
                if self.decay <= 0.0 {
                    self.sustain // Instant decay
                } else {
                    let linear = (self.time / self.decay).min(1.0);
                    let curved = self.apply_tension(linear);
                    1.0 - (1.0 - self.sustain) * curved
                }
            }
            ADSREnvelopeState::Sustain => {
                self.sustain // Hold at sustain level
            }
            ADSREnvelopeState::Release => {
                if self.release <= 0.0 {
                    0.0 // Instant release
                } else {
                    let linear = (self.time / self.release).min(1.0);
                    let curved = self.apply_tension(linear);
                    self.sustain * (1.0 - curved)
                }
            }
        };
        
        // Apply scale factor (for envelope level control)
        base_value * self.scale
    }

    fn trigger(&mut self) {
        self.state = ADSREnvelopeState::Attack;
        self.time = 0.0;
        self.is_sustained = false;
    }

    fn release(&mut self) {
        self.state = ADSREnvelopeState::Release;
        self.time = 0.0;
        self.is_sustained = false;
    }

    fn get_envelope_stage(&self) -> ADSREnvelopeState {
        self.state
    }

    fn set_envelope_stage(&mut self, stage: ADSREnvelopeState) {
        self.state = stage;
    }
    fn set_scale(&mut self, envelope_levels: f32) {
        self.scale = envelope_levels;
    }
}