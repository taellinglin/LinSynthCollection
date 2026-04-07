use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

use std::cell::RefCell;
use std::sync::Arc;

use crate::presets::PRESET_NAMES;
use crate::{DelayBankParams, ScopeBuffer};

#[derive(Lens)]
struct AppData {
    params: Arc<DelayBankParams>,
}

impl Model for AppData {}

struct ScopeView {
    scope: Arc<ScopeBuffer>,
    samples: RefCell<Vec<f32>>,
}

impl ScopeView {
    fn new(cx: &mut Context, scope: Arc<ScopeBuffer>) -> Handle<Self> {
        Self {
            scope,
            samples: RefCell::new(Vec::new()),
        }
        .build(cx, |_| {})
    }
}

impl View for ScopeView {
    fn element(&self) -> Option<&'static str> {
        Some("scope")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 2.0 || bounds.h <= 2.0 {
            return;
        }

        let mut samples = self.samples.borrow_mut();
        self.scope.snapshot(&mut samples);
        if samples.len() < 2 {
            return;
        }

        let mut path = vg::Path::new();
        let center_y = bounds.y + bounds.h * 0.5;
        let amp = bounds.h * 0.42;
        let step = bounds.w / (samples.len() - 1) as f32;

        for (i, sample) in samples.iter().enumerate() {
            let x = bounds.x + i as f32 * step;
            let y = center_y - sample.clamp(-1.0, 1.0) * amp;
            if i == 0 {
                path.move_to(x, y);
            } else {
                path.line_to(x, y);
            }
        }

        let mut paint = vg::Paint::color(vg::Color::rgb(255, 90, 90));
        paint.set_line_width(2.0);
        canvas.stroke_path(&mut path, &paint);
    }
}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (980, 620))
}

fn section_label(cx: &mut Context, text: &str) {
    Label::new(cx, text)
        .class("small")
        .height(Pixels(18.0))
        .width(Stretch(1.0));
}

pub(crate) fn create(
    params: Arc<DelayBankParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_theme(include_str!("../assets/theme.css"));

        AppData { params: params.clone() }.build(cx);
        ResizeHandle::new(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "DelayBank")
                .class("title")
                .height(Pixels(32.0));

            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    section_label(cx, "Preset");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.preset)
                        .class("knob");
                    Label::new(cx, PRESET_NAMES[0]).class("small");
                })
                .class("section")
                .width(Stretch(1.0));

                VStack::new(cx, |cx| {
                    section_label(cx, "Global Mix");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.mix)
                        .class("knob");
                    section_label(cx, "Input");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.input_trim)
                        .class("knob");
                    section_label(cx, "Output");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.output_trim)
                        .class("knob");
                })
                .class("section")
                .width(Stretch(1.0));

                VStack::new(cx, |cx| {
                    section_label(cx, "Filter HP");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.hp_cut)
                        .class("knob");
                    section_label(cx, "Filter LP");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.lp_cut)
                        .class("knob");
                })
                .class("section")
                .width(Stretch(1.0));

                VStack::new(cx, |cx| {
                    section_label(cx, "Crush Depth");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.crush_depth)
                        .class("knob");
                    section_label(cx, "Crush Rate");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.crush_rate)
                        .class("knob");
                    section_label(cx, "Crush Mix");
                    ParamSlider::new(cx, AppData::params.clone(), |p| &p.crush_mix)
                        .class("knob");
                })
                .class("section")
                .width(Stretch(1.0));

                VStack::new(cx, |cx| {
                    section_label(cx, "Scope");
                    ScopeView::new(cx, params.scope.clone())
                        .class("scope")
                        .height(Pixels(120.0))
                        .width(Pixels(240.0));
                })
                .class("section")
                .width(Stretch(1.0));
            })
            .class("container")
            .height(Pixels(160.0))
            .child_bottom(Pixels(10.0));

            for row in 0..2 {
                HStack::new(cx, |cx| {
                    for col in 0..3 {
                        let idx = row * 3 + col;
                        VStack::new(cx, |cx| {
                            let label = format!("Bank {}", idx + 1);
                            Label::new(cx, label.as_str())
                                .class("small")
                                .height(Pixels(18.0));

                            match idx {
                                0 => bank_controls(cx, AppData::params.clone(), 1),
                                1 => bank_controls(cx, AppData::params.clone(), 2),
                                2 => bank_controls(cx, AppData::params.clone(), 3),
                                3 => bank_controls(cx, AppData::params.clone(), 4),
                                4 => bank_controls(cx, AppData::params.clone(), 5),
                                _ => bank_controls(cx, AppData::params.clone(), 6),
                            }
                        })
                        .class("section")
                        .width(Stretch(1.0))
                        .child_right(Pixels(6.0));
                    }
                })
                .class("container")
                .height(Pixels(180.0))
                .child_bottom(Pixels(8.0));
            }
        })
        .class("container")
        .child_left(Pixels(8.0))
        .child_right(Pixels(8.0))
        .child_top(Pixels(8.0))
        .child_bottom(Pixels(8.0));
    })
}

fn bank_controls<L>(cx: &mut Context, params: L, bank: usize)
where
    L: Lens<Target = Arc<DelayBankParams>> + Clone,
{
    match bank {
        1 => {
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_enable)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_sync)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_time_note)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_time_ms)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_feedback)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_level)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank1_pan)
                .class("knob-small");
        }
        2 => {
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_enable)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_sync)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_time_note)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_time_ms)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_feedback)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_level)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank2_pan)
                .class("knob-small");
        }
        3 => {
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_enable)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_sync)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_time_note)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_time_ms)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_feedback)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_level)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank3_pan)
                .class("knob-small");
        }
        4 => {
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_enable)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_sync)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_time_note)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_time_ms)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_feedback)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_level)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank4_pan)
                .class("knob-small");
        }
        5 => {
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_enable)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_sync)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_time_note)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_time_ms)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_feedback)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_level)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank5_pan)
                .class("knob-small");
        }
        _ => {
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_enable)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_sync)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_time_note)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_time_ms)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_feedback)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_level)
                .class("knob-small");
            ParamSlider::new(cx, params.clone(), |p| &p.bank6_pan)
                .class("knob-small");
        }
    }
}
