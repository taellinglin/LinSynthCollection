use nih_plug::prelude::*;
use crate::envelope::*;
use crate::modulator::Modulator;
use crate::filter::{self, Filter, FilterType, FilterStyle};
use crate::resonator::ResonatorBank;
use crate::sub_synth::params::SubSynthBlockParams;
use crate::waveform::{WavetableBank, Waveform};
use crate::util;

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
    pub filter: Option<FilterType>,
    pub lowpass_filter: filter::LowpassFilter,
    pub highpass_filter: filter::HighpassFilter,
    pub bandpass_filter: filter::BandpassFilter,
    pub notch_filter: filter::NotchFilter,
    pub statevariable_filter: filter::StatevariableFilter,
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
    pub resonator: ResonatorBank,
    pub prng: rand_pcg::Pcg32,
}

impl Voice {
    pub fn render_sample(
        &mut self,
        bp: &SubSynthBlockParams,
        factory_wavetable: &WavetableBank,
        custom_wavetable: Option<&WavetableBank>,
        sample_rate: f32,
        gain_val: f32,
    ) -> (f32, f32) {
        let amp_env_value = self.amp_envelope.next_sample();
        let filter_cut_env_value = self.filter_cut_envelope.next_sample();
        let filter_res_env_value = self.filter_res_envelope.next_sample();

        if amp_env_value <= 0.0 && self.releasing {
            return (0.0, 0.0);
        }

        // Modulator updates
        let vib = self.vib_mod.next_sample(sample_rate);
        let lfo1 = self.mod_lfo1.next_sample(sample_rate);
        let lfo2 = self.mod_lfo2.next_sample(sample_rate);

        // Modulation Matrix Logic
        let mut filter_cut_mod = 0.0;
        let mut filter_res_mod = 0.0;
        let mut wavetable_pos_mod = 0.0;
        let mut pan_mod = 0.0;
        let mut gain_mod = 0.0;

        let mods = [
            (bp.mod1_source, bp.mod1_target, bp.mod1_amount),
            (bp.mod2_source, bp.mod2_target, bp.mod2_amount),
        ];

        for (src, tgt, amt) in mods {
            let val = match src {
                crate::common::ModSource::Lfo1 => lfo1,
                crate::common::ModSource::Lfo2 => lfo2,
                crate::common::ModSource::AmpEnv => amp_env_value,
                crate::common::ModSource::FilterEnv => filter_cut_env_value,
            };
            
            match tgt {
                crate::common::ModTarget::FilterCut => filter_cut_mod += val * amt,
                crate::common::ModTarget::FilterRes => filter_res_mod += val * amt,
                crate::common::ModTarget::WavetablePos => wavetable_pos_mod += val * amt,
                crate::common::ModTarget::Pan => pan_mod += val * amt,
                crate::common::ModTarget::Gain => gain_mod += val * amt,
            }
        }

        let drifted_phase_delta = self.phase_delta * (1.0 + vib * 0.02);
        
        let wavetable_bank = if bp.custom_wavetable_enable {
            custom_wavetable.unwrap_or(factory_wavetable)
        } else {
            factory_wavetable
        };

        let classic_sample = crate::waveform::generate_waveform(bp.waveform, self.phase);
        let wt_pos = (bp.wavetable_position + wavetable_pos_mod).clamp(0.0, 1.0);
        let wavetable_sample = wavetable_bank.sample(self.phase, wt_pos);

        let mut generated_sample = match bp.osc_routing {
            crate::common::OscRouting::ClassicOnly => classic_sample,
            crate::common::OscRouting::WavetableOnly => wavetable_sample,
            crate::common::OscRouting::Blend => classic_sample * (1.0 - bp.osc_blend) + wavetable_sample * bp.osc_blend,
        };

        if bp.sub_level > 0.0 {
            let sub_phase = (self.phase * 0.5).fract();
            generated_sample += (sub_phase * 2.0 * std::f32::consts::PI).sin() * bp.sub_level;
        }

        // Filtering
        let cutoff = (bp.filter_cut * (1.0 + filter_cut_mod + filter_cut_env_value)).clamp(20.0, 20000.0);
        let resonance = (bp.filter_res + filter_res_env_value).clamp(0.0, 1.0);
        
        let filtered_sample = match self.filter.unwrap_or(FilterType::None) {
            FilterType::Lowpass => {
                self.lowpass_filter.set_cutoff(cutoff);
                self.lowpass_filter.set_resonance(resonance);
                self.lowpass_filter.set_style(bp.filter_style);
                self.lowpass_filter.set_vintage_drive(bp.filter_vintage_drive);
                self.lowpass_filter.set_vintage_curve(bp.filter_vintage_curve);
                self.lowpass_filter.set_vintage_mix(bp.filter_vintage_mix);
                self.lowpass_filter.set_vintage_trim(bp.filter_vintage_trim);
                self.lowpass_filter.process(generated_sample)
            }
            FilterType::Highpass => {
                self.highpass_filter.set_cutoff(cutoff);
                self.highpass_filter.set_resonance(resonance);
                self.highpass_filter.set_style(bp.filter_style);
                self.highpass_filter.set_vintage_drive(bp.filter_vintage_drive);
                self.highpass_filter.set_vintage_curve(bp.filter_vintage_curve);
                self.highpass_filter.set_vintage_mix(bp.filter_vintage_mix);
                self.highpass_filter.set_vintage_trim(bp.filter_vintage_trim);
                self.highpass_filter.process(generated_sample)
            }
            FilterType::Bandpass => {
                self.bandpass_filter.set_cutoff(cutoff);
                self.bandpass_filter.set_resonance(resonance);
                self.bandpass_filter.set_style(bp.filter_style);
                self.bandpass_filter.set_vintage_drive(bp.filter_vintage_drive);
                self.bandpass_filter.set_vintage_curve(bp.filter_vintage_curve);
                self.bandpass_filter.set_vintage_mix(bp.filter_vintage_mix);
                self.bandpass_filter.set_vintage_trim(bp.filter_vintage_trim);
                self.bandpass_filter.process(generated_sample)
            }
            FilterType::Notch => {
                self.notch_filter.set_cutoff(cutoff);
                self.notch_filter.set_resonance(resonance);
                self.notch_filter.set_style(bp.filter_style);
                self.notch_filter.set_vintage_drive(bp.filter_vintage_drive);
                self.notch_filter.set_vintage_curve(bp.filter_vintage_curve);
                self.notch_filter.set_vintage_mix(bp.filter_vintage_mix);
                self.notch_filter.set_vintage_trim(bp.filter_vintage_trim);
                self.notch_filter.process(generated_sample)
            }
            FilterType::Statevariable => {
                self.statevariable_filter.set_cutoff(cutoff);
                self.statevariable_filter.set_resonance(resonance);
                self.statevariable_filter.set_style(bp.filter_style);
                self.statevariable_filter.set_vintage_drive(bp.filter_vintage_drive);
                self.statevariable_filter.set_vintage_curve(bp.filter_vintage_curve);
                self.statevariable_filter.set_vintage_mix(bp.filter_vintage_mix);
                self.statevariable_filter.set_vintage_trim(bp.filter_vintage_trim);
                self.statevariable_filter.process(generated_sample)
            }
            FilterType::None => generated_sample,
        };

        let mut final_sample = generated_sample * (1.0 - bp.filter_amount) + filtered_sample * bp.filter_amount;

        if bp.resonator_enable {
            let mix = bp.resonator_mix.clamp(0.0, 1.0);
            if mix > 0.0 {
                let base_freq = util::midi_note_to_freq(self.note) * (2.0_f32).powf(self.tuning / 12.0);
                self.resonator.set_sample_rate(sample_rate);
                self.resonator.set_base_freq(base_freq);
                self.resonator.set_tone(bp.resonator_tone);
                self.resonator.set_shape(bp.resonator_shape);
                self.resonator.set_timbre(bp.resonator_timbre);
                self.resonator.set_damping(bp.resonator_damping);
                let resonated = self.resonator.process(final_sample);
                final_sample = final_sample * (1.0 - mix) + resonated * mix;
            }
        }

        let amp = self.velocity_sqrt * gain_val * (amp_env_value * bp.amp_envelope_level) * 0.5 * (self.trem_mod.next_sample(sample_rate) + 1.0);
        
        let corrected = final_sample - crate::sub_synth::SubSynth::poly_blep(self.phase, drifted_phase_delta);
        let mut processed = corrected * amp;
        
        if bp.analog_enable {
            if bp.analog_noise > 0.0 {
                use rand::Rng;
                let noise: f32 = self.prng.gen_range(-1.0..1.0);
                processed += noise * bp.analog_noise * amp_env_value;
            }
            if bp.analog_drive > 0.0 {
                let drive = 1.0 + bp.analog_drive * 6.0;
                processed = (processed * drive).tanh() / drive;
            }
        }

        let pan = self.pan.clamp(0.0, 1.0);
        let left_amp = (1.0 - pan).sqrt();
        let right_amp = pan.sqrt();

        self.phase = (self.phase + drifted_phase_delta).fract();

        (processed * left_amp, processed * right_amp)
    }
}
