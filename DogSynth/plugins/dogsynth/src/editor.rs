use nih_plug::prelude::{Editor, Param};
use nih_plug_vizia::vizia::cache::BoundingBox;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::widgets::util::ModifiersExt;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{fs, path::PathBuf};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OPENFILENAMEW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
#[cfg(target_os = "windows")]
const CF_UNICODETEXT: u32 = 13;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

use crate::{
    util, waveform::load_wavetable_from_file, FilterType, ModSource, ModTarget, OscRouting,
    RingModPlacement, RingModSource, SpectralPlacement, SubSynthParams, UnisonVoices, Waveform,
};

// zCool font constant
const ZCOOL_XIAOWEI: &str = "ZCOOL XiaoWei";
const ZCOOL_FONT_DATA: &[u8] = include_bytes!("assets/ZCOOL_XIAOWEI_REGULAR.ttf");
const SEQ_LANE_COUNT: usize = 6;
const SEQ_STEP_COUNT: usize = 32;
const SEQ_PRESET_COUNT: usize = 5;

pub(crate) const FACTORY_PRESET_COUNT: usize = 128;
const FACTORY_VARIANT_COUNT: usize = 8;

const PRESET_VARIANTS: [&str; FACTORY_VARIANT_COUNT] = [
    "Clean",
    "Bright",
    "Warm",
    "Wide",
    "Tight",
    "Aggro",
    "Airy",
    "Dark",
];

const GM_PRESET_NAMES: [&str; 128] = [
    "Acoustic Grand Piano",
    "Bright Acoustic Piano",
    "Electric Grand Piano",
    "Honky-tonk Piano",
    "Electric Piano 1",
    "Electric Piano 2",
    "Harpsichord",
    "Clavinet",
    "Celesta",
    "Glockenspiel",
    "Music Box",
    "Vibraphone",
    "Marimba",
    "Xylophone",
    "Tubular Bells",
    "Dulcimer",
    "Drawbar Organ",
    "Percussive Organ",
    "Rock Organ",
    "Church Organ",
    "Reed Organ",
    "Accordion",
    "Harmonica",
    "Tango Accordion",
    "Acoustic Guitar (nylon)",
    "Acoustic Guitar (steel)",
    "Electric Guitar (jazz)",
    "Electric Guitar (clean)",
    "Electric Guitar (muted)",
    "Overdriven Guitar",
    "Distortion Guitar",
    "Guitar Harmonics",
    "Acoustic Bass",
    "Electric Bass (finger)",
    "Electric Bass (pick)",
    "Fretless Bass",
    "Slap Bass 1",
    "Slap Bass 2",
    "Synth Bass 1",
    "Synth Bass 2",
    "Violin",
    "Viola",
    "Cello",
    "Contrabass",
    "Tremolo Strings",
    "Pizzicato Strings",
    "Orchestral Harp",
    "Timpani",
    "String Ensemble 1",
    "String Ensemble 2",
    "SynthStrings 1",
    "SynthStrings 2",
    "Choir Aahs",
    "Voice Oohs",
    "Synth Voice",
    "Orchestra Hit",
    "Trumpet",
    "Trombone",
    "Tuba",
    "Muted Trumpet",
    "French Horn",
    "Brass Section",
    "SynthBrass 1",
    "SynthBrass 2",
    "Soprano Sax",
    "Alto Sax",
    "Tenor Sax",
    "Baritone Sax",
    "Oboe",
    "English Horn",
    "Bassoon",
    "Clarinet",
    "Piccolo",
    "Flute",
    "Recorder",
    "Pan Flute",
    "Blown Bottle",
    "Shakuhachi",
    "Whistle",
    "Ocarina",
    "Lead 1 (square)",
    "Lead 2 (sawtooth)",
    "Lead 3 (calliope)",
    "Lead 4 (chiff)",
    "Lead 5 (charang)",
    "Lead 6 (voice)",
    "Lead 7 (fifths)",
    "Lead 8 (bass + lead)",
    "Pad 1 (new age)",
    "Pad 2 (warm)",
    "Pad 3 (polysynth)",
    "Pad 4 (choir)",
    "Pad 5 (bowed)",
    "Pad 6 (metallic)",
    "Pad 7 (halo)",
    "Pad 8 (sweep)",
    "FX 1 (rain)",
    "FX 2 (soundtrack)",
    "FX 3 (crystal)",
    "FX 4 (atmosphere)",
    "FX 5 (brightness)",
    "FX 6 (goblins)",
    "FX 7 (echoes)",
    "FX 8 (sci-fi)",
    "Sitar",
    "Banjo",
    "Shamisen",
    "Koto",
    "Kalimba",
    "Bag pipe",
    "Fiddle",
    "Shanai",
    "Tinkle Bell",
    "Agogo",
    "Steel Drums",
    "Woodblock",
    "Taiko Drum",
    "Melodic Tom",
    "Synth Drum",
    "Reverse Cymbal",
    "Guitar Fret Noise",
    "Breath Noise",
    "Seashore",
    "Bird Tweet",
    "Telephone Ring",
    "Helicopter",
    "Applause",
    "Gunshot",
];

#[derive(Clone, Copy, Debug)]
enum GmGroup {
    Piano,
    Chromatic,
    Organ,
    Guitar,
    Bass,
    Strings,
    Ensemble,
    Brass,
    Reed,
    Pipe,
    SynthLead,
    SynthPad,
    SynthFx,
    Ethnic,
    Percussive,
    Sfx,
}

#[derive(Clone, Copy)]
enum BasePreset {
    Init,
    LaserLead,
    EdgeLead,
    SilkLead,
    SubBass,
    RubberBass,
    GrindBass,
    ThickBass,
    SuperSaw,
    LiftSaw,
    HaloSaw,
    WideSaw,
    MetalReese,
    DarkReese,
    WideReese,
    DirtyReese,
}

const BASE_PRESETS: [BasePreset; 16] = [
    BasePreset::Init,
    BasePreset::LaserLead,
    BasePreset::EdgeLead,
    BasePreset::SilkLead,
    BasePreset::SubBass,
    BasePreset::RubberBass,
    BasePreset::GrindBass,
    BasePreset::ThickBass,
    BasePreset::SuperSaw,
    BasePreset::LiftSaw,
    BasePreset::HaloSaw,
    BasePreset::WideSaw,
    BasePreset::MetalReese,
    BasePreset::DarkReese,
    BasePreset::WideReese,
    BasePreset::DirtyReese,
];

fn base_preset_label(base: BasePreset) -> &'static str {
    match base {
        BasePreset::Init => "Init",
        BasePreset::LaserLead => "Laser Lead",
        BasePreset::EdgeLead => "Edge Lead",
        BasePreset::SilkLead => "Silk Lead",
        BasePreset::SubBass => "Sub Bass",
        BasePreset::RubberBass => "Rubber Bass",
        BasePreset::GrindBass => "Grind Bass",
        BasePreset::ThickBass => "Thick Bass",
        BasePreset::SuperSaw => "Super Saw",
        BasePreset::LiftSaw => "Lift Saw",
        BasePreset::HaloSaw => "Halo Saw",
        BasePreset::WideSaw => "Wide Saw",
        BasePreset::MetalReese => "Metal Reese",
        BasePreset::DarkReese => "Dark Reese",
        BasePreset::WideReese => "Wide Reese",
        BasePreset::DirtyReese => "Dirty Reese",
    }
}

fn seq_preset_name(index: usize) -> &'static str {
    match index % SEQ_PRESET_COUNT {
        0 => "Init",
        1 => "Four On",
        2 => "Offbeat",
        3 => "Half Time",
        _ => "Stutter 16",
    }
}

fn seq_value_to_normalized(value: f32) -> f32 {
    (value.clamp(-1.0, 1.0) + 1.0) * 0.5
}

fn apply_seq_pattern(cx: &mut EventContext, params: &SubSynthParams, pattern: [[f32; 32]; SEQ_LANE_COUNT]) {
    for lane in 0..SEQ_LANE_COUNT {
        for step in 0..SEQ_STEP_COUNT {
            let normalized = seq_value_to_normalized(pattern[lane][step]);
            apply_param(cx, &params.seq_lanes[lane].steps[step].value, normalized);
        }
    }
}

fn seq_pattern_for(index: usize) -> [[f32; 32]; SEQ_LANE_COUNT] {
    let mut pattern = [[0.0; 32]; SEQ_LANE_COUNT];
    let gate = &mut pattern[0];
    match index % SEQ_PRESET_COUNT {
        0 => {
            for step in 0..SEQ_STEP_COUNT {
                gate[step] = -1.0;
            }
        }
        1 => {
            for step in 0..SEQ_STEP_COUNT {
                gate[step] = if step % 8 == 0 { 1.0 } else { -1.0 };
            }
        }
        2 => {
            for step in 0..SEQ_STEP_COUNT {
                gate[step] = if step % 8 == 4 { 1.0 } else { -1.0 };
            }
        }
        3 => {
            for step in 0..SEQ_STEP_COUNT {
                gate[step] = if step == 0 || step == 12 || step == 16 || step == 28 { 1.0 } else { -1.0 };
            }
        }
        _ => {
            for step in 0..SEQ_STEP_COUNT {
                gate[step] = if step % 2 == 0 { 1.0 } else { -1.0 };
            }
        }
    }

    if index % SEQ_PRESET_COUNT != 0 {
        for step in 0..SEQ_STEP_COUNT {
            pattern[1][step] = if step % 4 == 0 { 0.6 } else { -0.2 };
            pattern[3][step] = if step % 8 == 4 { 0.4 } else { 0.0 };
        }
    }

    pattern
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PresetData {
    gain: f32,
    amp_attack_ms: f32,
    #[serde(default)]
    amp_hold_ms: f32,
    amp_release_ms: f32,
    #[serde(default)]
    amp_tension: f32,
    waveform: f32,
    osc_routing: f32,
    osc_blend: f32,
    wavetable_position: f32,
    #[serde(default)]
    wavetable_distortion: f32,
    #[serde(default)]
    classic_drive: f32,
    custom_wavetable_enable: f32,
    analog_enable: f32,
    analog_drive: f32,
    analog_noise: f32,
    analog_drift: f32,
    sub_level: f32,
    #[serde(default)]
    classic_level: f32,
    #[serde(default)]
    wavetable_level: f32,
    #[serde(default)]
    noise_level: f32,
    #[serde(default)]
    classic_send: f32,
    #[serde(default)]
    wavetable_send: f32,
    #[serde(default)]
    sub_send: f32,
    #[serde(default)]
    noise_send: f32,
    #[serde(default)]
    ring_mod_send: f32,
    #[serde(default)]
    fx_bus_mix: f32,
    #[serde(default)]
    ring_mod_enable: f32,
    #[serde(default)]
    ring_mod_source: f32,
    #[serde(default)]
    ring_mod_freq: f32,
    #[serde(default)]
    ring_mod_mix: f32,
    #[serde(default)]
    ring_mod_level: f32,
    #[serde(default)]
    ring_mod_placement: f32,
    #[serde(default)]
    sizzle_osc_enable: f32,
    #[serde(default)]
    sizzle_wt_enable: f32,
    #[serde(default)]
    sizzle_dist_enable: f32,
    #[serde(default)]
    sizzle_cutoff: f32,
    #[serde(default)]
    spectral_enable: f32,
    #[serde(default)]
    spectral_amount: f32,
    #[serde(default)]
    spectral_tilt: f32,
    #[serde(default)]
    spectral_formant: f32,
    #[serde(default)]
    spectral_placement: f32,
    #[serde(default)]
    filter_tight_enable: f32,
    #[serde(default)]
    unison_voices: f32,
    #[serde(default)]
    unison_detune: f32,
    #[serde(default)]
    unison_spread: f32,
    #[serde(default)]
    glide_mode: f32,
    #[serde(default)]
    glide_time_ms: f32,
    lfo1_rate: f32,
    lfo1_attack: f32,
    lfo1_shape: f32,
    lfo2_rate: f32,
    lfo2_attack: f32,
    lfo2_shape: f32,
    mod1_source: f32,
    mod1_target: f32,
    mod1_amount: f32,
    #[serde(default)]
    mod1_smooth_ms: f32,
    mod2_source: f32,
    mod2_target: f32,
    mod2_amount: f32,
    #[serde(default)]
    mod2_smooth_ms: f32,
    #[serde(default)]
    mod3_source: f32,
    #[serde(default)]
    mod3_target: f32,
    #[serde(default)]
    mod3_amount: f32,
    #[serde(default)]
    mod3_smooth_ms: f32,
    #[serde(default)]
    mod4_source: f32,
    #[serde(default)]
    mod4_target: f32,
    #[serde(default)]
    mod4_amount: f32,
    #[serde(default)]
    mod4_smooth_ms: f32,
    #[serde(default)]
    mod5_source: f32,
    #[serde(default)]
    mod5_target: f32,
    #[serde(default)]
    mod5_amount: f32,
    #[serde(default)]
    mod5_smooth_ms: f32,
    #[serde(default)]
    mod6_source: f32,
    #[serde(default)]
    mod6_target: f32,
    #[serde(default)]
    mod6_amount: f32,
    #[serde(default)]
    mod6_smooth_ms: f32,
    #[serde(default)]
    seq_enable: f32,
    #[serde(default)]
    seq_rate: f32,
    #[serde(default)]
    seq_gate_amount: f32,
    #[serde(default)]
    seq_cut_amount: f32,
    #[serde(default)]
    seq_res_amount: f32,
    #[serde(default)]
    seq_wt_amount: f32,
    #[serde(default)]
    seq_dist_amount: f32,
    #[serde(default)]
    seq_fm_amount: f32,
    #[serde(default)]
    seq_steps: Vec<[f32; 32]>,
    amp_decay_ms: f32,
    #[serde(default)]
    amp_decay2_ms: f32,
    #[serde(default)]
    amp_decay2_level: f32,
    amp_sustain_level: f32,
    filter_type: f32,
    filter_cut: f32,
    filter_res: f32,
    filter_amount: f32,
    filter_cut_attack_ms: f32,
    #[serde(default)]
    filter_cut_hold_ms: f32,
    filter_cut_decay_ms: f32,
    #[serde(default)]
    filter_cut_decay2_ms: f32,
    #[serde(default)]
    filter_cut_decay2_level: f32,
    filter_cut_sustain_ms: f32,
    filter_cut_release_ms: f32,
    filter_res_attack_ms: f32,
    #[serde(default)]
    filter_res_hold_ms: f32,
    filter_res_decay_ms: f32,
    #[serde(default)]
    filter_res_decay2_ms: f32,
    #[serde(default)]
    filter_res_decay2_level: f32,
    filter_res_sustain_ms: f32,
    filter_res_release_ms: f32,
    amp_envelope_level: f32,
    filter_cut_envelope_level: f32,
    filter_res_envelope_level: f32,
    #[serde(default)]
    fm_enable: f32,
    #[serde(default)]
    fm_source: f32,
    #[serde(default)]
    fm_target: f32,
    #[serde(default)]
    fm_amount: f32,
    #[serde(default)]
    fm_ratio: f32,
    #[serde(default)]
    fm_feedback: f32,
    #[serde(default)]
    fm_env_attack_ms: f32,
    #[serde(default)]
    fm_env_hold_ms: f32,
    #[serde(default)]
    fm_env_decay_ms: f32,
    #[serde(default)]
    fm_env_decay2_ms: f32,
    #[serde(default)]
    fm_env_decay2_level: f32,
    #[serde(default)]
    fm_env_sustain_level: f32,
    #[serde(default)]
    fm_env_release_ms: f32,
    #[serde(default)]
    fm_env_amount: f32,
    vibrato_attack: f32,
    vibrato_intensity: f32,
    vibrato_rate: f32,
    tremolo_attack: f32,
    tremolo_intensity: f32,
    tremolo_rate: f32,
    vibrato_shape: f32,
    tremolo_shape: f32,
    filter_cut_env_polarity: f32,
    filter_res_env_polarity: f32,
    filter_cut_tension: f32,
    filter_res_tension: f32,
    cutoff_lfo_attack: f32,
    res_lfo_attack: f32,
    pan_lfo_attack: f32,
    cutoff_lfo_intensity: f32,
    cutoff_lfo_rate: f32,
    cutoff_lfo_shape: f32,
    res_lfo_intensity: f32,
    res_lfo_rate: f32,
    res_lfo_shape: f32,
    pan_lfo_intensity: f32,
    pan_lfo_rate: f32,
    pan_lfo_shape: f32,
    chorus_enable: f32,
    chorus_rate: f32,
    chorus_depth: f32,
    chorus_mix: f32,
    delay_enable: f32,
    delay_time_ms: f32,
    delay_feedback: f32,
    delay_mix: f32,
    reverb_enable: f32,
    reverb_size: f32,
    reverb_damp: f32,
    reverb_diffusion: f32,
    reverb_shimmer: f32,
    reverb_mix: f32,
    #[serde(default)]
    dist_enable: f32,
    #[serde(default)]
    dist_drive: f32,
    #[serde(default)]
    dist_tone: f32,
    #[serde(default)]
    dist_magic: f32,
    #[serde(default)]
    dist_mix: f32,
    #[serde(default)]
    dist_env_attack_ms: f32,
    #[serde(default)]
    dist_env_hold_ms: f32,
    #[serde(default)]
    dist_env_decay_ms: f32,
    #[serde(default)]
    dist_env_decay2_ms: f32,
    #[serde(default)]
    dist_env_decay2_level: f32,
    #[serde(default)]
    dist_env_sustain_level: f32,
    #[serde(default)]
    dist_env_release_ms: f32,
    #[serde(default)]
    dist_env_amount: f32,
    #[serde(default)]
    eq_enable: f32,
    #[serde(default)]
    eq_low_gain: f32,
    #[serde(default)]
    eq_mid_gain: f32,
    #[serde(default)]
    eq_mid_freq: f32,
    #[serde(default)]
    eq_mid_q: f32,
    #[serde(default)]
    eq_high_gain: f32,
    #[serde(default)]
    eq_mix: f32,
    #[serde(default)]
    output_sat_enable: f32,
    #[serde(default)]
    output_sat_type: f32,
    #[serde(default)]
    output_sat_drive: f32,
    #[serde(default)]
    output_sat_mix: f32,
    multi_filter_enable: f32,
    multi_filter_routing: f32,
    multi_filter_morph: f32,
    multi_filter_parallel_ab: f32,
    multi_filter_parallel_c: f32,
    multi_filter_a_type: f32,
    multi_filter_a_cut: f32,
    multi_filter_a_res: f32,
    multi_filter_a_amt: f32,
    multi_filter_b_type: f32,
    multi_filter_b_cut: f32,
    multi_filter_b_res: f32,
    multi_filter_b_amt: f32,
    multi_filter_c_type: f32,
    multi_filter_c_cut: f32,
    multi_filter_c_res: f32,
    multi_filter_c_amt: f32,
    limiter_enable: f32,
    limiter_threshold: f32,
    limiter_release: f32,
}

#[derive(Clone, Debug)]
struct PresetEntry {
    name: String,
    data: PresetData,
    user: bool,
}

#[derive(Lens)]
struct Data {
    params: Arc<SubSynthParams>,
    active_tab: usize,
    presets: Vec<PresetEntry>,
    preset_index: usize,
    preset_display: String,
    preset_name: String,
    seq_preset_index: usize,
    seq_preset_display: String,
    custom_wavetable_display: String,
    custom_wavetable_path_input: String,
}

enum UiEvent {
    SetTab(usize),
    PresetPrev,
    PresetNext,
    PresetLoad,
    PresetSave,
    PresetRefresh,
    PresetNameChanged(String),
    CustomWavetablePathChanged(String),
    PasteCustomWavetablePath,
    LoadCustomWavetablePath,
    SeqPresetPrev,
    SeqPresetNext,
    SeqPresetReset,
    SeqPresetRandom,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if let Some(msg) = event.take::<UiEvent>() {
            match msg {
                UiEvent::SetTab(tab) => {
                    self.active_tab = tab;
                }
                UiEvent::PresetPrev => {
                    if !self.presets.is_empty() {
                        if self.preset_index == 0 {
                            self.preset_index = self.presets.len() - 1;
                        } else {
                            self.preset_index -= 1;
                        }
                        self.preset_display = self.presets[self.preset_index].name.clone();
                    }
                }
                UiEvent::PresetNext => {
                    if !self.presets.is_empty() {
                        self.preset_index = (self.preset_index + 1) % self.presets.len();
                        self.preset_display = self.presets[self.preset_index].name.clone();
                    }
                }
                UiEvent::PresetLoad => {
                    if let Some(preset) = self.presets.get(self.preset_index) {
                        preset.data.apply(cx, &self.params);
                        if self.preset_index < FACTORY_PRESET_COUNT {
                            let normalized = self
                                .params
                                .preset_index
                                .preview_normalized(self.preset_index as i32);
                            apply_param(cx, &self.params.preset_index, normalized);
                        }
                    }
                }
                UiEvent::PresetSave => {
                    let name = sanitize_preset_name(&self.preset_name, &self.preset_display);
                    if !name.is_empty() {
                        let data = PresetData::from_params(&self.params);
                        if let Ok(saved) = save_user_preset(&name, &data) {
                            if let Some(existing) = self
                                .presets
                                .iter_mut()
                                .find(|preset| preset.user && preset.name == name)
                            {
                                existing.data = data;
                            } else {
                                self.presets.push(PresetEntry {
                                    name: name.clone(),
                                    data,
                                    user: true,
                                });
                                self.preset_index = self.presets.len() - 1;
                            }
                            self.preset_display = saved;
                        }
                    }
                }
                UiEvent::PresetRefresh => {
                    self.presets = load_presets(&self.params);
                    self.preset_index = self.preset_index.min(self.presets.len().saturating_sub(1));
                    if let Some(preset) = self.presets.get(self.preset_index) {
                        self.preset_display = preset.name.clone();
                    }
                }
                UiEvent::PresetNameChanged(value) => {
                    self.preset_name = value;
                }
                UiEvent::CustomWavetablePathChanged(value) => {
                    self.custom_wavetable_path_input = value;
                }
                UiEvent::PasteCustomWavetablePath => {
                    match paste_path_from_clipboard() {
                        Ok(Some(path)) => {
                            self.custom_wavetable_path_input = path;
                            self.load_custom_wavetable_from_input();
                        }
                        Ok(None) => {
                            self.custom_wavetable_display = "Clipboard empty".to_string();
                        }
                        Err(err) => {
                            self.custom_wavetable_display = format!("Clipboard error: {}", err);
                        }
                    }
                }
                UiEvent::LoadCustomWavetablePath => {
                    self.load_custom_wavetable_from_input();
                }
                        UiEvent::SeqPresetPrev => {
                            if self.seq_preset_index == 0 {
                                self.seq_preset_index = SEQ_PRESET_COUNT - 1;
                            } else {
                                self.seq_preset_index -= 1;
                            }
                            self.seq_preset_display = seq_preset_name(self.seq_preset_index).to_string();
                            let pattern = seq_pattern_for(self.seq_preset_index);
                            apply_seq_pattern(cx, &self.params, pattern);
                        }
                        UiEvent::SeqPresetNext => {
                            self.seq_preset_index = (self.seq_preset_index + 1) % SEQ_PRESET_COUNT;
                            self.seq_preset_display = seq_preset_name(self.seq_preset_index).to_string();
                            let pattern = seq_pattern_for(self.seq_preset_index);
                            apply_seq_pattern(cx, &self.params, pattern);
                        }
                        UiEvent::SeqPresetReset => {
                            let mut pattern = [[0.0; 32]; SEQ_LANE_COUNT];
                            for step in 0..SEQ_STEP_COUNT {
                                pattern[0][step] = -1.0;
                            }
                            apply_seq_pattern(cx, &self.params, pattern);
                        }
                        UiEvent::SeqPresetRandom => {
                            let mut rng = rand::thread_rng();
                            let mut pattern = [[0.0; 32]; SEQ_LANE_COUNT];
                            for step in 0..SEQ_STEP_COUNT {
                                pattern[0][step] = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                            }
                            for lane in 1..SEQ_LANE_COUNT {
                                for step in 0..SEQ_STEP_COUNT {
                                    pattern[lane][step] = rng.gen_range(-1.0..1.0);
                                }
                            }
                            apply_seq_pattern(cx, &self.params, pattern);
                        }
            }
        }
    }
}

impl Data {
    fn load_custom_wavetable_from_input(&mut self) {
        let path_text = self.custom_wavetable_path_input.trim();
        if path_text.is_empty() {
            self.custom_wavetable_display = "Enter WAV path".to_string();
            return;
        }

        let path = PathBuf::from(path_text);
        match load_wavetable_from_file(&path) {
            Ok(table) => {
                if let Ok(mut data) = self.params.custom_wavetable_data.write() {
                    *data = Some(table);
                }
                if let Ok(mut stored_path) = self.params.custom_wavetable_path.write() {
                    *stored_path = Some(path.to_string_lossy().to_string());
                }
                self.custom_wavetable_display = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Custom.wav")
                    .to_string();
            }
            Err(err) => {
                self.custom_wavetable_display = format!("WAV error: {}", err);
            }
        }
    }
}

fn pick_wav_path() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut buffer = vec![0u16; 1024];
        let filter = "WAV Files\0*.wav\0All Files\0*.*\0\0";
        let mut filter_wide: Vec<u16> = filter.encode_utf16().collect();

        let mut dialog = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            lpstrFile: buffer.as_mut_ptr(),
            nMaxFile: buffer.len() as u32,
            lpstrFilter: filter_wide.as_mut_ptr(),
            Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
            ..unsafe { std::mem::zeroed() }
        };

        let result = unsafe { GetOpenFileNameW(&mut dialog) };
        if result == 0 {
            return Ok(None);
        }

        let len = buffer.iter().position(|c| *c == 0).unwrap_or(0);
        let path = String::from_utf16_lossy(&buffer[..len]);
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("WAV picker is only available on Windows".to_string())
    }
}

fn paste_path_from_clipboard() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let opened = unsafe { OpenClipboard(0) };
        if opened == 0 {
            return Err("Failed to open clipboard".to_string());
        }

        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
        if handle == 0 {
            unsafe { CloseClipboard() };
            return Ok(None);
        }

        let ptr = unsafe { GlobalLock(handle as *mut std::ffi::c_void) };
        if ptr.is_null() {
            unsafe { CloseClipboard() };
            return Err("Failed to lock clipboard".to_string());
        }

        let mut len = 0usize;
        let mut cursor = ptr as *const u16;
        unsafe {
            while *cursor != 0 {
                len += 1;
                cursor = cursor.add(1);
            }
        }

        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u16, len) };
        let text = String::from_utf16_lossy(slice).trim().to_string();
        unsafe {
            GlobalUnlock(handle as *mut std::ffi::c_void);
            CloseClipboard();
        }

        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Clipboard paste is only available on Windows".to_string())
    }
}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (1500, 980))
}

fn create_label<'a, T>(
    cx: &'a mut Context,
    text: impl Res<T>,
    height: f32,
    width: f32,
    child_top: f32,
    child_bottom: f32,
) where
    T: ToString,
{
    Label::new(cx, text)
        .height(Pixels(height - 2.0))
        .width(Pixels(width - 10.0))
        .child_top(Stretch(child_top))
        .child_bottom(Pixels(child_bottom));
}

fn preset_root() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("DogSynth").join("Presets")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".dogsynth").join("presets")
    } else {
        PathBuf::from("presets")
    }
}

fn sanitize_preset_name(input: &str, fallback: &str) -> String {
    let name = input.trim();
    let base = if name.is_empty() { fallback.trim() } else { name };
    let cleaned: String = base
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    cleaned.trim().to_string()
}

fn save_user_preset(name: &str, data: &PresetData) -> Result<String, String> {
    let dir = preset_root();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let filename = format!("{}.syn", name);
    let path = dir.join(&filename);
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(name.to_string())
}

fn load_user_presets() -> Vec<PresetEntry> {
    let dir = preset_root();
    let mut presets = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return presets,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("syn") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("User Preset")
            .to_string();
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<PresetData>(&contents) {
                presets.push(PresetEntry {
                    name,
                    data,
                    user: true,
                });
            }
        }
    }

    presets
}

fn load_presets(params: &SubSynthParams) -> Vec<PresetEntry> {
    let mut presets = factory_presets(params);
    presets.extend(load_user_presets());
    presets
}

fn normalized<P: Param>(param: &P, plain: P::Plain) -> f32 {
    param.preview_normalized(plain)
}

pub(crate) fn factory_preset_names() -> Vec<String> {
    GM_PRESET_NAMES.iter().map(|name| name.to_string()).collect()
}

pub(crate) fn factory_preset_data(params: &SubSynthParams) -> Vec<PresetData> {
    gm_preset_data(params)
}

fn gm_group(index: usize) -> GmGroup {
    match index {
        0..=7 => GmGroup::Piano,
        8..=15 => GmGroup::Chromatic,
        16..=23 => GmGroup::Organ,
        24..=31 => GmGroup::Guitar,
        32..=39 => GmGroup::Bass,
        40..=47 => GmGroup::Strings,
        48..=55 => GmGroup::Ensemble,
        56..=63 => GmGroup::Brass,
        64..=71 => GmGroup::Reed,
        72..=79 => GmGroup::Pipe,
        80..=87 => GmGroup::SynthLead,
        88..=95 => GmGroup::SynthPad,
        96..=103 => GmGroup::SynthFx,
        104..=111 => GmGroup::Ethnic,
        112..=119 => GmGroup::Percussive,
        _ => GmGroup::Sfx,
    }
}

fn gm_env_settings(group: GmGroup) -> (f32, f32, f32, f32, f32, f32, f32) {
    match group {
        GmGroup::Piano => (0.02, 0.0, 0.8, 0.6, 0.4, 0.25, 0.8),
        GmGroup::Chromatic => (0.01, 0.0, 0.5, 0.4, 0.3, 0.1, 0.6),
        GmGroup::Organ => (0.03, 0.05, 0.2, 0.6, 0.85, 0.95, 1.0),
        GmGroup::Guitar => (0.01, 0.0, 0.6, 0.5, 0.35, 0.12, 0.7),
        GmGroup::Bass => (0.01, 0.0, 0.4, 0.5, 0.4, 0.65, 0.6),
        GmGroup::Strings => (0.2, 0.08, 0.8, 1.2, 0.8, 0.9, 1.4),
        GmGroup::Ensemble => (0.25, 0.1, 0.9, 1.4, 0.82, 0.92, 1.6),
        GmGroup::Brass => (0.08, 0.02, 0.6, 0.7, 0.7, 0.75, 0.9),
        GmGroup::Reed => (0.06, 0.02, 0.7, 0.8, 0.7, 0.78, 0.9),
        GmGroup::Pipe => (0.12, 0.03, 0.6, 0.9, 0.8, 0.85, 1.1),
        GmGroup::SynthLead => (0.02, 0.0, 0.35, 0.5, 0.6, 0.7, 0.6),
        GmGroup::SynthPad => (0.6, 0.2, 1.2, 2.0, 0.9, 0.95, 2.2),
        GmGroup::SynthFx => (0.1, 0.1, 1.0, 1.5, 0.6, 0.4, 1.6),
        GmGroup::Ethnic => (0.04, 0.02, 0.7, 0.9, 0.6, 0.5, 0.9),
        GmGroup::Percussive => (0.005, 0.0, 0.3, 0.25, 0.2, 0.0, 0.4),
        GmGroup::Sfx => (0.05, 0.1, 0.8, 1.2, 0.5, 0.3, 1.8),
    }
}

fn gm_filter_env_settings(group: GmGroup) -> (f32, f32, f32, f32, f32, f32, f32) {
    let (a, h, d, d2, d2_lvl, s, r) = gm_env_settings(group);
    (
        a * 0.6,
        h * 0.5,
        d * 0.6,
        d2 * 0.6,
        d2_lvl.clamp(0.2, 0.85),
        (s * 0.7).clamp(0.05, 0.9),
        r * 0.8,
    )
}

fn gm_waveform(group: GmGroup) -> Waveform {
    match group {
        GmGroup::Piano => Waveform::Triangle,
        GmGroup::Chromatic => Waveform::Sine,
        GmGroup::Organ => Waveform::Square,
        GmGroup::Guitar => Waveform::Sawtooth,
        GmGroup::Bass => Waveform::Sawtooth,
        GmGroup::Strings | GmGroup::Ensemble | GmGroup::Brass => Waveform::Sawtooth,
        GmGroup::Reed => Waveform::Pulse,
        GmGroup::Pipe => Waveform::Sine,
        GmGroup::SynthLead => Waveform::Sawtooth,
        GmGroup::SynthPad => Waveform::Triangle,
        GmGroup::SynthFx | GmGroup::Sfx => Waveform::Noise,
        GmGroup::Ethnic => Waveform::Triangle,
        GmGroup::Percussive => Waveform::Sine,
    }
}

fn gm_osc_routing(group: GmGroup) -> OscRouting {
    match group {
        GmGroup::SynthPad | GmGroup::SynthFx | GmGroup::Sfx => OscRouting::WavetableOnly,
        GmGroup::Percussive | GmGroup::Chromatic => OscRouting::ClassicOnly,
        _ => OscRouting::Blend,
    }
}

fn gm_preset_data(params: &SubSynthParams) -> Vec<PresetData> {
    let mut presets = Vec::with_capacity(FACTORY_PRESET_COUNT);
    let gain_min = util::db_to_gain(-36.0);
    let gain_max = util::db_to_gain(0.0);
    let default_gain = normalized(&params.gain, (gain_min + gain_max) * 0.5);

    for (index, _) in GM_PRESET_NAMES.iter().enumerate() {
        let group = gm_group(index);
        let mut preset = PresetData::from_params(params);
        let idx_mod = (index % 8) as f32 / 7.0;

        preset.gain = default_gain;
        preset.analog_enable = normalized(&params.analog_enable, false);
        preset.analog_drive = normalized(&params.analog_drive, 0.0);
        preset.analog_noise = normalized(&params.analog_noise, 0.0);
        preset.analog_drift = normalized(&params.analog_drift, 0.0);

        preset.waveform = normalized(&params.waveform, gm_waveform(group));
        preset.osc_routing = normalized(&params.osc_routing, gm_osc_routing(group));
        preset.osc_blend = normalized(&params.osc_blend, 0.45);
        preset.wavetable_position = normalized(&params.wavetable_position, 0.2 + 0.6 * idx_mod);
        preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.05 + 0.25 * idx_mod);
        preset.classic_drive = normalized(&params.classic_drive, 0.05 + 0.12 * idx_mod);

        preset.sub_level = normalized(&params.sub_level, if matches!(group, GmGroup::Bass) { 0.85 } else { 0.3 });
        preset.unison_voices = normalized(&params.unison_voices, if matches!(group, GmGroup::SynthPad | GmGroup::SynthLead) { UnisonVoices::Four } else { UnisonVoices::One });
        preset.unison_detune = normalized(&params.unison_detune, if matches!(group, GmGroup::SynthPad | GmGroup::SynthLead) { 0.22 } else { 0.08 });
        preset.unison_spread = normalized(&params.unison_spread, if matches!(group, GmGroup::SynthPad | GmGroup::SynthLead) { 0.4 } else { 0.12 });

        let (a, h, d, d2, d2_lvl, s, r) = gm_env_settings(group);
        preset.amp_attack_ms = normalized(&params.amp_attack_ms, a);
        preset.amp_hold_ms = normalized(&params.amp_hold_ms, h);
        preset.amp_decay_ms = normalized(&params.amp_decay_ms, d);
        preset.amp_decay2_ms = normalized(&params.amp_decay2_ms, d2);
        preset.amp_decay2_level = normalized(&params.amp_decay2_level, d2_lvl);
        preset.amp_sustain_level = normalized(&params.amp_sustain_level, s);
        preset.amp_release_ms = normalized(&params.amp_release_ms, r);

        let (fa, fh, fd, fd2, fd2_lvl, fs, fr) = gm_filter_env_settings(group);
        preset.filter_cut_attack_ms = normalized(&params.filter_cut_attack_ms, fa);
        preset.filter_cut_hold_ms = normalized(&params.filter_cut_hold_ms, fh);
        preset.filter_cut_decay_ms = normalized(&params.filter_cut_decay_ms, fd);
        preset.filter_cut_decay2_ms = normalized(&params.filter_cut_decay2_ms, fd2);
        preset.filter_cut_decay2_level = normalized(&params.filter_cut_decay2_level, fd2_lvl);
        preset.filter_cut_sustain_ms = normalized(&params.filter_cut_sustain_ms, fs);
        preset.filter_cut_release_ms = normalized(&params.filter_cut_release_ms, fr);
        preset.filter_cut_envelope_level = normalized(&params.filter_cut_envelope_level, 0.6);
        preset.filter_res_envelope_level = normalized(&params.filter_res_envelope_level, 0.3);
        preset.filter_res_attack_ms = normalized(&params.filter_res_attack_ms, fa * 0.8);
        preset.filter_res_hold_ms = normalized(&params.filter_res_hold_ms, fh * 0.8);
        preset.filter_res_decay_ms = normalized(&params.filter_res_decay_ms, fd * 0.7);
        preset.filter_res_decay2_ms = normalized(&params.filter_res_decay2_ms, fd2 * 0.7);
        preset.filter_res_decay2_level = normalized(&params.filter_res_decay2_level, fd2_lvl * 0.8);
        preset.filter_res_sustain_ms = normalized(&params.filter_res_sustain_ms, (fs * 0.8).clamp(0.05, 0.9));
        preset.filter_res_release_ms = normalized(&params.filter_res_release_ms, fr * 0.8);

        preset.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
        let cut_base = match group {
            GmGroup::Bass => 160.0,
            GmGroup::Guitar => 1200.0,
            GmGroup::Strings | GmGroup::Ensemble => 1800.0,
            GmGroup::Brass | GmGroup::Reed => 1600.0,
            GmGroup::Pipe => 1400.0,
            GmGroup::SynthPad => 1100.0,
            GmGroup::SynthLead => 2400.0,
            GmGroup::SynthFx | GmGroup::Sfx => 2600.0,
            GmGroup::Percussive => 2000.0,
            _ => 1500.0,
        };
        preset.filter_cut = normalized(&params.filter_cut, cut_base + 400.0 * idx_mod);
        preset.filter_res = normalized(&params.filter_res, match group {
            GmGroup::Percussive => 0.15,
            GmGroup::SynthLead => 0.35,
            GmGroup::SynthPad => 0.28,
            _ => 0.2,
        });
        preset.filter_amount = normalized(&params.filter_amount, 0.85);

        preset.classic_level = normalized(&params.classic_level, 1.0);
        preset.wavetable_level = normalized(&params.wavetable_level, 1.0);
        preset.noise_level = normalized(&params.noise_level, if matches!(group, GmGroup::Sfx) { 0.35 } else { 0.0 });

        preset.chorus_enable = normalized(&params.chorus_enable, matches!(group, GmGroup::SynthPad | GmGroup::Strings | GmGroup::Ensemble));
        preset.chorus_mix = normalized(&params.chorus_mix, if matches!(group, GmGroup::SynthPad | GmGroup::Strings | GmGroup::Ensemble) { 0.45 } else { 0.0 });
        preset.reverb_enable = normalized(&params.reverb_enable, matches!(group, GmGroup::SynthPad | GmGroup::Strings | GmGroup::Ensemble | GmGroup::SynthFx));
        preset.reverb_mix = normalized(&params.reverb_mix, if matches!(group, GmGroup::SynthPad | GmGroup::Strings | GmGroup::Ensemble | GmGroup::SynthFx) { 0.3 } else { 0.0 });
        preset.delay_enable = normalized(&params.delay_enable, matches!(group, GmGroup::SynthLead));
        preset.delay_mix = normalized(&params.delay_mix, if matches!(group, GmGroup::SynthLead) { 0.25 } else { 0.0 });

        preset.spectral_enable = normalized(
            &params.spectral_enable,
            matches!(group, GmGroup::SynthFx | GmGroup::Sfx),
        );
        preset.spectral_amount = normalized(
            &params.spectral_amount,
            if matches!(group, GmGroup::SynthFx | GmGroup::Sfx) { 0.35 } else { 0.0 },
        );
        preset.spectral_tilt = normalized(
            &params.spectral_tilt,
            if matches!(group, GmGroup::SynthFx | GmGroup::Sfx) { 0.1 } else { 0.0 },
        );
        preset.spectral_formant = normalized(
            &params.spectral_formant,
            if matches!(group, GmGroup::SynthFx | GmGroup::Sfx) { 0.4 } else { 0.0 },
        );
        preset.spectral_placement = normalized(
            &params.spectral_placement,
            if matches!(group, GmGroup::SynthFx | GmGroup::Sfx) {
                SpectralPlacement::PreFx
            } else {
                SpectralPlacement::PreFx
            },
        );

        preset.fm_enable = normalized(&params.fm_enable, false);
        preset.fm_amount = normalized(&params.fm_amount, 0.0);
        preset.fm_env_amount = normalized(&params.fm_env_amount, 0.0);
        preset.fm_env_attack_ms = normalized(&params.fm_env_attack_ms, 0.0);
        preset.fm_env_hold_ms = normalized(&params.fm_env_hold_ms, 0.0);
        preset.fm_env_decay_ms = normalized(&params.fm_env_decay_ms, 0.0);
        preset.fm_env_decay2_ms = normalized(&params.fm_env_decay2_ms, 0.0);
        preset.fm_env_decay2_level = normalized(&params.fm_env_decay2_level, 0.0);
        preset.fm_env_sustain_level = normalized(&params.fm_env_sustain_level, 0.0);
        preset.fm_env_release_ms = normalized(&params.fm_env_release_ms, 0.0);
        let use_dist = matches!(group, GmGroup::Guitar | GmGroup::SynthLead | GmGroup::SynthFx);
        preset.dist_enable = normalized(&params.dist_enable, use_dist);
        preset.dist_drive = normalized(&params.dist_drive, if use_dist { 0.28 } else { 0.0 });
        preset.dist_tone = normalized(&params.dist_tone, if use_dist { 0.6 } else { 0.5 });
        preset.dist_magic = normalized(&params.dist_magic, if use_dist { 0.18 } else { 0.0 });
        preset.dist_mix = normalized(&params.dist_mix, if use_dist { 0.35 } else { 0.0 });
        preset.dist_env_attack_ms = normalized(&params.dist_env_attack_ms, 0.0);
        preset.dist_env_hold_ms = normalized(&params.dist_env_hold_ms, 0.0);
        preset.dist_env_decay_ms = normalized(&params.dist_env_decay_ms, 0.0);
        preset.dist_env_decay2_ms = normalized(&params.dist_env_decay2_ms, 0.0);
        preset.dist_env_decay2_level = normalized(&params.dist_env_decay2_level, 0.0);
        preset.dist_env_sustain_level = normalized(&params.dist_env_sustain_level, 0.0);
        preset.dist_env_release_ms = normalized(&params.dist_env_release_ms, 0.0);
        let use_sat = matches!(group, GmGroup::Bass | GmGroup::SynthLead);
        preset.output_sat_enable = normalized(&params.output_sat_enable, use_sat);
        preset.output_sat_drive = normalized(&params.output_sat_drive, if use_sat { 0.2 } else { 0.0 });
        preset.output_sat_mix = normalized(&params.output_sat_mix, if use_sat { 0.4 } else { 0.0 });

        preset.limiter_enable = normalized(&params.limiter_enable, true);

        presets.push(preset);
    }

    presets
}

fn base_preset_data(params: &SubSynthParams, base: BasePreset) -> PresetData {
    let mut preset = match base {
        BasePreset::Init => PresetData::from_params(params),
        BasePreset::LaserLead => base_lead(params),
        BasePreset::EdgeLead => base_lead(params),
        BasePreset::SilkLead => base_lead(params),
        BasePreset::SubBass => base_bass(params),
        BasePreset::RubberBass => base_bass(params),
        BasePreset::GrindBass => base_bass(params),
        BasePreset::ThickBass => base_bass(params),
        BasePreset::SuperSaw => base_trance_saw(params),
        BasePreset::LiftSaw => base_trance_saw(params),
        BasePreset::HaloSaw => base_trance_saw(params),
        BasePreset::WideSaw => base_trance_saw(params),
        BasePreset::MetalReese => base_reese(params),
        BasePreset::DarkReese => base_reese(params),
        BasePreset::WideReese => base_reese(params),
        BasePreset::DirtyReese => base_reese(params),
    };

    match base {
        BasePreset::Init => {
            preset.limiter_enable = normalized(&params.limiter_enable, true);
        }
        BasePreset::LaserLead => {
            preset.wavetable_position = normalized(&params.wavetable_position, 0.52);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.18);
            preset.filter_cut = normalized(&params.filter_cut, 1400.0);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 0.2);
            preset.amp_decay_ms = normalized(&params.amp_decay_ms, 2.4);
            preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.15);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 1.8);
            preset.filter_cut_envelope_level = normalized(&params.filter_cut_envelope_level, 0.9);
            preset.filter_cut_attack_ms = normalized(&params.filter_cut_attack_ms, 0.3);
            preset.filter_cut_decay_ms = normalized(&params.filter_cut_decay_ms, 3.0);
            preset.filter_cut_sustain_ms = normalized(&params.filter_cut_sustain_ms, 0.0);
            preset.filter_cut_release_ms = normalized(&params.filter_cut_release_ms, 2.2);
            preset.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
            preset.mod1_target = normalized(&params.mod1_target, ModTarget::FilterCut);
            preset.mod1_amount = normalized(&params.mod1_amount, 0.15);
        }
        BasePreset::EdgeLead => {
            preset.wavetable_position = normalized(&params.wavetable_position, 0.6);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.22);
            preset.filter_cut = normalized(&params.filter_cut, 2400.0);
            preset.filter_res = normalized(&params.filter_res, 0.2);
            preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Six);
            preset.unison_detune = normalized(&params.unison_detune, 0.3);
            preset.unison_spread = normalized(&params.unison_spread, 0.6);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 4.0);
            preset.amp_decay_ms = normalized(&params.amp_decay_ms, 6.0);
            preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.85);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 7.0);
            preset.delay_mix = normalized(&params.delay_mix, 0.32);
            preset.reverb_mix = normalized(&params.reverb_mix, 0.28);
        }
        BasePreset::SilkLead => {
            preset.wavetable_position = normalized(&params.wavetable_position, 0.45);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.08);
            preset.waveform = normalized(&params.waveform, Waveform::Triangle);
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
            preset.sub_level = normalized(&params.sub_level, 0.0);
            preset.filter_cut = normalized(&params.filter_cut, 1200.0);
            preset.filter_res = normalized(&params.filter_res, 0.18);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 6.0);
            preset.amp_decay_ms = normalized(&params.amp_decay_ms, 5.0);
            preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.8);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 8.0);
            preset.vibrato_intensity = normalized(&params.vibrato_intensity, 0.12);
            preset.vibrato_rate = normalized(&params.vibrato_rate, 5.2);
            preset.reverb_mix = normalized(&params.reverb_mix, 0.32);
        }
        BasePreset::SubBass => {
            preset.waveform = normalized(&params.waveform, Waveform::Sine);
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
            preset.sub_level = normalized(&params.sub_level, 0.9);
            preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::One);
            preset.filter_cut = normalized(&params.filter_cut, 70.0);
            preset.filter_res = normalized(&params.filter_res, 0.15);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 3.0);
            preset.amp_decay_ms = normalized(&params.amp_decay_ms, 5.0);
            preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.9);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 6.0);
            preset.reverb_enable = normalized(&params.reverb_enable, false);
            preset.delay_enable = normalized(&params.delay_enable, false);
            preset.chorus_enable = normalized(&params.chorus_enable, false);
        }
        BasePreset::RubberBass => {
            preset.waveform = normalized(&params.waveform, Waveform::Sawtooth);
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
            preset.sub_level = normalized(&params.sub_level, 0.6);
            preset.filter_cut = normalized(&params.filter_cut, 220.0);
            preset.filter_res = normalized(&params.filter_res, 0.5);
            preset.filter_cut_envelope_level = normalized(&params.filter_cut_envelope_level, 0.85);
            preset.filter_amount = normalized(&params.filter_amount, 1.0);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 0.1);
            preset.amp_decay_ms = normalized(&params.amp_decay_ms, 2.0);
            preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.2);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 1.5);
            preset.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
            preset.mod1_target = normalized(&params.mod1_target, ModTarget::FilterCut);
            preset.mod1_amount = normalized(&params.mod1_amount, 0.22);
        }
        BasePreset::GrindBass => {
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
            preset.wavetable_position = normalized(&params.wavetable_position, 0.6);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.35);
            preset.sub_level = normalized(&params.sub_level, 0.5);
            preset.filter_cut = normalized(&params.filter_cut, 220.0);
            preset.filter_res = normalized(&params.filter_res, 0.4);
            preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Two);
            preset.unison_detune = normalized(&params.unison_detune, 0.18);
            preset.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
            preset.mod1_target = normalized(&params.mod1_target, ModTarget::FilterCut);
            preset.mod1_amount = normalized(&params.mod1_amount, 0.45);
            preset.mod2_source = normalized(&params.mod2_source, ModSource::Lfo2);
            preset.mod2_target = normalized(&params.mod2_target, ModTarget::WavetablePos);
            preset.mod2_amount = normalized(&params.mod2_amount, 0.3);
        }
        BasePreset::ThickBass => {
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
            preset.osc_blend = normalized(&params.osc_blend, 0.4);
            preset.sub_level = normalized(&params.sub_level, 0.75);
            preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Two);
            preset.unison_detune = normalized(&params.unison_detune, 0.12);
            preset.filter_cut = normalized(&params.filter_cut, 180.0);
            preset.filter_res = normalized(&params.filter_res, 0.3);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 1.0);
            preset.amp_decay_ms = normalized(&params.amp_decay_ms, 4.2);
            preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.7);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 4.0);
        }
        BasePreset::SuperSaw => {
            preset.filter_cut = normalized(&params.filter_cut, 2100.0);
            preset.chorus_depth = normalized(&params.chorus_depth, 26.0);
            preset.chorus_mix = normalized(&params.chorus_mix, 0.5);
            preset.delay_time_ms = normalized(&params.delay_time_ms, 420.0);
            preset.reverb_mix = normalized(&params.reverb_mix, 0.28);
        }
        BasePreset::LiftSaw => {
            preset.filter_cut = normalized(&params.filter_cut, 2600.0);
            preset.amp_attack_ms = normalized(&params.amp_attack_ms, 3.0);
            preset.amp_release_ms = normalized(&params.amp_release_ms, 6.8);
            preset.reverb_mix = normalized(&params.reverb_mix, 0.36);
            preset.delay_mix = normalized(&params.delay_mix, 0.32);
        }
        BasePreset::HaloSaw => {
            preset.filter_cut = normalized(&params.filter_cut, 1900.0);
            preset.chorus_depth = normalized(&params.chorus_depth, 30.0);
            preset.chorus_mix = normalized(&params.chorus_mix, 0.6);
            preset.reverb_mix = normalized(&params.reverb_mix, 0.38);
        }
        BasePreset::WideSaw => {
            preset.filter_cut = normalized(&params.filter_cut, 2000.0);
            preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Six);
            preset.unison_detune = normalized(&params.unison_detune, 0.32);
            preset.unison_spread = normalized(&params.unison_spread, 0.6);
            preset.chorus_mix = normalized(&params.chorus_mix, 0.55);
        }
        BasePreset::MetalReese => {
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
            preset.osc_blend = normalized(&params.osc_blend, 0.65);
            preset.wavetable_position = normalized(&params.wavetable_position, 0.38);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.55);
            preset.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
            preset.filter_cut = normalized(&params.filter_cut, 420.0);
            preset.filter_res = normalized(&params.filter_res, 0.35);
            preset.dist_drive = normalized(&params.dist_drive, 0.55);
            preset.dist_mix = normalized(&params.dist_mix, 0.6);
            preset.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
            preset.mod1_target = normalized(&params.mod1_target, ModTarget::Pan);
            preset.mod1_amount = normalized(&params.mod1_amount, 0.2);
        }
        BasePreset::DarkReese => {
            preset.wavetable_position = normalized(&params.wavetable_position, 0.32);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.65);
            preset.filter_cut = normalized(&params.filter_cut, 320.0);
            preset.filter_res = normalized(&params.filter_res, 0.45);
            preset.dist_drive = normalized(&params.dist_drive, 0.6);
            preset.dist_mix = normalized(&params.dist_mix, 0.65);
        }
        BasePreset::WideReese => {
            preset.wavetable_position = normalized(&params.wavetable_position, 0.44);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.5);
            preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Six);
            preset.unison_detune = normalized(&params.unison_detune, 0.28);
            preset.unison_spread = normalized(&params.unison_spread, 0.65);
            preset.chorus_mix = normalized(&params.chorus_mix, 0.5);
        }
        BasePreset::DirtyReese => {
            preset.wavetable_position = normalized(&params.wavetable_position, 0.58);
            preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.75);
            preset.filter_cut = normalized(&params.filter_cut, 520.0);
            preset.filter_res = normalized(&params.filter_res, 0.4);
            preset.dist_drive = normalized(&params.dist_drive, 0.75);
            preset.dist_mix = normalized(&params.dist_mix, 0.7);
            preset.classic_drive = normalized(&params.classic_drive, 0.35);
        }
    }

    preset
}

fn base_lead(params: &SubSynthParams) -> PresetData {
    let mut preset = PresetData::from_params(params);
    preset.waveform = normalized(&params.waveform, Waveform::Sawtooth);
    preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    preset.osc_blend = normalized(&params.osc_blend, 0.5);
    preset.wavetable_position = normalized(&params.wavetable_position, 0.48);
    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.08);
    preset.classic_drive = normalized(&params.classic_drive, 0.08);
    preset.sub_level = normalized(&params.sub_level, 0.18);
    preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Four);
    preset.unison_detune = normalized(&params.unison_detune, 0.22);
    preset.unison_spread = normalized(&params.unison_spread, 0.38);
    preset.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    preset.filter_cut = normalized(&params.filter_cut, 1400.0);
    preset.filter_res = normalized(&params.filter_res, 0.18);
    preset.filter_amount = normalized(&params.filter_amount, 0.75);
    preset.amp_attack_ms = normalized(&params.amp_attack_ms, 4.0);
    preset.amp_decay_ms = normalized(&params.amp_decay_ms, 3.2);
    preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.6);
    preset.amp_release_ms = normalized(&params.amp_release_ms, 3.0);
    preset.dist_enable = normalized(&params.dist_enable, true);
    preset.dist_drive = normalized(&params.dist_drive, 0.08);
    preset.dist_tone = normalized(&params.dist_tone, 0.55);
    preset.dist_magic = normalized(&params.dist_magic, 0.08);
    preset.dist_mix = normalized(&params.dist_mix, 0.18);
    preset.eq_enable = normalized(&params.eq_enable, true);
    preset.eq_low_gain = normalized(&params.eq_low_gain, 1.2);
    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.4);
    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 950.0);
    preset.eq_mid_q = normalized(&params.eq_mid_q, 0.8);
    preset.eq_high_gain = normalized(&params.eq_high_gain, 2.0);
    preset.eq_mix = normalized(&params.eq_mix, 1.0);
    preset.chorus_enable = normalized(&params.chorus_enable, true);
    preset.chorus_depth = normalized(&params.chorus_depth, 26.0);
    preset.chorus_mix = normalized(&params.chorus_mix, 0.45);
    preset.delay_enable = normalized(&params.delay_enable, true);
    preset.delay_time_ms = normalized(&params.delay_time_ms, 360.0);
    preset.delay_feedback = normalized(&params.delay_feedback, 0.24);
    preset.delay_mix = normalized(&params.delay_mix, 0.3);
    preset.reverb_enable = normalized(&params.reverb_enable, true);
    preset.reverb_size = normalized(&params.reverb_size, 0.65);
    preset.reverb_mix = normalized(&params.reverb_mix, 0.26);
    preset.limiter_enable = normalized(&params.limiter_enable, true);
    preset
}

fn base_bass(params: &SubSynthParams) -> PresetData {
    let mut preset = PresetData::from_params(params);
    preset.waveform = normalized(&params.waveform, Waveform::Sawtooth);
    preset.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
    preset.osc_blend = normalized(&params.osc_blend, 0.0);
    preset.wavetable_position = normalized(&params.wavetable_position, 0.35);
    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.12);
    preset.classic_drive = normalized(&params.classic_drive, 0.12);
    preset.sub_level = normalized(&params.sub_level, 0.8);
    preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Two);
    preset.unison_detune = normalized(&params.unison_detune, 0.1);
    preset.unison_spread = normalized(&params.unison_spread, 0.18);
    preset.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    preset.filter_cut = normalized(&params.filter_cut, 160.0);
    preset.filter_res = normalized(&params.filter_res, 0.22);
    preset.filter_amount = normalized(&params.filter_amount, 1.0);
    preset.amp_attack_ms = normalized(&params.amp_attack_ms, 0.6);
    preset.amp_decay_ms = normalized(&params.amp_decay_ms, 3.2);
    preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.75);
    preset.amp_release_ms = normalized(&params.amp_release_ms, 2.6);
    preset.dist_enable = normalized(&params.dist_enable, true);
    preset.dist_drive = normalized(&params.dist_drive, 0.1);
    preset.dist_tone = normalized(&params.dist_tone, 0.45);
    preset.dist_magic = normalized(&params.dist_magic, 0.08);
    preset.dist_mix = normalized(&params.dist_mix, 0.2);
    preset.eq_enable = normalized(&params.eq_enable, true);
    preset.eq_low_gain = normalized(&params.eq_low_gain, 3.2);
    preset.eq_mid_gain = normalized(&params.eq_mid_gain, -0.5);
    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 520.0);
    preset.eq_mid_q = normalized(&params.eq_mid_q, 0.9);
    preset.eq_high_gain = normalized(&params.eq_high_gain, -2.0);
    preset.eq_mix = normalized(&params.eq_mix, 1.0);
    preset.chorus_enable = normalized(&params.chorus_enable, true);
    preset.chorus_depth = normalized(&params.chorus_depth, 12.0);
    preset.chorus_mix = normalized(&params.chorus_mix, 0.18);
    preset.delay_enable = normalized(&params.delay_enable, false);
    preset.reverb_enable = normalized(&params.reverb_enable, false);
    preset.limiter_enable = normalized(&params.limiter_enable, true);
    preset
}

fn base_trance_saw(params: &SubSynthParams) -> PresetData {
    let mut preset = PresetData::from_params(params);
    preset.waveform = normalized(&params.waveform, Waveform::Sawtooth);
    preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    preset.osc_blend = normalized(&params.osc_blend, 0.6);
    preset.wavetable_position = normalized(&params.wavetable_position, 0.46);
    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.12);
    preset.classic_drive = normalized(&params.classic_drive, 0.1);
    preset.sub_level = normalized(&params.sub_level, 0.15);
    preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Six);
    preset.unison_detune = normalized(&params.unison_detune, 0.24);
    preset.unison_spread = normalized(&params.unison_spread, 0.5);
    preset.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    preset.filter_cut = normalized(&params.filter_cut, 1500.0);
    preset.filter_res = normalized(&params.filter_res, 0.18);
    preset.filter_amount = normalized(&params.filter_amount, 0.75);
    preset.amp_attack_ms = normalized(&params.amp_attack_ms, 0.6);
    preset.amp_decay_ms = normalized(&params.amp_decay_ms, 2.4);
    preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.35);
    preset.amp_release_ms = normalized(&params.amp_release_ms, 2.2);
    preset.filter_cut_envelope_level = normalized(&params.filter_cut_envelope_level, 0.65);
    preset.filter_cut_attack_ms = normalized(&params.filter_cut_attack_ms, 0.2);
    preset.filter_cut_decay_ms = normalized(&params.filter_cut_decay_ms, 2.2);
    preset.filter_cut_release_ms = normalized(&params.filter_cut_release_ms, 1.8);
    preset.dist_enable = normalized(&params.dist_enable, true);
    preset.dist_drive = normalized(&params.dist_drive, 0.12);
    preset.dist_tone = normalized(&params.dist_tone, 0.6);
    preset.dist_magic = normalized(&params.dist_magic, 0.1);
    preset.dist_mix = normalized(&params.dist_mix, 0.18);
    preset.eq_enable = normalized(&params.eq_enable, true);
    preset.eq_low_gain = normalized(&params.eq_low_gain, 1.8);
    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.4);
    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 900.0);
    preset.eq_mid_q = normalized(&params.eq_mid_q, 0.85);
    preset.eq_high_gain = normalized(&params.eq_high_gain, 2.6);
    preset.eq_mix = normalized(&params.eq_mix, 1.0);
    preset.chorus_enable = normalized(&params.chorus_enable, true);
    preset.chorus_depth = normalized(&params.chorus_depth, 20.0);
    preset.chorus_mix = normalized(&params.chorus_mix, 0.35);
    preset.delay_enable = normalized(&params.delay_enable, true);
    preset.delay_time_ms = normalized(&params.delay_time_ms, 320.0);
    preset.delay_feedback = normalized(&params.delay_feedback, 0.28);
    preset.delay_mix = normalized(&params.delay_mix, 0.3);
    preset.reverb_enable = normalized(&params.reverb_enable, true);
    preset.reverb_size = normalized(&params.reverb_size, 0.6);
    preset.reverb_mix = normalized(&params.reverb_mix, 0.32);
    preset.limiter_enable = normalized(&params.limiter_enable, true);
    preset
}

fn base_reese(params: &SubSynthParams) -> PresetData {
    let mut preset = PresetData::from_params(params);
    preset.waveform = normalized(&params.waveform, Waveform::Sawtooth);
    preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    preset.osc_blend = normalized(&params.osc_blend, 0.6);
    preset.wavetable_position = normalized(&params.wavetable_position, 0.4);
    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.4);
    preset.classic_drive = normalized(&params.classic_drive, 0.15);
    preset.sub_level = normalized(&params.sub_level, 0.55);
    preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Four);
    preset.unison_detune = normalized(&params.unison_detune, 0.2);
    preset.unison_spread = normalized(&params.unison_spread, 0.36);
    preset.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
    preset.filter_cut = normalized(&params.filter_cut, 360.0);
    preset.filter_res = normalized(&params.filter_res, 0.32);
    preset.filter_amount = normalized(&params.filter_amount, 1.0);
    preset.amp_attack_ms = normalized(&params.amp_attack_ms, 0.6);
    preset.amp_decay_ms = normalized(&params.amp_decay_ms, 4.4);
    preset.amp_sustain_level = normalized(&params.amp_sustain_level, 0.7);
    preset.amp_release_ms = normalized(&params.amp_release_ms, 4.4);
    preset.dist_enable = normalized(&params.dist_enable, true);
    preset.dist_drive = normalized(&params.dist_drive, 0.35);
    preset.dist_tone = normalized(&params.dist_tone, 0.5);
    preset.dist_magic = normalized(&params.dist_magic, 0.25);
    preset.dist_mix = normalized(&params.dist_mix, 0.4);
    preset.spectral_enable = normalized(&params.spectral_enable, true);
    preset.spectral_amount = normalized(&params.spectral_amount, 0.35);
    preset.spectral_tilt = normalized(&params.spectral_tilt, -0.2);
    preset.spectral_formant = normalized(&params.spectral_formant, 0.6);
    preset.spectral_placement = normalized(&params.spectral_placement, SpectralPlacement::PreDist);
    preset.eq_enable = normalized(&params.eq_enable, true);
    preset.eq_low_gain = normalized(&params.eq_low_gain, 1.8);
    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 1.2);
    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 720.0);
    preset.eq_mid_q = normalized(&params.eq_mid_q, 0.9);
    preset.eq_high_gain = normalized(&params.eq_high_gain, 0.6);
    preset.eq_mix = normalized(&params.eq_mix, 1.0);
    preset.chorus_enable = normalized(&params.chorus_enable, false);
    preset.delay_enable = normalized(&params.delay_enable, false);
    preset.reverb_enable = normalized(&params.reverb_enable, false);
    preset.limiter_enable = normalized(&params.limiter_enable, true);
    preset
}

fn apply_variant(preset: &mut PresetData, params: &SubSynthParams, variant: usize) {
    let cut_plain = params.filter_cut.preview_plain(preset.filter_cut);
    let res_plain = params.filter_res.preview_plain(preset.filter_res);
    let detune_plain = params.unison_detune.preview_plain(preset.unison_detune);
    let spread_plain = params.unison_spread.preview_plain(preset.unison_spread);
    let release_plain = params.amp_release_ms.preview_plain(preset.amp_release_ms);

    match variant {
        0 => {}
        1 => {
            preset.filter_cut = normalized(
                &params.filter_cut,
                (cut_plain * 1.3).clamp(80.0, 20000.0),
            );
            preset.filter_res = normalized(&params.filter_res, (res_plain * 0.85).clamp(0.0, 1.0));
            preset.eq_high_gain = normalized(&params.eq_high_gain, 4.0);
        }
        2 => {
            preset.filter_cut = normalized(
                &params.filter_cut,
                (cut_plain * 0.85).clamp(60.0, 20000.0),
            );
            preset.eq_low_gain = normalized(&params.eq_low_gain, 2.5);
            preset.eq_mid_gain = normalized(&params.eq_mid_gain, 1.2);
            preset.dist_drive = normalized(&params.dist_drive, 0.25);
        }
        3 => {
            preset.unison_detune = normalized(
                &params.unison_detune,
                (detune_plain * 1.35).clamp(0.0, 1.0),
            );
            preset.unison_spread = normalized(
                &params.unison_spread,
                (spread_plain * 1.4).clamp(0.0, 1.0),
            );
            preset.chorus_mix = normalized(&params.chorus_mix, 0.5);
        }
        4 => {
            preset.unison_detune = normalized(
                &params.unison_detune,
                (detune_plain * 0.65).clamp(0.0, 1.0),
            );
            preset.unison_spread = normalized(
                &params.unison_spread,
                (spread_plain * 0.6).clamp(0.0, 1.0),
            );
            preset.amp_release_ms = normalized(
                &params.amp_release_ms,
                (release_plain * 0.6).clamp(0.0, 10.0),
            );
            preset.delay_mix = normalized(&params.delay_mix, 0.15);
            preset.reverb_mix = normalized(&params.reverb_mix, 0.12);
        }
        5 => {
            preset.filter_res = normalized(&params.filter_res, (res_plain + 0.2).clamp(0.0, 1.0));
            preset.eq_mid_gain = normalized(&params.eq_mid_gain, 2.0);
            preset.eq_mid_freq = normalized(&params.eq_mid_freq, 950.0);
        }
        6 => {
            preset.filter_cut = normalized(
                &params.filter_cut,
                (cut_plain * 1.2).clamp(120.0, 20000.0),
            );
            preset.reverb_mix = normalized(&params.reverb_mix, 0.4);
            preset.delay_mix = normalized(&params.delay_mix, 0.35);
            preset.eq_high_gain = normalized(&params.eq_high_gain, 4.5);
        }
        _ => {
            preset.filter_cut = normalized(
                &params.filter_cut,
                (cut_plain * 0.7).clamp(40.0, 20000.0),
            );
            preset.eq_high_gain = normalized(&params.eq_high_gain, -1.5);
            preset.dist_tone = normalized(&params.dist_tone, 0.45);
        }
    }
}

impl PresetData {
    fn from_params(params: &SubSynthParams) -> Self {
        Self {
            gain: params.gain.unmodulated_normalized_value(),
            amp_attack_ms: params.amp_attack_ms.unmodulated_normalized_value(),
            amp_hold_ms: params.amp_hold_ms.unmodulated_normalized_value(),
            amp_release_ms: params.amp_release_ms.unmodulated_normalized_value(),
            amp_tension: params.amp_tension.unmodulated_normalized_value(),
            waveform: params.waveform.unmodulated_normalized_value(),
            osc_routing: params.osc_routing.unmodulated_normalized_value(),
            osc_blend: params.osc_blend.unmodulated_normalized_value(),
            wavetable_position: params.wavetable_position.unmodulated_normalized_value(),
            wavetable_distortion: params.wavetable_distortion.unmodulated_normalized_value(),
            classic_drive: params.classic_drive.unmodulated_normalized_value(),
            custom_wavetable_enable: params.custom_wavetable_enable.unmodulated_normalized_value(),
            analog_enable: params.analog_enable.unmodulated_normalized_value(),
            analog_drive: params.analog_drive.unmodulated_normalized_value(),
            analog_noise: params.analog_noise.unmodulated_normalized_value(),
            analog_drift: params.analog_drift.unmodulated_normalized_value(),
            sub_level: params.sub_level.unmodulated_normalized_value(),
            classic_level: params.classic_level.unmodulated_normalized_value(),
            wavetable_level: params.wavetable_level.unmodulated_normalized_value(),
            noise_level: params.noise_level.unmodulated_normalized_value(),
            classic_send: params.classic_send.unmodulated_normalized_value(),
            wavetable_send: params.wavetable_send.unmodulated_normalized_value(),
            sub_send: params.sub_send.unmodulated_normalized_value(),
            noise_send: params.noise_send.unmodulated_normalized_value(),
            ring_mod_send: params.ring_mod_send.unmodulated_normalized_value(),
            fx_bus_mix: params.fx_bus_mix.unmodulated_normalized_value(),
            ring_mod_enable: params.ring_mod_enable.unmodulated_normalized_value(),
            ring_mod_source: params.ring_mod_source.unmodulated_normalized_value(),
            ring_mod_freq: params.ring_mod_freq.unmodulated_normalized_value(),
            ring_mod_mix: params.ring_mod_mix.unmodulated_normalized_value(),
            ring_mod_level: params.ring_mod_level.unmodulated_normalized_value(),
            ring_mod_placement: params.ring_mod_placement.unmodulated_normalized_value(),
            sizzle_osc_enable: params.sizzle_osc_enable.unmodulated_normalized_value(),
            sizzle_wt_enable: params.sizzle_wt_enable.unmodulated_normalized_value(),
            sizzle_dist_enable: params.sizzle_dist_enable.unmodulated_normalized_value(),
            sizzle_cutoff: params.sizzle_cutoff.unmodulated_normalized_value(),
            spectral_enable: params.spectral_enable.unmodulated_normalized_value(),
            spectral_amount: params.spectral_amount.unmodulated_normalized_value(),
            spectral_tilt: params.spectral_tilt.unmodulated_normalized_value(),
            spectral_formant: params.spectral_formant.unmodulated_normalized_value(),
            spectral_placement: params.spectral_placement.unmodulated_normalized_value(),
            filter_tight_enable: params.filter_tight_enable.unmodulated_normalized_value(),
            unison_voices: params.unison_voices.unmodulated_normalized_value(),
            unison_detune: params.unison_detune.unmodulated_normalized_value(),
            unison_spread: params.unison_spread.unmodulated_normalized_value(),
            glide_mode: params.glide_mode.unmodulated_normalized_value(),
            glide_time_ms: params.glide_time_ms.unmodulated_normalized_value(),
            lfo1_rate: params.lfo1_rate.unmodulated_normalized_value(),
            lfo1_attack: params.lfo1_attack.unmodulated_normalized_value(),
            lfo1_shape: params.lfo1_shape.unmodulated_normalized_value(),
            lfo2_rate: params.lfo2_rate.unmodulated_normalized_value(),
            lfo2_attack: params.lfo2_attack.unmodulated_normalized_value(),
            lfo2_shape: params.lfo2_shape.unmodulated_normalized_value(),
            mod1_source: params.mod1_source.unmodulated_normalized_value(),
            mod1_target: params.mod1_target.unmodulated_normalized_value(),
            mod1_amount: params.mod1_amount.unmodulated_normalized_value(),
            mod1_smooth_ms: params.mod1_smooth_ms.unmodulated_normalized_value(),
            mod2_source: params.mod2_source.unmodulated_normalized_value(),
            mod2_target: params.mod2_target.unmodulated_normalized_value(),
            mod2_amount: params.mod2_amount.unmodulated_normalized_value(),
            mod2_smooth_ms: params.mod2_smooth_ms.unmodulated_normalized_value(),
            mod3_source: params.mod3_source.unmodulated_normalized_value(),
            mod3_target: params.mod3_target.unmodulated_normalized_value(),
            mod3_amount: params.mod3_amount.unmodulated_normalized_value(),
            mod3_smooth_ms: params.mod3_smooth_ms.unmodulated_normalized_value(),
            mod4_source: params.mod4_source.unmodulated_normalized_value(),
            mod4_target: params.mod4_target.unmodulated_normalized_value(),
            mod4_amount: params.mod4_amount.unmodulated_normalized_value(),
            mod4_smooth_ms: params.mod4_smooth_ms.unmodulated_normalized_value(),
            mod5_source: params.mod5_source.unmodulated_normalized_value(),
            mod5_target: params.mod5_target.unmodulated_normalized_value(),
            mod5_amount: params.mod5_amount.unmodulated_normalized_value(),
            mod5_smooth_ms: params.mod5_smooth_ms.unmodulated_normalized_value(),
            mod6_source: params.mod6_source.unmodulated_normalized_value(),
            mod6_target: params.mod6_target.unmodulated_normalized_value(),
            mod6_amount: params.mod6_amount.unmodulated_normalized_value(),
            mod6_smooth_ms: params.mod6_smooth_ms.unmodulated_normalized_value(),
            seq_enable: params.seq_enable.unmodulated_normalized_value(),
            seq_rate: params.seq_rate.unmodulated_normalized_value(),
            seq_gate_amount: params.seq_gate_amount.unmodulated_normalized_value(),
            seq_cut_amount: params.seq_cut_amount.unmodulated_normalized_value(),
            seq_res_amount: params.seq_res_amount.unmodulated_normalized_value(),
            seq_wt_amount: params.seq_wt_amount.unmodulated_normalized_value(),
            seq_dist_amount: params.seq_dist_amount.unmodulated_normalized_value(),
            seq_fm_amount: params.seq_fm_amount.unmodulated_normalized_value(),
            seq_steps: (0..SEQ_LANE_COUNT)
                .map(|lane| {
                    std::array::from_fn(|step| {
                        params.seq_lanes[lane].steps[step]
                            .value
                            .unmodulated_normalized_value()
                    })
                })
                .collect(),
            amp_decay_ms: params.amp_decay_ms.unmodulated_normalized_value(),
            amp_decay2_ms: params.amp_decay2_ms.unmodulated_normalized_value(),
            amp_decay2_level: params.amp_decay2_level.unmodulated_normalized_value(),
            amp_sustain_level: params.amp_sustain_level.unmodulated_normalized_value(),
            filter_type: params.filter_type.unmodulated_normalized_value(),
            filter_cut: params.filter_cut.unmodulated_normalized_value(),
            filter_res: params.filter_res.unmodulated_normalized_value(),
            filter_amount: params.filter_amount.unmodulated_normalized_value(),
            filter_cut_attack_ms: params.filter_cut_attack_ms.unmodulated_normalized_value(),
            filter_cut_hold_ms: params.filter_cut_hold_ms.unmodulated_normalized_value(),
            filter_cut_decay_ms: params.filter_cut_decay_ms.unmodulated_normalized_value(),
            filter_cut_decay2_ms: params.filter_cut_decay2_ms.unmodulated_normalized_value(),
            filter_cut_decay2_level: params.filter_cut_decay2_level.unmodulated_normalized_value(),
            filter_cut_sustain_ms: params.filter_cut_sustain_ms.unmodulated_normalized_value(),
            filter_cut_release_ms: params.filter_cut_release_ms.unmodulated_normalized_value(),
            filter_res_attack_ms: params.filter_res_attack_ms.unmodulated_normalized_value(),
            filter_res_hold_ms: params.filter_res_hold_ms.unmodulated_normalized_value(),
            filter_res_decay_ms: params.filter_res_decay_ms.unmodulated_normalized_value(),
            filter_res_decay2_ms: params.filter_res_decay2_ms.unmodulated_normalized_value(),
            filter_res_decay2_level: params.filter_res_decay2_level.unmodulated_normalized_value(),
            filter_res_sustain_ms: params.filter_res_sustain_ms.unmodulated_normalized_value(),
            filter_res_release_ms: params.filter_res_release_ms.unmodulated_normalized_value(),
            amp_envelope_level: params.amp_envelope_level.unmodulated_normalized_value(),
            filter_cut_envelope_level: params.filter_cut_envelope_level.unmodulated_normalized_value(),
            filter_res_envelope_level: params.filter_res_envelope_level.unmodulated_normalized_value(),
            fm_enable: params.fm_enable.unmodulated_normalized_value(),
            fm_source: params.fm_source.unmodulated_normalized_value(),
            fm_target: params.fm_target.unmodulated_normalized_value(),
            fm_amount: params.fm_amount.unmodulated_normalized_value(),
            fm_ratio: params.fm_ratio.unmodulated_normalized_value(),
            fm_feedback: params.fm_feedback.unmodulated_normalized_value(),
            fm_env_attack_ms: params.fm_env_attack_ms.unmodulated_normalized_value(),
            fm_env_hold_ms: params.fm_env_hold_ms.unmodulated_normalized_value(),
            fm_env_decay_ms: params.fm_env_decay_ms.unmodulated_normalized_value(),
            fm_env_decay2_ms: params.fm_env_decay2_ms.unmodulated_normalized_value(),
            fm_env_decay2_level: params.fm_env_decay2_level.unmodulated_normalized_value(),
            fm_env_sustain_level: params.fm_env_sustain_level.unmodulated_normalized_value(),
            fm_env_release_ms: params.fm_env_release_ms.unmodulated_normalized_value(),
            fm_env_amount: params.fm_env_amount.unmodulated_normalized_value(),
            vibrato_attack: params.vibrato_attack.unmodulated_normalized_value(),
            vibrato_intensity: params.vibrato_intensity.unmodulated_normalized_value(),
            vibrato_rate: params.vibrato_rate.unmodulated_normalized_value(),
            tremolo_attack: params.tremolo_attack.unmodulated_normalized_value(),
            tremolo_intensity: params.tremolo_intensity.unmodulated_normalized_value(),
            tremolo_rate: params.tremolo_rate.unmodulated_normalized_value(),
            vibrato_shape: params.vibrato_shape.unmodulated_normalized_value(),
            tremolo_shape: params.tremolo_shape.unmodulated_normalized_value(),
            filter_cut_env_polarity: params.filter_cut_env_polarity.unmodulated_normalized_value(),
            filter_res_env_polarity: params.filter_res_env_polarity.unmodulated_normalized_value(),
            filter_cut_tension: params.filter_cut_tension.unmodulated_normalized_value(),
            filter_res_tension: params.filter_res_tension.unmodulated_normalized_value(),
            cutoff_lfo_attack: params.cutoff_lfo_attack.unmodulated_normalized_value(),
            res_lfo_attack: params.res_lfo_attack.unmodulated_normalized_value(),
            pan_lfo_attack: params.pan_lfo_attack.unmodulated_normalized_value(),
            cutoff_lfo_intensity: params.cutoff_lfo_intensity.unmodulated_normalized_value(),
            cutoff_lfo_rate: params.cutoff_lfo_rate.unmodulated_normalized_value(),
            cutoff_lfo_shape: params.cutoff_lfo_shape.unmodulated_normalized_value(),
            res_lfo_intensity: params.res_lfo_intensity.unmodulated_normalized_value(),
            res_lfo_rate: params.res_lfo_rate.unmodulated_normalized_value(),
            res_lfo_shape: params.res_lfo_shape.unmodulated_normalized_value(),
            pan_lfo_intensity: params.pan_lfo_intensity.unmodulated_normalized_value(),
            pan_lfo_rate: params.pan_lfo_rate.unmodulated_normalized_value(),
            pan_lfo_shape: params.pan_lfo_shape.unmodulated_normalized_value(),
            chorus_enable: params.chorus_enable.unmodulated_normalized_value(),
            chorus_rate: params.chorus_rate.unmodulated_normalized_value(),
            chorus_depth: params.chorus_depth.unmodulated_normalized_value(),
            chorus_mix: params.chorus_mix.unmodulated_normalized_value(),
            delay_enable: params.delay_enable.unmodulated_normalized_value(),
            delay_time_ms: params.delay_time_ms.unmodulated_normalized_value(),
            delay_feedback: params.delay_feedback.unmodulated_normalized_value(),
            delay_mix: params.delay_mix.unmodulated_normalized_value(),
            reverb_enable: params.reverb_enable.unmodulated_normalized_value(),
            reverb_size: params.reverb_size.unmodulated_normalized_value(),
            reverb_damp: params.reverb_damp.unmodulated_normalized_value(),
            reverb_diffusion: params.reverb_diffusion.unmodulated_normalized_value(),
            reverb_shimmer: params.reverb_shimmer.unmodulated_normalized_value(),
            reverb_mix: params.reverb_mix.unmodulated_normalized_value(),
            dist_enable: params.dist_enable.unmodulated_normalized_value(),
            dist_drive: params.dist_drive.unmodulated_normalized_value(),
            dist_tone: params.dist_tone.unmodulated_normalized_value(),
            dist_magic: params.dist_magic.unmodulated_normalized_value(),
            dist_mix: params.dist_mix.unmodulated_normalized_value(),
            dist_env_attack_ms: params.dist_env_attack_ms.unmodulated_normalized_value(),
            dist_env_hold_ms: params.dist_env_hold_ms.unmodulated_normalized_value(),
            dist_env_decay_ms: params.dist_env_decay_ms.unmodulated_normalized_value(),
            dist_env_decay2_ms: params.dist_env_decay2_ms.unmodulated_normalized_value(),
            dist_env_decay2_level: params.dist_env_decay2_level.unmodulated_normalized_value(),
            dist_env_sustain_level: params.dist_env_sustain_level.unmodulated_normalized_value(),
            dist_env_release_ms: params.dist_env_release_ms.unmodulated_normalized_value(),
            dist_env_amount: params.dist_env_amount.unmodulated_normalized_value(),
            eq_enable: params.eq_enable.unmodulated_normalized_value(),
            eq_low_gain: params.eq_low_gain.unmodulated_normalized_value(),
            eq_mid_gain: params.eq_mid_gain.unmodulated_normalized_value(),
            eq_mid_freq: params.eq_mid_freq.unmodulated_normalized_value(),
            eq_mid_q: params.eq_mid_q.unmodulated_normalized_value(),
            eq_high_gain: params.eq_high_gain.unmodulated_normalized_value(),
            eq_mix: params.eq_mix.unmodulated_normalized_value(),
            output_sat_enable: params.output_sat_enable.unmodulated_normalized_value(),
            output_sat_type: params.output_sat_type.unmodulated_normalized_value(),
            output_sat_drive: params.output_sat_drive.unmodulated_normalized_value(),
            output_sat_mix: params.output_sat_mix.unmodulated_normalized_value(),
            multi_filter_enable: params.multi_filter_enable.unmodulated_normalized_value(),
            multi_filter_routing: params.multi_filter_routing.unmodulated_normalized_value(),
            multi_filter_morph: params.multi_filter_morph.unmodulated_normalized_value(),
            multi_filter_parallel_ab: params.multi_filter_parallel_ab.unmodulated_normalized_value(),
            multi_filter_parallel_c: params.multi_filter_parallel_c.unmodulated_normalized_value(),
            multi_filter_a_type: params.multi_filter_a_type.unmodulated_normalized_value(),
            multi_filter_a_cut: params.multi_filter_a_cut.unmodulated_normalized_value(),
            multi_filter_a_res: params.multi_filter_a_res.unmodulated_normalized_value(),
            multi_filter_a_amt: params.multi_filter_a_amt.unmodulated_normalized_value(),
            multi_filter_b_type: params.multi_filter_b_type.unmodulated_normalized_value(),
            multi_filter_b_cut: params.multi_filter_b_cut.unmodulated_normalized_value(),
            multi_filter_b_res: params.multi_filter_b_res.unmodulated_normalized_value(),
            multi_filter_b_amt: params.multi_filter_b_amt.unmodulated_normalized_value(),
            multi_filter_c_type: params.multi_filter_c_type.unmodulated_normalized_value(),
            multi_filter_c_cut: params.multi_filter_c_cut.unmodulated_normalized_value(),
            multi_filter_c_res: params.multi_filter_c_res.unmodulated_normalized_value(),
            multi_filter_c_amt: params.multi_filter_c_amt.unmodulated_normalized_value(),
            limiter_enable: params.limiter_enable.unmodulated_normalized_value(),
            limiter_threshold: params.limiter_threshold.unmodulated_normalized_value(),
            limiter_release: params.limiter_release.unmodulated_normalized_value(),
        }
    }

    fn apply(&self, cx: &mut EventContext, params: &SubSynthParams) {
        apply_param(cx, &params.gain, self.gain);
        apply_param(cx, &params.amp_attack_ms, self.amp_attack_ms);
        apply_param(cx, &params.amp_hold_ms, self.amp_hold_ms);
        apply_param(cx, &params.amp_release_ms, self.amp_release_ms);
        apply_param(cx, &params.amp_tension, self.amp_tension);
        apply_param(cx, &params.waveform, self.waveform);
        apply_param(cx, &params.osc_routing, self.osc_routing);
        apply_param(cx, &params.osc_blend, self.osc_blend);
        apply_param(cx, &params.wavetable_position, self.wavetable_position);
        apply_param(cx, &params.wavetable_distortion, self.wavetable_distortion);
        apply_param(cx, &params.classic_drive, self.classic_drive);
        apply_param(cx, &params.custom_wavetable_enable, self.custom_wavetable_enable);
        apply_param(cx, &params.analog_enable, self.analog_enable);
        apply_param(cx, &params.analog_drive, self.analog_drive);
        apply_param(cx, &params.analog_noise, self.analog_noise);
        apply_param(cx, &params.analog_drift, self.analog_drift);
        apply_param(cx, &params.sub_level, self.sub_level);
        apply_param(cx, &params.classic_level, self.classic_level);
        apply_param(cx, &params.wavetable_level, self.wavetable_level);
        apply_param(cx, &params.noise_level, self.noise_level);
        apply_param(cx, &params.classic_send, self.classic_send);
        apply_param(cx, &params.wavetable_send, self.wavetable_send);
        apply_param(cx, &params.sub_send, self.sub_send);
        apply_param(cx, &params.noise_send, self.noise_send);
        apply_param(cx, &params.ring_mod_send, self.ring_mod_send);
        apply_param(cx, &params.fx_bus_mix, self.fx_bus_mix);
        apply_param(cx, &params.ring_mod_enable, self.ring_mod_enable);
        apply_param(cx, &params.ring_mod_source, self.ring_mod_source);
        apply_param(cx, &params.ring_mod_freq, self.ring_mod_freq);
        apply_param(cx, &params.ring_mod_mix, self.ring_mod_mix);
        apply_param(cx, &params.ring_mod_level, self.ring_mod_level);
        apply_param(cx, &params.ring_mod_placement, self.ring_mod_placement);
        apply_param(cx, &params.sizzle_osc_enable, self.sizzle_osc_enable);
        apply_param(cx, &params.sizzle_wt_enable, self.sizzle_wt_enable);
        apply_param(cx, &params.sizzle_dist_enable, self.sizzle_dist_enable);
        apply_param(cx, &params.sizzle_cutoff, self.sizzle_cutoff);
        apply_param(cx, &params.spectral_enable, self.spectral_enable);
        apply_param(cx, &params.spectral_amount, self.spectral_amount);
        apply_param(cx, &params.spectral_tilt, self.spectral_tilt);
        apply_param(cx, &params.spectral_formant, self.spectral_formant);
        apply_param(cx, &params.spectral_placement, self.spectral_placement);
        apply_param(cx, &params.filter_tight_enable, self.filter_tight_enable);
        apply_param(cx, &params.unison_voices, self.unison_voices);
        apply_param(cx, &params.unison_detune, self.unison_detune);
        apply_param(cx, &params.unison_spread, self.unison_spread);
        apply_param(cx, &params.glide_mode, self.glide_mode);
        apply_param(cx, &params.glide_time_ms, self.glide_time_ms);
        apply_param(cx, &params.lfo1_rate, self.lfo1_rate);
        apply_param(cx, &params.lfo1_attack, self.lfo1_attack);
        apply_param(cx, &params.lfo1_shape, self.lfo1_shape);
        apply_param(cx, &params.lfo2_rate, self.lfo2_rate);
        apply_param(cx, &params.lfo2_attack, self.lfo2_attack);
        apply_param(cx, &params.lfo2_shape, self.lfo2_shape);
        apply_param(cx, &params.mod1_source, self.mod1_source);
        apply_param(cx, &params.mod1_target, self.mod1_target);
        apply_param(cx, &params.mod1_amount, self.mod1_amount);
        apply_param(cx, &params.mod1_smooth_ms, self.mod1_smooth_ms);
        apply_param(cx, &params.mod2_source, self.mod2_source);
        apply_param(cx, &params.mod2_target, self.mod2_target);
        apply_param(cx, &params.mod2_amount, self.mod2_amount);
        apply_param(cx, &params.mod2_smooth_ms, self.mod2_smooth_ms);
        apply_param(cx, &params.mod3_source, self.mod3_source);
        apply_param(cx, &params.mod3_target, self.mod3_target);
        apply_param(cx, &params.mod3_amount, self.mod3_amount);
        apply_param(cx, &params.mod3_smooth_ms, self.mod3_smooth_ms);
        apply_param(cx, &params.mod4_source, self.mod4_source);
        apply_param(cx, &params.mod4_target, self.mod4_target);
        apply_param(cx, &params.mod4_amount, self.mod4_amount);
        apply_param(cx, &params.mod4_smooth_ms, self.mod4_smooth_ms);
        apply_param(cx, &params.mod5_source, self.mod5_source);
        apply_param(cx, &params.mod5_target, self.mod5_target);
        apply_param(cx, &params.mod5_amount, self.mod5_amount);
        apply_param(cx, &params.mod5_smooth_ms, self.mod5_smooth_ms);
        apply_param(cx, &params.mod6_source, self.mod6_source);
        apply_param(cx, &params.mod6_target, self.mod6_target);
        apply_param(cx, &params.mod6_amount, self.mod6_amount);
        apply_param(cx, &params.mod6_smooth_ms, self.mod6_smooth_ms);
        apply_param(cx, &params.seq_enable, self.seq_enable);
        apply_param(cx, &params.seq_rate, self.seq_rate);
        apply_param(cx, &params.seq_gate_amount, self.seq_gate_amount);
        apply_param(cx, &params.seq_cut_amount, self.seq_cut_amount);
        apply_param(cx, &params.seq_res_amount, self.seq_res_amount);
        apply_param(cx, &params.seq_wt_amount, self.seq_wt_amount);
        apply_param(cx, &params.seq_dist_amount, self.seq_dist_amount);
        apply_param(cx, &params.seq_fm_amount, self.seq_fm_amount);
        for lane in 0..SEQ_LANE_COUNT {
            for step in 0..SEQ_STEP_COUNT {
                let step_value = self
                    .seq_steps
                    .get(lane)
                    .map(|lane_steps| lane_steps[step])
                    .unwrap_or(0.0);
                apply_param(
                    cx,
                    &params.seq_lanes[lane].steps[step].value,
                    step_value,
                );
            }
        }
        apply_param(cx, &params.amp_decay_ms, self.amp_decay_ms);
        apply_param(cx, &params.amp_decay2_ms, self.amp_decay2_ms);
        apply_param(cx, &params.amp_decay2_level, self.amp_decay2_level);
        apply_param(cx, &params.amp_sustain_level, self.amp_sustain_level);
        apply_param(cx, &params.filter_type, self.filter_type);
        apply_param(cx, &params.filter_cut, self.filter_cut);
        apply_param(cx, &params.filter_res, self.filter_res);
        apply_param(cx, &params.filter_amount, self.filter_amount);
        apply_param(cx, &params.filter_cut_attack_ms, self.filter_cut_attack_ms);
        apply_param(cx, &params.filter_cut_hold_ms, self.filter_cut_hold_ms);
        apply_param(cx, &params.filter_cut_decay_ms, self.filter_cut_decay_ms);
        apply_param(cx, &params.filter_cut_decay2_ms, self.filter_cut_decay2_ms);
        apply_param(cx, &params.filter_cut_decay2_level, self.filter_cut_decay2_level);
        apply_param(cx, &params.filter_cut_sustain_ms, self.filter_cut_sustain_ms);
        apply_param(cx, &params.filter_cut_release_ms, self.filter_cut_release_ms);
        apply_param(cx, &params.filter_res_attack_ms, self.filter_res_attack_ms);
        apply_param(cx, &params.filter_res_hold_ms, self.filter_res_hold_ms);
        apply_param(cx, &params.filter_res_decay_ms, self.filter_res_decay_ms);
        apply_param(cx, &params.filter_res_decay2_ms, self.filter_res_decay2_ms);
        apply_param(cx, &params.filter_res_decay2_level, self.filter_res_decay2_level);
        apply_param(cx, &params.filter_res_sustain_ms, self.filter_res_sustain_ms);
        apply_param(cx, &params.filter_res_release_ms, self.filter_res_release_ms);
        apply_param(cx, &params.amp_envelope_level, self.amp_envelope_level);
        apply_param(cx, &params.filter_cut_envelope_level, self.filter_cut_envelope_level);
        apply_param(cx, &params.filter_res_envelope_level, self.filter_res_envelope_level);
        apply_param(cx, &params.fm_enable, self.fm_enable);
        apply_param(cx, &params.fm_source, self.fm_source);
        apply_param(cx, &params.fm_target, self.fm_target);
        apply_param(cx, &params.fm_amount, self.fm_amount);
        apply_param(cx, &params.fm_ratio, self.fm_ratio);
        apply_param(cx, &params.fm_feedback, self.fm_feedback);
        apply_param(cx, &params.fm_env_attack_ms, self.fm_env_attack_ms);
        apply_param(cx, &params.fm_env_hold_ms, self.fm_env_hold_ms);
        apply_param(cx, &params.fm_env_decay_ms, self.fm_env_decay_ms);
        apply_param(cx, &params.fm_env_decay2_ms, self.fm_env_decay2_ms);
        apply_param(cx, &params.fm_env_decay2_level, self.fm_env_decay2_level);
        apply_param(cx, &params.fm_env_sustain_level, self.fm_env_sustain_level);
        apply_param(cx, &params.fm_env_release_ms, self.fm_env_release_ms);
        apply_param(cx, &params.fm_env_amount, self.fm_env_amount);
        apply_param(cx, &params.vibrato_attack, self.vibrato_attack);
        apply_param(cx, &params.vibrato_intensity, self.vibrato_intensity);
        apply_param(cx, &params.vibrato_rate, self.vibrato_rate);
        apply_param(cx, &params.tremolo_attack, self.tremolo_attack);
        apply_param(cx, &params.tremolo_intensity, self.tremolo_intensity);
        apply_param(cx, &params.tremolo_rate, self.tremolo_rate);
        apply_param(cx, &params.vibrato_shape, self.vibrato_shape);
        apply_param(cx, &params.tremolo_shape, self.tremolo_shape);
        apply_param(cx, &params.filter_cut_env_polarity, self.filter_cut_env_polarity);
        apply_param(cx, &params.filter_res_env_polarity, self.filter_res_env_polarity);
        apply_param(cx, &params.filter_cut_tension, self.filter_cut_tension);
        apply_param(cx, &params.filter_res_tension, self.filter_res_tension);
        apply_param(cx, &params.cutoff_lfo_attack, self.cutoff_lfo_attack);
        apply_param(cx, &params.res_lfo_attack, self.res_lfo_attack);
        apply_param(cx, &params.pan_lfo_attack, self.pan_lfo_attack);
        apply_param(cx, &params.cutoff_lfo_intensity, self.cutoff_lfo_intensity);
        apply_param(cx, &params.cutoff_lfo_rate, self.cutoff_lfo_rate);
        apply_param(cx, &params.cutoff_lfo_shape, self.cutoff_lfo_shape);
        apply_param(cx, &params.res_lfo_intensity, self.res_lfo_intensity);
        apply_param(cx, &params.res_lfo_rate, self.res_lfo_rate);
        apply_param(cx, &params.res_lfo_shape, self.res_lfo_shape);
        apply_param(cx, &params.pan_lfo_intensity, self.pan_lfo_intensity);
        apply_param(cx, &params.pan_lfo_rate, self.pan_lfo_rate);
        apply_param(cx, &params.pan_lfo_shape, self.pan_lfo_shape);
        apply_param(cx, &params.chorus_enable, self.chorus_enable);
        apply_param(cx, &params.chorus_rate, self.chorus_rate);
        apply_param(cx, &params.chorus_depth, self.chorus_depth);
        apply_param(cx, &params.chorus_mix, self.chorus_mix);
        apply_param(cx, &params.delay_enable, self.delay_enable);
        apply_param(cx, &params.delay_time_ms, self.delay_time_ms);
        apply_param(cx, &params.delay_feedback, self.delay_feedback);
        apply_param(cx, &params.delay_mix, self.delay_mix);
        apply_param(cx, &params.reverb_enable, self.reverb_enable);
        apply_param(cx, &params.reverb_size, self.reverb_size);
        apply_param(cx, &params.reverb_damp, self.reverb_damp);
        apply_param(cx, &params.reverb_diffusion, self.reverb_diffusion);
        apply_param(cx, &params.reverb_shimmer, self.reverb_shimmer);
        apply_param(cx, &params.reverb_mix, self.reverb_mix);
        apply_param(cx, &params.output_sat_enable, self.output_sat_enable);
        apply_param(cx, &params.output_sat_type, self.output_sat_type);
        apply_param(cx, &params.output_sat_drive, self.output_sat_drive);
        apply_param(cx, &params.output_sat_mix, self.output_sat_mix);
        apply_param(cx, &params.dist_env_attack_ms, self.dist_env_attack_ms);
        apply_param(cx, &params.dist_env_hold_ms, self.dist_env_hold_ms);
        apply_param(cx, &params.dist_env_decay_ms, self.dist_env_decay_ms);
        apply_param(cx, &params.dist_env_decay2_ms, self.dist_env_decay2_ms);
        apply_param(cx, &params.dist_env_decay2_level, self.dist_env_decay2_level);
        apply_param(cx, &params.dist_env_sustain_level, self.dist_env_sustain_level);
        apply_param(cx, &params.dist_env_release_ms, self.dist_env_release_ms);
        apply_param(cx, &params.dist_env_amount, self.dist_env_amount);
        apply_param(cx, &params.multi_filter_enable, self.multi_filter_enable);
        apply_param(cx, &params.multi_filter_routing, self.multi_filter_routing);
        apply_param(cx, &params.multi_filter_morph, self.multi_filter_morph);
        apply_param(cx, &params.multi_filter_parallel_ab, self.multi_filter_parallel_ab);
        apply_param(cx, &params.multi_filter_parallel_c, self.multi_filter_parallel_c);
        apply_param(cx, &params.multi_filter_a_type, self.multi_filter_a_type);
        apply_param(cx, &params.multi_filter_a_cut, self.multi_filter_a_cut);
        apply_param(cx, &params.multi_filter_a_res, self.multi_filter_a_res);
        apply_param(cx, &params.multi_filter_a_amt, self.multi_filter_a_amt);
        apply_param(cx, &params.multi_filter_b_type, self.multi_filter_b_type);
        apply_param(cx, &params.multi_filter_b_cut, self.multi_filter_b_cut);
        apply_param(cx, &params.multi_filter_b_res, self.multi_filter_b_res);
        apply_param(cx, &params.multi_filter_b_amt, self.multi_filter_b_amt);
        apply_param(cx, &params.multi_filter_c_type, self.multi_filter_c_type);
        apply_param(cx, &params.multi_filter_c_cut, self.multi_filter_c_cut);
        apply_param(cx, &params.multi_filter_c_res, self.multi_filter_c_res);
        apply_param(cx, &params.multi_filter_c_amt, self.multi_filter_c_amt);
        apply_param(cx, &params.limiter_enable, self.limiter_enable);
        apply_param(cx, &params.limiter_threshold, self.limiter_threshold);
        apply_param(cx, &params.limiter_release, self.limiter_release);
    }

    pub(crate) fn apply_direct(&self, params: &SubSynthParams) {
        params.gain.set_normalized_value(self.gain);
        params.amp_attack_ms.set_normalized_value(self.amp_attack_ms);
        params.amp_hold_ms.set_normalized_value(self.amp_hold_ms);
        params.amp_release_ms.set_normalized_value(self.amp_release_ms);
        params.amp_tension.set_normalized_value(self.amp_tension);
        params.waveform.set_normalized_value(self.waveform);
        params.osc_routing.set_normalized_value(self.osc_routing);
        params.osc_blend.set_normalized_value(self.osc_blend);
        params.wavetable_position.set_normalized_value(self.wavetable_position);
        params.wavetable_distortion.set_normalized_value(self.wavetable_distortion);
        params.classic_drive.set_normalized_value(self.classic_drive);
        params.custom_wavetable_enable
            .set_normalized_value(self.custom_wavetable_enable);
        params.analog_enable.set_normalized_value(self.analog_enable);
        params.analog_drive.set_normalized_value(self.analog_drive);
        params.analog_noise.set_normalized_value(self.analog_noise);
        params.analog_drift.set_normalized_value(self.analog_drift);
        params.sub_level.set_normalized_value(self.sub_level);
        params.classic_level.set_normalized_value(self.classic_level);
        params.wavetable_level.set_normalized_value(self.wavetable_level);
        params.noise_level.set_normalized_value(self.noise_level);
        params.classic_send.set_normalized_value(self.classic_send);
        params.wavetable_send.set_normalized_value(self.wavetable_send);
        params.sub_send.set_normalized_value(self.sub_send);
        params.noise_send.set_normalized_value(self.noise_send);
        params.ring_mod_send.set_normalized_value(self.ring_mod_send);
        params.fx_bus_mix.set_normalized_value(self.fx_bus_mix);
        params.ring_mod_enable.set_normalized_value(self.ring_mod_enable);
        params.ring_mod_source.set_normalized_value(self.ring_mod_source);
        params.ring_mod_freq.set_normalized_value(self.ring_mod_freq);
        params.ring_mod_mix.set_normalized_value(self.ring_mod_mix);
        params.ring_mod_level.set_normalized_value(self.ring_mod_level);
        params.ring_mod_placement.set_normalized_value(self.ring_mod_placement);
        params.sizzle_osc_enable.set_normalized_value(self.sizzle_osc_enable);
        params.sizzle_wt_enable.set_normalized_value(self.sizzle_wt_enable);
        params.sizzle_dist_enable.set_normalized_value(self.sizzle_dist_enable);
        params.sizzle_cutoff.set_normalized_value(self.sizzle_cutoff);
        params.spectral_enable.set_normalized_value(self.spectral_enable);
        params.spectral_amount.set_normalized_value(self.spectral_amount);
        params.spectral_tilt.set_normalized_value(self.spectral_tilt);
        params.spectral_formant.set_normalized_value(self.spectral_formant);
        params.spectral_placement.set_normalized_value(self.spectral_placement);
        params.filter_tight_enable.set_normalized_value(self.filter_tight_enable);
        params.unison_voices.set_normalized_value(self.unison_voices);
        params.unison_detune.set_normalized_value(self.unison_detune);
        params.unison_spread.set_normalized_value(self.unison_spread);
        params.glide_mode.set_normalized_value(self.glide_mode);
        params.glide_time_ms.set_normalized_value(self.glide_time_ms);
        params.lfo1_rate.set_normalized_value(self.lfo1_rate);
        params.lfo1_attack.set_normalized_value(self.lfo1_attack);
        params.lfo1_shape.set_normalized_value(self.lfo1_shape);
        params.lfo2_rate.set_normalized_value(self.lfo2_rate);
        params.lfo2_attack.set_normalized_value(self.lfo2_attack);
        params.lfo2_shape.set_normalized_value(self.lfo2_shape);
        params.mod1_source.set_normalized_value(self.mod1_source);
        params.mod1_target.set_normalized_value(self.mod1_target);
        params.mod1_amount.set_normalized_value(self.mod1_amount);
        params.mod1_smooth_ms.set_normalized_value(self.mod1_smooth_ms);
        params.mod2_source.set_normalized_value(self.mod2_source);
        params.mod2_target.set_normalized_value(self.mod2_target);
        params.mod2_amount.set_normalized_value(self.mod2_amount);
        params.mod2_smooth_ms.set_normalized_value(self.mod2_smooth_ms);
        params.mod3_source.set_normalized_value(self.mod3_source);
        params.mod3_target.set_normalized_value(self.mod3_target);
        params.mod3_amount.set_normalized_value(self.mod3_amount);
        params.mod3_smooth_ms.set_normalized_value(self.mod3_smooth_ms);
        params.mod4_source.set_normalized_value(self.mod4_source);
        params.mod4_target.set_normalized_value(self.mod4_target);
        params.mod4_amount.set_normalized_value(self.mod4_amount);
        params.mod4_smooth_ms.set_normalized_value(self.mod4_smooth_ms);
        params.mod5_source.set_normalized_value(self.mod5_source);
        params.mod5_target.set_normalized_value(self.mod5_target);
        params.mod5_amount.set_normalized_value(self.mod5_amount);
        params.mod5_smooth_ms.set_normalized_value(self.mod5_smooth_ms);
        params.mod6_source.set_normalized_value(self.mod6_source);
        params.mod6_target.set_normalized_value(self.mod6_target);
        params.mod6_amount.set_normalized_value(self.mod6_amount);
        params.mod6_smooth_ms.set_normalized_value(self.mod6_smooth_ms);
        params.seq_enable.set_normalized_value(self.seq_enable);
        params.seq_rate.set_normalized_value(self.seq_rate);
        params.seq_gate_amount
            .set_normalized_value(self.seq_gate_amount);
        params.seq_cut_amount.set_normalized_value(self.seq_cut_amount);
        params.seq_res_amount.set_normalized_value(self.seq_res_amount);
        params.seq_wt_amount.set_normalized_value(self.seq_wt_amount);
        params.seq_dist_amount.set_normalized_value(self.seq_dist_amount);
        params.seq_fm_amount.set_normalized_value(self.seq_fm_amount);
        for lane in 0..SEQ_LANE_COUNT {
            for step in 0..SEQ_STEP_COUNT {
                let step_value = self
                    .seq_steps
                    .get(lane)
                    .map(|lane_steps| lane_steps[step])
                    .unwrap_or(0.0);
                params.seq_lanes[lane].steps[step]
                    .value
                    .set_normalized_value(step_value);
            }
        }
        params.amp_decay_ms.set_normalized_value(self.amp_decay_ms);
        params.amp_decay2_ms.set_normalized_value(self.amp_decay2_ms);
        params.amp_decay2_level.set_normalized_value(self.amp_decay2_level);
        params.amp_sustain_level
            .set_normalized_value(self.amp_sustain_level);
        params.filter_type.set_normalized_value(self.filter_type);
        params.filter_cut.set_normalized_value(self.filter_cut);
        params.filter_res.set_normalized_value(self.filter_res);
        params.filter_amount.set_normalized_value(self.filter_amount);
        params.filter_cut_attack_ms
            .set_normalized_value(self.filter_cut_attack_ms);
        params.filter_cut_hold_ms
            .set_normalized_value(self.filter_cut_hold_ms);
        params.filter_cut_decay_ms
            .set_normalized_value(self.filter_cut_decay_ms);
        params.filter_cut_decay2_ms
            .set_normalized_value(self.filter_cut_decay2_ms);
        params.filter_cut_decay2_level
            .set_normalized_value(self.filter_cut_decay2_level);
        params.filter_cut_sustain_ms
            .set_normalized_value(self.filter_cut_sustain_ms);
        params.filter_cut_release_ms
            .set_normalized_value(self.filter_cut_release_ms);
        params.filter_res_attack_ms
            .set_normalized_value(self.filter_res_attack_ms);
        params.filter_res_hold_ms
            .set_normalized_value(self.filter_res_hold_ms);
        params.filter_res_decay_ms
            .set_normalized_value(self.filter_res_decay_ms);
        params.filter_res_decay2_ms
            .set_normalized_value(self.filter_res_decay2_ms);
        params.filter_res_decay2_level
            .set_normalized_value(self.filter_res_decay2_level);
        params.filter_res_sustain_ms
            .set_normalized_value(self.filter_res_sustain_ms);
        params.filter_res_release_ms
            .set_normalized_value(self.filter_res_release_ms);
        params.amp_envelope_level
            .set_normalized_value(self.amp_envelope_level);
        params.filter_cut_envelope_level
            .set_normalized_value(self.filter_cut_envelope_level);
        params.filter_res_envelope_level
            .set_normalized_value(self.filter_res_envelope_level);
        params.fm_enable.set_normalized_value(self.fm_enable);
        params.fm_source.set_normalized_value(self.fm_source);
        params.fm_target.set_normalized_value(self.fm_target);
        params.fm_amount.set_normalized_value(self.fm_amount);
        params.fm_ratio.set_normalized_value(self.fm_ratio);
        params.fm_feedback.set_normalized_value(self.fm_feedback);
        params.fm_env_attack_ms
            .set_normalized_value(self.fm_env_attack_ms);
        params.fm_env_hold_ms
            .set_normalized_value(self.fm_env_hold_ms);
        params.fm_env_decay_ms
            .set_normalized_value(self.fm_env_decay_ms);
        params.fm_env_decay2_ms
            .set_normalized_value(self.fm_env_decay2_ms);
        params.fm_env_decay2_level
            .set_normalized_value(self.fm_env_decay2_level);
        params.fm_env_sustain_level
            .set_normalized_value(self.fm_env_sustain_level);
        params.fm_env_release_ms
            .set_normalized_value(self.fm_env_release_ms);
        params.fm_env_amount.set_normalized_value(self.fm_env_amount);
        params.vibrato_attack
            .set_normalized_value(self.vibrato_attack);
        params.vibrato_intensity
            .set_normalized_value(self.vibrato_intensity);
        params.vibrato_rate.set_normalized_value(self.vibrato_rate);
        params.tremolo_attack
            .set_normalized_value(self.tremolo_attack);
        params.tremolo_intensity
            .set_normalized_value(self.tremolo_intensity);
        params.tremolo_rate.set_normalized_value(self.tremolo_rate);
        params.vibrato_shape
            .set_normalized_value(self.vibrato_shape);
        params.tremolo_shape
            .set_normalized_value(self.tremolo_shape);
        params.filter_cut_env_polarity
            .set_normalized_value(self.filter_cut_env_polarity);
        params.filter_res_env_polarity
            .set_normalized_value(self.filter_res_env_polarity);
        params.filter_cut_tension
            .set_normalized_value(self.filter_cut_tension);
        params.filter_res_tension
            .set_normalized_value(self.filter_res_tension);
        params.cutoff_lfo_attack
            .set_normalized_value(self.cutoff_lfo_attack);
        params.res_lfo_attack
            .set_normalized_value(self.res_lfo_attack);
        params.pan_lfo_attack
            .set_normalized_value(self.pan_lfo_attack);
        params.cutoff_lfo_intensity
            .set_normalized_value(self.cutoff_lfo_intensity);
        params.cutoff_lfo_rate
            .set_normalized_value(self.cutoff_lfo_rate);
        params.cutoff_lfo_shape
            .set_normalized_value(self.cutoff_lfo_shape);
        params.res_lfo_intensity
            .set_normalized_value(self.res_lfo_intensity);
        params.res_lfo_rate
            .set_normalized_value(self.res_lfo_rate);
        params.res_lfo_shape
            .set_normalized_value(self.res_lfo_shape);
        params.pan_lfo_intensity
            .set_normalized_value(self.pan_lfo_intensity);
        params.pan_lfo_rate
            .set_normalized_value(self.pan_lfo_rate);
        params.pan_lfo_shape
            .set_normalized_value(self.pan_lfo_shape);
        params.chorus_enable
            .set_normalized_value(self.chorus_enable);
        params.chorus_rate.set_normalized_value(self.chorus_rate);
        params.chorus_depth
            .set_normalized_value(self.chorus_depth);
        params.chorus_mix.set_normalized_value(self.chorus_mix);
        params.delay_enable.set_normalized_value(self.delay_enable);
        params.delay_time_ms
            .set_normalized_value(self.delay_time_ms);
        params.delay_feedback
            .set_normalized_value(self.delay_feedback);
        params.delay_mix.set_normalized_value(self.delay_mix);
        params.reverb_enable.set_normalized_value(self.reverb_enable);
        params.reverb_size.set_normalized_value(self.reverb_size);
        params.reverb_damp.set_normalized_value(self.reverb_damp);
        params.reverb_diffusion
            .set_normalized_value(self.reverb_diffusion);
        params.reverb_shimmer
            .set_normalized_value(self.reverb_shimmer);
        params.reverb_mix.set_normalized_value(self.reverb_mix);
        params.output_sat_enable
            .set_normalized_value(self.output_sat_enable);
        params.output_sat_type
            .set_normalized_value(self.output_sat_type);
        params.output_sat_drive
            .set_normalized_value(self.output_sat_drive);
        params.output_sat_mix
            .set_normalized_value(self.output_sat_mix);
        params.dist_env_attack_ms
            .set_normalized_value(self.dist_env_attack_ms);
        params.dist_env_hold_ms
            .set_normalized_value(self.dist_env_hold_ms);
        params.dist_env_decay_ms
            .set_normalized_value(self.dist_env_decay_ms);
        params.dist_env_decay2_ms
            .set_normalized_value(self.dist_env_decay2_ms);
        params.dist_env_decay2_level
            .set_normalized_value(self.dist_env_decay2_level);
        params.dist_env_sustain_level
            .set_normalized_value(self.dist_env_sustain_level);
        params.dist_env_release_ms
            .set_normalized_value(self.dist_env_release_ms);
        params.dist_env_amount
            .set_normalized_value(self.dist_env_amount);
        params.multi_filter_enable
            .set_normalized_value(self.multi_filter_enable);
        params.multi_filter_routing
            .set_normalized_value(self.multi_filter_routing);
        params.multi_filter_morph
            .set_normalized_value(self.multi_filter_morph);
        params.multi_filter_parallel_ab
            .set_normalized_value(self.multi_filter_parallel_ab);
        params.multi_filter_parallel_c
            .set_normalized_value(self.multi_filter_parallel_c);
        params.multi_filter_a_type
            .set_normalized_value(self.multi_filter_a_type);
        params.multi_filter_a_cut
            .set_normalized_value(self.multi_filter_a_cut);
        params.multi_filter_a_res
            .set_normalized_value(self.multi_filter_a_res);
        params.multi_filter_a_amt
            .set_normalized_value(self.multi_filter_a_amt);
        params.multi_filter_b_type
            .set_normalized_value(self.multi_filter_b_type);
        params.multi_filter_b_cut
            .set_normalized_value(self.multi_filter_b_cut);
        params.multi_filter_b_res
            .set_normalized_value(self.multi_filter_b_res);
        params.multi_filter_b_amt
            .set_normalized_value(self.multi_filter_b_amt);
        params.multi_filter_c_type
            .set_normalized_value(self.multi_filter_c_type);
        params.multi_filter_c_cut
            .set_normalized_value(self.multi_filter_c_cut);
        params.multi_filter_c_res
            .set_normalized_value(self.multi_filter_c_res);
        params.multi_filter_c_amt
            .set_normalized_value(self.multi_filter_c_amt);
        params.limiter_enable
            .set_normalized_value(self.limiter_enable);
        params.limiter_threshold
            .set_normalized_value(self.limiter_threshold);
        params.limiter_release
            .set_normalized_value(self.limiter_release);
    }
}

fn apply_param<P: Param>(cx: &mut EventContext, param: &P, normalized: f32) {
    cx.emit(ParamEvent::BeginSetParameter(param).upcast());
    cx.emit(ParamEvent::SetParameterNormalized(param, normalized).upcast());
    cx.emit(ParamEvent::EndSetParameter(param).upcast());
}

fn factory_presets(params: &SubSynthParams) -> Vec<PresetEntry> {
    let data = factory_preset_data(params);
    let names = factory_preset_names();
    let mut presets = Vec::with_capacity(data.len());
    for (index, preset) in data.into_iter().enumerate() {
        let name = names
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("Preset {}", index + 1));
        presets.push(PresetEntry {
            name,
            data: preset,
            user: false,
        });
    }

    presets
}

fn build_preset_bar(cx: &mut Context) {
    HStack::new(cx, |cx| {
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::PresetPrev),
            |cx| Label::new(cx, "<"),
        );
        Label::new(cx, Data::preset_display)
            .width(Pixels(180.0))
            .height(Pixels(22.0))
            .child_top(Stretch(1.0))
            .child_bottom(Pixels(0.0));
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::PresetNext),
            |cx| Label::new(cx, ">"),
        );
        Element::new(cx).width(Pixels(12.0));
        Label::new(cx, "Preset Name")
            .height(Pixels(18.0))
            .child_top(Stretch(1.0))
            .child_bottom(Pixels(0.0));
        Textbox::new(cx, Data::preset_name)
            .width(Pixels(180.0))
            .height(Pixels(24.0))
            .on_submit(|cx, text, _| cx.emit(UiEvent::PresetNameChanged(text)));
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::PresetLoad),
            |cx| Label::new(cx, "Load"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::PresetSave),
            |cx| Label::new(cx, "Save"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::PresetRefresh),
            |cx| Label::new(cx, "Refresh"),
        );
    })
    .col_between(Pixels(8.0))
    .row_between(Pixels(6.0))
    .height(Pixels(30.0))
    .width(Stretch(1.0))
    .child_top(Pixels(6.0));
}

fn build_tab_bar(cx: &mut Context) {
    HStack::new(cx, |cx| {
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(0)),
            |cx| Label::new(cx, "Osc + WT"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(1)),
            |cx| Label::new(cx, "Mixer Matrix"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(2)),
            |cx| Label::new(cx, "Filter + Env"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(3)),
            |cx| Label::new(cx, "Mod Matrix"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(4)),
            |cx| Label::new(cx, "Motion"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(5)),
            |cx| Label::new(cx, "Articulator"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(6)),
            |cx| Label::new(cx, "Sequencer"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(7)),
            |cx| Label::new(cx, "Multi Filter"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(8)),
            |cx| Label::new(cx, "FX"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(9)),
            |cx| Label::new(cx, "Utility"),
        );
    })
    .col_between(Pixels(8.0))
    .row_between(Pixels(6.0))
    .height(Pixels(28.0))
    .child_top(Pixels(6.0));
}

fn build_osc_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            create_label(cx, "Waveform", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.waveform)
                ;
            create_label(cx, "Osc Routing", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.osc_routing)
                ;
            create_label(cx, "Osc Blend", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.osc_blend)
                ;
            create_label(cx, "Wavetable Pos", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.wavetable_position)
                ;
            create_label(cx, "Wavetable Dist", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.wavetable_distortion)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Sub")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.sub_level)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "FM")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.fm_enable)
                .with_label("")
                .width(Pixels(70.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.fm_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.fm_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.fm_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.fm_ratio)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.fm_feedback)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Unison")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.unison_voices)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.unison_detune)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.unison_spread)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Glide")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.glide_mode)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.glide_time_ms)
                ;
        })
        .row_between(Pixels(12.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_mixer_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Levels")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.classic_level);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.wavetable_level);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.sub_level);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.noise_level);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.ring_mod_level);
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "FX Sends")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.classic_send);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.wavetable_send);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.sub_send);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.noise_send);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.ring_mod_send);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.fx_bus_mix);
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Ring Mod")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.ring_mod_enable)
                .with_label("")
                .width(Pixels(80.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.ring_mod_source);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.ring_mod_freq);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.ring_mod_mix);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.ring_mod_placement);
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Spectral")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.spectral_enable)
                .with_label("")
                .width(Pixels(80.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.spectral_amount);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.spectral_tilt);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.spectral_formant);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.spectral_placement);
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Sizzle Guard")
                .height(Pixels(16.0))
                .width(Pixels(110.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.sizzle_osc_enable)
                .with_label("Osc")
                .width(Pixels(80.0))
                .height(Pixels(26.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.sizzle_wt_enable)
                .with_label("WT")
                .width(Pixels(80.0))
                .height(Pixels(26.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.sizzle_dist_enable)
                .with_label("Dist")
                .width(Pixels(80.0))
                .height(Pixels(26.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.sizzle_cutoff);
            ParamButton::new(cx, Data::params.clone(), |params| &params.filter_tight_enable)
                .with_label("Tight")
                .width(Pixels(80.0))
                .height(Pixels(26.0));
        })
        .row_between(Pixels(10.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_env_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            create_label(cx, "Attack", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_attack_ms)
                ;
            create_label(cx, "Hold", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_hold_ms)
                ;
            create_label(cx, "Decay", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_decay_ms)
                ;
            create_label(cx, "Decay 2", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_decay2_ms)
                ;
            create_label(cx, "Decay 2 Level", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_decay2_level)
                ;
            create_label(cx, "Sustain", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_sustain_level)
                ;
            create_label(cx, "Release", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_release_ms)
                ;
            Label::new(cx, "Env Int")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_envelope_level)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            create_label(cx, "Filter Type", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_type)
                ;
            create_label(cx, "Filter Cut", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut)
                ;
            create_label(cx, "Filter Res", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res)
                ;
            create_label(cx, "Filter Amt", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_amount)
                ;
        })
        .row_between(Pixels(12.0));

    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_filter_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Multi Filter")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.multi_filter_enable)
                .with_label("")
                .width(Pixels(90.0))
                .height(Pixels(28.0));
            create_label(cx, "Routing", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_routing)
                ;
            create_label(cx, "Morph", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_morph)
                ;
            create_label(cx, "AB Mix", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_parallel_ab)
                ;
            create_label(cx, "C Mix", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_parallel_c)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Stage A")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            create_label(cx, "Type", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_type)
                ;
            create_label(cx, "Cut", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_cut)
                ;
            create_label(cx, "Res", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_res)
                ;
            create_label(cx, "Amt", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_amt)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Stage B")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            create_label(cx, "Type", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_type)
                ;
            create_label(cx, "Cut", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_cut)
                ;
            create_label(cx, "Res", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_res)
                ;
            create_label(cx, "Amt", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_amt)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Stage C")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            create_label(cx, "Type", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_type)
                ;
            create_label(cx, "Cut", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_cut)
                ;
            create_label(cx, "Res", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_res)
                ;
            create_label(cx, "Amt", 20.0, 70.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_amt)
                ;
        })
        .row_between(Pixels(12.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_lfo_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Vib Int")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.vibrato_intensity)
                ;
            Label::new(cx, "Vib Rate")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.vibrato_rate)
                ;
            Label::new(cx, "Vib Attack")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.vibrato_attack)
                ;
            Label::new(cx, "Vib Shape")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.vibrato_shape)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Trem Int")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.tremolo_intensity)
                ;
            Label::new(cx, "Trem Rate")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.tremolo_rate)
                ;
            Label::new(cx, "Trem Attack")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.tremolo_attack)
                ;
            Label::new(cx, "Trem Shape")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.tremolo_shape)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Cut LFO")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.cutoff_lfo_intensity)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.cutoff_lfo_rate)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.cutoff_lfo_shape)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.cutoff_lfo_attack)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Res LFO")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.res_lfo_intensity)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.res_lfo_rate)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.res_lfo_shape)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.res_lfo_attack)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Pan LFO")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.pan_lfo_intensity)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.pan_lfo_rate)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.pan_lfo_shape)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.pan_lfo_attack)
                ;
        })
        .row_between(Pixels(12.0));

    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_seq_lane(cx: &mut Context, label: &str, lane_idx: usize) {
    HStack::new(cx, |cx| {
        Label::new(cx, label)
            .height(Pixels(18.0))
            .width(Pixels(70.0));
        for step in 0..SEQ_STEP_COUNT {
            let step = step;
            let lane_idx = lane_idx;
            SeqStepBox::new(cx, Data::params.clone(), move |params| {
                &params.seq_lanes[lane_idx].steps[step].value
            })
            .width(Pixels(14.0))
            .height(Pixels(32.0));
        }
    })
    .col_between(Pixels(2.0))
    .row_between(Pixels(2.0));
}

fn build_seq_tab(cx: &mut Context) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                Label::new(cx, "Sequencer")
                    .height(Pixels(16.0))
                    .width(Pixels(90.0));
                ParamButton::new(cx, Data::params.clone(), |params| &params.seq_enable)
                    .with_label("")
                    .width(Pixels(90.0))
                    .height(Pixels(30.0));
                Label::new(cx, "Rate")
                    .height(Pixels(18.0))
                    .width(Pixels(70.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Pixels(0.0));
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_rate)
                    ;
                HStack::new(cx, |cx| {
                    Button::new(
                        cx,
                        |cx| cx.emit(UiEvent::SeqPresetPrev),
                        |cx| Label::new(cx, "<"),
                    );
                    Label::new(cx, Data::seq_preset_display)
                        .width(Pixels(120.0))
                        .height(Pixels(22.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    Button::new(
                        cx,
                        |cx| cx.emit(UiEvent::SeqPresetNext),
                        |cx| Label::new(cx, ">"),
                    );
                })
                .col_between(Pixels(6.0));
                HStack::new(cx, |cx| {
                    Button::new(
                        cx,
                        |cx| cx.emit(UiEvent::SeqPresetReset),
                        |cx| Label::new(cx, "Reset"),
                    );
                    Button::new(
                        cx,
                        |cx| cx.emit(UiEvent::SeqPresetRandom),
                        |cx| Label::new(cx, "Random"),
                    );
                })
                .col_between(Pixels(6.0));
            })
            .row_between(Pixels(10.0));

            VStack::new(cx, |cx| {
                Label::new(cx, "Sends")
                    .height(Pixels(16.0))
                    .width(Pixels(90.0));
                create_label(cx, "Gate", 20.0, 90.0, 1.0, 0.0);
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_gate_amount)
                    ;
                create_label(cx, "Cutoff", 20.0, 90.0, 1.0, 0.0);
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_cut_amount)
                    ;
                create_label(cx, "Res", 20.0, 90.0, 1.0, 0.0);
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_res_amount)
                    ;
                create_label(cx, "Wavetable", 20.0, 90.0, 1.0, 0.0);
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_wt_amount)
                    ;
                create_label(cx, "Dist", 20.0, 90.0, 1.0, 0.0);
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_dist_amount)
                    ;
                create_label(cx, "FM", 20.0, 90.0, 1.0, 0.0);
                ParamSlider::new(cx, Data::params.clone(), |params| &params.seq_fm_amount)
                    ;
            })
            .row_between(Pixels(8.0));
        })
        .col_between(Pixels(20.0))
        .row_between(Pixels(8.0));

        VStack::new(cx, |cx| {
            build_seq_lane(cx, "Gate", 0);
            build_seq_lane(cx, "Cutoff", 1);
            build_seq_lane(cx, "Res", 2);
            build_seq_lane(cx, "WT", 3);
            build_seq_lane(cx, "Dist", 4);
            build_seq_lane(cx, "FM", 5);
        })
        .row_between(Pixels(6.0))
        .child_top(Pixels(10.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn labeled_knob<L, Params, P, FMap>(
    cx: &mut Context,
    label: &str,
    params: L,
    params_to_param: FMap,
) where
    L: Lens<Target = Params> + Clone,
    Params: 'static,
    P: Param + 'static,
    FMap: Fn(&Params) -> &P + Copy + 'static,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label)
            .height(Pixels(18.0))
            .width(Pixels(60.0))
            .child_top(Stretch(1.0))
            .child_bottom(Pixels(0.0));
        ParamKnob::new(cx, params, params_to_param)
            .width(Pixels(40.0))
            .height(Pixels(40.0));
    })
    .row_between(Pixels(4.0));
}

fn build_articulator_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "FM Envelope")
                .height(Pixels(16.0))
                .width(Pixels(110.0));
            EnvelopeDisplay::new(
                cx,
                Data::params.clone(),
                |params| &params.fm_env_attack_ms,
                |params| &params.fm_env_hold_ms,
                |params| &params.fm_env_decay_ms,
                |params| &params.fm_env_decay2_ms,
                |params| &params.fm_env_decay2_level,
                |params| &params.fm_env_sustain_level,
                |params| &params.fm_env_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.fm_env_attack_ms);
                    labeled_knob(cx, "Hold", Data::params.clone(), |params| &params.fm_env_hold_ms);
                    labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.fm_env_decay_ms);
                    labeled_knob(cx, "D2", Data::params.clone(), |params| &params.fm_env_decay2_ms);
                })
                .col_between(Pixels(6.0));
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "D2Lvl", Data::params.clone(), |params| &params.fm_env_decay2_level);
                    labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.fm_env_sustain_level);
                    labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.fm_env_release_ms);
                    labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.fm_env_amount);
                })
                .col_between(Pixels(6.0));
            })
            .row_between(Pixels(6.0));

            Label::new(cx, "Filter Cut Env")
                .height(Pixels(16.0))
                .width(Pixels(120.0))
                .child_top(Pixels(8.0));
            EnvelopeDisplay::new(
                cx,
                Data::params.clone(),
                |params| &params.filter_cut_attack_ms,
                |params| &params.filter_cut_hold_ms,
                |params| &params.filter_cut_decay_ms,
                |params| &params.filter_cut_decay2_ms,
                |params| &params.filter_cut_decay2_level,
                |params| &params.filter_cut_sustain_ms,
                |params| &params.filter_cut_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.filter_cut_attack_ms);
                    labeled_knob(cx, "Hold", Data::params.clone(), |params| &params.filter_cut_hold_ms);
                    labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.filter_cut_decay_ms);
                    labeled_knob(cx, "D2", Data::params.clone(), |params| &params.filter_cut_decay2_ms);
                })
                .col_between(Pixels(6.0));
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "D2Lvl", Data::params.clone(), |params| &params.filter_cut_decay2_level);
                    labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.filter_cut_sustain_ms);
                    labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.filter_cut_release_ms);
                    labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.filter_cut_envelope_level);
                })
                .col_between(Pixels(6.0));
            })
            .row_between(Pixels(6.0));
        })
        .row_between(Pixels(8.0))
        .width(Stretch(1.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Dist Envelope")
                .height(Pixels(16.0))
                .width(Pixels(120.0));
            EnvelopeDisplay::new(
                cx,
                Data::params.clone(),
                |params| &params.dist_env_attack_ms,
                |params| &params.dist_env_hold_ms,
                |params| &params.dist_env_decay_ms,
                |params| &params.dist_env_decay2_ms,
                |params| &params.dist_env_decay2_level,
                |params| &params.dist_env_sustain_level,
                |params| &params.dist_env_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.dist_env_attack_ms);
                    labeled_knob(cx, "Hold", Data::params.clone(), |params| &params.dist_env_hold_ms);
                    labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.dist_env_decay_ms);
                    labeled_knob(cx, "D2", Data::params.clone(), |params| &params.dist_env_decay2_ms);
                })
                .col_between(Pixels(6.0));
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "D2Lvl", Data::params.clone(), |params| &params.dist_env_decay2_level);
                    labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.dist_env_sustain_level);
                    labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.dist_env_release_ms);
                    labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.dist_env_amount);
                })
                .col_between(Pixels(6.0));
            })
            .row_between(Pixels(6.0));

            Label::new(cx, "Filter Res Env")
                .height(Pixels(16.0))
                .width(Pixels(120.0))
                .child_top(Pixels(8.0));
            EnvelopeDisplay::new(
                cx,
                Data::params.clone(),
                |params| &params.filter_res_attack_ms,
                |params| &params.filter_res_hold_ms,
                |params| &params.filter_res_decay_ms,
                |params| &params.filter_res_decay2_ms,
                |params| &params.filter_res_decay2_level,
                |params| &params.filter_res_sustain_ms,
                |params| &params.filter_res_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.filter_res_attack_ms);
                    labeled_knob(cx, "Hold", Data::params.clone(), |params| &params.filter_res_hold_ms);
                    labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.filter_res_decay_ms);
                    labeled_knob(cx, "D2", Data::params.clone(), |params| &params.filter_res_decay2_ms);
                })
                .col_between(Pixels(6.0));
                HStack::new(cx, |cx| {
                    labeled_knob(cx, "D2Lvl", Data::params.clone(), |params| &params.filter_res_decay2_level);
                    labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.filter_res_sustain_ms);
                    labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.filter_res_release_ms);
                    labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.filter_res_envelope_level);
                })
                .col_between(Pixels(6.0));
            })
            .row_between(Pixels(6.0));

            HStack::new(cx, |cx| {
                ParamButton::new(cx, Data::params.clone(), |params| &params.filter_cut_env_polarity)
                    .with_label("Cut Pol")
                    .width(Pixels(80.0))
                    .height(Pixels(26.0));
                ParamButton::new(cx, Data::params.clone(), |params| &params.filter_res_env_polarity)
                    .with_label("Res Pol")
                    .width(Pixels(80.0))
                    .height(Pixels(26.0));
                ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_tension)
                    .width(Pixels(90.0))
                    .height(Pixels(26.0))
                    .with_label("Cut Tens");
                ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_tension)
                    .width(Pixels(90.0))
                    .height(Pixels(26.0))
                    .with_label("Res Tens");
            })
            .col_between(Pixels(6.0))
            .child_top(Pixels(6.0));
        })
        .row_between(Pixels(8.0))
        .width(Stretch(1.0));
    })
    .col_between(Pixels(16.0))
    .row_between(Pixels(10.0))
    .child_top(Pixels(6.0));
}

fn build_mod_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Mod LFO 1")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.lfo1_rate)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.lfo1_attack)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.lfo1_shape)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Mod LFO 2")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.lfo2_rate)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.lfo2_attack)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.lfo2_shape)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Mod Slot 1")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod1_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod1_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod1_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod1_smooth_ms)
                ;
            Label::new(cx, "Mod Slot 2")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_smooth_ms)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Mod Slot 3")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod3_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod3_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod3_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod3_smooth_ms)
                ;
            Label::new(cx, "Mod Slot 4")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod4_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod4_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod4_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod4_smooth_ms)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Mod Slot 5")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod5_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod5_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod5_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod5_smooth_ms)
                ;
            Label::new(cx, "Mod Slot 6")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod6_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod6_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod6_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod6_smooth_ms)
                ;
        })
        .row_between(Pixels(12.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_fx_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Distortion")
                .height(Pixels(16.0))
                .width(Pixels(80.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.dist_enable)
                .with_label("")
                .width(Pixels(70.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.dist_drive)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.dist_tone)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.dist_magic)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.dist_mix)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "EQ")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.eq_enable)
                .with_label("")
                .width(Pixels(63.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.eq_low_gain)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.eq_mid_gain)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.eq_mid_freq)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.eq_mid_q)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.eq_high_gain)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.eq_mix)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Output Sat")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.output_sat_enable)
                .with_label("")
                .width(Pixels(80.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.output_sat_type)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.output_sat_drive)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.output_sat_mix)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Chorus")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.chorus_enable)
                .with_label("")
                .width(Pixels(63.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.chorus_rate)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.chorus_depth)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.chorus_mix)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Delay")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.delay_enable)
                .with_label("")
                .width(Pixels(63.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.delay_time_ms)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.delay_feedback)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.delay_mix)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Reverb")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.reverb_enable)
                .with_label("")
                .width(Pixels(63.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.reverb_size)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.reverb_damp)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.reverb_diffusion)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.reverb_shimmer)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.reverb_mix)
                ;
        })
        .row_between(Pixels(12.0));

    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_utility_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Analog")
                .height(Pixels(16.0))
                .width(Pixels(80.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.analog_enable)
                .with_label("")
                .width(Pixels(80.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.analog_drive)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.analog_noise)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.analog_drift)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Master")
                .height(Pixels(16.0))
                .width(Pixels(80.0));
            Label::new(cx, "Gain")
                .height(Pixels(18.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.gain)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Custom Wavetable")
                .height(Pixels(16.0))
                .width(Pixels(120.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.custom_wavetable_enable)
                .with_label("")
                .width(Pixels(90.0))
                .height(Pixels(28.0));
            Label::new(cx, "WAV Path")
                .height(Pixels(18.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            Textbox::new(cx, Data::custom_wavetable_path_input)
                .width(Pixels(240.0))
                .height(Pixels(24.0))
                .on_edit(|cx, text| cx.emit(UiEvent::CustomWavetablePathChanged(text.clone())))
                .on_submit(|cx, text, _| cx.emit(UiEvent::CustomWavetablePathChanged(text)));
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::PasteCustomWavetablePath),
                |cx| Label::new(cx, "Paste Path"),
            );
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::LoadCustomWavetablePath),
                |cx| Label::new(cx, "Load Path"),
            );
            Label::new(cx, Data::custom_wavetable_display)
                .height(Pixels(18.0))
                .width(Pixels(160.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Limiter")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.limiter_enable)
                .with_label("")
                .width(Pixels(63.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.limiter_threshold)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.limiter_release)
                ;
        })
        .row_between(Pixels(12.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

#[derive(Lens)]
struct ParamKnob {
    param_base: ParamWidgetBase,
    drag_active: bool,
    start_value: f32,
    start_y: f32,
    scrolled_lines: f32,
}

#[derive(Lens)]
struct SeqStepBox {
    param_base: ParamWidgetBase,
    drag_active: bool,
    scrolled_lines: f32,
}

impl SeqStepBox {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
            drag_active: false,
            scrolled_lines: 0.0,
        }
        .build(cx, |_| {})
    }

    fn set_from_cursor(&self, cx: &mut EventContext, bounds: BoundingBox) {
        let t = ((bounds.y + bounds.h - cx.mouse.cursory) / bounds.h).clamp(0.0, 1.0);
        self.param_base.set_normalized_value(cx, t);
    }
}

impl View for SeqStepBox {
    fn element(&self) -> Option<&'static str> {
        Some("seq-step-box")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match *window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let bounds = cx.cache.get_bounds(cx.current());
                let inside = cx.mouse.cursorx >= bounds.x
                    && cx.mouse.cursorx <= bounds.x + bounds.w
                    && cx.mouse.cursory >= bounds.y
                    && cx.mouse.cursory <= bounds.y + bounds.h;
                if inside {
                    self.drag_active = true;
                    self.param_base.begin_set_parameter(cx);
                    self.set_from_cursor(cx, bounds);
                    cx.capture();
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    self.param_base.end_set_parameter(cx);
                    cx.release();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_x, _y) => {
                if self.drag_active {
                    let bounds = cx.cache.get_bounds(cx.current());
                    self.set_from_cursor(cx, bounds);
                    meta.consume();
                }
            }
            WindowEvent::MouseScroll(_scroll_x, scroll_y) => {
                self.scrolled_lines += scroll_y;
                if self.scrolled_lines.abs() >= 1.0 {
                    let use_finer_steps = cx.modifiers.shift();
                    let mut current_value = self.param_base.unmodulated_normalized_value();
                    self.param_base.begin_set_parameter(cx);
                    while self.scrolled_lines >= 1.0 {
                        current_value = self
                            .param_base
                            .next_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines -= 1.0;
                    }
                    while self.scrolled_lines <= -1.0 {
                        current_value = self
                            .param_base
                            .previous_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines += 1.0;
                    }
                    self.param_base.end_set_parameter(cx);
                    meta.consume();
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w == 0.0 || bounds.h == 0.0 {
            return;
        }

        let value = self.param_base.unmodulated_normalized_value().clamp(0.0, 1.0);
        let fill_h = bounds.h * value;
        let fill_y = bounds.y + (bounds.h - fill_h);

        let mut bg = vg::Path::new();
        bg.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        let bg_paint = vg::Paint::color(vg::Color::rgbf(0.12, 0.12, 0.12));
        canvas.fill_path(&mut bg, &bg_paint);

        let mut fill = vg::Path::new();
        fill.rect(bounds.x, fill_y, bounds.w, fill_h);
        let fill_paint = vg::Paint::color(vg::Color::rgbf(1.0, 0.1, 0.1));
        canvas.fill_path(&mut fill, &fill_paint);

        let mut border = vg::Path::new();
        border.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        let border_paint = vg::Paint::color(vg::Color::rgbf(0.4, 0.1, 0.1));
        canvas.stroke_path(&mut border, &border_paint);
    }
}

impl ParamKnob {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            param_base: ParamWidgetBase::new(cx, params, params_to_param),
            drag_active: false,
            start_value: 0.0,
            start_y: 0.0,
            scrolled_lines: 0.0,
        }
        .build(cx, |_| {})
    }
}

impl View for ParamKnob {
    fn element(&self) -> Option<&'static str> {
        Some("param-knob")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match *window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let bounds = cx.cache.get_bounds(cx.current());
                let inside = cx.mouse.cursorx >= bounds.x
                    && cx.mouse.cursorx <= bounds.x + bounds.w
                    && cx.mouse.cursory >= bounds.y
                    && cx.mouse.cursory <= bounds.y + bounds.h;
                if inside {
                    self.drag_active = true;
                    self.start_value = self.param_base.unmodulated_normalized_value();
                    self.start_y = cx.mouse.cursory;
                    self.param_base.begin_set_parameter(cx);
                    cx.capture();
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    self.param_base.end_set_parameter(cx);
                    cx.release();
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_x, y) => {
                if self.drag_active {
                    let delta = (self.start_y - y) / 150.0;
                    let next_value = (self.start_value + delta).clamp(0.0, 1.0);
                    self.param_base.set_normalized_value(cx, next_value);
                    meta.consume();
                }
            }
            WindowEvent::MouseScroll(_scroll_x, scroll_y) => {
                self.scrolled_lines += scroll_y;
                if self.scrolled_lines.abs() >= 1.0 {
                    if !self.drag_active {
                        self.param_base.begin_set_parameter(cx);
                    }
                    let use_finer_steps = cx.modifiers.shift();
                    let mut current_value = self.param_base.unmodulated_normalized_value();
                    while self.scrolled_lines >= 1.0 {
                        current_value = self
                            .param_base
                            .next_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines -= 1.0;
                    }
                    while self.scrolled_lines <= -1.0 {
                        current_value = self
                            .param_base
                            .previous_normalized_step(current_value, use_finer_steps);
                        self.param_base.set_normalized_value(cx, current_value);
                        self.scrolled_lines += 1.0;
                    }
                    if !self.drag_active {
                        self.param_base.end_set_parameter(cx);
                    }
                    meta.consume();
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w == 0.0 || bounds.h == 0.0 {
            return;
        }

        let value = self.param_base.unmodulated_normalized_value().clamp(0.0, 1.0);
        let center_x = bounds.x + bounds.w * 0.5;
        let center_y = bounds.y + bounds.h * 0.5;
        let radius = bounds.w.min(bounds.h) * 0.45;

        let start_angle = -std::f32::consts::PI * 0.75;
        let end_angle = std::f32::consts::PI * 0.75;
        let angle = start_angle + (end_angle - start_angle) * value;

        let mut ring = vg::Path::new();
        ring.arc(
            center_x,
            center_y,
            radius,
            start_angle,
            end_angle,
            vg::Solidity::Solid,
        );
        let ring_paint = vg::Paint::color(vg::Color::rgbf(0.2, 0.2, 0.2));
        canvas.stroke_path(&mut ring, &ring_paint);

        let mut arc = vg::Path::new();
        arc.arc(
            center_x,
            center_y,
            radius,
            start_angle,
            angle,
            vg::Solidity::Solid,
        );
        let arc_paint = vg::Paint::color(vg::Color::rgbf(1.0, 0.2, 0.2));
        canvas.stroke_path(&mut arc, &arc_paint);

        let dot_x = center_x + radius * angle.cos();
        let dot_y = center_y + radius * angle.sin();
        let mut dot = vg::Path::new();
        dot.circle(dot_x, dot_y, radius * 0.12);
        let dot_paint = vg::Paint::color(vg::Color::rgbf(0.95, 0.95, 0.95));
        canvas.fill_path(&mut dot, &dot_paint);
    }
}

enum EnvelopeDrag {
    Attack,
    Hold,
    Decay,
    Decay2,
    Sustain,
    Release,
}

#[derive(Lens)]
struct EnvelopeDisplay {
    attack: ParamWidgetBase,
    hold: ParamWidgetBase,
    decay: ParamWidgetBase,
    decay2: ParamWidgetBase,
    decay2_level: ParamWidgetBase,
    sustain: ParamWidgetBase,
    release: ParamWidgetBase,
    dragging: Option<EnvelopeDrag>,
}

impl EnvelopeDisplay {
    pub fn new<L, Params, P, FAtk, FHold, FDec, FDec2, FDec2Lvl, FSus, FRel>(
        cx: &mut Context,
        params: L,
        attack: FAtk,
        hold: FHold,
        decay: FDec,
        decay2: FDec2,
        decay2_level: FDec2Lvl,
        sustain: FSus,
        release: FRel,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FAtk: Fn(&Params) -> &P + Copy + 'static,
        FHold: Fn(&Params) -> &P + Copy + 'static,
        FDec: Fn(&Params) -> &P + Copy + 'static,
        FDec2: Fn(&Params) -> &P + Copy + 'static,
        FDec2Lvl: Fn(&Params) -> &P + Copy + 'static,
        FSus: Fn(&Params) -> &P + Copy + 'static,
        FRel: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            attack: ParamWidgetBase::new(cx, params.clone(), attack),
            hold: ParamWidgetBase::new(cx, params.clone(), hold),
            decay: ParamWidgetBase::new(cx, params.clone(), decay),
            decay2: ParamWidgetBase::new(cx, params.clone(), decay2),
            decay2_level: ParamWidgetBase::new(cx, params.clone(), decay2_level),
            sustain: ParamWidgetBase::new(cx, params.clone(), sustain),
            release: ParamWidgetBase::new(cx, params, release),
            dragging: None,
        }
        .build(cx, |_| {})
    }

    fn handle_positions(&self, bounds: BoundingBox) -> [(f32, f32); 6] {
        let attack = self.attack.unmodulated_normalized_value().clamp(0.0, 1.0);
        let hold = self.hold.unmodulated_normalized_value().clamp(0.0, 1.0);
        let decay = self.decay.unmodulated_normalized_value().clamp(0.0, 1.0);
        let decay2 = self.decay2.unmodulated_normalized_value().clamp(0.0, 1.0);
        let decay2_level = self.decay2_level.unmodulated_normalized_value().clamp(0.0, 1.0);
        let sustain = self.sustain.unmodulated_normalized_value().clamp(0.0, 1.0);
        let release = self.release.unmodulated_normalized_value().clamp(0.0, 1.0);

        let x_attack = bounds.x + bounds.w * (0.2 * attack);
        let x_hold = bounds.x + bounds.w * (0.2 + 0.1 * hold);
        let x_decay = bounds.x + bounds.w * (0.3 + 0.2 * decay);
        let x_decay2 = bounds.x + bounds.w * (0.5 + 0.2 * decay2);
        let x_sustain = bounds.x + bounds.w * 0.7;
        let x_release = bounds.x + bounds.w * (0.8 + 0.2 * release);
        let y_decay2 = bounds.y + bounds.h * (1.0 - decay2_level);
        let y_sustain = bounds.y + bounds.h * (1.0 - sustain);
        let y_top = bounds.y;
        let y_bottom = bounds.y + bounds.h;

        [
            (x_attack, y_top),
            (x_hold, y_top),
            (x_decay, y_decay2),
            (x_decay2, y_sustain),
            (x_sustain, y_sustain),
            (x_release, y_bottom),
        ]
    }

    fn update_from_drag(&mut self, cx: &mut EventContext, bounds: BoundingBox, x: f32, y: f32) {
        let x_norm = ((x - bounds.x) / bounds.w).clamp(0.0, 1.0);
        let y_norm = ((y - bounds.y) / bounds.h).clamp(0.0, 1.0);
        let sustain = (1.0 - y_norm).clamp(0.0, 1.0);

        match self.dragging {
            Some(EnvelopeDrag::Attack) => {
                let attack = (x_norm / 0.2).clamp(0.0, 1.0);
                self.attack.set_normalized_value(cx, attack);
            }
            Some(EnvelopeDrag::Hold) => {
                let hold = ((x_norm - 0.2) / 0.1).clamp(0.0, 1.0);
                self.hold.set_normalized_value(cx, hold);
            }
            Some(EnvelopeDrag::Decay) => {
                let decay = ((x_norm - 0.3) / 0.2).clamp(0.0, 1.0);
                self.decay.set_normalized_value(cx, decay);
                self.decay2_level.set_normalized_value(cx, sustain);
            }
            Some(EnvelopeDrag::Decay2) => {
                let decay2 = ((x_norm - 0.5) / 0.2).clamp(0.0, 1.0);
                self.decay2.set_normalized_value(cx, decay2);
                self.sustain.set_normalized_value(cx, sustain);
            }
            Some(EnvelopeDrag::Sustain) => {
                self.sustain.set_normalized_value(cx, sustain);
            }
            Some(EnvelopeDrag::Release) => {
                let release = ((x_norm - 0.8) / 0.2).clamp(0.0, 1.0);
                self.release.set_normalized_value(cx, release);
            }
            None => {}
        }
    }
}

impl View for EnvelopeDisplay {
    fn element(&self) -> Option<&'static str> {
        Some("envelope-display")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match *window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let bounds = cx.cache.get_bounds(cx.current());
                let inside = cx.mouse.cursorx >= bounds.x
                    && cx.mouse.cursorx <= bounds.x + bounds.w
                    && cx.mouse.cursory >= bounds.y
                    && cx.mouse.cursory <= bounds.y + bounds.h;
                if inside {
                    let handles = self.handle_positions(bounds);
                    let mut closest = None;
                    let mut best = 12.0_f32 * 12.0_f32;
                    for (idx, (hx, hy)) in handles.iter().enumerate() {
                        let dx = cx.mouse.cursorx - *hx;
                        let dy = cx.mouse.cursory - *hy;
                        let dist = dx * dx + dy * dy;
                        if dist < best {
                            best = dist;
                            closest = Some(idx);
                        }
                    }

                    self.dragging = match closest {
                        Some(0) => Some(EnvelopeDrag::Attack),
                        Some(1) => Some(EnvelopeDrag::Hold),
                        Some(2) => Some(EnvelopeDrag::Decay),
                        Some(3) => Some(EnvelopeDrag::Decay2),
                        Some(4) => Some(EnvelopeDrag::Sustain),
                        Some(5) => Some(EnvelopeDrag::Release),
                        _ => None,
                    };

                    if self.dragging.is_some() {
                        self.attack.begin_set_parameter(cx);
                        self.hold.begin_set_parameter(cx);
                        self.decay.begin_set_parameter(cx);
                        self.decay2.begin_set_parameter(cx);
                        self.decay2_level.begin_set_parameter(cx);
                        self.sustain.begin_set_parameter(cx);
                        self.release.begin_set_parameter(cx);
                        cx.capture();
                        meta.consume();
                    }
                }
            }
            WindowEvent::MouseMove(x, y) => {
                if self.dragging.is_some() {
                    let bounds = cx.cache.get_bounds(cx.current());
                    self.update_from_drag(cx, bounds, x, y);
                    meta.consume();
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging.is_some() {
                    self.attack.end_set_parameter(cx);
                    self.hold.end_set_parameter(cx);
                    self.decay.end_set_parameter(cx);
                    self.decay2.end_set_parameter(cx);
                    self.decay2_level.end_set_parameter(cx);
                    self.sustain.end_set_parameter(cx);
                    self.release.end_set_parameter(cx);
                    self.dragging = None;
                    cx.release();
                    meta.consume();
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w == 0.0 || bounds.h == 0.0 {
            return;
        }

        let handles = self.handle_positions(bounds);
        let attack_x = handles[0].0;
        let hold_x = handles[1].0;
        let decay_x = handles[2].0;
        let decay2_x = handles[3].0;
        let sustain_x = bounds.x + bounds.w * 0.7;
        let sustain_end = bounds.x + bounds.w * 0.8;
        let release_x = handles[5].0;
        let top = bounds.y;
        let bottom = bounds.y + bounds.h;
        let decay2_y = handles[2].1;
        let sustain_y = handles[4].1;

        let mut path = vg::Path::new();
        path.move_to(bounds.x, bottom);
        path.line_to(attack_x, top);
        path.line_to(hold_x, top);
        path.line_to(decay_x, decay2_y);
        path.line_to(decay2_x, sustain_y);
        path.line_to(sustain_x, sustain_y);
        path.line_to(sustain_end, sustain_y);
        path.line_to(release_x, bottom);
        path.line_to(bounds.x + bounds.w, bottom);

        let stroke = vg::Paint::color(vg::Color::rgbf(1.0, 0.2, 0.2));
        canvas.stroke_path(&mut path, &stroke);

        for (x, y) in handles.iter() {
            let mut dot = vg::Path::new();
            dot.circle(*x, *y, 4.0);
            let paint = vg::Paint::color(vg::Color::rgbf(0.9, 0.9, 0.9));
            canvas.fill_path(&mut dot, &paint);
        }

    }
}

pub(crate) fn create(
    params: Arc<SubSynthParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // Register zCool font
        cx.add_fonts_mem(&[ZCOOL_FONT_DATA]);
        
        // Set zCool as the default font for the entire UI
        cx.set_default_font(&[ZCOOL_XIAOWEI]);

        let presets = load_presets(&params);
        let default_index = 0;
        let preset_display = presets
            .get(default_index)
            .map(|preset| preset.name.clone())
            .unwrap_or_else(|| "Init".to_string());
        let seq_preset_display = seq_preset_name(0).to_string();

        Data {
            params: params.clone(),
            active_tab: 0,
            presets,
            preset_index: default_index,
            preset_display,
            preset_name: String::new(),
            seq_preset_index: 0,
            seq_preset_display,
            custom_wavetable_display: params
                .custom_wavetable_path
                .read()
                .ok()
                .and_then(|path| (*path).clone())
                .and_then(|path| PathBuf::from(path).file_name().map(|name| name.to_string_lossy().to_string()))
                .unwrap_or_else(|| "No custom wav".to_string()),
            custom_wavetable_path_input: params
                .custom_wavetable_path
                .read()
                .ok()
                .and_then(|path| (*path).clone())
                .unwrap_or_default(),
        }
        .build(cx);

        ResizeHandle::new(cx);
        ScrollView::new(cx, 0.0, 0.0, false, true, |cx| {
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "DogSynth")
                        .font_family(vec![FamilyOwned::Name(String::from(ZCOOL_XIAOWEI))])
                        .font_size(24.0)
                        .height(Pixels(36.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    Element::new(cx).width(Stretch(1.0));
                    Label::new(cx, "by Sanny + Ling Lin")
                        .font_family(vec![FamilyOwned::Name(String::from(ZCOOL_XIAOWEI))])
                        .font_size(14.0)
                        .height(Pixels(36.0))
                        .color(Color::rgb(255, 0, 0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                })
                .col_between(Pixels(10.0))
                .height(Pixels(36.0))
                .width(Stretch(1.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));

                build_preset_bar(cx);
                build_tab_bar(cx);

                Binding::new(cx, Data::active_tab, |cx, tab| match tab.get(cx) {
                    0 => build_osc_tab(cx),
                    1 => build_mixer_tab(cx),
                    2 => build_env_tab(cx),
                    3 => build_mod_tab(cx),
                    4 => build_lfo_tab(cx),
                    5 => build_articulator_tab(cx),
                    6 => build_seq_tab(cx),
                    7 => build_filter_tab(cx),
                    8 => build_fx_tab(cx),
                    _ => build_utility_tab(cx),
                });

                Element::new(cx)
                    .height(Pixels(12.0))
                    .width(Stretch(1.0));
            })
            .row_between(Pixels(6.0))
            .width(Stretch(1.0));
        })
        .width(Percentage(100.0))
        .height(Percentage(100.0));

        // レイアウトを初期化時にリセットする
        cx.emit(GuiContextEvent::Resize);
    })
}
                
