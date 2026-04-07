use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum InstrumentType {
    Kick,
    Snare,
    HatClosed,
    HatOpen,
    HatPedal,
    Ride,
    Crash,
    Rimshot,
    Clap,
    TomLow,
    TomMid,
    TomHigh,
    Perc1,
    Perc2,
    Fx1,
    Fx2,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum ExciterType {
    Mallet,
    Stick,
    Brush,
    Noise,
    Impulse,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum ResonatorType {
    Membrane,
    Plate,
    Metallic,
    Tube,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum MaterialType {
    Skin,
    Plastic,
    Metal,
    Wood,
    Composite,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum NoiseType {
    White,
    Pink,
    Metallic,
}

pub fn default_instrument_for_slot(index: usize) -> InstrumentType {
    let base_index = index % 16;
    match base_index {
        0 => InstrumentType::Kick,
        1 => InstrumentType::Snare,
        2 => InstrumentType::HatClosed,
        3 => InstrumentType::HatOpen,
        4 => InstrumentType::HatPedal,
        5 => InstrumentType::Ride,
        6 => InstrumentType::Crash,
        7 => InstrumentType::Rimshot,
        8 => InstrumentType::Clap,
        9 => InstrumentType::TomLow,
        10 => InstrumentType::TomMid,
        11 => InstrumentType::TomHigh,
        12 => InstrumentType::Perc1,
        13 => InstrumentType::Perc2,
        14 => InstrumentType::Fx1,
        15 => InstrumentType::Fx2,
        _ => InstrumentType::Kick,
    }
}

pub fn default_note_for_slot(index: usize) -> u8 {
    match index {
        0 => 36,
        1 => 38,
        2 => 42,
        3 => 46,
        4 => 44,
        5 => 51,
        6 => 49,
        7 => 37,
        8 => 39,
        9 => 41,
        10 => 45,
        11 => 48,
        12 => 50,
        13 => 52,
        14 => 55,
        15 => 57,
        16..=31 => 36 + (index as u8 - 16),
        _ => 36,
    }
}

pub fn instrument_label(instrument: InstrumentType) -> &'static str {
    match instrument {
        InstrumentType::Kick => "Kick",
        InstrumentType::Snare => "Snare",
        InstrumentType::HatClosed => "Hat Closed",
        InstrumentType::HatOpen => "Hat Open",
        InstrumentType::HatPedal => "Hat Pedal",
        InstrumentType::Ride => "Ride",
        InstrumentType::Crash => "Crash",
        InstrumentType::Rimshot => "Rimshot",
        InstrumentType::Clap => "Clap",
        InstrumentType::TomLow => "Tom Low",
        InstrumentType::TomMid => "Tom Mid",
        InstrumentType::TomHigh => "Tom High",
        InstrumentType::Perc1 => "Perc 1",
        InstrumentType::Perc2 => "Perc 2",
        InstrumentType::Fx1 => "FX 1",
        InstrumentType::Fx2 => "FX 2",
    }
}
