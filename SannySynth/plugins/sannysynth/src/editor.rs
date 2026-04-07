use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

use std::sync::Arc;
use std::{env, fs, path::Path, path::PathBuf};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OPENFILENAMEW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, OpenClipboard,
};
#[cfg(target_os = "windows")]
const CF_UNICODETEXT: u32 = 13;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

use crate::{
    preset_bank::{factory_presets, PresetData, PresetEntry},
    waveform::load_wavetable_from_file,
    SubSynthParams,
};

const ZCOOL_XIAOWEI: &str = "ZCOOL XiaoWei";
const ZCOOL_FONT_DATA: &[u8] = include_bytes!("assets/ZCOOL_XIAOWEI_REGULAR.ttf");

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (980, 720))
}

#[derive(Lens)]
struct Data {
    params: Arc<SubSynthParams>,
    active_tab: usize,
    presets: Vec<PresetEntry>,
    preset_index: usize,
    preset_display: String,
    preset_name: String,
    custom_wavetable_display: String,
    custom_wavetable_path_input: String,
}

#[derive(Debug)]
enum UiEvent {
    PresetPrev,
    PresetNext,
    PresetLoad,
    PresetSave,
    PresetRefresh,
    PresetNameChanged(String),
    SetTab(usize),
    CustomWavetablePathChanged(String),
    PasteCustomWavetablePath,
    LoadCustomWavetablePath,
}

impl Data {
    fn update_preset_display(&mut self) {
        self.preset_display = self
            .presets
            .get(self.preset_index)
            .map(|preset| preset.name.clone())
            .unwrap_or_else(|| "Init".to_string());
    }

    fn apply_preset(&mut self, cx: &mut EventContext, index: usize) {
        if self.presets.is_empty() {
            return;
        }

        let clamped = index.min(self.presets.len().saturating_sub(1));
        self.preset_index = clamped;
        self.update_preset_display();
        let preset = &self.presets[self.preset_index];
        preset.data.apply(cx, &self.params);
    }

    fn set_custom_wavetable_path(&mut self, path: String) {
        if let Ok(wavetable) = load_wavetable_from_file(Path::new(&path)) {
            if let Ok(mut data) = self.params.custom_wavetable_data.write() {
                *data = Some(wavetable);
            }
            if let Ok(mut store) = self.params.custom_wavetable_path.write() {
                *store = Some(path.clone());
            }
            self.custom_wavetable_display = PathBuf::from(&path)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "Custom wav".to_string());
            self.custom_wavetable_path_input = path;
        }
    }
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|message, _| match message {
            UiEvent::PresetPrev => {
                if self.presets.is_empty() {
                    return;
                }
                let next = if self.preset_index == 0 {
                    self.presets.len().saturating_sub(1)
                } else {
                    self.preset_index - 1
                };
                self.apply_preset(cx, next);
            }
            UiEvent::PresetNext => {
                if self.presets.is_empty() {
                    return;
                }
                let next = (self.preset_index + 1) % self.presets.len();
                self.apply_preset(cx, next);
            }
            UiEvent::PresetLoad => {
                self.apply_preset(cx, self.preset_index);
            }
            UiEvent::PresetSave => {
                let name = self.preset_name.trim();
                if name.is_empty() {
                    return;
                }
                let data = PresetData::from_params(&self.params);
                if save_user_preset(name, &data).is_ok() {
                    self.presets = load_presets(&self.params);
                    self.update_preset_display();
                }
            }
            UiEvent::PresetRefresh => {
                self.presets = load_presets(&self.params);
                self.preset_index = 0;
                self.update_preset_display();
            }
            UiEvent::PresetNameChanged(name) => {
                self.preset_name = name.clone();
            }
            UiEvent::SetTab(tab) => {
                self.active_tab = *tab;
            }
            UiEvent::CustomWavetablePathChanged(path) => {
                self.custom_wavetable_path_input = path.clone();
            }
            UiEvent::PasteCustomWavetablePath => {
                if let Some(path) = read_clipboard_text() {
                    self.set_custom_wavetable_path(path);
                }
            }
            UiEvent::LoadCustomWavetablePath => {
                if let Some(path) = open_wavetable_dialog() {
                    self.set_custom_wavetable_path(path);
                } else if !self.custom_wavetable_path_input.is_empty() {
                    self.set_custom_wavetable_path(self.custom_wavetable_path_input.clone());
                }
            }
        });
    }
}

fn preset_root() -> PathBuf {
    if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("SannySynth")
    } else if cfg!(target_os = "macos") {
        env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support/SannySynth")
    } else {
        env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".sannysynth")
    }
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

fn read_clipboard_text() -> Option<String> {
    #[cfg(target_os = "windows")]
    unsafe {
        if OpenClipboard(0) == 0 {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle == 0 {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle as *mut _) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16(slice).ok();
        GlobalUnlock(handle as *mut _);
        CloseClipboard();
        text
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

fn open_wavetable_dialog() -> Option<String> {
    #[cfg(target_os = "windows")]
    unsafe {
        let mut buffer = [0u16; 1024];
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFile = buffer.as_mut_ptr();
        ofn.nMaxFile = buffer.len() as u32;
        ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;

        if GetOpenFileNameW(&mut ofn) == 0 {
            return None;
        }

        let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        String::from_utf16(&buffer[..len]).ok()
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

fn create_label(cx: &mut Context, text: &str, height: f32, width: f32, top: f32, bottom: f32) {
    Label::new(cx, text)
        .height(Pixels(height))
        .width(Pixels(width))
        .child_top(Stretch(top))
        .child_bottom(Pixels(bottom));
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
            |cx| Label::new(cx, "Osc / Wavetable"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(1)),
            |cx| Label::new(cx, "Envelopes + Filters"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(2)),
            |cx| Label::new(cx, "Filters"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(3)),
            |cx| Label::new(cx, "LFO"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(4)),
            |cx| Label::new(cx, "FX"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(5)),
            |cx| Label::new(cx, "Resonator"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(6)),
            |cx| Label::new(cx, "Analog"),
        );
        Button::new(
            cx,
            |cx| cx.emit(UiEvent::SetTab(7)),
            |cx| Label::new(cx, "Misc"),
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
        })
        .row_between(Pixels(12.0));

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
            Label::new(cx, "Mod Slot 2")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_source)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_target)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.mod2_amount)
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
            create_label(cx, "Filter Style", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_style)
                ;
            create_label(cx, "Vintage Drive", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_vintage_drive)
                ;
            create_label(cx, "Vintage Curve", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_vintage_curve)
                ;
            create_label(cx, "Vintage Mix", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_vintage_mix)
                ;
            create_label(cx, "Vintage Trim", 20.0, 110.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_vintage_trim)
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

        VStack::new(cx, |cx| {
            Label::new(cx, "Filter Cut Atk")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_attack_ms)
                ;
            Label::new(cx, "Filter Cut Dec")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_decay_ms)
                ;
            Label::new(cx, "Filter Cut Sus")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_sustain_ms)
                ;
            Label::new(cx, "Filter Cut Rel")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_release_ms)
                ;
            Label::new(cx, "Cut Env Amt")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_envelope_level)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            create_label(cx, "Filter Q Atk", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_attack_ms)
                ;
            create_label(cx, "Filter Q Dec", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_decay_ms)
                ;
            create_label(cx, "Filter Q Sus", 20.0, 100.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_sustain_ms)
                ;
            Label::new(cx, "Filter Q Rel")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_release_ms)
                ;
            Label::new(cx, "Q Env Amt")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_envelope_level)
                ;
            Label::new(cx, "Cut Env Pol")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.filter_cut_env_polarity)
                .with_label("")
                .width(Pixels(90.0))
                .height(Pixels(28.0));
            Label::new(cx, "Res Env Pol")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamButton::new(cx, Data::params.clone(), |params| &params.filter_res_env_polarity)
                .with_label("")
                .width(Pixels(90.0))
                .height(Pixels(28.0));
            Label::new(cx, "Cut Tension")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_cut_tension)
                ;
            Label::new(cx, "Res Tension")
                .height(Pixels(18.0))
                .width(Pixels(90.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.filter_res_tension)
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
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_type)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_style)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_drive)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_curve)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_mix)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_trim)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_cut)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_res)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_a_amt)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Stage B")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_type)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_style)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_drive)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_curve)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_mix)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_trim)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_cut)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_res)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_b_amt)
                ;
        })
        .row_between(Pixels(12.0));

        VStack::new(cx, |cx| {
            Label::new(cx, "Stage C")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_type)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_style)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_drive)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_curve)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_mix)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_trim)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_cut)
                ;
            ParamSlider::new(cx, Data::params.clone(), |params| &params.multi_filter_c_res)
                ;
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

fn build_fx_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
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

fn build_resonator_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            Label::new(cx, "Resonator")
                .height(Pixels(16.0))
                .width(Pixels(90.0));
            ParamButton::new(cx, Data::params.clone(), |params| {
                &params.resonator_enable
            })
            .with_label("")
            .width(Pixels(90.0))
            .height(Pixels(28.0));
            create_label(cx, "Mix", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.resonator_mix)
                ;
            create_label(cx, "Tone", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.resonator_tone)
                ;
            create_label(cx, "Shape", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.resonator_shape)
                ;
            create_label(cx, "Timbre", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.resonator_timbre)
                ;
            create_label(cx, "Damping", 20.0, 90.0, 1.0, 0.0);
            ParamSlider::new(cx, Data::params.clone(), |params| &params.resonator_damping)
                ;
        })
        .row_between(Pixels(12.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_analog_tab(cx: &mut Context) {
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
            Label::new(cx, "Sub")
                .height(Pixels(16.0))
                .width(Pixels(70.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.sub_level)
                ;
        })
        .row_between(Pixels(12.0));
    })
    .col_between(Pixels(12.0))
    .row_between(Pixels(12.0))
    .child_top(Pixels(6.0));
}

fn build_misc_tab(cx: &mut Context) {
    HStack::new(cx, |cx| {
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
            Label::new(cx, "Program")
                .height(Pixels(18.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            ParamSlider::new(cx, Data::params.clone(), |params| &params.program)
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
        let preset_display = presets
            .get(0)
            .map(|preset| preset.name.clone())
            .unwrap_or_else(|| "Init".to_string());

        Data {
            params: params.clone(),
            active_tab: 0,
            presets,
            preset_index: 0,
            preset_display,
            preset_name: String::new(),
            custom_wavetable_display: params
                .custom_wavetable_path
                .read()
                .ok()
                .and_then(|path| path.clone())
                .and_then(|path| {
                    PathBuf::from(path)
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                })
                .unwrap_or_else(|| "No custom wav".to_string()),
            custom_wavetable_path_input: params
                .custom_wavetable_path
                .read()
                .ok()
                .and_then(|path| path.clone())
                .unwrap_or_default(),
        }
        .build(cx);

        ResizeHandle::new(cx);
        HStack::new(cx, |cx| {
            Label::new(cx, "SannySynth")
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
            2 => build_filter_tab(cx),
            3 => build_lfo_tab(cx),
            4 => build_fx_tab(cx),
            5 => build_resonator_tab(cx),
            6 => build_analog_tab(cx),
            _ => build_misc_tab(cx),
        });

        Element::new(cx)
            .height(Pixels(12.0))
            .width(Stretch(1.0));

        // レイアウトを初期化時にリセットする
        cx.emit(GuiContextEvent::Resize);
    })
}
