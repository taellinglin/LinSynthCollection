use enum_iterator::Sequence;
use crate::GAIN_POLY_MOD_ID;
use nih_plug::params::enums::Enum;
use nih_plug::params::enums::EnumParam;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, RwLock};

use crate::editor;
use crate::modulator::OscillatorShape;
use crate::waveform::Waveform;
use crate::filter::FilterType;
use crate::output_saturation::OutputSaturationType;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum OscRouting {
    ClassicOnly,
    WavetableOnly,
    Blend,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum ModSource {
    Lfo1,
    Lfo2,
    AmpEnv,
    FilterEnv,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum ModTarget {
    WavetablePos,
    FilterCut,
    FilterRes,
    #[name = "Filter Amount"]
    FilterAmount,
    Pan,
    Gain,
    #[name = "FM Amount"]
    FmAmount,
    #[name = "FM Ratio"]
    FmRatio,
    #[name = "FM Feedback"]
    FmFeedback,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum FmSource {
    Classic,
    Wavetable,
    Sub,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum FmTarget {
    Classic,
    Wavetable,
    Both,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum GlideMode {
    Off,
    Legato,
    Always,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum UnisonVoices {
    One,
    Two,
    Four,
    Six,
}

#[derive(Params)]
pub(crate) struct SeqStepParams {
    #[id = "val"]
    pub(crate) value: FloatParam,
}

impl Default for SeqStepParams {
    fn default() -> Self {
        Self {
            value: FloatParam::new(
                "Step",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
        }
    }
}

#[derive(Params)]
pub(crate) struct SeqLaneParams {
    #[nested(array)]
    pub steps: [SeqStepParams; 32],
}

impl Default for SeqLaneParams {
    fn default() -> Self {
        Self {
            steps: std::array::from_fn(|_| SeqStepParams::default()),
        }
    }
}

pub(crate) const SEQ_LANE_COUNT: usize = 6;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum FilterRouting {
    Serial,
    Parallel,
    Morph,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum RingModPlacement {
    PreFilter,
    PostFilter,
    PostFx,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum RingModSource {
    Sine,
    Classic,
    Wavetable,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub(crate) enum SpectralPlacement {
    PreFx,
    PreDist,
    PostFx,
}

#[derive(Params)]
pub(crate) struct SubSynthParams {
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,
    #[persist = "custom_wt_path"]
    pub(crate) custom_wavetable_path: Arc<RwLock<Option<String>>>,
    pub(crate) custom_wavetable_data: Arc<RwLock<Option<Vec<f32>>>>,
    #[id = "preset"]
    pub(crate) preset_index: IntParam,
    #[id = "gain"]
    pub(crate) gain: FloatParam,
    #[id = "amp_atk"]
    pub(crate) amp_attack_ms: FloatParam,
    #[id = "amp_hold"]
    pub(crate) amp_hold_ms: FloatParam,
    #[id = "amp_rel"]
    pub(crate) amp_release_ms: FloatParam,
    #[id = "amp_tension"]
    pub(crate) amp_tension: FloatParam,
    #[id = "waveform"]
    pub(crate) waveform: EnumParam<Waveform>,
    #[id = "osc_route"]
    pub(crate) osc_routing: EnumParam<OscRouting>,
    #[id = "osc_blend"]
    pub(crate) osc_blend: FloatParam,
    #[id = "wt_pos"]
    pub(crate) wavetable_position: FloatParam,
    #[id = "wt_dist"]
    pub(crate) wavetable_distortion: FloatParam,
    #[id = "classic_drive"]
    pub(crate) classic_drive: FloatParam,
    #[id = "wt_custom"]
    pub(crate) custom_wavetable_enable: BoolParam,
    #[id = "analog_en"]
    pub(crate) analog_enable: BoolParam,
    #[id = "analog_drive"]
    pub(crate) analog_drive: FloatParam,
    #[id = "analog_noise"]
    pub(crate) analog_noise: FloatParam,
    #[id = "analog_drift"]
    pub(crate) analog_drift: FloatParam,
    #[id = "sub_level"]
    pub(crate) sub_level: FloatParam,
    #[id = "mix_classic"]
    pub(crate) classic_level: FloatParam,
    #[id = "mix_wt"]
    pub(crate) wavetable_level: FloatParam,
    #[id = "mix_noise"]
    pub(crate) noise_level: FloatParam,
    #[id = "send_classic"]
    pub(crate) classic_send: FloatParam,
    #[id = "send_wt"]
    pub(crate) wavetable_send: FloatParam,
    #[id = "send_sub"]
    pub(crate) sub_send: FloatParam,
    #[id = "send_noise"]
    pub(crate) noise_send: FloatParam,
    #[id = "send_ring"]
    pub(crate) ring_mod_send: FloatParam,
    #[id = "fx_bus_mix"]
    pub(crate) fx_bus_mix: FloatParam,
    #[id = "ring_en"]
    pub(crate) ring_mod_enable: BoolParam,
    #[id = "ring_src"]
    pub(crate) ring_mod_source: EnumParam<RingModSource>,
    #[id = "ring_freq"]
    pub(crate) ring_mod_freq: FloatParam,
    #[id = "ring_mix"]
    pub(crate) ring_mod_mix: FloatParam,
    #[id = "ring_level"]
    pub(crate) ring_mod_level: FloatParam,
    #[id = "ring_place"]
    pub(crate) ring_mod_placement: EnumParam<RingModPlacement>,
    #[id = "sizzle_osc"]
    pub(crate) sizzle_osc_enable: BoolParam,
    #[id = "sizzle_wt"]
    pub(crate) sizzle_wt_enable: BoolParam,
    #[id = "sizzle_dist"]
    pub(crate) sizzle_dist_enable: BoolParam,
    #[id = "sizzle_cut"]
    pub(crate) sizzle_cutoff: FloatParam,
    #[id = "spec_en"]
    pub(crate) spectral_enable: BoolParam,
    #[id = "spec_amt"]
    pub(crate) spectral_amount: FloatParam,
    #[id = "spec_tilt"]
    pub(crate) spectral_tilt: FloatParam,
    #[id = "spec_formant"]
    pub(crate) spectral_formant: FloatParam,
    #[id = "spec_place"]
    pub(crate) spectral_placement: EnumParam<SpectralPlacement>,
    #[id = "filter_tight"]
    pub(crate) filter_tight_enable: BoolParam,
    #[id = "unison_voices"]
    pub(crate) unison_voices: EnumParam<UnisonVoices>,
    #[id = "unison_detune"]
    pub(crate) unison_detune: FloatParam,
    #[id = "unison_spread"]
    pub(crate) unison_spread: FloatParam,
    #[id = "glide_mode"]
    pub(crate) glide_mode: EnumParam<GlideMode>,
    #[id = "glide_time"]
    pub(crate) glide_time_ms: FloatParam,
    #[id = "lfo1_rate"]
    pub(crate) lfo1_rate: FloatParam,
    #[id = "lfo1_atk"]
    pub(crate) lfo1_attack: FloatParam,
    #[id = "lfo1_shape"]
    pub(crate) lfo1_shape: EnumParam<OscillatorShape>,
    #[id = "lfo2_rate"]
    pub(crate) lfo2_rate: FloatParam,
    #[id = "lfo2_atk"]
    pub(crate) lfo2_attack: FloatParam,
    #[id = "lfo2_shape"]
    pub(crate) lfo2_shape: EnumParam<OscillatorShape>,
    #[id = "mod1_src"]
    pub(crate) mod1_source: EnumParam<ModSource>,
    #[id = "mod1_tgt"]
    pub(crate) mod1_target: EnumParam<ModTarget>,
    #[id = "mod1_amt"]
    pub(crate) mod1_amount: FloatParam,
        #[id = "mod1_smooth"]
    pub(crate)     mod1_smooth_ms: FloatParam,
    #[id = "mod2_src"]
    pub(crate) mod2_source: EnumParam<ModSource>,
    #[id = "mod2_tgt"]
    pub(crate) mod2_target: EnumParam<ModTarget>,
    #[id = "mod2_amt"]
    pub(crate) mod2_amount: FloatParam,
    #[id = "mod2_smooth"]
    pub(crate) mod2_smooth_ms: FloatParam,
    #[id = "mod3_src"]
    pub(crate) mod3_source: EnumParam<ModSource>,
    #[id = "mod3_tgt"]
    pub(crate) mod3_target: EnumParam<ModTarget>,
    #[id = "mod3_amt"]
    pub(crate) mod3_amount: FloatParam,
    #[id = "mod3_smooth"]
    pub(crate) mod3_smooth_ms: FloatParam,
    #[id = "mod4_src"]
    pub(crate) mod4_source: EnumParam<ModSource>,
    #[id = "mod4_tgt"]
    pub(crate) mod4_target: EnumParam<ModTarget>,
    #[id = "mod4_amt"]
    pub(crate) mod4_amount: FloatParam,
    #[id = "mod4_smooth"]
    pub(crate) mod4_smooth_ms: FloatParam,
    #[id = "mod5_src"]
    pub(crate) mod5_source: EnumParam<ModSource>,
    #[id = "mod5_tgt"]
    pub(crate) mod5_target: EnumParam<ModTarget>,
    #[id = "mod5_amt"]
    pub(crate) mod5_amount: FloatParam,
    #[id = "mod5_smooth"]
    pub(crate) mod5_smooth_ms: FloatParam,
    #[id = "mod6_src"]
    pub(crate) mod6_source: EnumParam<ModSource>,
    #[id = "mod6_tgt"]
    pub(crate) mod6_target: EnumParam<ModTarget>,
    #[id = "mod6_amt"]
    pub(crate) mod6_amount: FloatParam,
    #[id = "mod6_smooth"]
    pub(crate) mod6_smooth_ms: FloatParam,
    #[id = "seq_enable"]
    pub(crate) seq_enable: BoolParam,
    #[id = "seq_rate"]
    pub(crate) seq_rate: FloatParam,
    #[id = "seq_gate_amt"]
    pub(crate) seq_gate_amount: FloatParam,
    #[id = "seq_cut_amt"]
    pub(crate) seq_cut_amount: FloatParam,
    #[id = "seq_res_amt"]
    pub(crate) seq_res_amount: FloatParam,
    #[id = "seq_wt_amt"]
    pub(crate) seq_wt_amount: FloatParam,
    #[id = "seq_dist_amt"]
    pub(crate) seq_dist_amount: FloatParam,
    #[id = "seq_fm_amt"]
    pub(crate) seq_fm_amount: FloatParam,
    #[nested(array, group = "Sequencer")]
    pub(crate) seq_lanes: [SeqLaneParams; SEQ_LANE_COUNT],

    // New parameters for ADSR envelope
    #[id = "amp_dec"]
    pub(crate) amp_decay_ms: FloatParam,
    #[id = "amp_dec2"]
    pub(crate) amp_decay2_ms: FloatParam,
    #[id = "amp_dec2_lvl"]
    pub(crate) amp_decay2_level: FloatParam,
    #[id = "amp_sus"]
    pub(crate) amp_sustain_level: FloatParam,
    #[id = "filter_cut_atk"]
    pub(crate) filter_cut_attack_ms: FloatParam,
    #[id = "filter_cut_hold"]
    pub(crate) filter_cut_hold_ms: FloatParam,
    #[id = "filter_cut_dec"]
    pub(crate) filter_cut_decay_ms: FloatParam,
    #[id = "filter_cut_dec2"]
    pub(crate) filter_cut_decay2_ms: FloatParam,
    #[id = "filter_cut_dec2_lvl"]
    pub(crate) filter_cut_decay2_level: FloatParam,
    #[id = "filter_cut_sus"]
    pub(crate) filter_cut_sustain_ms: FloatParam,
    #[id = "filter_cut_rel"]
    pub(crate) filter_cut_release_ms: FloatParam,
    #[id = "filter_res_atk"]
    pub(crate) filter_res_attack_ms: FloatParam,
    #[id = "filter_res_hold"]
    pub(crate) filter_res_hold_ms: FloatParam,
    #[id = "filter_res_dec"]
    pub(crate) filter_res_decay_ms: FloatParam,
    #[id = "filter_res_dec2"]
    pub(crate) filter_res_decay2_ms: FloatParam,
    #[id = "filter_res_dec2_lvl"]
    pub(crate) filter_res_decay2_level: FloatParam,
    #[id = "filter_res_sus"]
    pub(crate) filter_res_sustain_ms: FloatParam,
    #[id = "filter_res_rel"]
    pub(crate) filter_res_release_ms: FloatParam,
    #[id = "filter_type"]
    pub(crate) filter_type: EnumParam<FilterType>,
    #[id = "filter_cut"]
    pub(crate) filter_cut: FloatParam,
    #[id = "filter_res"]
    pub(crate) filter_res: FloatParam,
    #[id = "filter_amount"]
    pub(crate) filter_amount: FloatParam,
    // New parameters for ADSR envelope levels
    #[id = "amp_env_level"]
    pub(crate) amp_envelope_level: FloatParam,
    #[id = "filter_cut_env_level"]
    pub(crate) filter_cut_envelope_level: FloatParam,
    #[id = "filter_res_env_level"]
    pub(crate) filter_res_envelope_level: FloatParam,
    #[id = "fm_enable"]
    pub(crate) fm_enable: BoolParam,
    #[id = "fm_source"]
    pub(crate) fm_source: EnumParam<FmSource>,
    #[id = "fm_target"]
    pub(crate) fm_target: EnumParam<FmTarget>,
    #[id = "fm_amount"]
    pub(crate) fm_amount: FloatParam,
    #[id = "fm_ratio"]
    pub(crate) fm_ratio: FloatParam,
    #[id = "fm_feedback"]
    pub(crate) fm_feedback: FloatParam,
    #[id = "fm_env_atk"]
    pub(crate) fm_env_attack_ms: FloatParam,
    #[id = "fm_env_hold"]
    pub(crate) fm_env_hold_ms: FloatParam,
    #[id = "fm_env_dec"]
    pub(crate) fm_env_decay_ms: FloatParam,
    #[id = "fm_env_dec2"]
    pub(crate) fm_env_decay2_ms: FloatParam,
    #[id = "fm_env_dec2_lvl"]
    pub(crate) fm_env_decay2_level: FloatParam,
    #[id = "fm_env_sus"]
    pub(crate) fm_env_sustain_level: FloatParam,
    #[id = "fm_env_rel"]
    pub(crate) fm_env_release_ms: FloatParam,
    #[id = "fm_env_amt"]
    pub(crate) fm_env_amount: FloatParam,
    #[id = "vibrato_atk"]
    pub(crate) vibrato_attack: FloatParam,
    #[id = "vibrato_int"]
    pub(crate) vibrato_intensity: FloatParam,
    #[id = "vibrato_rate"]
    pub(crate) vibrato_rate: FloatParam,
    #[id = "tremolo_atk"]
    pub(crate) tremolo_attack: FloatParam,
    #[id = "tremolo_int"]
    pub(crate) tremolo_intensity: FloatParam,
    #[id = "tremolo_rate"]
    pub(crate) tremolo_rate: FloatParam,
    #[id = "vibrato_shape"]
    pub(crate) vibrato_shape: EnumParam<OscillatorShape>,
    #[id = "tremolo_shape"]
    pub(crate) tremolo_shape: EnumParam<OscillatorShape>,
    #[id = "filter_cut_env_pol"]
    pub(crate) filter_cut_env_polarity: BoolParam,
    #[id = "filter_res_env_pol"]
    pub(crate) filter_res_env_polarity: BoolParam,
    #[id = "filter_cut_tension"]
    pub(crate) filter_cut_tension: FloatParam,
    #[id = "filter_res_tension"]
    pub(crate) filter_res_tension: FloatParam,
    #[id = "cutoff_lfo_attack"]
    pub(crate) cutoff_lfo_attack: FloatParam,
    #[id = "res_lfo_attack"]
    pub(crate) res_lfo_attack: FloatParam,
    #[id = "pan_lfo_attack"]
    pub(crate) pan_lfo_attack: FloatParam,
    #[id = "cutoff_lfo_int"]
    pub(crate) cutoff_lfo_intensity: FloatParam,
    #[id = "cutoff_lfo_rate"]
    pub(crate) cutoff_lfo_rate: FloatParam,
    #[id = "cutoff_lfo_shape"]
    pub(crate) cutoff_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "res_lfo_int"]
    pub(crate) res_lfo_intensity: FloatParam,
    #[id = "res_lfo_rate"]
    pub(crate) res_lfo_rate: FloatParam,
    #[id = "res_lfo_shape"]
    pub(crate) res_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "pan_lfo_int"]
    pub(crate) pan_lfo_intensity: FloatParam,
    #[id = "pan_lfo_rate"]
    pub(crate) pan_lfo_rate: FloatParam,
    #[id = "pan_lfo_shape"]
    pub(crate) pan_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "chorus_enable"]
    pub(crate) chorus_enable: BoolParam,
    #[id = "chorus_rate"]
    pub(crate) chorus_rate: FloatParam,
    #[id = "chorus_depth"]
    pub(crate) chorus_depth: FloatParam,
    #[id = "chorus_mix"]
    pub(crate) chorus_mix: FloatParam,
    #[id = "delay_en"]
    pub(crate) delay_enable: BoolParam,
    #[id = "delay_time"]
    pub(crate) delay_time_ms: FloatParam,
    #[id = "delay_fb"]
    pub(crate) delay_feedback: FloatParam,
    #[id = "delay_mix"]
    pub(crate) delay_mix: FloatParam,
    #[id = "rev_en"]
    pub(crate) reverb_enable: BoolParam,
    #[id = "rev_size"]
    pub(crate) reverb_size: FloatParam,
    #[id = "rev_damp"]
    pub(crate) reverb_damp: FloatParam,
    #[id = "rev_diff"]
    pub(crate) reverb_diffusion: FloatParam,
    #[id = "rev_shim"]
    pub(crate) reverb_shimmer: FloatParam,
    #[id = "rev_mix"]
    pub(crate) reverb_mix: FloatParam,
    #[id = "dist_en"]
    pub(crate) dist_enable: BoolParam,
    #[id = "dist_drive"]
    pub(crate) dist_drive: FloatParam,
    #[id = "dist_tone"]
    pub(crate) dist_tone: FloatParam,
    #[id = "dist_magic"]
    pub(crate) dist_magic: FloatParam,
    #[id = "dist_mix"]
    pub(crate) dist_mix: FloatParam,
    #[id = "dist_env_atk"]
    pub(crate) dist_env_attack_ms: FloatParam,
    #[id = "dist_env_hold"]
    pub(crate) dist_env_hold_ms: FloatParam,
    #[id = "dist_env_dec"]
    pub(crate) dist_env_decay_ms: FloatParam,
    #[id = "dist_env_dec2"]
    pub(crate) dist_env_decay2_ms: FloatParam,
    #[id = "dist_env_dec2_lvl"]
    pub(crate) dist_env_decay2_level: FloatParam,
    #[id = "dist_env_sus"]
    pub(crate) dist_env_sustain_level: FloatParam,
    #[id = "dist_env_rel"]
    pub(crate) dist_env_release_ms: FloatParam,
    #[id = "dist_env_amt"]
    pub(crate) dist_env_amount: FloatParam,
    #[id = "eq_en"]
    pub(crate) eq_enable: BoolParam,
    #[id = "eq_low_gain"]
    pub(crate) eq_low_gain: FloatParam,
    #[id = "eq_mid_gain"]
    pub(crate) eq_mid_gain: FloatParam,
    #[id = "eq_mid_freq"]
    pub(crate) eq_mid_freq: FloatParam,
    #[id = "eq_mid_q"]
    pub(crate) eq_mid_q: FloatParam,
    #[id = "eq_high_gain"]
    pub(crate) eq_high_gain: FloatParam,
    #[id = "eq_mix"]
    pub(crate) eq_mix: FloatParam,
    #[id = "out_sat_en"]
    pub(crate) output_sat_enable: BoolParam,
    #[id = "out_sat_type"]
    pub(crate) output_sat_type: EnumParam<OutputSaturationType>,
    #[id = "out_sat_drive"]
    pub(crate) output_sat_drive: FloatParam,
    #[id = "out_sat_mix"]
    pub(crate) output_sat_mix: FloatParam,
    #[id = "mf_en"]
    pub(crate) multi_filter_enable: BoolParam,
    #[id = "mf_route"]
    pub(crate) multi_filter_routing: EnumParam<FilterRouting>,
    #[id = "mf_morph"]
    pub(crate) multi_filter_morph: FloatParam,
    #[id = "mf_par_ab"]
    pub(crate) multi_filter_parallel_ab: FloatParam,
    #[id = "mf_par_c"]
    pub(crate) multi_filter_parallel_c: FloatParam,
    #[id = "mf_a_type"]
    pub(crate) multi_filter_a_type: EnumParam<FilterType>,
    #[id = "mf_a_cut"]
    pub(crate) multi_filter_a_cut: FloatParam,
    #[id = "mf_a_res"]
    pub(crate) multi_filter_a_res: FloatParam,
    #[id = "mf_a_amt"]
    pub(crate) multi_filter_a_amt: FloatParam,
    #[id = "mf_b_type"]
    pub(crate) multi_filter_b_type: EnumParam<FilterType>,
    #[id = "mf_b_cut"]
    pub(crate) multi_filter_b_cut: FloatParam,
    #[id = "mf_b_res"]
    pub(crate) multi_filter_b_res: FloatParam,
    #[id = "mf_b_amt"]
    pub(crate) multi_filter_b_amt: FloatParam,
    #[id = "mf_c_type"]
    pub(crate) multi_filter_c_type: EnumParam<FilterType>,
    #[id = "mf_c_cut"]
    pub(crate) multi_filter_c_cut: FloatParam,
    #[id = "mf_c_res"]
    pub(crate) multi_filter_c_res: FloatParam,
    #[id = "mf_c_amt"]
    pub(crate) multi_filter_c_amt: FloatParam,
    #[id = "limiter_enable"]
    pub(crate) limiter_enable: BoolParam,
    #[id = "limiter_threshold"]
    pub(crate) limiter_threshold: FloatParam,
    #[id = "limiter_release"]
    pub(crate) limiter_release: FloatParam,
}


impl Default for SubSynthParams {
    fn default() -> Self {
        let preset_names = Arc::new(editor::factory_preset_names());

        Self {
            editor_state: editor::default_state(),
            custom_wavetable_path: Arc::new(RwLock::new(None)),
            custom_wavetable_data: Arc::new(RwLock::new(None)),
            preset_index: IntParam::new(
                "Preset",
                0,
                IntRange::Linear {
                    min: 0,
                    max: preset_names.len().saturating_sub(1) as i32,
                },
            )
            .with_value_to_string(Arc::new({
                let preset_names = preset_names.clone();
                move |value| {
                    preset_names
                        .get(value as usize)
                        .cloned()
                        .unwrap_or_else(|| format!("Preset {}", value + 1))
                }
            })),
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(-36.0),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(0.0),
                },
            )
            .with_poly_modulation_id(GAIN_POLY_MOD_ID)
            .with_smoother(SmoothingStyle::Logarithmic(5.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            amp_attack_ms: FloatParam::new(
                "Attack",
                0.4,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_hold_ms: FloatParam::new(
                "Hold",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_release_ms: FloatParam::new(
                "Release",
                3.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_tension: FloatParam::new(
                "Amp Tension",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            waveform: EnumParam::new("Waveform", Waveform::Sawtooth),
            osc_routing: EnumParam::new("Osc Routing", OscRouting::ClassicOnly),
            osc_blend: FloatParam::new(
                "Osc Blend",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_position: FloatParam::new(
                "Wavetable Position",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_distortion: FloatParam::new(
                "Wavetable Dist",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            classic_drive: FloatParam::new(
                "Classic Drive",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            custom_wavetable_enable: BoolParam::new("Custom Wavetable", false),
            analog_enable: BoolParam::new("Analog Enable", false),
            analog_drive: FloatParam::new(
                "Analog Drive",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            analog_noise: FloatParam::new(
                "Analog Noise",
                0.0,
                FloatRange::Linear { min: 0.0, max: 0.25 },
            )
            .with_step_size(0.001),
            analog_drift: FloatParam::new(
                "Analog Drift",
                0.02,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            sub_level: FloatParam::new(
                "Sub Level",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            classic_level: FloatParam::new(
                "Classic Level",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_level: FloatParam::new(
                "Wavetable Level",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            noise_level: FloatParam::new(
                "Noise Level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            classic_send: FloatParam::new(
                "Classic Send",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_send: FloatParam::new(
                "Wavetable Send",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            sub_send: FloatParam::new(
                "Sub Send",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            noise_send: FloatParam::new(
                "Noise Send",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            ring_mod_send: FloatParam::new(
                "Ring Send",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            fx_bus_mix: FloatParam::new(
                "FX Bus Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            ring_mod_enable: BoolParam::new("Ring Mod", false),
            ring_mod_source: EnumParam::new("Ring Source", RingModSource::Sine),
            ring_mod_freq: FloatParam::new(
                "Ring Freq",
                120.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_step_size(0.1),
            ring_mod_mix: FloatParam::new(
                "Ring Mix",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            ring_mod_level: FloatParam::new(
                "Ring Level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            ring_mod_placement: EnumParam::new("Ring Place", RingModPlacement::PreFilter),
            sizzle_osc_enable: BoolParam::new("Osc Sizzle Guard", true),
            sizzle_wt_enable: BoolParam::new("WT Sizzle Guard", true),
            sizzle_dist_enable: BoolParam::new("Dist Sizzle Guard", true),
            sizzle_cutoff: FloatParam::new(
                "Sizzle Cutoff",
                12000.0,
                FloatRange::Skewed {
                    min: 2000.0,
                    max: 18000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_step_size(1.0),
            spectral_enable: BoolParam::new("Spectral Enable", false),
            spectral_amount: FloatParam::new(
                "Spectral Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            spectral_tilt: FloatParam::new(
                "Spectral Tilt",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            spectral_formant: FloatParam::new(
                "Spectral Formant",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            spectral_placement: EnumParam::new("Spectral Place", SpectralPlacement::PreFx),
            filter_tight_enable: BoolParam::new("Filter Tight", true),
            unison_voices: EnumParam::new("Unison Voices", UnisonVoices::One),
            unison_detune: FloatParam::new(
                "Unison Detune",
                0.08,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            unison_spread: FloatParam::new(
                "Unison Spread",
                0.1,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            glide_mode: EnumParam::new("Glide Mode", GlideMode::Off),
            glide_time_ms: FloatParam::new(
                "Glide Time",
                0.0,
                FloatRange::Linear { min: 0.0, max: 500.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            lfo1_rate: FloatParam::new(
                "LFO1 Rate",
                2.0,
                FloatRange::Linear { min: 0.05, max: 16.0 },
            )
            .with_step_size(0.01),
            lfo1_attack: FloatParam::new(
                "LFO1 Attack",
                0.2,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            lfo1_shape: EnumParam::new("LFO1 Shape", OscillatorShape::Sine),
            lfo2_rate: FloatParam::new(
                "LFO2 Rate",
                0.8,
                FloatRange::Linear { min: 0.05, max: 8.0 },
            )
            .with_step_size(0.005),
            lfo2_attack: FloatParam::new(
                "LFO2 Attack",
                0.25,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            lfo2_shape: EnumParam::new("LFO2 Shape", OscillatorShape::Triangle),
            mod1_source: EnumParam::new("Mod1 Source", ModSource::Lfo1),
            mod1_target: EnumParam::new("Mod1 Target", ModTarget::FilterCut),
            mod1_amount: FloatParam::new(
                "Mod1 Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod1_smooth_ms: FloatParam::new(
                "Mod1 Smooth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 200.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            mod2_source: EnumParam::new("Mod2 Source", ModSource::Lfo2),
            mod2_target: EnumParam::new("Mod2 Target", ModTarget::WavetablePos),
            mod2_amount: FloatParam::new(
                "Mod2 Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod2_smooth_ms: FloatParam::new(
                "Mod2 Smooth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 200.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            mod3_source: EnumParam::new("Mod3 Source", ModSource::Lfo1),
            mod3_target: EnumParam::new("Mod3 Target", ModTarget::FilterRes),
            mod3_amount: FloatParam::new(
                "Mod3 Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod3_smooth_ms: FloatParam::new(
                "Mod3 Smooth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 200.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            mod4_source: EnumParam::new("Mod4 Source", ModSource::Lfo2),
            mod4_target: EnumParam::new("Mod4 Target", ModTarget::Pan),
            mod4_amount: FloatParam::new(
                "Mod4 Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod4_smooth_ms: FloatParam::new(
                "Mod4 Smooth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 200.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            mod5_source: EnumParam::new("Mod5 Source", ModSource::AmpEnv),
            mod5_target: EnumParam::new("Mod5 Target", ModTarget::FilterAmount),
            mod5_amount: FloatParam::new(
                "Mod5 Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod5_smooth_ms: FloatParam::new(
                "Mod5 Smooth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 200.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            mod6_source: EnumParam::new("Mod6 Source", ModSource::FilterEnv),
            mod6_target: EnumParam::new("Mod6 Target", ModTarget::FmAmount),
            mod6_amount: FloatParam::new(
                "Mod6 Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod6_smooth_ms: FloatParam::new(
                "Mod6 Smooth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 200.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            seq_enable: BoolParam::new("Seq Enable", false),
            seq_rate: FloatParam::new(
                "Seq Rate",
                2.0,
                FloatRange::Linear { min: 0.25, max: 16.0 },
            )
            .with_step_size(0.01),
            seq_gate_amount: FloatParam::new(
                "Seq Gate Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            seq_cut_amount: FloatParam::new(
                "Seq Cut Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            seq_res_amount: FloatParam::new(
                "Seq Res Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            seq_wt_amount: FloatParam::new(
                "Seq WT Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            seq_dist_amount: FloatParam::new(
                "Seq Dist Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            seq_fm_amount: FloatParam::new(
                "Seq FM Amount",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            seq_lanes: std::array::from_fn(|_| SeqLaneParams::default()),
            amp_decay_ms: FloatParam::new(
                "Decay",
                4.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_decay2_ms: FloatParam::new(
                "Decay 2",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            amp_decay2_level: FloatParam::new(
                "Decay 2 Level",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            amp_sustain_level: FloatParam::new(
                "Sustain",
                0.8,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" units"),
            filter_type: EnumParam::new("Filter Type", FilterType::Lowpass),
            filter_cut: FloatParam::new(
                "Filter Cutoff",
                120.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(10.0)),
            filter_res: FloatParam::new(
                "Filter Resonance",
                0.25,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            filter_amount: FloatParam::new(
                "Filter Amount",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(2))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            filter_cut_attack_ms: FloatParam::new(
                "Filter Cut Attack",
                0.8,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_cut_hold_ms: FloatParam::new(
                "Filter Cut Hold",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_cut_decay_ms: FloatParam::new(
                "Filter Cut Decay",
                6.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_cut_decay2_ms: FloatParam::new(
                "Filter Cut Decay 2",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            filter_cut_decay2_level: FloatParam::new(
                "Filter Cut Decay 2 Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_cut_sustain_ms: FloatParam::new(
                "Filter Cut Sustain",
                0.2,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_cut_release_ms: FloatParam::new(
                "Filter Cut Release",
                1.5,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_attack_ms: FloatParam::new(
                "Filter Resonance Attack",
                10.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_hold_ms: FloatParam::new(
                "Filter Resonance Hold",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_decay_ms: FloatParam::new(
                "Filter Resonance Decay",
                10.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_decay2_ms: FloatParam::new(
                "Filter Resonance Decay 2",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            filter_res_decay2_level: FloatParam::new(
                "Filter Resonance Decay 2 Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_res_sustain_ms: FloatParam::new(
                "Filter Resonance Sustain",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_res_release_ms: FloatParam::new(
                "Filter Resonance Release",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_envelope_level: FloatParam::new(
                "Amplitude Envelope Level",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_cut_envelope_level: FloatParam::new(
                "Filter Cutoff Envelope Level",
                0.25,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_res_envelope_level: FloatParam::new(
                "Filter Resonance Envelope Level",
                0.1,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            fm_enable: BoolParam::new("FM Enable", false),
            fm_source: EnumParam::new("FM Source", FmSource::Classic),
            fm_target: EnumParam::new("FM Target", FmTarget::Classic),
            fm_amount: FloatParam::new(
                "FM Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            fm_ratio: FloatParam::new(
                "FM Ratio",
                1.0,
                FloatRange::Skewed {
                    min: 0.25,
                    max: 8.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01),
            fm_feedback: FloatParam::new(
                "FM Feedback",
                0.0,
                FloatRange::Linear { min: 0.0, max: 0.95 },
            )
            .with_step_size(0.01),
            fm_env_attack_ms: FloatParam::new(
                "FM Env Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            fm_env_hold_ms: FloatParam::new(
                "FM Env Hold",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            fm_env_decay_ms: FloatParam::new(
                "FM Env Decay",
                120.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            fm_env_decay2_ms: FloatParam::new(
                "FM Env Decay 2",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            fm_env_decay2_level: FloatParam::new(
                "FM Env Decay 2 Level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            fm_env_sustain_level: FloatParam::new(
                "FM Env Sustain",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            fm_env_release_ms: FloatParam::new(
                "FM Env Release",
                120.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            fm_env_amount: FloatParam::new(
                "FM Env Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            vibrato_attack: FloatParam::new(
                "Vibrato Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            vibrato_intensity: FloatParam::new(
                "Vibrato Intensity",
                0.02,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            vibrato_rate: FloatParam::new(
                "Vibrato Rate",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 32.0,
                },
            )
            .with_step_size(1.0)
            .with_unit(" Hz"),
            tremolo_attack: FloatParam::new(
                "Tremolo Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            tremolo_intensity: FloatParam::new(
                "Tremolo Intensity",
                0.1,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            tremolo_rate: FloatParam::new(
                "Tremolo Rate",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            vibrato_shape: EnumParam::new("Vibrato Shape", OscillatorShape::Sine),
            tremolo_shape: EnumParam::new("Tremolo Shape", OscillatorShape::Sine),
            filter_cut_env_polarity: BoolParam::new("Filter Cut Env Polarity", true),
            filter_res_env_polarity: BoolParam::new("Filter Res Env Polarity", true),
            filter_cut_tension: FloatParam::new(
                "Filter Cut Tension",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_res_tension: FloatParam::new(
                "Filter Res Tension",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            cutoff_lfo_attack: FloatParam::new(
                "Cutoff LFO Attack",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            res_lfo_attack: FloatParam::new(
                "Res LFO Attack",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            pan_lfo_attack: FloatParam::new(
                "Pan LFO Attack",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            )
            .with_step_size(0.01),
            cutoff_lfo_intensity: FloatParam::new(
                "Cutoff LFO Intensity",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            cutoff_lfo_rate: FloatParam::new(
                "Cutoff LFO Rate",
                2.0,
                FloatRange::Linear { min: 0.01, max: 16.0 },
            )
            .with_step_size(0.01),
            cutoff_lfo_shape: EnumParam::new("Cutoff LFO Shape", OscillatorShape::Sine),
            res_lfo_intensity: FloatParam::new(
                "Res LFO Intensity",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            res_lfo_rate: FloatParam::new(
                "Res LFO Rate",
                2.0,
                FloatRange::Linear { min: 0.01, max: 16.0 },
            )
            .with_step_size(0.01),
            res_lfo_shape: EnumParam::new("Res LFO Shape", OscillatorShape::Sine),
            pan_lfo_intensity: FloatParam::new(
                "Pan LFO Intensity",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            pan_lfo_rate: FloatParam::new(
                "Pan LFO Rate",
                1.0,
                FloatRange::Linear { min: 0.01, max: 12.0 },
            )
            .with_step_size(0.01),
            pan_lfo_shape: EnumParam::new("Pan LFO Shape", OscillatorShape::Sine),
            chorus_enable: BoolParam::new("Chorus Enable", false),
            chorus_rate: FloatParam::new(
                "Chorus Rate",
                0.25,
                FloatRange::Linear { min: 0.1, max: 5.0 },
            )
            .with_step_size(0.01),
            chorus_depth: FloatParam::new(
                "Chorus Depth",
                18.0,
                FloatRange::Linear { min: 1.0, max: 50.0 },
            )
            .with_step_size(0.1),
            chorus_mix: FloatParam::new(
                "Chorus Mix",
                0.45,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            delay_enable: BoolParam::new("Delay Enable", false),
            delay_time_ms: FloatParam::new(
                "Delay Time",
                420.0,
                FloatRange::Linear { min: 10.0, max: 2000.0 },
            )
            .with_step_size(1.0)
            .with_unit(" ms"),
            delay_feedback: FloatParam::new(
                "Delay Feedback",
                0.35,
                FloatRange::Linear { min: 0.0, max: 0.95 },
            )
            .with_step_size(0.01),
            delay_mix: FloatParam::new(
                "Delay Mix",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            reverb_enable: BoolParam::new("Reverb Enable", false),
            reverb_size: FloatParam::new(
                "Reverb Size",
                0.65,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            reverb_damp: FloatParam::new(
                "Reverb Damp",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            reverb_diffusion: FloatParam::new(
                "Reverb Diffusion",
                0.55,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            reverb_shimmer: FloatParam::new(
                "Reverb Shimmer",
                0.15,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            reverb_mix: FloatParam::new(
                "Reverb Mix",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_enable: BoolParam::new("Dist Enable", true),
            dist_drive: FloatParam::new(
                "Dist Drive",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_tone: FloatParam::new(
                "Dist Tone",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_magic: FloatParam::new(
                "Dist Magic",
                0.4,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_mix: FloatParam::new(
                "Dist Mix",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_env_attack_ms: FloatParam::new(
                "Dist Env Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            dist_env_hold_ms: FloatParam::new(
                "Dist Env Hold",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            dist_env_decay_ms: FloatParam::new(
                "Dist Env Decay",
                120.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            dist_env_decay2_ms: FloatParam::new(
                "Dist Env Decay 2",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            dist_env_decay2_level: FloatParam::new(
                "Dist Env Decay 2 Level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_env_sustain_level: FloatParam::new(
                "Dist Env Sustain",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            dist_env_release_ms: FloatParam::new(
                "Dist Env Release",
                120.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 4000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            dist_env_amount: FloatParam::new(
                "Dist Env Amount",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            eq_enable: BoolParam::new("EQ Enable", true),
            eq_low_gain: FloatParam::new(
                "EQ Low Gain",
                3.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_step_size(0.1)
            .with_unit(" dB"),
            eq_mid_gain: FloatParam::new(
                "EQ Mid Gain",
                2.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_step_size(0.1)
            .with_unit(" dB"),
            eq_mid_freq: FloatParam::new(
                "EQ Mid Freq",
                700.0,
                FloatRange::Skewed {
                    min: 120.0,
                    max: 3500.0,
                    factor: FloatRange::skew_factor(-1.4),
                },
            )
            .with_unit(" Hz")
            .with_step_size(1.0),
            eq_mid_q: FloatParam::new(
                "EQ Mid Q",
                0.8,
                FloatRange::Linear { min: 0.2, max: 4.0 },
            )
            .with_step_size(0.01),
            eq_high_gain: FloatParam::new(
                "EQ High Gain",
                2.5,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_step_size(0.1)
            .with_unit(" dB"),
            eq_mix: FloatParam::new(
                "EQ Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            output_sat_enable: BoolParam::new("Output Sat", false),
            output_sat_type: EnumParam::new("Output Sat Type", OutputSaturationType::Tape),
            output_sat_drive: FloatParam::new(
                "Output Sat Drive",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            output_sat_mix: FloatParam::new(
                "Output Sat Mix",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            multi_filter_enable: BoolParam::new("Multi Filter Enable", false),
            multi_filter_routing: EnumParam::new("Multi Filter Routing", FilterRouting::Serial),
            multi_filter_morph: FloatParam::new(
                "Multi Filter Morph",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            multi_filter_parallel_ab: FloatParam::new(
                "Multi Filter AB Mix",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            multi_filter_parallel_c: FloatParam::new(
                "Multi Filter C Mix",
                0.33,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            multi_filter_a_type: EnumParam::new("Multi Filter A Type", FilterType::Lowpass),
            multi_filter_a_cut: FloatParam::new(
                "Multi Filter A Cut",
                1200.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(10.0)),
            multi_filter_a_res: FloatParam::new(
                "Multi Filter A Res",
                0.25,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            multi_filter_a_amt: FloatParam::new(
                "Multi Filter A Amt",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            multi_filter_b_type: EnumParam::new("Multi Filter B Type", FilterType::Bandpass),
            multi_filter_b_cut: FloatParam::new(
                "Multi Filter B Cut",
                1600.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(10.0)),
            multi_filter_b_res: FloatParam::new(
                "Multi Filter B Res",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            multi_filter_b_amt: FloatParam::new(
                "Multi Filter B Amt",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            multi_filter_c_type: EnumParam::new("Multi Filter C Type", FilterType::Highpass),
            multi_filter_c_cut: FloatParam::new(
                "Multi Filter C Cut",
                220.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(10.0)),
            multi_filter_c_res: FloatParam::new(
                "Multi Filter C Res",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            multi_filter_c_amt: FloatParam::new(
                "Multi Filter C Amt",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            limiter_enable: BoolParam::new("Limiter Enable", true),
            limiter_threshold: FloatParam::new(
                "Limiter Threshold",
                0.9,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            limiter_release: FloatParam::new(
                "Limiter Release",
                100.0,
                FloatRange::Linear { min: 1.0, max: 1000.0 },
            )
            .with_step_size(1.0),
        }
    }
}
