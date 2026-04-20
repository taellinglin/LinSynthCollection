use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, RwLock};
use crate::common::*;
use crate::modulator::OscillatorShape;
use crate::waveform::Waveform;
use crate::filter::FilterType;
use crate::output_saturation::OutputSaturationType;
use crate::util;
use crate::editor;

pub const SEQ_LANE_COUNT: usize = 6;
pub const GAIN_POLY_MOD_ID: u32 = 0;

#[derive(Params)]
pub struct SeqStepParams {
    #[id = "val"]
    pub value: FloatParam,
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
pub struct SeqLaneParams {
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

#[derive(Params)]
pub struct SubSynthParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[persist = "custom_wt_path"]
    pub custom_wavetable_path: Arc<RwLock<Option<String>>>,
    pub custom_wavetable_data: Arc<RwLock<Option<Vec<f32>>>>,
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "amp_atk"]
    pub amp_attack_ms: FloatParam,
    #[id = "amp_rel"]
    pub amp_release_ms: FloatParam,
    #[id = "amp_tension"]
    pub amp_tension: FloatParam,
    #[id = "waveform"]
    pub waveform: EnumParam<Waveform>,
    #[id = "osc_route"]
    pub osc_routing: EnumParam<OscRouting>,
    #[id = "osc_blend"]
    pub osc_blend: FloatParam,
    #[id = "wt_pos"]
    pub wavetable_position: FloatParam,
    #[id = "wt_dist"]
    pub wavetable_distortion: FloatParam,
    #[id = "classic_drive"]
    pub classic_drive: FloatParam,
    #[id = "wt_custom"]
    pub custom_wavetable_enable: BoolParam,
    #[id = "analog_en"]
    pub analog_enable: BoolParam,
    #[id = "analog_drive"]
    pub analog_drive: FloatParam,
    #[id = "analog_noise"]
    pub analog_noise: FloatParam,
    #[id = "analog_drift"]
    pub analog_drift: FloatParam,
    #[id = "sub_level"]
    pub sub_level: FloatParam,
    #[id = "unison_voices"]
    pub unison_voices: EnumParam<UnisonVoices>,
    #[id = "unison_detune"]
    pub unison_detune: FloatParam,
    #[id = "unison_spread"]
    pub unison_spread: FloatParam,
    #[id = "glide_mode"]
    pub glide_mode: EnumParam<GlideMode>,
    #[id = "glide_time"]
    pub glide_time_ms: FloatParam,
    #[id = "lfo1_rate"]
    pub lfo1_rate: FloatParam,
    #[id = "lfo1_atk"]
    pub lfo1_attack: FloatParam,
    #[id = "lfo1_shape"]
    pub lfo1_shape: EnumParam<OscillatorShape>,
    #[id = "lfo2_rate"]
    pub lfo2_rate: FloatParam,
    #[id = "lfo2_atk"]
    pub lfo2_attack: FloatParam,
    #[id = "lfo2_shape"]
    pub lfo2_shape: EnumParam<OscillatorShape>,
    #[id = "mod1_src"]
    pub mod1_source: EnumParam<ModSource>,
    #[id = "mod1_tgt"]
    pub mod1_target: EnumParam<ModTarget>,
    #[id = "mod1_amt"]
    pub mod1_amount: FloatParam,
    #[id = "mod1_smooth"]
    pub mod1_smooth_ms: FloatParam,
    #[id = "mod2_src"]
    pub mod2_source: EnumParam<ModSource>,
    #[id = "mod2_tgt"]
    pub mod2_target: EnumParam<ModTarget>,
    #[id = "mod2_amt"]
    pub mod2_amount: FloatParam,
    #[id = "mod2_smooth"]
    pub mod2_smooth_ms: FloatParam,
    #[id = "mod3_src"]
    pub mod3_source: EnumParam<ModSource>,
    #[id = "mod3_tgt"]
    pub mod3_target: EnumParam<ModTarget>,
    #[id = "mod3_amt"]
    pub mod3_amount: FloatParam,
    #[id = "mod3_smooth"]
    pub mod3_smooth_ms: FloatParam,
    #[id = "mod4_src"]
    pub mod4_source: EnumParam<ModSource>,
    #[id = "mod4_tgt"]
    pub mod4_target: EnumParam<ModTarget>,
    #[id = "mod4_amt"]
    pub mod4_amount: FloatParam,
    #[id = "mod4_smooth"]
    pub mod4_smooth_ms: FloatParam,
    #[id = "mod5_src"]
    pub mod5_source: EnumParam<ModSource>,
    #[id = "mod5_tgt"]
    pub mod5_target: EnumParam<ModTarget>,
    #[id = "mod5_amt"]
    pub mod5_amount: FloatParam,
    #[id = "mod5_smooth"]
    pub mod5_smooth_ms: FloatParam,
    #[id = "mod6_src"]
    pub mod6_source: EnumParam<ModSource>,
    #[id = "mod6_tgt"]
    pub mod6_target: EnumParam<ModTarget>,
    #[id = "mod6_amt"]
    pub mod6_amount: FloatParam,
    #[id = "mod6_smooth"]
    pub mod6_smooth_ms: FloatParam,
    #[id = "seq_enable"]
    pub seq_enable: BoolParam,
    #[id = "seq_rate"]
    pub seq_rate: FloatParam,
    #[id = "seq_gate_amt"]
    pub seq_gate_amount: FloatParam,
    #[id = "seq_cut_amt"]
    pub seq_cut_amount: FloatParam,
    #[id = "seq_res_amt"]
    pub seq_res_amount: FloatParam,
    #[id = "seq_wt_amt"]
    pub seq_wt_amount: FloatParam,
    #[id = "seq_dist_amt"]
    pub seq_dist_amount: FloatParam,
    #[id = "seq_fm_amt"]
    pub seq_fm_amount: FloatParam,
    #[nested(array, group = "Sequencer")]
    pub seq_lanes: [SeqLaneParams; SEQ_LANE_COUNT],

    #[id = "amp_dec"]
    pub amp_decay_ms: FloatParam,
    #[id = "amp_sus"]
    pub amp_sustain_level: FloatParam,
    #[id = "filter_cut_atk"]
    pub filter_cut_attack_ms: FloatParam,
    #[id = "filter_cut_dec"]
    pub filter_cut_decay_ms: FloatParam,
    #[id = "filter_cut_sus"]
    pub filter_cut_sustain_ms: FloatParam,
    #[id = "filter_cut_rel"]
    pub filter_cut_release_ms: FloatParam,
    #[id = "filter_res_atk"]
    pub filter_res_attack_ms: FloatParam,
    #[id = "filter_res_dec"]
    pub filter_res_decay_ms: FloatParam,
    #[id = "filter_res_sus"]
    pub filter_res_sustain_ms: FloatParam,
    #[id = "filter_res_rel"]
    pub filter_res_release_ms: FloatParam,
    #[id = "filter_type"]
    pub filter_type: EnumParam<FilterType>,
    #[id = "filter_cut"]
    pub filter_cut: FloatParam,
    #[id = "filter_res"]
    pub filter_res: FloatParam,
    #[id = "filter_amount"]
    pub filter_amount: FloatParam,
    #[id = "amp_env_level"]
    pub amp_envelope_level: FloatParam,
    #[id = "filter_cut_env_level"]
    pub filter_cut_envelope_level: FloatParam,
    #[id = "filter_res_env_level"]
    pub filter_res_envelope_level: FloatParam,
    #[id = "fm_enable"]
    pub fm_enable: BoolParam,
    #[id = "fm_source"]
    pub fm_source: EnumParam<FmSource>,
    #[id = "fm_target"]
    pub fm_target: EnumParam<FmTarget>,
    #[id = "fm_amount"]
    pub fm_amount: FloatParam,
    #[id = "fm_ratio"]
    pub fm_ratio: FloatParam,
    #[id = "fm_feedback"]
    pub fm_feedback: FloatParam,
    #[id = "fm_env_atk"]
    pub fm_env_attack_ms: FloatParam,
    #[id = "fm_env_dec"]
    pub fm_env_decay_ms: FloatParam,
    #[id = "fm_env_sus"]
    pub fm_env_sustain_level: FloatParam,
    #[id = "fm_env_rel"]
    pub fm_env_release_ms: FloatParam,
    #[id = "fm_env_amt"]
    pub fm_env_amount: FloatParam,
    #[id = "vibrato_atk"]
    pub vibrato_attack: FloatParam,
    #[id = "vibrato_int"]
    pub vibrato_intensity: FloatParam,
    #[id = "vibrato_rate"]
    pub vibrato_rate: FloatParam,
    #[id = "tremolo_atk"]
    pub tremolo_attack: FloatParam,
    #[id = "tremolo_int"]
    pub tremolo_intensity: FloatParam,
    #[id = "tremolo_rate"]
    pub tremolo_rate: FloatParam,
    #[id = "vibrato_shape"]
    pub vibrato_shape: EnumParam<OscillatorShape>,
    #[id = "tremolo_shape"]
    pub tremolo_shape: EnumParam<OscillatorShape>,
    #[id = "filter_cut_env_pol"]
    pub filter_cut_env_polarity: BoolParam,
    #[id = "filter_res_env_pol"]
    pub filter_res_env_polarity: BoolParam,
    #[id = "filter_cut_tension"]
    pub filter_cut_tension: FloatParam,
    #[id = "filter_res_tension"]
    pub filter_res_tension: FloatParam,
    #[id = "cutoff_lfo_attack"]
    pub cutoff_lfo_attack: FloatParam,
    #[id = "res_lfo_attack"]
    pub res_lfo_attack: FloatParam,
    #[id = "pan_lfo_attack"]
    pub pan_lfo_attack: FloatParam,
    #[id = "cutoff_lfo_int"]
    pub cutoff_lfo_intensity: FloatParam,
    #[id = "cutoff_lfo_rate"]
    pub cutoff_lfo_rate: FloatParam,
    #[id = "cutoff_lfo_shape"]
    pub cutoff_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "res_lfo_int"]
    pub res_lfo_intensity: FloatParam,
    #[id = "res_lfo_rate"]
    pub res_lfo_rate: FloatParam,
    #[id = "res_lfo_shape"]
    pub res_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "pan_lfo_int"]
    pub pan_lfo_intensity: FloatParam,
    #[id = "pan_lfo_rate"]
    pub pan_lfo_rate: FloatParam,
    #[id = "pan_lfo_shape"]
    pub pan_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "chorus_enable"]
    pub chorus_enable: BoolParam,
    #[id = "chorus_rate"]
    pub chorus_rate: FloatParam,
    #[id = "chorus_depth"]
    pub chorus_depth: FloatParam,
    #[id = "chorus_mix"]
    pub chorus_mix: FloatParam,
    #[id = "delay_en"]
    pub delay_enable: BoolParam,
    #[id = "delay_time"]
    pub delay_time_ms: FloatParam,
    #[id = "delay_fb"]
    pub delay_feedback: FloatParam,
    #[id = "delay_mix"]
    pub delay_mix: FloatParam,
    #[id = "rev_en"]
    pub reverb_enable: BoolParam,
    #[id = "rev_size"]
    pub reverb_size: FloatParam,
    #[id = "rev_damp"]
    pub reverb_damp: FloatParam,
    #[id = "rev_diff"]
    pub reverb_diffusion: FloatParam,
    #[id = "rev_shim"]
    pub reverb_shimmer: FloatParam,
    #[id = "rev_mix"]
    pub reverb_mix: FloatParam,
    #[id = "dist_en"]
    pub dist_enable: BoolParam,
    #[id = "dist_drive"]
    pub dist_drive: FloatParam,
    #[id = "dist_tone"]
    pub dist_tone: FloatParam,
    #[id = "dist_magic"]
    pub dist_magic: FloatParam,
    #[id = "dist_mix"]
    pub dist_mix: FloatParam,
    #[id = "dist_env_atk"]
    pub dist_env_attack_ms: FloatParam,
    #[id = "dist_env_dec"]
    pub dist_env_decay_ms: FloatParam,
    #[id = "dist_env_sus"]
    pub dist_env_sustain_level: FloatParam,
    #[id = "dist_env_rel"]
    pub dist_env_release_ms: FloatParam,
    #[id = "dist_env_amt"]
    pub dist_env_amount: FloatParam,
    #[id = "eq_en"]
    pub eq_enable: BoolParam,
    #[id = "eq_low_gain"]
    pub eq_low_gain: FloatParam,
    #[id = "eq_mid_gain"]
    pub eq_mid_gain: FloatParam,
    #[id = "eq_mid_freq"]
    pub eq_mid_freq: FloatParam,
    #[id = "eq_mid_q"]
    pub eq_mid_q: FloatParam,
    #[id = "eq_high_gain"]
    pub eq_high_gain: FloatParam,
    #[id = "eq_mix"]
    pub eq_mix: FloatParam,
    #[id = "out_sat_en"]
    pub output_sat_enable: BoolParam,
    #[id = "out_sat_type"]
    pub output_sat_type: EnumParam<OutputSaturationType>,
    #[id = "out_sat_drive"]
    pub output_sat_drive: FloatParam,
    #[id = "out_sat_mix"]
    pub output_sat_mix: FloatParam,
    #[id = "mf_en"]
    pub multi_filter_enable: BoolParam,
    #[id = "mf_route"]
    pub multi_filter_routing: EnumParam<FilterRouting>,
    #[id = "mf_morph"]
    pub multi_filter_morph: FloatParam,
    #[id = "mf_par_ab"]
    pub multi_filter_parallel_ab: FloatParam,
    #[id = "mf_par_c"]
    pub multi_filter_parallel_c: FloatParam,
    #[id = "mf_a_type"]
    pub multi_filter_a_type: EnumParam<FilterType>,
    #[id = "mf_a_cut"]
    pub multi_filter_a_cut: FloatParam,
    #[id = "mf_a_res"]
    pub multi_filter_a_res: FloatParam,
    #[id = "mf_a_amt"]
    pub multi_filter_a_amt: FloatParam,
    #[id = "mf_b_type"]
    pub multi_filter_b_type: EnumParam<FilterType>,
    #[id = "mf_b_cut"]
    pub multi_filter_b_cut: FloatParam,
    #[id = "mf_b_res"]
    pub multi_filter_b_res: FloatParam,
    #[id = "mf_b_amt"]
    pub multi_filter_b_amt: FloatParam,
    #[id = "mf_c_type"]
    pub multi_filter_c_type: EnumParam<FilterType>,
    #[id = "mf_c_cut"]
    pub multi_filter_c_cut: FloatParam,
    #[id = "mf_c_res"]
    pub multi_filter_c_res: FloatParam,
    #[id = "mf_c_amt"]
    pub multi_filter_c_amt: FloatParam,
    #[id = "limiter_enable"]
    pub limiter_enable: BoolParam,
    #[id = "limiter_threshold"]
    pub limiter_threshold: FloatParam,
    #[id = "limiter_release"]
    pub limiter_release: FloatParam,
}

impl Default for SubSynthParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            custom_wavetable_path: Arc::new(RwLock::new(None)),
            custom_wavetable_data: Arc::new(RwLock::new(None)),
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
                0.8,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_release_ms: FloatParam::new(
                "Release",
                8.0,
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
            osc_routing: EnumParam::new("Osc Routing", OscRouting::Blend),
            osc_blend: FloatParam::new(
                "Osc Blend",
                0.8,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_position: FloatParam::new(
                "Wavetable Position",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_distortion: FloatParam::new(
                "Wavetable Dist",
                0.45,
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
            analog_enable: BoolParam::new("Analog Enable", true),
            analog_drive: FloatParam::new(
                "Analog Drive",
                0.55,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            analog_noise: FloatParam::new(
                "Analog Noise",
                0.03,
                FloatRange::Linear { min: 0.0, max: 0.25 },
            )
            .with_step_size(0.001),
            analog_drift: FloatParam::new(
                "Analog Drift",
                0.08,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            sub_level: FloatParam::new(
                "Sub Level",
                0.55,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            unison_voices: EnumParam::new("Unison Voices", UnisonVoices::One),
            unison_detune: FloatParam::new(
                "Unison Detune",
                0.1,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            unison_spread: FloatParam::new(
                "Unison Spread",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            glide_mode: EnumParam::new("Glide Mode", GlideMode::Off),
            glide_time_ms: FloatParam::new(
                "Glide Time",
                50.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1000.0,
                    factor: FloatRange::skew_factor(-2.5),
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            lfo1_rate: FloatParam::new(
                "LFO1 Rate",
                2.0,
                FloatRange::Skewed {
                    min: 0.01,
                    max: 50.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            lfo1_attack: FloatParam::new(
                "LFO1 Attack",
                0.0,
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
                0.5,
                FloatRange::Skewed {
                    min: 0.01,
                    max: 50.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            lfo2_attack: FloatParam::new(
                "LFO2 Attack",
                0.0,
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
                0.8,
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
                0.5,
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
                6.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_sustain_level: FloatParam::new(
                "Sustain",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_cut_attack_ms: FloatParam::new(
                "Filter Cut Attack",
                1.0,
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
                20.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
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
            filter_type: EnumParam::new("Filter Type", FilterType::Lowpass),
            filter_cut: FloatParam::new(
                "Filter Cutoff",
                800.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz"),
            filter_res: FloatParam::new(
                "Filter Resonance",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            filter_amount: FloatParam::new(
                "Filter Amount",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
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
                0.45,
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
            .with_step_size(0.01),
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
            .with_step_size(0.01),
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
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            res_lfo_attack: FloatParam::new(
                "Res LFO Attack",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            pan_lfo_attack: FloatParam::new(
                "Pan LFO Attack",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            cutoff_lfo_intensity: FloatParam::new(
                "Cutoff LFO Intensity",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            cutoff_lfo_rate: FloatParam::new(
                "Cutoff LFO Rate",
                1.0,
                FloatRange::Linear { min: 0.0, max: 20.0 },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            cutoff_lfo_shape: EnumParam::new("Cutoff LFO Shape", OscillatorShape::Sine),
            res_lfo_intensity: FloatParam::new(
                "Res LFO Intensity",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            res_lfo_rate: FloatParam::new(
                "Res LFO Rate",
                1.0,
                FloatRange::Linear { min: 0.0, max: 20.0 },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            res_lfo_shape: EnumParam::new("Res LFO Shape", OscillatorShape::Sine),
            pan_lfo_intensity: FloatParam::new(
                "Pan LFO Intensity",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            pan_lfo_rate: FloatParam::new(
                "Pan LFO Rate",
                1.0,
                FloatRange::Linear { min: 0.0, max: 20.0 },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            pan_lfo_shape: EnumParam::new("Pan LFO Shape", OscillatorShape::Sine),
            chorus_enable: BoolParam::new("Chorus Enable", false),
            chorus_rate: FloatParam::new(
                "Chorus Rate",
                0.5,
                FloatRange::Linear { min: 0.1, max: 5.0 },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            chorus_depth: FloatParam::new(
                "Chorus Depth",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            chorus_mix: FloatParam::new(
                "Chorus Mix",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            delay_enable: BoolParam::new("Delay Enable", false),
            delay_time_ms: FloatParam::new(
                "Delay Time",
                300.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms"),
            delay_feedback: FloatParam::new(
                "Delay Feedback",
                0.4,
                FloatRange::Linear { min: 0.0, max: 0.95 },
            ),
            delay_mix: FloatParam::new(
                "Delay Mix",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            reverb_enable: BoolParam::new("Reverb Enable", false),
            reverb_size: FloatParam::new(
                "Reverb Size",
                0.7,
                FloatRange::Linear { min: 0.1, max: 0.99 },
            ),
            reverb_damp: FloatParam::new(
                "Reverb Damp",
                0.4,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            reverb_diffusion: FloatParam::new(
                "Reverb Diffusion",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            reverb_shimmer: FloatParam::new(
                "Reverb Shimmer",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            reverb_mix: FloatParam::new(
                "Reverb Mix",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            dist_enable: BoolParam::new("Distort Enable", false),
            dist_drive: FloatParam::new(
                "Distort Drive",
                0.4,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            dist_tone: FloatParam::new(
                "Distort Tone",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            dist_magic: FloatParam::new(
                "Distort Magic",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            dist_mix: FloatParam::new(
                "Distort Mix",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
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

pub struct SubSynthBlockParams {
    pub gain: f32,
    pub amp_attack_ms: f32,
    pub amp_decay_ms: f32,
    pub amp_sustain_level: f32,
    pub amp_release_ms: f32,
    pub amp_tension: f32,
    pub waveform: Waveform,
    pub osc_routing: OscRouting,
    pub osc_blend: f32,
    pub wavetable_position: f32,
    pub wavetable_distortion: f32,
    pub classic_drive: f32,
    pub custom_wavetable_enable: bool,
    pub analog_enable: bool,
    pub analog_drive: f32,
    pub analog_noise: f32,
    pub analog_drift: f32,
    pub sub_level: f32,
    pub unison_voices: UnisonVoices,
    pub unison_detune: f32,
    pub unison_spread: f32,
    pub glide_mode: GlideMode,
    pub glide_time_ms: f32,
    pub lfo1_rate: f32,
    pub lfo1_attack: f32,
    pub lfo1_shape: OscillatorShape,
    pub lfo2_rate: f32,
    pub lfo2_attack: f32,
    pub lfo2_shape: OscillatorShape,
    pub filter_type: FilterType,
    pub filter_cut: f32,
    pub filter_res: f32,
    pub filter_amount: f32,
    pub amp_envelope_level: f32,
    pub filter_cut_envelope_level: f32,
    pub filter_res_envelope_level: f32,
    pub mod1_source: ModSource,
    pub mod1_target: ModTarget,
    pub mod1_amount: f32,
    pub mod2_source: ModSource,
    pub mod2_target: ModTarget,
    pub mod2_amount: f32,
    pub mod3_source: ModSource,
    pub mod3_target: ModTarget,
    pub mod3_amount: f32,
    pub mod4_source: ModSource,
    pub mod4_target: ModTarget,
    pub mod4_amount: f32,
    pub mod5_source: ModSource,
    pub mod5_target: ModTarget,
    pub mod5_amount: f32,
    pub mod6_source: ModSource,
    pub mod6_target: ModTarget,
    pub mod6_amount: f32,
    pub fm_enable: bool,
    pub fm_source: FmSource,
    pub fm_target: FmTarget,
    pub fm_amount: f32,
    pub fm_ratio: f32,
    pub fm_feedback: f32,
    pub fm_env_attack_ms: f32,
    pub fm_env_decay_ms: f32,
    pub fm_env_sustain_level: f32,
    pub fm_env_release_ms: f32,
    pub fm_env_amount: f32,
    pub dist_enable: bool,
    pub dist_drive: f32,
    pub dist_tone: f32,
    pub dist_magic: f32,
    pub dist_mix: f32,
    pub dist_env_attack_ms: f32,
    pub dist_env_decay_ms: f32,
    pub dist_env_sustain_level: f32,
    pub dist_env_release_ms: f32,
    pub dist_env_amount: f32,
    pub chorus_enable: bool,
    pub chorus_rate: f32,
    pub chorus_depth: f32,
    pub chorus_mix: f32,
    pub delay_enable: bool,
    pub delay_time_ms: f32,
    pub delay_feedback: f32,
    pub delay_mix: f32,
    pub reverb_enable: bool,
    pub reverb_size: f32,
    pub reverb_damp: f32,
    pub reverb_diffusion: f32,
    pub reverb_shimmer: f32,
    pub reverb_mix: f32,
    pub eq_enable: bool,
    pub eq_low_gain: f32,
    pub eq_mid_gain: f32,
    pub eq_mid_freq: f32,
    pub eq_mid_q: f32,
    pub eq_high_gain: f32,
    pub eq_mix: f32,
    pub output_sat_enable: bool,
    pub output_sat_type: OutputSaturationType,
    pub output_sat_drive: f32,
    pub output_sat_mix: f32,
    pub multi_filter_enable: bool,
    pub multi_filter_routing: FilterRouting,
    pub multi_filter_morph: f32,
    pub multi_filter_parallel_ab: f32,
    pub multi_filter_parallel_c: f32,
    pub fm_ratio_mod: f32,
    pub fm_feedback_mod: f32,
    pub limiter_enable: bool,
    pub limiter_threshold: f32,
    pub limiter_release: f32,
}

impl SubSynthBlockParams {
    pub fn cache(params: &SubSynthParams) -> Self {
        Self {
            gain: params.gain.value(),
            amp_attack_ms: params.amp_attack_ms.value(),
            amp_decay_ms: params.amp_decay_ms.value(),
            amp_sustain_level: params.amp_sustain_level.value(),
            amp_release_ms: params.amp_release_ms.value(),
            amp_tension: params.amp_tension.value(),
            waveform: params.waveform.value(),
            osc_routing: params.osc_routing.value(),
            osc_blend: params.osc_blend.value(),
            wavetable_position: params.wavetable_position.value(),
            wavetable_distortion: params.wavetable_distortion.value(),
            classic_drive: params.classic_drive.value(),
            custom_wavetable_enable: params.custom_wavetable_enable.value(),
            analog_enable: params.analog_enable.value(),
            analog_drive: params.analog_drive.value(),
            analog_noise: params.analog_noise.value(),
            analog_drift: params.analog_drift.value(),
            sub_level: params.sub_level.value(),
            unison_voices: params.unison_voices.value(),
            unison_detune: params.unison_detune.value(),
            unison_spread: params.unison_spread.value(),
            glide_mode: params.glide_mode.value(),
            glide_time_ms: params.glide_time_ms.value(),
            lfo1_rate: params.lfo1_rate.value(),
            lfo1_attack: params.lfo1_attack.value(),
            lfo1_shape: params.lfo1_shape.value(),
            lfo2_rate: params.lfo2_rate.value(),
            lfo2_attack: params.lfo2_attack.value(),
            lfo2_shape: params.lfo2_shape.value(),
            filter_type: params.filter_type.value(),
            filter_cut: params.filter_cut.value(),
            filter_res: params.filter_res.value(),
            filter_amount: params.filter_amount.value(),
            amp_envelope_level: params.amp_envelope_level.value(),
            filter_cut_envelope_level: params.filter_cut_envelope_level.value(),
            filter_res_envelope_level: params.filter_res_envelope_level.value(),
            mod1_source: params.mod1_source.value(),
            mod1_target: params.mod1_target.value(),
            mod1_amount: params.mod1_amount.value(),
            mod2_source: params.mod2_source.value(),
            mod2_target: params.mod2_target.value(),
            mod2_amount: params.mod2_amount.value(),
            mod3_source: params.mod3_source.value(),
            mod3_target: params.mod3_target.value(),
            mod3_amount: params.mod3_amount.value(),
            mod4_source: params.mod4_source.value(),
            mod4_target: params.mod4_target.value(),
            mod4_amount: params.mod4_amount.value(),
            mod5_source: params.mod5_source.value(),
            mod5_target: params.mod5_target.value(),
            mod5_amount: params.mod5_amount.value(),
            mod6_source: params.mod6_source.value(),
            mod6_target: params.mod6_target.value(),
            mod6_amount: params.mod6_amount.value(),
            fm_enable: params.fm_enable.value(),
            fm_source: params.fm_source.value(),
            fm_target: params.fm_target.value(),
            fm_amount: params.fm_amount.value(),
            fm_ratio: params.fm_ratio.value(),
            fm_feedback: params.fm_feedback.value(),
            fm_env_attack_ms: params.fm_env_attack_ms.value(),
            fm_env_decay_ms: params.fm_env_decay_ms.value(),
            fm_env_sustain_level: params.fm_env_sustain_level.value(),
            fm_env_release_ms: params.fm_env_release_ms.value(),
            fm_env_amount: params.fm_env_amount.value(),
            dist_enable: params.dist_enable.value(),
            dist_drive: params.dist_drive.value(),
            dist_tone: params.dist_tone.value(),
            dist_magic: params.dist_magic.value(),
            dist_mix: params.dist_mix.value(),
            dist_env_attack_ms: params.dist_env_attack_ms.value(),
            dist_env_decay_ms: params.dist_env_decay_ms.value(),
            dist_env_sustain_level: params.dist_env_sustain_level.value(),
            dist_env_release_ms: params.dist_env_release_ms.value(),
            dist_env_amount: params.dist_env_amount.value(),
            chorus_enable: params.chorus_enable.value(),
            chorus_rate: params.chorus_rate.value(),
            chorus_depth: params.chorus_depth.value(),
            chorus_mix: params.chorus_mix.value(),
            delay_enable: params.delay_enable.value(),
            delay_time_ms: params.delay_time_ms.value(),
            delay_feedback: params.delay_feedback.value(),
            delay_mix: params.delay_mix.value(),
            reverb_enable: params.reverb_enable.value(),
            reverb_size: params.reverb_size.value(),
            reverb_damp: params.reverb_damp.value(),
            reverb_diffusion: params.reverb_diffusion.value(),
            reverb_shimmer: params.reverb_shimmer.value(),
            reverb_mix: params.reverb_mix.value(),
            eq_enable: params.eq_enable.value(),
            eq_low_gain: params.eq_low_gain.value(),
            eq_mid_gain: params.eq_mid_gain.value(),
            eq_mid_freq: params.eq_mid_freq.value(),
            eq_mid_q: params.eq_mid_q.value(),
            eq_high_gain: params.eq_high_gain.value(),
            eq_mix: params.eq_mix.value(),
            output_sat_enable: params.output_sat_enable.value(),
            output_sat_type: params.output_sat_type.value(),
            output_sat_drive: params.output_sat_drive.value(),
            output_sat_mix: params.output_sat_mix.value(),
            multi_filter_enable: params.multi_filter_enable.value(),
            multi_filter_routing: params.multi_filter_routing.value(),
            multi_filter_morph: params.multi_filter_morph.value(),
            multi_filter_parallel_ab: params.multi_filter_parallel_ab.value(),
            multi_filter_parallel_c: params.multi_filter_parallel_c.value(),
            fm_ratio_mod: 0.0,
            fm_feedback_mod: 0.0,
            limiter_enable: params.limiter_enable.value(),
            limiter_threshold: params.limiter_threshold.value(),
            limiter_release: params.limiter_release.value(),
        }
    }
}
