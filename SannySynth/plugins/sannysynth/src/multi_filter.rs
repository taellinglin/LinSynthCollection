use crate::filter::{BandpassFilter, Filter, FilterStyle, FilterType, HighpassFilter, LowpassFilter, NotchFilter, StatevariableFilter};
use crate::FilterRouting;

struct FilterBank {
    lowpass: LowpassFilter,
    highpass: HighpassFilter,
    bandpass: BandpassFilter,
    notch: NotchFilter,
    statevariable: StatevariableFilter,
}

impl FilterBank {
    fn new(sample_rate: f32) -> Self {
        Self {
            lowpass: LowpassFilter::new(1000.0, 0.5, sample_rate),
            highpass: HighpassFilter::new(1000.0, 0.5, sample_rate),
            bandpass: BandpassFilter::new(1000.0, 0.5, sample_rate),
            notch: NotchFilter::new(1000.0, 1.0, sample_rate),
            statevariable: StatevariableFilter::new(1000.0, 0.5, sample_rate),
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.lowpass.set_sample_rate(sample_rate);
        self.highpass.set_sample_rate(sample_rate);
        self.bandpass.set_sample_rate(sample_rate);
        self.notch.set_sample_rate(sample_rate);
        self.statevariable.set_sample_rate(sample_rate);
    }

    fn apply(
        &mut self,
        filter_type: FilterType,
        style: FilterStyle,
        vintage_drive: f32,
        vintage_curve: f32,
        vintage_mix: f32,
        vintage_trim: f32,
        cutoff: f32,
        resonance: f32,
        input: f32,
    ) -> f32 {
        match filter_type {
            FilterType::None => input,
            FilterType::Lowpass => {
                self.lowpass.set_cutoff(cutoff);
                self.lowpass.set_resonance(resonance);
                self.lowpass.set_style(style);
                self.lowpass.set_vintage_drive(vintage_drive);
                self.lowpass.set_vintage_curve(vintage_curve);
                self.lowpass.set_vintage_mix(vintage_mix);
                self.lowpass.set_vintage_trim(vintage_trim);
                self.lowpass.process(input)
            }
            FilterType::Highpass => {
                self.highpass.set_cutoff(cutoff);
                self.highpass.set_resonance(resonance);
                self.highpass.set_style(style);
                self.highpass.set_vintage_drive(vintage_drive);
                self.highpass.set_vintage_curve(vintage_curve);
                self.highpass.set_vintage_mix(vintage_mix);
                self.highpass.set_vintage_trim(vintage_trim);
                self.highpass.process(input)
            }
            FilterType::Bandpass => {
                self.bandpass.set_cutoff(cutoff);
                self.bandpass.set_resonance(resonance);
                self.bandpass.set_style(style);
                self.bandpass.set_vintage_drive(vintage_drive);
                self.bandpass.set_vintage_curve(vintage_curve);
                self.bandpass.set_vintage_mix(vintage_mix);
                self.bandpass.set_vintage_trim(vintage_trim);
                self.bandpass.process(input)
            }
            FilterType::Notch => {
                self.notch.set_cutoff(cutoff);
                self.notch.set_resonance(resonance);
                self.notch.set_style(style);
                self.notch.set_vintage_drive(vintage_drive);
                self.notch.set_vintage_curve(vintage_curve);
                self.notch.set_vintage_mix(vintage_mix);
                self.notch.set_vintage_trim(vintage_trim);
                self.notch.process(input)
            }
            FilterType::Statevariable => {
                self.statevariable.set_cutoff(cutoff);
                self.statevariable.set_resonance(resonance);
                self.statevariable.set_style(style);
                self.statevariable.set_vintage_drive(vintage_drive);
                self.statevariable.set_vintage_curve(vintage_curve);
                self.statevariable.set_vintage_mix(vintage_mix);
                self.statevariable.set_vintage_trim(vintage_trim);
                self.statevariable.process(input)
            }
        }
    }
}

pub struct MultiStageFilter {
    stage_a_left: FilterBank,
    stage_a_right: FilterBank,
    stage_b_left: FilterBank,
    stage_b_right: FilterBank,
    stage_c_left: FilterBank,
    stage_c_right: FilterBank,
}

impl MultiStageFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            stage_a_left: FilterBank::new(sample_rate),
            stage_a_right: FilterBank::new(sample_rate),
            stage_b_left: FilterBank::new(sample_rate),
            stage_b_right: FilterBank::new(sample_rate),
            stage_c_left: FilterBank::new(sample_rate),
            stage_c_right: FilterBank::new(sample_rate),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.stage_a_left.set_sample_rate(sample_rate);
        self.stage_a_right.set_sample_rate(sample_rate);
        self.stage_b_left.set_sample_rate(sample_rate);
        self.stage_b_right.set_sample_rate(sample_rate);
        self.stage_c_left.set_sample_rate(sample_rate);
        self.stage_c_right.set_sample_rate(sample_rate);
    }

    pub fn process(
        &mut self,
        left: f32,
        right: f32,
        routing: FilterRouting,
        a_type: FilterType,
        a_style: FilterStyle,
        a_drive: f32,
        a_curve: f32,
        a_mix: f32,
        a_trim: f32,
        a_cut: f32,
        a_res: f32,
        a_amt: f32,
        b_type: FilterType,
        b_style: FilterStyle,
        b_drive: f32,
        b_curve: f32,
        b_mix: f32,
        b_trim: f32,
        b_cut: f32,
        b_res: f32,
        b_amt: f32,
        c_type: FilterType,
        c_style: FilterStyle,
        c_drive: f32,
        c_curve: f32,
        c_mix: f32,
        c_trim: f32,
        c_cut: f32,
        c_res: f32,
        c_amt: f32,
        morph: f32,
        parallel_ab: f32,
        parallel_c: f32,
    ) -> (f32, f32) {
        let a_amt = a_amt.clamp(0.0, 1.0);
        let b_amt = b_amt.clamp(0.0, 1.0);
        let c_amt = c_amt.clamp(0.0, 1.0);

        let mut a_left =
            self.stage_a_left
                .apply(a_type, a_style, a_drive, a_curve, a_mix, a_trim, a_cut, a_res, left);
        let mut a_right =
            self.stage_a_right
                .apply(a_type, a_style, a_drive, a_curve, a_mix, a_trim, a_cut, a_res, right);
        a_left = left * (1.0 - a_amt) + a_left * a_amt;
        a_right = right * (1.0 - a_amt) + a_right * a_amt;

        let mut b_left =
            self.stage_b_left
                .apply(b_type, b_style, b_drive, b_curve, b_mix, b_trim, b_cut, b_res, left);
        let mut b_right =
            self.stage_b_right
                .apply(b_type, b_style, b_drive, b_curve, b_mix, b_trim, b_cut, b_res, right);
        b_left = left * (1.0 - b_amt) + b_left * b_amt;
        b_right = right * (1.0 - b_amt) + b_right * b_amt;

        let mut c_left =
            self.stage_c_left
                .apply(c_type, c_style, c_drive, c_curve, c_mix, c_trim, c_cut, c_res, left);
        let mut c_right =
            self.stage_c_right
                .apply(c_type, c_style, c_drive, c_curve, c_mix, c_trim, c_cut, c_res, right);
        c_left = left * (1.0 - c_amt) + c_left * c_amt;
        c_right = right * (1.0 - c_amt) + c_right * c_amt;

        match routing {
            FilterRouting::Serial => {
                let mut out_left = self.stage_a_left.apply(
                    a_type,
                    a_style,
                    a_drive,
                    a_curve,
                    a_mix,
                    a_trim,
                    a_cut,
                    a_res,
                    left,
                );
                let mut out_right = self.stage_a_right.apply(
                    a_type,
                    a_style,
                    a_drive,
                    a_curve,
                    a_mix,
                    a_trim,
                    a_cut,
                    a_res,
                    right,
                );
                out_left = left * (1.0 - a_amt) + out_left * a_amt;
                out_right = right * (1.0 - a_amt) + out_right * a_amt;

                out_left = self.stage_b_left.apply(
                    b_type,
                    b_style,
                    b_drive,
                    b_curve,
                    b_mix,
                    b_trim,
                    b_cut,
                    b_res,
                    out_left,
                );
                out_right = self.stage_b_right.apply(
                    b_type,
                    b_style,
                    b_drive,
                    b_curve,
                    b_mix,
                    b_trim,
                    b_cut,
                    b_res,
                    out_right,
                );
                out_left = left * (1.0 - b_amt) + out_left * b_amt;
                out_right = right * (1.0 - b_amt) + out_right * b_amt;

                out_left = self.stage_c_left.apply(
                    c_type,
                    c_style,
                    c_drive,
                    c_curve,
                    c_mix,
                    c_trim,
                    c_cut,
                    c_res,
                    out_left,
                );
                out_right = self.stage_c_right.apply(
                    c_type,
                    c_style,
                    c_drive,
                    c_curve,
                    c_mix,
                    c_trim,
                    c_cut,
                    c_res,
                    out_right,
                );
                out_left = left * (1.0 - c_amt) + out_left * c_amt;
                out_right = right * (1.0 - c_amt) + out_right * c_amt;

                (out_left, out_right)
            }
            FilterRouting::Parallel => {
                let parallel_ab = parallel_ab.clamp(0.0, 1.0);
                let parallel_c = parallel_c.clamp(0.0, 1.0);
                let mix_left = a_left * (1.0 - parallel_ab) + b_left * parallel_ab;
                let mix_right = a_right * (1.0 - parallel_ab) + b_right * parallel_ab;
                let out_left = mix_left * (1.0 - parallel_c) + c_left * parallel_c;
                let out_right = mix_right * (1.0 - parallel_c) + c_right * parallel_c;
                (out_left, out_right)
            }
            FilterRouting::Morph => {
                let morph = morph.clamp(0.0, 1.0);
                if morph < 0.5 {
                    let t = morph * 2.0;
                    (
                        a_left * (1.0 - t) + b_left * t,
                        a_right * (1.0 - t) + b_right * t,
                    )
                } else {
                    let t = (morph - 0.5) * 2.0;
                    (
                        b_left * (1.0 - t) + c_left * t,
                        b_right * (1.0 - t) + c_right * t,
                    )
                }
            }
        }
    }
}
