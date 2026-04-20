pub mod common;
pub mod sub_synth;
mod editor;
pub mod envelope;
pub mod filter;
pub mod chorus;
pub mod delay;
pub mod reverb;
pub mod limiter;
pub mod multi_filter;
pub mod preset_bank;
pub mod waveform;
pub mod modulator;
pub mod resonator;
pub mod util;

use nih_plug::prelude::*;

// Re-export common types
pub use common::*;
pub use sub_synth::SubSynth;
pub use sub_synth::params::SubSynthParams;

// Re-export nested types for compatibility
pub use filter::{FilterStyle, FilterType};
pub use resonator::ResonatorTimbre;
pub use waveform::{Waveform};
pub use modulator::{OscillatorShape};

// NIH-plug exports
nih_export_clap!(SubSynth);
nih_export_vst3!(SubSynth);
