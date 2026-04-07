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
use std::collections::HashMap;
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
    util,
    waveform::load_wavetable_from_file,
    FilterType,
    GlideMode,
    ModSource,
    ModTarget,
    OscRouting,
    OscillatorShape,
    SubSynthParams,
    Waveform,
    DrumSynthParams,
};
use crate::drum_model::{
    default_instrument_for_slot,
    default_note_for_slot,
    instrument_label,
    ExciterType,
    InstrumentType,
    MaterialType,
    NoiseType,
    ResonatorType,
};
use crate::drum_params::{DrumSlotParams, DRUM_SLOTS, DRUM_STEPS};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PresetData {
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
    custom_wavetable_enable: f32,
    analog_enable: f32,
    analog_drive: f32,
    analog_noise: f32,
    analog_drift: f32,
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
        PathBuf::from(appdata).join("CatSynth").join("Presets")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".catsynth").join("presets")
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
            custom_wavetable_enable: params.custom_wavetable_enable.unmodulated_normalized_value(),
            analog_enable: params.analog_enable.unmodulated_normalized_value(),
            analog_drive: params.analog_drive.unmodulated_normalized_value(),
            analog_noise: params.analog_noise.unmodulated_normalized_value(),
            analog_drift: params.analog_drift.unmodulated_normalized_value(),
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
        apply_param(cx, &params.custom_wavetable_enable, self.custom_wavetable_enable);
        apply_param(cx, &params.analog_enable, self.analog_enable);
        apply_param(cx, &params.analog_drive, self.analog_drive);
        apply_param(cx, &params.analog_noise, self.analog_noise);
        apply_param(cx, &params.analog_drift, self.analog_drift);
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
}

fn apply_param<P: Param>(cx: &mut EventContext, param: &P, normalized: f32) {
    cx.emit(ParamEvent::BeginSetParameter(param).upcast());
    cx.emit(ParamEvent::SetParameterNormalized(param, normalized).upcast());
    cx.emit(ParamEvent::EndSetParameter(param).upcast());
}

fn factory_presets(params: &SubSynthParams) -> Vec<PresetEntry> {
    let mut presets = Vec::new();
    let default_gain = normalized(&params.gain, util::db_to_gain(-12.0));
    let apply_audible_gain = |preset: &mut PresetData| {
        preset.gain = default_gain;
    };

    let mut default = PresetData::from_params(params);
    default.waveform = normalized(&params.waveform, Waveform::Sawtooth);
    default.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
    default.osc_blend = normalized(&params.osc_blend, 0.0);
    default.wavetable_position = normalized(&params.wavetable_position, 0.5);
    default.wavetable_distortion = normalized(&params.wavetable_distortion, 0.0);
    default.classic_drive = normalized(&params.classic_drive, 0.25);
    default.sub_level = normalized(&params.sub_level, 0.0);
    default.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    default.filter_cut = normalized(&params.filter_cut, 320.0);
    default.filter_res = normalized(&params.filter_res, 0.85);
    default.filter_amount = normalized(&params.filter_amount, 1.0);
    default.amp_attack_ms = normalized(&params.amp_attack_ms, 0.2);
    default.amp_decay_ms = normalized(&params.amp_decay_ms, 40.0);
    default.amp_sustain_level = normalized(&params.amp_sustain_level, 0.0);
    default.amp_release_ms = normalized(&params.amp_release_ms, 60.0);
    default.filter_cut_attack_ms = normalized(&params.filter_cut_attack_ms, 0.2);
    default.filter_cut_decay_ms = normalized(&params.filter_cut_decay_ms, 60.0);
    default.filter_cut_sustain_ms = normalized(&params.filter_cut_sustain_ms, 0.0);
    default.filter_cut_release_ms = normalized(&params.filter_cut_release_ms, 80.0);
    default.filter_cut_envelope_level = normalized(&params.filter_cut_envelope_level, 0.9);
    default.filter_res_envelope_level = normalized(&params.filter_res_envelope_level, 0.2);
    default.glide_mode = normalized(&params.glide_mode, GlideMode::Legato);
    default.glide_time_ms = normalized(&params.glide_time_ms, 80.0);
    default.dist_enable = normalized(&params.dist_enable, true);
    default.dist_drive = normalized(&params.dist_drive, 0.2);
    default.dist_tone = normalized(&params.dist_tone, 0.6);
    default.dist_magic = normalized(&params.dist_magic, 0.2);
    default.dist_mix = normalized(&params.dist_mix, 0.4);
    default.eq_enable = normalized(&params.eq_enable, false);
    default.limiter_enable = normalized(&params.limiter_enable, true);
    apply_audible_gain(&mut default);
    presets.push(PresetEntry {
        name: "Default".to_string(),
        data: default,
        user: false,
    });

    let mut growl = PresetData::from_params(params);
    growl.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
    growl.osc_blend = normalized(&params.osc_blend, 1.0);
    growl.wavetable_position = normalized(&params.wavetable_position, 0.64);
    growl.wavetable_distortion = normalized(&params.wavetable_distortion, 0.8);
    growl.sub_level = normalized(&params.sub_level, 0.8);
    growl.analog_drive = normalized(&params.analog_drive, 0.65);
    growl.dist_enable = normalized(&params.dist_enable, true);
    growl.dist_drive = normalized(&params.dist_drive, 0.7);
    growl.dist_tone = normalized(&params.dist_tone, 0.75);
    growl.dist_magic = normalized(&params.dist_magic, 0.6);
    growl.dist_mix = normalized(&params.dist_mix, 0.85);
    growl.eq_enable = normalized(&params.eq_enable, true);
    growl.eq_low_gain = normalized(&params.eq_low_gain, 4.0);
    growl.eq_mid_gain = normalized(&params.eq_mid_gain, 3.0);
    growl.eq_mid_freq = normalized(&params.eq_mid_freq, 520.0);
    growl.eq_mid_q = normalized(&params.eq_mid_q, 1.1);
    growl.eq_high_gain = normalized(&params.eq_high_gain, 2.5);
    growl.eq_mix = normalized(&params.eq_mix, 1.0);
    growl.amp_attack_ms = normalized(&params.amp_attack_ms, 0.4);
    growl.amp_decay_ms = normalized(&params.amp_decay_ms, 6.0);
    growl.amp_sustain_level = normalized(&params.amp_sustain_level, 0.7);
    growl.amp_release_ms = normalized(&params.amp_release_ms, 5.5);
    growl.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    growl.filter_cut = normalized(&params.filter_cut, 160.0);
    growl.filter_res = normalized(&params.filter_res, 0.7);
    growl.filter_amount = normalized(&params.filter_amount, 1.0);
    growl.lfo1_rate = normalized(&params.lfo1_rate, 3.8);
    growl.lfo1_shape = normalized(&params.lfo1_shape, OscillatorShape::Triangle);
    growl.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
    growl.mod1_target = normalized(&params.mod1_target, ModTarget::FilterCut);
    growl.mod1_amount = normalized(&params.mod1_amount, 0.95);
    growl.mod2_source = normalized(&params.mod2_source, ModSource::Lfo2);
    growl.mod2_target = normalized(&params.mod2_target, ModTarget::WavetablePos);
    growl.mod2_amount = normalized(&params.mod2_amount, 0.55);
    growl.chorus_enable = normalized(&params.chorus_enable, false);
    growl.reverb_enable = normalized(&params.reverb_enable, false);
    growl.delay_enable = normalized(&params.delay_enable, false);
    growl.limiter_enable = normalized(&params.limiter_enable, true);
    apply_audible_gain(&mut growl);
    presets.push(PresetEntry {
        name: "Dubstep Growl".to_string(),
        data: growl,
        user: false,
    });

    let mut wobble = PresetData::from_params(params);
    wobble.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    wobble.osc_blend = normalized(&params.osc_blend, 0.75);
    wobble.wavetable_position = normalized(&params.wavetable_position, 0.58);
    wobble.wavetable_distortion = normalized(&params.wavetable_distortion, 0.65);
    wobble.sub_level = normalized(&params.sub_level, 0.7);
    wobble.dist_enable = normalized(&params.dist_enable, true);
    wobble.dist_drive = normalized(&params.dist_drive, 0.6);
    wobble.dist_tone = normalized(&params.dist_tone, 0.7);
    wobble.dist_magic = normalized(&params.dist_magic, 0.55);
    wobble.dist_mix = normalized(&params.dist_mix, 0.8);
    wobble.eq_enable = normalized(&params.eq_enable, true);
    wobble.eq_low_gain = normalized(&params.eq_low_gain, 3.5);
    wobble.eq_mid_gain = normalized(&params.eq_mid_gain, 2.0);
    wobble.eq_mid_freq = normalized(&params.eq_mid_freq, 680.0);
    wobble.eq_mid_q = normalized(&params.eq_mid_q, 0.9);
    wobble.eq_high_gain = normalized(&params.eq_high_gain, 2.0);
    wobble.eq_mix = normalized(&params.eq_mix, 1.0);
    wobble.amp_attack_ms = normalized(&params.amp_attack_ms, 0.5);
    wobble.amp_decay_ms = normalized(&params.amp_decay_ms, 7.0);
    wobble.amp_sustain_level = normalized(&params.amp_sustain_level, 0.65);
    wobble.amp_release_ms = normalized(&params.amp_release_ms, 6.0);
    wobble.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    wobble.filter_cut = normalized(&params.filter_cut, 190.0);
    wobble.filter_res = normalized(&params.filter_res, 0.6);
    wobble.lfo1_rate = normalized(&params.lfo1_rate, 6.5);
    wobble.lfo1_shape = normalized(&params.lfo1_shape, OscillatorShape::Triangle);
    wobble.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
    wobble.mod1_target = normalized(&params.mod1_target, ModTarget::FilterCut);
    wobble.mod1_amount = normalized(&params.mod1_amount, 0.85);
    wobble.mod2_source = normalized(&params.mod2_source, ModSource::Lfo2);
    wobble.mod2_target = normalized(&params.mod2_target, ModTarget::WavetablePos);
    wobble.mod2_amount = normalized(&params.mod2_amount, 0.5);
    wobble.chorus_enable = normalized(&params.chorus_enable, false);
    wobble.reverb_enable = normalized(&params.reverb_enable, false);
    wobble.delay_enable = normalized(&params.delay_enable, false);
    wobble.limiter_enable = normalized(&params.limiter_enable, true);
    apply_audible_gain(&mut wobble);
    presets.push(PresetEntry {
        name: "Ripper Wobble".to_string(),
        data: wobble,
        user: false,
    });

    let mut reese = PresetData::from_params(params);
    reese.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    reese.osc_blend = normalized(&params.osc_blend, 0.65);
    reese.wavetable_position = normalized(&params.wavetable_position, 0.38);
    reese.wavetable_distortion = normalized(&params.wavetable_distortion, 0.55);
    reese.sub_level = normalized(&params.sub_level, 0.6);
    reese.dist_enable = normalized(&params.dist_enable, true);
    reese.dist_drive = normalized(&params.dist_drive, 0.55);
    reese.dist_tone = normalized(&params.dist_tone, 0.55);
    reese.dist_magic = normalized(&params.dist_magic, 0.45);
    reese.dist_mix = normalized(&params.dist_mix, 0.6);
    reese.eq_enable = normalized(&params.eq_enable, true);
    reese.eq_low_gain = normalized(&params.eq_low_gain, 2.5);
    reese.eq_mid_gain = normalized(&params.eq_mid_gain, 1.5);
    reese.eq_mid_freq = normalized(&params.eq_mid_freq, 780.0);
    reese.eq_mid_q = normalized(&params.eq_mid_q, 1.0);
    reese.eq_high_gain = normalized(&params.eq_high_gain, 1.0);
    reese.eq_mix = normalized(&params.eq_mix, 1.0);
    reese.amp_attack_ms = normalized(&params.amp_attack_ms, 0.9);
    reese.amp_decay_ms = normalized(&params.amp_decay_ms, 6.5);
    reese.amp_sustain_level = normalized(&params.amp_sustain_level, 0.75);
    reese.amp_release_ms = normalized(&params.amp_release_ms, 5.5);
    reese.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
    reese.filter_cut = normalized(&params.filter_cut, 420.0);
    reese.filter_res = normalized(&params.filter_res, 0.35);
    reese.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
    reese.mod1_target = normalized(&params.mod1_target, ModTarget::Pan);
    reese.mod1_amount = normalized(&params.mod1_amount, 0.2);
    reese.limiter_enable = normalized(&params.limiter_enable, true);
    apply_audible_gain(&mut reese);
    presets.push(PresetEntry {
        name: "Metal Reese".to_string(),
        data: reese,
        user: false,
    });

    let mut sub = PresetData::from_params(params);
    sub.waveform = normalized(&params.waveform, Waveform::Sine);
    sub.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
    sub.sub_level = normalized(&params.sub_level, 0.9);
    sub.dist_enable = normalized(&params.dist_enable, true);
    sub.dist_drive = normalized(&params.dist_drive, 0.25);
    sub.dist_tone = normalized(&params.dist_tone, 0.45);
    sub.dist_magic = normalized(&params.dist_magic, 0.2);
    sub.dist_mix = normalized(&params.dist_mix, 0.35);
    sub.eq_enable = normalized(&params.eq_enable, true);
    sub.eq_low_gain = normalized(&params.eq_low_gain, 6.0);
    sub.eq_mid_gain = normalized(&params.eq_mid_gain, -2.5);
    sub.eq_mid_freq = normalized(&params.eq_mid_freq, 520.0);
    sub.eq_mid_q = normalized(&params.eq_mid_q, 1.0);
    sub.eq_high_gain = normalized(&params.eq_high_gain, -1.5);
    sub.eq_mix = normalized(&params.eq_mix, 1.0);
    sub.amp_attack_ms = normalized(&params.amp_attack_ms, 0.3);
    sub.amp_decay_ms = normalized(&params.amp_decay_ms, 4.5);
    sub.amp_sustain_level = normalized(&params.amp_sustain_level, 0.85);
    sub.amp_release_ms = normalized(&params.amp_release_ms, 4.0);
    sub.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    sub.filter_cut = normalized(&params.filter_cut, 110.0);
    sub.filter_res = normalized(&params.filter_res, 0.3);
    sub.limiter_enable = normalized(&params.limiter_enable, true);
    apply_audible_gain(&mut sub);
    presets.push(PresetEntry {
        name: "Sub Hammer".to_string(),
        data: sub,
        user: false,
    });

    let mut formant = PresetData::from_params(params);
    formant.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
    formant.wavetable_position = normalized(&params.wavetable_position, 0.52);
    formant.wavetable_distortion = normalized(&params.wavetable_distortion, 0.85);
    formant.sub_level = normalized(&params.sub_level, 0.55);
    formant.dist_enable = normalized(&params.dist_enable, true);
    formant.dist_drive = normalized(&params.dist_drive, 0.75);
    formant.dist_tone = normalized(&params.dist_tone, 0.8);
    formant.dist_magic = normalized(&params.dist_magic, 0.75);
    formant.dist_mix = normalized(&params.dist_mix, 0.9);
    formant.eq_enable = normalized(&params.eq_enable, true);
    formant.eq_low_gain = normalized(&params.eq_low_gain, 3.0);
    formant.eq_mid_gain = normalized(&params.eq_mid_gain, 4.5);
    formant.eq_mid_freq = normalized(&params.eq_mid_freq, 980.0);
    formant.eq_mid_q = normalized(&params.eq_mid_q, 1.6);
    formant.eq_high_gain = normalized(&params.eq_high_gain, 2.5);
    formant.eq_mix = normalized(&params.eq_mix, 1.0);
    formant.amp_attack_ms = normalized(&params.amp_attack_ms, 0.6);
    formant.amp_decay_ms = normalized(&params.amp_decay_ms, 7.5);
    formant.amp_sustain_level = normalized(&params.amp_sustain_level, 0.6);
    formant.amp_release_ms = normalized(&params.amp_release_ms, 6.5);
    formant.filter_type = normalized(&params.filter_type, FilterType::Statevariable);
    formant.filter_cut = normalized(&params.filter_cut, 320.0);
    formant.filter_res = normalized(&params.filter_res, 0.8);
    formant.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
    formant.mod1_target = normalized(&params.mod1_target, ModTarget::FilterRes);
    formant.mod1_amount = normalized(&params.mod1_amount, 0.6);
    formant.limiter_enable = normalized(&params.limiter_enable, true);
    apply_audible_gain(&mut formant);
    presets.push(PresetEntry {
        name: "Formant Grind".to_string(),
        data: formant,
        user: false,
    });

    let mut pluck = PresetData::from_params(params);
    pluck.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    pluck.osc_blend = normalized(&params.osc_blend, 0.7);
    pluck.wavetable_position = normalized(&params.wavetable_position, 0.62);
    pluck.wavetable_distortion = normalized(&params.wavetable_distortion, 0.3);
    pluck.dist_enable = normalized(&params.dist_enable, true);
    pluck.dist_drive = normalized(&params.dist_drive, 0.25);
    pluck.dist_tone = normalized(&params.dist_tone, 0.7);
    pluck.dist_magic = normalized(&params.dist_magic, 0.2);
    pluck.dist_mix = normalized(&params.dist_mix, 0.35);
    pluck.eq_enable = normalized(&params.eq_enable, true);
    pluck.eq_low_gain = normalized(&params.eq_low_gain, 1.0);
    pluck.eq_mid_gain = normalized(&params.eq_mid_gain, 1.5);
    pluck.eq_mid_freq = normalized(&params.eq_mid_freq, 1200.0);
    pluck.eq_mid_q = normalized(&params.eq_mid_q, 0.7);
    pluck.eq_high_gain = normalized(&params.eq_high_gain, 3.5);
    pluck.eq_mix = normalized(&params.eq_mix, 1.0);
    pluck.amp_attack_ms = normalized(&params.amp_attack_ms, 0.4);
    pluck.amp_decay_ms = normalized(&params.amp_decay_ms, 3.0);
    pluck.amp_sustain_level = normalized(&params.amp_sustain_level, 0.2);
    pluck.amp_release_ms = normalized(&params.amp_release_ms, 2.0);
    pluck.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    pluck.filter_cut = normalized(&params.filter_cut, 1500.0);
    pluck.filter_res = normalized(&params.filter_res, 0.2);
    pluck.chorus_enable = normalized(&params.chorus_enable, true);
    pluck.chorus_mix = normalized(&params.chorus_mix, 0.35);
    pluck.delay_enable = normalized(&params.delay_enable, true);
    pluck.delay_time_ms = normalized(&params.delay_time_ms, 360.0);
    pluck.delay_feedback = normalized(&params.delay_feedback, 0.35);
    pluck.delay_mix = normalized(&params.delay_mix, 0.35);
    pluck.reverb_enable = normalized(&params.reverb_enable, true);
    pluck.reverb_mix = normalized(&params.reverb_mix, 0.25);
    apply_audible_gain(&mut pluck);
    presets.push(PresetEntry {
        name: "Trance Pluck".to_string(),
        data: pluck,
        user: false,
    });

    let mut supersaw = PresetData::from_params(params);
    supersaw.waveform = normalized(&params.waveform, Waveform::Sawtooth);
    supersaw.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    supersaw.osc_blend = normalized(&params.osc_blend, 0.6);
    supersaw.wavetable_position = normalized(&params.wavetable_position, 0.5);
    supersaw.wavetable_distortion = normalized(&params.wavetable_distortion, 0.2);
    supersaw.dist_enable = normalized(&params.dist_enable, true);
    supersaw.dist_drive = normalized(&params.dist_drive, 0.2);
    supersaw.dist_tone = normalized(&params.dist_tone, 0.65);
    supersaw.dist_magic = normalized(&params.dist_magic, 0.15);
    supersaw.dist_mix = normalized(&params.dist_mix, 0.25);
    supersaw.eq_enable = normalized(&params.eq_enable, true);
    supersaw.eq_low_gain = normalized(&params.eq_low_gain, 1.5);
    supersaw.eq_mid_gain = normalized(&params.eq_mid_gain, 0.5);
    supersaw.eq_mid_freq = normalized(&params.eq_mid_freq, 950.0);
    supersaw.eq_mid_q = normalized(&params.eq_mid_q, 0.8);
    supersaw.eq_high_gain = normalized(&params.eq_high_gain, 3.5);
    supersaw.eq_mix = normalized(&params.eq_mix, 1.0);
    supersaw.amp_attack_ms = normalized(&params.amp_attack_ms, 2.2);
    supersaw.amp_decay_ms = normalized(&params.amp_decay_ms, 5.5);
    supersaw.amp_sustain_level = normalized(&params.amp_sustain_level, 0.8);
    supersaw.amp_release_ms = normalized(&params.amp_release_ms, 6.5);
    supersaw.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    supersaw.filter_cut = normalized(&params.filter_cut, 1800.0);
    supersaw.filter_res = normalized(&params.filter_res, 0.2);
    supersaw.chorus_enable = normalized(&params.chorus_enable, true);
    supersaw.chorus_depth = normalized(&params.chorus_depth, 26.0);
    supersaw.chorus_mix = normalized(&params.chorus_mix, 0.5);
    supersaw.delay_enable = normalized(&params.delay_enable, true);
    supersaw.delay_time_ms = normalized(&params.delay_time_ms, 420.0);
    supersaw.delay_feedback = normalized(&params.delay_feedback, 0.3);
    supersaw.delay_mix = normalized(&params.delay_mix, 0.28);
    supersaw.reverb_enable = normalized(&params.reverb_enable, true);
    supersaw.reverb_mix = normalized(&params.reverb_mix, 0.3);
    apply_audible_gain(&mut supersaw);
    presets.push(PresetEntry {
        name: "Super Saw Air".to_string(),
        data: supersaw,
        user: false,
    });

    let mut pad = PresetData::from_params(params);
    pad.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
    pad.osc_blend = normalized(&params.osc_blend, 0.65);
    pad.wavetable_position = normalized(&params.wavetable_position, 0.42);
    pad.wavetable_distortion = normalized(&params.wavetable_distortion, 0.15);
    pad.dist_enable = normalized(&params.dist_enable, true);
    pad.dist_drive = normalized(&params.dist_drive, 0.15);
    pad.dist_tone = normalized(&params.dist_tone, 0.55);
    pad.dist_magic = normalized(&params.dist_magic, 0.1);
    pad.dist_mix = normalized(&params.dist_mix, 0.2);
    pad.eq_enable = normalized(&params.eq_enable, true);
    pad.eq_low_gain = normalized(&params.eq_low_gain, 1.0);
    pad.eq_mid_gain = normalized(&params.eq_mid_gain, -0.5);
    pad.eq_mid_freq = normalized(&params.eq_mid_freq, 850.0);
    pad.eq_mid_q = normalized(&params.eq_mid_q, 0.7);
    pad.eq_high_gain = normalized(&params.eq_high_gain, 2.0);
    pad.eq_mix = normalized(&params.eq_mix, 1.0);
    pad.amp_attack_ms = normalized(&params.amp_attack_ms, 6.0);
    pad.amp_decay_ms = normalized(&params.amp_decay_ms, 8.5);
    pad.amp_sustain_level = normalized(&params.amp_sustain_level, 0.85);
    pad.amp_release_ms = normalized(&params.amp_release_ms, 9.0);
    pad.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    pad.filter_cut = normalized(&params.filter_cut, 900.0);
    pad.filter_res = normalized(&params.filter_res, 0.25);
    pad.chorus_enable = normalized(&params.chorus_enable, true);
    pad.chorus_mix = normalized(&params.chorus_mix, 0.45);
    pad.delay_enable = normalized(&params.delay_enable, true);
    pad.delay_time_ms = normalized(&params.delay_time_ms, 560.0);
    pad.delay_feedback = normalized(&params.delay_feedback, 0.35);
    pad.delay_mix = normalized(&params.delay_mix, 0.25);
    pad.reverb_enable = normalized(&params.reverb_enable, true);
    pad.reverb_size = normalized(&params.reverb_size, 0.85);
    pad.reverb_mix = normalized(&params.reverb_mix, 0.5);
    apply_audible_gain(&mut pad);
    presets.push(PresetEntry {
        name: "Trance Pad Cloud".to_string(),
        data: pad,
        user: false,
    });

    let mut arp = PresetData::from_params(params);
    arp.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
    arp.wavetable_position = normalized(&params.wavetable_position, 0.72);
    arp.wavetable_distortion = normalized(&params.wavetable_distortion, 0.35);
    arp.dist_enable = normalized(&params.dist_enable, true);
    arp.dist_drive = normalized(&params.dist_drive, 0.35);
    arp.dist_tone = normalized(&params.dist_tone, 0.85);
    arp.dist_magic = normalized(&params.dist_magic, 0.25);
    arp.dist_mix = normalized(&params.dist_mix, 0.45);
    arp.eq_enable = normalized(&params.eq_enable, true);
    arp.eq_low_gain = normalized(&params.eq_low_gain, 1.0);
    arp.eq_mid_gain = normalized(&params.eq_mid_gain, 1.0);
    arp.eq_mid_freq = normalized(&params.eq_mid_freq, 1400.0);
    arp.eq_mid_q = normalized(&params.eq_mid_q, 0.9);
    arp.eq_high_gain = normalized(&params.eq_high_gain, 4.0);
    arp.eq_mix = normalized(&params.eq_mix, 1.0);
    arp.amp_attack_ms = normalized(&params.amp_attack_ms, 0.3);
    arp.amp_decay_ms = normalized(&params.amp_decay_ms, 2.5);
    arp.amp_sustain_level = normalized(&params.amp_sustain_level, 0.4);
    arp.amp_release_ms = normalized(&params.amp_release_ms, 1.8);
    arp.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
    arp.filter_cut = normalized(&params.filter_cut, 2200.0);
    arp.filter_res = normalized(&params.filter_res, 0.3);
    arp.delay_enable = normalized(&params.delay_enable, true);
    arp.delay_time_ms = normalized(&params.delay_time_ms, 320.0);
    arp.delay_feedback = normalized(&params.delay_feedback, 0.28);
    arp.delay_mix = normalized(&params.delay_mix, 0.3);
    arp.reverb_enable = normalized(&params.reverb_enable, true);
    arp.reverb_mix = normalized(&params.reverb_mix, 0.22);
    apply_audible_gain(&mut arp);
    presets.push(PresetEntry {
        name: "Arp Spark".to_string(),
        data: arp,
        user: false,
    });

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
            |cx| Label::new(cx, "Filter + Env"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(2)),
            |cx| Label::new(cx, "Mod Matrix"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(3)),
            |cx| Label::new(cx, "Motion"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(4)),
            |cx| Label::new(cx, "Articulator"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(5)),
            |cx| Label::new(cx, "Sequencer"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(6)),
            |cx| Label::new(cx, "Multi Filter"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(7)),
            |cx| Label::new(cx, "FX"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(8)),
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
            .position(|preset| preset.name == "Default")
            .unwrap_or(0);
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
        HStack::new(cx, |cx| {
            Label::new(cx, "CatSynth")
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
            0 => build_osc_tab(cx),
            1 => build_env_tab(cx),
            2 => build_mod_tab(cx),
            3 => build_lfo_tab(cx),
            4 => build_articulator_tab(cx),
            5 => build_seq_tab(cx),
            6 => build_filter_tab(cx),
            7 => build_fx_tab(cx),
            _ => build_utility_tab(cx),
        });

        Element::new(cx)
            .height(Pixels(12.0))
            .width(Stretch(1.0));

        // レイアウトを初期化時にリセットする
        cx.emit(GuiContextEvent::Resize);
    })
}

#[derive(Lens)]
struct DrumUiData {
    params: Arc<DrumSynthParams>,
    active_slot: usize,
    active_tab: usize,
    active_bank: usize,
    kit_index: usize,
    kit_display: String,
    pad_counter: u32,
}

enum DrumUiEvent {
    SelectSlot(usize),
    PadTrigger(usize),
    SetTab(usize),
    SetBank(usize),
    KitPrev,
    KitNext,
    KitLoad,
    KitSave,
    Randomize,
}

const DRUM_BANK_SIZE: usize = 16;
const DRUM_BANK_COUNT: usize = DRUM_SLOTS / DRUM_BANK_SIZE;

const DRUM_SLOT_LABELS: [&str; DRUM_SLOTS] = [
    "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14",
    "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28",
    "29", "30", "31", "32",
];

#[derive(Clone, Copy, Debug)]
enum DrumKitPreset {
    Studio,
    StudioFactory,
    EightBit,
    Tr808,
    Jazz,
    House,
    Dubstep,
    Electronica,
    Industrial,
    VideoGame,
    Acoustic,
    Trap,
    LoFi,
    Techno,
}

const DRUM_KITS: [DrumKitPreset; 14] = [
    DrumKitPreset::Studio,
    DrumKitPreset::StudioFactory,
    DrumKitPreset::EightBit,
    DrumKitPreset::Tr808,
    DrumKitPreset::Jazz,
    DrumKitPreset::House,
    DrumKitPreset::Dubstep,
    DrumKitPreset::Electronica,
    DrumKitPreset::Industrial,
    DrumKitPreset::VideoGame,
    DrumKitPreset::Acoustic,
    DrumKitPreset::Trap,
    DrumKitPreset::LoFi,
    DrumKitPreset::Techno,
];

#[derive(Clone, Copy, Debug)]
struct DrumSlotPreset {
    instrument: InstrumentType,
    exciter: ExciterType,
    exciter_mix: f32,
    resonator: ResonatorType,
    material: MaterialType,
    level: f32,
    pan: f32,
    tune: f32,
    decay: f32,
    damping: f32,
    spread: f32,
    strike_position: f32,
    strike_hardness: f32,
    attack: f32,
    pitch_env: f32,
    pitch_decay: f32,
    noise: f32,
    diffusion: f32,
    noise_color: f32,
    noise_type: NoiseType,
    noise_density: f32,
    resonator_mix: f32,
    mode_tilt: f32,
    cymbal_shape: f32,
    snare_wire: f32,
    snare_wire_decay: f32,
    drive: f32,
    transient: f32,
    body: f32,
    velocity_sensitivity: f32,
    tone_low: f32,
    tone_mid: f32,
    tone_high: f32,
    pan_lfo_rate: f32,
    pan_lfo_depth: f32,
}

fn base_slot_preset(instrument: InstrumentType) -> DrumSlotPreset {
    let noise_type = match instrument {
        InstrumentType::HatClosed
        | InstrumentType::HatOpen
        | InstrumentType::HatPedal
        | InstrumentType::Ride
        | InstrumentType::Crash => NoiseType::Metallic,
        InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot => NoiseType::Pink,
        _ => NoiseType::White,
    };
    let mut preset = DrumSlotPreset {
        instrument,
        exciter: ExciterType::Mallet,
        exciter_mix: 0.4,
        resonator: ResonatorType::Membrane,
        material: MaterialType::Skin,
        level: 0.9,
        pan: 0.5,
        tune: 0.0,
        decay: 0.6,
        damping: 0.5,
        spread: 0.0,
        strike_position: 0.5,
        strike_hardness: 0.6,
        attack: 0.4,
        pitch_env: 0.35,
        pitch_decay: 0.4,
        noise: 0.2,
        diffusion: 0.5,
        noise_color: 0.5,
        noise_type,
        noise_density: 0.5,
        resonator_mix: 0.7,
        mode_tilt: 0.2,
        cymbal_shape: 0.5,
        snare_wire: 0.6,
        snare_wire_decay: 0.5,
        drive: 0.0,
        transient: 0.4,
        body: 0.8,
        velocity_sensitivity: 0.85,
        tone_low: 0.0,
        tone_mid: 0.0,
        tone_high: 0.0,
        pan_lfo_rate: 1.2,
        pan_lfo_depth: 0.0,
    };

    match instrument {
        InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot => {
            preset.noise = 0.55;
            preset.diffusion = 0.7;
            preset.noise_color = 0.7;
            preset.noise_density = 0.7;
            preset.snare_wire = 0.8;
            preset.snare_wire_decay = 0.6;
            preset.transient = 0.55;
            preset.tone_mid = 0.8;
            preset.tone_high = 1.5;
        }
        InstrumentType::Clap => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Plate;
            preset.decay = 0.28;
            preset.noise = 0.75;
            preset.noise_color = 0.8;
            preset.noise_density = 0.65;
            preset.diffusion = 0.75;
            preset.attack = 0.2;
            preset.transient = 0.6;
            preset.body = 0.2;
            preset.tone_mid = 1.4;
            preset.tone_high = 2.2;
        }
        InstrumentType::Rimshot => {
            preset.exciter = ExciterType::Stick;
            preset.resonator = ResonatorType::Plate;
            preset.decay = 0.18;
            preset.noise = 0.45;
            preset.noise_color = 0.75;
            preset.noise_density = 0.5;
            preset.diffusion = 0.6;
            preset.attack = 0.15;
            preset.transient = 0.7;
            preset.body = 0.2;
            preset.tone_mid = 1.6;
            preset.tone_high = 2.8;
        }
        InstrumentType::Perc1 | InstrumentType::Perc2 => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Metallic;
            preset.material = MaterialType::Metal;
            preset.decay = 0.22;
            preset.noise = 0.7;
            preset.noise_color = 0.85;
            preset.noise_type = NoiseType::White;
            preset.noise_density = 0.8;
            preset.diffusion = 0.75;
            preset.cymbal_shape = 0.6;
            preset.attack = 0.12;
            preset.transient = 0.55;
            preset.body = 0.1;
            preset.tone_high = 1.8;
        }
        InstrumentType::HatClosed => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Metallic;
            preset.material = MaterialType::Metal;
            preset.noise = 0.75;
            preset.diffusion = 0.8;
            preset.noise_color = 0.9;
            preset.noise_density = 0.85;
            preset.cymbal_shape = 0.75;
            preset.decay = 0.2;
            preset.attack = 0.15;
            preset.transient = 0.6;
            preset.tone_high = 2.5;
        }
        InstrumentType::HatOpen => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Metallic;
            preset.material = MaterialType::Metal;
            preset.noise = 0.8;
            preset.diffusion = 0.82;
            preset.noise_color = 0.9;
            preset.noise_density = 0.9;
            preset.cymbal_shape = 0.8;
            preset.decay = 0.6;
            preset.attack = 0.18;
            preset.transient = 0.6;
            preset.tone_high = 2.0;
        }
        InstrumentType::HatPedal => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Metallic;
            preset.material = MaterialType::Metal;
            preset.noise = 0.7;
            preset.diffusion = 0.75;
            preset.noise_color = 0.85;
            preset.noise_density = 0.8;
            preset.cymbal_shape = 0.7;
            preset.decay = 0.15;
            preset.attack = 0.12;
            preset.transient = 0.55;
            preset.tone_high = 2.0;
        }
        InstrumentType::Ride => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Metallic;
            preset.material = MaterialType::Metal;
            preset.noise = 0.6;
            preset.diffusion = 0.7;
            preset.noise_color = 0.85;
            preset.noise_density = 0.85;
            preset.cymbal_shape = 0.75;
            preset.decay = 0.65;
            preset.attack = 0.2;
            preset.transient = 0.55;
            preset.tone_high = 1.5;
        }
        InstrumentType::Crash => {
            preset.exciter = ExciterType::Noise;
            preset.resonator = ResonatorType::Metallic;
            preset.material = MaterialType::Metal;
            preset.noise = 0.65;
            preset.diffusion = 0.8;
            preset.noise_color = 0.9;
            preset.noise_density = 0.9;
            preset.cymbal_shape = 0.8;
            preset.decay = 0.75;
            preset.attack = 0.22;
            preset.transient = 0.6;
            preset.tone_high = 2.0;
        }
        _ => {}
    }

    preset
}

fn kit_name(preset: DrumKitPreset) -> &'static str {
    match preset {
        DrumKitPreset::Studio => "Studio",
        DrumKitPreset::StudioFactory => "Studio (Factory)",
        DrumKitPreset::EightBit => "8-Bit",
        DrumKitPreset::Tr808 => "808",
        DrumKitPreset::Jazz => "Jazz",
        DrumKitPreset::House => "House",
        DrumKitPreset::Dubstep => "Dubstep",
        DrumKitPreset::Electronica => "Electronica",
        DrumKitPreset::Industrial => "Industrial",
        DrumKitPreset::VideoGame => "Video Game",
        DrumKitPreset::Acoustic => "Acoustic",
        DrumKitPreset::Trap => "Trap",
        DrumKitPreset::LoFi => "Lo-Fi",
        DrumKitPreset::Techno => "Techno",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DrumSlotSnapshot {
    instrument: f32,
    exciter: f32,
    #[serde(default)]
    exciter_mix: f32,
    resonator: f32,
    material: f32,
    level: f32,
    pan: f32,
    tune: f32,
    decay: f32,
    damping: f32,
    spread: f32,
    strike_position: f32,
    strike_hardness: f32,
    attack: f32,
    pitch_env: f32,
    pitch_decay: f32,
    noise: f32,
    diffusion: f32,
    #[serde(default)]
    noise_color: f32,
    #[serde(default)]
    noise_type: f32,
    #[serde(default)]
    noise_density: f32,
    #[serde(default)]
    resonator_mix: f32,
    #[serde(default)]
    mode_tilt: f32,
    #[serde(default)]
    cymbal_shape: f32,
    #[serde(default)]
    snare_wire: f32,
    #[serde(default)]
    snare_wire_decay: f32,
    drive: f32,
    #[serde(default)]
    transient: f32,
    #[serde(default)]
    body: f32,
    #[serde(default)]
    velocity_sensitivity: f32,
    #[serde(default)]
    tone_low: f32,
    #[serde(default)]
    tone_mid: f32,
    #[serde(default)]
    tone_high: f32,
    #[serde(default)]
    pan_lfo_rate: f32,
    #[serde(default)]
    pan_lfo_depth: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DrumKitSnapshot {
    slots: [DrumSlotSnapshot; DRUM_SLOTS],
}

fn kit_slots(preset: DrumKitPreset) -> [DrumSlotPreset; DRUM_SLOTS] {
    let mut slots = std::array::from_fn(|index| base_slot_preset(default_instrument_for_slot(index)));

    for (index, slot) in slots.iter_mut().enumerate() {
        apply_instrument_patch(index, slot);
        apply_kit_style(preset, index, slot);
    }

    slots
}

fn apply_instrument_patch(index: usize, slot: &mut DrumSlotPreset) {
    match slot.instrument {
        InstrumentType::Kick => {
            slot.exciter = ExciterType::Impulse;
            slot.resonator = ResonatorType::Membrane;
            slot.material = MaterialType::Skin;
            slot.level = 0.95;
            slot.pan = 0.5;
            slot.tune = 0.0;
            slot.decay = 0.7;
            slot.damping = 0.35;
            slot.spread = 0.0;
            slot.strike_position = 0.45;
            slot.strike_hardness = 0.7;
            slot.attack = 0.18;
            slot.pitch_env = 0.6;
            slot.pitch_decay = 0.24;
            slot.exciter_mix = 0.2;
            slot.noise = 0.1;
            slot.diffusion = 0.3;
            slot.noise_color = 0.4;
            slot.noise_type = NoiseType::Pink;
            slot.noise_density = 0.4;
            slot.resonator_mix = 0.85;
            slot.mode_tilt = 0.15;
            slot.cymbal_shape = 0.0;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.2;
            slot.transient = 0.65;
            slot.body = 0.95;
            slot.velocity_sensitivity = 0.7;
            slot.tone_low = 4.0;
            slot.tone_mid = -3.0;
            slot.tone_high = -4.0;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Snare => {
            slot.exciter = ExciterType::Stick;
            slot.resonator = ResonatorType::Plate;
            slot.material = MaterialType::Skin;
            slot.level = 0.9;
            slot.pan = 0.48;
            slot.tune = 0.0;
            slot.decay = 0.5;
            slot.damping = 0.4;
            slot.spread = 0.0;
            slot.strike_position = 0.4;
            slot.strike_hardness = 0.7;
            slot.attack = 0.2;
            slot.pitch_env = 0.2;
            slot.pitch_decay = 0.2;
            slot.exciter_mix = 0.55;
            slot.noise = 0.8;
            slot.diffusion = 0.8;
            slot.noise_color = 0.8;
            slot.noise_type = NoiseType::Pink;
            slot.noise_density = 0.7;
            slot.resonator_mix = 0.65;
            slot.mode_tilt = 0.25;
            slot.cymbal_shape = 0.0;
            slot.snare_wire = 0.9;
            slot.snare_wire_decay = 0.6;
            slot.drive = 0.1;
            slot.transient = 0.6;
            slot.body = 0.3;
            slot.velocity_sensitivity = 0.8;
            slot.tone_low = -1.0;
            slot.tone_mid = 1.6;
            slot.tone_high = 2.2;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::HatClosed => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Metallic;
            slot.material = MaterialType::Metal;
            slot.level = 0.75;
            slot.pan = 0.45;
            slot.tune = 0.0;
            slot.decay = 0.15;
            slot.damping = 0.4;
            slot.spread = 0.1;
            slot.strike_position = 0.6;
            slot.strike_hardness = 0.75;
            slot.attack = 0.1;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.85;
            slot.noise = 0.9;
            slot.diffusion = 0.85;
            slot.noise_color = 0.95;
            slot.noise_type = NoiseType::Metallic;
            slot.noise_density = 0.9;
            slot.resonator_mix = 0.35;
            slot.mode_tilt = 0.45;
            slot.cymbal_shape = 0.85;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.05;
            slot.transient = 0.65;
            slot.body = 0.1;
            slot.velocity_sensitivity = 0.95;
            slot.tone_low = -6.0;
            slot.tone_mid = 0.0;
            slot.tone_high = 4.0;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::HatOpen => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Metallic;
            slot.material = MaterialType::Metal;
            slot.level = 0.8;
            slot.pan = 0.55;
            slot.tune = 0.0;
            slot.decay = 0.6;
            slot.damping = 0.4;
            slot.spread = 0.15;
            slot.strike_position = 0.62;
            slot.strike_hardness = 0.72;
            slot.attack = 0.14;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.8;
            slot.noise = 0.85;
            slot.diffusion = 0.88;
            slot.noise_color = 0.95;
            slot.noise_type = NoiseType::Metallic;
            slot.noise_density = 0.9;
            slot.resonator_mix = 0.45;
            slot.mode_tilt = 0.4;
            slot.cymbal_shape = 0.88;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.05;
            slot.transient = 0.6;
            slot.body = 0.15;
            slot.velocity_sensitivity = 0.92;
            slot.tone_low = -6.0;
            slot.tone_mid = 0.0;
            slot.tone_high = 3.6;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::HatPedal => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Metallic;
            slot.material = MaterialType::Metal;
            slot.level = 0.7;
            slot.pan = 0.5;
            slot.tune = 0.0;
            slot.decay = 0.1;
            slot.damping = 0.4;
            slot.spread = 0.08;
            slot.strike_position = 0.55;
            slot.strike_hardness = 0.7;
            slot.attack = 0.08;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.85;
            slot.noise = 0.85;
            slot.diffusion = 0.85;
            slot.noise_color = 0.92;
            slot.noise_type = NoiseType::Metallic;
            slot.noise_density = 0.88;
            slot.resonator_mix = 0.3;
            slot.mode_tilt = 0.4;
            slot.cymbal_shape = 0.82;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.05;
            slot.transient = 0.6;
            slot.body = 0.1;
            slot.velocity_sensitivity = 0.9;
            slot.tone_low = -6.0;
            slot.tone_mid = 0.0;
            slot.tone_high = 3.2;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Ride => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Metallic;
            slot.material = MaterialType::Metal;
            slot.level = 0.8;
            slot.pan = 0.6;
            slot.tune = 0.0;
            slot.decay = 0.7;
            slot.damping = 0.45;
            slot.spread = 0.2;
            slot.strike_position = 0.65;
            slot.strike_hardness = 0.7;
            slot.attack = 0.18;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.7;
            slot.noise = 0.7;
            slot.diffusion = 0.8;
            slot.noise_color = 0.9;
            slot.noise_type = NoiseType::Metallic;
            slot.noise_density = 0.85;
            slot.resonator_mix = 0.5;
            slot.mode_tilt = 0.35;
            slot.cymbal_shape = 0.8;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.05;
            slot.transient = 0.55;
            slot.body = 0.2;
            slot.velocity_sensitivity = 0.9;
            slot.tone_low = -4.0;
            slot.tone_mid = -0.5;
            slot.tone_high = 2.4;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Crash => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Metallic;
            slot.material = MaterialType::Metal;
            slot.level = 0.85;
            slot.pan = 0.4;
            slot.tune = 0.0;
            slot.decay = 0.8;
            slot.damping = 0.45;
            slot.spread = 0.25;
            slot.strike_position = 0.7;
            slot.strike_hardness = 0.7;
            slot.attack = 0.2;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.75;
            slot.noise = 0.75;
            slot.diffusion = 0.85;
            slot.noise_color = 0.92;
            slot.noise_type = NoiseType::Metallic;
            slot.noise_density = 0.88;
            slot.resonator_mix = 0.55;
            slot.mode_tilt = 0.35;
            slot.cymbal_shape = 0.82;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.05;
            slot.transient = 0.6;
            slot.body = 0.2;
            slot.velocity_sensitivity = 0.9;
            slot.tone_low = -4.0;
            slot.tone_mid = 0.0;
            slot.tone_high = 2.8;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Rimshot => {
            slot.exciter = ExciterType::Stick;
            slot.resonator = ResonatorType::Plate;
            slot.material = MaterialType::Wood;
            slot.level = 0.7;
            slot.pan = 0.4;
            slot.tune = 0.0;
            slot.decay = 0.18;
            slot.damping = 0.5;
            slot.spread = 0.0;
            slot.strike_position = 0.35;
            slot.strike_hardness = 0.75;
            slot.attack = 0.15;
            slot.pitch_env = 0.1;
            slot.pitch_decay = 0.12;
            slot.exciter_mix = 0.5;
            slot.noise = 0.4;
            slot.diffusion = 0.6;
            slot.noise_color = 0.75;
            slot.noise_type = NoiseType::Pink;
            slot.noise_density = 0.5;
            slot.resonator_mix = 0.6;
            slot.mode_tilt = 0.3;
            slot.cymbal_shape = 0.0;
            slot.snare_wire = 0.4;
            slot.snare_wire_decay = 0.4;
            slot.drive = 0.1;
            slot.transient = 0.75;
            slot.body = 0.2;
            slot.velocity_sensitivity = 0.85;
            slot.tone_low = -2.0;
            slot.tone_mid = 1.8;
            slot.tone_high = 3.0;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Clap => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Plate;
            slot.material = MaterialType::Skin;
            slot.level = 0.8;
            slot.pan = 0.5;
            slot.tune = 0.0;
            slot.decay = 0.28;
            slot.damping = 0.45;
            slot.spread = 0.05;
            slot.strike_position = 0.5;
            slot.strike_hardness = 0.6;
            slot.attack = 0.2;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.8;
            slot.noise = 0.8;
            slot.diffusion = 0.8;
            slot.noise_color = 0.8;
            slot.noise_type = NoiseType::Pink;
            slot.noise_density = 0.65;
            slot.resonator_mix = 0.5;
            slot.mode_tilt = 0.3;
            slot.cymbal_shape = 0.0;
            slot.snare_wire = 0.5;
            slot.snare_wire_decay = 0.5;
            slot.drive = 0.05;
            slot.transient = 0.7;
            slot.body = 0.2;
            slot.velocity_sensitivity = 0.85;
            slot.tone_low = -2.0;
            slot.tone_mid = 1.4;
            slot.tone_high = 2.2;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh => {
            slot.exciter = ExciterType::Mallet;
            slot.resonator = ResonatorType::Membrane;
            slot.material = MaterialType::Skin;
            slot.level = 0.85;
            slot.pan = if matches!(slot.instrument, InstrumentType::TomLow) {
                0.4
            } else if matches!(slot.instrument, InstrumentType::TomHigh) {
                0.6
            } else {
                0.5
            };
            slot.tune = 0.0;
            slot.decay = 0.55;
            slot.damping = 0.45;
            slot.spread = 0.0;
            slot.strike_position = 0.5;
            slot.strike_hardness = 0.65;
            slot.attack = 0.2;
            slot.pitch_env = 0.3;
            slot.pitch_decay = 0.25;
            slot.exciter_mix = 0.35;
            slot.noise = 0.2;
            slot.diffusion = 0.4;
            slot.noise_color = 0.5;
            slot.noise_type = NoiseType::Pink;
            slot.noise_density = 0.4;
            slot.resonator_mix = 0.75;
            slot.mode_tilt = 0.2;
            slot.cymbal_shape = 0.0;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.1;
            slot.transient = 0.55;
            slot.body = 0.8;
            slot.velocity_sensitivity = 0.8;
            slot.tone_low = 2.0;
            slot.tone_mid = -1.0;
            slot.tone_high = -2.0;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Perc1 | InstrumentType::Perc2 => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Metallic;
            slot.material = MaterialType::Metal;
            slot.level = 0.7;
            slot.pan = if matches!(slot.instrument, InstrumentType::Perc1) {
                0.35
            } else {
                0.65
            };
            slot.tune = 0.0;
            slot.decay = 0.22;
            slot.damping = 0.5;
            slot.spread = 0.1;
            slot.strike_position = 0.6;
            slot.strike_hardness = 0.7;
            slot.attack = 0.12;
            slot.pitch_env = 0.0;
            slot.pitch_decay = 0.0;
            slot.exciter_mix = 0.7;
            slot.noise = 0.75;
            slot.diffusion = 0.75;
            slot.noise_color = 0.88;
            slot.noise_type = NoiseType::White;
            slot.noise_density = 0.8;
            slot.resonator_mix = 0.55;
            slot.mode_tilt = 0.35;
            slot.cymbal_shape = 0.6;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.05;
            slot.transient = 0.55;
            slot.body = 0.1;
            slot.velocity_sensitivity = 0.9;
            slot.tone_low = -2.0;
            slot.tone_mid = 0.5;
            slot.tone_high = 1.8;
            slot.pan_lfo_rate = 1.2;
            slot.pan_lfo_depth = 0.0;
        }
        InstrumentType::Fx1 | InstrumentType::Fx2 => {
            slot.exciter = ExciterType::Noise;
            slot.resonator = ResonatorType::Plate;
            slot.material = MaterialType::Composite;
            slot.level = 0.7;
            slot.pan = if matches!(slot.instrument, InstrumentType::Fx1) {
                0.3
            } else {
                0.7
            };
            slot.tune = if matches!(slot.instrument, InstrumentType::Fx1) {
                -6.0
            } else {
                6.0
            };
            slot.decay = 0.5;
            slot.damping = 0.4;
            slot.spread = 0.2;
            slot.strike_position = 0.5;
            slot.strike_hardness = 0.6;
            slot.attack = 0.2;
            slot.pitch_env = 0.2;
            slot.pitch_decay = 0.4;
            slot.exciter_mix = 0.6;
            slot.noise = 0.6;
            slot.diffusion = 0.7;
            slot.noise_color = 0.8;
            slot.noise_type = NoiseType::Pink;
            slot.noise_density = 0.7;
            slot.resonator_mix = 0.5;
            slot.mode_tilt = 0.4;
            slot.cymbal_shape = 0.5;
            slot.snare_wire = 0.0;
            slot.snare_wire_decay = 0.0;
            slot.drive = 0.2;
            slot.transient = 0.6;
            slot.body = 0.2;
            slot.velocity_sensitivity = 0.8;
            slot.tone_low = 0.0;
            slot.tone_mid = 1.0;
            slot.tone_high = 2.0;
            slot.pan_lfo_rate = 2.0;
            slot.pan_lfo_depth = 0.2;
        }
    }

    if index == 12 {
        slot.pan = 0.4;
    } else if index == 13 {
        slot.pan = 0.6;
    }
}

fn apply_kit_style(preset: DrumKitPreset, index: usize, slot: &mut DrumSlotPreset) {
    apply_kit_global(preset, slot);
    match preset {
        DrumKitPreset::Studio | DrumKitPreset::StudioFactory => {
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.decay *= 0.65;
                slot.damping = (slot.damping + 0.18).clamp(0.0, 1.0);
                slot.resonator_mix = (slot.resonator_mix - 0.2).clamp(0.0, 1.0);
                slot.noise = (slot.noise + 0.02).clamp(0.0, 1.0);
                slot.noise_color = (slot.noise_color - 0.2).clamp(0.0, 1.0);
                slot.noise_density = (slot.noise_density - 0.15).clamp(0.0, 1.0);
                slot.cymbal_shape = (slot.cymbal_shape - 0.3).clamp(0.0, 1.0);
                slot.transient = (slot.transient + 0.05).clamp(0.0, 1.0);
                slot.tone_high = (slot.tone_high - 1.0).clamp(-60.0, 12.0);
            }
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.damping = (slot.damping + 0.1).clamp(0.0, 1.0);
                slot.transient = (slot.transient - 0.05).clamp(0.0, 1.0);
            }
        }
        DrumKitPreset::EightBit => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay = 0.35;
                slot.tune = -6.0;
                slot.pitch_env = 0.8;
                slot.pitch_decay = 0.16;
                slot.noise = 0.03;
                slot.resonator_mix = 0.92;
                slot.tone_mid = 2.4;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay = 0.32;
                slot.noise = 0.28;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.3;
                slot.noise_density = 0.3;
                slot.pitch_env = 0.25;
                slot.pitch_decay = 0.18;
                slot.tone_mid = 2.4;
                slot.tone_high = 2.2;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay *= 0.5;
                slot.damping = (slot.damping + 0.25).clamp(0.0, 1.0);
                slot.noise = 0.18;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.25;
                slot.noise_density = 0.25;
                slot.cymbal_shape = 0.15;
                slot.resonator_mix = 0.8;
                slot.tone_high = 1.0;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay = 0.22;
                slot.damping = 0.65;
                slot.noise = 0.2;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.2;
                slot.noise_density = 0.2;
                slot.resonator_mix = 0.82;
                slot.mode_tilt = 0.15;
                slot.tune = if index == 12 { 10.0 } else { 13.0 };
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay = 0.25;
                slot.damping = 0.7;
                slot.noise = 0.2;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.2;
                slot.noise_density = 0.2;
                slot.resonator_mix = 0.78;
                slot.mode_tilt = 0.2;
                slot.pitch_env = 0.4;
                slot.pitch_decay = 0.18;
                slot.pan_lfo_depth = 0.4;
                slot.pan_lfo_rate = 2.4;
            }
        }
        DrumKitPreset::Tr808 => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.9;
                slot.tune = -8.0;
                slot.drive = 0.25;
                slot.pitch_env = 0.75;
                slot.pitch_decay = 0.2;
                slot.transient = 0.75;
                slot.tone_low = 4.5;
                slot.tone_mid = -3.5;
                slot.tone_high = -5.0;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise_color = 0.82;
                slot.noise_density = 0.7;
                slot.tone_mid = 1.8;
                slot.tone_high = 2.8;
                slot.decay = 0.5;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.decay *= 0.9;
                slot.noise = 0.9;
                slot.noise_color = 0.95;
                slot.noise_density = 0.92;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.2;
                slot.damping = 0.55;
                slot.noise = 0.7;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.8;
                slot.noise_density = 0.75;
                slot.exciter_mix = 0.75;
                slot.resonator_mix = 0.4;
                slot.mode_tilt = 0.45;
                slot.cymbal_shape = 0.5;
                if index == 12 {
                    slot.tune = 4.0;
                    slot.pan = 0.35;
                } else {
                    slot.tune = 7.0;
                    slot.pan = 0.65;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Composite;
                slot.decay = 0.35;
                slot.damping = 0.6;
                slot.noise = 0.55;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.7;
                slot.noise_density = 0.6;
                slot.exciter_mix = 0.6;
                slot.resonator_mix = 0.45;
                slot.mode_tilt = 0.2;
                slot.pitch_env = 0.15;
                slot.pitch_decay = 0.3;
                slot.pan_lfo_depth = 0.3;
                slot.pan_lfo_rate = 1.8;
            }
        }
        DrumKitPreset::Jazz => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.45;
                slot.damping = 0.7;
                slot.pitch_env = 0.25;
                slot.transient = 0.35;
                slot.tone_low = 2.0;
                slot.tone_high = -3.0;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.exciter = ExciterType::Brush;
                slot.noise = 0.6;
                slot.noise_color = 0.6;
                slot.noise_density = 0.6;
                slot.snare_wire = 0.8;
                slot.snare_wire_decay = 0.65;
                slot.transient = 0.45;
                slot.tone_high = 0.8;
                slot.decay = 0.55;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.75;
                slot.noise_color = 0.82;
                slot.noise_density = 0.75;
                slot.decay *= 1.05;
                slot.tone_high -= 0.8;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Plate;
                slot.material = MaterialType::Wood;
                slot.decay = 0.18;
                slot.damping = 0.6;
                slot.noise = 0.55;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.55;
                slot.noise_density = 0.55;
                slot.exciter_mix = 0.6;
                slot.resonator_mix = 0.35;
                slot.mode_tilt = 0.25;
                slot.cymbal_shape = 0.35;
                if index == 12 {
                    slot.tune = 2.0;
                    slot.pan = 0.4;
                } else {
                    slot.tune = 5.0;
                    slot.pan = 0.6;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Plate;
                slot.material = MaterialType::Wood;
                slot.decay = 0.25;
                slot.damping = 0.7;
                slot.noise = 0.45;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.5;
                slot.noise_density = 0.5;
                slot.exciter_mix = 0.5;
                slot.resonator_mix = 0.3;
                slot.mode_tilt = 0.1;
                slot.pitch_env = 0.1;
                slot.pitch_decay = 0.25;
                slot.pan_lfo_depth = 0.2;
                slot.pan_lfo_rate = 1.4;
            }
        }
        DrumKitPreset::House => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.65;
                slot.drive = 0.25;
                slot.tune = -2.0;
                slot.pitch_env = 0.45;
                slot.pitch_decay = 0.3;
                slot.transient = 0.6;
                slot.tone_low = 3.2;
                slot.tone_mid = -1.0;
                slot.exciter_mix = 0.25;
                slot.resonator_mix = 0.9;
                slot.mode_tilt = 0.1;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.78;
                slot.noise_color = 0.85;
                slot.noise_density = 0.7;
                slot.snare_wire = 0.9;
                slot.snare_wire_decay = 0.55;
                slot.tone_high = 3.0;
                slot.decay = 0.42;
                slot.exciter_mix = 0.7;
                slot.resonator_mix = 0.6;
                slot.mode_tilt = 0.35;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.9;
                slot.noise_color = 0.95;
                slot.noise_density = 0.9;
                slot.tone_high = 4.0;
                slot.exciter_mix = 0.85;
                slot.resonator_mix = 0.4;
                slot.mode_tilt = 0.5;
            }
            if matches!(
                slot.instrument,
                InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh
            ) {
                slot.decay = 0.4;
                slot.damping = 0.55;
                slot.transient = 0.6;
                slot.pitch_env = 0.35;
                slot.pitch_decay = 0.22;
                slot.exciter_mix = 0.3;
                slot.resonator_mix = 0.8;
                slot.mode_tilt = 0.15;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.2;
                slot.damping = 0.5;
                slot.noise = 0.75;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.85;
                slot.noise_density = 0.8;
                slot.exciter_mix = 0.8;
                slot.resonator_mix = 0.5;
                slot.mode_tilt = 0.5;
                slot.cymbal_shape = 0.55;
                if index == 12 {
                    slot.tune = 5.0;
                } else {
                    slot.tune = 8.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Composite;
                slot.decay = 0.3;
                slot.damping = 0.6;
                slot.noise = 0.6;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.7;
                slot.noise_density = 0.65;
                slot.exciter_mix = 0.65;
                slot.resonator_mix = 0.4;
                slot.mode_tilt = 0.3;
                slot.pitch_env = 0.2;
                slot.pitch_decay = 0.3;
                slot.pan_lfo_depth = 0.3;
                slot.pan_lfo_rate = 1.6;
            }
        }
        DrumKitPreset::Dubstep => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.85;
                slot.tune = -10.0;
                slot.drive = 0.5;
                slot.pitch_env = 0.8;
                slot.pitch_decay = 0.2;
                slot.transient = 0.8;
                slot.tone_low = 4.5;
                slot.tone_mid = -2.5;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.9;
                slot.noise_color = 0.9;
                slot.noise_density = 0.8;
                slot.snare_wire = 0.95;
                slot.snare_wire_decay = 0.5;
                slot.tone_high = 3.4;
                slot.decay = 0.55;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.95;
                slot.noise_color = 0.98;
                slot.noise_density = 1.0;
                slot.tone_high = 4.8;
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.drive = 0.4;
                slot.pan_lfo_depth = 0.35;
                slot.pan_lfo_rate = 2.2;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.22;
                slot.damping = 0.45;
                slot.noise = 0.8;
                slot.noise_type = NoiseType::Metallic;
                slot.noise_color = 0.95;
                slot.noise_density = 0.9;
                slot.exciter_mix = 0.9;
                slot.resonator_mix = 0.55;
                slot.mode_tilt = 0.6;
                slot.cymbal_shape = 0.7;
                if index == 12 {
                    slot.tune = 6.0;
                } else {
                    slot.tune = 10.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Composite;
                slot.decay = 0.4;
                slot.damping = 0.5;
                slot.noise = 0.7;
                slot.noise_type = NoiseType::Metallic;
                slot.noise_color = 0.9;
                slot.noise_density = 0.85;
                slot.exciter_mix = 0.85;
                slot.resonator_mix = 0.5;
                slot.mode_tilt = 0.55;
                slot.pitch_env = 0.25;
                slot.pitch_decay = 0.35;
                slot.pan_lfo_depth = 0.45;
                slot.pan_lfo_rate = 2.0;
            }
        }
        DrumKitPreset::Electronica => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.7;
                slot.drive = 0.2;
                slot.pitch_env = 0.4;
                slot.pitch_decay = 0.35;
                slot.transient = 0.55;
                slot.tone_low = 2.8;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.75;
                slot.noise_color = 0.78;
                slot.noise_density = 0.7;
                slot.snare_wire = 0.85;
                slot.snare_wire_decay = 0.6;
                slot.tone_high = 2.6;
                slot.decay = 0.48;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.85;
                slot.noise_color = 0.9;
                slot.noise_density = 0.85;
                slot.tone_high = 3.6;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.pan_lfo_depth = 0.2;
                slot.pan_lfo_rate = 1.8;
                if index == 13 {
                    slot.tune = 7.0;
                    slot.pan_lfo_depth = 0.25;
                    slot.pan_lfo_rate = 2.0;
                }
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.24;
                slot.damping = 0.5;
                slot.noise = 0.75;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.9;
                slot.noise_density = 0.85;
                slot.exciter_mix = 0.8;
                slot.resonator_mix = 0.5;
                slot.mode_tilt = 0.5;
                slot.cymbal_shape = 0.6;
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Composite;
                slot.decay = 0.32;
                slot.damping = 0.55;
                slot.noise = 0.65;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.75;
                slot.noise_density = 0.7;
                slot.exciter_mix = 0.7;
                slot.resonator_mix = 0.45;
                slot.mode_tilt = 0.35;
                slot.pitch_env = 0.2;
                slot.pitch_decay = 0.3;
                slot.pan_lfo_depth = 0.3;
                slot.pan_lfo_rate = 1.8;
            }
        }
        DrumKitPreset::Industrial => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.8;
                slot.drive = 0.6;
                slot.damping = 0.3;
                slot.pitch_env = 0.6;
                slot.pitch_decay = 0.25;
                slot.transient = 0.9;
                slot.tone_low = 3.5;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.9;
                slot.noise_color = 0.95;
                slot.noise_density = 0.85;
                slot.snare_wire = 0.95;
                slot.snare_wire_decay = 0.45;
                slot.tone_high = 3.6;
                slot.decay = 0.6;
                slot.drive = 0.35;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 1.0;
                slot.noise_color = 1.0;
                slot.noise_density = 1.0;
                slot.tone_high = 5.0;
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.drive = 0.6;
                slot.pan_lfo_depth = 0.5;
                slot.pan_lfo_rate = 2.4;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.25;
                slot.damping = 0.4;
                slot.noise = 0.85;
                slot.noise_type = NoiseType::Metallic;
                slot.noise_color = 1.0;
                slot.noise_density = 0.95;
                slot.exciter_mix = 0.95;
                slot.resonator_mix = 0.6;
                slot.mode_tilt = 0.65;
                slot.cymbal_shape = 0.75;
                slot.drive = 0.35;
                if index == 12 {
                    slot.tune = 7.0;
                } else {
                    slot.tune = 11.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Metal;
                slot.decay = 0.45;
                slot.damping = 0.4;
                slot.noise = 0.8;
                slot.noise_type = NoiseType::Metallic;
                slot.noise_color = 0.95;
                slot.noise_density = 0.9;
                slot.exciter_mix = 0.9;
                slot.resonator_mix = 0.55;
                slot.mode_tilt = 0.6;
                slot.pitch_env = 0.3;
                slot.pitch_decay = 0.35;
                slot.drive = 0.5;
                slot.pan_lfo_depth = 0.6;
                slot.pan_lfo_rate = 2.6;
            }
        }
        DrumKitPreset::VideoGame => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.exciter = ExciterType::Impulse;
                slot.resonator = ResonatorType::Tube;
                slot.decay = 0.5;
                slot.tune = -5.0;
                slot.pitch_env = 0.5;
                slot.pitch_decay = 0.3;
                slot.transient = 0.65;
                slot.tone_mid = 1.0;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.resonator = ResonatorType::Membrane;
                slot.noise = 0.65;
                slot.noise_color = 0.7;
                slot.noise_density = 0.6;
                slot.snare_wire = 0.75;
                slot.snare_wire_decay = 0.65;
                slot.tone_mid = 1.0;
                slot.tone_high = 1.5;
                slot.decay = 0.4;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.8;
                slot.noise_color = 0.85;
                slot.noise_density = 0.8;
                slot.tone_high = 2.8;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.tune = if index == 12 { 12.0 } else { 14.0 };
                slot.pan_lfo_depth = 0.35;
                slot.pan_lfo_rate = if index == 12 { 3.0 } else { 2.8 };
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay = 0.18;
                slot.damping = 0.65;
                slot.noise = 0.6;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.7;
                slot.noise_density = 0.7;
                slot.exciter_mix = 0.7;
                slot.resonator_mix = 0.35;
                slot.mode_tilt = 0.4;
                slot.cymbal_shape = 0.45;
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Plastic;
                slot.decay = 0.22;
                slot.damping = 0.7;
                slot.noise = 0.55;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.65;
                slot.noise_density = 0.65;
                slot.exciter_mix = 0.65;
                slot.resonator_mix = 0.3;
                slot.mode_tilt = 0.3;
                slot.pitch_env = 0.2;
                slot.pitch_decay = 0.25;
                slot.pan_lfo_depth = 0.4;
                slot.pan_lfo_rate = 2.6;
            }
        }
        DrumKitPreset::Acoustic => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.55;
                slot.damping = 0.55;
                slot.pitch_env = 0.25;
                slot.transient = 0.45;
                slot.tone_low = 2.0;
                slot.tone_high = -2.0;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.resonator = ResonatorType::Membrane;
                slot.noise = 0.65;
                slot.noise_color = 0.6;
                slot.noise_density = 0.6;
                slot.snare_wire = 0.85;
                slot.snare_wire_decay = 0.7;
                slot.transient = 0.5;
                slot.tone_mid = 0.8;
                slot.tone_high = 0.8;
                slot.decay = 0.5;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.75;
                slot.noise_color = 0.8;
                slot.noise_density = 0.75;
                slot.tone_high = 2.6;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Plate;
                slot.material = MaterialType::Wood;
                slot.decay = 0.2;
                slot.damping = 0.6;
                slot.noise = 0.55;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.6;
                slot.noise_density = 0.6;
                slot.exciter_mix = 0.6;
                slot.resonator_mix = 0.4;
                slot.mode_tilt = 0.2;
                slot.cymbal_shape = 0.4;
                if index == 12 {
                    slot.tune = 3.0;
                } else {
                    slot.tune = 6.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Plate;
                slot.material = MaterialType::Wood;
                slot.decay = 0.28;
                slot.damping = 0.65;
                slot.noise = 0.5;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.55;
                slot.noise_density = 0.55;
                slot.exciter_mix = 0.5;
                slot.resonator_mix = 0.35;
                slot.mode_tilt = 0.15;
                slot.pitch_env = 0.1;
                slot.pitch_decay = 0.2;
                slot.pan_lfo_depth = 0.2;
                slot.pan_lfo_rate = 1.4;
            }
        }
        DrumKitPreset::Trap => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.75;
                slot.drive = 0.35;
                slot.tune = -4.0;
                slot.pitch_env = 0.55;
                slot.pitch_decay = 0.25;
                slot.transient = 0.7;
                slot.tone_low = 3.8;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.85;
                slot.noise_color = 0.9;
                slot.noise_density = 0.8;
                slot.snare_wire = 0.95;
                slot.snare_wire_decay = 0.45;
                slot.tone_high = 3.2;
                slot.decay = 0.38;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.95;
                slot.noise_color = 0.98;
                slot.noise_density = 0.98;
                slot.tone_high = 5.0;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.18;
                slot.damping = 0.5;
                slot.noise = 0.85;
                slot.noise_type = NoiseType::Metallic;
                slot.noise_color = 0.95;
                slot.noise_density = 0.9;
                slot.exciter_mix = 0.9;
                slot.resonator_mix = 0.5;
                slot.mode_tilt = 0.6;
                slot.cymbal_shape = 0.7;
                if index == 12 {
                    slot.tune = 6.0;
                } else {
                    slot.tune = 10.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Composite;
                slot.decay = 0.32;
                slot.damping = 0.55;
                slot.noise = 0.7;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.8;
                slot.noise_density = 0.75;
                slot.exciter_mix = 0.75;
                slot.resonator_mix = 0.45;
                slot.mode_tilt = 0.45;
                slot.pitch_env = 0.25;
                slot.pitch_decay = 0.3;
                slot.pan_lfo_depth = 0.4;
                slot.pan_lfo_rate = 2.0;
            }
        }
        DrumKitPreset::LoFi => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.6;
                slot.damping = 0.7;
                slot.drive = 0.15;
                slot.tone_low = 1.5;
                slot.tone_high = -6.0;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.55;
                slot.noise_color = 0.5;
                slot.noise_density = 0.5;
                slot.snare_wire = 0.7;
                slot.snare_wire_decay = 0.75;
                slot.tone_high = -1.0;
                slot.decay = 0.5;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.6;
                slot.noise_color = 0.55;
                slot.noise_density = 0.55;
                slot.tone_high = 1.6;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Plate;
                slot.material = MaterialType::Wood;
                slot.decay = 0.22;
                slot.damping = 0.7;
                slot.noise = 0.5;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.5;
                slot.noise_density = 0.5;
                slot.exciter_mix = 0.55;
                slot.resonator_mix = 0.35;
                slot.mode_tilt = 0.15;
                slot.cymbal_shape = 0.35;
                if index == 12 {
                    slot.tune = 2.0;
                } else {
                    slot.tune = 5.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Plate;
                slot.material = MaterialType::Wood;
                slot.decay = 0.28;
                slot.damping = 0.75;
                slot.noise = 0.45;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.45;
                slot.noise_density = 0.45;
                slot.exciter_mix = 0.5;
                slot.resonator_mix = 0.3;
                slot.mode_tilt = 0.1;
                slot.pitch_env = 0.1;
                slot.pitch_decay = 0.2;
                slot.pan_lfo_depth = 0.25;
                slot.pan_lfo_rate = 1.2;
            }
        }
        DrumKitPreset::Techno => {
            if matches!(slot.instrument, InstrumentType::Kick) {
                slot.decay = 0.7;
                slot.drive = 0.25;
                slot.pitch_env = 0.5;
                slot.pitch_decay = 0.28;
                slot.transient = 0.65;
                slot.tone_low = 3.2;
            }
            if matches!(slot.instrument, InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot) {
                slot.noise = 0.8;
                slot.noise_color = 0.85;
                slot.noise_density = 0.75;
                slot.snare_wire = 0.9;
                slot.snare_wire_decay = 0.55;
                slot.tone_high = 2.8;
                slot.decay = 0.45;
            }
            if DrumKitPreset::is_hat_like(slot.instrument) {
                slot.noise = 0.9;
                slot.noise_color = 0.95;
                slot.noise_density = 0.9;
                slot.tone_high = 4.5;
            }
            if matches!(slot.instrument, InstrumentType::Perc1 | InstrumentType::Perc2) {
                slot.resonator = ResonatorType::Metallic;
                slot.material = MaterialType::Metal;
                slot.decay = 0.2;
                slot.damping = 0.5;
                slot.noise = 0.8;
                slot.noise_type = NoiseType::White;
                slot.noise_color = 0.9;
                slot.noise_density = 0.85;
                slot.exciter_mix = 0.85;
                slot.resonator_mix = 0.5;
                slot.mode_tilt = 0.55;
                slot.cymbal_shape = 0.6;
                if index == 12 {
                    slot.tune = 5.0;
                } else {
                    slot.tune = 9.0;
                }
            }
            if matches!(slot.instrument, InstrumentType::Fx1 | InstrumentType::Fx2) {
                slot.resonator = ResonatorType::Tube;
                slot.material = MaterialType::Composite;
                slot.decay = 0.3;
                slot.damping = 0.55;
                slot.noise = 0.65;
                slot.noise_type = NoiseType::Pink;
                slot.noise_color = 0.8;
                slot.noise_density = 0.75;
                slot.exciter_mix = 0.7;
                slot.resonator_mix = 0.4;
                slot.mode_tilt = 0.4;
                slot.pitch_env = 0.2;
                slot.pitch_decay = 0.28;
                slot.pan_lfo_depth = 0.35;
                slot.pan_lfo_rate = 1.8;
            }
        }
    }

    if index >= 8 {
        apply_perc_layer_variation(index, slot);
    }
}

fn apply_perc_layer_variation(index: usize, slot: &mut DrumSlotPreset) {
    let (tune_offset, decay_add, noise_add, tilt_add, exc_mix_add, trans_add) = match index {
        8 => (1.0, -0.03, 0.05, 0.05, 0.08, 0.05),
        9 => (-1.0, -0.05, -0.02, -0.02, -0.05, 0.02),
        10 => (0.5, -0.04, -0.01, 0.0, -0.03, 0.03),
        11 => (1.5, -0.03, 0.0, 0.02, -0.02, 0.04),
        12 => (2.5, -0.02, 0.05, 0.08, 0.1, 0.02),
        13 => (4.0, -0.02, 0.06, 0.1, 0.1, 0.02),
        14 => (5.0, 0.02, 0.08, 0.12, 0.12, 0.04),
        _ => (7.0, 0.03, 0.1, 0.14, 0.12, 0.05),
    };

    slot.tune = (slot.tune + tune_offset).clamp(-24.0, 24.0);
    slot.decay = (slot.decay + decay_add).clamp(0.01, 1.0);
    slot.noise = (slot.noise + noise_add).clamp(0.0, 1.0);
    slot.mode_tilt = (slot.mode_tilt + tilt_add).clamp(-1.0, 1.0);
    slot.exciter_mix = (slot.exciter_mix + exc_mix_add).clamp(0.0, 1.0);
    slot.transient = (slot.transient + trans_add).clamp(0.0, 1.0);
}

fn apply_kit_global(preset: DrumKitPreset, slot: &mut DrumSlotPreset) {
    let (decay_mul, noise_add, noise_color_add, tone_high_add, res_mix_add, mode_tilt_add,
        exciter_mix_add, transient_add, drive_add) = match preset {
        DrumKitPreset::Studio | DrumKitPreset::StudioFactory => {
            (1.0, -0.02, -0.08, -0.3, -0.05, -0.05, 0.0, -0.02, 0.0)
        }
        DrumKitPreset::EightBit => (0.9, -0.08, -0.12, 0.4, 0.1, -0.05, 0.05, 0.0, 0.0),
        DrumKitPreset::Tr808 => (0.95, 0.05, 0.03, 0.6, -0.05, 0.05, 0.05, 0.05, 0.02),
        DrumKitPreset::Jazz => (1.05, -0.05, -0.05, -0.6, -0.08, -0.05, -0.05, -0.05, 0.0),
        DrumKitPreset::House => (0.98, 0.04, 0.06, 0.8, 0.0, 0.1, 0.06, 0.06, 0.03),
        DrumKitPreset::Dubstep => (1.05, 0.08, 0.1, 1.0, 0.05, 0.15, 0.1, 0.08, 0.06),
        DrumKitPreset::Electronica => (1.0, 0.03, 0.06, 0.7, 0.02, 0.12, 0.06, 0.04, 0.02),
        DrumKitPreset::Industrial => (1.1, 0.1, 0.12, 1.2, 0.08, 0.2, 0.12, 0.1, 0.08),
        DrumKitPreset::VideoGame => (0.9, 0.02, 0.02, 0.4, -0.05, 0.05, 0.04, 0.02, 0.01),
        DrumKitPreset::Acoustic => (1.0, -0.02, -0.04, -0.4, -0.06, -0.05, -0.04, -0.03, 0.0),
        DrumKitPreset::Trap => (0.95, 0.08, 0.1, 1.1, 0.04, 0.18, 0.1, 0.08, 0.05),
        DrumKitPreset::LoFi => (1.05, -0.08, -0.1, -1.0, -0.1, -0.12, -0.08, -0.06, -0.02),
        DrumKitPreset::Techno => (0.98, 0.06, 0.08, 0.9, 0.02, 0.14, 0.08, 0.06, 0.04),
    };

    slot.decay = (slot.decay * decay_mul).clamp(0.01, 1.0);
    slot.noise = (slot.noise + noise_add).clamp(0.0, 1.0);
    slot.noise_color = (slot.noise_color + noise_color_add).clamp(0.0, 1.0);
    slot.tone_high = (slot.tone_high + tone_high_add).clamp(-60.0, 12.0);
    slot.resonator_mix = (slot.resonator_mix + res_mix_add).clamp(0.0, 1.0);
    slot.mode_tilt = (slot.mode_tilt + mode_tilt_add).clamp(-1.0, 1.0);
    slot.exciter_mix = (slot.exciter_mix + exciter_mix_add).clamp(0.0, 1.0);
    slot.transient = (slot.transient + transient_add).clamp(0.0, 1.0);
    slot.drive = (slot.drive + drive_add).clamp(0.0, 1.0);
}

impl DrumKitPreset {
    fn is_hat_like(instrument: InstrumentType) -> bool {
        matches!(
            instrument,
            InstrumentType::HatClosed
                | InstrumentType::HatOpen
                | InstrumentType::HatPedal
                | InstrumentType::Ride
                | InstrumentType::Crash
        )
    }
}

fn drum_kit_preset_path() -> PathBuf {
    preset_root().join("drum_kits.json")
}

fn load_user_drum_kits() -> HashMap<String, DrumKitSnapshot> {
    let path = drum_kit_preset_path();
    let Ok(contents) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_user_drum_kits(kits: &HashMap<String, DrumKitSnapshot>) -> Result<(), String> {
    let path = drum_kit_preset_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(kits).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

fn slot_snapshot_from_preset(
    slot: &DrumSlotParams,
    preset: &DrumSlotPreset,
) -> DrumSlotSnapshot {
    DrumSlotSnapshot {
        instrument: normalized(&slot.instrument, preset.instrument),
        exciter: normalized(&slot.exciter, preset.exciter),
        exciter_mix: normalized(&slot.exciter_mix, preset.exciter_mix),
        resonator: normalized(&slot.resonator, preset.resonator),
        material: normalized(&slot.material, preset.material),
        level: normalized(&slot.level, preset.level),
        pan: normalized(&slot.pan, preset.pan),
        tune: normalized(&slot.tune, preset.tune),
        decay: normalized(&slot.decay, preset.decay),
        damping: normalized(&slot.damping, preset.damping),
        spread: normalized(&slot.spread, preset.spread),
        strike_position: normalized(&slot.strike_position, preset.strike_position),
        strike_hardness: normalized(&slot.strike_hardness, preset.strike_hardness),
        attack: normalized(&slot.attack, preset.attack),
        pitch_env: normalized(&slot.pitch_env, preset.pitch_env),
        pitch_decay: normalized(&slot.pitch_decay, preset.pitch_decay),
        noise: normalized(&slot.noise, preset.noise),
        diffusion: normalized(&slot.diffusion, preset.diffusion),
        noise_color: normalized(&slot.noise_color, preset.noise_color),
        noise_type: normalized(&slot.noise_type, preset.noise_type),
        noise_density: normalized(&slot.noise_density, preset.noise_density),
        resonator_mix: normalized(&slot.resonator_mix, preset.resonator_mix),
        mode_tilt: normalized(&slot.mode_tilt, preset.mode_tilt),
        cymbal_shape: normalized(&slot.cymbal_shape, preset.cymbal_shape),
        snare_wire: normalized(&slot.snare_wire, preset.snare_wire),
        snare_wire_decay: normalized(&slot.snare_wire_decay, preset.snare_wire_decay),
        drive: normalized(&slot.drive, preset.drive),
        transient: normalized(&slot.transient, preset.transient),
        body: normalized(&slot.body, preset.body),
        velocity_sensitivity: normalized(&slot.velocity_sensitivity, preset.velocity_sensitivity),
        tone_low: normalized(&slot.tone_low, preset.tone_low),
        tone_mid: normalized(&slot.tone_mid, preset.tone_mid),
        tone_high: normalized(&slot.tone_high, preset.tone_high),
        pan_lfo_rate: normalized(&slot.pan_lfo_rate, preset.pan_lfo_rate),
        pan_lfo_depth: normalized(&slot.pan_lfo_depth, preset.pan_lfo_depth),
    }
}

fn default_kit_snapshot(params: &DrumSynthParams, kit: DrumKitPreset) -> DrumKitSnapshot {
    let presets = kit_slots(kit);
    let slots = std::array::from_fn(|index| {
        let preset = &presets[index];
        slot_snapshot_from_preset(&params.slots[index], preset)
    });
    DrumKitSnapshot { slots }
}

fn snapshot_from_params(params: &DrumSynthParams) -> DrumKitSnapshot {
    let slots = std::array::from_fn(|index| {
        let slot = &params.slots[index];
        DrumSlotSnapshot {
            instrument: slot.instrument.unmodulated_normalized_value(),
            exciter: slot.exciter.unmodulated_normalized_value(),
            exciter_mix: slot.exciter_mix.unmodulated_normalized_value(),
            resonator: slot.resonator.unmodulated_normalized_value(),
            material: slot.material.unmodulated_normalized_value(),
            level: slot.level.unmodulated_normalized_value(),
            pan: slot.pan.unmodulated_normalized_value(),
            tune: slot.tune.unmodulated_normalized_value(),
            decay: slot.decay.unmodulated_normalized_value(),
            damping: slot.damping.unmodulated_normalized_value(),
            spread: slot.spread.unmodulated_normalized_value(),
            strike_position: slot.strike_position.unmodulated_normalized_value(),
            strike_hardness: slot.strike_hardness.unmodulated_normalized_value(),
            attack: slot.attack.unmodulated_normalized_value(),
            pitch_env: slot.pitch_env.unmodulated_normalized_value(),
            pitch_decay: slot.pitch_decay.unmodulated_normalized_value(),
            noise: slot.noise.unmodulated_normalized_value(),
            diffusion: slot.diffusion.unmodulated_normalized_value(),
            noise_color: slot.noise_color.unmodulated_normalized_value(),
            noise_type: slot.noise_type.unmodulated_normalized_value(),
            noise_density: slot.noise_density.unmodulated_normalized_value(),
            resonator_mix: slot.resonator_mix.unmodulated_normalized_value(),
            mode_tilt: slot.mode_tilt.unmodulated_normalized_value(),
            cymbal_shape: slot.cymbal_shape.unmodulated_normalized_value(),
            snare_wire: slot.snare_wire.unmodulated_normalized_value(),
            snare_wire_decay: slot.snare_wire_decay.unmodulated_normalized_value(),
            drive: slot.drive.unmodulated_normalized_value(),
            transient: slot.transient.unmodulated_normalized_value(),
            body: slot.body.unmodulated_normalized_value(),
            velocity_sensitivity: slot.velocity_sensitivity.unmodulated_normalized_value(),
            tone_low: slot.tone_low.unmodulated_normalized_value(),
            tone_mid: slot.tone_mid.unmodulated_normalized_value(),
            tone_high: slot.tone_high.unmodulated_normalized_value(),
            pan_lfo_rate: slot.pan_lfo_rate.unmodulated_normalized_value(),
            pan_lfo_depth: slot.pan_lfo_depth.unmodulated_normalized_value(),
        }
    });
    DrumKitSnapshot { slots }
}

fn apply_drum_kit_snapshot(cx: &mut EventContext, params: &DrumSynthParams, kit: &DrumKitSnapshot) {
    for (index, snapshot) in kit.slots.iter().enumerate() {
        let slot = &params.slots[index];
        let default_instrument = default_instrument_for_slot(index);
        let default_note = default_note_for_slot(index) as i32;
        apply_param(cx, &slot.instrument, normalized(&slot.instrument, default_instrument));
        apply_param(cx, &slot.midi_note, normalized(&slot.midi_note, default_note));
        apply_param(cx, &slot.exciter, snapshot.exciter);
        apply_param(cx, &slot.exciter_mix, snapshot.exciter_mix);
        apply_param(cx, &slot.resonator, snapshot.resonator);
        apply_param(cx, &slot.material, snapshot.material);
        apply_param(cx, &slot.level, snapshot.level);
        apply_param(cx, &slot.pan, snapshot.pan);
        apply_param(cx, &slot.tune, snapshot.tune);
        apply_param(cx, &slot.decay, snapshot.decay);
        apply_param(cx, &slot.damping, snapshot.damping);
        apply_param(cx, &slot.spread, snapshot.spread);
        apply_param(cx, &slot.strike_position, snapshot.strike_position);
        apply_param(cx, &slot.strike_hardness, snapshot.strike_hardness);
        apply_param(cx, &slot.attack, snapshot.attack);
        apply_param(cx, &slot.pitch_env, snapshot.pitch_env);
        apply_param(cx, &slot.pitch_decay, snapshot.pitch_decay);
        apply_param(cx, &slot.noise, snapshot.noise);
        apply_param(cx, &slot.diffusion, snapshot.diffusion);
        apply_param(cx, &slot.noise_color, snapshot.noise_color);
        apply_param(cx, &slot.noise_type, snapshot.noise_type);
        apply_param(cx, &slot.noise_density, snapshot.noise_density);
        apply_param(cx, &slot.resonator_mix, snapshot.resonator_mix);
        apply_param(cx, &slot.mode_tilt, snapshot.mode_tilt);
        apply_param(cx, &slot.cymbal_shape, snapshot.cymbal_shape);
        apply_param(cx, &slot.snare_wire, snapshot.snare_wire);
        apply_param(cx, &slot.snare_wire_decay, snapshot.snare_wire_decay);
        apply_param(cx, &slot.drive, snapshot.drive);
        apply_param(cx, &slot.transient, snapshot.transient);
        apply_param(cx, &slot.body, snapshot.body);
        apply_param(cx, &slot.velocity_sensitivity, snapshot.velocity_sensitivity);
        apply_param(cx, &slot.tone_low, snapshot.tone_low);
        apply_param(cx, &slot.tone_mid, snapshot.tone_mid);
        apply_param(cx, &slot.tone_high, snapshot.tone_high);
        apply_param(cx, &slot.pan_lfo_rate, snapshot.pan_lfo_rate);
        apply_param(cx, &slot.pan_lfo_depth, snapshot.pan_lfo_depth);
    }
}

fn apply_drum_kit(cx: &mut EventContext, params: &DrumSynthParams, kit: DrumKitPreset) {
    let kit_snapshot = if matches!(kit, DrumKitPreset::StudioFactory) {
        default_kit_snapshot(params, kit)
    } else {
        let kits = load_user_drum_kits();
        kits.get(kit_name(kit))
            .cloned()
            .unwrap_or_else(|| default_kit_snapshot(params, kit))
    };
    apply_drum_kit_snapshot(cx, params, &kit_snapshot);
}

impl Model for DrumUiData {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if let Some(msg) = event.take::<DrumUiEvent>() {
            match msg {
                DrumUiEvent::SelectSlot(slot) => {
                    self.active_slot = slot.min(DRUM_SLOTS - 1);
                    self.active_bank = self.active_slot / DRUM_BANK_SIZE;
                }
                DrumUiEvent::PadTrigger(slot) => {
                    let slot_index = slot.min(DRUM_SLOTS - 1);
                    self.active_slot = slot_index;
                    self.active_bank = slot_index / DRUM_BANK_SIZE;
                    self.pad_counter = self.pad_counter.wrapping_add(1);
                    let value = ((self.pad_counter % 100) as f32) / 100.0;
                    let pad_param = &self.params.slots[slot_index].pad_trigger;
                    apply_param(cx, pad_param, value);
                }
                DrumUiEvent::SetTab(tab) => {
                    self.active_tab = tab;
                }
                DrumUiEvent::SetBank(bank) => {
                    let bank_index = bank.min(DRUM_BANK_COUNT.saturating_sub(1));
                    self.active_bank = bank_index;
                    let bank_start = bank_index * DRUM_BANK_SIZE;
                    if self.active_slot < bank_start
                        || self.active_slot >= bank_start + DRUM_BANK_SIZE
                    {
                        self.active_slot = bank_start;
                    }
                }
                DrumUiEvent::KitPrev => {
                    if self.kit_index == 0 {
                        self.kit_index = DRUM_KITS.len() - 1;
                    } else {
                        self.kit_index -= 1;
                    }
                    self.kit_display = kit_name(DRUM_KITS[self.kit_index]).to_string();
                    let kit = DRUM_KITS[self.kit_index];
                    apply_drum_kit(cx, &self.params, kit);
                }
                DrumUiEvent::KitNext => {
                    self.kit_index = (self.kit_index + 1) % DRUM_KITS.len();
                    self.kit_display = kit_name(DRUM_KITS[self.kit_index]).to_string();
                    let kit = DRUM_KITS[self.kit_index];
                    apply_drum_kit(cx, &self.params, kit);
                }
                DrumUiEvent::KitLoad => {
                    let kit = DRUM_KITS[self.kit_index];
                    apply_drum_kit(cx, &self.params, kit);
                }
                DrumUiEvent::KitSave => {
                    let kit = DRUM_KITS[self.kit_index];
                    if !matches!(kit, DrumKitPreset::StudioFactory) {
                        let mut kits = load_user_drum_kits();
                        kits.insert(kit_name(kit).to_string(), snapshot_from_params(&self.params));
                        let _ = save_user_drum_kits(&kits);
                    }
                }
                DrumUiEvent::Randomize => {
                    randomize_drum_kit(cx, &self.params);
                }
            }
        }
    }
}

fn randomize_drum_kit(cx: &mut EventContext, params: &DrumSynthParams) {
    let mut rng = rand::thread_rng();
    let seed_kit = DRUM_KITS[rng.gen_range(0..DRUM_KITS.len())];
    let mut slots = kit_slots(seed_kit);

    for slot in slots.iter_mut() {
        randomize_slot_organic(slot, &mut rng);
    }

    let snapshot_slots = std::array::from_fn(|index| {
        slot_snapshot_from_preset(&params.slots[index], &slots[index])
    });
    let snapshot = DrumKitSnapshot {
        slots: snapshot_slots,
    };
    apply_drum_kit_snapshot(cx, params, &snapshot);
}

fn randomize_slot_organic<R: Rng + ?Sized>(slot: &mut DrumSlotPreset, rng: &mut R) {
    let jitter = |value: f32, spread: f32, min: f32, max: f32, rng: &mut R| {
        (value + rng.gen_range(-spread..spread)).clamp(min, max)
    };

    slot.level = jitter(slot.level, 0.12, 0.45, 1.0, rng);
    slot.pan = jitter(slot.pan, 0.18, 0.0, 1.0, rng);
    slot.tune = jitter(slot.tune, 3.0, -12.0, 12.0, rng);
    slot.decay = jitter(slot.decay, 0.2, 0.04, 1.0, rng);
    slot.damping = jitter(slot.damping, 0.18, 0.05, 1.0, rng);
    slot.spread = jitter(slot.spread, 0.18, 0.0, 1.0, rng);
    slot.strike_position = jitter(slot.strike_position, 0.2, 0.0, 1.0, rng);
    slot.strike_hardness = jitter(slot.strike_hardness, 0.2, 0.0, 1.0, rng);
    slot.attack = jitter(slot.attack, 0.18, 0.0, 1.0, rng);
    slot.pitch_env = jitter(slot.pitch_env, 0.2, 0.0, 1.0, rng);
    slot.pitch_decay = jitter(slot.pitch_decay, 0.2, 0.0, 1.0, rng);
    slot.noise = jitter(slot.noise, 0.22, 0.0, 1.0, rng);
    slot.diffusion = jitter(slot.diffusion, 0.2, 0.0, 1.0, rng);
    slot.noise_color = jitter(slot.noise_color, 0.2, 0.0, 1.0, rng);
    slot.noise_density = jitter(slot.noise_density, 0.2, 0.0, 1.0, rng);
    slot.resonator_mix = jitter(slot.resonator_mix, 0.22, 0.0, 1.0, rng);
    slot.mode_tilt = jitter(slot.mode_tilt, 0.28, -1.0, 1.0, rng);
    slot.cymbal_shape = jitter(slot.cymbal_shape, 0.2, 0.0, 1.0, rng);
    slot.snare_wire = jitter(slot.snare_wire, 0.2, 0.0, 1.0, rng);
    slot.snare_wire_decay = jitter(slot.snare_wire_decay, 0.2, 0.0, 1.0, rng);
    slot.drive = jitter(slot.drive, 0.16, 0.0, 0.75, rng);
    slot.transient = jitter(slot.transient, 0.2, 0.0, 1.0, rng);
    slot.body = jitter(slot.body, 0.22, 0.0, 1.0, rng);
    slot.velocity_sensitivity = jitter(slot.velocity_sensitivity, 0.16, 0.0, 1.0, rng);
    slot.tone_low = jitter(slot.tone_low, 2.8, -12.0, 8.0, rng);
    slot.tone_mid = jitter(slot.tone_mid, 2.8, -12.0, 8.0, rng);
    slot.tone_high = jitter(slot.tone_high, 2.8, -12.0, 8.0, rng);
    slot.pan_lfo_rate = jitter(slot.pan_lfo_rate, 1.0, 0.1, 12.0, rng);
    slot.pan_lfo_depth = jitter(slot.pan_lfo_depth, 0.18, 0.0, 1.0, rng);

    if rng.gen_bool(0.2) {
        slot.exciter_mix = jitter(slot.exciter_mix, 0.2, 0.0, 1.0, rng);
    }
    if rng.gen_bool(0.12) {
        slot.material = match rng.gen_range(0..5) {
            0 => MaterialType::Skin,
            1 => MaterialType::Plastic,
            2 => MaterialType::Metal,
            3 => MaterialType::Wood,
            _ => MaterialType::Composite,
        };
    }
}

fn build_pad_grid(cx: &mut Context) {
    Binding::new(cx, DrumUiData::active_bank, |cx, bank| {
        let bank_index = bank.get(cx);
        VStack::new(cx, move |cx| {
            for row in 0..4 {
                HStack::new(cx, move |cx| {
                    for col in 0..4 {
                        let slot_index = bank_index * DRUM_BANK_SIZE + row * 4 + col;
                        Button::new(
                            cx,
                            move |cx| cx.emit(DrumUiEvent::PadTrigger(slot_index)),
                            move |cx| Label::new(cx, DRUM_SLOT_LABELS[slot_index]),
                        )
                        .width(Pixels(52.0))
                        .height(Pixels(52.0));
                    }
                })
                .col_between(Pixels(8.0));
            }
        })
        .row_between(Pixels(8.0))
        .height(Pixels(230.0));
    });
}

fn drum_tab_button(cx: &mut Context, label: &str, tab: usize) {
    Button::new(
        cx,
        move |cx| cx.emit(DrumUiEvent::SetTab(tab)),
        move |cx| Label::new(cx, label),
    )
    .height(Pixels(28.0))
    .width(Pixels(120.0));
}

fn build_drum_bank_panel(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "Bank")
            .font_size(16.0)
            .height(Pixels(24.0));
        HStack::new(cx, |cx| {
            Button::new(cx, move |cx| cx.emit(DrumUiEvent::SetBank(0)), |cx| {
                Label::new(cx, "A")
            })
            .width(Pixels(24.0))
            .height(Pixels(24.0));
            Button::new(cx, move |cx| cx.emit(DrumUiEvent::SetBank(1)), |cx| {
                Label::new(cx, "B")
            })
            .width(Pixels(24.0))
            .height(Pixels(24.0));
        })
        .col_between(Pixels(6.0));

        Binding::new(cx, DrumUiData::active_bank, |cx, bank| {
            let bank_index = bank.get(cx);
            for slot in 0..DRUM_BANK_SIZE {
                HStack::new(cx, move |cx| {
                    let slot_index = bank_index * DRUM_BANK_SIZE + slot;
                    Button::new(
                        cx,
                        move |cx| cx.emit(DrumUiEvent::SelectSlot(slot_index)),
                        move |cx| Label::new(cx, DRUM_SLOT_LABELS[slot_index]),
                    )
                    .width(Pixels(36.0))
                    .height(Pixels(24.0));

                    Binding::new(cx, DrumUiData::params, move |cx, params| {
                        let params = params.get(cx);
                        let name = instrument_label(params.slots[slot_index].instrument.value());
                        Label::new(cx, name).width(Pixels(110.0));
                    });

                    ParamSlider::new(cx, DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].level
                    })
                    .width(Pixels(100.0));

                    ParamSlider::new(cx, DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].midi_note
                    })
                    .width(Pixels(100.0));
                })
                .col_between(Pixels(6.0))
                .height(Pixels(26.0));
            }
        });
    })
    .row_between(Pixels(6.0))
    .width(Pixels(370.0));
}

fn labeled_param_slider<P, F>(
    cx: &mut Context,
    label: &str,
    params: impl Lens<Target = Arc<DrumSynthParams>> + Clone + 'static,
    make_param: F,
) where
    P: Param + 'static,
    F: Fn(&Arc<DrumSynthParams>) -> &P + Copy + 'static,
{
    HStack::new(cx, move |cx| {
        Label::new(cx, label)
            .font_size(11.0)
            .width(Pixels(64.0))
            .height(Pixels(18.0));
        ParamSlider::new(cx, params.clone(), make_param)
            .width(Percentage(50.0));
    })
    .col_between(Pixels(6.0));
}

fn build_drum_instrument_panel(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "Instrument")
            .font_size(16.0)
            .height(Pixels(24.0));
        Binding::new(cx, DrumUiData::active_slot, |cx, active| {
            let slot_index = active.get(cx);
            HStack::new(cx, move |cx| {
                VStack::new(cx, move |cx| {
                    Label::new(cx, "Main / Output")
                        .font_size(12.0)
                        .height(Pixels(18.0));
                    labeled_param_slider(cx, "Inst", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].instrument
                    });
                    labeled_param_slider(cx, "Note", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].midi_note
                    });
                    labeled_param_slider(cx, "Level", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].level
                    });
                    labeled_param_slider(cx, "Pan", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].pan
                    });
                    labeled_param_slider(cx, "Spread", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].spread
                    });
                    labeled_param_slider(cx, "Vel", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].velocity_sensitivity
                    });
                    labeled_param_slider(cx, "Drive", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].drive
                    });
                    labeled_param_slider(cx, "Pad", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].pad_trigger
                    });
                })
                .row_between(Pixels(6.0))
                .width(Stretch(1.0));

                VStack::new(cx, move |cx| {
                    Label::new(cx, "Pitch / Envelope")
                        .font_size(12.0)
                        .height(Pixels(18.0));
                    labeled_param_slider(cx, "Tune", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].tune
                    });
                    labeled_param_slider(cx, "Decay", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].decay
                    });
                    labeled_param_slider(cx, "Damp", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].damping
                    });
                    labeled_param_slider(cx, "Attack", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].attack
                    });
                    labeled_param_slider(cx, "Pitch Env", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].pitch_env
                    });
                    labeled_param_slider(cx, "Pitch Dec", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].pitch_decay
                    });
                    labeled_param_slider(cx, "Transient", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].transient
                    });
                    labeled_param_slider(cx, "Body", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].body
                    });
                })
                .row_between(Pixels(6.0))
                .width(Stretch(1.0));

                VStack::new(cx, move |cx| {
                    Label::new(cx, "Tone / Color")
                        .font_size(12.0)
                        .height(Pixels(18.0));
                    labeled_param_slider(cx, "Exciter", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].exciter
                    });
                    labeled_param_slider(cx, "Exc Mix", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].exciter_mix
                    });
                    labeled_param_slider(cx, "Reson", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].resonator
                    });
                    labeled_param_slider(cx, "Material", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].material
                    });
                    labeled_param_slider(cx, "Res Mix", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].resonator_mix
                    });
                    labeled_param_slider(cx, "Mode Tilt", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].mode_tilt
                    });
                    labeled_param_slider(cx, "Pos", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].strike_position
                    });
                    labeled_param_slider(cx, "Hard", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].strike_hardness
                    });
                    labeled_param_slider(cx, "Tone Low", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].tone_low
                    });
                    labeled_param_slider(cx, "Tone Mid", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].tone_mid
                    });
                    labeled_param_slider(cx, "Tone High", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].tone_high
                    });
                })
                .row_between(Pixels(6.0))
                .width(Stretch(1.0));

                VStack::new(cx, move |cx| {
                    Label::new(cx, "Noise / Mod")
                        .font_size(12.0)
                        .height(Pixels(18.0));
                    labeled_param_slider(cx, "Noise", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].noise
                    });
                    labeled_param_slider(cx, "Color", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].noise_color
                    });
                    labeled_param_slider(cx, "Type", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].noise_type
                    });
                    labeled_param_slider(cx, "Density", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].noise_density
                    });
                    labeled_param_slider(cx, "Cymbal", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].cymbal_shape
                    });
                    labeled_param_slider(cx, "Snare", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].snare_wire
                    });
                    labeled_param_slider(cx, "Wire Dec", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].snare_wire_decay
                    });
                    labeled_param_slider(cx, "Diffuse", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].diffusion
                    });
                    labeled_param_slider(cx, "Pan Rate", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].pan_lfo_rate
                    });
                    labeled_param_slider(cx, "Pan Depth", DrumUiData::params.clone(), move |params| {
                        &params.slots[slot_index].pan_lfo_depth
                    });
                })
                .row_between(Pixels(6.0))
                .width(Stretch(1.0));
            })
            .col_between(Pixels(12.0));
        });
    })
    .row_between(Pixels(8.0));
}

fn build_drum_sequencer_panel(cx: &mut Context) {
    VStack::new(cx, |cx| {
        Label::new(cx, "Sequencer")
            .font_size(16.0)
            .height(Pixels(24.0));
        HStack::new(cx, |cx| {
            ParamButton::new(cx, DrumUiData::params.clone(), |params| {
                &params.sequencer.enabled
            });
            ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                &params.sequencer.rate
            })
            .width(Pixels(160.0));
            ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                &params.sequencer.swing
            })
            .width(Pixels(160.0));
        })
        .col_between(Pixels(8.0))
        .height(Pixels(36.0));

        Binding::new(cx, DrumUiData::active_slot, |cx, active| {
            let slot_index = active.get(cx);
            HStack::new(cx, move |cx| {
                for step in 0..DRUM_STEPS {
                    let step_index = step;
                    ParamButton::new(cx, DrumUiData::params.clone(), move |params| {
                        &params.sequencer.lanes[slot_index].steps[step_index].gate
                    })
                    .width(Pixels(24.0))
                    .height(Pixels(24.0));
                }
            })
            .col_between(Pixels(4.0))
            .height(Pixels(28.0));

            HStack::new(cx, move |cx| {
                for step in 0..DRUM_STEPS {
                    let step_index = step;
                    ParamSlider::new(cx, DrumUiData::params.clone(), move |params| {
                        &params.sequencer.lanes[slot_index].steps[step_index].velocity
                    })
                    .width(Pixels(24.0))
                    .height(Pixels(80.0));
                }
            })
            .col_between(Pixels(4.0));
        });
    })
    .row_between(Pixels(8.0));
}

pub(crate) fn create_drum(
    params: Arc<DrumSynthParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_fonts_mem(&[ZCOOL_FONT_DATA]);
        cx.set_default_font(&[ZCOOL_XIAOWEI]);

        DrumUiData {
            params: params.clone(),
            active_slot: 0,
            active_tab: 0,
            active_bank: 0,
            kit_index: 0,
            kit_display: kit_name(DRUM_KITS[0]).to_string(),
            pad_counter: 0,
        }
        .build(cx);

        ResizeHandle::new(cx);

        VStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "CatSynth Drums")
                        .font_size(24.0)
                        .height(Pixels(36.0));
                    Element::new(cx).width(Stretch(1.0));
                    Label::new(cx, DrumUiData::kit_display)
                        .height(Pixels(26.0))
                        .width(Pixels(120.0));

                    HStack::new(cx, |cx| {
                        Button::new(
                            cx,
                            move |cx| cx.emit(DrumUiEvent::KitPrev),
                            |cx| Label::new(cx, "<"),
                        )
                        .height(Pixels(26.0))
                        .width(Pixels(28.0));

                        Button::new(
                            cx,
                            move |cx| cx.emit(DrumUiEvent::KitNext),
                            |cx| Label::new(cx, ">"),
                        )
                        .height(Pixels(26.0))
                        .width(Pixels(28.0));

                        Button::new(
                            cx,
                            move |cx| cx.emit(DrumUiEvent::KitLoad),
                            |cx| Label::new(cx, "Load"),
                        )
                        .height(Pixels(26.0))
                        .width(Pixels(56.0));

                        Button::new(
                            cx,
                            move |cx| cx.emit(DrumUiEvent::KitSave),
                            |cx| Label::new(cx, "Save"),
                        )
                        .height(Pixels(26.0))
                        .width(Pixels(56.0));

                        Button::new(
                            cx,
                            move |cx| cx.emit(DrumUiEvent::Randomize),
                            |cx| Label::new(cx, "Random"),
                        )
                        .height(Pixels(26.0))
                        .width(Pixels(72.0));
                    })
                    .col_between(Pixels(6.0))
                    .height(Pixels(28.0));
                })
                .col_between(Pixels(10.0))
                .height(Pixels(40.0));

                HStack::new(cx, |cx| {
                    Element::new(cx).width(Stretch(1.0));
                    ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                        &params.kit_preset
                    })
                    .width(Pixels(170.0));
                    ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                        &params.master_gain
                    })
                    .width(Pixels(110.0));
                    ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                        &params.master_drive
                    })
                    .width(Pixels(110.0));
                    ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                        &params.master_comp
                    })
                    .width(Pixels(110.0));
                    ParamSlider::new(cx, DrumUiData::params.clone(), |params| {
                        &params.master_clip
                    })
                    .width(Pixels(110.0));
                })
                .col_between(Pixels(10.0))
                .height(Pixels(34.0));
            })
            .row_between(Pixels(6.0));

            HStack::new(cx, |cx| {
                build_drum_bank_panel(cx);
                VStack::new(cx, |cx| {
                    build_pad_grid(cx);
                    HStack::new(cx, |cx| {
                        drum_tab_button(cx, "Instrument", 0);
                        drum_tab_button(cx, "Sequencer", 1);
                    })
                    .col_between(Pixels(8.0));

                    Binding::new(cx, DrumUiData::active_tab, |cx, tab| match tab.get(cx) {
                        0 => build_drum_instrument_panel(cx),
                        _ => build_drum_sequencer_panel(cx),
                    });
                })
                .row_between(Pixels(10.0))
                .width(Stretch(1.0));
            })
            .col_between(Pixels(14.0))
            .height(Stretch(1.0));
        })
        .row_between(Pixels(12.0))
        .size(Stretch(1.0));

        cx.emit(GuiContextEvent::Resize);
    })
}
                
