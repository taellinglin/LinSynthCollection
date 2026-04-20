pub mod common;
pub mod sub_synth;
pub mod drum_synth;
mod editor;
pub mod envelope;
pub mod filter;
pub mod chorus;
pub mod delay;
pub mod reverb;
pub mod limiter;
pub mod multi_filter;
pub mod waveform;
pub mod modulator;
pub mod eq;
pub mod distortion;
pub mod output_saturation;
pub mod drum_engine;
pub mod sample;
pub mod util;

use nih_plug::prelude::*;

// Re-export common types for the editor and other modules
pub use common::*;
pub use sub_synth::SubSynth;
pub use sub_synth::params::SubSynthParams;
pub use drum_synth::DrumSynth;
pub use drum_synth::params::DrumSynthParams;

// Re-export nested types for convenience and compatibility
pub use filter::FilterType;
pub use waveform::Waveform;
pub use modulator::OscillatorShape;
pub use output_saturation::OutputSaturationType;

// NIH-plug exports
nih_export_clap!(SubSynth);
nih_export_vst3!(SubSynth);

// DrumSynth might be exported via a separate binary or just kept internal
// nih_export_clap!(DrumSynth);
