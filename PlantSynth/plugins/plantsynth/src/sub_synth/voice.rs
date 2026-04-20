use nih_plug::prelude::*;
use crate::envelope::*;
use crate::modulator::Modulator;
use crate::filter::{self, Filter, FilterType};
use crate::sub_synth::params::SubSynthBlockParams;
use crate::waveform::{WavetableBank, Waveform};

#[derive(Debug, Clone)]
pub struct Voice {
    pub voice_id: i32,
    pub channel: u8,
    pub note: u8,
    pub internal_voice_id: u64,
    pub velocity: f32,
    pub velocity_sqrt: f32,
    pub phase: f32,
    pub phase_delta: f32,
    pub target_phase_delta: f32,
    pub releasing: bool,
    pub amp_envelope: ADSREnvelope,
    pub voice_gain: Option<(f32, Smoother<f32>)>,
    pub filter_cut_envelope: ADSREnvelope,
    pub filter_res_envelope: ADSREnvelope,
    pub fm_envelope: ADSREnvelope,
    pub dist_envelope: ADSREnvelope,
    pub filter: Option<FilterType>,
    pub lowpass_filter: filter::LowpassFilter,
    pub highpass_filter: filter::HighpassFilter,
    pub bandpass_filter: filter::BandpassFilter,
    pub notch_filter: filter::NotchFilter,
    pub statevariable_filter: filter::StatevariableFilter,
    pub comb_filter: filter::CombFilter,
    pub rainbow_comb_filter: filter::RainbowCombFilter,
    pub diode_ladder_lp_filter: filter::DiodeLadderFilter,
    pub diode_ladder_hp_filter: filter::DiodeLadderFilter,
    pub ms20_filter: filter::Ms20Filter,
    pub formant_morph_filter: filter::FormantMorphFilter,
    pub phaser_filter: filter::PhaserFilter,
    pub comb_allpass_filter: filter::CombAllpassFilter,
    pub bitcrush_lp_filter: filter::BitcrushLpFilter,
    pub pressure: f32,
    pub pan: f32,
    pub tuning: f32,
    pub vibrato: f32,
    pub expression: f32,
    pub brightness: f32,
    pub vib_mod: Modulator,
    pub trem_mod: Modulator,
    pub pan_mod: Modulator,
    pub mod_lfo1: Modulator,
    pub mod_lfo2: Modulator,
    pub drift_offset: f32,
    pub mod_smooth: [f32; 6],
    pub fm_feedback_state: f32,
    pub unison_phases: [f32; 6],
    pub stereo_prev: f32,
    pub dc_blocker: filter::DCBlocker,
    pub prng: rand_pcg::Pcg32,
}

impl Voice {
    pub fn render_sample(
        &mut self,
        bp: &SubSynthBlockParams,
        factory_wavetable: &WavetableBank,
        custom_wavetable: Option<&WavetableBank>,
        sample_rate: f32,
        seq_gate: f32,
        seq_cut: f32,
        seq_res: f32,
        seq_wt: f32,
        seq_dist: f32,
        seq_fm: f32,
    ) -> (f32, f32) {
        let amp = self.amp_envelope.next_sample() * self.velocity * seq_gate;
        if amp <= 0.0 && self.releasing {
            return (0.0, 0.0);
        }

        let filter_cut_env = self.filter_cut_envelope.next_sample() + seq_cut;
        let filter_res_env = self.filter_res_envelope.next_sample() + seq_res;
        let fm_env = self.fm_envelope.next_sample() + seq_fm;
        let dist_env = self.dist_envelope.next_sample() + seq_dist;

        // Modulator updates
        let vib = self.vib_mod.next_sample(sample_rate);
        let trem = self.trem_mod.next_sample(sample_rate);
        let lfo1 = self.mod_lfo1.next_sample(sample_rate);
        let lfo2 = self.mod_lfo2.next_sample(sample_rate);

        // Modulation Matrix Logic
        let mut filter_cut_mod = 0.0;
        let mut filter_res_mod = 0.0;
        let mut filter_amt_mod = 0.0;
        let mut wavetable_pos_mod = 0.0;
        let mut pan_mod = 0.0;
        let mut gain_mod = 0.0;
        let mut fm_amt_mod = 0.0;
        let mut fm_ratio_mod = 0.0;
        let mut fm_fb_mod = 0.0;

        let mods = [
            (bp.mod1_source, bp.mod1_target, bp.mod1_amount),
            (bp.mod2_source, bp.mod2_target, bp.mod2_amount),
            (bp.mod3_source, bp.mod3_target, bp.mod3_amount),
            (bp.mod4_source, bp.mod4_target, bp.mod4_amount),
            (bp.mod5_source, bp.mod5_target, bp.mod5_amount),
            (bp.mod6_source, bp.mod6_target, bp.mod6_amount),
        ];

        for (src, tgt, amt) in mods {
            let val = match src {
                crate::common::ModSource::Lfo1 => lfo1,
                crate::common::ModSource::Lfo2 => lfo2,
                crate::common::ModSource::AmpEnv => amp / self.velocity,
                crate::common::ModSource::FilterEnv => filter_cut_env,
            };
            
            match tgt {
                crate::common::ModTarget::FilterCut => filter_cut_mod += val * amt,
                crate::common::ModTarget::FilterRes => filter_res_mod += val * amt,
                crate::common::ModTarget::FilterAmount => filter_amt_mod += val * amt,
                crate::common::ModTarget::WavetablePos => wavetable_pos_mod += val * amt,
                crate::common::ModTarget::Pan => pan_mod += val * amt,
                crate::common::ModTarget::Gain => gain_mod += val * amt,
                crate::common::ModTarget::FmAmount => fm_amt_mod += val * amt,
                crate::common::ModTarget::FmRatio => fm_ratio_mod += val * amt,
                crate::common::ModTarget::FmFeedback => fm_fb_mod += val * amt,
            }
        }
        
        wavetable_pos_mod += seq_wt;
        
        let mut phase_delta = self.phase_delta;
        if phase_delta < self.target_phase_delta {
            phase_delta = (phase_delta + bp.glide_time_ms / sample_rate).min(self.target_phase_delta);
        } else if phase_delta > self.target_phase_delta {
            phase_delta = (phase_delta - bp.glide_time_ms / sample_rate).max(self.target_phase_delta);
        }
        self.phase_delta = phase_delta;

        let base_phase = self.phase;
        let base_phase_delta = phase_delta * (1.0 + vib * 0.02);
        
        let unison_count = match bp.unison_voices {
            crate::common::UnisonVoices::One => 1,
            crate::common::UnisonVoices::Two => 2,
            crate::common::UnisonVoices::Four => 4,
            crate::common::UnisonVoices::Six => 6,
        };

        let mut classic_sum = 0.0;
        let mut wavetable_sum = 0.0;
        
        let wavetable_bank = if bp.custom_wavetable_enable {
            custom_wavetable.unwrap_or(factory_wavetable)
        } else {
            factory_wavetable
        };

        for i in 0..unison_count {
            let detune = (i as f32 - (unison_count - 1) as f32 * 0.5) * bp.unison_detune * 0.01;
            let ratio = 1.0 + detune;
            let phase = self.unison_phases[i];
            
            let mut classic_sample = crate::waveform::generate_waveform(bp.waveform, phase);
            classic_sample -= crate::sub_synth::SubSynth::poly_blep(phase, base_phase_delta * ratio);
            
            let wt_pos = (bp.wavetable_position + wavetable_pos_mod).clamp(0.0, 1.0);
            let mut wavetable_sample = wavetable_bank.sample(phase, wt_pos);
            
            classic_sum += classic_sample;
            wavetable_sum += wavetable_sample;
            
            let next_phase = phase + base_phase_delta * ratio;
            self.unison_phases[i] = if next_phase >= 1.0 { next_phase - 1.0 } else { next_phase };
        }
        
        let classic_sum = classic_sum / unison_count as f32;
        let wavetable_sum = wavetable_sum / unison_count as f32;
        
        let mut generated = match bp.osc_routing {
            crate::common::OscRouting::ClassicOnly => classic_sum,
            crate::common::OscRouting::WavetableOnly => wavetable_sum,
            crate::common::OscRouting::Blend => classic_sum * (1.0 - bp.osc_blend) + wavetable_sum * bp.osc_blend,
        };

        if bp.sub_level > 0.0 {
            let sub_phase = (self.phase * 0.5).fract();
            generated += (sub_phase * 2.0 * std::f32::consts::PI).sin() * bp.sub_level;
        }

        self.phase = (self.phase + base_phase_delta).fract();

        // Filtering
        let cutoff = (bp.filter_cut * (1.0 + filter_cut_mod + filter_cut_env)).clamp(20.0, 20000.0);
        let resonance = (bp.filter_res + filter_res_env).clamp(0.0, 1.0);
        
        let filtered = match self.filter.unwrap_or(FilterType::None) {
            FilterType::Lowpass => {
                self.lowpass_filter.set_cutoff(cutoff);
                self.lowpass_filter.set_resonance(resonance);
                self.lowpass_filter.process(generated)
            }
            FilterType::Highpass => {
                self.highpass_filter.set_cutoff(cutoff);
                self.highpass_filter.set_resonance(resonance);
                self.highpass_filter.process(generated)
            }
            FilterType::Bandpass => {
                self.bandpass_filter.set_cutoff(cutoff);
                self.bandpass_filter.set_resonance(resonance);
                self.bandpass_filter.process(generated)
            }
            FilterType::Notch => {
                self.notch_filter.set_cutoff(cutoff);
                self.notch_filter.set_resonance(resonance);
                self.notch_filter.process(generated)
            }
            FilterType::Statevariable => {
                self.statevariable_filter.set_cutoff(cutoff);
                self.statevariable_filter.set_resonance(resonance);
                self.statevariable_filter.process(generated)
            }
            FilterType::Comb => {
                self.comb_filter.set_cutoff(cutoff);
                self.comb_filter.set_resonance(resonance);
                self.comb_filter.process(generated)
            }
            FilterType::RainbowComb => {
                self.rainbow_comb_filter.set_cutoff(cutoff);
                self.rainbow_comb_filter.set_resonance(resonance);
                self.rainbow_comb_filter.process(generated)
            }
            FilterType::DiodeLadderLp => {
                self.diode_ladder_lp_filter.set_cutoff(cutoff);
                self.diode_ladder_lp_filter.set_resonance(resonance);
                self.diode_ladder_lp_filter.process(generated)
            }
            FilterType::DiodeLadderHp => {
                self.diode_ladder_hp_filter.set_cutoff(cutoff);
                self.diode_ladder_hp_filter.set_resonance(resonance);
                self.diode_ladder_hp_filter.process(generated)
            }
            FilterType::Ms20Pair => {
                self.ms20_filter.set_cutoff(cutoff);
                self.ms20_filter.set_resonance(resonance);
                self.ms20_filter.process(generated)
            }
            FilterType::FormantMorph => {
                self.formant_morph_filter.set_cutoff(cutoff);
                self.formant_morph_filter.set_resonance(resonance);
                self.formant_morph_filter.process(generated)
            }
            FilterType::Phaser => {
                self.phaser_filter.set_cutoff(cutoff);
                self.phaser_filter.set_resonance(resonance);
                self.phaser_filter.process(generated)
            }
            FilterType::CombAllpass => {
                self.comb_allpass_filter.set_cutoff(cutoff);
                self.comb_allpass_filter.set_resonance(resonance);
                self.comb_allpass_filter.process(generated)
            }
            FilterType::BitcrushLp => {
                self.bitcrush_lp_filter.set_cutoff(cutoff);
                self.bitcrush_lp_filter.set_resonance(resonance);
                self.bitcrush_lp_filter.process(generated)
            }
            FilterType::None => generated,
        };

        let mut output_sample = filtered * amp * trem * (1.0 + gain_mod);
        
        if bp.analog_enable {
            if bp.analog_noise > 0.0 {
                use rand::Rng;
                let noise: f32 = self.prng.gen_range(-1.0..1.0);
                output_sample += noise * bp.analog_noise * amp;
            }
            if bp.analog_drive > 0.0 {
                let drive = 1.0 + bp.analog_drive * 6.0;
                output_sample = (output_sample * drive).tanh() / drive;
            }
        }

        let pan = (self.pan + pan_mod + self.pan_mod.next_sample(sample_rate) * 0.5).clamp(0.0, 1.0);
        
        (output_sample * (1.0 - pan), output_sample * pan)
    }
}
