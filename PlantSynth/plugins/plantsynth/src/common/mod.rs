use nih_plug::prelude::*;
use enum_iterator::Sequence;
pub use crate::waveform::Waveform;
pub use crate::filter::FilterType;
pub use crate::modulator::OscillatorShape;
pub use crate::output_saturation::OutputSaturationType;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum OscRouting {
    ClassicOnly,
    WavetableOnly,
    Blend,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum ModSource {
    Lfo1,
    Lfo2,
    AmpEnv,
    FilterEnv,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum ModTarget {
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
pub enum FmSource {
    Classic,
    Wavetable,
    Sub,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum FmTarget {
    Classic,
    Wavetable,
    Both,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum GlideMode {
    Off,
    Legato,
    Always,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum UnisonVoices {
    One,
    Two,
    Four,
    Six,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum FilterRouting {
    Serial,
    Parallel,
    Morph,
}
