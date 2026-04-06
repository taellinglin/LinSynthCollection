use crate::filter::{self, BandpassFilter, BitcrushLpFilter, CombAllpassFilter, CombFilter, DiodeLadderFilter, Filter, FilterType, FormantMorphFilter, HighpassFilter, Ms20Filter, LowpassFilter, NotchFilter, PhaserFilter, RainbowCombFilter, StatevariableFilter};
use crate::FilterRouting;

struct FilterBank {
    lowpass: LowpassFilter,
    highpass: HighpassFilter,
    bandpass: BandpassFilter,
    notch: NotchFilter,
    statevariable: StatevariableFilter,
    comb: CombFilter,
    rainbow_comb: RainbowCombFilter,
    diode_ladder_lp: DiodeLadderFilter,
    diode_ladder_hp: DiodeLadderFilter,
    ms20: Ms20Filter,
    formant_morph: FormantMorphFilter,
    phaser: PhaserFilter,
    comb_allpass: CombAllpassFilter,
    bitcrush_lp: BitcrushLpFilter,
}

impl FilterBank {
    fn new(sample_rate: f32) -> Self {
        Self {
            lowpass: LowpassFilter::new(1000.0, 0.5, sample_rate),
            highpass: HighpassFilter::new(1000.0, 0.5, sample_rate),
            bandpass: BandpassFilter::new(1000.0, 0.5, sample_rate),
            notch: NotchFilter::new(1000.0, 1.0, sample_rate),
            statevariable: StatevariableFilter::new(1000.0, 0.5, sample_rate),
            comb: CombFilter::new(sample_rate),
            rainbow_comb: RainbowCombFilter::new(sample_rate),
            diode_ladder_lp: DiodeLadderFilter::new_lowpass(sample_rate),
            diode_ladder_hp: DiodeLadderFilter::new_highpass(sample_rate),
            ms20: Ms20Filter::new(sample_rate),
            formant_morph: FormantMorphFilter::new(sample_rate),
            phaser: PhaserFilter::new(sample_rate),
            comb_allpass: CombAllpassFilter::new(sample_rate),
            bitcrush_lp: BitcrushLpFilter::new(sample_rate),
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.lowpass.set_sample_rate(sample_rate);
        self.highpass.set_sample_rate(sample_rate);
        self.bandpass.set_sample_rate(sample_rate);
        self.notch.set_sample_rate(sample_rate);
        self.statevariable.set_sample_rate(sample_rate);
        self.comb.set_sample_rate(sample_rate);
        self.rainbow_comb.set_sample_rate(sample_rate);
        self.diode_ladder_lp.set_sample_rate(sample_rate);
        self.diode_ladder_hp.set_sample_rate(sample_rate);
        self.ms20.set_sample_rate(sample_rate);
        self.formant_morph.set_sample_rate(sample_rate);
        self.phaser.set_sample_rate(sample_rate);
        self.comb_allpass.set_sample_rate(sample_rate);
        self.bitcrush_lp.set_sample_rate(sample_rate);
    }

    fn apply(&mut self, filter_type: FilterType, cutoff: f32, resonance: f32, input: f32) -> f32 {
        let out = match filter_type {
            FilterType::None => input,
            FilterType::Lowpass => {
                self.lowpass.set_cutoff(cutoff);
                self.lowpass.set_resonance(resonance);
                self.lowpass.process(input)
            }
            FilterType::Highpass => {
                self.highpass.set_cutoff(cutoff);
                self.highpass.set_resonance(resonance);
                self.highpass.process(input)
            }
            FilterType::Bandpass => {
                self.bandpass.set_cutoff(cutoff);
                self.bandpass.set_resonance(resonance);
                self.bandpass.process(input)
            }
            FilterType::Notch => {
                self.notch.set_cutoff(cutoff);
                self.notch.set_resonance(resonance);
                self.notch.process(input)
            }
            FilterType::Statevariable => {
                self.statevariable.set_cutoff(cutoff);
                self.statevariable.set_resonance(resonance);
                self.statevariable.process(input)
            }
            FilterType::Comb => {
                self.comb.set_cutoff(cutoff);
                self.comb.set_resonance(resonance);
                self.comb.process(input)
            }
            FilterType::RainbowComb => {
                self.rainbow_comb.set_cutoff(cutoff);
                self.rainbow_comb.set_resonance(resonance);
                self.rainbow_comb.process(input)
            }
            FilterType::DiodeLadderLp => {
                self.diode_ladder_lp.set_cutoff(cutoff);
                self.diode_ladder_lp.set_resonance(resonance);
                self.diode_ladder_lp.process(input)
            }
            FilterType::DiodeLadderHp => {
                self.diode_ladder_hp.set_cutoff(cutoff);
                self.diode_ladder_hp.set_resonance(resonance);
                self.diode_ladder_hp.process(input)
            }
            FilterType::Ms20Pair => {
                self.ms20.set_cutoff(cutoff);
                self.ms20.set_resonance(resonance);
                self.ms20.process(input)
            }
            FilterType::FormantMorph => {
                self.formant_morph.set_cutoff(cutoff);
                self.formant_morph.set_resonance(resonance);
                self.formant_morph.process(input)
            }
            FilterType::Phaser => {
                self.phaser.set_cutoff(cutoff);
                self.phaser.set_resonance(resonance);
                self.phaser.process(input)
            }
            FilterType::CombAllpass => {
                self.comb_allpass.set_cutoff(cutoff);
                self.comb_allpass.set_resonance(resonance);
                self.comb_allpass.process(input)
            }
            FilterType::BitcrushLp => {
                self.bitcrush_lp.set_cutoff(cutoff);
                self.bitcrush_lp.set_resonance(resonance);
                self.bitcrush_lp.process(input)
            }
        };

        if matches!(filter_type, FilterType::None) {
            out
        } else {
            filter::tame_resonance(out, resonance)
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
        a_cut: f32,
        a_res: f32,
        a_amt: f32,
        b_type: FilterType,
        b_cut: f32,
        b_res: f32,
        b_amt: f32,
        c_type: FilterType,
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

        let mut a_left = self.stage_a_left.apply(a_type, a_cut, a_res, left);
        let mut a_right = self.stage_a_right.apply(a_type, a_cut, a_res, right);
        a_left = left * (1.0 - a_amt) + a_left * a_amt;
        a_right = right * (1.0 - a_amt) + a_right * a_amt;

        let mut b_left = self.stage_b_left.apply(b_type, b_cut, b_res, left);
        let mut b_right = self.stage_b_right.apply(b_type, b_cut, b_res, right);
        b_left = left * (1.0 - b_amt) + b_left * b_amt;
        b_right = right * (1.0 - b_amt) + b_right * b_amt;

        let mut c_left = self.stage_c_left.apply(c_type, c_cut, c_res, left);
        let mut c_right = self.stage_c_right.apply(c_type, c_cut, c_res, right);
        c_left = left * (1.0 - c_amt) + c_left * c_amt;
        c_right = right * (1.0 - c_amt) + c_right * c_amt;

        match routing {
            FilterRouting::Serial => {
                let mut out_left = self.stage_a_left.apply(a_type, a_cut, a_res, left);
                let mut out_right = self.stage_a_right.apply(a_type, a_cut, a_res, right);
                out_left = left * (1.0 - a_amt) + out_left * a_amt;
                out_right = right * (1.0 - a_amt) + out_right * a_amt;

                out_left = self.stage_b_left.apply(b_type, b_cut, b_res, out_left);
                out_right = self.stage_b_right.apply(b_type, b_cut, b_res, out_right);
                out_left = left * (1.0 - b_amt) + out_left * b_amt;
                out_right = right * (1.0 - b_amt) + out_right * b_amt;

                out_left = self.stage_c_left.apply(c_type, c_cut, c_res, out_left);
                out_right = self.stage_c_right.apply(c_type, c_cut, c_res, out_right);
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
