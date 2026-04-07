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
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum FilterStyle {
    Digital,
    Vintage,
}

pub trait Filter: Send {
    fn process(&mut self, input: f32) -> f32;
    fn set_sample_rate(&mut self, sample_rate: f32);
}

const MIN_CUTOFF_HZ: f32 = 20.0;
const MAX_CUTOFF_FRACTION: f32 = 0.49;

fn clamp_cutoff(cutoff: f32, sample_rate: f32) -> f32 {
    cutoff.clamp(MIN_CUTOFF_HZ, sample_rate * MAX_CUTOFF_FRACTION)
}

fn apply_style_cutoff(cutoff: f32, sample_rate: f32, style: FilterStyle) -> f32 {
    let cutoff = clamp_cutoff(cutoff, sample_rate);
    match style {
        FilterStyle::Digital => cutoff,
        FilterStyle::Vintage => (cutoff * 0.9).max(MIN_CUTOFF_HZ),
    }
}

fn apply_style_resonance(resonance: f32, style: FilterStyle) -> f32 {
    let resonance = resonance.clamp(0.0, 1.0);
    match style {
        FilterStyle::Digital => resonance,
        FilterStyle::Vintage => (resonance * 0.85).min(1.0),
    }
}

fn resonance_to_q(resonance: f32) -> f32 {
    let r = resonance.clamp(0.0, 1.0);
    let min_q: f32 = 0.5;
    let max_q: f32 = 18.0;
    min_q * (max_q / min_q).powf(r)
}

fn saturate_vintage(value: f32, drive: f32) -> f32 {
    let drive = drive.max(0.1);
    (value * drive).tanh() / drive
}

fn vintage_drive_value(vintage_drive: f32, resonance: f32) -> f32 {
    let drive = vintage_drive.clamp(0.0, 1.0);
    let res = resonance.clamp(0.0, 1.0);
    1.0 + drive * 2.5 + res * 0.75
}

fn shape_vintage_drive(vintage_drive: f32, curve: f32) -> f32 {
    let drive = vintage_drive.clamp(0.0, 1.0);
    let curve = curve.clamp(0.0, 1.0);
    let exponent = 0.6 + curve * 1.8;
    drive.powf(exponent)
}

fn vintage_output_gain(vintage_drive: f32, trim: f32) -> f32 {
    let trim = trim.clamp(0.5, 1.5);
    let compensation = 1.0 / (1.0 + vintage_drive * 0.7);
    compensation * trim
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

fn biquad_coefficients(filter_type: FilterType, cutoff: f32, q: f32, sample_rate: f32) -> (f32, f32, f32, f32, f32) {
    let cutoff = clamp_cutoff(cutoff, sample_rate);
    let w0 = 2.0 * PI * cutoff / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q.max(0.001));

    let (b0, b1, b2, a0, a1, a2) = match filter_type {
        FilterType::Lowpass => {
            let b0 = (1.0 - cos_w0) * 0.5;
            let b1 = 1.0 - cos_w0;
            let b2 = (1.0 - cos_w0) * 0.5;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Highpass => {
            let b0 = (1.0 + cos_w0) * 0.5;
            let b1 = -(1.0 + cos_w0);
            let b2 = (1.0 + cos_w0) * 0.5;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Bandpass | FilterType::Statevariable => {
            let b0 = alpha;
            let b1 = 0.0;
            let b2 = -alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::Notch => {
            let b0 = 1.0;
            let b1 = -2.0 * cos_w0;
            let b2 = 1.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (b0, b1, b2, a0, a1, a2)
        }
        FilterType::None => (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
    };

    let inv_a0 = 1.0 / a0.max(0.0001);
    (b0 * inv_a0, b1 * inv_a0, b2 * inv_a0, a1 * inv_a0, a2 * inv_a0)
}

#[derive(Debug, Clone)]
pub struct HighpassFilter {
    cutoff: f32,
    resonance: f32,
    style: FilterStyle,
    vintage_drive: f32,
    vintage_curve: f32,
    vintage_mix: f32,
    vintage_trim: f32,
    sample_rate: f32,
    biquad: Biquad,
    dirty: bool,
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
            style: FilterStyle::Digital,
            vintage_drive: 0.35,
            vintage_curve: 0.5,
            vintage_mix: 1.0,
            vintage_trim: 1.0,
            sample_rate,
            biquad: Biquad::new(),
            dirty: true,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        if (cutoff - self.cutoff).abs() > 0.1 {
            self.cutoff = cutoff;
            self.dirty = true;
        }
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        if (resonance - self.resonance).abs() > 0.001 {
            self.resonance = resonance;
            self.dirty = true;
        }
    }

    pub fn set_style(&mut self, style: FilterStyle) {
        if style != self.style {
            self.style = style;
            self.dirty = true;
        }
    }

    pub fn set_vintage_drive(&mut self, drive: f32) {
        self.vintage_drive = drive;
    }

    pub fn set_vintage_curve(&mut self, curve: f32) {
        self.vintage_curve = curve;
    }

    pub fn set_vintage_mix(&mut self, mix: f32) {
        self.vintage_mix = mix;
    }

    pub fn set_vintage_trim(&mut self, trim: f32) {
        self.vintage_trim = trim;
    }

    fn update_coeffs(&mut self) {
        let cutoff = apply_style_cutoff(self.cutoff, self.sample_rate, self.style);
        let resonance = apply_style_resonance(self.resonance, self.style);
        let q = resonance_to_q(resonance);
        let (b0, b1, b2, a1, a2) = biquad_coefficients(FilterType::Highpass, cutoff, q, self.sample_rate);
        self.biquad.set_coefficients(b0, b1, b2, a1, a2);
        self.dirty = false;
    }
}

impl Filter for HighpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            self.update_coeffs();
        }
        let output = match self.style {
            FilterStyle::Digital => self.biquad.process(input),
            FilterStyle::Vintage => {
                let drive = shape_vintage_drive(self.vintage_drive, self.vintage_curve);
                let input = saturate_vintage(input, vintage_drive_value(drive, self.resonance));
                let filtered = self.biquad.process(input);
                let wet = saturate_vintage(filtered, 1.0 + drive * 1.5);
                let mix = self.vintage_mix.clamp(0.0, 1.0);
                let mixed = filtered * (1.0 - mix) + wet * mix;
                mixed * vintage_output_gain(drive, self.vintage_trim)
            }
        };
        output
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.dirty = true;
    }
}

#[derive(Debug, Clone)]
pub struct BandpassFilter {
    cutoff: f32,
    resonance: f32,
    style: FilterStyle,
    vintage_drive: f32,
    vintage_curve: f32,
    vintage_mix: f32,
    vintage_trim: f32,
    sample_rate: f32,
    biquad: Biquad,
    dirty: bool,
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
            style: FilterStyle::Digital,
            vintage_drive: 0.35,
            vintage_curve: 0.5,
            vintage_mix: 1.0,
            vintage_trim: 1.0,
            sample_rate,
            biquad: Biquad::new(),
            dirty: true,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        if (cutoff - self.cutoff).abs() > 0.1 {
            self.cutoff = cutoff;
            self.dirty = true;
        }
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        if (resonance - self.resonance).abs() > 0.001 {
            self.resonance = resonance;
            self.dirty = true;
        }
    }

    pub fn set_style(&mut self, style: FilterStyle) {
        if style != self.style {
            self.style = style;
            self.dirty = true;
        }
    }

    pub fn set_vintage_drive(&mut self, drive: f32) {
        self.vintage_drive = drive;
    }

    pub fn set_vintage_curve(&mut self, curve: f32) {
        self.vintage_curve = curve;
    }

    pub fn set_vintage_mix(&mut self, mix: f32) {
        self.vintage_mix = mix;
    }

    pub fn set_vintage_trim(&mut self, trim: f32) {
        self.vintage_trim = trim;
    }

    fn update_coeffs(&mut self) {
        let cutoff = apply_style_cutoff(self.cutoff, self.sample_rate, self.style);
        let resonance = apply_style_resonance(self.resonance, self.style);
        let q = resonance_to_q(resonance);
        let (b0, b1, b2, a1, a2) = biquad_coefficients(FilterType::Bandpass, cutoff, q, self.sample_rate);
        self.biquad.set_coefficients(b0, b1, b2, a1, a2);
        self.dirty = false;
    }
}
impl Filter for BandpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            self.update_coeffs();
        }
        let output = match self.style {
            FilterStyle::Digital => self.biquad.process(input),
            FilterStyle::Vintage => {
                let drive = shape_vintage_drive(self.vintage_drive, self.vintage_curve);
                let input = saturate_vintage(input, vintage_drive_value(drive, self.resonance));
                let filtered = self.biquad.process(input);
                let wet = saturate_vintage(filtered, 1.0 + drive * 1.5);
                let mix = self.vintage_mix.clamp(0.0, 1.0);
                let mixed = filtered * (1.0 - mix) + wet * mix;
                mixed * vintage_output_gain(drive, self.vintage_trim)
            }
        };
        output
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.dirty = true;
    }
}

#[derive(Debug, Clone)]
pub struct LowpassFilter {
    cutoff: f32,
    resonance: f32,
    style: FilterStyle,
    vintage_drive: f32,
    vintage_curve: f32,
    vintage_mix: f32,
    vintage_trim: f32,
    sample_rate: f32,
    biquad: Biquad,
    dirty: bool,
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
            style: FilterStyle::Digital,
            vintage_drive: 0.35,
            vintage_curve: 0.5,
            vintage_mix: 1.0,
            vintage_trim: 1.0,
            sample_rate,
            biquad: Biquad::new(),
            dirty: true,
        }
    }
    pub fn set_cutoff(&mut self, cutoff: f32) {
        if (cutoff - self.cutoff).abs() > 0.1 {
            self.cutoff = cutoff;
            self.dirty = true;
        }
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        if (resonance - self.resonance).abs() > 0.001 {
            self.resonance = resonance;
            self.dirty = true;
        }
    }

    pub fn set_style(&mut self, style: FilterStyle) {
        if style != self.style {
            self.style = style;
            self.dirty = true;
        }
    }

    pub fn set_vintage_drive(&mut self, drive: f32) {
        self.vintage_drive = drive;
    }

    pub fn set_vintage_curve(&mut self, curve: f32) {
        self.vintage_curve = curve;
    }

    pub fn set_vintage_mix(&mut self, mix: f32) {
        self.vintage_mix = mix;
    }

    pub fn set_vintage_trim(&mut self, trim: f32) {
        self.vintage_trim = trim;
    }

    fn update_coeffs(&mut self) {
        let cutoff = apply_style_cutoff(self.cutoff, self.sample_rate, self.style);
        let resonance = apply_style_resonance(self.resonance, self.style);
        let q = resonance_to_q(resonance);
        let (b0, b1, b2, a1, a2) = biquad_coefficients(FilterType::Lowpass, cutoff, q, self.sample_rate);
        self.biquad.set_coefficients(b0, b1, b2, a1, a2);
        self.dirty = false;
    }
}

impl Filter for LowpassFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            self.update_coeffs();
        }
        let output = match self.style {
            FilterStyle::Digital => self.biquad.process(input),
            FilterStyle::Vintage => {
                let drive = shape_vintage_drive(self.vintage_drive, self.vintage_curve);
                let input = saturate_vintage(input, vintage_drive_value(drive, self.resonance));
                let filtered = self.biquad.process(input);
                let wet = saturate_vintage(filtered, 1.0 + drive * 1.8);
                let mix = self.vintage_mix.clamp(0.0, 1.0);
                let mixed = filtered * (1.0 - mix) + wet * mix;
                mixed * vintage_output_gain(drive, self.vintage_trim)
            }
        };
        output
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.dirty = true;
    }
}

#[derive(Debug, Clone)]
pub struct NotchFilter {
    cutoff: f32,
    resonance: f32,
    style: FilterStyle,
    vintage_drive: f32,
    vintage_curve: f32,
    vintage_mix: f32,
    vintage_trim: f32,
    sample_rate: f32,
    biquad: Biquad,
    dirty: bool,
}

impl NotchFilter {
    pub fn new(
        cutoff: f32,
        bandwidth: f32,
        sample_rate: f32,
    ) -> Self {
        let mut filter = NotchFilter {
            cutoff,
            resonance: bandwidth,
            style: FilterStyle::Digital,
            vintage_drive: 0.35,
            vintage_curve: 0.5,
            vintage_mix: 1.0,
            vintage_trim: 1.0,
            sample_rate,
            biquad: Biquad::new(),
            dirty: true,
        };
        filter.update_coeffs();
        filter
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        if (cutoff - self.cutoff).abs() > 0.1 {
            self.cutoff = cutoff;
            self.dirty = true;
        }
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        if (resonance - self.resonance).abs() > 0.001 {
            self.resonance = resonance;
            self.dirty = true;
        }
    }

    pub fn set_style(&mut self, style: FilterStyle) {
        if style != self.style {
            self.style = style;
            self.dirty = true;
        }
    }

    pub fn set_vintage_drive(&mut self, drive: f32) {
        self.vintage_drive = drive;
    }

    pub fn set_vintage_curve(&mut self, curve: f32) {
        self.vintage_curve = curve;
    }

    pub fn set_vintage_mix(&mut self, mix: f32) {
        self.vintage_mix = mix;
    }

    pub fn set_vintage_trim(&mut self, trim: f32) {
        self.vintage_trim = trim;
    }

    fn update_coeffs(&mut self) {
        let cutoff = apply_style_cutoff(self.cutoff, self.sample_rate, self.style);
        let resonance = apply_style_resonance(self.resonance, self.style);
        let q = resonance_to_q(resonance);
        let (b0, b1, b2, a1, a2) = biquad_coefficients(FilterType::Notch, cutoff, q, self.sample_rate);
        self.biquad.set_coefficients(b0, b1, b2, a1, a2);
        self.dirty = false;
    }
}

impl Filter for NotchFilter {
    fn process(&mut self, input: f32) -> f32 {
        if self.dirty {
            self.update_coeffs();
        }
        let output = match self.style {
            FilterStyle::Digital => self.biquad.process(input),
            FilterStyle::Vintage => {
                let drive = shape_vintage_drive(self.vintage_drive, self.vintage_curve);
                let input = saturate_vintage(input, vintage_drive_value(drive, self.resonance));
                let filtered = self.biquad.process(input);
                let wet = saturate_vintage(filtered, 1.0 + drive * 1.3);
                let mix = self.vintage_mix.clamp(0.0, 1.0);
                let mixed = filtered * (1.0 - mix) + wet * mix;
                mixed * vintage_output_gain(drive, self.vintage_trim)
            }
        };
        output
    }
    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.dirty = true;
    }
}

#[derive(Debug, Clone)]
pub struct StatevariableFilter {
    cutoff: f32,
    resonance: f32,
    style: FilterStyle,
    vintage_drive: f32,
    vintage_curve: f32,
    vintage_mix: f32,
    vintage_trim: f32,
    sample_rate: f32,
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
            style: FilterStyle::Digital,
            vintage_drive: 0.35,
            vintage_curve: 0.5,
            vintage_mix: 1.0,
            vintage_trim: 1.0,
            sample_rate,
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

    pub fn set_style(&mut self, style: FilterStyle) {
        self.style = style;
    }

    pub fn set_vintage_drive(&mut self, drive: f32) {
        self.vintage_drive = drive;
    }

    pub fn set_vintage_curve(&mut self, curve: f32) {
        self.vintage_curve = curve;
    }

    pub fn set_vintage_mix(&mut self, mix: f32) {
        self.vintage_mix = mix;
    }

    pub fn set_vintage_trim(&mut self, trim: f32) {
        self.vintage_trim = trim;
    }
}

impl Filter for StatevariableFilter {
    fn process(&mut self, input: f32) -> f32 {
        let cutoff = apply_style_cutoff(self.cutoff, self.sample_rate, self.style);
        let resonance = apply_style_resonance(self.resonance, self.style);
        let input = match self.style {
            FilterStyle::Digital => input,
            FilterStyle::Vintage => {
                let drive = shape_vintage_drive(self.vintage_drive, self.vintage_curve);
                saturate_vintage(input, vintage_drive_value(drive, resonance))
            }
        };

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
        let output = self.bandpass_output.clamp(-2.0, 2.0);
        match self.style {
            FilterStyle::Digital => output,
            FilterStyle::Vintage => {
                let drive = shape_vintage_drive(self.vintage_drive, self.vintage_curve);
                let wet = saturate_vintage(output, 1.0 + drive * 1.2);
                let mix = self.vintage_mix.clamp(0.0, 1.0);
                let mixed = output * (1.0 - mix) + wet * mix;
                mixed * vintage_output_gain(drive, self.vintage_trim)
            }
        }
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
    
    match filter_type {
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
    }
}

