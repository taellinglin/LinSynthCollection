use nih_plug::prelude::*;
use enum_iterator::Sequence;

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
    Pan,
    Gain,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum FilterRouting {
    Serial,
    Parallel,
    Morph,
}
