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

use crate::{util, waveform::load_wavetable_from_file, FilterType, ModSource, ModTarget, OscRouting, OscillatorShape, SubSynthParams, UnisonVoices, Waveform};

// zCool font constant
const ZCOOL_XIAOWEI: &str = "ZCOOL XiaoWei";
const ZCOOL_FONT_DATA: &[u8] = include_bytes!("assets/ZCOOL_XIAOWEI_REGULAR.ttf");
const SEQ_LANE_COUNT: usize = 6;
const SEQ_STEP_COUNT: usize = 32;
const SEQ_PRESET_COUNT: usize = 5;

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct PresetData {
    gain: f32,
    amp_attack_ms: f32,
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
    #[serde(default)]
    additive_mix: f32,
    #[serde(default)]
    additive_partials: f32,
    #[serde(default)]
    additive_tilt: f32,
    #[serde(default)]
    additive_inharm: f32,
    #[serde(default)]
    additive_morph: f32,
    #[serde(default)]
    additive_decay: f32,
    #[serde(default)]
    additive_drift: f32,
    #[serde(default)]
    vel_additive_amount: f32,
    custom_wavetable_enable: f32,
    analog_enable: f32,
    analog_drive: f32,
    analog_noise: f32,
    analog_drift: f32,
    #[serde(default)]
    breath_enable: f32,
    #[serde(default)]
    breath_amount: f32,
    #[serde(default)]
    breath_attack_ms: f32,
    #[serde(default)]
    breath_decay_ms: f32,
    #[serde(default)]
    breath_tone: f32,
    sub_level: f32,
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
    amp_sustain_level: f32,
    filter_type: f32,
    filter_cut: f32,
    filter_res: f32,
    filter_amount: f32,
    filter_cut_attack_ms: f32,
    filter_cut_decay_ms: f32,
    filter_cut_sustain_ms: f32,
    filter_cut_release_ms: f32,
    filter_res_attack_ms: f32,
    filter_res_decay_ms: f32,
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
    fm_env_decay_ms: f32,
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
    dist_env_decay_ms: f32,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PresetEntry {
    name: String,
    pub(crate) data: PresetData,
    user: bool,
    category_index: usize,
}

impl nih_plug_vizia::vizia::prelude::Data for PresetData {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl nih_plug_vizia::vizia::prelude::Data for PresetEntry {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Lens)]
struct Data {
    params: Arc<SubSynthParams>,
    active_tab: usize,
    presets: Vec<PresetEntry>,
    preset_index: usize,
    preset_display: String,
    preset_name: String,
    preset_filter: String,
    preset_category: usize,
    seq_preset_index: usize,
    seq_preset_display: String,
    custom_wavetable_display: String,
    custom_wavetable_path_input: String,
}

enum UiEvent {
    SetTab(usize),
    PresetPrev,
    PresetNext,
    PresetSelect(usize, bool),
    PresetLoad,
    PresetSave,
    PresetRefresh,
    PresetNameChanged(String),
    PresetFilterChanged(String),
    PresetCategorySelect(usize),
    MorphStoreA,
    MorphStoreB,
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
                        let preset = &self.presets[self.preset_index];
                        self.preset_display = preset.name.clone();
                        self.preset_category = preset.category_index;
                    }
                }
                UiEvent::PresetNext => {
                    if !self.presets.is_empty() {
                        self.preset_index = (self.preset_index + 1) % self.presets.len();
                        let preset = &self.presets[self.preset_index];
                        self.preset_display = preset.name.clone();
                        self.preset_category = preset.category_index;
                    }
                }
                UiEvent::PresetSelect(index, apply_now) => {
                    if let Some(preset) = self.presets.get(index) {
                        self.preset_index = index;
                        self.preset_display = preset.name.clone();
                        self.preset_category = preset.category_index;
                        if apply_now {
                            preset.data.apply(cx, &self.params);
                            apply_param(
                                cx,
                                &self.params.preset_index,
                                normalized(&self.params.preset_index, self.preset_index as i32),
                            );
                        }
                    }
                }
                UiEvent::PresetLoad => {
                    if let Some(preset) = self.presets.get(self.preset_index) {
                        preset.data.apply(cx, &self.params);
                        apply_param(
                            cx,
                            &self.params.preset_index,
                            normalized(&self.params.preset_index, self.preset_index as i32),
                        );
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
                                    category_index: PRESET_CATEGORY_USER,
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
                        self.preset_category = preset.category_index;
                    }
                }
                UiEvent::PresetNameChanged(value) => {
                    self.preset_name = value;
                }
                UiEvent::PresetFilterChanged(value) => {
                    self.preset_filter = value;
                }
                UiEvent::PresetCategorySelect(index) => {
                    self.preset_category = index;
                }
                UiEvent::MorphStoreA => {
                    self.store_morph_snapshot(true);
                }
                UiEvent::MorphStoreB => {
                    self.store_morph_snapshot(false);
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
    fn store_morph_snapshot(&mut self, is_a: bool) {
        let snapshot = PresetData::from_params(&self.params);
        if let Ok(json) = serde_json::to_string(&snapshot) {
            let target = if is_a {
                &self.params.morph_a_snapshot
            } else {
                &self.params.morph_b_snapshot
            };
            if let Ok(mut slot) = target.write() {
                *slot = Some(json);
            }
        }
    }

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
    ViziaState::new(|| (1180, 860))
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
        PathBuf::from(appdata).join("MiceSynth").join("Presets")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".micesynth").join("presets")
    } else {
        PathBuf::from("presets")
    }
}

const GM_CATEGORY_NAMES: [&str; 16] = [
    "Piano",
    "Chromatic",
    "Organ",
    "Guitar",
    "Bass",
    "Strings",
    "Ensemble",
    "Brass",
    "Reed",
    "Pipe",
    "Synth Lead",
    "Synth Pad",
    "Synth FX",
    "Ethnic",
    "Percussive",
    "Sound FX",
];

const PRESET_CATEGORY_USER: usize = 16;

const GM_PROGRAMS: [&str; 128] = [
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

fn gm_category_index(program_index: usize) -> usize {
    program_index / 8
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

fn preset_filter_match(name: &str, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&filter.to_lowercase())
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
                    category_index: PRESET_CATEGORY_USER,
                });
            }
        }
    }

    presets
}

pub(crate) fn load_presets(params: &SubSynthParams) -> Vec<PresetEntry> {
    let mut presets = factory_presets(params);
    save_factory_presets_if_missing(&presets);
    presets.extend(load_user_presets());
    presets
}

fn normalized<P: Param>(param: &P, plain: P::Plain) -> f32 {
    param.preview_normalized(plain)
}

fn save_factory_presets_if_missing(presets: &[PresetEntry]) {
    let dir = preset_root().join("Factory");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    for (index, preset) in presets.iter().enumerate() {
        if preset.user {
            continue;
        }
        let name = sanitize_preset_name(&preset.name, "Factory Preset");
        let filename = format!("{:03}_{}.syn", index + 1, name);
        let path = dir.join(filename);
        if path.exists() {
            continue;
        }
        if let Ok(json) = serde_json::to_string_pretty(&preset.data) {
            let _ = fs::write(path, json);
        }
    }
}

impl PresetData {
    fn from_params(params: &SubSynthParams) -> Self {
        Self {
            gain: params.gain.unmodulated_normalized_value(),
            amp_attack_ms: params.amp_attack_ms.unmodulated_normalized_value(),
            amp_release_ms: params.amp_release_ms.unmodulated_normalized_value(),
            amp_tension: params.amp_tension.unmodulated_normalized_value(),
            waveform: params.waveform.unmodulated_normalized_value(),
            osc_routing: params.osc_routing.unmodulated_normalized_value(),
            osc_blend: params.osc_blend.unmodulated_normalized_value(),
            wavetable_position: params.wavetable_position.unmodulated_normalized_value(),
            wavetable_distortion: params.wavetable_distortion.unmodulated_normalized_value(),
            classic_drive: params.classic_drive.unmodulated_normalized_value(),
            additive_mix: params.additive_mix.unmodulated_normalized_value(),
            additive_partials: params.additive_partials.unmodulated_normalized_value(),
            additive_tilt: params.additive_tilt.unmodulated_normalized_value(),
            additive_inharm: params.additive_inharm.unmodulated_normalized_value(),
            additive_morph: params.additive_morph.unmodulated_normalized_value(),
            additive_decay: params.additive_decay.unmodulated_normalized_value(),
            additive_drift: params.additive_drift.unmodulated_normalized_value(),
            vel_additive_amount: params.vel_additive_amount.unmodulated_normalized_value(),
            custom_wavetable_enable: params.custom_wavetable_enable.unmodulated_normalized_value(),
            analog_enable: params.analog_enable.unmodulated_normalized_value(),
            analog_drive: params.analog_drive.unmodulated_normalized_value(),
            analog_noise: params.analog_noise.unmodulated_normalized_value(),
            analog_drift: params.analog_drift.unmodulated_normalized_value(),
            breath_enable: params.breath_enable.unmodulated_normalized_value(),
            breath_amount: params.breath_amount.unmodulated_normalized_value(),
            breath_attack_ms: params.breath_attack_ms.unmodulated_normalized_value(),
            breath_decay_ms: params.breath_decay_ms.unmodulated_normalized_value(),
            breath_tone: params.breath_tone.unmodulated_normalized_value(),
            sub_level: params.sub_level.unmodulated_normalized_value(),
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
            amp_sustain_level: params.amp_sustain_level.unmodulated_normalized_value(),
            filter_type: params.filter_type.unmodulated_normalized_value(),
            filter_cut: params.filter_cut.unmodulated_normalized_value(),
            filter_res: params.filter_res.unmodulated_normalized_value(),
            filter_amount: params.filter_amount.unmodulated_normalized_value(),
            filter_cut_attack_ms: params.filter_cut_attack_ms.unmodulated_normalized_value(),
            filter_cut_decay_ms: params.filter_cut_decay_ms.unmodulated_normalized_value(),
            filter_cut_sustain_ms: params.filter_cut_sustain_ms.unmodulated_normalized_value(),
            filter_cut_release_ms: params.filter_cut_release_ms.unmodulated_normalized_value(),
            filter_res_attack_ms: params.filter_res_attack_ms.unmodulated_normalized_value(),
            filter_res_decay_ms: params.filter_res_decay_ms.unmodulated_normalized_value(),
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
            fm_env_decay_ms: params.fm_env_decay_ms.unmodulated_normalized_value(),
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
            dist_env_decay_ms: params.dist_env_decay_ms.unmodulated_normalized_value(),
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
        apply_param(cx, &params.amp_release_ms, self.amp_release_ms);
        apply_param(cx, &params.amp_tension, self.amp_tension);
        apply_param(cx, &params.waveform, self.waveform);
        apply_param(cx, &params.osc_routing, self.osc_routing);
        apply_param(cx, &params.osc_blend, self.osc_blend);
        apply_param(cx, &params.wavetable_position, self.wavetable_position);
        apply_param(cx, &params.wavetable_distortion, self.wavetable_distortion);
        apply_param(cx, &params.classic_drive, self.classic_drive);
        apply_param(cx, &params.additive_mix, self.additive_mix);
        apply_param(cx, &params.additive_partials, self.additive_partials);
        apply_param(cx, &params.additive_tilt, self.additive_tilt);
        apply_param(cx, &params.additive_inharm, self.additive_inharm);
        apply_param(cx, &params.additive_morph, self.additive_morph);
        apply_param(cx, &params.additive_decay, self.additive_decay);
        apply_param(cx, &params.additive_drift, self.additive_drift);
        apply_param(cx, &params.vel_additive_amount, self.vel_additive_amount);
        apply_param(cx, &params.custom_wavetable_enable, self.custom_wavetable_enable);
        apply_param(cx, &params.analog_enable, self.analog_enable);
        apply_param(cx, &params.analog_drive, self.analog_drive);
        apply_param(cx, &params.analog_noise, self.analog_noise);
        apply_param(cx, &params.analog_drift, self.analog_drift);
        apply_param(cx, &params.breath_enable, self.breath_enable);
        apply_param(cx, &params.breath_amount, self.breath_amount);
        apply_param(cx, &params.breath_attack_ms, self.breath_attack_ms);
        apply_param(cx, &params.breath_decay_ms, self.breath_decay_ms);
        apply_param(cx, &params.breath_tone, self.breath_tone);
        apply_param(cx, &params.sub_level, self.sub_level);
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
        apply_param(cx, &params.amp_sustain_level, self.amp_sustain_level);
        apply_param(cx, &params.filter_type, self.filter_type);
        apply_param(cx, &params.filter_cut, self.filter_cut);
        apply_param(cx, &params.filter_res, self.filter_res);
        apply_param(cx, &params.filter_amount, self.filter_amount);
        apply_param(cx, &params.filter_cut_attack_ms, self.filter_cut_attack_ms);
        apply_param(cx, &params.filter_cut_decay_ms, self.filter_cut_decay_ms);
        apply_param(cx, &params.filter_cut_sustain_ms, self.filter_cut_sustain_ms);
        apply_param(cx, &params.filter_cut_release_ms, self.filter_cut_release_ms);
        apply_param(cx, &params.filter_res_attack_ms, self.filter_res_attack_ms);
        apply_param(cx, &params.filter_res_decay_ms, self.filter_res_decay_ms);
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
        apply_param(cx, &params.fm_env_decay_ms, self.fm_env_decay_ms);
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
        apply_param(cx, &params.dist_env_decay_ms, self.dist_env_decay_ms);
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
        apply_param_direct(&params.gain, self.gain);
        apply_param_direct(&params.amp_attack_ms, self.amp_attack_ms);
        apply_param_direct(&params.amp_release_ms, self.amp_release_ms);
        apply_param_direct(&params.amp_tension, self.amp_tension);
        apply_param_direct(&params.waveform, self.waveform);
        apply_param_direct(&params.osc_routing, self.osc_routing);
        apply_param_direct(&params.osc_blend, self.osc_blend);
        apply_param_direct(&params.wavetable_position, self.wavetable_position);
        apply_param_direct(&params.wavetable_distortion, self.wavetable_distortion);
        apply_param_direct(&params.classic_drive, self.classic_drive);
        apply_param_direct(&params.additive_mix, self.additive_mix);
        apply_param_direct(&params.additive_partials, self.additive_partials);
        apply_param_direct(&params.additive_tilt, self.additive_tilt);
        apply_param_direct(&params.additive_inharm, self.additive_inharm);
        apply_param_direct(&params.additive_morph, self.additive_morph);
        apply_param_direct(&params.additive_decay, self.additive_decay);
        apply_param_direct(&params.additive_drift, self.additive_drift);
        apply_param_direct(&params.vel_additive_amount, self.vel_additive_amount);
        apply_param_direct(&params.custom_wavetable_enable, self.custom_wavetable_enable);
        apply_param_direct(&params.analog_enable, self.analog_enable);
        apply_param_direct(&params.analog_drive, self.analog_drive);
        apply_param_direct(&params.analog_noise, self.analog_noise);
        apply_param_direct(&params.analog_drift, self.analog_drift);
        apply_param_direct(&params.breath_enable, self.breath_enable);
        apply_param_direct(&params.breath_amount, self.breath_amount);
        apply_param_direct(&params.breath_attack_ms, self.breath_attack_ms);
        apply_param_direct(&params.breath_decay_ms, self.breath_decay_ms);
        apply_param_direct(&params.breath_tone, self.breath_tone);
        apply_param_direct(&params.sub_level, self.sub_level);
        apply_param_direct(&params.unison_voices, self.unison_voices);
        apply_param_direct(&params.unison_detune, self.unison_detune);
        apply_param_direct(&params.unison_spread, self.unison_spread);
        apply_param_direct(&params.glide_mode, self.glide_mode);
        apply_param_direct(&params.glide_time_ms, self.glide_time_ms);
        apply_param_direct(&params.lfo1_rate, self.lfo1_rate);
        apply_param_direct(&params.lfo1_attack, self.lfo1_attack);
        apply_param_direct(&params.lfo1_shape, self.lfo1_shape);
        apply_param_direct(&params.lfo2_rate, self.lfo2_rate);
        apply_param_direct(&params.lfo2_attack, self.lfo2_attack);
        apply_param_direct(&params.lfo2_shape, self.lfo2_shape);
        apply_param_direct(&params.mod1_source, self.mod1_source);
        apply_param_direct(&params.mod1_target, self.mod1_target);
        apply_param_direct(&params.mod1_amount, self.mod1_amount);
        apply_param_direct(&params.mod1_smooth_ms, self.mod1_smooth_ms);
        apply_param_direct(&params.mod2_source, self.mod2_source);
        apply_param_direct(&params.mod2_target, self.mod2_target);
        apply_param_direct(&params.mod2_amount, self.mod2_amount);
        apply_param_direct(&params.mod2_smooth_ms, self.mod2_smooth_ms);
        apply_param_direct(&params.mod3_source, self.mod3_source);
        apply_param_direct(&params.mod3_target, self.mod3_target);
        apply_param_direct(&params.mod3_amount, self.mod3_amount);
        apply_param_direct(&params.mod3_smooth_ms, self.mod3_smooth_ms);
        apply_param_direct(&params.mod4_source, self.mod4_source);
        apply_param_direct(&params.mod4_target, self.mod4_target);
        apply_param_direct(&params.mod4_amount, self.mod4_amount);
        apply_param_direct(&params.mod4_smooth_ms, self.mod4_smooth_ms);
        apply_param_direct(&params.mod5_source, self.mod5_source);
        apply_param_direct(&params.mod5_target, self.mod5_target);
        apply_param_direct(&params.mod5_amount, self.mod5_amount);
        apply_param_direct(&params.mod5_smooth_ms, self.mod5_smooth_ms);
        apply_param_direct(&params.mod6_source, self.mod6_source);
        apply_param_direct(&params.mod6_target, self.mod6_target);
        apply_param_direct(&params.mod6_amount, self.mod6_amount);
        apply_param_direct(&params.mod6_smooth_ms, self.mod6_smooth_ms);
        apply_param_direct(&params.seq_enable, self.seq_enable);
        apply_param_direct(&params.seq_rate, self.seq_rate);
        apply_param_direct(&params.seq_gate_amount, self.seq_gate_amount);
        apply_param_direct(&params.seq_cut_amount, self.seq_cut_amount);
        apply_param_direct(&params.seq_res_amount, self.seq_res_amount);
        apply_param_direct(&params.seq_wt_amount, self.seq_wt_amount);
        apply_param_direct(&params.seq_dist_amount, self.seq_dist_amount);
        apply_param_direct(&params.seq_fm_amount, self.seq_fm_amount);
        for lane in 0..SEQ_LANE_COUNT {
            for step in 0..SEQ_STEP_COUNT {
                let step_value = self
                    .seq_steps
                    .get(lane)
                    .map(|lane_steps| lane_steps[step])
                    .unwrap_or(0.0);
                apply_param_direct(&params.seq_lanes[lane].steps[step].value, step_value);
            }
        }
        apply_param_direct(&params.amp_decay_ms, self.amp_decay_ms);
        apply_param_direct(&params.amp_sustain_level, self.amp_sustain_level);
        apply_param_direct(&params.filter_type, self.filter_type);
        apply_param_direct(&params.filter_cut, self.filter_cut);
        apply_param_direct(&params.filter_res, self.filter_res);
        apply_param_direct(&params.filter_amount, self.filter_amount);
        apply_param_direct(&params.filter_cut_attack_ms, self.filter_cut_attack_ms);
        apply_param_direct(&params.filter_cut_decay_ms, self.filter_cut_decay_ms);
        apply_param_direct(&params.filter_cut_sustain_ms, self.filter_cut_sustain_ms);
        apply_param_direct(&params.filter_cut_release_ms, self.filter_cut_release_ms);
        apply_param_direct(&params.filter_res_attack_ms, self.filter_res_attack_ms);
        apply_param_direct(&params.filter_res_decay_ms, self.filter_res_decay_ms);
        apply_param_direct(&params.filter_res_sustain_ms, self.filter_res_sustain_ms);
        apply_param_direct(&params.filter_res_release_ms, self.filter_res_release_ms);
        apply_param_direct(&params.amp_envelope_level, self.amp_envelope_level);
        apply_param_direct(&params.filter_cut_envelope_level, self.filter_cut_envelope_level);
        apply_param_direct(&params.filter_res_envelope_level, self.filter_res_envelope_level);
        apply_param_direct(&params.fm_enable, self.fm_enable);
        apply_param_direct(&params.fm_source, self.fm_source);
        apply_param_direct(&params.fm_target, self.fm_target);
        apply_param_direct(&params.fm_amount, self.fm_amount);
        apply_param_direct(&params.fm_ratio, self.fm_ratio);
        apply_param_direct(&params.fm_feedback, self.fm_feedback);
        apply_param_direct(&params.fm_env_attack_ms, self.fm_env_attack_ms);
        apply_param_direct(&params.fm_env_decay_ms, self.fm_env_decay_ms);
        apply_param_direct(&params.fm_env_sustain_level, self.fm_env_sustain_level);
        apply_param_direct(&params.fm_env_release_ms, self.fm_env_release_ms);
        apply_param_direct(&params.fm_env_amount, self.fm_env_amount);
        apply_param_direct(&params.vibrato_attack, self.vibrato_attack);
        apply_param_direct(&params.vibrato_intensity, self.vibrato_intensity);
        apply_param_direct(&params.vibrato_rate, self.vibrato_rate);
        apply_param_direct(&params.tremolo_attack, self.tremolo_attack);
        apply_param_direct(&params.tremolo_intensity, self.tremolo_intensity);
        apply_param_direct(&params.tremolo_rate, self.tremolo_rate);
        apply_param_direct(&params.vibrato_shape, self.vibrato_shape);
        apply_param_direct(&params.tremolo_shape, self.tremolo_shape);
        apply_param_direct(&params.filter_cut_env_polarity, self.filter_cut_env_polarity);
        apply_param_direct(&params.filter_res_env_polarity, self.filter_res_env_polarity);
        apply_param_direct(&params.filter_cut_tension, self.filter_cut_tension);
        apply_param_direct(&params.filter_res_tension, self.filter_res_tension);
        apply_param_direct(&params.cutoff_lfo_attack, self.cutoff_lfo_attack);
        apply_param_direct(&params.res_lfo_attack, self.res_lfo_attack);
        apply_param_direct(&params.pan_lfo_attack, self.pan_lfo_attack);
        apply_param_direct(&params.cutoff_lfo_intensity, self.cutoff_lfo_intensity);
        apply_param_direct(&params.cutoff_lfo_rate, self.cutoff_lfo_rate);
        apply_param_direct(&params.cutoff_lfo_shape, self.cutoff_lfo_shape);
        apply_param_direct(&params.res_lfo_intensity, self.res_lfo_intensity);
        apply_param_direct(&params.res_lfo_rate, self.res_lfo_rate);
        apply_param_direct(&params.res_lfo_shape, self.res_lfo_shape);
        apply_param_direct(&params.pan_lfo_intensity, self.pan_lfo_intensity);
        apply_param_direct(&params.pan_lfo_rate, self.pan_lfo_rate);
        apply_param_direct(&params.pan_lfo_shape, self.pan_lfo_shape);
        apply_param_direct(&params.chorus_enable, self.chorus_enable);
        apply_param_direct(&params.chorus_rate, self.chorus_rate);
        apply_param_direct(&params.chorus_depth, self.chorus_depth);
        apply_param_direct(&params.chorus_mix, self.chorus_mix);
        apply_param_direct(&params.delay_enable, self.delay_enable);
        apply_param_direct(&params.delay_time_ms, self.delay_time_ms);
        apply_param_direct(&params.delay_feedback, self.delay_feedback);
        apply_param_direct(&params.delay_mix, self.delay_mix);
        apply_param_direct(&params.reverb_enable, self.reverb_enable);
        apply_param_direct(&params.reverb_size, self.reverb_size);
        apply_param_direct(&params.reverb_damp, self.reverb_damp);
        apply_param_direct(&params.reverb_diffusion, self.reverb_diffusion);
        apply_param_direct(&params.reverb_shimmer, self.reverb_shimmer);
        apply_param_direct(&params.reverb_mix, self.reverb_mix);
        apply_param_direct(&params.output_sat_enable, self.output_sat_enable);
        apply_param_direct(&params.output_sat_type, self.output_sat_type);
        apply_param_direct(&params.output_sat_drive, self.output_sat_drive);
        apply_param_direct(&params.output_sat_mix, self.output_sat_mix);
        apply_param_direct(&params.dist_env_attack_ms, self.dist_env_attack_ms);
        apply_param_direct(&params.dist_env_decay_ms, self.dist_env_decay_ms);
        apply_param_direct(&params.dist_env_sustain_level, self.dist_env_sustain_level);
        apply_param_direct(&params.dist_env_release_ms, self.dist_env_release_ms);
        apply_param_direct(&params.dist_env_amount, self.dist_env_amount);
        apply_param_direct(&params.multi_filter_enable, self.multi_filter_enable);
        apply_param_direct(&params.multi_filter_routing, self.multi_filter_routing);
        apply_param_direct(&params.multi_filter_morph, self.multi_filter_morph);
        apply_param_direct(&params.multi_filter_parallel_ab, self.multi_filter_parallel_ab);
        apply_param_direct(&params.multi_filter_parallel_c, self.multi_filter_parallel_c);
        apply_param_direct(&params.multi_filter_a_type, self.multi_filter_a_type);
        apply_param_direct(&params.multi_filter_a_cut, self.multi_filter_a_cut);
        apply_param_direct(&params.multi_filter_a_res, self.multi_filter_a_res);
        apply_param_direct(&params.multi_filter_a_amt, self.multi_filter_a_amt);
        apply_param_direct(&params.multi_filter_b_type, self.multi_filter_b_type);
        apply_param_direct(&params.multi_filter_b_cut, self.multi_filter_b_cut);
        apply_param_direct(&params.multi_filter_b_res, self.multi_filter_b_res);
        apply_param_direct(&params.multi_filter_b_amt, self.multi_filter_b_amt);
        apply_param_direct(&params.multi_filter_c_type, self.multi_filter_c_type);
        apply_param_direct(&params.multi_filter_c_cut, self.multi_filter_c_cut);
        apply_param_direct(&params.multi_filter_c_res, self.multi_filter_c_res);
        apply_param_direct(&params.multi_filter_c_amt, self.multi_filter_c_amt);
        apply_param_direct(&params.limiter_enable, self.limiter_enable);
        apply_param_direct(&params.limiter_threshold, self.limiter_threshold);
        apply_param_direct(&params.limiter_release, self.limiter_release);
    }

    pub(crate) fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let lerp = |x: f32, y: f32| x + (y - x) * t;

        let mut seq_steps = Vec::with_capacity(SEQ_LANE_COUNT);
        for lane in 0..SEQ_LANE_COUNT {
            let mut steps = [0.0f32; SEQ_STEP_COUNT];
            for step in 0..SEQ_STEP_COUNT {
                let a_val = a.seq_steps.get(lane).map(|s| s[step]).unwrap_or(0.0);
                let b_val = b.seq_steps.get(lane).map(|s| s[step]).unwrap_or(0.0);
                steps[step] = lerp(a_val, b_val);
            }
            seq_steps.push(steps);
        }

        Self {
            gain: lerp(a.gain, b.gain),
            amp_attack_ms: lerp(a.amp_attack_ms, b.amp_attack_ms),
            amp_release_ms: lerp(a.amp_release_ms, b.amp_release_ms),
            amp_tension: lerp(a.amp_tension, b.amp_tension),
            waveform: lerp(a.waveform, b.waveform),
            osc_routing: lerp(a.osc_routing, b.osc_routing),
            osc_blend: lerp(a.osc_blend, b.osc_blend),
            wavetable_position: lerp(a.wavetable_position, b.wavetable_position),
            wavetable_distortion: lerp(a.wavetable_distortion, b.wavetable_distortion),
            classic_drive: lerp(a.classic_drive, b.classic_drive),
            additive_mix: lerp(a.additive_mix, b.additive_mix),
            additive_partials: lerp(a.additive_partials, b.additive_partials),
            additive_tilt: lerp(a.additive_tilt, b.additive_tilt),
            additive_inharm: lerp(a.additive_inharm, b.additive_inharm),
            additive_morph: lerp(a.additive_morph, b.additive_morph),
            additive_decay: lerp(a.additive_decay, b.additive_decay),
            additive_drift: lerp(a.additive_drift, b.additive_drift),
            vel_additive_amount: lerp(a.vel_additive_amount, b.vel_additive_amount),
            custom_wavetable_enable: lerp(a.custom_wavetable_enable, b.custom_wavetable_enable),
            analog_enable: lerp(a.analog_enable, b.analog_enable),
            analog_drive: lerp(a.analog_drive, b.analog_drive),
            analog_noise: lerp(a.analog_noise, b.analog_noise),
            analog_drift: lerp(a.analog_drift, b.analog_drift),
            breath_enable: lerp(a.breath_enable, b.breath_enable),
            breath_amount: lerp(a.breath_amount, b.breath_amount),
            breath_attack_ms: lerp(a.breath_attack_ms, b.breath_attack_ms),
            breath_decay_ms: lerp(a.breath_decay_ms, b.breath_decay_ms),
            breath_tone: lerp(a.breath_tone, b.breath_tone),
            sub_level: lerp(a.sub_level, b.sub_level),
            unison_voices: lerp(a.unison_voices, b.unison_voices),
            unison_detune: lerp(a.unison_detune, b.unison_detune),
            unison_spread: lerp(a.unison_spread, b.unison_spread),
            glide_mode: lerp(a.glide_mode, b.glide_mode),
            glide_time_ms: lerp(a.glide_time_ms, b.glide_time_ms),
            lfo1_rate: lerp(a.lfo1_rate, b.lfo1_rate),
            lfo1_attack: lerp(a.lfo1_attack, b.lfo1_attack),
            lfo1_shape: lerp(a.lfo1_shape, b.lfo1_shape),
            lfo2_rate: lerp(a.lfo2_rate, b.lfo2_rate),
            lfo2_attack: lerp(a.lfo2_attack, b.lfo2_attack),
            lfo2_shape: lerp(a.lfo2_shape, b.lfo2_shape),
            mod1_source: lerp(a.mod1_source, b.mod1_source),
            mod1_target: lerp(a.mod1_target, b.mod1_target),
            mod1_amount: lerp(a.mod1_amount, b.mod1_amount),
            mod1_smooth_ms: lerp(a.mod1_smooth_ms, b.mod1_smooth_ms),
            mod2_source: lerp(a.mod2_source, b.mod2_source),
            mod2_target: lerp(a.mod2_target, b.mod2_target),
            mod2_amount: lerp(a.mod2_amount, b.mod2_amount),
            mod2_smooth_ms: lerp(a.mod2_smooth_ms, b.mod2_smooth_ms),
            mod3_source: lerp(a.mod3_source, b.mod3_source),
            mod3_target: lerp(a.mod3_target, b.mod3_target),
            mod3_amount: lerp(a.mod3_amount, b.mod3_amount),
            mod3_smooth_ms: lerp(a.mod3_smooth_ms, b.mod3_smooth_ms),
            mod4_source: lerp(a.mod4_source, b.mod4_source),
            mod4_target: lerp(a.mod4_target, b.mod4_target),
            mod4_amount: lerp(a.mod4_amount, b.mod4_amount),
            mod4_smooth_ms: lerp(a.mod4_smooth_ms, b.mod4_smooth_ms),
            mod5_source: lerp(a.mod5_source, b.mod5_source),
            mod5_target: lerp(a.mod5_target, b.mod5_target),
            mod5_amount: lerp(a.mod5_amount, b.mod5_amount),
            mod5_smooth_ms: lerp(a.mod5_smooth_ms, b.mod5_smooth_ms),
            mod6_source: lerp(a.mod6_source, b.mod6_source),
            mod6_target: lerp(a.mod6_target, b.mod6_target),
            mod6_amount: lerp(a.mod6_amount, b.mod6_amount),
            mod6_smooth_ms: lerp(a.mod6_smooth_ms, b.mod6_smooth_ms),
            seq_enable: lerp(a.seq_enable, b.seq_enable),
            seq_rate: lerp(a.seq_rate, b.seq_rate),
            seq_gate_amount: lerp(a.seq_gate_amount, b.seq_gate_amount),
            seq_cut_amount: lerp(a.seq_cut_amount, b.seq_cut_amount),
            seq_res_amount: lerp(a.seq_res_amount, b.seq_res_amount),
            seq_wt_amount: lerp(a.seq_wt_amount, b.seq_wt_amount),
            seq_dist_amount: lerp(a.seq_dist_amount, b.seq_dist_amount),
            seq_fm_amount: lerp(a.seq_fm_amount, b.seq_fm_amount),
            seq_steps,
            amp_decay_ms: lerp(a.amp_decay_ms, b.amp_decay_ms),
            amp_sustain_level: lerp(a.amp_sustain_level, b.amp_sustain_level),
            filter_type: lerp(a.filter_type, b.filter_type),
            filter_cut: lerp(a.filter_cut, b.filter_cut),
            filter_res: lerp(a.filter_res, b.filter_res),
            filter_amount: lerp(a.filter_amount, b.filter_amount),
            filter_cut_attack_ms: lerp(a.filter_cut_attack_ms, b.filter_cut_attack_ms),
            filter_cut_decay_ms: lerp(a.filter_cut_decay_ms, b.filter_cut_decay_ms),
            filter_cut_sustain_ms: lerp(a.filter_cut_sustain_ms, b.filter_cut_sustain_ms),
            filter_cut_release_ms: lerp(a.filter_cut_release_ms, b.filter_cut_release_ms),
            filter_res_attack_ms: lerp(a.filter_res_attack_ms, b.filter_res_attack_ms),
            filter_res_decay_ms: lerp(a.filter_res_decay_ms, b.filter_res_decay_ms),
            filter_res_sustain_ms: lerp(a.filter_res_sustain_ms, b.filter_res_sustain_ms),
            filter_res_release_ms: lerp(a.filter_res_release_ms, b.filter_res_release_ms),
            amp_envelope_level: lerp(a.amp_envelope_level, b.amp_envelope_level),
            filter_cut_envelope_level: lerp(a.filter_cut_envelope_level, b.filter_cut_envelope_level),
            filter_res_envelope_level: lerp(a.filter_res_envelope_level, b.filter_res_envelope_level),
            fm_enable: lerp(a.fm_enable, b.fm_enable),
            fm_source: lerp(a.fm_source, b.fm_source),
            fm_target: lerp(a.fm_target, b.fm_target),
            fm_amount: lerp(a.fm_amount, b.fm_amount),
            fm_ratio: lerp(a.fm_ratio, b.fm_ratio),
            fm_feedback: lerp(a.fm_feedback, b.fm_feedback),
            fm_env_attack_ms: lerp(a.fm_env_attack_ms, b.fm_env_attack_ms),
            fm_env_decay_ms: lerp(a.fm_env_decay_ms, b.fm_env_decay_ms),
            fm_env_sustain_level: lerp(a.fm_env_sustain_level, b.fm_env_sustain_level),
            fm_env_release_ms: lerp(a.fm_env_release_ms, b.fm_env_release_ms),
            fm_env_amount: lerp(a.fm_env_amount, b.fm_env_amount),
            vibrato_attack: lerp(a.vibrato_attack, b.vibrato_attack),
            vibrato_intensity: lerp(a.vibrato_intensity, b.vibrato_intensity),
            vibrato_rate: lerp(a.vibrato_rate, b.vibrato_rate),
            tremolo_attack: lerp(a.tremolo_attack, b.tremolo_attack),
            tremolo_intensity: lerp(a.tremolo_intensity, b.tremolo_intensity),
            tremolo_rate: lerp(a.tremolo_rate, b.tremolo_rate),
            vibrato_shape: lerp(a.vibrato_shape, b.vibrato_shape),
            tremolo_shape: lerp(a.tremolo_shape, b.tremolo_shape),
            filter_cut_env_polarity: lerp(a.filter_cut_env_polarity, b.filter_cut_env_polarity),
            filter_res_env_polarity: lerp(a.filter_res_env_polarity, b.filter_res_env_polarity),
            filter_cut_tension: lerp(a.filter_cut_tension, b.filter_cut_tension),
            filter_res_tension: lerp(a.filter_res_tension, b.filter_res_tension),
            cutoff_lfo_attack: lerp(a.cutoff_lfo_attack, b.cutoff_lfo_attack),
            res_lfo_attack: lerp(a.res_lfo_attack, b.res_lfo_attack),
            pan_lfo_attack: lerp(a.pan_lfo_attack, b.pan_lfo_attack),
            cutoff_lfo_intensity: lerp(a.cutoff_lfo_intensity, b.cutoff_lfo_intensity),
            cutoff_lfo_rate: lerp(a.cutoff_lfo_rate, b.cutoff_lfo_rate),
            cutoff_lfo_shape: lerp(a.cutoff_lfo_shape, b.cutoff_lfo_shape),
            res_lfo_intensity: lerp(a.res_lfo_intensity, b.res_lfo_intensity),
            res_lfo_rate: lerp(a.res_lfo_rate, b.res_lfo_rate),
            res_lfo_shape: lerp(a.res_lfo_shape, b.res_lfo_shape),
            pan_lfo_intensity: lerp(a.pan_lfo_intensity, b.pan_lfo_intensity),
            pan_lfo_rate: lerp(a.pan_lfo_rate, b.pan_lfo_rate),
            pan_lfo_shape: lerp(a.pan_lfo_shape, b.pan_lfo_shape),
            chorus_enable: lerp(a.chorus_enable, b.chorus_enable),
            chorus_rate: lerp(a.chorus_rate, b.chorus_rate),
            chorus_depth: lerp(a.chorus_depth, b.chorus_depth),
            chorus_mix: lerp(a.chorus_mix, b.chorus_mix),
            delay_enable: lerp(a.delay_enable, b.delay_enable),
            delay_time_ms: lerp(a.delay_time_ms, b.delay_time_ms),
            delay_feedback: lerp(a.delay_feedback, b.delay_feedback),
            delay_mix: lerp(a.delay_mix, b.delay_mix),
            reverb_enable: lerp(a.reverb_enable, b.reverb_enable),
            reverb_size: lerp(a.reverb_size, b.reverb_size),
            reverb_damp: lerp(a.reverb_damp, b.reverb_damp),
            reverb_diffusion: lerp(a.reverb_diffusion, b.reverb_diffusion),
            reverb_shimmer: lerp(a.reverb_shimmer, b.reverb_shimmer),
            reverb_mix: lerp(a.reverb_mix, b.reverb_mix),
            dist_enable: lerp(a.dist_enable, b.dist_enable),
            dist_drive: lerp(a.dist_drive, b.dist_drive),
            dist_tone: lerp(a.dist_tone, b.dist_tone),
            dist_magic: lerp(a.dist_magic, b.dist_magic),
            dist_mix: lerp(a.dist_mix, b.dist_mix),
            dist_env_attack_ms: lerp(a.dist_env_attack_ms, b.dist_env_attack_ms),
            dist_env_decay_ms: lerp(a.dist_env_decay_ms, b.dist_env_decay_ms),
            dist_env_sustain_level: lerp(a.dist_env_sustain_level, b.dist_env_sustain_level),
            dist_env_release_ms: lerp(a.dist_env_release_ms, b.dist_env_release_ms),
            dist_env_amount: lerp(a.dist_env_amount, b.dist_env_amount),
            eq_enable: lerp(a.eq_enable, b.eq_enable),
            eq_low_gain: lerp(a.eq_low_gain, b.eq_low_gain),
            eq_mid_gain: lerp(a.eq_mid_gain, b.eq_mid_gain),
            eq_mid_freq: lerp(a.eq_mid_freq, b.eq_mid_freq),
            eq_mid_q: lerp(a.eq_mid_q, b.eq_mid_q),
            eq_high_gain: lerp(a.eq_high_gain, b.eq_high_gain),
            eq_mix: lerp(a.eq_mix, b.eq_mix),
            output_sat_enable: lerp(a.output_sat_enable, b.output_sat_enable),
            output_sat_type: lerp(a.output_sat_type, b.output_sat_type),
            output_sat_drive: lerp(a.output_sat_drive, b.output_sat_drive),
            output_sat_mix: lerp(a.output_sat_mix, b.output_sat_mix),
            multi_filter_enable: lerp(a.multi_filter_enable, b.multi_filter_enable),
            multi_filter_routing: lerp(a.multi_filter_routing, b.multi_filter_routing),
            multi_filter_morph: lerp(a.multi_filter_morph, b.multi_filter_morph),
            multi_filter_parallel_ab: lerp(a.multi_filter_parallel_ab, b.multi_filter_parallel_ab),
            multi_filter_parallel_c: lerp(a.multi_filter_parallel_c, b.multi_filter_parallel_c),
            multi_filter_a_type: lerp(a.multi_filter_a_type, b.multi_filter_a_type),
            multi_filter_a_cut: lerp(a.multi_filter_a_cut, b.multi_filter_a_cut),
            multi_filter_a_res: lerp(a.multi_filter_a_res, b.multi_filter_a_res),
            multi_filter_a_amt: lerp(a.multi_filter_a_amt, b.multi_filter_a_amt),
            multi_filter_b_type: lerp(a.multi_filter_b_type, b.multi_filter_b_type),
            multi_filter_b_cut: lerp(a.multi_filter_b_cut, b.multi_filter_b_cut),
            multi_filter_b_res: lerp(a.multi_filter_b_res, b.multi_filter_b_res),
            multi_filter_b_amt: lerp(a.multi_filter_b_amt, b.multi_filter_b_amt),
            multi_filter_c_type: lerp(a.multi_filter_c_type, b.multi_filter_c_type),
            multi_filter_c_cut: lerp(a.multi_filter_c_cut, b.multi_filter_c_cut),
            multi_filter_c_res: lerp(a.multi_filter_c_res, b.multi_filter_c_res),
            multi_filter_c_amt: lerp(a.multi_filter_c_amt, b.multi_filter_c_amt),
            limiter_enable: lerp(a.limiter_enable, b.limiter_enable),
            limiter_threshold: lerp(a.limiter_threshold, b.limiter_threshold),
            limiter_release: lerp(a.limiter_release, b.limiter_release),
        }
    }
}

fn apply_param<P: Param>(cx: &mut EventContext, param: &P, normalized: f32) {
    cx.emit(ParamEvent::BeginSetParameter(param).upcast());
    cx.emit(ParamEvent::SetParameterNormalized(param, normalized).upcast());
    cx.emit(ParamEvent::EndSetParameter(param).upcast());
}

fn apply_param_direct<P: Param>(param: &P, normalized: f32) {
    unsafe {
        param.as_ptr().set_normalized_value(normalized);
    }
}

fn blend_towards(value: f32, target: f32, amount: f32) -> f32 {
    value + (target - value) * amount
}

fn apply_category_envelope(
    category: usize,
    attack_ms: f32,
    decay_ms: f32,
    sustain: f32,
    release_ms: f32,
) -> (f32, f32, f32, f32) {
    let (target_attack, target_decay, target_sustain, target_release, blend) = match category {
        0 => (1.0, 12.0, 0.2, 3.0, 0.4),   // Piano
        1 => (0.4, 18.0, 0.1, 2.5, 0.4),   // Chromatic
        2 => (0.6, 6.0, 0.95, 2.8, 0.4),   // Organ
        3 => (0.6, 18.0, 0.3, 2.8, 0.4),   // Guitar
        4 => (0.7, 14.0, 0.65, 2.8, 0.4),  // Bass
        5 => (4.0, 40.0, 0.85, 6.0, 0.35), // Strings
        6 => (3.0, 32.0, 0.8, 5.2, 0.35),  // Ensemble
        7 => (2.0, 24.0, 0.75, 4.0, 0.35), // Brass
        8 => (1.6, 20.0, 0.75, 3.6, 0.35), // Reed
        9 => (1.8, 22.0, 0.8, 3.8, 0.35),  // Pipe
        10 => (0.6, 14.0, 0.6, 2.6, 0.35), // Synth Lead
        11 => (6.0, 46.0, 0.85, 7.5, 0.35), // Synth Pad
        12 => (2.0, 26.0, 0.4, 4.5, 0.4),  // Synth FX
        13 => (1.4, 22.0, 0.55, 3.6, 0.35), // Ethnic
        14 => (0.3, 12.0, 0.0, 1.6, 0.5),  // Percussive
        _ => (1.0, 20.0, 0.3, 3.5, 0.45),  // Sound FX
    };

    (
        blend_towards(attack_ms, target_attack, blend),
        blend_towards(decay_ms, target_decay, blend),
        blend_towards(sustain, target_sustain, blend),
        blend_towards(release_ms, target_release, blend),
    )
}

pub(crate) fn factory_presets(params: &SubSynthParams) -> Vec<PresetEntry> {
    let mut presets = Vec::with_capacity(GM_PROGRAMS.len());
    let default_gain = normalized(&params.gain, util::db_to_gain(-18.0));

    for (index, name) in GM_PROGRAMS.iter().enumerate() {
        let category = gm_category_index(index);
        let variant = (index % 8) as f32;
        let mut preset = PresetData::from_params(params);
        let name_lower = name.to_lowercase();

        preset.gain = default_gain;
        preset.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
        preset.osc_blend = normalized(&params.osc_blend, 0.0);
        preset.classic_drive = normalized(&params.classic_drive, 0.0);
        preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.0);
        preset.dist_enable = normalized(&params.dist_enable, false);
        preset.dist_drive = normalized(&params.dist_drive, 0.0);
        preset.dist_tone = normalized(&params.dist_tone, 0.6);
        preset.dist_magic = normalized(&params.dist_magic, 0.2);
        preset.dist_mix = normalized(&params.dist_mix, 0.0);
        preset.analog_enable = normalized(&params.analog_enable, false);
        preset.analog_drive = normalized(&params.analog_drive, 0.0);
        preset.analog_noise = normalized(&params.analog_noise, 0.0);
        preset.analog_drift = normalized(&params.analog_drift, 0.0);
        preset.breath_enable = normalized(&params.breath_enable, false);
        preset.breath_amount = normalized(&params.breath_amount, 0.0);
        preset.breath_attack_ms = normalized(&params.breath_attack_ms, 2.0);
        preset.breath_decay_ms = normalized(&params.breath_decay_ms, 180.0);
        preset.breath_tone = normalized(&params.breath_tone, 2400.0);
        preset.additive_mix = normalized(&params.additive_mix, 0.04);
        preset.additive_partials = normalized(&params.additive_partials, 20.0);
        preset.additive_tilt = normalized(&params.additive_tilt, -0.1);
        preset.additive_inharm = normalized(&params.additive_inharm, 0.02);
        preset.additive_morph = normalized(&params.additive_morph, 0.25);
        preset.additive_decay = normalized(&params.additive_decay, 0.3);
        preset.additive_drift = normalized(&params.additive_drift, 0.05);
        preset.vel_additive_amount = normalized(&params.vel_additive_amount, 0.0);
        preset.output_sat_enable = normalized(&params.output_sat_enable, true);
        preset.output_sat_drive = normalized(&params.output_sat_drive, 0.35);
        preset.output_sat_mix = normalized(&params.output_sat_mix, 0.7);
        preset.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
        preset.filter_res = normalized(&params.filter_res, 0.2);
        preset.filter_amount = normalized(&params.filter_amount, 0.35);
        preset.sub_level = normalized(&params.sub_level, 0.0);
        preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::One);
        preset.unison_detune = normalized(&params.unison_detune, 0.12);
        preset.unison_spread = normalized(&params.unison_spread, 0.2);

        if category == 8
            || category == 9
            || name_lower.contains("flute")
            || name_lower.contains("oboe")
            || name_lower.contains("clarinet")
            || name_lower.contains("sax")
            || name_lower.contains("bassoon")
            || name_lower.contains("recorder")
            || name_lower.contains("pan flute")
            || name_lower.contains("shakuhachi")
            || name_lower.contains("shanai")
        {
            preset.breath_enable = normalized(&params.breath_enable, true);
            preset.breath_amount = normalized(&params.breath_amount, 0.35);
            preset.breath_attack_ms = normalized(&params.breath_attack_ms, 2.5);
            preset.breath_decay_ms = normalized(&params.breath_decay_ms, 220.0);
            preset.breath_tone = normalized(&params.breath_tone, 2600.0);
        }

        let mut waveform = Waveform::Triangle;
        let mut filter_cut = 2000.0;
        let mut attack_ms = 1.0;
        let mut decay_ms = 22.0;
        let mut sustain = 0.5;
        let mut release_ms = 3.0;

        if index < 8 {
            preset.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
            preset.osc_blend = normalized(&params.osc_blend, 0.0);
            preset.additive_mix = normalized(&params.additive_mix, 0.18);
            preset.additive_partials = normalized(&params.additive_partials, 48.0);
            preset.additive_tilt = normalized(&params.additive_tilt, -0.25);
            preset.additive_inharm = normalized(&params.additive_inharm, 0.03);
            preset.additive_morph = normalized(&params.additive_morph, 0.15);
            preset.additive_decay = normalized(&params.additive_decay, 0.55);
            preset.additive_drift = normalized(&params.additive_drift, 0.02);
            preset.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
            preset.filter_amount = normalized(&params.filter_amount, 0.55);
            preset.filter_cut_attack_ms = normalized(&params.filter_cut_attack_ms, 0.8);
            preset.filter_cut_decay_ms = normalized(&params.filter_cut_decay_ms, 4.0);
            preset.filter_cut_release_ms = normalized(&params.filter_cut_release_ms, 4.0);
            preset.filter_cut_envelope_level =
                normalized(&params.filter_cut_envelope_level, 0.55);
            preset.eq_enable = normalized(&params.eq_enable, true);
            preset.eq_low_gain = normalized(&params.eq_low_gain, 1.5);
            preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.3);
            preset.eq_mid_freq = normalized(&params.eq_mid_freq, 900.0);
            preset.eq_mid_q = normalized(&params.eq_mid_q, 0.9);
            preset.eq_high_gain = normalized(&params.eq_high_gain, 1.5);

            match index {
                0 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 2600.0;
                    attack_ms = 1.8;
                    decay_ms = 8.0;
                    sustain = 0.18;
                    release_ms = 4.0;
                    preset.sub_level = normalized(&params.sub_level, 0.08);
                    preset.additive_partials = normalized(&params.additive_partials, 56.0);
                    preset.additive_tilt = normalized(&params.additive_tilt, -0.3);
                    preset.additive_inharm = normalized(&params.additive_inharm, 0.04);
                    preset.additive_decay = normalized(&params.additive_decay, 0.6);
                    preset.eq_low_gain = normalized(&params.eq_low_gain, 1.8);
                    preset.eq_mid_gain = normalized(&params.eq_mid_gain, -0.2);
                }
                1 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 6200.0;
                    attack_ms = 1.2;
                    decay_ms = 6.0;
                    sustain = 0.12;
                    release_ms = 3.2;
                    preset.additive_tilt = normalized(&params.additive_tilt, 0.05);
                    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.6);
                    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1600.0);
                    preset.eq_high_gain = normalized(&params.eq_high_gain, 3.0);
                }
                2 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 4200.0;
                    attack_ms = 1.2;
                    decay_ms = 10.0;
                    sustain = 0.35;
                    release_ms = 5.0;
                    preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                    preset.osc_blend = normalized(&params.osc_blend, 0.45);
                    preset.wavetable_position = normalized(&params.wavetable_position, 0.55);
                    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.15);
                    preset.additive_mix = normalized(&params.additive_mix, 0.12);
                    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.5);
                    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1200.0);
                    preset.chorus_enable = normalized(&params.chorus_enable, true);
                    preset.chorus_mix = normalized(&params.chorus_mix, 0.2);
                }
                3 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 4000.0;
                    attack_ms = 0.7;
                    decay_ms = 5.0;
                    sustain = 0.1;
                    release_ms = 2.5;
                    preset.unison_voices =
                        normalized(&params.unison_voices, UnisonVoices::Two);
                    preset.unison_detune = normalized(&params.unison_detune, 0.1);
                    preset.unison_spread = normalized(&params.unison_spread, 0.2);
                    preset.additive_inharm = normalized(&params.additive_inharm, 0.05);
                    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 1.5);
                    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1000.0);
                }
                4 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 3600.0;
                    attack_ms = 1.0;
                    decay_ms = 12.0;
                    sustain = 0.45;
                    release_ms = 6.0;
                    preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                    preset.osc_blend = normalized(&params.osc_blend, 0.6);
                    preset.wavetable_position = normalized(&params.wavetable_position, 0.45);
                    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.2);
                    preset.additive_tilt = normalized(&params.additive_tilt, 0.1);
                    preset.additive_morph = normalized(&params.additive_morph, 0.35);
                    preset.additive_inharm = normalized(&params.additive_inharm, 0.05);
                    preset.chorus_enable = normalized(&params.chorus_enable, true);
                    preset.chorus_mix = normalized(&params.chorus_mix, 0.4);
                }
                5 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 5200.0;
                    attack_ms = 0.8;
                    decay_ms = 9.0;
                    sustain = 0.4;
                    release_ms = 4.5;
                    preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                    preset.osc_blend = normalized(&params.osc_blend, 0.75);
                    preset.wavetable_position = normalized(&params.wavetable_position, 0.7);
                    preset.wavetable_distortion = normalized(&params.wavetable_distortion, 0.35);
                    preset.additive_mix = normalized(&params.additive_mix, 0.08);
                    preset.additive_tilt = normalized(&params.additive_tilt, 0.2);
                    preset.additive_inharm = normalized(&params.additive_inharm, 0.08);
                    preset.chorus_enable = normalized(&params.chorus_enable, true);
                    preset.chorus_mix = normalized(&params.chorus_mix, 0.25);
                }
                6 => {
                    waveform = Waveform::Pulse;
                    filter_cut = 7200.0;
                    attack_ms = 0.2;
                    decay_ms = 3.0;
                    sustain = 0.0;
                    release_ms = 1.2;
                    preset.filter_amount = normalized(&params.filter_amount, 0.2);
                    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.7);
                    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 2000.0);
                    preset.eq_high_gain = normalized(&params.eq_high_gain, 2.5);
                    preset.additive_mix = normalized(&params.additive_mix, 0.05);
                }
                _ => {
                    waveform = Waveform::Pulse;
                    filter_cut = 4800.0;
                    attack_ms = 0.3;
                    decay_ms = 4.0;
                    sustain = 0.2;
                    release_ms = 2.0;
                    preset.classic_drive = normalized(&params.classic_drive, 0.12);
                    preset.eq_mid_gain = normalized(&params.eq_mid_gain, 1.2);
                    preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1200.0);
                    preset.additive_mix = normalized(&params.additive_mix, 0.08);
                }
            }
        } else {
            match category {
                0 => {
                    waveform = Waveform::Triangle;
                    filter_cut = 2600.0;
                    attack_ms = 2.0;
                    decay_ms = 35.0;
                    sustain = 0.2;
                    release_ms = 4.0;
                }
                1 => {
                match index {
                    8 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 4800.0;
                        attack_ms = 0.3;
                        decay_ms = 20.0;
                        sustain = 0.0;
                        release_ms = 2.2;
                        preset.additive_mix = normalized(&params.additive_mix, 0.12);
                        preset.additive_partials = normalized(&params.additive_partials, 36.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.15);
                    }
                    9 => {
                        waveform = Waveform::Sine;
                        filter_cut = 7200.0;
                        attack_ms = 0.12;
                        decay_ms = 12.0;
                        sustain = 0.0;
                        release_ms = 1.6;
                        preset.additive_mix = normalized(&params.additive_mix, 0.18);
                        preset.additive_partials = normalized(&params.additive_partials, 28.0);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.12);
                    }
                    10 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 6200.0;
                        attack_ms = 0.2;
                        decay_ms = 14.0;
                        sustain = 0.0;
                        release_ms = 1.8;
                        preset.additive_mix = normalized(&params.additive_mix, 0.2);
                        preset.additive_partials = normalized(&params.additive_partials, 24.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, 0.1);
                    }
                    11 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 5200.0;
                        attack_ms = 0.6;
                        decay_ms = 26.0;
                        sustain = 0.2;
                        release_ms = 3.5;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.55);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.2);
                    }
                    12 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3600.0;
                        attack_ms = 0.8;
                        decay_ms = 30.0;
                        sustain = 0.25;
                        release_ms = 4.2;
                        preset.additive_partials = normalized(&params.additive_partials, 42.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.05);
                    }
                    13 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 5200.0;
                        attack_ms = 0.2;
                        decay_ms = 16.0;
                        sustain = 0.0;
                        release_ms = 2.2;
                        preset.additive_partials = normalized(&params.additive_partials, 32.0);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.1);
                    }
                    14 => {
                        waveform = Waveform::Sine;
                        filter_cut = 6800.0;
                        attack_ms = 0.15;
                        decay_ms = 45.0;
                        sustain = 0.0;
                        release_ms = 6.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.2);
                        preset.additive_partials = normalized(&params.additive_partials, 30.0);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.18);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.25);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 4200.0;
                        attack_ms = 0.35;
                        decay_ms = 18.0;
                        sustain = 0.0;
                        release_ms = 2.2;
                        preset.additive_partials = normalized(&params.additive_partials, 40.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.1);
                    }
                }
            }
            2 => {
                match index {
                    16 => {
                        waveform = Waveform::Square;
                        filter_cut = 5200.0;
                        attack_ms = 0.4;
                        decay_ms = 6.0;
                        sustain = 0.95;
                        release_ms = 2.2;
                        preset.additive_mix = normalized(&params.additive_mix, 0.12);
                        preset.additive_partials = normalized(&params.additive_partials, 28.0);
                    }
                    17 => {
                        waveform = Waveform::Square;
                        filter_cut = 4800.0;
                        attack_ms = 0.8;
                        decay_ms = 10.0;
                        sustain = 0.85;
                        release_ms = 3.0;
                        preset.additive_partials = normalized(&params.additive_partials, 40.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.1);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.2);
                    }
                    18 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 3600.0;
                        attack_ms = 0.3;
                        decay_ms = 8.0;
                        sustain = 0.8;
                        release_ms = 2.2;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.35);
                        preset.additive_mix = normalized(&params.additive_mix, 0.08);
                    }
                    19 => {
                        waveform = Waveform::Square;
                        filter_cut = 3000.0;
                        attack_ms = 1.0;
                        decay_ms = 12.0;
                        sustain = 0.9;
                        release_ms = 3.8;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::Lowpass);
                        preset.filter_amount = normalized(&params.filter_amount, 0.45);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.25);
                    }
                    20 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 0.9;
                        decay_ms = 12.0;
                        sustain = 0.8;
                        release_ms = 3.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.15);
                        preset.additive_partials = normalized(&params.additive_partials, 36.0);
                    }
                    21 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 4200.0;
                        attack_ms = 0.6;
                        decay_ms = 14.0;
                        sustain = 0.85;
                        release_ms = 3.2;
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.22);
                    }
                    22 => {
                        waveform = Waveform::Square;
                        filter_cut = 3800.0;
                        attack_ms = 0.5;
                        decay_ms = 16.0;
                        sustain = 0.85;
                        release_ms = 3.4;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.4);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.2);
                    }
                    _ => {
                        waveform = Waveform::Square;
                        filter_cut = 3400.0;
                        attack_ms = 0.7;
                        decay_ms = 18.0;
                        sustain = 0.88;
                        release_ms = 3.6;
                        preset.additive_partials = normalized(&params.additive_partials, 32.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.25);
                    }
                }
            }
            3 => {
                match index {
                    24 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3200.0;
                        attack_ms = 1.2;
                        decay_ms = 26.0;
                        sustain = 0.25;
                        release_ms = 4.0;
                        preset.additive_partials = normalized(&params.additive_partials, 44.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.15);
                        preset.filter_amount = normalized(&params.filter_amount, 0.45);
                    }
                    25 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 4600.0;
                        attack_ms = 0.9;
                        decay_ms = 20.0;
                        sustain = 0.22;
                        release_ms = 3.6;
                        preset.additive_tilt = normalized(&params.additive_tilt, 0.1);
                        preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.7);
                        preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1400.0);
                    }
                    26 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 5200.0;
                        attack_ms = 0.8;
                        decay_ms = 18.0;
                        sustain = 0.18;
                        release_ms = 3.2;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.35);
                        preset.eq_mid_gain = normalized(&params.eq_mid_gain, 1.2);
                        preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1800.0);
                    }
                    27 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 3000.0;
                        attack_ms = 1.0;
                        decay_ms = 22.0;
                        sustain = 0.2;
                        release_ms = 3.8;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.45);
                        preset.filter_amount = normalized(&params.filter_amount, 0.5);
                    }
                    28 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.3;
                        decay_ms = 30.0;
                        sustain = 0.28;
                        release_ms = 4.5;
                        preset.additive_mix = normalized(&params.additive_mix, 0.16);
                        preset.additive_partials = normalized(&params.additive_partials, 48.0);
                    }
                    29 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3600.0;
                        attack_ms = 1.0;
                        decay_ms = 24.0;
                        sustain = 0.24;
                        release_ms = 4.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.12);
                        preset.additive_tilt = normalized(&params.additive_tilt, 0.05);
                    }
                    30 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 5200.0;
                        attack_ms = 0.9;
                        decay_ms = 18.0;
                        sustain = 0.2;
                        release_ms = 3.5;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Two);
                        preset.unison_detune = normalized(&params.unison_detune, 0.08);
                        preset.unison_spread = normalized(&params.unison_spread, 0.25);
                        preset.eq_mid_gain = normalized(&params.eq_mid_gain, 1.0);
                        preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1700.0);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3000.0;
                        attack_ms = 1.1;
                        decay_ms = 26.0;
                        sustain = 0.2;
                        release_ms = 4.2;
                        preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.6);
                        preset.eq_mid_freq = normalized(&params.eq_mid_freq, 1200.0);
                    }
                }
            }
            4 => {
                match index {
                    32 => {
                        waveform = Waveform::Sine;
                        filter_cut = 240.0;
                        attack_ms = 0.6;
                        decay_ms = 18.0;
                        sustain = 0.7;
                        release_ms = 3.0;
                        preset.sub_level = normalized(&params.sub_level, 0.8);
                        preset.filter_amount = normalized(&params.filter_amount, 0.25);
                    }
                    33 => {
                        waveform = Waveform::Sine;
                        filter_cut = 280.0;
                        attack_ms = 0.5;
                        decay_ms = 16.0;
                        sustain = 0.65;
                        release_ms = 2.8;
                        preset.sub_level = normalized(&params.sub_level, 0.85);
                        preset.additive_mix = normalized(&params.additive_mix, 0.06);
                    }
                    34 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 420.0;
                        attack_ms = 0.4;
                        decay_ms = 14.0;
                        sustain = 0.6;
                        release_ms = 2.6;
                        preset.sub_level = normalized(&params.sub_level, 0.7);
                        preset.additive_mix = normalized(&params.additive_mix, 0.08);
                    }
                    35 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 520.0;
                        attack_ms = 0.35;
                        decay_ms = 12.0;
                        sustain = 0.55;
                        release_ms = 2.4;
                        preset.sub_level = normalized(&params.sub_level, 0.65);
                        preset.eq_mid_gain = normalized(&params.eq_mid_gain, 0.6);
                        preset.eq_mid_freq = normalized(&params.eq_mid_freq, 700.0);
                    }
                    36 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 620.0;
                        attack_ms = 0.3;
                        decay_ms = 10.0;
                        sustain = 0.5;
                        release_ms = 2.2;
                        preset.sub_level = normalized(&params.sub_level, 0.55);
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.25);
                    }
                    37 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 780.0;
                        attack_ms = 0.25;
                        decay_ms = 9.0;
                        sustain = 0.45;
                        release_ms = 2.0;
                        preset.sub_level = normalized(&params.sub_level, 0.5);
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Two);
                        preset.unison_detune = normalized(&params.unison_detune, 0.1);
                        preset.unison_spread = normalized(&params.unison_spread, 0.2);
                    }
                    38 => {
                        waveform = Waveform::Square;
                        filter_cut = 640.0;
                        attack_ms = 0.2;
                        decay_ms = 8.0;
                        sustain = 0.4;
                        release_ms = 1.8;
                        preset.sub_level = normalized(&params.sub_level, 0.45);
                        preset.filter_amount = normalized(&params.filter_amount, 0.3);
                    }
                    _ => {
                        waveform = Waveform::Square;
                        filter_cut = 900.0;
                        attack_ms = 0.2;
                        decay_ms = 7.0;
                        sustain = 0.38;
                        release_ms = 1.6;
                        preset.sub_level = normalized(&params.sub_level, 0.4);
                        preset.filter_amount = normalized(&params.filter_amount, 0.35);
                    }
                }
            }
            5 => {
                match index {
                    40 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2200.0;
                        attack_ms = 3.8;
                        decay_ms = 40.0;
                        sustain = 0.82;
                        release_ms = 6.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.12);
                        preset.additive_partials = normalized(&params.additive_partials, 52.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.3);
                    }
                    41 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 4.2;
                        decay_ms = 44.0;
                        sustain = 0.85;
                        release_ms = 6.4;
                        preset.additive_mix = normalized(&params.additive_mix, 0.16);
                        preset.additive_partials = normalized(&params.additive_partials, 64.0);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.25);
                    }
                    42 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 4.5;
                        decay_ms = 48.0;
                        sustain = 0.8;
                        release_ms = 6.8;
                        preset.additive_mix = normalized(&params.additive_mix, 0.18);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.05);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.25);
                    }
                    43 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2000.0;
                        attack_ms = 3.6;
                        decay_ms = 38.0;
                        sustain = 0.78;
                        release_ms = 5.8;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.35);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.3);
                    }
                    44 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 1800.0;
                        attack_ms = 4.8;
                        decay_ms = 52.0;
                        sustain = 0.9;
                        release_ms = 7.2;
                        preset.additive_partials = normalized(&params.additive_partials, 72.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.2);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.35);
                    }
                    45 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2000.0;
                        attack_ms = 5.0;
                        decay_ms = 55.0;
                        sustain = 0.88;
                        release_ms = 7.5;
                        preset.additive_partials = normalized(&params.additive_partials, 68.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.1);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.4);
                    }
                    46 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2400.0;
                        attack_ms = 3.2;
                        decay_ms = 34.0;
                        sustain = 0.75;
                        release_ms = 5.2;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.45);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.3);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2100.0;
                        attack_ms = 4.2;
                        decay_ms = 46.0;
                        sustain = 0.82;
                        release_ms = 6.2;
                        preset.additive_partials = normalized(&params.additive_partials, 60.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.35);
                    }
                }
            }
            6 => {
                match index {
                    48 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2400.0;
                        attack_ms = 3.2;
                        decay_ms = 36.0;
                        sustain = 0.78;
                        release_ms = 5.4;
                        preset.additive_partials = normalized(&params.additive_partials, 56.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.28);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.26);
                    }
                    49 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 3.6;
                        decay_ms = 40.0;
                        sustain = 0.8;
                        release_ms = 5.8;
                        preset.additive_mix = normalized(&params.additive_mix, 0.14);
                        preset.additive_partials = normalized(&params.additive_partials, 64.0);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.3);
                    }
                    50 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2200.0;
                        attack_ms = 4.0;
                        decay_ms = 44.0;
                        sustain = 0.82;
                        release_ms = 6.2;
                        preset.additive_partials = normalized(&params.additive_partials, 70.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.2);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.26);
                    }
                    51 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2000.0;
                        attack_ms = 4.4;
                        decay_ms = 48.0;
                        sustain = 0.84;
                        release_ms = 6.6;
                        preset.additive_partials = normalized(&params.additive_partials, 74.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.15);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.34);
                    }
                    52 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2600.0;
                        attack_ms = 2.4;
                        decay_ms = 26.0;
                        sustain = 0.7;
                        release_ms = 4.4;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.4);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.3);
                    }
                    53 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2800.0;
                        attack_ms = 2.2;
                        decay_ms = 24.0;
                        sustain = 0.68;
                        release_ms = 4.2;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.5);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.32);
                    }
                    54 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2100.0;
                        attack_ms = 3.8;
                        decay_ms = 42.0;
                        sustain = 0.8;
                        release_ms = 5.9;
                        preset.additive_partials = normalized(&params.additive_partials, 66.0);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.12);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.28);
                    }
                    _ => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2500.0;
                        attack_ms = 2.8;
                        decay_ms = 30.0;
                        sustain = 0.72;
                        release_ms = 4.8;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.45);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.3);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.28);
                    }
                }
            }
            7 => {
                match index {
                    56 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2400.0;
                        attack_ms = 1.6;
                        decay_ms = 18.0;
                        sustain = 0.7;
                        release_ms = 3.6;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Two);
                        preset.unison_detune = normalized(&params.unison_detune, 0.08);
                        preset.unison_spread = normalized(&params.unison_spread, 0.2);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.2);
                    }
                    57 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2200.0;
                        attack_ms = 1.8;
                        decay_ms = 20.0;
                        sustain = 0.72;
                        release_ms = 3.8;
                        preset.filter_amount = normalized(&params.filter_amount, 0.45);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.2);
                    }
                    58 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2800.0;
                        attack_ms = 1.4;
                        decay_ms = 16.0;
                        sustain = 0.65;
                        release_ms = 3.2;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.22);
                        preset.breath_tone = normalized(&params.breath_tone, 2200.0);
                    }
                    59 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 1.5;
                        decay_ms = 18.0;
                        sustain = 0.68;
                        release_ms = 3.4;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.26);
                        preset.breath_tone = normalized(&params.breath_tone, 2400.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.18);
                    }
                    60 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.8;
                        decay_ms = 22.0;
                        sustain = 0.7;
                        release_ms = 3.8;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.28);
                        preset.breath_tone = normalized(&params.breath_tone, 2600.0);
                    }
                    61 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2000.0;
                        attack_ms = 2.0;
                        decay_ms = 24.0;
                        sustain = 0.72;
                        release_ms = 4.0;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.3);
                        preset.breath_tone = normalized(&params.breath_tone, 2500.0);
                    }
                    62 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2100.0;
                        attack_ms = 2.1;
                        decay_ms = 26.0;
                        sustain = 0.74;
                        release_ms = 4.2;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.32);
                        preset.breath_tone = normalized(&params.breath_tone, 2700.0);
                    }
                    _ => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2300.0;
                        attack_ms = 2.3;
                        decay_ms = 28.0;
                        sustain = 0.75;
                        release_ms = 4.5;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.34);
                        preset.breath_tone = normalized(&params.breath_tone, 2800.0);
                    }
                }
            }
            8 => {
                match index {
                    64 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 1.2;
                        decay_ms = 20.0;
                        sustain = 0.65;
                        release_ms = 3.0;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.3);
                        preset.breath_tone = normalized(&params.breath_tone, 2400.0);
                    }
                    65 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2800.0;
                        attack_ms = 1.4;
                        decay_ms = 22.0;
                        sustain = 0.68;
                        release_ms = 3.2;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.32);
                        preset.breath_tone = normalized(&params.breath_tone, 2600.0);
                    }
                    66 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.6;
                        decay_ms = 24.0;
                        sustain = 0.7;
                        release_ms = 3.4;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.34);
                        preset.breath_tone = normalized(&params.breath_tone, 2500.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.18);
                    }
                    67 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2200.0;
                        attack_ms = 1.8;
                        decay_ms = 26.0;
                        sustain = 0.72;
                        release_ms = 3.6;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.36);
                        preset.breath_tone = normalized(&params.breath_tone, 2300.0);
                    }
                    68 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 1.5;
                        decay_ms = 22.0;
                        sustain = 0.68;
                        release_ms = 3.2;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.38);
                        preset.breath_tone = normalized(&params.breath_tone, 2700.0);
                    }
                    69 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.7;
                        decay_ms = 24.0;
                        sustain = 0.7;
                        release_ms = 3.4;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.4);
                        preset.breath_tone = normalized(&params.breath_tone, 2550.0);
                    }
                    70 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2500.0;
                        attack_ms = 1.9;
                        decay_ms = 26.0;
                        sustain = 0.72;
                        release_ms = 3.8;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.42);
                        preset.breath_tone = normalized(&params.breath_tone, 2600.0);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2700.0;
                        attack_ms = 2.0;
                        decay_ms = 28.0;
                        sustain = 0.74;
                        release_ms = 4.0;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.44);
                        preset.breath_tone = normalized(&params.breath_tone, 2800.0);
                    }
                }
            }
            9 => {
                match index {
                    72 => {
                        waveform = Waveform::Sine;
                        filter_cut = 2800.0;
                        attack_ms = 1.4;
                        decay_ms = 20.0;
                        sustain = 0.7;
                        release_ms = 3.2;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.28);
                        preset.breath_tone = normalized(&params.breath_tone, 2300.0);
                    }
                    73 => {
                        waveform = Waveform::Sine;
                        filter_cut = 3000.0;
                        attack_ms = 1.6;
                        decay_ms = 22.0;
                        sustain = 0.72;
                        release_ms = 3.4;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.3);
                        preset.breath_tone = normalized(&params.breath_tone, 2500.0);
                    }
                    74 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 1.8;
                        decay_ms = 24.0;
                        sustain = 0.74;
                        release_ms = 3.6;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.32);
                        preset.breath_tone = normalized(&params.breath_tone, 2400.0);
                    }
                    75 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 2.0;
                        decay_ms = 26.0;
                        sustain = 0.76;
                        release_ms = 3.8;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.34);
                        preset.breath_tone = normalized(&params.breath_tone, 2200.0);
                    }
                    76 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2800.0;
                        attack_ms = 1.5;
                        decay_ms = 22.0;
                        sustain = 0.72;
                        release_ms = 3.4;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.36);
                        preset.breath_tone = normalized(&params.breath_tone, 2700.0);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.18);
                    }
                    77 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.9;
                        decay_ms = 26.0;
                        sustain = 0.78;
                        release_ms = 4.0;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.38);
                        preset.breath_tone = normalized(&params.breath_tone, 2500.0);
                    }
                    78 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 2.1;
                        decay_ms = 28.0;
                        sustain = 0.8;
                        release_ms = 4.2;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.4);
                        preset.breath_tone = normalized(&params.breath_tone, 2600.0);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3000.0;
                        attack_ms = 2.2;
                        decay_ms = 30.0;
                        sustain = 0.82;
                        release_ms = 4.4;
                        preset.breath_enable = normalized(&params.breath_enable, true);
                        preset.breath_amount = normalized(&params.breath_amount, 0.42);
                        preset.breath_tone = normalized(&params.breath_tone, 2800.0);
                    }
                }
            }
            10 => {
                waveform = if index % 2 == 0 {
                    Waveform::Sawtooth
                } else {
                    Waveform::Pulse
                };
                filter_cut = 1600.0;
                attack_ms = 0.9;
                decay_ms = 20.0;
                sustain = 0.6;
                release_ms = 3.2;
                preset.unison_voices = normalized(&params.unison_voices, UnisonVoices::Two);
                preset.unison_detune = normalized(&params.unison_detune, 0.18);
                preset.unison_spread = normalized(&params.unison_spread, 0.3);
                match index {
                    80 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2200.0;
                        attack_ms = 0.7;
                        decay_ms = 16.0;
                        sustain = 0.55;
                        release_ms = 2.8;
                        preset.unison_detune = normalized(&params.unison_detune, 0.14);
                    }
                    81 => {
                        waveform = Waveform::Pulse;
                        filter_cut = 1800.0;
                        attack_ms = 0.6;
                        decay_ms = 14.0;
                        sustain = 0.5;
                        release_ms = 2.6;
                        preset.unison_detune = normalized(&params.unison_detune, 0.16);
                        preset.unison_spread = normalized(&params.unison_spread, 0.35);
                    }
                    82 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2600.0;
                        attack_ms = 0.5;
                        decay_ms = 12.0;
                        sustain = 0.48;
                        release_ms = 2.4;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Four);
                        preset.unison_detune = normalized(&params.unison_detune, 0.2);
                    }
                    83 => {
                        waveform = Waveform::Pulse;
                        filter_cut = 2000.0;
                        attack_ms = 0.5;
                        decay_ms = 10.0;
                        sustain = 0.45;
                        release_ms = 2.2;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Four);
                        preset.unison_detune = normalized(&params.unison_detune, 0.22);
                        preset.unison_spread = normalized(&params.unison_spread, 0.4);
                    }
                    84 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2400.0;
                        attack_ms = 0.8;
                        decay_ms = 18.0;
                        sustain = 0.6;
                        release_ms = 3.0;
                        preset.unison_detune = normalized(&params.unison_detune, 0.18);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.22);
                    }
                    85 => {
                        waveform = Waveform::Pulse;
                        filter_cut = 1900.0;
                        attack_ms = 0.7;
                        decay_ms = 15.0;
                        sustain = 0.55;
                        release_ms = 2.9;
                        preset.unison_detune = normalized(&params.unison_detune, 0.2);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.2);
                    }
                    86 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 2800.0;
                        attack_ms = 0.6;
                        decay_ms = 13.0;
                        sustain = 0.5;
                        release_ms = 2.6;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Six);
                        preset.unison_detune = normalized(&params.unison_detune, 0.24);
                        preset.unison_spread = normalized(&params.unison_spread, 0.45);
                    }
                    _ => {
                        waveform = Waveform::Pulse;
                        filter_cut = 2100.0;
                        attack_ms = 0.9;
                        decay_ms = 20.0;
                        sustain = 0.62;
                        release_ms = 3.2;
                        preset.unison_detune = normalized(&params.unison_detune, 0.18);
                        preset.unison_spread = normalized(&params.unison_spread, 0.3);
                    }
                }
            }
            11 => {
                match index {
                    88 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 1600.0;
                        attack_ms = 5.8;
                        decay_ms = 44.0;
                        sustain = 0.82;
                        release_ms = 7.2;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Four);
                        preset.unison_detune = normalized(&params.unison_detune, 0.18);
                        preset.unison_spread = normalized(&params.unison_spread, 0.35);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.42);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.35);
                    }
                    89 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 1800.0;
                        attack_ms = 7.0;
                        decay_ms = 50.0;
                        sustain = 0.86;
                        release_ms = 8.2;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Two);
                        preset.unison_detune = normalized(&params.unison_detune, 0.12);
                        preset.unison_spread = normalized(&params.unison_spread, 0.3);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.35);
                    }
                    90 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2000.0;
                        attack_ms = 6.2;
                        decay_ms = 46.0;
                        sustain = 0.84;
                        release_ms = 7.6;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.35);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.4);
                    }
                    91 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 1500.0;
                        attack_ms = 7.5;
                        decay_ms = 52.0;
                        sustain = 0.88;
                        release_ms = 8.6;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Six);
                        preset.unison_detune = normalized(&params.unison_detune, 0.24);
                        preset.unison_spread = normalized(&params.unison_spread, 0.45);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.5);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.45);
                    }
                    92 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 1700.0;
                        attack_ms = 6.8;
                        decay_ms = 48.0;
                        sustain = 0.86;
                        release_ms = 8.0;
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.38);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.36);
                    }
                    93 => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 1400.0;
                        attack_ms = 7.2;
                        decay_ms = 54.0;
                        sustain = 0.9;
                        release_ms = 9.0;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Four);
                        preset.unison_detune = normalized(&params.unison_detune, 0.2);
                        preset.unison_spread = normalized(&params.unison_spread, 0.4);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.5);
                    }
                    94 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2200.0;
                        attack_ms = 5.6;
                        decay_ms = 42.0;
                        sustain = 0.8;
                        release_ms = 7.0;
                        preset.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                        preset.osc_blend = normalized(&params.osc_blend, 0.4);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.32);
                    }
                    _ => {
                        waveform = Waveform::Sawtooth;
                        filter_cut = 1600.0;
                        attack_ms = 6.0;
                        decay_ms = 46.0;
                        sustain = 0.84;
                        release_ms = 7.4;
                        preset.unison_voices =
                            normalized(&params.unison_voices, UnisonVoices::Four);
                        preset.unison_detune = normalized(&params.unison_detune, 0.22);
                        preset.unison_spread = normalized(&params.unison_spread, 0.4);
                        preset.chorus_enable = normalized(&params.chorus_enable, true);
                        preset.chorus_mix = normalized(&params.chorus_mix, 0.45);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.4);
                    }
                }
            }
            12 => {
                match index {
                    96 => {
                        waveform = Waveform::Noise;
                        filter_cut = 1800.0;
                        attack_ms = 1.8;
                        decay_ms = 18.0;
                        sustain = 0.35;
                        release_ms = 3.2;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::CombAllpass);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.35);
                        preset.delay_mix = normalized(&params.delay_mix, 0.2);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.3);
                    }
                    97 => {
                        waveform = Waveform::Noise;
                        filter_cut = 1400.0;
                        attack_ms = 2.2;
                        decay_ms = 20.0;
                        sustain = 0.3;
                        release_ms = 3.6;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::Phaser);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.3);
                        preset.delay_mix = normalized(&params.delay_mix, 0.22);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.35);
                    }
                    98 => {
                        waveform = Waveform::Noise;
                        filter_cut = 2000.0;
                        attack_ms = 1.4;
                        decay_ms = 16.0;
                        sustain = 0.28;
                        release_ms = 3.0;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::Comb);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.4);
                        preset.delay_mix = normalized(&params.delay_mix, 0.18);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.28);
                    }
                    99 => {
                        waveform = Waveform::Noise;
                        filter_cut = 1200.0;
                        attack_ms = 2.8;
                        decay_ms = 24.0;
                        sustain = 0.25;
                        release_ms = 4.2;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::BitcrushLp);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.45);
                        preset.delay_mix = normalized(&params.delay_mix, 0.25);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.38);
                    }
                    100 => {
                        waveform = Waveform::Noise;
                        filter_cut = 2200.0;
                        attack_ms = 1.6;
                        decay_ms = 18.0;
                        sustain = 0.3;
                        release_ms = 3.2;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::FormantMorph);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.35);
                        preset.delay_mix = normalized(&params.delay_mix, 0.2);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.35);
                    }
                    101 => {
                        waveform = Waveform::Noise;
                        filter_cut = 1500.0;
                        attack_ms = 2.4;
                        decay_ms = 22.0;
                        sustain = 0.28;
                        release_ms = 3.8;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::CombAllpass);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.38);
                        preset.delay_mix = normalized(&params.delay_mix, 0.22);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.4);
                    }
                    102 => {
                        waveform = Waveform::Noise;
                        filter_cut = 1800.0;
                        attack_ms = 1.9;
                        decay_ms = 20.0;
                        sustain = 0.32;
                        release_ms = 3.4;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::RainbowComb);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.42);
                        preset.delay_mix = normalized(&params.delay_mix, 0.2);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.36);
                    }
                    _ => {
                        waveform = Waveform::Noise;
                        filter_cut = 1600.0;
                        attack_ms = 2.5;
                        decay_ms = 28.0;
                        sustain = 0.35;
                        release_ms = 4.5;
                        preset.filter_type =
                            normalized(&params.filter_type, FilterType::CombAllpass);
                        preset.delay_enable = normalized(&params.delay_enable, true);
                        preset.delay_feedback = normalized(&params.delay_feedback, 0.4);
                        preset.delay_mix = normalized(&params.delay_mix, 0.25);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.4);
                    }
                }
            }
            13 => {
                match index {
                    104 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 1.6;
                        decay_ms = 24.0;
                        sustain = 0.42;
                        release_ms = 4.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.14);
                        preset.additive_partials = normalized(&params.additive_partials, 40.0);
                    }
                    105 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3000.0;
                        attack_ms = 1.4;
                        decay_ms = 22.0;
                        sustain = 0.4;
                        release_ms = 3.8;
                        preset.additive_mix = normalized(&params.additive_mix, 0.12);
                        preset.additive_tilt = normalized(&params.additive_tilt, 0.1);
                    }
                    106 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.8;
                        decay_ms = 26.0;
                        sustain = 0.45;
                        release_ms = 4.2;
                        preset.additive_mix = normalized(&params.additive_mix, 0.16);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.06);
                    }
                    107 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2200.0;
                        attack_ms = 2.0;
                        decay_ms = 28.0;
                        sustain = 0.48;
                        release_ms = 4.4;
                        preset.additive_mix = normalized(&params.additive_mix, 0.18);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.08);
                    }
                    108 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 1.7;
                        decay_ms = 24.0;
                        sustain = 0.44;
                        release_ms = 4.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.15);
                        preset.additive_partials = normalized(&params.additive_partials, 36.0);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.2);
                    }
                    109 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2800.0;
                        attack_ms = 1.5;
                        decay_ms = 22.0;
                        sustain = 0.42;
                        release_ms = 3.8;
                        preset.additive_mix = normalized(&params.additive_mix, 0.14);
                        preset.additive_tilt = normalized(&params.additive_tilt, -0.1);
                        preset.reverb_enable = normalized(&params.reverb_enable, true);
                        preset.reverb_mix = normalized(&params.reverb_mix, 0.22);
                    }
                    110 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2400.0;
                        attack_ms = 1.9;
                        decay_ms = 28.0;
                        sustain = 0.46;
                        release_ms = 4.2;
                        preset.additive_mix = normalized(&params.additive_mix, 0.16);
                        preset.additive_partials = normalized(&params.additive_partials, 44.0);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2300.0;
                        attack_ms = 2.0;
                        decay_ms = 30.0;
                        sustain = 0.48;
                        release_ms = 4.6;
                        preset.additive_mix = normalized(&params.additive_mix, 0.18);
                        preset.additive_partials = normalized(&params.additive_partials, 48.0);
                    }
                }
            }
            14 => {
                match index {
                    112 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3600.0;
                        attack_ms = 0.2;
                        decay_ms = 8.0;
                        sustain = 0.0;
                        release_ms = 1.2;
                        preset.additive_mix = normalized(&params.additive_mix, 0.12);
                    }
                    113 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3200.0;
                        attack_ms = 0.25;
                        decay_ms = 10.0;
                        sustain = 0.0;
                        release_ms = 1.4;
                        preset.additive_mix = normalized(&params.additive_mix, 0.14);
                    }
                    114 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3000.0;
                        attack_ms = 0.3;
                        decay_ms = 12.0;
                        sustain = 0.0;
                        release_ms = 1.6;
                        preset.additive_mix = normalized(&params.additive_mix, 0.16);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.08);
                    }
                    115 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2800.0;
                        attack_ms = 0.35;
                        decay_ms = 14.0;
                        sustain = 0.0;
                        release_ms = 1.8;
                        preset.additive_mix = normalized(&params.additive_mix, 0.18);
                        preset.additive_inharm = normalized(&params.additive_inharm, 0.1);
                    }
                    116 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3400.0;
                        attack_ms = 0.28;
                        decay_ms = 11.0;
                        sustain = 0.0;
                        release_ms = 1.5;
                        preset.additive_mix = normalized(&params.additive_mix, 0.2);
                        preset.additive_partials = normalized(&params.additive_partials, 36.0);
                    }
                    117 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 2600.0;
                        attack_ms = 0.4;
                        decay_ms = 16.0;
                        sustain = 0.0;
                        release_ms = 2.0;
                        preset.additive_mix = normalized(&params.additive_mix, 0.22);
                        preset.additive_partials = normalized(&params.additive_partials, 40.0);
                    }
                    118 => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3600.0;
                        attack_ms = 0.22;
                        decay_ms = 9.0;
                        sustain = 0.0;
                        release_ms = 1.3;
                        preset.additive_mix = normalized(&params.additive_mix, 0.2);
                        preset.additive_partials = normalized(&params.additive_partials, 32.0);
                    }
                    _ => {
                        waveform = Waveform::Triangle;
                        filter_cut = 3000.0;
                        attack_ms = 0.32;
                        decay_ms = 13.0;
                        sustain = 0.0;
                        release_ms = 1.7;
                        preset.additive_mix = normalized(&params.additive_mix, 0.24);
                        preset.additive_partials = normalized(&params.additive_partials, 44.0);
                    }
                }
            }
                _ => {
                    match index {
                        120 => {
                            waveform = Waveform::Noise;
                            filter_cut = 1400.0;
                            attack_ms = 0.6;
                            decay_ms = 12.0;
                            sustain = 0.2;
                            release_ms = 2.6;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::Bandpass);
                            preset.delay_enable = normalized(&params.delay_enable, true);
                            preset.delay_mix = normalized(&params.delay_mix, 0.18);
                        }
                        121 => {
                            waveform = Waveform::Noise;
                            filter_cut = 1200.0;
                            attack_ms = 0.8;
                            decay_ms = 14.0;
                            sustain = 0.22;
                            release_ms = 2.8;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::CombAllpass);
                            preset.reverb_enable = normalized(&params.reverb_enable, true);
                            preset.reverb_mix = normalized(&params.reverb_mix, 0.25);
                        }
                        122 => {
                            waveform = Waveform::Noise;
                            filter_cut = 1800.0;
                            attack_ms = 0.5;
                            decay_ms = 10.0;
                            sustain = 0.18;
                            release_ms = 2.4;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::BitcrushLp);
                            preset.delay_enable = normalized(&params.delay_enable, true);
                            preset.delay_feedback = normalized(&params.delay_feedback, 0.35);
                            preset.delay_mix = normalized(&params.delay_mix, 0.22);
                        }
                        123 => {
                            waveform = Waveform::Noise;
                            filter_cut = 1500.0;
                            attack_ms = 1.0;
                            decay_ms = 16.0;
                            sustain = 0.25;
                            release_ms = 3.0;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::Phaser);
                            preset.reverb_enable = normalized(&params.reverb_enable, true);
                            preset.reverb_mix = normalized(&params.reverb_mix, 0.3);
                        }
                        124 => {
                            waveform = Waveform::Noise;
                            filter_cut = 2000.0;
                            attack_ms = 0.7;
                            decay_ms = 12.0;
                            sustain = 0.2;
                            release_ms = 2.6;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::Comb);
                            preset.delay_enable = normalized(&params.delay_enable, true);
                            preset.delay_feedback = normalized(&params.delay_feedback, 0.4);
                            preset.delay_mix = normalized(&params.delay_mix, 0.2);
                        }
                        125 => {
                            waveform = Waveform::Noise;
                            filter_cut = 1300.0;
                            attack_ms = 1.1;
                            decay_ms = 18.0;
                            sustain = 0.28;
                            release_ms = 3.4;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::FormantMorph);
                            preset.reverb_enable = normalized(&params.reverb_enable, true);
                            preset.reverb_mix = normalized(&params.reverb_mix, 0.35);
                        }
                        126 => {
                            waveform = Waveform::Noise;
                            filter_cut = 1700.0;
                            attack_ms = 0.6;
                            decay_ms = 11.0;
                            sustain = 0.2;
                            release_ms = 2.5;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::RainbowComb);
                            preset.delay_enable = normalized(&params.delay_enable, true);
                            preset.delay_feedback = normalized(&params.delay_feedback, 0.3);
                            preset.delay_mix = normalized(&params.delay_mix, 0.18);
                        }
                        _ => {
                            waveform = Waveform::Noise;
                            filter_cut = 1200.0;
                            attack_ms = 1.2;
                            decay_ms = 24.0;
                            sustain = 0.3;
                            release_ms = 4.0;
                            preset.filter_type =
                                normalized(&params.filter_type, FilterType::Statevariable);
                            preset.delay_enable = normalized(&params.delay_enable, true);
                            preset.delay_mix = normalized(&params.delay_mix, 0.22);
                            preset.reverb_enable = normalized(&params.reverb_enable, true);
                            preset.reverb_mix = normalized(&params.reverb_mix, 0.35);
                        }
                    }
                }
            }
        }

        let (attack_ms, decay_ms, sustain, release_ms) =
            apply_category_envelope(category, attack_ms, decay_ms, sustain, release_ms);
        let attack_ms = (attack_ms + variant * 0.12).clamp(0.1, 10.0);
        let release_ms = (release_ms + variant * 0.15).clamp(0.1, 10.0);
        let decay_ms = (decay_ms + variant * 1.2).clamp(1.0, 100.0);
        let sustain = (sustain + variant * 0.02).clamp(0.0, 1.0);

        preset.waveform = normalized(&params.waveform, waveform);
        preset.filter_cut = normalized(&params.filter_cut, filter_cut);
        preset.amp_attack_ms = normalized(&params.amp_attack_ms, attack_ms);
        preset.amp_decay_ms = normalized(&params.amp_decay_ms, decay_ms);
        preset.amp_sustain_level = normalized(&params.amp_sustain_level, sustain);
        preset.amp_release_ms = normalized(&params.amp_release_ms, release_ms);

        if category >= 10 && category <= 12 || name_lower.contains("synth") {
            preset.analog_enable = normalized(&params.analog_enable, true);
            preset.analog_drive = normalized(&params.analog_drive, 0.2);
            preset.analog_drift = normalized(&params.analog_drift, 0.12);
        }

        if name_lower.contains("distortion") || name_lower.contains("overdriven") {
            preset.dist_enable = normalized(&params.dist_enable, true);
            preset.dist_drive = normalized(&params.dist_drive, 0.35);
            preset.dist_mix = normalized(&params.dist_mix, 0.35);
        }

        preset.analog_enable = normalized(&params.analog_enable, false);
        preset.dist_enable = normalized(&params.dist_enable, false);
        preset.dist_drive = normalized(&params.dist_drive, 0.0);
        preset.dist_mix = normalized(&params.dist_mix, 0.0);

        presets.push(PresetEntry {
            name: format!("GM: {:03} {}", index + 1, name),
            data: preset,
            user: false,
            category_index: category,
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
            |cx| Label::new(cx, "Presets"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(1)),
            |cx| Label::new(cx, "Osc + WT"),
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

        VStack::new(cx, |cx| {
            Label::new(cx, "Breath")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.breath_enable)
                .with_label("")
                .width(Pixels(70.0))
                .height(Pixels(30.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.breath_amount)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.breath_attack_ms)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.breath_decay_ms)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.breath_tone)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Velocity")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.vel_additive_amount)
                .with_label("Vel Add");
        })
        .row_between(Pixels(12.0));
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
            create_label(cx, "Decay", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.amp_decay_ms)
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
                |params| &params.fm_env_decay_ms,
                |params| &params.fm_env_sustain_level,
                |params| &params.fm_env_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            HStack::new(cx, |cx| {
                labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.fm_env_attack_ms);
                labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.fm_env_decay_ms);
                labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.fm_env_sustain_level);
                labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.fm_env_release_ms);
                labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.fm_env_amount);
            })
            .col_between(Pixels(6.0));

            Label::new(cx, "Filter Cut Env")
                .height(Pixels(16.0))
                .width(Pixels(120.0))
                .child_top(Pixels(8.0));
            EnvelopeDisplay::new(
                cx,
                Data::params.clone(),
                |params| &params.filter_cut_attack_ms,
                |params| &params.filter_cut_decay_ms,
                |params| &params.filter_cut_sustain_ms,
                |params| &params.filter_cut_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            HStack::new(cx, |cx| {
                labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.filter_cut_attack_ms);
                labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.filter_cut_decay_ms);
                labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.filter_cut_sustain_ms);
                labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.filter_cut_release_ms);
                labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.filter_cut_envelope_level);
            })
            .col_between(Pixels(6.0));
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
                |params| &params.dist_env_decay_ms,
                |params| &params.dist_env_sustain_level,
                |params| &params.dist_env_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            HStack::new(cx, |cx| {
                labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.dist_env_attack_ms);
                labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.dist_env_decay_ms);
                labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.dist_env_sustain_level);
                labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.dist_env_release_ms);
                labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.dist_env_amount);
            })
            .col_between(Pixels(6.0));

            Label::new(cx, "Filter Res Env")
                .height(Pixels(16.0))
                .width(Pixels(120.0))
                .child_top(Pixels(8.0));
            EnvelopeDisplay::new(
                cx,
                Data::params.clone(),
                |params| &params.filter_res_attack_ms,
                |params| &params.filter_res_decay_ms,
                |params| &params.filter_res_sustain_ms,
                |params| &params.filter_res_release_ms,
            )
            .width(Stretch(1.0))
            .height(Pixels(90.0));
            HStack::new(cx, |cx| {
                labeled_knob(cx, "Atk", Data::params.clone(), |params| &params.filter_res_attack_ms);
                labeled_knob(cx, "Dec", Data::params.clone(), |params| &params.filter_res_decay_ms);
                labeled_knob(cx, "Sus", Data::params.clone(), |params| &params.filter_res_sustain_ms);
                labeled_knob(cx, "Rel", Data::params.clone(), |params| &params.filter_res_release_ms);
                labeled_knob(cx, "Amt", Data::params.clone(), |params| &params.filter_res_envelope_level);
            })
            .col_between(Pixels(6.0));

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

fn build_preset_tab(cx: &mut Context) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, "Presets")
                .height(Pixels(18.0))
                .width(Pixels(80.0));
            Label::new(cx, Data::preset_display)
                .height(Pixels(18.0))
                .width(Pixels(220.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::PresetLoad),
                |cx| Label::new(cx, "Load"),
            )
            .height(Pixels(22.0));
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::PresetSave),
                |cx| Label::new(cx, "Save"),
            )
            .height(Pixels(22.0));
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::PresetRefresh),
                |cx| Label::new(cx, "Refresh"),
            )
            .height(Pixels(22.0));
        })
        .col_between(Pixels(8.0))
        .row_between(Pixels(6.0))
        .height(Pixels(24.0));
        HStack::new(cx, |cx| {
            Label::new(cx, "Morph")
                .height(Pixels(18.0))
                .width(Pixels(60.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.morph_amount)
                .width(Pixels(220.0))
                .height(Pixels(22.0));
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::MorphStoreA),
                |cx| Label::new(cx, "Store A"),
            )
            .height(Pixels(22.0));
            Button::new(
                cx,
                |cx| cx.emit(UiEvent::MorphStoreB),
                |cx| Label::new(cx, "Store B"),
            )
            .height(Pixels(22.0));
        })
        .col_between(Pixels(8.0))
        .row_between(Pixels(6.0))
        .height(Pixels(24.0));
        HStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                Label::new(cx, "Categories")
                    .height(Pixels(18.0))
                    .width(Stretch(1.0));
                let mut categories = Vec::from(GM_CATEGORY_NAMES);
                categories.push("User");
                ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                    Binding::new(cx, Data::preset_category, move |cx, active| {
                        VStack::new(cx, |cx| {
                            let active_index = active.get(cx);
                            for (index, name) in categories.iter().enumerate() {
                                let label = *name;
                                let mut button = Button::new(
                                    cx,
                                    move |cx| cx.emit(UiEvent::PresetCategorySelect(index)),
                                    move |cx| Label::new(cx, label),
                                );
                                button = button.class("preset-category");
                                if index == active_index {
                                    button = button.class("preset-selected");
                                }
                                button.height(Pixels(24.0)).width(Stretch(1.0));
                            }
                        })
                        .row_between(Pixels(6.0))
                        .width(Stretch(1.0));
                    });
                })
                .height(Stretch(1.0))
                .width(Stretch(1.0));
            })
            .row_between(Pixels(6.0))
            .width(Pixels(180.0));

            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "Search")
                        .height(Pixels(18.0))
                        .width(Pixels(60.0));
                    Textbox::new(cx, Data::preset_filter)
                        .width(Pixels(240.0))
                        .height(Pixels(24.0))
                        .on_edit(|cx, text| cx.emit(UiEvent::PresetFilterChanged(text.clone())))
                        .on_submit(|cx, text, _| cx.emit(UiEvent::PresetFilterChanged(text)));
                    Button::new(
                        cx,
                        |cx| cx.emit(UiEvent::PresetFilterChanged(String::new())),
                        |cx| Label::new(cx, "Clear"),
                    )
                    .height(Pixels(22.0));
                })
                .col_between(Pixels(8.0))
                .row_between(Pixels(6.0))
                .height(Pixels(24.0));

                ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                    Binding::new(cx, Data::preset_category, move |cx, category| {
                        Binding::new(cx, Data::preset_filter, move |cx, filter| {
                            Binding::new(cx, Data::presets, move |cx, presets| {
                                Binding::new(cx, Data::preset_index, move |cx, preset_index| {
                                    VStack::new(cx, |cx| {
                                        let filter_text = filter.get(cx);
                                        let category_index = category.get(cx);
                                        let presets = presets.get(cx);
                                        let selected_index = preset_index.get(cx);
                                        let mut visible = Vec::new();
                                        let mut total_in_category = 0usize;

                                        for (index, preset) in presets.iter().enumerate() {
                                            let category_match = if category_index == PRESET_CATEGORY_USER {
                                                preset.user
                                            } else {
                                                !preset.user && preset.category_index == category_index
                                            };

                                            if category_match {
                                                total_in_category += 1;
                                            }

                                            if category_match && preset_filter_match(&preset.name, &filter_text) {
                                                visible.push((index, preset));
                                            }
                                        }

                                        let patch_label = format!(
                                            "Patches ({}/{})",
                                            visible.len(),
                                            total_in_category
                                        );
                                        Label::new(cx, &patch_label)
                                            .height(Pixels(18.0))
                                            .width(Stretch(1.0));

                                        for (index, preset) in visible.iter() {
                                            let name = preset.name.clone();
                                            let preset_index = *index;
                                            let mut button = Button::new(
                                                cx,
                                                move |cx| cx.emit(UiEvent::PresetSelect(preset_index, true)),
                                                move |cx| Label::new(cx, name.as_str()),
                                            );
                                            button = button.class("preset-button");
                                            if preset_index == selected_index {
                                                button = button.class("preset-selected");
                                            }
                                            button.height(Pixels(24.0)).width(Stretch(1.0));
                                        }

                                        if visible.is_empty() {
                                            Label::new(cx, "No presets")
                                                .height(Pixels(18.0))
                                                .width(Stretch(1.0));
                                        }
                                    })
                                    .row_between(Pixels(6.0))
                                    .width(Stretch(1.0));
                                });
                            });
                        });
                    });
                })
                .height(Stretch(1.0))
                .width(Stretch(1.0));
            })
            .row_between(Pixels(10.0))
            .width(Stretch(1.0));
        })
        .col_between(Pixels(12.0))
        .height(Stretch(1.0))
        .width(Stretch(1.0));
    })
    .row_between(Pixels(10.0))
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
    Decay,
    Sustain,
    Release,
}

#[derive(Lens)]
struct EnvelopeDisplay {
    attack: ParamWidgetBase,
    decay: ParamWidgetBase,
    sustain: ParamWidgetBase,
    release: ParamWidgetBase,
    dragging: Option<EnvelopeDrag>,
}

impl EnvelopeDisplay {
    pub fn new<L, Params, P, FAtk, FDec, FSus, FRel>(
        cx: &mut Context,
        params: L,
        attack: FAtk,
        decay: FDec,
        sustain: FSus,
        release: FRel,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FAtk: Fn(&Params) -> &P + Copy + 'static,
        FDec: Fn(&Params) -> &P + Copy + 'static,
        FSus: Fn(&Params) -> &P + Copy + 'static,
        FRel: Fn(&Params) -> &P + Copy + 'static,
    {
        Self {
            attack: ParamWidgetBase::new(cx, params.clone(), attack),
            decay: ParamWidgetBase::new(cx, params.clone(), decay),
            sustain: ParamWidgetBase::new(cx, params.clone(), sustain),
            release: ParamWidgetBase::new(cx, params, release),
            dragging: None,
        }
        .build(cx, |_| {})
    }

    fn handle_positions(&self, bounds: BoundingBox) -> [(f32, f32); 4] {
        let attack = self.attack.unmodulated_normalized_value().clamp(0.0, 1.0);
        let decay = self.decay.unmodulated_normalized_value().clamp(0.0, 1.0);
        let sustain = self.sustain.unmodulated_normalized_value().clamp(0.0, 1.0);
        let release = self.release.unmodulated_normalized_value().clamp(0.0, 1.0);

        let x_attack = bounds.x + bounds.w * (0.3 * attack);
        let x_decay = bounds.x + bounds.w * (0.3 + 0.3 * decay);
        let x_sustain = bounds.x + bounds.w * 0.7;
        let x_release = bounds.x + bounds.w * (0.8 + 0.2 * release);
        let y_sustain = bounds.y + bounds.h * (1.0 - sustain);
        let y_top = bounds.y;
        let y_bottom = bounds.y + bounds.h;

        [(x_attack, y_top), (x_decay, y_sustain), (x_sustain, y_sustain), (x_release, y_bottom)]
    }

    fn update_from_drag(&mut self, cx: &mut EventContext, bounds: BoundingBox, x: f32, y: f32) {
        let x_norm = ((x - bounds.x) / bounds.w).clamp(0.0, 1.0);
        let y_norm = ((y - bounds.y) / bounds.h).clamp(0.0, 1.0);
        let sustain = (1.0 - y_norm).clamp(0.0, 1.0);

        match self.dragging {
            Some(EnvelopeDrag::Attack) => {
                let attack = (x_norm / 0.3).clamp(0.0, 1.0);
                self.attack.set_normalized_value(cx, attack);
            }
            Some(EnvelopeDrag::Decay) => {
                let decay = ((x_norm - 0.3) / 0.3).clamp(0.0, 1.0);
                self.decay.set_normalized_value(cx, decay);
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
                        Some(1) => Some(EnvelopeDrag::Decay),
                        Some(2) => Some(EnvelopeDrag::Sustain),
                        Some(3) => Some(EnvelopeDrag::Release),
                        _ => None,
                    };

                    if self.dragging.is_some() {
                        self.attack.begin_set_parameter(cx);
                        self.decay.begin_set_parameter(cx);
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
                    self.decay.end_set_parameter(cx);
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
        let decay_x = handles[1].0;
        let sustain_x = bounds.x + bounds.w * 0.6;
        let sustain_end = bounds.x + bounds.w * 0.8;
        let release_x = handles[3].0;
        let top = bounds.y;
        let bottom = bounds.y + bounds.h;
        let sustain_y = handles[1].1;

        let mut path = vg::Path::new();
        path.move_to(bounds.x, bottom);
        path.line_to(attack_x, top);
        path.line_to(decay_x, sustain_y);
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
        let default_index = presets
            .iter()
            .position(|preset| preset.name == "GM: 001 Acoustic Grand Piano")
            .unwrap_or(0);
        let preset_display = presets
            .get(default_index)
            .map(|preset| preset.name.clone())
            .unwrap_or_else(|| "Init".to_string());
        let preset_category = presets
            .get(default_index)
            .map(|preset| preset.category_index)
            .unwrap_or(0);
        let seq_preset_display = seq_preset_name(0).to_string();

        Data {
            params: params.clone(),
            active_tab: 0,
            presets,
            preset_index: default_index,
            preset_display,
            preset_name: String::new(),
            preset_filter: String::new(),
            preset_category,
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
        HStack::new(cx, |cx| {
            Label::new(cx, "MiceSynth")
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
        .width(Stretch(0.4))
        .child_top(Stretch(1.0))
        .child_bottom(Pixels(0.0));
        build_preset_bar(cx);
        build_tab_bar(cx);

        Binding::new(cx, Data::active_tab, |cx, tab| match tab.get(cx) {
            0 => build_preset_tab(cx),
            1 => build_osc_tab(cx),
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

        // レイアウトを初期化時にリセットする
        cx.emit(GuiContextEvent::Resize);
    })
}
                
