use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, RwLock};
use crate::sample::SampleBuffer;

pub const DRUM_SLOTS: usize = 32;
pub const DRUM_OUTPUT_PAIRS: usize = 17;

#[derive(Params)]
pub struct DrumSlotParams {
    #[id = "note"]
    pub midi_note: IntParam,
    #[id = "level"]
    pub level: FloatParam,
    #[id = "pan"]
    pub pan: FloatParam,
    #[id = "tune"]
    pub tune: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "vel_sense"]
    pub velocity_sensitivity: FloatParam,
    #[id = "tone_low"]
    pub tone_low: FloatParam,
    #[id = "tone_mid"]
    pub tone_mid: FloatParam,
    #[id = "tone_high"]
    pub tone_high: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "s_env_sus"]
    pub sample_env_sustain: FloatParam,
    #[id = "s_env_rel"]
    pub sample_env_release: FloatParam,
    #[id = "pad_trig"]
    pub pad_trigger: FloatParam,
    #[id = "out_bus"]
    pub output_bus: IntParam,
}

impl DrumSlotParams {
    pub fn default_for(index: usize) -> Self {
        let midi_note = (36 + index as i32).clamp(0, 127);
        let output_bus = (index % DRUM_OUTPUT_PAIRS) as i32 + 1;
        Self {
            midi_note: IntParam::new("Note", midi_note, IntRange::Linear { min: 0, max: 127 }),
            level: FloatParam::new("Gain", 0.9, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            pan: FloatParam::new("Pan", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            tune: FloatParam::new("Pitch", 0.0, FloatRange::Linear { min: -24.0, max: 24.0 })
                .with_step_size(0.1),
            drive: FloatParam::new("Drive", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            velocity_sensitivity: FloatParam::new(
                "Velocity",
                0.85,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            tone_low: FloatParam::new(
                "Tone Low",
                0.0,
                FloatRange::Linear { min: -60.0, max: 12.0 },
            )
            .with_step_size(0.1),
            tone_mid: FloatParam::new(
                "Tone Mid",
                0.0,
                FloatRange::Linear { min: -60.0, max: 12.0 },
            )
            .with_step_size(0.1),
            tone_high: FloatParam::new(
                "Tone High",
                0.0,
                FloatRange::Linear { min: -60.0, max: 12.0 },
            )
            .with_step_size(0.1),
            attack: FloatParam::new("Attack", 0.05, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            decay: FloatParam::new("Decay", 0.35, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            sample_env_sustain: FloatParam::new(
                "Sustain",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            sample_env_release: FloatParam::new(
                "Release",
                0.12,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            pad_trigger: FloatParam::new(
                "Pad Trigger",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.001),
            output_bus: IntParam::new(
                "Output",
                output_bus,
                IntRange::Linear {
                    min: 1,
                    max: DRUM_OUTPUT_PAIRS as i32,
                },
            ),
        }
    }
}

#[derive(Params)]
pub struct DrumSynthParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[persist = "drum_kit_label"]
    pub kit_label: Arc<RwLock<String>>,
    #[persist = "drum_kit_custom"]
    pub kit_custom: Arc<RwLock<bool>>,
    #[persist = "drum_sample_paths"]
    pub sample_paths: Arc<RwLock<[Option<String>; DRUM_SLOTS]>>,
    pub sample_data: Arc<RwLock<[Option<Arc<SampleBuffer>>; DRUM_SLOTS]>>,
    #[id = "master_gain"]
    pub master_gain: FloatParam,
    #[id = "master_drive"]
    pub master_drive: FloatParam,
    #[id = "master_comp"]
    pub master_comp: FloatParam,
    #[id = "master_clip"]
    pub master_clip: FloatParam,
    #[nested(array, group = "Drum Slots")]
    pub slots: [DrumSlotParams; DRUM_SLOTS],
}

impl Default for DrumSynthParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new(|| (1500, 900)),
            kit_label: Arc::new(RwLock::new(String::from("Example Kit"))),
            kit_custom: Arc::new(RwLock::new(false)),
            sample_paths: Arc::new(RwLock::new(std::array::from_fn(|_| None))),
            sample_data: Arc::new(RwLock::new(std::array::from_fn(|_| None))),
            master_gain: FloatParam::new(
                "Master Gain",
                0.9,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            master_drive: FloatParam::new(
                "Master Drive",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            master_comp: FloatParam::new(
                "Master Comp",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            master_clip: FloatParam::new(
                "Master Clip",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            slots: std::array::from_fn(|index| DrumSlotParams::default_for(index)),
        }
    }
}

pub struct DrumSynthBlockParams {
    pub master_gain: f32,
    pub master_drive: f32,
    pub master_comp: f32,
    pub master_clip: f32,
    pub slots: [(f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, i32); DRUM_SLOTS],
}

impl DrumSynthBlockParams {
    pub fn cache(params: &DrumSynthParams) -> Self {
        let mut slots = [(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0); DRUM_SLOTS];
        for (i, slot) in params.slots.iter().enumerate() {
            slots[i] = (
                slot.level.value(),
                slot.pan.value(),
                slot.tune.value(),
                slot.drive.value(),
                slot.velocity_sensitivity.value(),
                slot.tone_low.value(),
                slot.tone_mid.value(),
                slot.tone_high.value(),
                slot.attack.value(),
                slot.decay.value(),
                slot.sample_env_sustain.value(),
                slot.sample_env_release.value(),
                slot.pad_trigger.value(),
                slot.output_bus.value(),
            );
        }
        Self {
            master_gain: params.master_gain.value(),
            master_drive: params.master_drive.value(),
            master_comp: params.master_comp.value(),
            master_clip: params.master_clip.value(),
            slots,
        }
    }
}
