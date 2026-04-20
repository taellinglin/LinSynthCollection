pub mod params;

use nih_plug::prelude::*;
use std::sync::Arc;
use crate::drum_engine::DrumEngine;
use crate::drum_synth::params::{DrumSynthParams, DRUM_SLOTS, DRUM_OUTPUT_PAIRS};
use crate::drum_synth::params::DrumSynthBlockParams;

pub struct DrumSynth {
    pub params: Arc<DrumSynthParams>,
    pub engine: DrumEngine,
    pub comp_env: [f32; DRUM_OUTPUT_PAIRS],
    pub pad_triggers: [f32; DRUM_SLOTS],
}

impl Default for DrumSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(DrumSynthParams::default()),
            engine: DrumEngine::new(),
            comp_env: [0.0; DRUM_OUTPUT_PAIRS],
            pad_triggers: [0.0; DRUM_SLOTS],
        }
    }
}

impl DrumSynth {
    fn apply_master_fx(&mut self, output_slices: &mut Vec<&mut [f32]>, bp: &DrumSynthBlockParams, sample_rate: f32) {
        if bp.master_gain == 1.0 && bp.master_drive <= 0.0 && bp.master_comp <= 0.0 && bp.master_clip <= 0.0 {
            return;
        }

        let drive_amount = 1.0 + bp.master_drive * 6.0;
        let clip_amount = 1.0 + bp.master_clip * 10.0;
        let threshold = util::db_to_gain(-18.0 + bp.master_comp * 10.0);
        let ratio = 1.5 + bp.master_comp * 5.0;
        let attack = (-1.0 / (0.005 * sample_rate)).exp();
        let release = (-1.0 / (0.08 * sample_rate)).exp();

        let output_pairs = output_slices.len() / 2;
        for pair in 0..output_pairs {
            let left_index = pair * 2;
            let right_index = left_index + 1;
            
            // Safety check for slicing
            if right_index >= output_slices.len() { break; }

            // Temporary workarounds for split_at_mut complexity in a loop
            // In a real implementation we'd use a more efficient way to access pairs
            let (left_slice, right_slice) = output_slices.split_at_mut(right_index);
            let left = &mut left_slice[left_index];
            let right = &mut right_slice[0];

            for idx in 0..left.len() {
                let mut left_sample = left[idx] * bp.master_gain;
                let mut right_sample = right[idx] * bp.master_gain;

                if bp.master_comp > 0.0 {
                    let detector = left_sample.abs().max(right_sample.abs());
                    if detector > self.comp_env[pair] {
                        self.comp_env[pair] = self.comp_env[pair] * attack + detector * (1.0 - attack);
                    } else {
                        self.comp_env[pair] = self.comp_env[pair] * release + detector * (1.0 - release);
                    }
                    if self.comp_env[pair] > threshold {
                        let gain = (threshold + (self.comp_env[pair] - threshold) / ratio) / self.comp_env[pair];
                        left_sample *= gain;
                        right_sample *= gain;
                    }
                }

                if bp.master_drive > 0.0 {
                    left_sample = (left_sample * drive_amount).tanh() / drive_amount;
                    right_sample = (right_sample * drive_amount).tanh() / drive_amount;
                }

                if bp.master_clip > 0.0 {
                    left_sample = (left_sample * clip_amount).tanh() / clip_amount;
                    right_sample = (right_sample * clip_amount).tanh() / clip_amount;
                }

                left[idx] = left_sample;
                right[idx] = right_sample;
            }
        }
    }
}

impl Plugin for DrumSynth {
    const NAME: &'static str = "PlantSynth Drums";
    const VENDOR: &'static str = "Ling Lin";
    const URL: &'static str = "https://github.com/robbert-vdh/nih-plug";
    const EMAIL: &'static str = "taellinglin@gmail.com";
    const VERSION: &'static str = "0.1.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        // Aux outputs handled by Nih-plug
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn process(&mut self, buffer: &mut Buffer, aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        self.engine.sync_sample_buffers(&self.params.sample_data, &self.params.sample_paths);
        
        let sample_rate = context.transport().sample_rate;
        let mut output_slices: Vec<&mut [f32]> = Vec::new();
        
        // Collect all buffers (main + aux)
        for channel in buffer.as_slice().iter_mut() {
            channel.fill(0.0);
            output_slices.push(*channel);
        }
        for aux_buffer in aux.outputs.iter_mut() {
            for channel in aux_buffer.as_slice().iter_mut() {
                channel.fill(0.0);
                output_slices.push(*channel);
            }
        }

        if output_slices.len() < 2 { return ProcessStatus::Normal; }

        let bp = DrumSynthBlockParams::cache(&self.params);

        // Handle Events
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { note, velocity, .. } = event {
                if let Some(slot) = self.params.slots.iter().position(|s| s.midi_note.value() as u8 == note) {
                    self.engine.trigger(slot, &self.params.slots[slot], velocity, Some(note));
                }
            }
        }

        // Handle GUI Pad Triggers
        for (slot, slot_params) in self.params.slots.iter().enumerate() {
            let current = slot_params.pad_trigger.value();
            if (current - self.pad_triggers[slot]).abs() > 1.0e-5 {
                self.pad_triggers[slot] = current;
                self.engine.trigger(slot, slot_params, 1.0, None);
            }
        }

        // Engine Processing
        self.engine.process(&mut output_slices);

        // Master FX
        self.apply_master_fx(&mut output_slices, &bp, sample_rate);

        ProcessStatus::Normal
    }
}

impl ClapPlugin for DrumSynth {
    const CLAP_ID: &'static str = "art.plantsynth.drums";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Sample-based drum machine with 32 slots");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Drum,
        ClapFeature::DrumMachine,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for DrumSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"CatDrumLing1A01X";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Drum,
        Vst3SubCategory::Stereo,
    ];
}
