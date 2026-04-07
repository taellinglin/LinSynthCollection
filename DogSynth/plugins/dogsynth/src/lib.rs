mod editor;
mod envelope;
mod filter;
mod chorus;
mod delay;
mod reverb;
mod limiter;
mod multi_filter;
mod waveform;
mod modulator;
mod eq;
mod distortion;
mod output_saturation;
mod spectral;

use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;
use nih_plug::params::enums::EnumParam;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::Rng;
use rand_pcg::Pcg32;
use std::sync::{Arc, RwLock};

use modulator::{Modulator, OscillatorShape};
use chorus::Chorus;
use delay::StereoDelay;
use envelope::{ADSREnvelope, Envelope, ADSREnvelopeState};
use filter::{FilterType, Filter};
use limiter::Limiter;
use multi_filter::MultiStageFilter;
use waveform::{generate_waveform, WavetableBank, Waveform};
use eq::{Biquad, ThreeBandEq};
use distortion::Distortion;
use output_saturation::{OutputSaturation, OutputSaturationType};
use spectral::SpectralShaper;

const NUM_VOICES: usize = 16;
const MAX_BLOCK_SIZE: usize = 64;
const GAIN_POLY_MOD_ID: u32 = 0;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum OscRouting {
    ClassicOnly,
    WavetableOnly,
    Blend,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum ModSource {
    Lfo1,
    Lfo2,
    AmpEnv,
    FilterEnv,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum ModTarget {
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
enum FmSource {
    Classic,
    Wavetable,
    Sub,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum FmTarget {
    Classic,
    Wavetable,
    Both,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum GlideMode {
    Off,
    Legato,
    Always,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum UnisonVoices {
    One,
    Two,
    Four,
    Six,
}

#[derive(Params)]
struct SeqStepParams {
    #[id = "val"]
    value: FloatParam,
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
struct SeqLaneParams {
    #[nested(array)]
    steps: [SeqStepParams; 32],
}

impl Default for SeqLaneParams {
    fn default() -> Self {
        Self {
            steps: std::array::from_fn(|_| SeqStepParams::default()),
        }
    }
}

const SEQ_LANE_COUNT: usize = 6;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum FilterRouting {
    Serial,
    Parallel,
    Morph,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum RingModPlacement {
    PreFilter,
    PostFilter,
    PostFx,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum RingModSource {
    Sine,
    Classic,
    Wavetable,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
enum SpectralPlacement {
    PreFx,
    PreDist,
    PostFx,
}

struct SubSynth {
    params: Arc<SubSynthParams>,
    prng: Pcg32,
    voices: [Option<Voice>; NUM_VOICES as usize],
    next_voice_index: usize,
    next_internal_voice_id: u64,
    chorus: Chorus,
    delay: StereoDelay,
    reverb: reverb::Reverb,
    limiter_left: Limiter,
    limiter_right: Limiter,
    multi_filter: MultiStageFilter,
    distortion: Distortion,
    eq: ThreeBandEq,
    output_saturation: OutputSaturation,
    spectral_main: SpectralShaper,
    spectral_fx: SpectralShaper,
    factory_wavetable: WavetableBank,
    custom_wavetable: Option<WavetableBank>,
    custom_wavetable_path: Option<String>,
    factory_presets: Vec<editor::PresetData>,
    last_preset_index: i32,
    seq_phase: f32,
    last_note_phase_delta: f32,
    last_note_active: bool,
    ring_mod_post_phase: [f32; 2],
}

#[derive(Params)]
struct SubSynthParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    #[persist = "custom_wt_path"]
    custom_wavetable_path: Arc<RwLock<Option<String>>>,
    custom_wavetable_data: Arc<RwLock<Option<Vec<f32>>>>,
    #[id = "preset"]
    preset_index: IntParam,
    #[id = "gain"]
    gain: FloatParam,
    #[id = "amp_atk"]
    amp_attack_ms: FloatParam,
    #[id = "amp_hold"]
    amp_hold_ms: FloatParam,
    #[id = "amp_rel"]
    amp_release_ms: FloatParam,
    #[id = "amp_tension"]
    amp_tension: FloatParam,
    #[id = "waveform"]
    waveform: EnumParam<Waveform>,
    #[id = "osc_route"]
    osc_routing: EnumParam<OscRouting>,
    #[id = "osc_blend"]
    osc_blend: FloatParam,
    #[id = "wt_pos"]
    wavetable_position: FloatParam,
    #[id = "wt_dist"]
    wavetable_distortion: FloatParam,
    #[id = "classic_drive"]
    classic_drive: FloatParam,
    #[id = "wt_custom"]
    custom_wavetable_enable: BoolParam,
    #[id = "analog_en"]
    analog_enable: BoolParam,
    #[id = "analog_drive"]
    analog_drive: FloatParam,
    #[id = "analog_noise"]
    analog_noise: FloatParam,
    #[id = "analog_drift"]
    analog_drift: FloatParam,
    #[id = "sub_level"]
    sub_level: FloatParam,
    #[id = "mix_classic"]
    classic_level: FloatParam,
    #[id = "mix_wt"]
    wavetable_level: FloatParam,
    #[id = "mix_noise"]
    noise_level: FloatParam,
    #[id = "send_classic"]
    classic_send: FloatParam,
    #[id = "send_wt"]
    wavetable_send: FloatParam,
    #[id = "send_sub"]
    sub_send: FloatParam,
    #[id = "send_noise"]
    noise_send: FloatParam,
    #[id = "send_ring"]
    ring_mod_send: FloatParam,
    #[id = "fx_bus_mix"]
    fx_bus_mix: FloatParam,
    #[id = "ring_en"]
    ring_mod_enable: BoolParam,
    #[id = "ring_src"]
    ring_mod_source: EnumParam<RingModSource>,
    #[id = "ring_freq"]
    ring_mod_freq: FloatParam,
    #[id = "ring_mix"]
    ring_mod_mix: FloatParam,
    #[id = "ring_level"]
    ring_mod_level: FloatParam,
    #[id = "ring_place"]
    ring_mod_placement: EnumParam<RingModPlacement>,
    #[id = "sizzle_osc"]
    sizzle_osc_enable: BoolParam,
    #[id = "sizzle_wt"]
    sizzle_wt_enable: BoolParam,
    #[id = "sizzle_dist"]
    sizzle_dist_enable: BoolParam,
    #[id = "sizzle_cut"]
    sizzle_cutoff: FloatParam,
    #[id = "spec_en"]
    spectral_enable: BoolParam,
    #[id = "spec_amt"]
    spectral_amount: FloatParam,
    #[id = "spec_tilt"]
    spectral_tilt: FloatParam,
    #[id = "spec_formant"]
    spectral_formant: FloatParam,
    #[id = "spec_place"]
    spectral_placement: EnumParam<SpectralPlacement>,
    #[id = "filter_tight"]
    filter_tight_enable: BoolParam,
    #[id = "unison_voices"]
    unison_voices: EnumParam<UnisonVoices>,
    #[id = "unison_detune"]
    unison_detune: FloatParam,
    #[id = "unison_spread"]
    unison_spread: FloatParam,
    #[id = "glide_mode"]
    glide_mode: EnumParam<GlideMode>,
    #[id = "glide_time"]
    glide_time_ms: FloatParam,
    #[id = "lfo1_rate"]
    lfo1_rate: FloatParam,
    #[id = "lfo1_atk"]
    lfo1_attack: FloatParam,
    #[id = "lfo1_shape"]
    lfo1_shape: EnumParam<OscillatorShape>,
    #[id = "lfo2_rate"]
    lfo2_rate: FloatParam,
    #[id = "lfo2_atk"]
    lfo2_attack: FloatParam,
    #[id = "lfo2_shape"]
    lfo2_shape: EnumParam<OscillatorShape>,
    #[id = "mod1_src"]
    mod1_source: EnumParam<ModSource>,
    #[id = "mod1_tgt"]
    mod1_target: EnumParam<ModTarget>,
    #[id = "mod1_amt"]
    mod1_amount: FloatParam,
        #[id = "mod1_smooth"]
        mod1_smooth_ms: FloatParam,
    #[id = "mod2_src"]
    mod2_source: EnumParam<ModSource>,
    #[id = "mod2_tgt"]
    mod2_target: EnumParam<ModTarget>,
    #[id = "mod2_amt"]
    mod2_amount: FloatParam,
    #[id = "mod2_smooth"]
    mod2_smooth_ms: FloatParam,
    #[id = "mod3_src"]
    mod3_source: EnumParam<ModSource>,
    #[id = "mod3_tgt"]
    mod3_target: EnumParam<ModTarget>,
    #[id = "mod3_amt"]
    mod3_amount: FloatParam,
    #[id = "mod3_smooth"]
    mod3_smooth_ms: FloatParam,
    #[id = "mod4_src"]
    mod4_source: EnumParam<ModSource>,
    #[id = "mod4_tgt"]
    mod4_target: EnumParam<ModTarget>,
    #[id = "mod4_amt"]
    mod4_amount: FloatParam,
    #[id = "mod4_smooth"]
    mod4_smooth_ms: FloatParam,
    #[id = "mod5_src"]
    mod5_source: EnumParam<ModSource>,
    #[id = "mod5_tgt"]
    mod5_target: EnumParam<ModTarget>,
    #[id = "mod5_amt"]
    mod5_amount: FloatParam,
    #[id = "mod5_smooth"]
    mod5_smooth_ms: FloatParam,
    #[id = "mod6_src"]
    mod6_source: EnumParam<ModSource>,
    #[id = "mod6_tgt"]
    mod6_target: EnumParam<ModTarget>,
    #[id = "mod6_amt"]
    mod6_amount: FloatParam,
    #[id = "mod6_smooth"]
    mod6_smooth_ms: FloatParam,
    #[id = "seq_enable"]
    seq_enable: BoolParam,
    #[id = "seq_rate"]
    seq_rate: FloatParam,
    #[id = "seq_gate_amt"]
    seq_gate_amount: FloatParam,
    #[id = "seq_cut_amt"]
    seq_cut_amount: FloatParam,
    #[id = "seq_res_amt"]
    seq_res_amount: FloatParam,
    #[id = "seq_wt_amt"]
    seq_wt_amount: FloatParam,
    #[id = "seq_dist_amt"]
    seq_dist_amount: FloatParam,
    #[id = "seq_fm_amt"]
    seq_fm_amount: FloatParam,
    #[nested(array, group = "Sequencer")]
    seq_lanes: [SeqLaneParams; SEQ_LANE_COUNT],

    // New parameters for ADSR envelope
    #[id = "amp_dec"]
    amp_decay_ms: FloatParam,
    #[id = "amp_dec2"]
    amp_decay2_ms: FloatParam,
    #[id = "amp_dec2_lvl"]
    amp_decay2_level: FloatParam,
    #[id = "amp_sus"]
    amp_sustain_level: FloatParam,
    #[id = "filter_cut_atk"]
    filter_cut_attack_ms: FloatParam,
    #[id = "filter_cut_hold"]
    filter_cut_hold_ms: FloatParam,
    #[id = "filter_cut_dec"]
    filter_cut_decay_ms: FloatParam,
    #[id = "filter_cut_dec2"]
    filter_cut_decay2_ms: FloatParam,
    #[id = "filter_cut_dec2_lvl"]
    filter_cut_decay2_level: FloatParam,
    #[id = "filter_cut_sus"]
    filter_cut_sustain_ms: FloatParam,
    #[id = "filter_cut_rel"]
    filter_cut_release_ms: FloatParam,
    #[id = "filter_res_atk"]
    filter_res_attack_ms: FloatParam,
    #[id = "filter_res_hold"]
    filter_res_hold_ms: FloatParam,
    #[id = "filter_res_dec"]
    filter_res_decay_ms: FloatParam,
    #[id = "filter_res_dec2"]
    filter_res_decay2_ms: FloatParam,
    #[id = "filter_res_dec2_lvl"]
    filter_res_decay2_level: FloatParam,
    #[id = "filter_res_sus"]
    filter_res_sustain_ms: FloatParam,
    #[id = "filter_res_rel"]
    filter_res_release_ms: FloatParam,
    #[id = "filter_type"]
    filter_type: EnumParam<FilterType>,
    #[id = "filter_cut"]
    filter_cut: FloatParam,
    #[id = "filter_res"]
    filter_res: FloatParam,
    #[id = "filter_amount"]
    filter_amount: FloatParam,
    // New parameters for ADSR envelope levels
    #[id = "amp_env_level"]
    amp_envelope_level: FloatParam,
    #[id = "filter_cut_env_level"]
    filter_cut_envelope_level: FloatParam,
    #[id = "filter_res_env_level"]
    filter_res_envelope_level: FloatParam,
    #[id = "fm_enable"]
    fm_enable: BoolParam,
    #[id = "fm_source"]
    fm_source: EnumParam<FmSource>,
    #[id = "fm_target"]
    fm_target: EnumParam<FmTarget>,
    #[id = "fm_amount"]
    fm_amount: FloatParam,
    #[id = "fm_ratio"]
    fm_ratio: FloatParam,
    #[id = "fm_feedback"]
    fm_feedback: FloatParam,
    #[id = "fm_env_atk"]
    fm_env_attack_ms: FloatParam,
    #[id = "fm_env_hold"]
    fm_env_hold_ms: FloatParam,
    #[id = "fm_env_dec"]
    fm_env_decay_ms: FloatParam,
    #[id = "fm_env_dec2"]
    fm_env_decay2_ms: FloatParam,
    #[id = "fm_env_dec2_lvl"]
    fm_env_decay2_level: FloatParam,
    #[id = "fm_env_sus"]
    fm_env_sustain_level: FloatParam,
    #[id = "fm_env_rel"]
    fm_env_release_ms: FloatParam,
    #[id = "fm_env_amt"]
    fm_env_amount: FloatParam,
    #[id = "vibrato_atk"]
    vibrato_attack: FloatParam,
    #[id = "vibrato_int"]
    vibrato_intensity: FloatParam,
    #[id = "vibrato_rate"]
    vibrato_rate: FloatParam,
    #[id = "tremolo_atk"]
    tremolo_attack: FloatParam,
    #[id = "tremolo_int"]
    tremolo_intensity: FloatParam,
    #[id = "tremolo_rate"]
    tremolo_rate: FloatParam,
    #[id = "vibrato_shape"]
    vibrato_shape: EnumParam<OscillatorShape>,
    #[id = "tremolo_shape"]
    tremolo_shape: EnumParam<OscillatorShape>,
    #[id = "filter_cut_env_pol"]
    filter_cut_env_polarity: BoolParam,
    #[id = "filter_res_env_pol"]
    filter_res_env_polarity: BoolParam,
    #[id = "filter_cut_tension"]
    filter_cut_tension: FloatParam,
    #[id = "filter_res_tension"]
    filter_res_tension: FloatParam,
    #[id = "cutoff_lfo_attack"]
    cutoff_lfo_attack: FloatParam,
    #[id = "res_lfo_attack"]
    res_lfo_attack: FloatParam,
    #[id = "pan_lfo_attack"]
    pan_lfo_attack: FloatParam,
    #[id = "cutoff_lfo_int"]
    cutoff_lfo_intensity: FloatParam,
    #[id = "cutoff_lfo_rate"]
    cutoff_lfo_rate: FloatParam,
    #[id = "cutoff_lfo_shape"]
    cutoff_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "res_lfo_int"]
    res_lfo_intensity: FloatParam,
    #[id = "res_lfo_rate"]
    res_lfo_rate: FloatParam,
    #[id = "res_lfo_shape"]
    res_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "pan_lfo_int"]
    pan_lfo_intensity: FloatParam,
    #[id = "pan_lfo_rate"]
    pan_lfo_rate: FloatParam,
    #[id = "pan_lfo_shape"]
    pan_lfo_shape: EnumParam<OscillatorShape>,
    #[id = "chorus_enable"]
    chorus_enable: BoolParam,
    #[id = "chorus_rate"]
    chorus_rate: FloatParam,
    #[id = "chorus_depth"]
    chorus_depth: FloatParam,
    #[id = "chorus_mix"]
    chorus_mix: FloatParam,
    #[id = "delay_en"]
    delay_enable: BoolParam,
    #[id = "delay_time"]
    delay_time_ms: FloatParam,
    #[id = "delay_fb"]
    delay_feedback: FloatParam,
    #[id = "delay_mix"]
    delay_mix: FloatParam,
    #[id = "rev_en"]
    reverb_enable: BoolParam,
    #[id = "rev_size"]
    reverb_size: FloatParam,
    #[id = "rev_damp"]
    reverb_damp: FloatParam,
    #[id = "rev_diff"]
    reverb_diffusion: FloatParam,
    #[id = "rev_shim"]
    reverb_shimmer: FloatParam,
    #[id = "rev_mix"]
    reverb_mix: FloatParam,
    #[id = "dist_en"]
    dist_enable: BoolParam,
    #[id = "dist_drive"]
    dist_drive: FloatParam,
    #[id = "dist_tone"]
    dist_tone: FloatParam,
    #[id = "dist_magic"]
    dist_magic: FloatParam,
    #[id = "dist_mix"]
    dist_mix: FloatParam,
    #[id = "dist_env_atk"]
    dist_env_attack_ms: FloatParam,
    #[id = "dist_env_hold"]
    dist_env_hold_ms: FloatParam,
    #[id = "dist_env_dec"]
    dist_env_decay_ms: FloatParam,
    #[id = "dist_env_dec2"]
    dist_env_decay2_ms: FloatParam,
    #[id = "dist_env_dec2_lvl"]
    dist_env_decay2_level: FloatParam,
    #[id = "dist_env_sus"]
    dist_env_sustain_level: FloatParam,
    #[id = "dist_env_rel"]
    dist_env_release_ms: FloatParam,
    #[id = "dist_env_amt"]
    dist_env_amount: FloatParam,
    #[id = "eq_en"]
    eq_enable: BoolParam,
    #[id = "eq_low_gain"]
    eq_low_gain: FloatParam,
    #[id = "eq_mid_gain"]
    eq_mid_gain: FloatParam,
    #[id = "eq_mid_freq"]
    eq_mid_freq: FloatParam,
    #[id = "eq_mid_q"]
    eq_mid_q: FloatParam,
    #[id = "eq_high_gain"]
    eq_high_gain: FloatParam,
    #[id = "eq_mix"]
    eq_mix: FloatParam,
    #[id = "out_sat_en"]
    output_sat_enable: BoolParam,
    #[id = "out_sat_type"]
    output_sat_type: EnumParam<OutputSaturationType>,
    #[id = "out_sat_drive"]
    output_sat_drive: FloatParam,
    #[id = "out_sat_mix"]
    output_sat_mix: FloatParam,
    #[id = "mf_en"]
    multi_filter_enable: BoolParam,
    #[id = "mf_route"]
    multi_filter_routing: EnumParam<FilterRouting>,
    #[id = "mf_morph"]
    multi_filter_morph: FloatParam,
    #[id = "mf_par_ab"]
    multi_filter_parallel_ab: FloatParam,
    #[id = "mf_par_c"]
    multi_filter_parallel_c: FloatParam,
    #[id = "mf_a_type"]
    multi_filter_a_type: EnumParam<FilterType>,
    #[id = "mf_a_cut"]
    multi_filter_a_cut: FloatParam,
    #[id = "mf_a_res"]
    multi_filter_a_res: FloatParam,
    #[id = "mf_a_amt"]
    multi_filter_a_amt: FloatParam,
    #[id = "mf_b_type"]
    multi_filter_b_type: EnumParam<FilterType>,
    #[id = "mf_b_cut"]
    multi_filter_b_cut: FloatParam,
    #[id = "mf_b_res"]
    multi_filter_b_res: FloatParam,
    #[id = "mf_b_amt"]
    multi_filter_b_amt: FloatParam,
    #[id = "mf_c_type"]
    multi_filter_c_type: EnumParam<FilterType>,
    #[id = "mf_c_cut"]
    multi_filter_c_cut: FloatParam,
    #[id = "mf_c_res"]
    multi_filter_c_res: FloatParam,
    #[id = "mf_c_amt"]
    multi_filter_c_amt: FloatParam,
    #[id = "limiter_enable"]
    limiter_enable: BoolParam,
    #[id = "limiter_threshold"]
    limiter_threshold: FloatParam,
    #[id = "limiter_release"]
    limiter_release: FloatParam,
}

#[derive(Debug, Clone)]
struct Voice {
    voice_id: i32,
    channel: u8,
    note: u8,
    internal_voice_id: u64,
    velocity: f32,
    velocity_sqrt: f32,
    phase: f32,
    phase_delta: f32,
    target_phase_delta: f32,
    releasing: bool,
    amp_envelope: ADSREnvelope,
    voice_gain: Option<(f32, Smoother<f32>)>,
    filter_cut_envelope: ADSREnvelope,
    filter_res_envelope: ADSREnvelope,
    fm_envelope: ADSREnvelope,
    dist_envelope: ADSREnvelope,
    filter: Option<FilterType>,
    lowpass_filter: filter::LowpassFilter,
    highpass_filter: filter::HighpassFilter,
    bandpass_filter: filter::BandpassFilter,
    notch_filter: filter::NotchFilter,
    statevariable_filter: filter::StatevariableFilter,
    comb_filter: filter::CombFilter,
    rainbow_comb_filter: filter::RainbowCombFilter,
    diode_ladder_lp_filter: filter::DiodeLadderFilter,
    diode_ladder_hp_filter: filter::DiodeLadderFilter,
    ms20_filter: filter::Ms20Filter,
    formant_morph_filter: filter::FormantMorphFilter,
    phaser_filter: filter::PhaserFilter,
    comb_allpass_filter: filter::CombAllpassFilter,
    bitcrush_lp_filter: filter::BitcrushLpFilter,
    pressure: f32,
    pan: f32,        // Added pan field
    tuning: f32,     // Add tuning field
    vibrato: f32,    // Add vibrato field
    expression: f32, // Add expression field
    brightness: f32, // Add brightness field
    vib_mod: Modulator,
    trem_mod: Modulator,
    pan_mod: Modulator,
    mod_lfo1: Modulator,
    mod_lfo2: Modulator,
    drift_offset: f32,
    mod_smooth: [f32; 6],
    fm_feedback_state: f32,
    unison_phases: [f32; 6],
    stereo_prev: f32,
    dc_blocker: filter::DCBlocker,
    sizzle_osc_lp: Biquad,
    sizzle_wt_lp: Biquad,
    alias_lp1: Biquad,
    alias_lp2: Biquad,
    tight_lp: Biquad,
    tight_hp: Biquad,
    ring_phase: f32,
}

impl Default for SubSynth {
    fn default() -> Self {
        let params = Arc::new(SubSynthParams::default());
        let factory_presets = editor::factory_preset_data(&params);

        Self {
            params: params.clone(),

            prng: Pcg32::new(420, 1337),
            voices: [0; NUM_VOICES as usize].map(|_| None),
            next_internal_voice_id: 0,
            next_voice_index: 0,
            chorus: Chorus::new(44100.0),
            delay: StereoDelay::new(44100.0),
            reverb: reverb::Reverb::new(44100.0),
            limiter_left: Limiter::new(),
            limiter_right: Limiter::new(),
            multi_filter: MultiStageFilter::new(44100.0),
            distortion: Distortion::new(44100.0),
            eq: ThreeBandEq::new(44100.0),
            output_saturation: OutputSaturation::new(44100.0),
            spectral_main: SpectralShaper::new(44100.0, 2048, 4),
            spectral_fx: SpectralShaper::new(44100.0, 2048, 4),
            factory_wavetable: WavetableBank::new(),
            custom_wavetable: None,
            custom_wavetable_path: None,
            factory_presets,
            last_preset_index: params.preset_index.value(),
            seq_phase: 0.0,
            last_note_phase_delta: 0.0,
            last_note_active: false,
            ring_mod_post_phase: [0.0; 2],
        }
    }
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

impl Plugin for SubSynth {
    const NAME: &'static str = "DogSynth";
    const VENDOR: &'static str = "DogSynth";
    const URL: &'static str = "https://taellinglin.art";
    const EMAIL: &'static str = "taellinglin@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        // After `PEAK_METER_DECAY_MS` milliseconds of pure silence, the peak meter's value should
        // have dropped by 12 dB
        self.refresh_custom_wavetable();
        context.set_latency_samples(self.spectral_main.latency_samples());
        true
    }

    fn reset(&mut self) {
        self.prng = Pcg32::new(420, 1337);

        self.voices.fill(None);
        self.next_internal_voice_id = 0;
        self.ring_mod_post_phase = [0.0; 2];
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // NIH-plug has a block-splitting adapter for `Buffer`. While this works great for effect
        // plugins, for polyphonic synths the block size should be `min(MAX_BLOCK_SIZE,
        // num_remaining_samples, next_event_idx - block_start_idx)`. Because blocks also need to be
        // split on note events, it's easier to work with raw audio here and to do the splitting by
        // hand.
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let output = buffer.as_slice();

        self.delay.set_sample_rate(sample_rate);
        self.reverb.set_sample_rate(sample_rate);
        self.multi_filter.set_sample_rate(sample_rate);
        self.distortion.set_sample_rate(sample_rate);
        self.eq.set_sample_rate(sample_rate);
        self.output_saturation.set_sample_rate(sample_rate);
        self.spectral_main.set_sample_rate(sample_rate);
        self.spectral_fx.set_sample_rate(sample_rate);
        self.refresh_custom_wavetable();
        self.sync_preset_if_changed();

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);
        while block_start < num_samples {
            // First of all, handle all note events that happen at the start of the block, and cut
            // the block short if another event happens before the end of it. To handle polyphonic
            // modulation for new notes properly, we'll keep track of the next internal note index
            // at the block's start. If we receive polyphonic modulation that matches a voice that
            // has an internal note ID that's great than or equal to this one, then we should start
            // the note's smoother at the new value instead of fading in from the global value.
            let this_sample_internal_voice_id_start = self.next_internal_voice_id;
            'events: loop {
                match next_event {
                    // If the event happens now, then we'll keep processing events
                    Some(event) if (event.timing() as usize) < block_end => {
                        // This synth doesn't support any of the polyphonic expression events. A
                        // real synth plugin, however, will want to support those.
                        match event {
                            NoteEvent::NoteOn {
                                timing,
                                voice_id,
                                channel,
                                note,
                                velocity,
                            } => {
                                let pan: f32 = 0.5;
                                let pressure: f32 = 1.0;
                                let brightness: f32 = 1.0;
                                let expression: f32 = 1.0;
                                let vibrato: f32 = 0.0;
                                let tuning: f32 = 0.0;
                                let initial_phase: f32 = self.prng.gen();
                                let vibrato_lfo = Modulator::new(
                                    self.params.vibrato_rate.value(), 
                                    self.params.vibrato_intensity.value(), 
                                    self.params.vibrato_attack.value(), 
                                    self.params.vibrato_shape.value(),
                                );
                                let tremolo_lfo = Modulator::new(
                                    self.params.tremolo_rate.value(), 
                                    self.params.tremolo_intensity.value(), 
                                    self.params.tremolo_attack.value(), 
                                    self.params.tremolo_shape.value(),
                                );
                                let pan_lfo = Modulator::new(
                                    self.params.pan_lfo_rate.value(),
                                    self.params.pan_lfo_intensity.value(),
                                    self.params.pan_lfo_attack.value(),
                                    self.params.pan_lfo_shape.value(),
                                );
                                let mod_lfo1 = Modulator::new(
                                    self.params.lfo1_rate.value(),
                                    1.0,
                                    self.params.lfo1_attack.value(),
                                    self.params.lfo1_shape.value(),
                                );
                                let mod_lfo2 = Modulator::new(
                                    self.params.lfo2_rate.value(),
                                    1.0,
                                    self.params.lfo2_attack.value(),
                                    self.params.lfo2_shape.value(),
                                );
                                let pitch = util::midi_note_to_freq(note)
                                    * (2.0_f32).powf((tuning + 0.0) / 12.0);
                                let target_phase_delta = pitch / sample_rate;
                                let glide_mode = self.params.glide_mode.value();
                                let last_note_active = self.last_note_active;
                                let last_note_phase_delta = self.last_note_phase_delta;
                                let use_glide = match glide_mode {
                                    GlideMode::Off => false,
                                    GlideMode::Always => true,
                                    GlideMode::Legato => last_note_active,
                                };
                                let start_phase_delta =
                                    if use_glide && last_note_phase_delta > 0.0 {
                                        last_note_phase_delta
                                    } else {
                                        target_phase_delta
                                    };
                                // This starts with the attack portion of the amplitude envelope
                                let (amp_envelope, cutoff_envelope, resonance_envelope) =
                                    self.construct_envelopes(sample_rate, velocity);
                                let fm_envelope = ADSREnvelope::new(
                                    self.params.fm_env_attack_ms.value(),
                                    self.params.fm_env_hold_ms.value(),
                                    self.params.fm_env_decay_ms.value(),
                                    self.params.fm_env_decay2_ms.value(),
                                    self.params.fm_env_decay2_level.value(),
                                    self.params.fm_env_sustain_level.value(),
                                    self.params.fm_env_release_ms.value(),
                                    sample_rate,
                                    velocity,
                                    0.0,
                                );
                                let dist_envelope = ADSREnvelope::new(
                                    self.params.dist_env_attack_ms.value(),
                                    self.params.dist_env_hold_ms.value(),
                                    self.params.dist_env_decay_ms.value(),
                                    self.params.dist_env_decay2_ms.value(),
                                    self.params.dist_env_decay2_level.value(),
                                    self.params.dist_env_sustain_level.value(),
                                    self.params.dist_env_release_ms.value(),
                                    sample_rate,
                                    velocity,
                                    0.0,
                                );
                                {
                                    let voice = self.start_voice(
                                        context, timing, voice_id, channel, note,
                                        velocity, // Add velocity parameter
                                        pan, pressure, brightness, expression, // Add expression parameter
                                        vibrato,    // Add vibrato parameter
                                        tuning,
                                        vibrato_lfo,
                                        tremolo_lfo,
                                        mod_lfo1,
                                        mod_lfo2,
                                        amp_envelope,
                                        cutoff_envelope,
                                        resonance_envelope,
                                        fm_envelope,
                                        dist_envelope,
                                        self.params.filter_type.value(),
                                        sample_rate,  // Pass actual sample rate
                                    );
                                    
                                    voice.vib_mod = vibrato_lfo.clone();
                                    voice.trem_mod = tremolo_lfo.clone();
                                    voice.pan_mod = pan_lfo.clone();
                                    voice.mod_lfo1 = mod_lfo1.clone();
                                    voice.mod_lfo2 = mod_lfo2.clone();
                                    voice.velocity_sqrt = velocity.sqrt();
                                    voice.phase = initial_phase;
                                    voice.vib_mod.trigger();
                                    voice.trem_mod.trigger();
                                    voice.pan_mod.trigger();
                                    voice.mod_lfo1.trigger();
                                    voice.mod_lfo2.trigger();
                                    voice.phase_delta = start_phase_delta;
                                    voice.target_phase_delta = target_phase_delta;
                                    voice.amp_envelope = amp_envelope;
                                    voice.filter_cut_envelope = cutoff_envelope;
                                    voice.filter_res_envelope = resonance_envelope;
                                    voice.fm_envelope = fm_envelope;
                                    voice.dist_envelope = dist_envelope;
                                    voice.velocity = velocity;
                                    voice.pan = pan;
                                    voice.unison_phases = [initial_phase; 6];
                                    voice.stereo_prev = 0.0;
                                }

                                self.last_note_phase_delta = target_phase_delta;
                                self.last_note_active = true;

                                
                            }
                            NoteEvent::NoteOff {
                                timing: _,
                                voice_id,
                                channel,
                                note,
                                velocity: _,
                            } => {
                                self.start_release_for_voices(sample_rate, voice_id, channel, note);
                            }
                            NoteEvent::Choke {
                                timing,
                                voice_id,
                                channel,
                                note,
                            } => {
                                self.choke_voices(context, timing, voice_id, channel, note);
                            }
                            NoteEvent::PolyModulation {
                                timing: _,
                                voice_id,
                                poly_modulation_id,
                                normalized_offset,
                            } => {
                                // Polyphonic modulation events are matched to voices using the
                                // voice ID, and to parameters using the poly modulation ID. The
                                // host will probably send a modulation event every N samples. This
                                // will happen before the voice is active, and of course also after
                                // it has been terminated (because the host doesn't know that it
                                // will be). Because of that, we won't print any assertion failures
                                // when we can't find the voice index here.
                                if let Some(voice_idx) = self.get_voice_idx(voice_id) {
                                    let voice = self.voices[voice_idx].as_mut().unwrap();

                                    match poly_modulation_id {
                                        GAIN_POLY_MOD_ID => {
                                            // This should either create a smoother for this
                                            // modulated parameter or update the existing one.
                                            // Notice how this uses the parameter's unmodulated
                                            // normalized value in combination with the normalized
                                            // offset to create the target plain value
                                            let target_plain_value = self
                                                .params
                                                .gain
                                                .preview_modulated(normalized_offset);
                                            let (_, smoother) =
                                                voice.voice_gain.get_or_insert_with(|| {
                                                    (
                                                        normalized_offset,
                                                        self.params.gain.smoothed.clone(),
                                                    )
                                                });

                                            // If this `PolyModulation` events happens on the
                                            // same sample as a voice's `NoteOn` event, then it
                                            // should immediately use the modulated value
                                            // instead of slowly fading in
                                            if voice.internal_voice_id
                                                >= this_sample_internal_voice_id_start
                                            {
                                                smoother.reset(target_plain_value);
                                            } else {
                                                smoother
                                                    .set_target(sample_rate, target_plain_value);
                                            }
                                        }
                                        n => nih_debug_assert_failure!(
                                            "Polyphonic modulation sent for unknown poly \
                                            modulation ID {}",
                                            n
                                        ),
                                    }
                                }
                            }
                            NoteEvent::MonoAutomation {
                                timing: _,
                                poly_modulation_id,
                                normalized_value,
                            } => {
                                // Modulation always acts as an offset to the parameter's current
                                // automated value. So if the host sends a new automation value for
                                // a modulated parameter, the modulated values/smoothing targets
                                // need to be updated for all polyphonically modulated voices.
                                for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                                    match poly_modulation_id {
                                        GAIN_POLY_MOD_ID => {
                                            let (normalized_offset, smoother) =
                                                match voice.voice_gain.as_mut() {
                                                    Some((o, s)) => (o, s),
                                                    // If the voice does not have existing
                                                    // polyphonic modulation, then there's nothing
                                                    // to do here. The global automation/monophonic
                                                    // modulation has already been taken care of by
                                                    // the framework.
                                                    None => continue,
                                                };
                                            let target_plain_value =
                                                self.params.gain.preview_plain(
                                                    normalized_value + *normalized_offset,
                                                );
                                            smoother.set_target(sample_rate, target_plain_value);
                                        }
                                        n => nih_debug_assert_failure!(
                                            "Automation event sent for unknown poly modulation ID \
                                            {}",
                                            n
                                        ),
                                    }
                                }
                            }
                            NoteEvent::PolyPressure {
                                timing,
                                voice_id,
                                channel,
                                note,
                                pressure,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice.as_mut() {
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                0.0,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyVolume {
                                timing,
                                voice_id,
                                channel,
                                note,
                                gain,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyPan {
                                timing,
                                voice_id,
                                channel,
                                note,
                                pan,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let gain = voice_inner.velocity;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyTuning {
                                timing,
                                voice_id,
                                channel,
                                note,
                                tuning,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let gain = voice_inner.velocity;
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyVibrato {
                                timing,
                                voice_id,
                                channel,
                                note,
                                vibrato,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let gain = voice_inner.velocity;
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }

                            NoteEvent::MidiProgramChange { program, .. } => {
                                self.apply_factory_preset(program as usize, true);
                            }
                            
                            
                            // Handle other MIDI events if needed
                            _ => (),
                        };

                        next_event = context.next_event();
                    }
                    // If the event happens before the end of the block, then the block should be cut
                    // short so the next block starts at the event
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            // These are the smoothed global parameter values. These are used for voices that do not
            // have polyphonic modulation applied to them. With a plugin as simple as this it would
            // be possible to avoid this completely by simply always copying the smoother into the
            // voice's struct, but that may not be realistic when the plugin has hundreds of
            // parameters. The `voice_*` arrays are scratch arrays that an individual voice can use.
            let block_len = block_end - block_start;
            let mut gain = [0.0; MAX_BLOCK_SIZE];
            let mut voice_gain = [0.0; MAX_BLOCK_SIZE];
            let mut seq_gate_values = [1.0; MAX_BLOCK_SIZE];
            let mut seq_dist_values = [0.0; MAX_BLOCK_SIZE];
            let mut dist_env_values = [0.0; MAX_BLOCK_SIZE];
            let mut fx_left = [0.0; MAX_BLOCK_SIZE];
            let mut fx_right = [0.0; MAX_BLOCK_SIZE];

            // We'll start with silence, and then add the output from the active voices
            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);
            fx_left[..block_len].fill(0.0);
            fx_right[..block_len].fill(0.0);
            self.params.gain.smoothed.next_block(&mut gain, block_len);

            // TODO: Some form of band limiting
            // TODO: Filter
            for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
                let seq_enable = self.params.seq_enable.value();
                let (seq_gate, seq_cut, seq_res, seq_wt, seq_dist, seq_fm) = if seq_enable {
                    let tempo = context.transport().tempo.unwrap_or(120.0) as f32;
                    let step_rate = (tempo / 60.0) * self.params.seq_rate.value();
                    let step_idx = (self.seq_phase.floor() as usize) % 32;
                    let gate = self.params.seq_lanes[0].steps[step_idx].value.value();
                    let cut = self.params.seq_lanes[1].steps[step_idx].value.value();
                    let res = self.params.seq_lanes[2].steps[step_idx].value.value();
                    let wt = self.params.seq_lanes[3].steps[step_idx].value.value();
                    let dist = self.params.seq_lanes[4].steps[step_idx].value.value();
                    let fm = self.params.seq_lanes[5].steps[step_idx].value.value();

                    let phase_inc = step_rate / sample_rate;
                    self.seq_phase += phase_inc;
                    if self.seq_phase >= 32.0 {
                        self.seq_phase -= 32.0;
                    }

                    let gate_amount = self.params.seq_gate_amount.value();
                    let gate_step = (gate * 0.5 + 0.5).clamp(0.0, 1.0);
                    let gate_value = (1.0 - gate_amount) + gate_amount * gate_step;

                    (
                        gate_value,
                        cut * self.params.seq_cut_amount.value(),
                        res * self.params.seq_res_amount.value(),
                        wt * self.params.seq_wt_amount.value(),
                        dist * self.params.seq_dist_amount.value(),
                        fm * self.params.seq_fm_amount.value(),
                    )
                } else {
                    (1.0, 0.0, 0.0, 0.0, 0.0, 0.0)
                };

                seq_gate_values[value_idx] = seq_gate;
                seq_dist_values[value_idx] = seq_dist;

                // Get mutable reference to the voice at sample_idx
                for voice in self.voices.iter_mut() {
                    if let Some(voice) = voice {
                        // Depending on whether the voice has polyphonic modulation applied to it,
                        // either the global parameter values are used, or the voice's smoother is used
                        // to generate unique modulated values for that voice
                        let _gain = match &voice.voice_gain {
                            Some((_, smoother)) => {
                                smoother.next_block(&mut voice_gain, block_len);
                                &voice_gain
                            }
                            None => &gain,
                        };

                        // This is an exponential smoother repurposed as an AR envelope with values between
                        // 0 and 1. When a note off event is received, this envelope will start fading out
                        // again. When it reaches 0, we will terminate the voice.
                        
                        
                        // Apply filter
                        let filter_type = self.params.filter_type.value();
                        voice.filter = Some(filter_type);
                        let cutoff = self.params.filter_cut.value();
                        let resonance = self.params.filter_res.value();
                        let waveform = self.params.waveform.value();
                        let osc_routing = self.params.osc_routing.value();
                        let osc_blend = self.params.osc_blend.value();
                        let wavetable_position_base = self.params.wavetable_position.value();
                        let wavetable_distortion = self.params.wavetable_distortion.value();
                        let fm_env_amount = self.params.fm_env_amount.value();
                        let dist_env_amount = self.params.dist_env_amount.value();
                        let analog_enable = self.params.analog_enable.value();
                        let analog_drive = self.params.analog_drive.value();
                        let analog_noise = self.params.analog_noise.value();
                        let analog_drift = self.params.analog_drift.value();
                        let sub_level = self.params.sub_level.value();
                        let vib_int: f32 = self.params.vibrato_intensity.value();
                        let pan_lfo = voice.pan_mod.get_modulation(sample_rate);
                        let glide_time = self.params.glide_time_ms.value();
                        if glide_time > 0.0 {
                            let coeff = (-1.0_f32 / (glide_time * 0.001 * sample_rate)).exp();
                            voice.phase_delta = voice.phase_delta * coeff
                                + voice.target_phase_delta * (1.0 - coeff);
                        } else {
                            voice.phase_delta = voice.target_phase_delta;
                        }
                        // Vibrato modulation (LFO-based)
                        let vibrato_modulation = voice.vib_mod.get_modulation(sample_rate);
                        // Apply vibrato to the voice's phase_delta (which affects pitch)
                        let vibrato_phase_delta =
                            voice.phase_delta * (1.0 + (vib_int * vibrato_modulation));
                        if analog_enable && analog_drift > 0.0 {
                            let jitter = (self.prng.gen::<f32>() - 0.5) * analog_drift * 0.0005;
                            voice.drift_offset = (voice.drift_offset + jitter).clamp(-0.02, 0.02);
                        }
                        let drifted_phase_delta =
                            vibrato_phase_delta * (1.0 + voice.drift_offset);
                        //filtered_sample.set_sample_rate(sample_rate);
                        // Advance envelopes once per sample
                        voice.amp_envelope.advance();
                        voice.filter_cut_envelope.advance();
                        voice.filter_res_envelope.advance();
                        voice.fm_envelope.advance();
                        voice.dist_envelope.advance();

                        // Get envelope values (scaled from 0-1)
                        let amp_env_value = voice.amp_envelope.get_value();
                        let filter_cut_env_value = voice.filter_cut_envelope.get_value();
                        let filter_res_env_value = voice.filter_res_envelope.get_value();
                        let fm_env_value = voice.fm_envelope.get_value();
                        let dist_env_value = voice.dist_envelope.get_value();

                        let lfo1_mod = voice.mod_lfo1.get_modulation(sample_rate);
                        let lfo2_mod = voice.mod_lfo2.get_modulation(sample_rate);
                        let mut wavetable_pos_mod = 0.0;
                        let mut filter_cut_mod = 0.0;
                        let mut filter_res_mod = 0.0;
                        let mut filter_amount_mod = 0.0;
                        let mut pan_mod_extra = 0.0;
                        let mut gain_mod = 0.0;
                        let mut fm_amount_mod = 0.0;
                        let mut fm_ratio_mod = 0.0;
                        let mut fm_feedback_mod = 0.0;

                        let mut apply_mod = |
                            slot: usize,
                            source: ModSource,
                            target: ModTarget,
                            amount: f32,
                            smooth_ms: f32,
                        | {
                            let source_value = match source {
                                ModSource::Lfo1 => lfo1_mod,
                                ModSource::Lfo2 => lfo2_mod,
                                ModSource::AmpEnv => amp_env_value,
                                ModSource::FilterEnv => filter_cut_env_value,
                            };
                            let mod_value = source_value * amount;
                            let mod_value = if smooth_ms > 0.0 {
                                let coeff = (-1.0 / (smooth_ms * 0.001 * sample_rate)).exp();
                                let prev = voice.mod_smooth[slot];
                                let smoothed = prev * coeff + mod_value * (1.0 - coeff);
                                voice.mod_smooth[slot] = smoothed;
                                smoothed
                            } else {
                                voice.mod_smooth[slot] = mod_value;
                                mod_value
                            };
                            match target {
                                ModTarget::WavetablePos => wavetable_pos_mod += mod_value,
                                ModTarget::FilterCut => filter_cut_mod += mod_value,
                                ModTarget::FilterRes => filter_res_mod += mod_value,
                                ModTarget::FilterAmount => filter_amount_mod += mod_value,
                                ModTarget::Pan => pan_mod_extra += mod_value,
                                ModTarget::Gain => gain_mod += mod_value,
                                ModTarget::FmAmount => fm_amount_mod += mod_value,
                                ModTarget::FmRatio => fm_ratio_mod += mod_value,
                                ModTarget::FmFeedback => fm_feedback_mod += mod_value,
                            }
                        };

                        apply_mod(
                            0,
                            self.params.mod1_source.value(),
                            self.params.mod1_target.value(),
                            self.params.mod1_amount.value(),
                            self.params.mod1_smooth_ms.value(),
                        );
                        apply_mod(
                            1,
                            self.params.mod2_source.value(),
                            self.params.mod2_target.value(),
                            self.params.mod2_amount.value(),
                            self.params.mod2_smooth_ms.value(),
                        );
                        apply_mod(
                            2,
                            self.params.mod3_source.value(),
                            self.params.mod3_target.value(),
                            self.params.mod3_amount.value(),
                            self.params.mod3_smooth_ms.value(),
                        );
                        apply_mod(
                            3,
                            self.params.mod4_source.value(),
                            self.params.mod4_target.value(),
                            self.params.mod4_amount.value(),
                            self.params.mod4_smooth_ms.value(),
                        );
                        apply_mod(
                            4,
                            self.params.mod5_source.value(),
                            self.params.mod5_target.value(),
                            self.params.mod5_amount.value(),
                            self.params.mod5_smooth_ms.value(),
                        );
                        apply_mod(
                            5,
                            self.params.mod6_source.value(),
                            self.params.mod6_target.value(),
                            self.params.mod6_amount.value(),
                            self.params.mod6_smooth_ms.value(),
                        );

                        wavetable_pos_mod += seq_wt;
                        filter_cut_mod += seq_cut;
                        filter_res_mod += seq_res;
                        fm_amount_mod += seq_fm;
                        fm_amount_mod += fm_env_value * fm_env_amount;
                        dist_env_values[value_idx] += dist_env_value * dist_env_amount;

                        let wavetable_position =
                            (wavetable_position_base + wavetable_pos_mod).clamp(0.0, 1.0);
                        let pan = (voice.pan + pan_lfo + pan_mod_extra).clamp(0.0, 1.0);
                        let left_amp = (1.0 - pan).sqrt() as f32;
                        let right_amp = pan.sqrt() as f32;

                        // Generate waveform for voice
                        let fm_enable = self.params.fm_enable.value();
                        let fm_amount = (self.params.fm_amount.value() + fm_amount_mod)
                            .clamp(-1.0, 1.0);
                        let fm_ratio = (self.params.fm_ratio.value() + fm_ratio_mod)
                            .clamp(0.25, 8.0);
                        let fm_feedback = (self.params.fm_feedback.value() + fm_feedback_mod)
                            .clamp(0.0, 0.99);
                        let fm_source = self.params.fm_source.value();
                        let fm_target = self.params.fm_target.value();
                        let unison_voices = self.params.unison_voices.value();
                        let unison_detune = self.params.unison_detune.value();
                        let unison_spread = self.params.unison_spread.value();
                        let classic_drive = self.params.classic_drive.value();
                        let base_phase = voice.unison_phases[0];
                        let fm_signal = if fm_enable {
                            let mod_phase = (base_phase * fm_ratio
                                + voice.fm_feedback_state * fm_feedback)
                                .fract();
                            let mod_sample = match fm_source {
                                FmSource::Classic => generate_waveform(waveform, mod_phase),
                                FmSource::Wavetable => {
                                    let wavetable_bank = if self.params.custom_wavetable_enable.value() {
                                        self.custom_wavetable
                                            .as_ref()
                                            .unwrap_or(&self.factory_wavetable)
                                    } else {
                                        &self.factory_wavetable
                                    };
                                    wavetable_bank.sample(mod_phase, wavetable_position)
                                }
                                FmSource::Sub => (2.0 * std::f32::consts::PI * mod_phase).sin(),
                            };
                            voice.fm_feedback_state = mod_sample;
                            mod_sample * fm_amount * 0.25
                        } else {
                            voice.fm_feedback_state = 0.0;
                            0.0
                        };
                        let unison_count = match unison_voices {
                            UnisonVoices::One => 1,
                            UnisonVoices::Two => 2,
                            UnisonVoices::Four => 4,
                            UnisonVoices::Six => 6,
                        };
                        let detune_cents = unison_detune * 30.0;
                        let offsets: &[f32] = match unison_count {
                            1 => &[0.0],
                            2 => &[-0.5, 0.5],
                            4 => &[-0.75, -0.25, 0.25, 0.75],
                            _ => &[-1.0, -0.6, -0.2, 0.2, 0.6, 1.0],
                        };

                        let wavetable_bank = if self.params.custom_wavetable_enable.value() {
                            self.custom_wavetable
                                .as_ref()
                                .unwrap_or(&self.factory_wavetable)
                        } else {
                            &self.factory_wavetable
                        };

                        let mut classic_sum = 0.0;
                        let mut wavetable_sum = 0.0;
                        for i in 0..unison_count {
                            let offset = offsets[i];
                            let ratio = 2.0_f32.powf(detune_cents * offset / 1200.0);
                            let phase = voice.unison_phases[i];
                            let classic_phase = if fm_enable
                                && matches!(fm_target, FmTarget::Classic | FmTarget::Both)
                            {
                                (phase + fm_signal).fract()
                            } else {
                                phase
                            };
                            let wavetable_phase = if fm_enable
                                && matches!(fm_target, FmTarget::Wavetable | FmTarget::Both)
                            {
                                (phase + fm_signal).fract()
                            } else {
                                phase
                            };

                            let mut classic_sample = generate_waveform(waveform, classic_phase);
                            classic_sample = SubSynth::wavefold(classic_sample, classic_drive);
                            classic_sample -= SubSynth::poly_blep(phase, drifted_phase_delta * ratio);

                            let mut wavetable_sample =
                                wavetable_bank.sample(wavetable_phase, wavetable_position);
                            wavetable_sample = SubSynth::wavefold(wavetable_sample, wavetable_distortion);

                            classic_sum += classic_sample;
                            wavetable_sum += wavetable_sample;

                            let next_phase = phase + drifted_phase_delta * ratio;
                            voice.unison_phases[i] = if next_phase >= 1.0 {
                                next_phase - 1.0
                            } else {
                                next_phase
                            };
                        }
                        let classic_sum = classic_sum / unison_count as f32;
                        let wavetable_sum = wavetable_sum / unison_count as f32;

                        let sizzle_cutoff = self.params.sizzle_cutoff.value();
                        let sizzle_osc_enable = self.params.sizzle_osc_enable.value();
                        let sizzle_wt_enable = self.params.sizzle_wt_enable.value();
                        let filter_tight_enable = self.params.filter_tight_enable.value();
                        let classic_level = self.params.classic_level.value();
                        let wavetable_level = self.params.wavetable_level.value();
                        let noise_level = self.params.noise_level.value();
                        let ring_enable = self.params.ring_mod_enable.value();
                        let ring_source = self.params.ring_mod_source.value();
                        let ring_place = self.params.ring_mod_placement.value();
                        let ring_level = self.params.ring_mod_level.value();
                        let ring_freq = self.params.ring_mod_freq.value();

                        let mut classic_sample = classic_sum;
                        if sizzle_osc_enable {
                            voice
                                .sizzle_osc_lp
                                .set_lowpass(sample_rate, sizzle_cutoff, 0.7);
                            classic_sample = voice.sizzle_osc_lp.process(classic_sample);
                        }
                        let mut wavetable_sample = wavetable_sum;
                        if sizzle_wt_enable {
                            voice
                                .sizzle_wt_lp
                                .set_lowpass(sample_rate, sizzle_cutoff, 0.7);
                            wavetable_sample = voice.sizzle_wt_lp.process(wavetable_sample);
                        }
                        classic_sample *= classic_level;
                        wavetable_sample *= wavetable_level;

                        let mut sub_sample = 0.0;
                        if sub_level > 0.0 {
                            let sub_phase = (base_phase * 0.5).fract();
                            sub_sample = (2.0 * std::f32::consts::PI * sub_phase).sin()
                                * sub_level;
                        }

                        let noise_sample = if noise_level > 0.0 {
                            (self.prng.gen::<f32>() * 2.0 - 1.0) * noise_level
                        } else {
                            0.0
                        };

                        let mut pre_filter_mix = match osc_routing {
                            OscRouting::ClassicOnly => classic_sample,
                            OscRouting::WavetableOnly => wavetable_sample,
                            OscRouting::Blend => {
                                classic_sample * (1.0 - osc_blend) + wavetable_sample * osc_blend
                            }
                        };
                        pre_filter_mix += sub_sample + noise_sample;

                        if sizzle_osc_enable || sizzle_wt_enable {
                            let alias_cutoff = sizzle_cutoff.min(sample_rate * 0.45);
                            voice.alias_lp1.set_lowpass(sample_rate, alias_cutoff, 0.7);
                            voice.alias_lp2.set_lowpass(sample_rate, alias_cutoff, 0.7);
                            pre_filter_mix = voice.alias_lp2.process(voice.alias_lp1.process(pre_filter_mix));
                        }

                        let mut ring_carrier = 0.0;
                        if ring_enable {
                            ring_carrier = match ring_source {
                                RingModSource::Sine => {
                                    (2.0 * std::f32::consts::PI * voice.ring_phase).sin()
                                }
                                RingModSource::Classic => classic_sum,
                                RingModSource::Wavetable => wavetable_sum,
                            };
                            let ring_phase_next = voice.ring_phase + ring_freq / sample_rate;
                            voice.ring_phase = ring_phase_next.fract();
                        }

                        if ring_enable && ring_level > 0.0 && ring_place == RingModPlacement::PreFilter {
                            let ring_sample = pre_filter_mix * ring_carrier;
                            pre_filter_mix += ring_sample * ring_level;
                        }
                        
                        // Apply envelope modulation to filter parameters
                        // Envelope level controls the depth of modulation (0-1 range)
                        let env_cut_amount = self.params.filter_cut_envelope_level.value().max(0.0).min(1.0);
                        let env_res_amount = self.params.filter_res_envelope_level.value().max(0.0).min(1.0);
                        
                        // Modulate cutoff and resonance
                        // When env_amount = 0: use base value only
                        // When env_amount = 1: envelope fully controls the parameter (0 to base value)
                        // Formula: base * (1 - amount + amount * envelope)
                        let cutoff_base = (cutoff * (1.0 + filter_cut_mod)).clamp(20.0, 20000.0);
                        let resonance_base = (resonance + filter_res_mod).clamp(0.0, 1.0);
                        let cutoff_multiplier =
                            1.0 - env_cut_amount + (env_cut_amount * filter_cut_env_value);
                        let modulated_cutoff = cutoff_base * cutoff_multiplier;
                        
                        let res_multiplier =
                            1.0 - env_res_amount + (env_res_amount * filter_res_env_value);
                        let modulated_resonance = resonance_base * res_multiplier;
                        
                        // Clamp to valid ranges
                        let modulated_cutoff = modulated_cutoff.max(20.0).min(20000.0);
                        let modulated_resonance = modulated_resonance.max(0.0).min(1.0);
                        
                        // Apply filters using stored filter instances
                        let filtered_sample = match voice.filter.unwrap() {
                            FilterType::None => pre_filter_mix,
                            FilterType::Lowpass => {
                                voice.lowpass_filter.set_cutoff(modulated_cutoff);
                                voice.lowpass_filter.set_resonance(modulated_resonance);
                                voice.lowpass_filter.process(pre_filter_mix)
                            }
                            FilterType::Highpass => {
                                voice.highpass_filter.set_cutoff(modulated_cutoff);
                                voice.highpass_filter.set_resonance(modulated_resonance);
                                voice.highpass_filter.process(pre_filter_mix)
                            }
                            FilterType::Bandpass => {
                                voice.bandpass_filter.set_cutoff(modulated_cutoff);
                                voice.bandpass_filter.set_resonance(modulated_resonance);
                                voice.bandpass_filter.process(pre_filter_mix)
                            }
                            FilterType::Notch => {
                                voice.notch_filter.set_cutoff(modulated_cutoff);
                                voice.notch_filter.set_resonance(modulated_resonance);
                                voice.notch_filter.process(pre_filter_mix)
                            }
                            FilterType::Statevariable => {
                                voice.statevariable_filter.set_cutoff(modulated_cutoff);
                                voice.statevariable_filter.set_resonance(modulated_resonance);
                                voice.statevariable_filter.process(pre_filter_mix)
                            }
                            FilterType::Comb => {
                                voice.comb_filter.set_cutoff(modulated_cutoff);
                                voice.comb_filter.set_resonance(modulated_resonance);
                                voice.comb_filter.process(pre_filter_mix)
                            }
                            FilterType::RainbowComb => {
                                voice.rainbow_comb_filter.set_cutoff(modulated_cutoff);
                                voice.rainbow_comb_filter
                                    .set_resonance(modulated_resonance);
                                voice.rainbow_comb_filter.process(pre_filter_mix)
                            }
                            FilterType::DiodeLadderLp => {
                                voice.diode_ladder_lp_filter.set_cutoff(modulated_cutoff);
                                voice.diode_ladder_lp_filter
                                    .set_resonance(modulated_resonance);
                                voice.diode_ladder_lp_filter.process(pre_filter_mix)
                            }
                            FilterType::DiodeLadderHp => {
                                voice.diode_ladder_hp_filter.set_cutoff(modulated_cutoff);
                                voice.diode_ladder_hp_filter
                                    .set_resonance(modulated_resonance);
                                voice.diode_ladder_hp_filter.process(pre_filter_mix)
                            }
                            FilterType::Ms20Pair => {
                                voice.ms20_filter.set_cutoff(modulated_cutoff);
                                voice.ms20_filter.set_resonance(modulated_resonance);
                                voice.ms20_filter.process(pre_filter_mix)
                            }
                            FilterType::FormantMorph => {
                                voice.formant_morph_filter.set_cutoff(modulated_cutoff);
                                voice.formant_morph_filter
                                    .set_resonance(modulated_resonance);
                                voice.formant_morph_filter.process(pre_filter_mix)
                            }
                            FilterType::Phaser => {
                                voice.phaser_filter.set_cutoff(modulated_cutoff);
                                voice.phaser_filter.set_resonance(modulated_resonance);
                                voice.phaser_filter.process(pre_filter_mix)
                            }
                            FilterType::CombAllpass => {
                                voice.comb_allpass_filter.set_cutoff(modulated_cutoff);
                                voice.comb_allpass_filter
                                    .set_resonance(modulated_resonance);
                                voice.comb_allpass_filter.process(pre_filter_mix)
                            }
                            FilterType::BitcrushLp => {
                                voice.bitcrush_lp_filter.set_cutoff(modulated_cutoff);
                                voice.bitcrush_lp_filter
                                    .set_resonance(modulated_resonance);
                                voice.bitcrush_lp_filter.process(pre_filter_mix)
                            }
                        };
                        let mut filtered_sample = if matches!(voice.filter.unwrap(), FilterType::None) {
                            filtered_sample
                        } else {
                            filter::tame_resonance(filtered_sample, modulated_resonance)
                        };

                        if filter_tight_enable {
                            match voice.filter.unwrap() {
                                FilterType::Lowpass
                                | FilterType::DiodeLadderLp
                                | FilterType::BitcrushLp => {
                                    voice
                                        .tight_lp
                                        .set_lowpass(sample_rate, modulated_cutoff, 0.7);
                                    filtered_sample = voice.tight_lp.process(filtered_sample);
                                }
                                FilterType::Highpass | FilterType::DiodeLadderHp => {
                                    voice
                                        .tight_hp
                                        .set_highpass(sample_rate, modulated_cutoff, 0.7);
                                    filtered_sample = voice.tight_hp.process(filtered_sample);
                                }
                                _ => {}
                            }
                        }
                        
                        // Apply filter amount (dry/wet blend)
                        let filter_amount =
                            (self.params.filter_amount.value() + filter_amount_mod)
                                .clamp(0.0, 1.0);
                        let mut final_sample = pre_filter_mix * (1.0 - filter_amount)
                            + filtered_sample * filter_amount;
                        if ring_enable && ring_level > 0.0 && ring_place == RingModPlacement::PostFilter {
                            let ring_sample = final_sample * ring_carrier;
                            final_sample += ring_sample * ring_level;
                        }

                        // Calculate amplitude for voice with envelope level scaling
                        let amp_env_level = self.params.amp_envelope_level.value();
                        let gain_mod = (1.0 + gain_mod).clamp(0.0, 2.0);
                        let seq_gate = seq_gate_values[value_idx];
                        let amp = voice.velocity_sqrt
                            * (amp_env_value * amp_env_level)
                            * 0.5
                            * (voice.trem_mod.get_modulation(sample_rate) + 1.0)
                            * gain_mod
                            * seq_gate;
            
                        // Apply voice-specific processing to the filtered sample
                        let naive_waveform = final_sample;
                        let corrected_waveform = naive_waveform - SubSynth::poly_blep(voice.phase, voice.phase_delta);
                        let mut processed_sample = corrected_waveform * amp;
                        if analog_enable {
                            if analog_noise > 0.0 {
                                let noise = (self.prng.gen::<f32>() * 2.0 - 1.0) * analog_noise;
                                processed_sample += noise;
                            }
                            if analog_drive > 0.0 {
                                let drive = 1.0 + analog_drive * 4.0;
                                processed_sample = (processed_sample * drive).tanh() / drive;
                            }
                        }

                        let classic_send = self.params.classic_send.value();
                        let wavetable_send = self.params.wavetable_send.value();
                        let sub_send = self.params.sub_send.value();
                        let noise_send = self.params.noise_send.value();
                        let ring_send = self.params.ring_mod_send.value();
                        let ring_in_mix = if ring_enable && ring_place != RingModPlacement::PostFx {
                            ring_level
                        } else {
                            0.0
                        };
                        let send_sum = classic_level * classic_send
                            + wavetable_level * wavetable_send
                            + sub_level * sub_send
                            + noise_level * noise_send
                            + ring_in_mix * ring_send;
                        let source_sum = classic_level + wavetable_level + sub_level + noise_level + ring_in_mix;
                        let send_scale = if source_sum > 0.0 {
                            (send_sum / source_sum).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let fx_sample = processed_sample * send_scale;

                        // Calculate panning based on voice's pan value
                        // Apply panning and DC blocking
                        let dc_blocked_sample = voice.dc_blocker.process(processed_sample);
                        let spread = unison_spread.clamp(0.0, 1.0);
                        let diff = dc_blocked_sample - voice.stereo_prev;
                        voice.stereo_prev = dc_blocked_sample;
                        let left_wide = dc_blocked_sample + diff * spread;
                        let right_wide = dc_blocked_sample - diff * spread;
                        let processed_left_sample = left_amp * left_wide;
                        let processed_right_sample = right_amp * right_wide;
                        let fx_left_sample = left_amp * fx_sample;
                        let fx_right_sample = right_amp * fx_sample;

                        // Add the processed sample to the output channels
                        output[0][sample_idx] += processed_left_sample;
                        output[1][sample_idx] += processed_right_sample;
                        fx_left[value_idx] += fx_left_sample;
                        fx_right[value_idx] += fx_right_sample;

                        // Update voice phase from unison base
                        voice.phase = voice.unison_phases[0];
                    }
                }
            }

            let spectral_enable = self.params.spectral_enable.value();
            let spectral_place = self.params.spectral_placement.value();
            let spectral_amount = self.params.spectral_amount.value();
            let spectral_tilt = self.params.spectral_tilt.value();
            let spectral_formant = self.params.spectral_formant.value();
            if spectral_enable && spectral_place == SpectralPlacement::PreFx {
                let (out_left, out_right) = output.split_at_mut(1);
                self.spectral_main.process_block(
                    &mut out_left[0][block_start..block_end],
                    &mut out_right[0][block_start..block_end],
                    spectral_amount,
                    spectral_tilt,
                    spectral_formant,
                );
                self.spectral_fx.process_block(
                    &mut fx_left[..block_len],
                    &mut fx_right[..block_len],
                    spectral_amount,
                    spectral_tilt,
                    spectral_formant,
                );
            }

            if self.params.chorus_enable.value() {
                self.chorus.set_enabled(true);
                self.chorus.set_sample_rate(sample_rate);
                let chorus_rate = self.params.chorus_rate.value();
                let chorus_depth = self.params.chorus_depth.value();
                let chorus_mix = self.params.chorus_mix.value();

                for sample_idx in block_start..block_end {
                    let value_idx = sample_idx - block_start;
                    let (left, right) = self.chorus.process(
                        fx_left[value_idx],
                        fx_right[value_idx],
                        chorus_rate,
                        chorus_depth,
                        chorus_mix,
                    );
                    fx_left[value_idx] = left;
                    fx_right[value_idx] = right;
                }
            } else {
                self.chorus.set_enabled(false);
            }

            if self.params.multi_filter_enable.value() {
                let routing = self.params.multi_filter_routing.value();
                let a_type = self.params.multi_filter_a_type.value();
                let b_type = self.params.multi_filter_b_type.value();
                let c_type = self.params.multi_filter_c_type.value();
                let a_cut = self.params.multi_filter_a_cut.value();
                let b_cut = self.params.multi_filter_b_cut.value();
                let c_cut = self.params.multi_filter_c_cut.value();
                let a_res = self.params.multi_filter_a_res.value();
                let b_res = self.params.multi_filter_b_res.value();
                let c_res = self.params.multi_filter_c_res.value();
                let a_amt = self.params.multi_filter_a_amt.value();
                let b_amt = self.params.multi_filter_b_amt.value();
                let c_amt = self.params.multi_filter_c_amt.value();
                let morph = self.params.multi_filter_morph.value();
                let parallel_ab = self.params.multi_filter_parallel_ab.value();
                let parallel_c = self.params.multi_filter_parallel_c.value();

                for sample_idx in block_start..block_end {
                    let value_idx = sample_idx - block_start;
                    let (left, right) = self.multi_filter.process(
                        fx_left[value_idx],
                        fx_right[value_idx],
                        routing,
                        a_type,
                        a_cut,
                        a_res,
                        a_amt,
                        b_type,
                        b_cut,
                        b_res,
                        b_amt,
                        c_type,
                        c_cut,
                        c_res,
                        c_amt,
                        morph,
                        parallel_ab,
                        parallel_c,
                    );
                    fx_left[value_idx] = left;
                    fx_right[value_idx] = right;
                }
            }

            if spectral_enable && spectral_place == SpectralPlacement::PreDist {
                self.spectral_fx.process_block(
                    &mut fx_left[..block_len],
                    &mut fx_right[..block_len],
                    spectral_amount,
                    spectral_tilt,
                    spectral_formant,
                );
            }

            if self.params.dist_enable.value() {
                let dist_drive_base = self.params.dist_drive.value();
                let dist_tone = self.params.dist_tone.value();
                let dist_magic = self.params.dist_magic.value();
                let dist_mix = self.params.dist_mix.value();
                self.distortion.set_tone(dist_tone);
                self.distortion.set_sizzle_guard(
                    self.params.sizzle_dist_enable.value(),
                    self.params.sizzle_cutoff.value(),
                );
                for sample_idx in block_start..block_end {
                    let value_idx = sample_idx - block_start;
                    let dist_drive =
                        (dist_drive_base + seq_dist_values[value_idx] + dist_env_values[value_idx])
                            .clamp(0.0, 1.0);
                    let left = self.distortion.process_sample(
                        0,
                        fx_left[value_idx],
                        dist_drive,
                        dist_magic,
                        dist_mix,
                    );
                    let right = self.distortion.process_sample(
                        1,
                        fx_right[value_idx],
                        dist_drive,
                        dist_magic,
                        dist_mix,
                    );
                    fx_left[value_idx] = left;
                    fx_right[value_idx] = right;
                }
            }

            if self.params.eq_enable.value() {
                let eq_low_gain = self.params.eq_low_gain.value();
                let eq_mid_gain = self.params.eq_mid_gain.value();
                let eq_mid_freq = self.params.eq_mid_freq.value();
                let eq_mid_q = self.params.eq_mid_q.value();
                let eq_high_gain = self.params.eq_high_gain.value();
                let eq_mix = self.params.eq_mix.value();
                self.eq
                    .set_params(eq_low_gain, eq_mid_gain, eq_mid_freq, eq_mid_q, eq_high_gain);
                for sample_idx in block_start..block_end {
                    let value_idx = sample_idx - block_start;
                    let left = self.eq.process_sample(0, fx_left[value_idx]);
                    let right = self.eq.process_sample(1, fx_right[value_idx]);
                    fx_left[value_idx] = fx_left[value_idx] * (1.0 - eq_mix) + left * eq_mix;
                    fx_right[value_idx] = fx_right[value_idx] * (1.0 - eq_mix) + right * eq_mix;
                }
            }

            if self.params.delay_enable.value() {
                let delay_time = self.params.delay_time_ms.value();
                let delay_feedback = self.params.delay_feedback.value();
                let delay_mix = self.params.delay_mix.value();
                for sample_idx in block_start..block_end {
                    let value_idx = sample_idx - block_start;
                    let (left, right) = self.delay.process(
                        fx_left[value_idx],
                        fx_right[value_idx],
                        delay_time,
                        delay_feedback,
                        delay_mix,
                    );
                    fx_left[value_idx] = left;
                    fx_right[value_idx] = right;
                }
            }

            if self.params.reverb_enable.value() {
                let reverb_size = self.params.reverb_size.value();
                let reverb_damp = self.params.reverb_damp.value();
                let reverb_diffusion = self.params.reverb_diffusion.value();
                let reverb_shimmer = self.params.reverb_shimmer.value();
                let reverb_mix = self.params.reverb_mix.value();
                for sample_idx in block_start..block_end {
                    let value_idx = sample_idx - block_start;
                    let (left, right) = self.reverb.process(
                        fx_left[value_idx],
                        fx_right[value_idx],
                        reverb_size,
                        reverb_damp,
                        reverb_diffusion,
                        reverb_shimmer,
                        reverb_mix,
                    );
                    fx_left[value_idx] = left;
                    fx_right[value_idx] = right;
                }
            }

            let fx_bus_mix = self.params.fx_bus_mix.value().clamp(0.0, 1.0);
            for sample_idx in block_start..block_end {
                let value_idx = sample_idx - block_start;
                output[0][sample_idx] = output[0][sample_idx] * (1.0 - fx_bus_mix)
                    + fx_left[value_idx] * fx_bus_mix;
                output[1][sample_idx] = output[1][sample_idx] * (1.0 - fx_bus_mix)
                    + fx_right[value_idx] * fx_bus_mix;
            }

            if spectral_enable && spectral_place == SpectralPlacement::PostFx {
                let (out_left, out_right) = output.split_at_mut(1);
                self.spectral_main.process_block(
                    &mut out_left[0][block_start..block_end],
                    &mut out_right[0][block_start..block_end],
                    spectral_amount,
                    spectral_tilt,
                    spectral_formant,
                );
            }

            if self.params.ring_mod_enable.value()
                && self.params.ring_mod_placement.value() == RingModPlacement::PostFx
            {
                let ring_freq = self.params.ring_mod_freq.value();
                let ring_mix = self.params.ring_mod_mix.value().clamp(0.0, 1.0);
                let ring_source = self.params.ring_mod_source.value();
                for sample_idx in block_start..block_end {
                    let phase_left = self.ring_mod_post_phase[0];
                    let phase_right = self.ring_mod_post_phase[1];
                    let carrier_left = match ring_source {
                        RingModSource::Sine => {
                            (2.0 * std::f32::consts::PI * phase_left).sin()
                        }
                        RingModSource::Classic | RingModSource::Wavetable => output[0][sample_idx],
                    };
                    let carrier_right = match ring_source {
                        RingModSource::Sine => {
                            (2.0 * std::f32::consts::PI * phase_right).sin()
                        }
                        RingModSource::Classic | RingModSource::Wavetable => output[1][sample_idx],
                    };
                    let ring_left = output[0][sample_idx] * carrier_left;
                    let ring_right = output[1][sample_idx] * carrier_right;
                    output[0][sample_idx] = output[0][sample_idx] * (1.0 - ring_mix)
                        + ring_left * ring_mix;
                    output[1][sample_idx] = output[1][sample_idx] * (1.0 - ring_mix)
                        + ring_right * ring_mix;
                    self.ring_mod_post_phase[0] =
                        (phase_left + ring_freq / sample_rate).fract();
                    self.ring_mod_post_phase[1] =
                        (phase_right + ring_freq / sample_rate).fract();
                }
            }

            if self.params.output_sat_enable.value() {
                let drive = self.params.output_sat_drive.value();
                let mix = self.params.output_sat_mix.value();
                let mode = self.params.output_sat_type.value();
                for sample_idx in block_start..block_end {
                    let left = self
                        .output_saturation
                        .process_sample(0, output[0][sample_idx], drive, mode, mix);
                    let right = self
                        .output_saturation
                        .process_sample(1, output[1][sample_idx], drive, mode, mix);
                    output[0][sample_idx] = left;
                    output[1][sample_idx] = right;
                }
            }

            self.limiter_left.set_enabled(self.params.limiter_enable.value());
            self.limiter_right.set_enabled(self.params.limiter_enable.value());
            self.limiter_left
                .set_threshold(self.params.limiter_threshold.value());
            self.limiter_right
                .set_threshold(self.params.limiter_threshold.value());
            self.limiter_left
                .set_release(self.params.limiter_release.value());
            self.limiter_right
                .set_release(self.params.limiter_release.value());

            if self.params.limiter_enable.value() {
                for sample_idx in block_start..block_end {
                    output[0][sample_idx] =
                        self.limiter_left.process(output[0][sample_idx]);
                    output[1][sample_idx] =
                        self.limiter_right.process(output[1][sample_idx]);
                }
            }

            for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
                output[0][sample_idx] *= gain[value_idx];
                output[1][sample_idx] *= gain[value_idx];
            }

            // Terminate voices whose release period has fully ended. This could be done as part of
            // the previous loop but this is simpler.
            for voice in &mut self.voices {
                if let Some(v) = voice {
                    if v.releasing && v.amp_envelope.get_state() == ADSREnvelopeState::Idle {
                        context.send_event(NoteEvent::VoiceTerminated {
                            timing: block_end as u32,
                            voice_id: Some(v.voice_id),
                            channel: v.channel,
                            note: v.note,
                        });
                        *voice = None;
                    }
                }
            }
            self.last_note_active = self.voices.iter().any(|v| v.is_some());

            // And then just keep processing blocks until we've run out of buffer to fill
            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl SubSynth {
    fn apply_factory_preset(&mut self, index: usize, update_param: bool) {
        if self.factory_presets.is_empty() {
            return;
        }

        let clamped = index.min(self.factory_presets.len() - 1);
        self.factory_presets[clamped].apply_direct(&self.params);
        if update_param {
            self.params.preset_index.set_plain_value(clamped as i32);
        }
        self.last_preset_index = clamped as i32;
    }

    fn sync_preset_if_changed(&mut self) {
        let preset_index = self.params.preset_index.value();
        if preset_index != self.last_preset_index {
            let clamped = if preset_index < 0 { 0 } else { preset_index as usize };
            self.apply_factory_preset(clamped, false);
        }
    }

    fn get_voice_idx(&mut self, voice_id: i32) -> Option<usize> {
        self.voices
            .iter_mut()
            .position(|voice| matches!(voice, Some(voice) if voice.voice_id == voice_id))
    }

    fn refresh_custom_wavetable(&mut self) {
        if let Ok(mut data) = self.params.custom_wavetable_data.try_write() {
            if let Some(table) = data.take() {
                self.custom_wavetable = Some(WavetableBank::from_table(table));
                if let Ok(path) = self.params.custom_wavetable_path.read() {
                    self.custom_wavetable_path = (*path).clone();
                }
            }
        }

        if self.custom_wavetable.is_none() {
            if let Ok(path) = self.params.custom_wavetable_path.read() {
                if let Some(path) = (*path).as_ref() {
                    if self.custom_wavetable_path.as_deref() != Some(path.as_str()) {
                        if let Ok(table) = waveform::load_wavetable_from_file(std::path::Path::new(path)) {
                            self.custom_wavetable = Some(WavetableBank::from_table(table));
                            self.custom_wavetable_path = Some(path.clone());
                        }
                    }
                }
            }
        }
    }

    fn construct_envelopes(
        &self,
        sample_rate: f32,
        velocity: f32,
    ) -> (ADSREnvelope, ADSREnvelope, ADSREnvelope) {
        (
            ADSREnvelope::new(
                self.params.amp_attack_ms.value(),
                self.params.amp_hold_ms.value(),
                self.params.amp_decay_ms.value(),
                self.params.amp_decay2_ms.value(),
                self.params.amp_decay2_level.value(),
                self.params.amp_sustain_level.value(),
                self.params.amp_release_ms.value(),
                sample_rate,
                velocity,
                self.params.amp_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_cut_attack_ms.value(),
                self.params.filter_cut_hold_ms.value(),
                self.params.filter_cut_decay_ms.value(),
                self.params.filter_cut_decay2_ms.value(),
                self.params.filter_cut_decay2_level.value(),
                self.params.filter_cut_sustain_ms.value(),
                self.params.filter_cut_release_ms.value(),
                sample_rate,
                velocity,
                self.params.filter_cut_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_res_attack_ms.value(),
                self.params.filter_res_hold_ms.value(),
                self.params.filter_res_decay_ms.value(),
                self.params.filter_res_decay2_ms.value(),
                self.params.filter_res_decay2_level.value(),
                self.params.filter_res_sustain_ms.value(),
                self.params.filter_res_release_ms.value(),
                sample_rate,
                velocity,
                self.params.filter_res_tension.value(),
            ),
        )
    }

    fn start_voice(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
        pan: f32,
        pressure: f32,
        brightness: f32,
        expression: f32,
        vibrato: f32,
        tuning: f32,
        vib_mod: Modulator,
        trem_mod: Modulator,
        mod_lfo1: Modulator,
        mod_lfo2: Modulator,
        amp_envelope: ADSREnvelope,
        filter_cut_envelope: ADSREnvelope,
        filter_res_envelope: ADSREnvelope,
        fm_envelope: ADSREnvelope,
        dist_envelope: ADSREnvelope,
        filter: FilterType,
        sample_rate: f32,
    ) -> &mut Voice {
        // Use the passed envelopes instead of creating new ones
        let new_voice = Voice {
            voice_id: voice_id.unwrap_or_else(|| compute_fallback_voice_id(note, channel)),
            internal_voice_id: self.next_internal_voice_id,
            channel,
            note,
            velocity,
            velocity_sqrt: velocity.sqrt(),
            pan,
            pressure,
            brightness,
            expression,
            vibrato,
            tuning,
            phase: 0.0,
            phase_delta: 0.0,
            target_phase_delta: 0.0,
            releasing: false,
            amp_envelope,
            voice_gain: None,
            filter_cut_envelope,
            filter_res_envelope,
            fm_envelope,
            dist_envelope,
            filter: Some(filter),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, sample_rate),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, sample_rate),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, sample_rate),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, sample_rate),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, sample_rate),
            comb_filter: filter::CombFilter::new(sample_rate),
            rainbow_comb_filter: filter::RainbowCombFilter::new(sample_rate),
            diode_ladder_lp_filter: filter::DiodeLadderFilter::new_lowpass(sample_rate),
            diode_ladder_hp_filter: filter::DiodeLadderFilter::new_highpass(sample_rate),
            ms20_filter: filter::Ms20Filter::new(sample_rate),
            formant_morph_filter: filter::FormantMorphFilter::new(sample_rate),
            phaser_filter: filter::PhaserFilter::new(sample_rate),
            comb_allpass_filter: filter::CombAllpassFilter::new(sample_rate),
            bitcrush_lp_filter: filter::BitcrushLpFilter::new(sample_rate),
            vib_mod,
            trem_mod,
            mod_lfo1,
            mod_lfo2,
            pan_mod: Modulator::new(
                self.params.pan_lfo_rate.value(),
                self.params.pan_lfo_intensity.value(),
                self.params.pan_lfo_attack.value(),
                self.params.pan_lfo_shape.value(),
            ),
            drift_offset: 0.0,
            mod_smooth: [0.0; 6],
            fm_feedback_state: 0.0,
            unison_phases: [0.0; 6],
            stereo_prev: 0.0,
            dc_blocker: filter::DCBlocker::new(),
            sizzle_osc_lp: Biquad::new(),
            sizzle_wt_lp: Biquad::new(),
            alias_lp1: Biquad::new(),
            alias_lp2: Biquad::new(),
            tight_lp: Biquad::new(),
            tight_hp: Biquad::new(),
            ring_phase: 0.0,
        };

        self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);

        if let Some(free_voice_idx) = self.voices.iter().position(|voice| voice.is_none()) {
            let voice = &mut self.voices[free_voice_idx];
            if voice.is_none() {
                *voice = Some(new_voice);
                let voice = voice.as_mut().unwrap();
                voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.fm_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.dist_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.vib_mod.trigger();
                voice.trem_mod.trigger();
                voice.mod_lfo1.trigger();
                voice.mod_lfo2.trigger();
            }
            voice.as_mut().unwrap()
        } else {
            let oldest_voice = self
                .voices
                .iter_mut()
                .min_by_key(|voice| voice.as_ref().unwrap().internal_voice_id)
                .unwrap();
            let oldest_voice = oldest_voice.as_mut().unwrap();
    
            if oldest_voice.amp_envelope.get_state() == ADSREnvelopeState::Idle ||
                oldest_voice.amp_envelope.get_state() == ADSREnvelopeState::Release
            {
                // If the oldest voice's amp envelope is already idle or releasing, no need to send a voice terminated event
                *oldest_voice = new_voice;
                oldest_voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                oldest_voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                oldest_voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                oldest_voice.releasing = false; // Reset the releasing flag
                oldest_voice.vib_mod.trigger();
                oldest_voice.trem_mod.trigger();
                oldest_voice.mod_lfo1.trigger();
                oldest_voice.mod_lfo2.trigger();
            } else {
                context.send_event(NoteEvent::VoiceTerminated {
                    timing: sample_offset,
                    voice_id: Some(oldest_voice.voice_id),
                    channel: oldest_voice.channel,
                    note: oldest_voice.note,
                });
    
                *oldest_voice = new_voice;
            }
    
            oldest_voice
        }
    }

    fn start_release_for_voices(
        &mut self,
        _sample_rate: f32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        for voice in &mut self.voices {
            if let Some(voice) = voice {
                if voice_id == Some(voice.voice_id) || (channel == voice.channel && note == voice.note) {
                    voice.releasing = true;
                    voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.fm_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.dist_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                }
            }
        }
    }

    fn _find_voice(&mut self, voice_id: Option<i32>, channel: u8, note: u8) -> Option<&mut Voice> {
        self.voices
            .iter_mut()
            .find(|voice| {
                let voice_id = voice_id.clone(); // Clone the voice_id for comparison inside the closure
                if let Some(voice) = voice {
                    voice.voice_id == voice_id.unwrap_or(voice.voice_id)
                        && voice.channel == channel
                        && voice.note == note
                } else {
                    false
                }
            })
            .map(|voice| voice.as_mut().unwrap())
    }

    fn compute_fallback_voice_id(note: u8, channel: u8, next_voice_id: i32) -> i32 {
        // Fallback voice ID computation...
        // Modify this function to generate a unique voice ID based on note, channel, and next_voice_id.
        // Example implementation:
        (note as i32) + (channel as i32) + next_voice_id
    }

    fn find_or_create_voice(
        &mut self,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        pan: f32,
        pressure:f32,
        brightness: f32,
        expression: f32,
        tuning: f32,
        vibrato: f32,
        amp_envelope: ADSREnvelope,
        filter_cut_envelope: ADSREnvelope,
        filter_res_envelope: ADSREnvelope,
        vib_mod: Modulator,
        trem_mod: Modulator,
    ) -> &mut Voice {
        // Search for an existing voice with the given voice_id
        if let Some(existing_index) = self.voices.iter().position(|voice| {
            voice
                .as_ref()
                .map(|voice_ref| {
                    voice_ref.voice_id == voice_id.unwrap_or(voice_ref.voice_id)
                        && voice_ref.channel == channel
                        && voice_ref.note == note
                })
                .unwrap_or(false)
        }) {
            return self.voices[existing_index].as_mut().unwrap();
        }

        // If no existing voice found, create a new voice
        let new_voice_id = voice_id.unwrap_or_else(|| {
            // Generate a fallback voice ID
            self.next_voice_index += 1;
            Self::compute_fallback_voice_id(
                note,
                channel,
                self.next_voice_index.try_into().unwrap(),
            )
        });

        let mut new_voice = Voice {
            voice_id: new_voice_id,
            channel,
            note,
            internal_voice_id: self.next_internal_voice_id,
            velocity: 0.0,
            velocity_sqrt: 0.0,
            phase: 0.0,
            phase_delta: 0.0,
            target_phase_delta: 0.0,
            releasing: false,
            amp_envelope,
            voice_gain: None,
            filter_cut_envelope,
            filter_res_envelope,
            fm_envelope: ADSREnvelope::new(
                self.params.fm_env_attack_ms.value(),
                self.params.fm_env_hold_ms.value(),
                self.params.fm_env_decay_ms.value(),
                self.params.fm_env_decay2_ms.value(),
                self.params.fm_env_decay2_level.value(),
                self.params.fm_env_sustain_level.value(),
                self.params.fm_env_release_ms.value(),
                44100.0,
                1.0,
                0.0,
            ),
            dist_envelope: ADSREnvelope::new(
                self.params.dist_env_attack_ms.value(),
                self.params.dist_env_hold_ms.value(),
                self.params.dist_env_decay_ms.value(),
                self.params.dist_env_decay2_ms.value(),
                self.params.dist_env_decay2_level.value(),
                self.params.dist_env_sustain_level.value(),
                self.params.dist_env_release_ms.value(),
                44100.0,
                1.0,
                0.0,
            ),
            filter: Some(self.params.filter_type.value()),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, 192000.0),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, 192000.0),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, 192000.0),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, 192000.0),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, 192000.0),
            comb_filter: filter::CombFilter::new(192000.0),
            rainbow_comb_filter: filter::RainbowCombFilter::new(192000.0),
            diode_ladder_lp_filter: filter::DiodeLadderFilter::new_lowpass(192000.0),
            diode_ladder_hp_filter: filter::DiodeLadderFilter::new_highpass(192000.0),
            ms20_filter: filter::Ms20Filter::new(192000.0),
            formant_morph_filter: filter::FormantMorphFilter::new(192000.0),
            phaser_filter: filter::PhaserFilter::new(192000.0),
            comb_allpass_filter: filter::CombAllpassFilter::new(192000.0),
            bitcrush_lp_filter: filter::BitcrushLpFilter::new(192000.0),
            pan,
            pressure,
            brightness,
            expression,
            tuning,
            vibrato,
            vib_mod,
            trem_mod,
            mod_lfo1: Modulator::new(
                self.params.lfo1_rate.value(),
                1.0,
                self.params.lfo1_attack.value(),
                self.params.lfo1_shape.value(),
            ),
            mod_lfo2: Modulator::new(
                self.params.lfo2_rate.value(),
                1.0,
                self.params.lfo2_attack.value(),
                self.params.lfo2_shape.value(),
            ),
            pan_mod: Modulator::new(
                self.params.pan_lfo_rate.value(),
                self.params.pan_lfo_intensity.value(),
                self.params.pan_lfo_attack.value(),
                self.params.pan_lfo_shape.value(),
            ),
            drift_offset: 0.0,
            mod_smooth: [0.0; 6],
            fm_feedback_state: 0.0,
            unison_phases: [0.0; 6],
            stereo_prev: 0.0,
            dc_blocker: filter::DCBlocker::new(),
            sizzle_osc_lp: Biquad::new(),
            sizzle_wt_lp: Biquad::new(),
            alias_lp1: Biquad::new(),
            alias_lp2: Biquad::new(),
            tight_lp: Biquad::new(),
            tight_hp: Biquad::new(),
            ring_phase: 0.0,
        };
        new_voice.amp_envelope.trigger();
        new_voice.filter_cut_envelope.trigger();
        new_voice.filter_res_envelope.trigger();
        new_voice.fm_envelope.trigger();
        new_voice.dist_envelope.trigger();
        new_voice.vib_mod.trigger();
        new_voice.trem_mod.trigger();
        // Find the next available slot for a new voice
        let mut next_voice_index = self.next_voice_index;
        while self.voices[next_voice_index].is_some() {
            next_voice_index = (next_voice_index + 1) % NUM_VOICES;
            if next_voice_index == self.next_voice_index {
                panic!("No available slots for new voices");
            }
        }

        // Store the new voice in the found slot
        self.voices[next_voice_index] = Some(new_voice);

        // Update the next available slot index
        self.next_voice_index = next_voice_index;

        // Return a mutable reference to the newly created voice
        self.voices[next_voice_index].as_mut().unwrap()

    }

    fn handle_poly_event(
        &mut self,
        _timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        gain: f32,
        pan: f32,
        brightness: f32,
        expression: f32,
        tuning: f32,
        pressure: f32,
        vibrato: f32,
        amp_envelope: Option<&ADSREnvelope>,
        filter_cut_envelope: Option<&ADSREnvelope>,
        filter_res_envelope: Option<&ADSREnvelope>,
        vibrato_modulator: Option<&Modulator>,
        tremolo_modulator: Option<&Modulator>,
    ) {
        let voice = self.find_or_create_voice(
            voice_id,
            channel,
            note,
            pan,
            pressure,
            brightness,
            expression,
            tuning,
            vibrato,
            amp_envelope.cloned().unwrap(),
            filter_cut_envelope.cloned().unwrap(),
            filter_res_envelope.cloned().unwrap(),
            vibrato_modulator.cloned().unwrap(),
            tremolo_modulator.cloned().unwrap(),
        );
        voice.velocity = gain;
        voice.velocity_sqrt = gain.sqrt();
        if let Some(amp_envelope) = amp_envelope {
            voice.amp_envelope = amp_envelope.clone();
            voice.amp_envelope.set_velocity(gain);
        }
    }
    
    

    fn choke_voices(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        for voice in self.voices.iter_mut() {
            match voice {
                Some(Voice {
                    voice_id: candidate_voice_id,
                    channel: candidate_channel,
                    note: candidate_note,
                    ..
                }) if voice_id == Some(*candidate_voice_id)
                    || (channel == *candidate_channel && note == *candidate_note) =>
                {
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: sample_offset,
                        voice_id: Some(*candidate_voice_id),
                        channel,
                        note,
                    });
                    *voice = None;

                    if voice_id.is_some() {
                        return;
                    }
                }
                _ => (),
            }
        }
    }
    pub fn clip(input: f32, limit: f32) -> f32 {
        if input > limit {
            limit
        } else if input < -limit {
            -limit
        } else {
            input
        }
    }
    pub fn poly_blep(t: f32, dt: f32) -> f32 {
        if t < dt {
            let t = t / dt;
            // 2 * (t - t^2/2 - 0.5)
            return t + t - t * t - 1.0;
        } else if t > 1.0 - dt {
            let t = (t - 1.0) / dt;
            // 2 * (t^2/2 + t + 0.5)
            return t * t + t + t + 1.0;
        }
        0.0
    }

    pub fn wavefold(sample: f32, amount: f32) -> f32 {
        if amount <= 0.0 {
            return sample;
        }

        let drive = 1.0 + amount * 6.0;
        let post_gain = 1.0 / (1.0 + amount * 1.5);
        let x = sample * drive;
        let mut folded = (x + 1.0).rem_euclid(4.0);
        if folded > 2.0 {
            folded = 4.0 - folded;
        }
        (folded - 1.0) * post_gain
    }
}

const fn compute_fallback_voice_id(note: u8, channel: u8) -> i32 {
    note as i32 | ((channel as i32) << 16)
}

impl ClapPlugin for SubSynth {
    const CLAP_ID: &'static str = "art.dogsynth";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A knitty gritty wavetable bass synth for dubstep and heavy electronic sound design");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];

    const CLAP_POLY_MODULATION_CONFIG: Option<PolyModulationConfig> = Some(PolyModulationConfig {
        max_voice_capacity: NUM_VOICES as u32,
        supports_overlapping_voices: true,
    });
}

impl Vst3Plugin for SubSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"DogSynthLing1A01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(SubSynth);
nih_export_vst3!(SubSynth);
