use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;
use std::f32::consts::PI;

use crate::envelope::*;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum FilterType {
    None,
    Lowpass,
    Bandpass,
    Highpass,
    Notch,
    Statevariable,
    Comb,
    #[name = "Rainbow Comb"]
    RainbowComb,
    #[name = "Diode Ladder LP"]
    DiodeLadderLp,
    #[name = "Diode Ladder HP"]
    DiodeLadderHp,
    #[name = "MS-20 HP/LP"]
    Ms20Pair,
    #[name = "Formant Morph"]
    FormantMorph,
    Phaser,
    #[name = "Comb Allpass"]
    CombAllpass,
    #[name = "Bitcrush LP"]
    BitcrushLp,
}

pub trait Filter: Send {
    fn process(&mut self, input: f32) -> f32;
    fn set_sample_rate(&mut self, sample_rate: f32);
}

#[derive(Debug, Clone)]
pub struct HighpassFilter {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
    prev_input: f32,
    prev_output: f32,
}

impl HighpassFilter {
    pub fn new(
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> Self {
        HighpassFilter {
            cutoff,
            resonance,
            sample_rate,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance;
    }
}

impl Filter for HighpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        let cutoff = self.cutoff.max(20.0).min(20000.0);
        let resonance = self.resonance.max(0.0).min(0.99);
        
        // Calculate filter coefficient
        let omega = 2.0 * std::f32::consts::PI * cutoff / self.sample_rate;
        let alpha = omega / (omega + 1.0);
        
        // Highpass = input - lowpass
        let lowpass = self.prev_output + alpha * (input - self.prev_output);
        let highpass = input - lowpass;
        
        // Apply resonance as feedback
        self.prev_output = lowpass + highpass * resonance;
        
        highpass
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}

#[derive(Debug, Clone)]
pub struct BandpassFilter {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
    prev_input: f32,
    prev_output: f32,
}

impl BandpassFilter {
    pub fn new(
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> Self {
        BandpassFilter {
            cutoff,
            resonance,
            sample_rate,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance;
    }
}
impl Filter for BandpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        let cutoff = self.cutoff.max(20.0).min(20000.0);
        let resonance = self.resonance.max(0.01).min(0.99);
        
        // Calculate filter coefficient
        let omega = 2.0 * std::f32::consts::PI * cutoff / self.sample_rate;
        let alpha = omega / (omega + 1.0);
        
        // Bandpass filter using state variable approach
        let lowpass = self.prev_output + alpha * (input - self.prev_output);
        let highpass = input - lowpass;
        let bandpass = lowpass * (1.0 - resonance) + highpass * resonance;
        
        self.prev_output = lowpass;
        self.prev_input = input;
        
        bandpass
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}

#[derive(Debug, Clone)]
pub struct LowpassFilter {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
    prev_output: f32,
}

#[derive(Debug, Clone)]
pub struct CombFilter {
    sample_rate: f32,
    buffer: Vec<f32>,
    write_idx: usize,
    delay_samples: usize,
    feedback: f32,
    damp: f32,
    lp_state: f32,
}

impl CombFilter {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay = (sample_rate.max(1.0) * 0.2) as usize;
        Self {
            sample_rate: sample_rate.max(1.0),
            buffer: vec![0.0; max_delay.max(1)],
            write_idx: 0,
            delay_samples: (sample_rate / 220.0).max(1.0) as usize,
            feedback: 0.6,
            damp: 0.2,
            lp_state: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, freq: f32) {
        let freq = freq.max(40.0).min(4000.0);
        let delay = (self.sample_rate / freq).round() as usize;
        self.delay_samples = delay.clamp(1, self.buffer.len().saturating_sub(1).max(1));
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        let r = resonance.clamp(0.0, 1.0);
        self.feedback = 0.2 + r * 0.75;
        self.damp = 0.05 + (1.0 - r) * 0.35;
    }
}

impl Filter for CombFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.buffer.is_empty() {
            return input;
        }

        let len = self.buffer.len();
        let read_idx = (self.write_idx + len - self.delay_samples) % len;
        let delayed = self.buffer[read_idx];
        self.lp_state += self.damp * (delayed - self.lp_state);
        let filtered = self.lp_state;
        let output = input + filtered;

        self.buffer[self.write_idx] = input + filtered * self.feedback;
        self.write_idx = (self.write_idx + 1) % len;
        output
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        let max_delay = (self.sample_rate * 0.2) as usize;
        self.buffer.resize(max_delay.max(1), 0.0);
        self.write_idx %= self.buffer.len();
        self.delay_samples = self.delay_samples.clamp(1, self.buffer.len().saturating_sub(1).max(1));
    }
}

#[derive(Debug, Clone, Copy)]
enum LadderMode {
    Lowpass,
    Highpass,
}

#[derive(Debug, Clone)]
pub struct DiodeLadderFilter {
    sample_rate: f32,
    cutoff: f32,
    resonance: f32,
    stage: [f32; 4],
    mode: LadderMode,
}

impl DiodeLadderFilter {
    pub fn new(sample_rate: f32, mode: LadderMode) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            cutoff: 1000.0,
            resonance: 0.3,
            stage: [0.0; 4],
            mode,
        }
    }

    pub fn new_lowpass(sample_rate: f32) -> Self {
        Self::new(sample_rate, LadderMode::Lowpass)
    }

    pub fn new_highpass(sample_rate: f32) -> Self {
        Self::new(sample_rate, LadderMode::Highpass)
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.max(20.0).min(self.sample_rate * 0.45);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
    }
}

impl Filter for DiodeLadderFilter {
    fn process(&mut self, input: f32) -> f32 {
        let f = (self.cutoff / self.sample_rate).clamp(0.0, 0.45);
        let fb = self.resonance * 4.0 * (1.0 - 0.2 * f * f);
        let mut x = (input - fb * self.stage[3]).tanh();

        for stage in &mut self.stage {
            *stage += f * (x - *stage);
            x = stage.tanh();
        }

        let low = self.stage[3];
        match self.mode {
            LadderMode::Lowpass => low,
            LadderMode::Highpass => input - low,
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }
}

#[derive(Debug, Clone)]
pub struct Ms20Filter {
    sample_rate: f32,
    cutoff: f32,
    resonance: f32,
    hp_state: f32,
    hp_prev_in: f32,
    lp_state: f32,
}

impl Ms20Filter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            cutoff: 1000.0,
            resonance: 0.3,
            hp_state: 0.0,
            hp_prev_in: 0.0,
            lp_state: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.max(30.0).min(self.sample_rate * 0.45);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
    }
}

impl Filter for Ms20Filter {
    fn process(&mut self, input: f32) -> f32 {
        let g = (self.cutoff / self.sample_rate).clamp(0.0, 0.45);
        let fb = self.resonance * 0.8;
        let in_fb = input - fb * self.lp_state;

        let hp = g * (self.hp_state + in_fb - self.hp_prev_in);
        self.hp_prev_in = in_fb;
        self.hp_state = hp;

        let hp_sat = (hp * (1.0 + self.resonance * 1.5)).tanh();
        self.lp_state += g * (hp_sat - self.lp_state);
        self.lp_state
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }
}

#[derive(Debug, Clone)]
struct AllpassStage {
    a: f32,
    x1: f32,
    y1: f32,
}

impl AllpassStage {
    fn new() -> Self {
        Self { a: 0.0, x1: 0.0, y1: 0.0 }
    }

    fn set_coeff(&mut self, a: f32) {
        self.a = a;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = -self.a * input + self.x1 + self.a * self.y1;
        self.x1 = input;
        self.y1 = output;
        output
    }
}

#[derive(Debug, Clone)]
pub struct PhaserFilter {
    sample_rate: f32,
    cutoff: f32,
    resonance: f32,
    stages: [AllpassStage; 6],
    feedback: f32,
}

impl PhaserFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            cutoff: 800.0,
            resonance: 0.2,
            stages: [AllpassStage::new(), AllpassStage::new(), AllpassStage::new(), AllpassStage::new(), AllpassStage::new(), AllpassStage::new()],
            feedback: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.max(20.0).min(self.sample_rate * 0.45);
        let g = (PI * self.cutoff / self.sample_rate).tan();
        let a = (1.0 - g) / (1.0 + g).max(1.0e-6);
        for stage in &mut self.stages {
            stage.set_coeff(a);
        }
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
        self.feedback = self.resonance * 0.7;
    }
}

impl Filter for PhaserFilter {
    fn process(&mut self, input: f32) -> f32 {
        let mut x = input + self.feedback;
        for stage in &mut self.stages {
            x = stage.process(x);
        }
        self.feedback = x * (self.resonance * 0.7);
        x
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.set_cutoff(self.cutoff);
    }
}

#[derive(Debug, Clone)]
pub struct CombAllpassFilter {
    sample_rate: f32,
    delay_samples: usize,
    feedback: f32,
    write_idx: usize,
    in_buffer: Vec<f32>,
    out_buffer: Vec<f32>,
}

impl CombAllpassFilter {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.max(1.0);
        let max_delay = (sample_rate * 0.2) as usize;
        Self {
            sample_rate,
            delay_samples: (sample_rate / 220.0).max(1.0) as usize,
            feedback: 0.6,
            write_idx: 0,
            in_buffer: vec![0.0; max_delay.max(1)],
            out_buffer: vec![0.0; max_delay.max(1)],
        }
    }

    pub fn set_cutoff(&mut self, freq: f32) {
        let freq = freq.max(40.0).min(4000.0);
        let delay = (self.sample_rate / freq).round() as usize;
        let max_delay = self.in_buffer.len().saturating_sub(1).max(1);
        self.delay_samples = delay.clamp(1, max_delay);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        let r = resonance.clamp(0.0, 1.0);
        self.feedback = 0.1 + r * 0.85;
    }
}

impl Filter for CombAllpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.in_buffer.is_empty() {
            return input;
        }

        let len = self.in_buffer.len();
        let read_idx = (self.write_idx + len - self.delay_samples) % len;
        let x_del = self.in_buffer[read_idx];
        let y_del = self.out_buffer[read_idx];
        let y = -self.feedback * input + x_del + self.feedback * y_del;

        self.in_buffer[self.write_idx] = input;
        self.out_buffer[self.write_idx] = y;
        self.write_idx = (self.write_idx + 1) % len;
        y
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        let max_delay = (self.sample_rate * 0.2) as usize;
        self.in_buffer.resize(max_delay.max(1), 0.0);
        self.out_buffer.resize(max_delay.max(1), 0.0);
        self.write_idx %= self.in_buffer.len();
        self.delay_samples = self.delay_samples.clamp(1, self.in_buffer.len().saturating_sub(1).max(1));
    }
}

#[derive(Debug, Clone)]
pub struct FormantMorphFilter {
    sample_rate: f32,
    cutoff: f32,
    resonance: f32,
    formants: [BandpassFilter; 3],
}

impl FormantMorphFilter {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.max(1.0);
        Self {
            sample_rate,
            cutoff: 800.0,
            resonance: 0.4,
            formants: [
                BandpassFilter::new(700.0, 0.6, sample_rate),
                BandpassFilter::new(1200.0, 0.6, sample_rate),
                BandpassFilter::new(2600.0, 0.6, sample_rate),
            ],
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.max(80.0).min(self.sample_rate * 0.45);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
    }

    fn current_formants(&self) -> (f32, f32, f32) {
        const VOWELS: [[f32; 3]; 5] = [
            [800.0, 1150.0, 2900.0],
            [450.0, 1700.0, 2600.0],
            [350.0, 2000.0, 3000.0],
            [500.0, 900.0, 2400.0],
            [325.0, 700.0, 2500.0],
        ];
        let pos = (self.resonance * 4.0).clamp(0.0, 4.0);
        let idx = pos.floor() as usize;
        let frac = pos - idx as f32;
        let a = VOWELS[idx % VOWELS.len()];
        let b = VOWELS[(idx + 1) % VOWELS.len()];
        (
            a[0] + (b[0] - a[0]) * frac,
            a[1] + (b[1] - a[1]) * frac,
            a[2] + (b[2] - a[2]) * frac,
        )
    }
}

impl Filter for FormantMorphFilter {
    fn process(&mut self, input: f32) -> f32 {
        let (mut f1, mut f2, mut f3) = self.current_formants();
        let scale = (self.cutoff / 800.0).powf(0.35).clamp(0.6, 1.7);
        let nyquist = self.sample_rate * 0.45;
        f1 = (f1 * scale).min(nyquist);
        f2 = (f2 * scale).min(nyquist);
        f3 = (f3 * scale).min(nyquist);
        let q = 0.45 + self.resonance * 0.45;

        self.formants[0].set_cutoff(f1);
        self.formants[1].set_cutoff(f2);
        self.formants[2].set_cutoff(f3);
        self.formants[0].set_resonance(q);
        self.formants[1].set_resonance(q);
        self.formants[2].set_resonance(q);

        let mut out = 0.0;
        out += self.formants[0].process(input);
        out += self.formants[1].process(input);
        out += self.formants[2].process(input);
        out * 0.33
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        for filter in &mut self.formants {
            filter.set_sample_rate(self.sample_rate);
        }
    }
}

#[derive(Debug, Clone)]
pub struct BitcrushLpFilter {
    sample_rate: f32,
    cutoff: f32,
    resonance: f32,
    lp_state: f32,
    hold_samples: usize,
    hold_counter: usize,
    held: f32,
    bit_depth: f32,
}

impl BitcrushLpFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            cutoff: 2000.0,
            resonance: 0.0,
            lp_state: 0.0,
            hold_samples: 1,
            hold_counter: 0,
            held: 0.0,
            bit_depth: 12.0,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.max(40.0).min(self.sample_rate * 0.45);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        let crush = resonance.clamp(0.0, 1.0);
        self.resonance = crush;
        self.bit_depth = (12.0 - crush * 8.0).clamp(3.0, 12.0);
        self.hold_samples = (1.0 + crush * 14.0).round() as usize;
        self.hold_samples = self.hold_samples.max(1);
    }
}

impl Filter for BitcrushLpFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.hold_counter == 0 {
            self.held = input;
            self.hold_counter = self.hold_samples;
        }
        self.hold_counter = self.hold_counter.saturating_sub(1);

        let levels = 2.0_f32.powf(self.bit_depth);
        let crushed = (self.held * levels).round() / levels.max(1.0);

        let g = (2.0 * PI * self.cutoff / self.sample_rate).sin().clamp(0.0, 1.0);
        self.lp_state += g * (crushed - self.lp_state);
        self.lp_state
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }
}

#[derive(Debug, Clone)]
pub struct RainbowCombFilter {
    comb: CombFilter,
    formants: [BandpassFilter; 3],
    sample_rate: f32,
    base_cutoff: f32,
    vowel_phase: f32,
    vowel_rate: f32,
    formant_mix: f32,
    formant_res: f32,
}

impl RainbowCombFilter {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.max(1.0);
        Self {
            comb: CombFilter::new(sample_rate),
            formants: [
                BandpassFilter::new(700.0, 0.6, sample_rate),
                BandpassFilter::new(1200.0, 0.6, sample_rate),
                BandpassFilter::new(2600.0, 0.6, sample_rate),
            ],
            sample_rate,
            base_cutoff: 220.0,
            vowel_phase: 0.0,
            vowel_rate: 0.35,
            formant_mix: 0.6,
            formant_res: 0.55,
        }
    }

    pub fn set_cutoff(&mut self, freq: f32) {
        let freq = freq.max(40.0).min(4000.0);
        self.base_cutoff = freq;
        self.comb.set_cutoff(freq);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        let r = resonance.clamp(0.0, 1.0);
        self.comb.set_resonance(r);
        self.vowel_rate = 0.15 + r * 1.6;
        self.formant_mix = (0.25 + r * 0.6).clamp(0.0, 0.9);
        self.formant_res = (0.35 + r * 0.55).clamp(0.1, 0.95);
    }

    fn advance_lfo(&mut self) {
        let phase_inc = self.vowel_rate / self.sample_rate;
        self.vowel_phase = (self.vowel_phase + phase_inc).fract();
    }

    fn current_formants(&self) -> (f32, f32, f32) {
        const VOWELS: [[f32; 3]; 5] = [
            [800.0, 1150.0, 2900.0], // A
            [350.0, 2000.0, 3000.0], // I
            [325.0, 700.0, 2500.0],  // U
            [450.0, 1700.0, 2600.0], // E
            [500.0, 900.0, 2400.0],  // O
        ];
        let pos = self.vowel_phase * 5.0;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f32;
        let a = VOWELS[idx % VOWELS.len()];
        let b = VOWELS[(idx + 1) % VOWELS.len()];
        (
            a[0] + (b[0] - a[0]) * frac,
            a[1] + (b[1] - a[1]) * frac,
            a[2] + (b[2] - a[2]) * frac,
        )
    }
}

impl Filter for RainbowCombFilter {
    fn process(&mut self, input: f32) -> f32 {
        self.advance_lfo();

        let (mut f1, mut f2, mut f3) = self.current_formants();
        let scale = (self.base_cutoff / 800.0).powf(0.25).clamp(0.6, 1.6);
        let nyquist = self.sample_rate * 0.45;
        f1 = (f1 * scale).min(nyquist);
        f2 = (f2 * scale).min(nyquist);
        f3 = (f3 * scale).min(nyquist);

        self.formants[0].set_cutoff(f1);
        self.formants[1].set_cutoff(f2);
        self.formants[2].set_cutoff(f3);
        self.formants[0].set_resonance(self.formant_res);
        self.formants[1].set_resonance(self.formant_res);
        self.formants[2].set_resonance(self.formant_res);

        let comb_out = self.comb.process(input);
        let mut formant_out = 0.0;
        formant_out += self.formants[0].process(comb_out);
        formant_out += self.formants[1].process(comb_out);
        formant_out += self.formants[2].process(comb_out);
        formant_out *= 0.33;

        let mix = self.formant_mix;
        comb_out * (1.0 - mix) + formant_out * mix
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.comb.set_sample_rate(self.sample_rate);
        for filter in &mut self.formants {
            filter.set_sample_rate(self.sample_rate);
        }
    }
}

impl LowpassFilter {
    pub fn new(
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> Self {
        LowpassFilter {
            cutoff,
            resonance,
            sample_rate,
            prev_output: 0.0,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance;
    }
}

impl Filter for LowpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        let cutoff = self.cutoff.max(20.0).min(20000.0);
        let resonance = self.resonance.max(0.0).min(0.99);
        
        // Calculate filter coefficient based on cutoff frequency
        let omega = 2.0 * std::f32::consts::PI * cutoff / self.sample_rate;
        let alpha = omega / (omega + 1.0);
        
        // Apply resonance as feedback
        let feedback = self.prev_output * resonance;
        self.prev_output = self.prev_output + alpha * (input + feedback - self.prev_output);
        
        self.prev_output
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}

#[derive(Debug, Clone)]
pub struct NotchFilter {
    cutoff: f32,
    bandwidth: f32,
    sample_rate: f32,
    buf0: f32,
    buf1: f32,
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
}

impl NotchFilter {
    pub fn new(
        cutoff: f32,
        bandwidth: f32,
        sample_rate: f32,
    ) -> Self {
        let mut filter = NotchFilter {
            cutoff,
            bandwidth,
            sample_rate,
            buf0: 0.0,
            buf1: 0.0,
            a0: 0.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
        };
        filter.calculate_coefficients();
        filter
    }

    pub fn calculate_coefficients(&mut self) {
        let wc = 2.0 * PI * self.cutoff / self.sample_rate; // cutoff frequency in radians
        let q = (self.bandwidth * 10.0).max(0.1); // Convert bandwidth to Q factor
        let alpha = wc.sin() / (2.0 * q); // bandwidth parameter

        self.a0 = 1.0;
        self.a1 = -2.0 * wc.cos();
        self.a2 = 1.0;
        let norm = 1.0 / (1.0 + alpha); // normalization factor
        self.a0 *= norm;
        self.a1 *= norm;
        self.a2 *= norm;
        self.b1 = -2.0 * wc.cos() * norm;
        self.b2 = (1.0 - alpha) * norm;
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.bandwidth = resonance;
    }
}

impl Filter for NotchFilter {
    fn process(&mut self, input: f32) -> f32 {
        let cutoff = self.cutoff.max(20.0).min(20000.0);
        let bandwidth = self.bandwidth.max(0.01).min(1.0);

        if (cutoff - self.cutoff).abs() > 0.1 || (bandwidth - self.bandwidth).abs() > 0.001 {
            self.cutoff = cutoff;
            self.bandwidth = bandwidth;
            self.calculate_coefficients();
        }

        // apply filter
        let output = self.a0 * input + self.a1 * self.buf0 + self.a2 * self.buf1
            - self.b1 * self.buf0
            - self.b2 * self.buf1;
        self.buf1 = self.buf0;
        self.buf0 = output;
        output
    }
    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.calculate_coefficients();
    }
}

#[derive(Debug, Clone)]
pub struct StatevariableFilter {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
    prev_input: f32,
    lowpass_output: f32,
    highpass_output: f32,
    bandpass_output: f32,
}

impl StatevariableFilter {
    pub fn new(
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> Self {
        StatevariableFilter {
            cutoff,
            resonance,
            sample_rate,
            prev_input: 0.0,
            lowpass_output: 0.0,
            highpass_output: 0.0,
            bandpass_output: 0.0,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance;
    }
}

impl Filter for StatevariableFilter {
    fn process(&mut self, input: f32) -> f32 {
        let cutoff = self.cutoff.max(20.0).min(self.sample_rate * 0.45);
        let resonance = self.resonance.max(0.0).min(1.0);

        // Calculate frequency parameter (must be < 0.5 for stability)
        let f = (2.0 * PI * cutoff / self.sample_rate).sin().clamp(0.0, 0.25);
        
        // Q factor: higher resonance = lower damping
        let damp = 2.0 * (1.0 - resonance * 0.9).max(0.1);

        // Chamberlin state variable filter
        self.lowpass_output += f * self.bandpass_output;
        self.highpass_output = input - self.lowpass_output - damp * self.bandpass_output;
        self.bandpass_output += f * self.highpass_output;

        // Prevent NaN/Inf
        if !self.lowpass_output.is_finite() {
            self.lowpass_output = 0.0;
            self.bandpass_output = 0.0;
            self.highpass_output = 0.0;
        }

        // Return bandpass output
        self.bandpass_output.clamp(-2.0, 2.0)
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}

#[derive(Debug, Clone)]
pub struct NoneFilter {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
}

impl NoneFilter {
    pub fn new(
        cutoff: f32,
        resonance: f32,
        sample_rate: f32,
    ) -> Self {
        NoneFilter {
            cutoff,
            resonance,
            sample_rate,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance;
    }
}

impl Filter for NoneFilter {
    fn process(&mut self, input: f32) -> f32 {
        // No filtering, simply return the input unchanged
        input
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
}

#[derive(Debug, Clone)]
pub struct DCBlocker {
    x1: f32,
    y1: f32,
    r: f32,
}

impl DCBlocker {
    pub fn new() -> Self {
        DCBlocker {
            x1: 0.0,
            y1: 0.0,
            r: 0.995, // The closer this value to 1.0, the lower the cutoff frequency
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.y1 = input - self.x1 + self.r * self.y1;
        self.x1 = input;
        self.y1
    }
}

pub fn generate_filter(
    filter_type: FilterType,
    cutoff: f32,
    resonance: f32,
    filter_cut_envelope: &mut ADSREnvelope,
    filter_res_envelope: &mut ADSREnvelope,
    input: f32,
    sample_rate: f32,
) -> f32 {
    filter_cut_envelope.advance();
    filter_res_envelope.advance();
    let filter_cut = filter_cut_envelope.get_value() * cutoff;
    let filter_res = filter_res_envelope.get_value() * resonance;
    
    let out = match filter_type {
        FilterType::None => input,
        FilterType::Lowpass => {
            let mut filter = LowpassFilter::new(cutoff, resonance, sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Highpass => {
            let mut filter = HighpassFilter::new(cutoff, resonance, sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Bandpass => {
            let mut filter = BandpassFilter::new(cutoff, resonance, sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Notch => {
            let mut filter = NotchFilter::new(cutoff, resonance, sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Statevariable => {
            let mut filter = StatevariableFilter::new(cutoff, resonance, sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Comb => {
            let mut filter = CombFilter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::RainbowComb => {
            let mut filter = RainbowCombFilter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::DiodeLadderLp => {
            let mut filter = DiodeLadderFilter::new_lowpass(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::DiodeLadderHp => {
            let mut filter = DiodeLadderFilter::new_highpass(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Ms20Pair => {
            let mut filter = Ms20Filter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::FormantMorph => {
            let mut filter = FormantMorphFilter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::Phaser => {
            let mut filter = PhaserFilter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::CombAllpass => {
            let mut filter = CombAllpassFilter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
        FilterType::BitcrushLp => {
            let mut filter = BitcrushLpFilter::new(sample_rate);
            filter.set_cutoff(filter_cut);
            filter.set_resonance(filter_res);
            filter.process(input)
        }
    };

    if matches!(filter_type, FilterType::None) {
        out
    } else {
        tame_resonance(out, filter_res)
    }
}

pub fn tame_resonance(output: f32, resonance: f32) -> f32 {
    let r = resonance.clamp(0.0, 1.0);
    let comp = 1.0 / (1.0 + r * 1.6);
    let driven = output * comp * (1.0 + r * 0.7);
    let clipped = driven.tanh();
    clipped / (1.0 + clipped.abs() * 0.35)
}

