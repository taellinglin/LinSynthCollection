use nih_plug::prelude::{Editor, ParamMut};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

use std::sync::Arc;

use crate::presets::{apply_preset_to_params, preset_values_for, GM_CATEGORIES, GM_PRESETS};
use crate::sf2::{load_sf2_preset_names, resolve_sf2_path};
use crate::FishSynthParams;

// zCool font constant
const ZCOOL_XIAOWEI: &str = "ZCOOL XiaoWei";
const ZCOOL_FONT_DATA: &[u8] = include_bytes!("assets/ZCOOL_XIAOWEI_REGULAR.ttf");

#[derive(Lens)]
struct AppData {
    params: Arc<FishSynthParams>,
    preset_names: Arc<Vec<String>>,
    active_tab: UiTab,
    synth_tab: SynthTab,
    selected_category: usize,
    selected_preset: Option<usize>,
}

impl Model for AppData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match *app_event {
            AppEvent::SelectTab(tab) => {
                self.active_tab = tab;
            }
            AppEvent::SelectSynthTab(tab) => {
                self.synth_tab = tab;
            }
            AppEvent::SelectCategory(category) => {
                self.selected_category = category.min(GM_CATEGORIES.len() - 1);
            }
            AppEvent::SelectPreset(preset) => {
                let preset = preset.min(GM_PRESETS.len().saturating_sub(1));
                self.selected_preset = Some(preset);
                self.selected_category = preset / 8;
                let values = preset_values_for(preset);
                apply_preset_to_params(&self.params, values);
                self.params.program.set_plain_value(preset as i32);
            }
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Data)]
enum UiTab {
    Synth,
    Presets,
}

#[derive(Clone, Copy, PartialEq, Eq, Data)]
enum SynthTab {
    Osc,
    Amp,
    Filter,
    FilterEnv,
    Mod,
    Fx,
}

#[derive(Clone, Copy)]
enum AppEvent {
    SelectTab(UiTab),
    SelectSynthTab(SynthTab),
    SelectCategory(usize),
    SelectPreset(usize),
}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (960, 720))
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
        .height(Pixels(height))
        .width(Pixels(width))
        .child_top(Stretch(child_top))
        .child_bottom(Pixels(child_bottom));
}

pub(crate) fn create(
    params: Arc<FishSynthParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        let sf2_path = resolve_sf2_path();
        let preset_names = load_sf2_preset_names(&sf2_path)
            .map(Arc::new)
            .unwrap_or_else(|_| Arc::new(GM_PRESETS.iter().map(|name| name.to_string()).collect()));

        // Register zCool font
        cx.add_fonts_mem(&[ZCOOL_FONT_DATA]);
        
        // Set zCool as the default font for the entire UI
        cx.set_default_font(&[ZCOOL_XIAOWEI]);

        AppData {
            params: params.clone(),
            preset_names: preset_names.clone(),
            active_tab: UiTab::Synth,
            synth_tab: SynthTab::Osc,
            selected_category: 0,
            selected_preset: None,
        }
        .build(cx);

        ResizeHandle::new(cx);
        
        VStack::new(cx, |cx| {
            // Title
            Label::new(cx, "FishSynth - FM Synthesizer")
                .font_family(vec![FamilyOwned::Name(String::from(ZCOOL_XIAOWEI))])
                .font_size(28.0)
                .height(Pixels(40.0))
                .width(Stretch(1.0))
                .child_top(Pixels(5.0))
                .child_bottom(Pixels(5.0));
            HStack::new(cx, |cx| {
                Button::new(
                    cx,
                    |cx| cx.emit(AppEvent::SelectTab(UiTab::Synth)),
                    |cx| Label::new(cx, "Synth"),
                )
                .width(Pixels(120.0));
                Button::new(
                    cx,
                    |cx| cx.emit(AppEvent::SelectTab(UiTab::Presets)),
                    |cx| Label::new(cx, "Presets"),
                )
                .width(Pixels(120.0));
            })
            .child_top(Pixels(5.0))
            .child_bottom(Pixels(10.0));

            Binding::new(cx, AppData::active_tab, |cx, active_tab| {
                match active_tab.get(cx) {
                    UiTab::Synth => {
                        VStack::new(cx, |cx| {
                            HStack::new(cx, |cx| {
                                Button::new(
                                    cx,
                                    |cx| cx.emit(AppEvent::SelectSynthTab(SynthTab::Osc)),
                                    |cx| Label::new(cx, "Osc"),
                                )
                                .width(Pixels(110.0));
                                Button::new(
                                    cx,
                                    |cx| cx.emit(AppEvent::SelectSynthTab(SynthTab::Amp)),
                                    |cx| Label::new(cx, "Amp"),
                                )
                                .width(Pixels(110.0));
                                Button::new(
                                    cx,
                                    |cx| cx.emit(AppEvent::SelectSynthTab(SynthTab::Filter)),
                                    |cx| Label::new(cx, "Filter"),
                                )
                                .width(Pixels(110.0));
                                Button::new(
                                    cx,
                                    |cx| cx.emit(AppEvent::SelectSynthTab(SynthTab::FilterEnv)),
                                    |cx| Label::new(cx, "Filter Env"),
                                )
                                .width(Pixels(110.0));
                                Button::new(
                                    cx,
                                    |cx| cx.emit(AppEvent::SelectSynthTab(SynthTab::Mod)),
                                    |cx| Label::new(cx, "Mod"),
                                )
                                .width(Pixels(110.0));
                                Button::new(
                                    cx,
                                    |cx| cx.emit(AppEvent::SelectSynthTab(SynthTab::Fx)),
                                    |cx| Label::new(cx, "FX"),
                                )
                                .width(Pixels(110.0));
                            })
                            .child_bottom(Pixels(8.0));

                            Binding::new(cx, AppData::synth_tab, |cx, synth_tab| {
                                match synth_tab.get(cx) {
                                    SynthTab::Osc => {
                                        HStack::new(cx, |cx| {
                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "OSC")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Carrier", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.waveform);

                                                create_label(cx, "Sub Wave", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.sub_waveform);

                                                create_label(cx, "Sub Mix", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.sub_mix);

                                                create_label(cx, "Noise Mix", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.noise_mix);
                                            })
                                            .width(Pixels(240.0))
                                            .child_right(Pixels(10.0));

                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "FM")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Enable", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.fm_enable);

                                                create_label(cx, "Ratio", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.fm_ratio);

                                                create_label(cx, "Amount", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.fm_amount);

                                                create_label(cx, "Mod Wave", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.modulator_waveform);
                                            })
                                            .width(Pixels(240.0));
                                        })
                                        .child_left(Pixels(10.0));
                                    }
                                    SynthTab::Amp => {
                                        HStack::new(cx, |cx| {
                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "AMPLITUDE")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Gain", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.gain);

                                                create_label(cx, "Attack", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.amp_attack_ms);

                                                create_label(cx, "Decay", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.amp_decay_ms);

                                                create_label(cx, "Sustain", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.amp_sustain_level);

                                                create_label(cx, "Release", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.amp_release_ms);
                                            })
                                            .width(Pixels(240.0))
                                            .child_right(Pixels(10.0));

                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "AMP SHAPE")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Env Level", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.amp_envelope_level);

                                                create_label(cx, "Tension", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.tension);
                                            })
                                            .width(Pixels(240.0));
                                        })
                                        .child_left(Pixels(10.0));
                                    }
                                    SynthTab::Filter => {
                                        HStack::new(cx, |cx| {
                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "FILTER")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Type", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_type);

                                                create_label(cx, "Cutoff", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_cut);

                                                create_label(cx, "Resonance", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_res);

                                                create_label(cx, "Amount", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_amount);
                                            })
                                            .width(Pixels(240.0));
                                        })
                                        .child_left(Pixels(10.0));
                                    }
                                    SynthTab::FilterEnv => {
                                        HStack::new(cx, |cx| {
                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "CUT ENV")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Attack", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_cut_attack_ms);

                                                create_label(cx, "Decay", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_cut_decay_ms);

                                                create_label(cx, "Sustain", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_cut_sustain_ms);

                                                create_label(cx, "Release", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_cut_release_ms);
                                            })
                                            .width(Pixels(240.0))
                                            .child_right(Pixels(10.0));

                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "RES ENV")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Attack", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_res_attack_ms);

                                                create_label(cx, "Decay", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_res_decay_ms);

                                                create_label(cx, "Sustain", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_res_sustain_ms);

                                                create_label(cx, "Release", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_res_release_ms);

                                                create_label(cx, "Cut Env Amt", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_cut_envelope_level);

                                                create_label(cx, "Res Env Amt", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.filter_res_envelope_level);
                                            })
                                            .width(Pixels(240.0));
                                        })
                                        .child_left(Pixels(10.0));
                                    }
                                    SynthTab::Mod => {
                                        HStack::new(cx, |cx| {
                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "VIBRATO")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Rate", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.vibrato_rate);

                                                create_label(cx, "Intensity", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.vibrato_intensity);

                                                create_label(cx, "Attack", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.vibrato_attack);

                                                create_label(cx, "Shape", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.vibrato_shape);
                                            })
                                            .width(Pixels(240.0))
                                            .child_right(Pixels(10.0));

                                            VStack::new(cx, |cx| {
                                                Label::new(cx, "TREMOLO")
                                                    .font_size(14.0)
                                                    .font_weight(Weight::BOLD)
                                                    .height(Pixels(22.0));

                                                create_label(cx, "Rate", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.tremolo_rate);

                                                create_label(cx, "Intensity", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.tremolo_intensity);

                                                create_label(cx, "Attack", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.tremolo_attack);

                                                create_label(cx, "Shape", 16.0, 120.0, 0.5, 2.0);
                                                ParamSlider::new(cx, AppData::params.clone(), |params| &params.tremolo_shape);
                                            })
                                            .width(Pixels(240.0));
                                        })
                                        .child_left(Pixels(10.0));
                                    }
                                    SynthTab::Fx => {
                                        VStack::new(cx, |cx| {
                                            Label::new(cx, "CHORUS")
                                                .font_size(14.0)
                                                .font_weight(Weight::BOLD)
                                                .height(Pixels(22.0));

                                            create_label(cx, "Enable", 16.0, 120.0, 0.5, 2.0);
                                            ParamSlider::new(cx, AppData::params.clone(), |params| &params.chorus_enable);

                                            create_label(cx, "Rate", 16.0, 120.0, 0.5, 2.0);
                                            ParamSlider::new(cx, AppData::params.clone(), |params| &params.chorus_rate);

                                            create_label(cx, "Depth", 16.0, 120.0, 0.5, 2.0);
                                            ParamSlider::new(cx, AppData::params.clone(), |params| &params.chorus_depth);

                                            create_label(cx, "Mix", 16.0, 120.0, 0.5, 2.0);
                                            ParamSlider::new(cx, AppData::params.clone(), |params| &params.chorus_mix);
                                        })
                                        .width(Pixels(240.0))
                                        .child_left(Pixels(10.0));
                                    }
                                }
                            });
                        })
                        .child_top(Pixels(5.0))
                        .child_bottom(Pixels(10.0));
                    }
                    UiTab::Presets => {
                        HStack::new(cx, |cx| {
                            VStack::new(cx, |cx| {
                                Label::new(cx, "Categories")
                                    .font_size(14.0)
                                    .font_weight(Weight::BOLD)
                                    .height(Pixels(25.0));

                                for (index, category) in GM_CATEGORIES.iter().enumerate() {
                                    Button::new(
                                        cx,
                                        move |cx| cx.emit(AppEvent::SelectCategory(index)),
                                        move |cx| Label::new(cx, category.name),
                                    )
                                    .width(Pixels(220.0));
                                }
                            })
                            .width(Pixels(240.0))
                            .child_left(Pixels(10.0))
                            .child_right(Pixels(10.0));

                            VStack::new(cx, |cx| {
                                Label::new(cx, "Presets")
                                    .font_size(14.0)
                                    .font_weight(Weight::BOLD)
                                    .height(Pixels(25.0));

                                Binding::new(cx, AppData::selected_category, move |cx, selected_category| {
                                    let preset_names = cx
                                        .data::<AppData>()
                                        .map(|data| data.preset_names.clone())
                                        .unwrap_or_else(|| Arc::new(Vec::new()));
                                    let category = selected_category.get(cx).min(GM_CATEGORIES.len() - 1);
                                    let start = GM_CATEGORIES[category].start;

                                    for offset in 0..8 {
                                        let preset_index = start + offset;
                                        let name = preset_names
                                            .get(preset_index)
                                            .map(|name| name.as_str())
                                            .unwrap_or(GM_PRESETS[preset_index]);
                                        let label = format!("{:03} {}", preset_index + 1, name);
                                        Button::new(
                                            cx,
                                            move |cx| cx.emit(AppEvent::SelectPreset(preset_index)),
                                            move |cx| Label::new(cx, &label),
                                        )
                                        .width(Stretch(1.0));
                                    }
                                });
                            })
                            .width(Stretch(1.0))
                            .child_right(Pixels(10.0));
                        })
                        .height(Stretch(1.0))
                        .child_top(Pixels(5.0));
                    }
                }
            });
        })
        .row_between(Pixels(5.0));


    })
}
                
