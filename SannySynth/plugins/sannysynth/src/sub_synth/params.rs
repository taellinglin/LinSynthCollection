use nih_plug::prelude::*;
use std::sync::{Arc, RwLock};
use nih_plug_vizia::ViziaState;
use crate::editor;
use crate::preset_bank::GM_PROGRAM_NAMES;
use crate::util;
use crate::common::*;
use crate::waveform::Waveform;
use crate::modulator::OscillatorShape;
use crate::filter::{FilterType, FilterStyle};
use crate::resonator::ResonatorTimbre;

pub const GAIN_POLY_MOD_ID: u32 = 0;

#[derive(Params)]
pub struct SubSynthParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[persist = "custom_wt_path"]
    pub custom_wavetable_path: Arc<RwLock<Option<String>>>,
    pub custom_wavetable_data: Arc<RwLock<Option<Vec<f32>>>>,
    #[id = "program"]
    pub program: IntParam,
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "amp_atk"]
    pub amp_attack_ms: FloatParam,
    #[id = "amp_rel"]
    pub amp_release_ms: FloatParam,
    #[id = "waveform"]
    pub waveform: EnumParam<Waveform>,
    #[id = "osc_route"]
    pub osc_routing: EnumParam<OscRouting>,
    #[id = "osc_blend"]
    pub osc_blend: FloatParam,
    #[id = "wt_pos"]
    pub wavetable_position: FloatParam,
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
    #[id = "mod2_src"]
    pub mod2_source: EnumParam<ModSource>,
    #[id = "mod2_tgt"]
    pub mod2_target: EnumParam<ModTarget>,
    #[id = "mod2_amt"]
    pub mod2_amount: FloatParam,

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
    #[id = "filter_style"]
    pub filter_style: EnumParam<FilterStyle>,
    #[id = "filter_drive"]
    pub filter_vintage_drive: FloatParam,
    #[id = "filter_curve"]
    pub filter_vintage_curve: FloatParam,
    #[id = "filter_mix"]
    pub filter_vintage_mix: FloatParam,
    #[id = "filter_trim"]
    pub filter_vintage_trim: FloatParam,
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
    #[id = "res_en"]
    pub resonator_enable: BoolParam,
    #[id = "res_mix"]
    pub resonator_mix: FloatParam,
    #[id = "res_tone"]
    pub resonator_tone: FloatParam,
    #[id = "res_shape"]
    pub resonator_shape: FloatParam,
    #[id = "res_map"]
    pub resonator_timbre: EnumParam<ResonatorTimbre>,
    #[id = "res_damp"]
    pub resonator_damping: FloatParam,
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
    #[id = "mf_a_style"]
    pub multi_filter_a_style: EnumParam<FilterStyle>,
    #[id = "mf_a_drive"]
    pub multi_filter_a_drive: FloatParam,
    #[id = "mf_a_curve"]
    pub multi_filter_a_curve: FloatParam,
    #[id = "mf_a_mix"]
    pub multi_filter_a_mix: FloatParam,
    #[id = "mf_a_trim"]
    pub multi_filter_a_trim: FloatParam,
    #[id = "mf_a_cut"]
    pub multi_filter_a_cut: FloatParam,
    #[id = "mf_a_res"]
    pub multi_filter_a_res: FloatParam,
    #[id = "mf_a_amt"]
    pub multi_filter_a_amt: FloatParam,
    #[id = "mf_b_type"]
    pub multi_filter_b_type: EnumParam<FilterType>,
    #[id = "mf_b_style"]
    pub multi_filter_b_style: EnumParam<FilterStyle>,
    #[id = "mf_b_drive"]
    pub multi_filter_b_drive: FloatParam,
    #[id = "mf_b_curve"]
    pub multi_filter_b_curve: FloatParam,
    #[id = "mf_b_mix"]
    pub multi_filter_b_mix: FloatParam,
    #[id = "mf_b_trim"]
    pub multi_filter_b_trim: FloatParam,
    #[id = "mf_b_cut"]
    pub multi_filter_b_cut: FloatParam,
    #[id = "mf_b_res"]
    pub multi_filter_b_res: FloatParam,
    #[id = "mf_b_amt"]
    pub multi_filter_b_amt: FloatParam,
    #[id = "mf_c_type"]
    pub multi_filter_c_type: EnumParam<FilterType>,
    #[id = "mf_c_style"]
    pub multi_filter_c_style: EnumParam<FilterStyle>,
    #[id = "mf_c_drive"]
    pub multi_filter_c_drive: FloatParam,
    #[id = "mf_c_curve"]
    pub multi_filter_c_curve: FloatParam,
    #[id = "mf_c_mix"]
    pub multi_filter_c_mix: FloatParam,
    #[id = "mf_c_trim"]
    pub multi_filter_c_trim: FloatParam,
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
            program: IntParam::new("Program", 0, IntRange::Linear { min: 0, max: 127 })
                .with_value_to_string(Arc::new(|value| {
                    GM_PROGRAM_NAMES[value as usize].to_string()
                }))
                .with_string_to_value(Arc::new(|text| {
                    let trimmed = text.trim();
                    GM_PROGRAM_NAMES
                        .iter()
                        .position(|name| name.eq_ignore_ascii_case(trimmed))
                        .map(|index| index as i32)
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
                1.5,
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
                4.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            waveform: EnumParam::new("Waveform", Waveform::Sine),
            osc_routing: EnumParam::new("Osc Routing", OscRouting::Blend),
            osc_blend: FloatParam::new(
                "Osc Blend",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            wavetable_position: FloatParam::new(
                "Wavetable Position",
                0.55,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            custom_wavetable_enable: BoolParam::new("Custom Wavetable", false),
            analog_enable: BoolParam::new("Analog Enable", true),
            analog_drive: FloatParam::new(
                "Analog Drive",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            analog_noise: FloatParam::new(
                "Analog Noise",
                0.02,
                FloatRange::Linear { min: 0.0, max: 0.25 },
            )
            .with_step_size(0.001),
            analog_drift: FloatParam::new(
                "Analog Drift",
                0.15,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            sub_level: FloatParam::new(
                "Sub Level",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            lfo1_rate: FloatParam::new(
                "LFO1 Rate",
                0.08,
                FloatRange::Linear { min: 0.01, max: 2.0 },
            )
            .with_step_size(0.01),
            lfo1_attack: FloatParam::new(
                "LFO1 Attack",
                4.0,
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
                0.015,
                FloatRange::Linear { min: 0.01, max: 1.0 },
            )
            .with_step_size(0.005),
            lfo2_attack: FloatParam::new(
                "LFO2 Attack",
                6.0,
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
            mod1_target: EnumParam::new("Mod1 Target", ModTarget::WavetablePos),
            mod1_amount: FloatParam::new(
                "Mod1 Amount",
                0.35,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mod2_source: EnumParam::new("Mod2 Source", ModSource::Lfo2),
            mod2_target: EnumParam::new("Mod2 Target", ModTarget::FilterCut),
            mod2_amount: FloatParam::new(
                "Mod2 Amount",
                0.25,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            amp_decay_ms: FloatParam::new(
                "Decay",
                3.0,
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
                0.8,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" units"),
            filter_cut_attack_ms: FloatParam::new("Filter Cut Attack", 2.0, FloatRange::Linear { min: 0.0, max: 10.0 }),
            filter_cut_decay_ms: FloatParam::new("Filter Cut Decay", 5.0, FloatRange::Linear { min: 0.0, max: 100.0 }),
            filter_cut_sustain_ms: FloatParam::new("Filter Cut Sustain", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            filter_cut_release_ms: FloatParam::new("Filter Cut Release", 10.0, FloatRange::Linear { min: 0.0, max: 100.0 }),
            filter_res_attack_ms: FloatParam::new("Filter Res Attack", 1.0, FloatRange::Linear { min: 0.0, max: 10.0 }),
            filter_res_decay_ms: FloatParam::new("Filter Res Decay", 4.0, FloatRange::Linear { min: 0.0, max: 100.0 }),
            filter_res_sustain_ms: FloatParam::new("Filter Res Sustain", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 }),
            filter_res_release_ms: FloatParam::new("Filter Res Release", 8.0, FloatRange::Linear { min: 0.0, max: 100.0 }),
            filter_type: EnumParam::new("Filter Type", FilterType::Lowpass),
            filter_style: EnumParam::new("Filter Style", FilterStyle::Digital),
            filter_vintage_drive: FloatParam::new(
                "Filter Vintage Drive",
                0.35,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_vintage_curve: FloatParam::new(
                "Filter Vintage Curve",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_vintage_mix: FloatParam::new(
                "Filter Vintage Mix",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            filter_vintage_trim: FloatParam::new(
                "Filter Vintage Trim",
                1.0,
                FloatRange::Linear { min: 0.5, max: 1.5 },
            )
            .with_step_size(0.01),
            filter_cut: FloatParam::new(
                "Filter Cutoff",
                900.0,
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
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            amp_envelope_level: FloatParam::new(
                "Amp Env Level",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            filter_cut_envelope_level: FloatParam::new(
                "Filter Cut Env Level",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            filter_res_envelope_level: FloatParam::new(
                "Filter Res Env Level",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            vibrato_attack: FloatParam::new("Vibrato Attack", 0.5, FloatRange::Linear { min: 0.0, max: 10.0 }),
            vibrato_intensity: FloatParam::new("Vibrato Intensity", 0.05, FloatRange::Linear { min: 0.0, max: 1.0 }),
            vibrato_rate: FloatParam::new("Vibrato Rate", 6.0, FloatRange::Linear { min: 0.1, max: 20.0 }),
            tremolo_attack: FloatParam::new("Tremolo Attack", 0.2, FloatRange::Linear { min: 0.0, max: 10.0 }),
            tremolo_intensity: FloatParam::new("Tremolo Intensity", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 }),
            tremolo_rate: FloatParam::new("Tremolo Rate", 4.0, FloatRange::Linear { min: 0.1, max: 20.0 }),
            vibrato_shape: EnumParam::new("Vibrato Shape", OscillatorShape::Sine),
            tremolo_shape: EnumParam::new("Tremolo Shape", OscillatorShape::Sine),
            filter_cut_env_polarity: BoolParam::new("Filter Cut Polarity", true),
            filter_res_env_polarity: BoolParam::new("Filter Res Polarity", true),
            filter_cut_tension: FloatParam::new("Filter Cut Tension", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 }),
            filter_res_tension: FloatParam::new("Filter Res Tension", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 }),
            cutoff_lfo_attack: FloatParam::new("Cutoff LFO Attack", 0.0, FloatRange::Linear { min: 0.0, max: 10.0 }),
            res_lfo_attack: FloatParam::new("Res LFO Attack", 0.0, FloatRange::Linear { min: 0.0, max: 10.0 }),
            pan_lfo_attack: FloatParam::new("Pan LFO Attack", 0.0, FloatRange::Linear { min: 0.0, max: 10.0 }),
            cutoff_lfo_intensity: FloatParam::new("Cutoff LFO Int", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            cutoff_lfo_rate: FloatParam::new("Cutoff LFO Rate", 1.0, FloatRange::Linear { min: 0.1, max: 20.0 }),
            cutoff_lfo_shape: EnumParam::new("Cutoff LFO Shape", OscillatorShape::Sine),
            res_lfo_intensity: FloatParam::new("Res LFO Int", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            res_lfo_rate: FloatParam::new("Res LFO Rate", 1.0, FloatRange::Linear { min: 0.1, max: 20.0 }),
            res_lfo_shape: EnumParam::new("Res LFO Shape", OscillatorShape::Sine),
            pan_lfo_intensity: FloatParam::new("Pan LFO Int", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            pan_lfo_rate: FloatParam::new("Pan LFO Rate", 1.0, FloatRange::Linear { min: 0.1, max: 20.0 }),
            pan_lfo_shape: EnumParam::new("Pan LFO Shape", OscillatorShape::Sine),
            chorus_enable: BoolParam::new("Chorus Enable", false),
            chorus_rate: FloatParam::new("Chorus Rate", 0.5, FloatRange::Linear { min: 0.1, max: 10.0 }),
            chorus_depth: FloatParam::new("Chorus Depth", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 }),
            chorus_mix: FloatParam::new("Chorus Mix", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            delay_enable: BoolParam::new("Delay Enable", false),
            delay_time_ms: FloatParam::new("Delay Time", 500.0, FloatRange::Linear { min: 1.0, max: 2000.0 }),
            delay_feedback: FloatParam::new("Delay Feedback", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 }),
            delay_mix: FloatParam::new("Delay Mix", 0.2, FloatRange::Linear { min: 0.0, max: 1.0 }),
            reverb_enable: BoolParam::new("Reverb Enable", false),
            reverb_size: FloatParam::new("Reverb Size", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 }),
            reverb_damp: FloatParam::new("Reverb Damp", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            reverb_diffusion: FloatParam::new("Reverb Diff", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            reverb_shimmer: FloatParam::new("Reverb Shim", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            reverb_mix: FloatParam::new("Reverb Mix", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 }),
            resonator_enable: BoolParam::new("Resonator Enable", false),
            resonator_mix: FloatParam::new("Resonator Mix", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            resonator_tone: FloatParam::new("Resonator Tone", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            resonator_shape: FloatParam::new("Resonator Shape", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            resonator_timbre: EnumParam::new("Resonator Timbre", ResonatorTimbre::Balanced),
            resonator_damping: FloatParam::new("Resonator Damp", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_enable: BoolParam::new("Multi Filter Enable", false),
            multi_filter_routing: EnumParam::new("Multi Filter Routing", FilterRouting::Serial),
            multi_filter_morph: FloatParam::new("Multi Filter Morph", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_parallel_ab: FloatParam::new("MF Parallel AB", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_parallel_c: FloatParam::new("MF Parallel C", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_a_type: EnumParam::new("MF A Type", FilterType::Lowpass),
            multi_filter_a_style: EnumParam::new("MF A Style", FilterStyle::Digital),
            multi_filter_a_drive: FloatParam::new("MF A Drive", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_a_curve: FloatParam::new("MF A Curve", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_a_mix: FloatParam::new("MF A Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_a_trim: FloatParam::new("MF A Trim", 1.0, FloatRange::Linear { min: 0.5, max: 1.5 }),
            multi_filter_a_cut: FloatParam::new("MF A Cut", 1000.0, FloatRange::Skewed { min: 20.0, max: 20000.0, factor: 0.5 }),
            multi_filter_a_res: FloatParam::new("MF A Res", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_a_amt: FloatParam::new("MF A Amt", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_b_type: EnumParam::new("MF B Type", FilterType::Bandpass),
            multi_filter_b_style: EnumParam::new("MF B Style", FilterStyle::Digital),
            multi_filter_b_drive: FloatParam::new("MF B Drive", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_b_curve: FloatParam::new("MF B Curve", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_b_mix: FloatParam::new("MF B Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_b_trim: FloatParam::new("MF B Trim", 1.0, FloatRange::Linear { min: 0.5, max: 1.5 }),
            multi_filter_b_cut: FloatParam::new("MF B Cut", 1000.0, FloatRange::Skewed { min: 20.0, max: 20000.0, factor: 0.5 }),
            multi_filter_b_res: FloatParam::new("MF B Res", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_b_amt: FloatParam::new("MF B Amt", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_c_type: EnumParam::new("MF C Type", FilterType::Highpass),
            multi_filter_c_style: EnumParam::new("MF C Style", FilterStyle::Digital),
            multi_filter_c_drive: FloatParam::new("MF C Drive", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_c_curve: FloatParam::new("MF C Curve", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_c_mix: FloatParam::new("MF C Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_c_trim: FloatParam::new("MF C Trim", 1.0, FloatRange::Linear { min: 0.5, max: 1.5 }),
            multi_filter_c_cut: FloatParam::new("MF C Cut", 1000.0, FloatRange::Skewed { min: 20.0, max: 20000.0, factor: 0.5 }),
            multi_filter_c_res: FloatParam::new("MF C Res", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 }),
            multi_filter_c_amt: FloatParam::new("MF C Amt", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            limiter_enable: BoolParam::new("Limiter Enable", false),
            limiter_threshold: FloatParam::new("Limiter Threshold", 0.95, FloatRange::Linear { min: 0.0, max: 1.0 }),
            limiter_release: FloatParam::new("Limiter Release", 100.0, FloatRange::Linear { min: 1.0, max: 1000.0 }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubSynthBlockParams {
    pub gain: f32,
    pub waveform: Waveform,
    pub osc_routing: OscRouting,
    pub osc_blend: f32,
    pub wavetable_position: f32,
    pub custom_wavetable_enable: bool,
    pub analog_enable: bool,
    pub analog_drive: f32,
    pub analog_noise: f32,
    pub analog_drift: f32,
    pub sub_level: f32,
    pub filter_type: FilterType,
    pub filter_style: FilterStyle,
    pub filter_cut: f32,
    pub filter_res: f32,
    pub filter_amount: f32,
    pub filter_vintage_drive: f32,
    pub filter_vintage_curve: f32,
    pub filter_vintage_mix: f32,
    pub filter_vintage_trim: f32,
    pub amp_envelope_level: f32,
    pub filter_cut_envelope_level: f32,
    pub filter_res_envelope_level: f32,
    pub mod1_source: ModSource,
    pub mod1_target: ModTarget,
    pub mod1_amount: f32,
    pub mod2_source: ModSource,
    pub mod2_target: ModTarget,
    pub mod2_amount: f32,
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
    pub resonator_enable: bool,
    pub resonator_mix: f32,
    pub resonator_tone: f32,
    pub resonator_shape: f32,
    pub resonator_timbre: ResonatorTimbre,
    pub resonator_damping: f32,
    pub multi_filter_enable: bool,
    pub multi_filter_routing: FilterRouting,
    pub multi_filter_morph: f32,
    pub multi_filter_parallel_ab: f32,
    pub multi_filter_parallel_c: f32,
    pub multi_filter_a_type: FilterType,
    pub multi_filter_a_style: FilterStyle,
    pub multi_filter_a_drive: f32,
    pub multi_filter_a_curve: f32,
    pub multi_filter_a_mix: f32,
    pub multi_filter_a_trim: f32,
    pub multi_filter_a_cut: f32,
    pub multi_filter_a_res: f32,
    pub multi_filter_a_amt: f32,
    pub multi_filter_b_type: FilterType,
    pub multi_filter_b_style: FilterStyle,
    pub multi_filter_b_drive: f32,
    pub multi_filter_b_curve: f32,
    pub multi_filter_b_mix: f32,
    pub multi_filter_b_trim: f32,
    pub multi_filter_b_cut: f32,
    pub multi_filter_b_res: f32,
    pub multi_filter_b_amt: f32,
    pub multi_filter_c_type: FilterType,
    pub multi_filter_c_style: FilterStyle,
    pub multi_filter_c_drive: f32,
    pub multi_filter_c_curve: f32,
    pub multi_filter_c_mix: f32,
    pub multi_filter_c_trim: f32,
    pub multi_filter_c_cut: f32,
    pub multi_filter_c_res: f32,
    pub multi_filter_c_amt: f32,
    pub limiter_enable: bool,
    pub limiter_threshold: f32,
    pub limiter_release: f32,
}

impl SubSynthBlockParams {
    pub fn cache(params: &SubSynthParams) -> Self {
        Self {
            gain: params.gain.value(),
            waveform: params.waveform.value(),
            osc_routing: params.osc_routing.value(),
            osc_blend: params.osc_blend.value(),
            wavetable_position: params.wavetable_position.value(),
            custom_wavetable_enable: params.custom_wavetable_enable.value(),
            analog_enable: params.analog_enable.value(),
            analog_drive: params.analog_drive.value(),
            analog_noise: params.analog_noise.value(),
            analog_drift: params.analog_drift.value(),
            sub_level: params.sub_level.value(),
            filter_type: params.filter_type.value(),
            filter_style: params.filter_style.value(),
            filter_cut: params.filter_cut.value(),
            filter_res: params.filter_res.value(),
            filter_amount: params.filter_amount.value(),
            filter_vintage_drive: params.filter_vintage_drive.value(),
            filter_vintage_curve: params.filter_vintage_curve.value(),
            filter_vintage_mix: params.filter_vintage_mix.value(),
            filter_vintage_trim: params.filter_vintage_trim.value(),
            amp_envelope_level: params.amp_envelope_level.value(),
            filter_cut_envelope_level: params.filter_cut_envelope_level.value(),
            filter_res_envelope_level: params.filter_res_envelope_level.value(),
            mod1_source: params.mod1_source.value(),
            mod1_target: params.mod1_target.value(),
            mod1_amount: params.mod1_amount.value(),
            mod2_source: params.mod2_source.value(),
            mod2_target: params.mod2_target.value(),
            mod2_amount: params.mod2_amount.value(),
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
            resonator_enable: params.resonator_enable.value(),
            resonator_mix: params.resonator_mix.value(),
            resonator_tone: params.resonator_tone.value(),
            resonator_shape: params.resonator_shape.value(),
            resonator_timbre: params.resonator_timbre.value(),
            resonator_damping: params.resonator_damping.value(),
            multi_filter_enable: params.multi_filter_enable.value(),
            multi_filter_routing: params.multi_filter_routing.value(),
            multi_filter_morph: params.multi_filter_morph.value(),
            multi_filter_parallel_ab: params.multi_filter_parallel_ab.value(),
            multi_filter_parallel_c: params.multi_filter_parallel_c.value(),
            multi_filter_a_type: params.multi_filter_a_type.value(),
            multi_filter_a_style: params.multi_filter_a_style.value(),
            multi_filter_a_drive: params.multi_filter_a_drive.value(),
            multi_filter_a_curve: params.multi_filter_a_curve.value(),
            multi_filter_a_mix: params.multi_filter_a_mix.value(),
            multi_filter_a_trim: params.multi_filter_a_trim.value(),
            multi_filter_a_cut: params.multi_filter_a_cut.value(),
            multi_filter_a_res: params.multi_filter_a_res.value(),
            multi_filter_a_amt: params.multi_filter_a_amt.value(),
            multi_filter_b_type: params.multi_filter_b_type.value(),
            multi_filter_b_style: params.multi_filter_b_style.value(),
            multi_filter_b_drive: params.multi_filter_b_drive.value(),
            multi_filter_b_curve: params.multi_filter_b_curve.value(),
            multi_filter_b_mix: params.multi_filter_b_mix.value(),
            multi_filter_b_trim: params.multi_filter_b_trim.value(),
            multi_filter_b_cut: params.multi_filter_b_cut.value(),
            multi_filter_b_res: params.multi_filter_b_res.value(),
            multi_filter_b_amt: params.multi_filter_b_amt.value(),
            multi_filter_c_type: params.multi_filter_c_type.value(),
            multi_filter_c_style: params.multi_filter_c_style.value(),
            multi_filter_c_drive: params.multi_filter_c_drive.value(),
            multi_filter_c_curve: params.multi_filter_c_curve.value(),
            multi_filter_c_mix: params.multi_filter_c_mix.value(),
            multi_filter_c_trim: params.multi_filter_c_trim.value(),
            multi_filter_c_cut: params.multi_filter_c_cut.value(),
            multi_filter_c_res: params.multi_filter_c_res.value(),
            multi_filter_c_amt: params.multi_filter_c_amt.value(),
            limiter_enable: params.limiter_enable.value(),
            limiter_threshold: params.limiter_threshold.value(),
            limiter_release: params.limiter_release.value(),
        }
    }
}
