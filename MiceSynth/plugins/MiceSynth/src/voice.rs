use nih_plug::prelude::Smoother;
use crate::modulator::Modulator;
use crate::envelope::ADSREnvelope;
use crate::filter;
use crate::filter::FilterType;
use crate::eq::Biquad;

#[derive(Debug, Clone)]
pub(crate) struct Voice {
    pub(crate) voice_id: i32,
    pub(crate) channel: u8,
    pub(crate) note: u8,
    pub(crate) internal_voice_id: u64,
    pub(crate) velocity: f32,
    pub(crate) velocity_sqrt: f32,
    pub(crate) phase: f32,
    pub(crate) phase_delta: f32,
    pub(crate) target_phase_delta: f32,
    pub(crate) releasing: bool,
    pub(crate) amp_envelope: ADSREnvelope,
    pub(crate) voice_gain: Option<(f32, Smoother<f32>)>,
    pub(crate) filter_cut_envelope: ADSREnvelope,
    pub(crate) filter_res_envelope: ADSREnvelope,
    pub(crate) fm_envelope: ADSREnvelope,
    pub(crate) dist_envelope: ADSREnvelope,
    pub(crate) breath_envelope: ADSREnvelope,
    pub(crate) filter: Option<FilterType>,
    pub(crate) lowpass_filter: filter::LowpassFilter,
    pub(crate) highpass_filter: filter::HighpassFilter,
    pub(crate) bandpass_filter: filter::BandpassFilter,
    pub(crate) notch_filter: filter::NotchFilter,
    pub(crate) statevariable_filter: filter::StatevariableFilter,
    pub(crate) comb_filter: filter::CombFilter,
    pub(crate) rainbow_comb_filter: filter::RainbowCombFilter,
    pub(crate) diode_ladder_lp_filter: filter::DiodeLadderFilter,
    pub(crate) diode_ladder_hp_filter: filter::DiodeLadderFilter,
    pub(crate) ms20_filter: filter::Ms20Filter,
    pub(crate) formant_morph_filter: filter::FormantMorphFilter,
    pub(crate) phaser_filter: filter::PhaserFilter,
    pub(crate) comb_allpass_filter: filter::CombAllpassFilter,
    pub(crate) bitcrush_lp_filter: filter::BitcrushLpFilter,
    pub(crate) breath_filter: filter::BandpassFilter,
    pub(crate) pressure: f32,
    pub(crate) pan: f32,        // Added pan field
    pub(crate) tuning: f32,     // Add tuning field
    pub(crate) vibrato: f32,    // Add vibrato field
    pub(crate) expression: f32, // Add expression field
    pub(crate) brightness: f32, // Add brightness field
    pub(crate) vib_mod: Modulator,
    pub(crate) trem_mod: Modulator,
    pub(crate) pan_mod: Modulator,
    pub(crate) mod_lfo1: Modulator,
    pub(crate) mod_lfo2: Modulator,
    pub(crate) drift_offset: f32,
    pub(crate) mod_smooth: [f32; 6],
    pub(crate) fm_feedback_state: f32,
    pub(crate) unison_phases: [f32; 6],
    pub(crate) stereo_prev: f32,
    pub(crate) dc_blocker: filter::DCBlocker,
}

