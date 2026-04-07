use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

use crate::drum_model::{
    default_instrument_for_slot, default_note_for_slot, ExciterType, InstrumentType, MaterialType,
    NoiseType, ResonatorType,
};

pub const DRUM_SLOTS: usize = 32;
pub const DRUM_STEPS: usize = 16;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum)]
pub enum DrumOrganicKitPreset {
    NaturalStudio,
    DryStudio,
    WarmTape,
    VintageSixties,
    VintageSeventies,
    VintageEighties,
    TightFunk,
    SoulPocket,
    MotownLite,
    IndieRoom,
    LoFiDust,
    BrushKit,
    JazzClub,
    FolkWood,
    PercussionWood,
    HybridOrganic,
    PunchyRock,
    BigRoom,
    ArenaLive,
    DarkCinematic,
    BrightPop,
    AmbientAir,
    BrokenToy,
    ExperimentalFoley,
}

#[derive(Params)]
pub struct DrumStepParams {
    #[id = "gate"]
    pub gate: BoolParam,
    #[id = "vel"]
    pub velocity: FloatParam,
}

impl Default for DrumStepParams {
    fn default() -> Self {
        Self {
            gate: BoolParam::new("Gate", false),
            velocity: FloatParam::new(
                "Velocity",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
        }
    }
}

#[derive(Params)]
pub struct DrumLaneParams {
    #[nested(array)]
    pub steps: [DrumStepParams; DRUM_STEPS],
}

impl Default for DrumLaneParams {
    fn default() -> Self {
        Self {
            steps: std::array::from_fn(|_| DrumStepParams::default()),
        }
    }
}

#[derive(Params)]
pub struct DrumSlotParams {
    #[id = "inst"]
    pub instrument: EnumParam<InstrumentType>,
    #[id = "note"]
    pub midi_note: IntParam,
    #[id = "level"]
    pub level: FloatParam,
    #[id = "pan"]
    pub pan: FloatParam,
    #[id = "tune"]
    pub tune: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "damp"]
    pub damping: FloatParam,
    #[id = "spread"]
    pub spread: FloatParam,
    #[id = "exciter"]
    pub exciter: EnumParam<ExciterType>,
    #[id = "exc_mix"]
    pub exciter_mix: FloatParam,
    #[id = "resonator"]
    pub resonator: EnumParam<ResonatorType>,
    #[id = "material"]
    pub material: EnumParam<MaterialType>,
    #[id = "strike_pos"]
    pub strike_position: FloatParam,
    #[id = "strike_hard"]
    pub strike_hardness: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "pitch_env"]
    pub pitch_env: FloatParam,
    #[id = "pitch_decay"]
    pub pitch_decay: FloatParam,
    #[id = "noise"]
    pub noise: FloatParam,
    #[id = "diff"]
    pub diffusion: FloatParam,
    #[id = "noise_color"]
    pub noise_color: FloatParam,
    #[id = "noise_type"]
    pub noise_type: EnumParam<NoiseType>,
    #[id = "noise_density"]
    pub noise_density: FloatParam,
    #[id = "res_mix"]
    pub resonator_mix: FloatParam,
    #[id = "mode_tilt"]
    pub mode_tilt: FloatParam,
    #[id = "cymbal_shape"]
    pub cymbal_shape: FloatParam,
    #[id = "snare_wire"]
    pub snare_wire: FloatParam,
    #[id = "snare_wire_decay"]
    pub snare_wire_decay: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "transient"]
    pub transient: FloatParam,
    #[id = "body"]
    pub body: FloatParam,
    #[id = "vel_sense"]
    pub velocity_sensitivity: FloatParam,
    #[id = "tone_low"]
    pub tone_low: FloatParam,
    #[id = "tone_mid"]
    pub tone_mid: FloatParam,
    #[id = "tone_high"]
    pub tone_high: FloatParam,
    #[id = "pan_lfo_rate"]
    pub pan_lfo_rate: FloatParam,
    #[id = "pan_lfo_depth"]
    pub pan_lfo_depth: FloatParam,
    #[id = "pad_trig"]
    pub pad_trigger: FloatParam,
}

impl DrumSlotParams {
    pub fn default_for(index: usize) -> Self {
        let instrument = default_instrument_for_slot(index);
        let midi_note = default_note_for_slot(index) as i32;
        let mut exciter = ExciterType::Mallet;
        let mut exciter_mix = 0.4;
        let mut resonator = ResonatorType::Membrane;
        let mut material = MaterialType::Skin;
        let mut level = 0.9;
        let mut pan = 0.5;
        let mut tune = 0.0;
        let mut decay = 0.6;
        let mut damping = 0.5;
        let mut spread = 0.0;
        let mut strike_position = 0.5;
        let mut strike_hardness = 0.6;
        let mut attack = 0.4;
        let mut pitch_env = 0.35;
        let mut pitch_decay = 0.4;
        let mut noise = 0.2;
        let mut diffusion = 0.5;
        let mut noise_color = 0.5;
        let mut noise_type = match instrument {
            InstrumentType::HatClosed
            | InstrumentType::HatOpen
            | InstrumentType::HatPedal
            | InstrumentType::Ride
            | InstrumentType::Crash => NoiseType::Metallic,
            InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot => {
                NoiseType::Pink
            }
            _ => NoiseType::White,
        };
        let mut noise_density = 0.5;
        let mut resonator_mix = 0.7;
        let mut mode_tilt = 0.2;
        let mut cymbal_shape = 0.5;
        let mut snare_wire = 0.6;
        let mut snare_wire_decay = 0.5;
        let mut drive = 0.0;
        let (mut transient, mut body, mut velocity_sensitivity, mut tone_low, mut tone_mid, mut tone_high) =
            match instrument {
                InstrumentType::Kick => (0.6, 0.95, 0.7, 4.0, -3.0, -4.0),
                InstrumentType::Snare => (0.6, 0.3, 0.8, -1.0, 1.6, 2.2),
                InstrumentType::HatClosed => (0.6, 0.08, 0.9, -4.0, 0.0, 2.2),
                InstrumentType::HatOpen => (0.6, 0.12, 0.9, -4.0, 0.0, 2.6),
                InstrumentType::HatPedal => (0.55, 0.06, 0.9, -4.0, 0.0, 2.0),
                InstrumentType::Ride => (0.55, 0.2, 0.9, -4.0, -0.5, 2.4),
                InstrumentType::Crash => (0.6, 0.2, 0.9, -4.0, 0.0, 2.8),
                InstrumentType::Rimshot => (0.75, 0.2, 0.85, -2.0, 1.8, 3.0),
                InstrumentType::Clap => (0.7, 0.2, 0.85, -2.0, 1.4, 2.2),
                InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh => {
                    (0.55, 0.8, 0.8, 2.0, -1.0, -2.0)
                }
                InstrumentType::Perc1 | InstrumentType::Perc2 => (0.55, 0.1, 0.9, -2.0, 0.5, 1.8),
                InstrumentType::Fx1 | InstrumentType::Fx2 => (0.6, 0.2, 0.8, 0.0, 1.0, 2.0),
            };
        let mut pan_lfo_rate = 1.2;
        let mut pan_lfo_depth = 0.0;

        match instrument {
            InstrumentType::Kick => {
                exciter = ExciterType::Impulse;
                resonator = ResonatorType::Membrane;
                material = MaterialType::Skin;
                level = 0.95;
                pan = 0.5;
                tune = 0.0;
                decay = 0.7;
                damping = 0.35;
                spread = 0.0;
                strike_position = 0.45;
                strike_hardness = 0.7;
                attack = 0.18;
                pitch_env = 0.6;
                pitch_decay = 0.24;
                exciter_mix = 0.2;
                noise = 0.1;
                diffusion = 0.3;
                noise_color = 0.4;
                noise_type = NoiseType::Pink;
                noise_density = 0.4;
                resonator_mix = 0.85;
                mode_tilt = 0.15;
                cymbal_shape = 0.0;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.2;
            }
            InstrumentType::Snare => {
                exciter = ExciterType::Stick;
                resonator = ResonatorType::Plate;
                material = MaterialType::Skin;
                level = 0.9;
                pan = 0.48;
                tune = 0.0;
                decay = 0.5;
                damping = 0.4;
                spread = 0.0;
                strike_position = 0.4;
                strike_hardness = 0.7;
                attack = 0.2;
                pitch_env = 0.2;
                pitch_decay = 0.2;
                exciter_mix = 0.55;
                noise = 0.8;
                diffusion = 0.8;
                noise_color = 0.8;
                noise_type = NoiseType::Pink;
                noise_density = 0.7;
                resonator_mix = 0.65;
                mode_tilt = 0.25;
                cymbal_shape = 0.0;
                snare_wire = 0.9;
                snare_wire_decay = 0.6;
                drive = 0.1;
            }
            InstrumentType::HatClosed => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Metallic;
                material = MaterialType::Metal;
                level = 0.7;
                pan = 0.45;
                tune = 0.0;
                decay = 0.12;
                damping = 0.6;
                spread = 0.1;
                strike_position = 0.5;
                strike_hardness = 0.7;
                attack = 0.08;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.9;
                noise = 0.85;
                diffusion = 0.7;
                noise_color = 0.7;
                noise_type = NoiseType::Metallic;
                noise_density = 0.7;
                resonator_mix = 0.25;
                mode_tilt = 0.3;
                cymbal_shape = 0.45;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.03;
            }
            InstrumentType::HatOpen => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Metallic;
                material = MaterialType::Metal;
                level = 0.78;
                pan = 0.55;
                tune = 0.0;
                decay = 0.35;
                damping = 0.55;
                spread = 0.12;
                strike_position = 0.58;
                strike_hardness = 0.7;
                attack = 0.12;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.85;
                noise = 0.88;
                diffusion = 0.75;
                noise_color = 0.72;
                noise_type = NoiseType::Metallic;
                noise_density = 0.75;
                resonator_mix = 0.3;
                mode_tilt = 0.32;
                cymbal_shape = 0.5;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.03;
            }
            InstrumentType::HatPedal => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Metallic;
                material = MaterialType::Metal;
                level = 0.65;
                pan = 0.5;
                tune = 0.0;
                decay = 0.08;
                damping = 0.65;
                spread = 0.08;
                strike_position = 0.5;
                strike_hardness = 0.7;
                attack = 0.06;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.9;
                noise = 0.8;
                diffusion = 0.68;
                noise_color = 0.68;
                noise_type = NoiseType::Metallic;
                noise_density = 0.7;
                resonator_mix = 0.2;
                mode_tilt = 0.28;
                cymbal_shape = 0.4;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.03;
            }
            InstrumentType::Ride => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Metallic;
                material = MaterialType::Metal;
                level = 0.8;
                pan = 0.6;
                tune = 0.0;
                decay = 0.7;
                damping = 0.45;
                spread = 0.2;
                strike_position = 0.65;
                strike_hardness = 0.7;
                attack = 0.18;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.7;
                noise = 0.7;
                diffusion = 0.8;
                noise_color = 0.9;
                noise_type = NoiseType::Metallic;
                noise_density = 0.85;
                resonator_mix = 0.5;
                mode_tilt = 0.35;
                cymbal_shape = 0.8;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.05;
            }
            InstrumentType::Crash => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Metallic;
                material = MaterialType::Metal;
                level = 0.85;
                pan = 0.4;
                tune = 0.0;
                decay = 0.8;
                damping = 0.45;
                spread = 0.25;
                strike_position = 0.7;
                strike_hardness = 0.7;
                attack = 0.2;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.75;
                noise = 0.75;
                diffusion = 0.85;
                noise_color = 0.92;
                noise_type = NoiseType::Metallic;
                noise_density = 0.88;
                resonator_mix = 0.55;
                mode_tilt = 0.35;
                cymbal_shape = 0.82;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.05;
            }
            InstrumentType::Rimshot => {
                exciter = ExciterType::Stick;
                resonator = ResonatorType::Plate;
                material = MaterialType::Wood;
                level = 0.7;
                pan = 0.4;
                tune = 0.0;
                decay = 0.18;
                damping = 0.5;
                spread = 0.0;
                strike_position = 0.35;
                strike_hardness = 0.75;
                attack = 0.15;
                pitch_env = 0.1;
                pitch_decay = 0.12;
                exciter_mix = 0.5;
                noise = 0.4;
                diffusion = 0.6;
                noise_color = 0.75;
                noise_type = NoiseType::Pink;
                noise_density = 0.5;
                resonator_mix = 0.6;
                mode_tilt = 0.3;
                cymbal_shape = 0.0;
                snare_wire = 0.4;
                snare_wire_decay = 0.4;
                drive = 0.1;
            }
            InstrumentType::Clap => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Plate;
                material = MaterialType::Skin;
                level = 0.8;
                pan = 0.5;
                tune = 0.0;
                decay = 0.28;
                damping = 0.45;
                spread = 0.05;
                strike_position = 0.5;
                strike_hardness = 0.6;
                attack = 0.2;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.8;
                noise = 0.8;
                diffusion = 0.8;
                noise_color = 0.8;
                noise_type = NoiseType::Pink;
                noise_density = 0.65;
                resonator_mix = 0.5;
                mode_tilt = 0.3;
                cymbal_shape = 0.0;
                snare_wire = 0.5;
                snare_wire_decay = 0.5;
                drive = 0.05;
            }
            InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh => {
                exciter = ExciterType::Mallet;
                resonator = ResonatorType::Membrane;
                material = MaterialType::Skin;
                level = 0.85;
                pan = if matches!(instrument, InstrumentType::TomLow) {
                    0.4
                } else if matches!(instrument, InstrumentType::TomHigh) {
                    0.6
                } else {
                    0.5
                };
                tune = 0.0;
                decay = 0.55;
                damping = 0.45;
                spread = 0.0;
                strike_position = 0.5;
                strike_hardness = 0.65;
                attack = 0.2;
                pitch_env = 0.3;
                pitch_decay = 0.25;
                exciter_mix = 0.35;
                noise = 0.2;
                diffusion = 0.4;
                noise_color = 0.5;
                noise_type = NoiseType::Pink;
                noise_density = 0.4;
                resonator_mix = 0.75;
                mode_tilt = 0.2;
                cymbal_shape = 0.0;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.1;
            }
            InstrumentType::Perc1 | InstrumentType::Perc2 => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Metallic;
                material = MaterialType::Metal;
                level = 0.7;
                pan = if matches!(instrument, InstrumentType::Perc1) {
                    0.35
                } else {
                    0.65
                };
                tune = 0.0;
                decay = 0.22;
                damping = 0.5;
                spread = 0.1;
                strike_position = 0.6;
                strike_hardness = 0.7;
                attack = 0.12;
                pitch_env = 0.0;
                pitch_decay = 0.0;
                exciter_mix = 0.7;
                noise = 0.75;
                diffusion = 0.75;
                noise_color = 0.88;
                noise_type = NoiseType::White;
                noise_density = 0.8;
                resonator_mix = 0.55;
                mode_tilt = 0.35;
                cymbal_shape = 0.6;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.05;
            }
            InstrumentType::Fx1 | InstrumentType::Fx2 => {
                exciter = ExciterType::Noise;
                resonator = ResonatorType::Plate;
                material = MaterialType::Composite;
                level = 0.7;
                pan = if matches!(instrument, InstrumentType::Fx1) { 0.3 } else { 0.7 };
                tune = if matches!(instrument, InstrumentType::Fx1) {
                    -6.0
                } else {
                    6.0
                };
                decay = 0.5;
                damping = 0.4;
                spread = 0.2;
                strike_position = 0.5;
                strike_hardness = 0.6;
                attack = 0.2;
                pitch_env = 0.2;
                pitch_decay = 0.4;
                exciter_mix = 0.6;
                noise = 0.6;
                diffusion = 0.7;
                noise_color = 0.8;
                noise_type = NoiseType::Pink;
                noise_density = 0.7;
                resonator_mix = 0.5;
                mode_tilt = 0.4;
                cymbal_shape = 0.5;
                snare_wire = 0.0;
                snare_wire_decay = 0.0;
                drive = 0.2;
                pan_lfo_rate = 2.0;
                pan_lfo_depth = 0.2;
            }
        }
        Self {
            instrument: EnumParam::new("κ", instrument),
            midi_note: IntParam::new("ν", midi_note, IntRange::Linear { min: 0, max: 127 }),
            level: FloatParam::new("●", level, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            pan: FloatParam::new("↔", pan, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            tune: FloatParam::new("♭/♯", tune, FloatRange::Linear { min: -24.0, max: 24.0 })
                .with_step_size(0.1),
            decay: FloatParam::new("τ", decay, FloatRange::Linear { min: 0.01, max: 1.0 })
                .with_step_size(0.01),
            damping: FloatParam::new("ζ", damping, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            spread: FloatParam::new("⇔", spread, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            exciter: EnumParam::new("ε", exciter),
            exciter_mix: FloatParam::new(
                "εm",
                exciter_mix,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            resonator: EnumParam::new("ρ", resonator),
            material: EnumParam::new("μ", material),
            strike_position: FloatParam::new(
                "x",
                strike_position,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            strike_hardness: FloatParam::new(
                "|x|",
                strike_hardness,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            attack: FloatParam::new("∧", attack, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            pitch_env: FloatParam::new(
                "Δ",
                pitch_env,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            pitch_decay: FloatParam::new(
                "τp",
                pitch_decay,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            noise: FloatParam::new("∿", noise, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            diffusion: FloatParam::new(
                "⋯",
                diffusion,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            noise_color: FloatParam::new(
                "λ",
                noise_color,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            noise_type: EnumParam::new("ηt", noise_type),
            noise_density: FloatParam::new(
                "η",
                noise_density,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            resonator_mix: FloatParam::new(
                "ρm",
                resonator_mix,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            mode_tilt: FloatParam::new(
                "τm",
                mode_tilt,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            )
            .with_step_size(0.01),
            cymbal_shape: FloatParam::new(
                "φ",
                cymbal_shape,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            snare_wire: FloatParam::new(
                "≋",
                snare_wire,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            snare_wire_decay: FloatParam::new(
                "τw",
                snare_wire_decay,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            drive: FloatParam::new("×", drive, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            transient: FloatParam::new(
                "δ",
                transient,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            body: FloatParam::new("β", body, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01),
            velocity_sensitivity: FloatParam::new(
                "∂",
                velocity_sensitivity,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            tone_low: FloatParam::new(
                "±₁",
                tone_low,
                FloatRange::Linear { min: -60.0, max: 12.0 },
            )
            .with_step_size(0.1),
            tone_mid: FloatParam::new(
                "±₂",
                tone_mid,
                FloatRange::Linear { min: -60.0, max: 12.0 },
            )
            .with_step_size(0.1),
            tone_high: FloatParam::new(
                "±₃",
                tone_high,
                FloatRange::Linear { min: -60.0, max: 12.0 },
            )
            .with_step_size(0.1),
            pan_lfo_rate: FloatParam::new(
                "ω",
                pan_lfo_rate,
                FloatRange::Linear { min: 0.1, max: 12.0 },
            )
            .with_step_size(0.01),
            pan_lfo_depth: FloatParam::new(
                "α",
                pan_lfo_depth,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            pad_trigger: FloatParam::new(
                "∎",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.001),
        }
    }
}

#[derive(Params)]
pub struct DrumSequencerParams {
    #[id = "seq_en"]
    pub enabled: BoolParam,
    #[id = "seq_rate"]
    pub rate: FloatParam,
    #[id = "seq_swing"]
    pub swing: FloatParam,
    #[nested(array, group = "Sequencer")]
    pub lanes: Vec<DrumLaneParams>,
}

impl Default for DrumSequencerParams {
    fn default() -> Self {
        let mut lanes = Vec::with_capacity(DRUM_SLOTS);
        for _ in 0..DRUM_SLOTS {
            lanes.push(DrumLaneParams::default());
        }
        Self {
            enabled: BoolParam::new("Sequencer Enable", false),
            rate: FloatParam::new("Rate", 4.0, FloatRange::Linear { min: 0.25, max: 16.0 })
                .with_step_size(0.01),
            swing: FloatParam::new("Swing", 0.0, FloatRange::Linear { min: 0.0, max: 0.75 })
                .with_step_size(0.01),
            lanes,
        }
    }
}

#[derive(Params)]
pub struct DrumSynthParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,
    #[id = "kit_prog"]
    pub kit_preset: EnumParam<DrumOrganicKitPreset>,
    #[id = "master_gain"]
    pub master_gain: FloatParam,
    #[id = "master_drive"]
    pub master_drive: FloatParam,
    #[id = "master_comp"]
    pub master_comp: FloatParam,
    #[id = "master_clip"]
    pub master_clip: FloatParam,
    #[nested(array, group = "Drum Slots")]
    pub slots: Vec<DrumSlotParams>,
    #[nested(group = "Sequencer")]
    pub sequencer: DrumSequencerParams,
}

impl Default for DrumSynthParams {
    fn default() -> Self {
        let mut slots = Vec::with_capacity(DRUM_SLOTS);
        for index in 0..DRUM_SLOTS {
            slots.push(DrumSlotParams::default_for(index));
        }
        Self {
            editor_state: ViziaState::new(|| (1180, 860)),
            kit_preset: EnumParam::new("Kit Program", DrumOrganicKitPreset::NaturalStudio),
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
            slots,
            sequencer: DrumSequencerParams::default(),
        }
    }
}
