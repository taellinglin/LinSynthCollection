mod editor;
mod envelope;
mod filter;
mod waveform;
mod modulator;
mod chorus;
mod presets;
mod sf2;

use nih_plug::params::enums::EnumParam;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::Rng;
use rand_pcg::Pcg32;
use std::sync::Arc;

use modulator::{Modulator, OscillatorShape};
use envelope::{ADSREnvelope, Envelope, ADSREnvelopeState};
use filter::{FilterType, Filter};
use waveform::{generate_waveform, generate_fm_waveform, Waveform};
use chorus::Chorus;
use sf2::{load_sf2_bank, resolve_sf2_path, wavetable_sample, Sf2Bank, Sf2Env, Sf2Zone};

const NUM_VOICES: usize = 16;
const MAX_BLOCK_SIZE: usize = 64;
const GAIN_POLY_MOD_ID: u32 = 0;

struct FishSynth {
    params: Arc<FishSynthParams>,
    prng: Pcg32,
    voices: [Option<Voice>; NUM_VOICES as usize],
    next_voice_index: usize,
    next_internal_voice_id: u64,
    chorus: Chorus,
    current_program: i32,
    sf2_bank: Option<Arc<Sf2Bank>>,
    compressor_env: f32,
}

#[derive(Params)]
struct FishSynthParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    #[id = "program"]
    program: IntParam,
    #[id = "gain"]
    gain: FloatParam,
    #[id = "amp_atk"]
    amp_attack_ms: FloatParam,
    #[id = "amp_rel"]
    amp_release_ms: FloatParam,
    #[id = "waveform"]
    waveform: EnumParam<Waveform>,
    #[id = "sub_waveform"]
    sub_waveform: EnumParam<Waveform>,
    #[id = "sub_mix"]
    sub_mix: FloatParam,
    #[id = "noise_mix"]
    noise_mix: FloatParam,

    // New parameters for ADSR envelope
    #[id = "amp_dec"]
    amp_decay_ms: FloatParam,
    #[id = "amp_sus"]
    amp_sustain_level: FloatParam,
    #[id = "filter_cut_atk"]
    filter_cut_attack_ms: FloatParam,
    #[id = "filter_cut_dec"]
    filter_cut_decay_ms: FloatParam,
    #[id = "filter_cut_sus"]
    filter_cut_sustain_ms: FloatParam,
    #[id = "filter_cut_rel"]
    filter_cut_release_ms: FloatParam,
    #[id = "filter_res_atk"]
    filter_res_attack_ms: FloatParam,
    #[id = "filter_res_dec"]
    filter_res_decay_ms: FloatParam,
    #[id = "filter_res_sus"]
    filter_res_sustain_ms: FloatParam,
    #[id = "filter_res_rel"]
    filter_res_release_ms: FloatParam,
    #[id = "filter_type"]
    filter_type: EnumParam<FilterType>,
    #[id = "filter_cut"]
    filter_cut: FloatParam,
    #[id = "filter_res"]
    filter_res: FloatParam,
    #[id = "filter_amount"]
    filter_amount: FloatParam,
    // New parameters for ADSR envelope levels
    #[id = "amp_env_level"]
    amp_envelope_level: FloatParam,
    #[id = "filter_cut_env_level"]
    filter_cut_envelope_level: FloatParam,
    #[id = "filter_res_env_level"]
    filter_res_envelope_level: FloatParam,
    #[id = "vibrato_atk"]
    vibrato_attack: FloatParam,
    #[id = "vibrato_int"]
    vibrato_intensity: FloatParam,
    #[id = "vibrato_rate"]
    vibrato_rate: FloatParam,
    #[id = "tremolo_atk"]
    tremolo_attack: FloatParam,
    #[id = "tremolo_int"]
    tremolo_intensity: FloatParam,
    #[id = "tremolo_rate"]
    tremolo_rate: FloatParam,
    #[id = "vibrato_shape"]
    vibrato_shape: EnumParam<OscillatorShape>,
    #[id = "tremolo_shape"]
    tremolo_shape: EnumParam<OscillatorShape>,
    #[id = "tension"]
    tension: FloatParam,
    #[id = "chorus_enable"]
    chorus_enable: BoolParam,
    #[id = "chorus_rate"]
    chorus_rate: FloatParam,
    #[id = "chorus_depth"]
    chorus_depth: FloatParam,
    #[id = "chorus_mix"]
    chorus_mix: FloatParam,
    
    // FM Synthesis parameters
    #[id = "fm_enable"]
    fm_enable: BoolParam,
    #[id = "fm_ratio"]
    fm_ratio: FloatParam,
    #[id = "fm_amount"]
    fm_amount: FloatParam,
    #[id = "modulator_waveform"]
    modulator_waveform: EnumParam<Waveform>,
}

#[derive(Debug, Clone)]
struct Voice {
    voice_id: i32,
    channel: u8,
    note: u8,
    internal_voice_id: u64,
    velocity: f32,
    velocity_sqrt: f32,
    phase: f32,
    phase_delta: f32,
    releasing: bool,
    amp_envelope: ADSREnvelope,
    voice_gain: Option<(f32, Smoother<f32>)>,
    filter_cut_envelope: ADSREnvelope,
    filter_res_envelope: ADSREnvelope,
    filter: Option<FilterType>,
    lowpass_filter: filter::LowpassFilter,
    highpass_filter: filter::HighpassFilter,
    bandpass_filter: filter::BandpassFilter,
    notch_filter: filter::NotchFilter,
    statevariable_filter: filter::StatevariableFilter,
    pressure: f32,
    pan: f32,        // Added pan field
    tuning: f32,     // Add tuning field
    vibrato: f32,    // Add vibrato field
    expression: f32, // Add expression field
    brightness: f32, // Add brightness field
    vib_mod: Modulator,
    trem_mod: Modulator,
    
    // FM Synthesis state
    modulator_phase: f32,
    modulator_phase_delta: f32,
    sub_phase: f32,
    sub_phase_delta: f32,
    sf2_zone: Option<Arc<Sf2Zone>>,
    sf2_phase: f32,
    sf2_phase_delta: f32,
}

impl Default for FishSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(FishSynthParams::default()),

            prng: Pcg32::new(420, 1337),
            voices: [0; NUM_VOICES as usize].map(|_| None),
            next_internal_voice_id: 0,
            next_voice_index: 0,
            chorus: Chorus::new(44100.0),
            current_program: 0,
            sf2_bank: None,
            compressor_env: 0.0,
        }
    }
}

impl Default for FishSynthParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            program: IntParam::new(
                "Program",
                0,
                IntRange::Linear { min: 0, max: 127 },
            ),
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(-24.0),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(0.0),
                },
            )
            .with_poly_modulation_id(GAIN_POLY_MOD_ID)
            .with_smoother(SmoothingStyle::Logarithmic(5.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            amp_attack_ms: FloatParam::new(
                "Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_release_ms: FloatParam::new(
                "Release",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            waveform: EnumParam::new("Carrier Wave", Waveform::Sine),
            sub_waveform: EnumParam::new("Sub Wave", Waveform::Sine),
            sub_mix: FloatParam::new(
                "Sub Mix",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            noise_mix: FloatParam::new(
                "Noise Mix",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),
            amp_decay_ms: FloatParam::new(
                "Decay",
                10.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_sustain_level: FloatParam::new(
                "Sustain",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" units"),
            filter_type: EnumParam::new("Filter Type", FilterType::None),
            filter_cut: FloatParam::new(
                "Filter Cutoff",
                1000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(10.0)),
            filter_res: FloatParam::new(
                "Filter Resonance",
                0.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0)),
            filter_amount: FloatParam::new(
                "Filter Amount",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(2))
            .with_string_to_value(formatters::s2v_f32_percentage()),
            filter_cut_attack_ms: FloatParam::new(
                "Filter Cut Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_cut_decay_ms: FloatParam::new(
                "Filter Cut Decay",
                10.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_cut_sustain_ms: FloatParam::new(
                "Filter Cut Sustain",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_cut_release_ms: FloatParam::new(
                "Filter Cut Release",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_attack_ms: FloatParam::new(
                "Filter Resonance Attack",
                10.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_decay_ms: FloatParam::new(
                "Filter Resonance Decay",
                10.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            filter_res_sustain_ms: FloatParam::new(
                "Filter Resonance Sustain",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_res_release_ms: FloatParam::new(
                "Filter Resonance Release",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            amp_envelope_level: FloatParam::new(
                "Amplitude Envelope Level",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_cut_envelope_level: FloatParam::new(
                "Filter Cutoff Envelope Level",
                0.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            filter_res_envelope_level: FloatParam::new(
                "Filter Resonance Envelope Level",
                0.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01),
            vibrato_attack: FloatParam::new(
                "Vibrato Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            vibrato_intensity: FloatParam::new(
                "Vibrato Intensity",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            vibrato_rate: FloatParam::new(
                "Vibrato Rate",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 32.0,
                },
            )
            .with_step_size(1.0)
            .with_unit(" Hz"),
            tremolo_attack: FloatParam::new(
                "Tremolo Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_step_size(0.01)
            .with_unit(" ms"),
            tremolo_intensity: FloatParam::new(
                "Tremolo Intensity",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            tremolo_rate: FloatParam::new(
                "Tremolo Rate",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            vibrato_shape: EnumParam::new("Vibrato Shape", OscillatorShape::Sine),
            tremolo_shape: EnumParam::new("Tremolo Shape", OscillatorShape::Sine),
            tension: FloatParam::new(
                "Envelope Tension",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            chorus_enable: BoolParam::new("Chorus Enable", false),
            chorus_rate: FloatParam::new(
                "Chorus Rate",
                0.5,
                FloatRange::Linear {
                    min: 0.1,
                    max: 5.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(" Hz"),
            chorus_depth: FloatParam::new(
                "Chorus Depth",
                10.0,
                FloatRange::Linear {
                    min: 1.0,
                    max: 50.0,
                },
            )
            .with_step_size(0.1)
            .with_unit(" ms"),
            chorus_mix: FloatParam::new(
                "Chorus Mix",
                0.5,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            
            // FM Synthesis parameters initialization
            fm_enable: BoolParam::new("FM Enable", true),
            fm_ratio: FloatParam::new(
                "FM Ratio",
                1.0,
                FloatRange::Linear {
                    min: 0.25,
                    max: 16.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            fm_amount: FloatParam::new(
                "FM Amount",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(""),
            modulator_waveform: EnumParam::new("Modulator Waveform", Waveform::Sine),
        }
    }
}

impl Plugin for FishSynth {
    const NAME: &'static str = "FishSynth";
    const VENDOR: &'static str = "LingYue Synth";
    const URL: &'static str = "https://taellinglin.art";
    const EMAIL: &'static str = "taellinglin@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // After `PEAK_METER_DECAY_MS` milliseconds of pure silence, the peak meter's value should
        // have dropped by 12 dB
        self.chorus.set_sample_rate(buffer_config.sample_rate);

        if self.sf2_bank.is_none() {
            let sf2_path = resolve_sf2_path();
            match load_sf2_bank(&sf2_path) {
                Ok(bank) => {
                    self.sf2_bank = Some(Arc::new(bank));
                }
                Err(err) => {
                    eprintln!("sf2 load failed: {err}");
                }
            }
        }

        let selected_program = self.params.program.value();
        let values = presets::preset_values_for(selected_program as usize);
        presets::apply_preset_to_params(&self.params, values);
        self.current_program = selected_program;

        true
    }

    fn reset(&mut self) {
        self.prng = Pcg32::new(420, 1337);

        self.voices.fill(None);
        self.next_internal_voice_id = 0;
        self.chorus.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let selected_program = self.params.program.value();
        if selected_program != self.current_program {
            let values = presets::preset_values_for(selected_program as usize);
            presets::apply_preset_to_params(&self.params, values);
            self.current_program = selected_program;
        }

        // NIH-plug has a block-splitting adapter for `Buffer`. While this works great for effect
        // plugins, for polyphonic synths the block size should be `min(MAX_BLOCK_SIZE,
        // num_remaining_samples, next_event_idx - block_start_idx)`. Because blocks also need to be
        // split on note events, it's easier to work with raw audio here and to do the splitting by
        // hand.
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let output = buffer.as_slice();

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);
        while block_start < num_samples {
            // First of all, handle all note events that happen at the start of the block, and cut
            // the block short if another event happens before the end of it. To handle polyphonic
            // modulation for new notes properly, we'll keep track of the next internal note index
            // at the block's start. If we receive polyphonic modulation that matches a voice that
            // has an internal note ID that's great than or equal to this one, then we should start
            // the note's smoother at the new value instead of fading in from the global value.
            let this_sample_internal_voice_id_start = self.next_internal_voice_id;
            'events: loop {
                match next_event {
                    // If the event happens now, then we'll keep processing events
                    Some(event) if (event.timing() as usize) < block_end => {
                        // This synth doesn't support any of the polyphonic expression events. A
                        // real synth plugin, however, will want to support those.
                        match event {
                            NoteEvent::NoteOn {
                                timing,
                                voice_id,
                                channel,
                                note,
                                velocity,
                            } => {
                                let pan: f32 = 0.5;
                                let pressure: f32 = 1.0;
                                let brightness: f32 = 1.0;
                                let expression: f32 = 1.0;
                                let vibrato: f32 = 0.0;
                                let tuning: f32 = 0.0;
                                let initial_phase: f32 = self.prng.gen();
                                let vibrato_lfo = Modulator::new(
                                    self.params.vibrato_rate.value(), 
                                    self.params.vibrato_intensity.value(), 
                                    self.params.vibrato_attack.value(), 
                                    self.params.vibrato_shape.value(),
                                );
                                let tremolo_lfo = Modulator::new(
                                    self.params.tremolo_rate.value(), 
                                    self.params.tremolo_intensity.value(), 
                                    self.params.tremolo_attack.value(), 
                                    self.params.tremolo_shape.value(),
                                );
                                // This starts with the attack portion of the amplitude envelope
                                let (mut amp_envelope, cutoff_envelope, resonance_envelope) =
                                    self.construct_envelopes(sample_rate, velocity);
                                let sf2_zone = self.select_sf2_zone(note, velocity);
                                if let Some(zone) = sf2_zone.as_ref() {
                                    Self::apply_sf2_env(&mut amp_envelope, zone.env);
                                }
                                let voice = self.start_voice(
                                    context, timing, voice_id, channel, note,
                                    velocity, // Add velocity parameter
                                    pan, pressure, brightness, expression, // Add expression parameter
                                    vibrato,    // Add vibrato parameter
                                    tuning,
                                    vibrato_lfo,
                                    tremolo_lfo,
                                    amp_envelope,
                                    cutoff_envelope,
                                    resonance_envelope,
                                    self.params.filter_type.value(),
                                    sample_rate,  // Pass actual sample rate
                                );
                                
                                voice.vib_mod = vibrato_lfo.clone();
                                voice.trem_mod = tremolo_lfo.clone();
                                voice.velocity_sqrt = velocity.sqrt();
                                voice.phase = initial_phase;
                                voice.sf2_phase = initial_phase;
                                voice.vib_mod.trigger();
                                voice.trem_mod.trigger();
                                let pitch = util::midi_note_to_freq(note)
                                    * (2.0_f32).powf((tuning + voice.tuning ) / 12.0);
                                voice.phase_delta = pitch / sample_rate;
                                if let Some(zone) = sf2_zone {
                                    voice.sf2_zone = Some(zone.clone());
                                    let root_freq = util::midi_note_to_freq(zone.root_key);
                                    let note_freq = util::midi_note_to_freq(note);
                                    let tune = (2.0_f32).powf(zone.tune_cents as f32 / 1200.0);
                                    let rate = (note_freq / root_freq) * tune;
                                    let sample_rate_ratio = zone.sample_rate / sample_rate;
                                    voice.sf2_phase_delta = rate * sample_rate_ratio;
                                    voice.pan = zone.pan;
                                }
                                voice.amp_envelope = amp_envelope;
                                voice.filter_cut_envelope = cutoff_envelope;
                                voice.filter_res_envelope = resonance_envelope;
                                voice.velocity = velocity;
                                voice.pan = pan;

                                
                            }
                            NoteEvent::NoteOff {
                                timing: _,
                                voice_id,
                                channel,
                                note,
                                velocity: _,
                            } => {
                                self.start_release_for_voices(sample_rate, voice_id, channel, note);
                            }
                            NoteEvent::MidiProgramChange {
                                timing: _,
                                channel: _,
                                program,
                            } => {
                                let values = presets::preset_values_for(program as usize);
                                presets::apply_preset_to_params(&self.params, values);
                                if let Some(bank) = &self.sf2_bank {
                                    if let Some(env) = bank.preset_env(program as u8) {
                                        self.params.amp_attack_ms.set_plain_value(env.attack);
                                        self.params.amp_decay_ms.set_plain_value(env.decay);
                                        self.params.amp_sustain_level.set_plain_value(env.sustain);
                                        self.params.amp_release_ms.set_plain_value(env.release);
                                    }
                                }
                                self.params.program.set_plain_value(program as i32);
                                self.current_program = program as i32;
                            }
                            NoteEvent::Choke {
                                timing,
                                voice_id,
                                channel,
                                note,
                            } => {
                                self.choke_voices(context, timing, voice_id, channel, note);
                            }
                            NoteEvent::PolyModulation {
                                timing: _,
                                voice_id,
                                poly_modulation_id,
                                normalized_offset,
                            } => {
                                // Polyphonic modulation events are matched to voices using the
                                // voice ID, and to parameters using the poly modulation ID. The
                                // host will probably send a modulation event every N samples. This
                                // will happen before the voice is active, and of course also after
                                // it has been terminated (because the host doesn't know that it
                                // will be). Because of that, we won't print any assertion failures
                                // when we can't find the voice index here.
                                if let Some(voice_idx) = self.get_voice_idx(voice_id) {
                                    let voice = self.voices[voice_idx].as_mut().unwrap();

                                    match poly_modulation_id {
                                        GAIN_POLY_MOD_ID => {
                                            // This should either create a smoother for this
                                            // modulated parameter or update the existing one.
                                            // Notice how this uses the parameter's unmodulated
                                            // normalized value in combination with the normalized
                                            // offset to create the target plain value
                                            let target_plain_value = self
                                                .params
                                                .gain
                                                .preview_modulated(normalized_offset);
                                            let (_, smoother) =
                                                voice.voice_gain.get_or_insert_with(|| {
                                                    (
                                                        normalized_offset,
                                                        self.params.gain.smoothed.clone(),
                                                    )
                                                });

                                            // If this `PolyModulation` events happens on the
                                            // same sample as a voice's `NoteOn` event, then it
                                            // should immediately use the modulated value
                                            // instead of slowly fading in
                                            if voice.internal_voice_id
                                                >= this_sample_internal_voice_id_start
                                            {
                                                smoother.reset(target_plain_value);
                                            } else {
                                                smoother
                                                    .set_target(sample_rate, target_plain_value);
                                            }
                                        }
                                        n => nih_debug_assert_failure!(
                                            "Polyphonic modulation sent for unknown poly \
                                            modulation ID {}",
                                            n
                                        ),
                                    }
                                }
                            }
                            NoteEvent::MonoAutomation {
                                timing: _,
                                poly_modulation_id,
                                normalized_value,
                            } => {
                                // Modulation always acts as an offset to the parameter's current
                                // automated value. So if the host sends a new automation value for
                                // a modulated parameter, the modulated values/smoothing targets
                                // need to be updated for all polyphonically modulated voices.
                                for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                                    match poly_modulation_id {
                                        GAIN_POLY_MOD_ID => {
                                            let (normalized_offset, smoother) =
                                                match voice.voice_gain.as_mut() {
                                                    Some((o, s)) => (o, s),
                                                    // If the voice does not have existing
                                                    // polyphonic modulation, then there's nothing
                                                    // to do here. The global automation/monophonic
                                                    // modulation has already been taken care of by
                                                    // the framework.
                                                    None => continue,
                                                };
                                            let target_plain_value =
                                                self.params.gain.preview_plain(
                                                    normalized_value + *normalized_offset,
                                                );
                                            smoother.set_target(sample_rate, target_plain_value);
                                        }
                                        n => nih_debug_assert_failure!(
                                            "Automation event sent for unknown poly modulation ID \
                                            {}",
                                            n
                                        ),
                                    }
                                }
                            }
                            NoteEvent::PolyPressure {
                                timing,
                                voice_id,
                                channel,
                                note,
                                pressure,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice.as_mut() {
                                            let velocity_sqrt = voice_inner.velocity_sqrt;
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                0.0,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyVolume {
                                timing,
                                voice_id,
                                channel,
                                note,
                                gain,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyPan {
                                timing,
                                voice_id,
                                channel,
                                note,
                                pan,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let gain = voice_inner.velocity;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyTuning {
                                timing,
                                voice_id,
                                channel,
                                note,
                                tuning,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let gain = voice_inner.velocity;
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let vibrato = voice_inner.vibrato;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            NoteEvent::PolyVibrato {
                                timing,
                                voice_id,
                                channel,
                                note,
                                vibrato,
                            } => {
                                if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
                                    if let Some(voice) = self.voices.get_mut(voice_idx) {
                                        if let Some(voice_inner) = voice {
                                            let gain = voice_inner.velocity;
                                            let pan = voice_inner.pan;
                                            let brightness = voice_inner.brightness;
                                            let expression = voice_inner.expression;
                                            let tuning = voice_inner.tuning;
                                            let amp_envelope = voice_inner.amp_envelope.clone();
                                            let filter_cut_envelope = voice_inner.filter_cut_envelope.clone();
                                            let filter_res_envelope = voice_inner.filter_res_envelope.clone();
                                            let vib_mod = voice_inner.vib_mod.clone();
                                            let trem_mod = voice_inner.trem_mod.clone();
                                            let pressure = voice_inner.pressure;
                            
                                            self.handle_poly_event(
                                                timing,
                                                voice_id,
                                                channel,
                                                note,
                                                gain,
                                                pan,
                                                brightness,
                                                expression,
                                                tuning,
                                                pressure,
                                                vibrato,
                                                Some(&amp_envelope),
                                                Some(&filter_cut_envelope),
                                                Some(&filter_res_envelope),
                                                Some(&vib_mod),
                                                Some(&trem_mod),
                                            );
                                        }
                                    }
                                }
                            }
                            
                            
                            // Handle other MIDI events if needed
                            _ => (),
                        };

                        next_event = context.next_event();
                    }
                    // If the event happens before the end of the block, then the block should be cut
                    // short so the next block starts at the event
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            // We'll start with silence, and then add the output from the active voices
            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);

            // These are the smoothed global parameter values. These are used for voices that do not
            // have polyphonic modulation applied to them. With a plugin as simple as this it would
            // be possible to avoid this completely by simply always copying the smoother into the
            // voice's struct, but that may not be realistic when the plugin has hundreds of
            // parameters. The `voice_*` arrays are scratch arrays that an individual voice can use.
            let block_len = block_end - block_start;
            let mut gain = [0.0; MAX_BLOCK_SIZE];
            let mut voice_gain = [0.0; MAX_BLOCK_SIZE];
            self.params.gain.smoothed.next_block(&mut gain, block_len);

            // TODO: Some form of band limiting
            // TODO: Filter
            for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
                // Get mutable reference to the voice at sample_idx
                for voice in self.voices.iter_mut() {
                    if let Some(voice) = voice {
                        // Depending on whether the voice has polyphonic modulation applied to it,
                        // either the global parameter values are used, or the voice's smoother is used
                        // to generate unique modulated values for that voice
                        let gain = match &voice.voice_gain {
                            Some((_, smoother)) => {
                                smoother.next_block(&mut voice_gain, block_len);
                                &voice_gain
                            }
                            None => &gain,
                        };

                        // This is an exponential smoother repurposed as an AR envelope with values between
                        // 0 and 1. When a note off event is received, this envelope will start fading out
                        // again. When it reaches 0, we will terminate the voice.
                        
                        
                        let dc_blocker = filter::DCBlocker::new();
                        // Apply filter
                        let filter_type = self.params.filter_type.value();
                        let vib_shape =  self.params.vibrato_shape.value();
                        let trem_shape =  self.params.tremolo_shape.value();
                        voice.filter = Some(filter_type);
                        let cutoff = self.params.filter_cut.value();
                        let resonance = self.params.filter_res.value();
                        let waveform = self.params.waveform.value();
                        let vib_int: f32 = self.params.vibrato_intensity.value();
                        let vib_rate: f32 = self.params.vibrato_rate.value();
                        // Calculate panning based on voice's pan value
                        let pan = voice.pan;
                        let left_amp = (1.0 - pan).sqrt() as f32;
                        let right_amp = pan.sqrt() as f32;
                        // Vibrato modulation (LFO-based)
                        let vibrato_modulation = voice.vib_mod.get_modulation(sample_rate);
                        // Apply vibrato to the voice's phase_delta (which affects pitch)
                        let vibrato_phase_delta = voice.phase_delta * (1.0 + (vib_int * vibrato_modulation)); 
                        //filtered_sample.set_sample_rate(sample_rate);
                        // Advance envelopes once per sample
                        voice.amp_envelope.advance();
                        voice.filter_cut_envelope.advance();
                        voice.filter_res_envelope.advance();

                        // Generate waveform for voice (SF2 wavetable or synth)
                        let (base_sample, sf2_right, sf2_stereo, is_sf2) =
                            if let Some(zone) = &voice.sf2_zone {
                                let left = wavetable_sample(&zone.table_left, voice.sf2_phase);
                                let right = zone
                                    .table_right
                                    .as_ref()
                                    .map(|table| wavetable_sample(table, voice.sf2_phase));
                                (left, right.unwrap_or(left), right.is_some(), true)
                        } else if self.params.fm_enable.value() {
                            // FM synthesis
                            let fm_ratio = self.params.fm_ratio.value();
                            let fm_amount = self.params.fm_amount.value();
                            let modulator_waveform = self.params.modulator_waveform.value();
                            
                            // Update modulator phase and phase delta
                            voice.modulator_phase_delta = voice.phase_delta * fm_ratio;
                            
                            (
                                generate_fm_waveform(
                                    waveform,
                                    modulator_waveform,
                                    voice.phase,
                                    voice.modulator_phase,
                                    fm_amount,
                                ),
                                0.0,
                                false,
                                false,
                            )
                        } else {
                            // Standard waveform generation
                            (generate_waveform(waveform, voice.phase), 0.0, false, false)
                        };

                        voice.sub_phase_delta = voice.phase_delta * 0.5;
                        let generated_sample = if is_sf2 {
                            base_sample
                        } else {
                            let sub_mix = self.params.sub_mix.value().clamp(0.0, 1.0);
                            let noise_mix = self.params.noise_mix.value().clamp(0.0, 1.0);
                            let main_mix = (1.0 - (sub_mix + noise_mix)).max(0.0);
                            let sub_waveform = self.params.sub_waveform.value();
                            let sub_sample = generate_waveform(sub_waveform, voice.sub_phase);
                            let noise_sample = generate_waveform(Waveform::Noise, 0.0);
                            base_sample * main_mix + sub_sample * sub_mix + noise_sample * noise_mix
                        };
                        
                        // Get envelope values (scaled from 0-1)
                        let filter_cut_env_value = voice.filter_cut_envelope.get_value();
                        let filter_res_env_value = voice.filter_res_envelope.get_value();
                        
                        // Apply envelope modulation to filter parameters
                        // For squelchy sounds, we want the envelope to sweep the filter
                        // Envelope level controls the depth of modulation (0-1 range)
                        let env_cut_amount = self.params.filter_cut_envelope_level.value().max(0.0).min(1.0);
                        let env_res_amount = self.params.filter_res_envelope_level.value().max(0.0).min(1.0);
                        
                        // Modulate cutoff: sweep from a low frequency up to the set cutoff
                        // When env_amount = 0: use base cutoff value
                        // When env_amount = 1: envelope sweeps from 50Hz to base cutoff
                        // This creates classic filter sweep/squelch sounds
                        let min_cutoff = 50.0; // Minimum cutoff frequency for sweep
                        let cutoff_range = cutoff - min_cutoff;
                        let env_cutoff_offset = cutoff_range * (1.0 - filter_cut_env_value) * env_cut_amount;
                        let modulated_cutoff = cutoff - env_cutoff_offset;
                        
                        // Modulate resonance: envelope increases resonance
                        // When env_amount = 0: use base resonance
                        // When env_amount = 1: envelope adds resonance (0 to base resonance)
                        // Higher envelope values = more resonance for squelchy peaks
                        let env_res_offset = resonance * filter_res_env_value * env_res_amount;
                        let modulated_resonance = resonance * (1.0 - env_res_amount) + env_res_offset;
                        
                        // Clamp to valid ranges
                        let modulated_cutoff = modulated_cutoff.max(20.0).min(20000.0);
                        let modulated_resonance = modulated_resonance.max(0.0).min(1.0);
                        
                        // Apply filters using stored filter instances
                        let filtered_sample = match voice.filter.unwrap() {
                            FilterType::None => generated_sample,
                            FilterType::Lowpass => {
                                voice.lowpass_filter.set_cutoff(modulated_cutoff);
                                voice.lowpass_filter.set_resonance(modulated_resonance);
                                voice.lowpass_filter.process(generated_sample)
                            }
                            FilterType::Highpass => {
                                voice.highpass_filter.set_cutoff(modulated_cutoff);
                                voice.highpass_filter.set_resonance(modulated_resonance);
                                voice.highpass_filter.process(generated_sample)
                            }
                            FilterType::Bandpass => {
                                voice.bandpass_filter.set_cutoff(modulated_cutoff);
                                voice.bandpass_filter.set_resonance(modulated_resonance);
                                voice.bandpass_filter.process(generated_sample)
                            }
                            FilterType::Notch => {
                                voice.notch_filter.set_cutoff(modulated_cutoff);
                                voice.notch_filter.set_resonance(modulated_resonance);
                                voice.notch_filter.process(generated_sample)
                            }
                            FilterType::Statevariable => {
                                voice.statevariable_filter.set_cutoff(modulated_cutoff);
                                voice.statevariable_filter.set_resonance(modulated_resonance);
                                voice.statevariable_filter.process(generated_sample)
                            }
                        };
                        
                        // Apply filter amount (dry/wet blend)
                        let filter_amount = if is_sf2 {
                            0.0
                        } else {
                            self.params.filter_amount.value()
                        };
                        let final_sample = generated_sample * (1.0 - filter_amount) + filtered_sample * filter_amount;

                        // Calculate amplitude for voice with envelope level scaling
                        let amp_env_value = voice.amp_envelope.get_value();
                        let amp_env_level = if let Some(zone) = &voice.sf2_zone {
                            zone.amp_gain
                        } else {
                            self.params.amp_envelope_level.value()
                        };
                        let amp = voice.velocity_sqrt * gain[value_idx] * (amp_env_value * amp_env_level) * 0.5 *(voice.trem_mod.get_modulation(sample_rate)+1.0) ;
            
                        // Apply voice-specific processing to the filtered sample
                        let naive_waveform = final_sample;
                        let processed_sample = if is_sf2 {
                            naive_waveform * amp
                        } else {
                            let corrected_waveform =
                                naive_waveform - FishSynth::poly_blep(voice.phase, voice.phase_delta);
                            corrected_waveform * amp
                        };

                        // Apply panning and DC blocking
                        let (processed_left_sample, processed_right_sample) =
                            if is_sf2 && sf2_stereo {
                                let left = filter::DCBlocker::new().process(base_sample * amp);
                                let right = filter::DCBlocker::new().process(sf2_right * amp);
                                (left, right)
                            } else {
                                let dc_blocked_sample =
                                    filter::DCBlocker::new().process(processed_sample);
                                let left = (1.0 - voice.pan).sqrt() as f32 * dc_blocked_sample;
                                let right = voice.pan.sqrt() as f32 * dc_blocked_sample;
                                (left, right)
                            };

                        // Add the processed sample to the output channels
                        output[0][sample_idx] += processed_left_sample;
                        output[1][sample_idx] += processed_right_sample;

                        // Update voice phase
                        voice.phase += vibrato_phase_delta;
                        if voice.phase >= 1.0 {
                            voice.phase -= 1.0;
                        }

                        if is_sf2 {
                            voice.sf2_phase += voice.sf2_phase_delta;
                            if voice.sf2_phase >= 1.0 {
                                voice.sf2_phase -= 1.0;
                            }
                        }

                        voice.sub_phase += voice.sub_phase_delta;
                        if voice.sub_phase >= 1.0 {
                            voice.sub_phase -= 1.0;
                        }
                        
                        // Update modulator phase for FM synthesis
                        if self.params.fm_enable.value() {
                            voice.modulator_phase += voice.modulator_phase_delta;
                            if voice.modulator_phase >= 1.0 {
                                voice.modulator_phase -= 1.0;
                            }
                        }
                    }
                }
            }

            // Apply chorus effect to the entire block if enabled
            if self.params.chorus_enable.value() {
                let chorus_rate = self.params.chorus_rate.value();
                let chorus_depth = self.params.chorus_depth.value();
                let chorus_mix = self.params.chorus_mix.value();
                
                for sample_idx in block_start..block_end {
                    let (left, right) = self.chorus.process(
                        output[0][sample_idx],
                        output[1][sample_idx],
                        chorus_rate,
                        chorus_depth,
                        chorus_mix,
                    );
                    output[0][sample_idx] = left;
                    output[1][sample_idx] = right;
                }
            }

            // Simple stereo-linked compressor for overall level control
            let threshold = util::db_to_gain(-12.0);
            let ratio = 3.0;
            let attack_coeff = (-1.0 / (0.010 * sample_rate)).exp();
            let release_coeff = (-1.0 / (0.100 * sample_rate)).exp();
            let makeup_gain = util::db_to_gain(3.0);

            for sample_idx in block_start..block_end {
                let left = output[0][sample_idx];
                let right = output[1][sample_idx];
                let level = left.abs().max(right.abs());

                if level > self.compressor_env {
                    self.compressor_env =
                        attack_coeff * self.compressor_env + (1.0 - attack_coeff) * level;
                } else {
                    self.compressor_env =
                        release_coeff * self.compressor_env + (1.0 - release_coeff) * level;
                }

                let gain = if self.compressor_env <= threshold {
                    1.0
                } else {
                    let over = self.compressor_env / threshold;
                    let compressed = over.powf(1.0 / ratio);
                    (compressed / over).clamp(0.0, 1.0)
                };
                let final_gain = gain * makeup_gain;
                output[0][sample_idx] *= final_gain;
                output[1][sample_idx] *= final_gain;
            }

            // Terminate voices whose release period has fully ended. This could be done as part of
            // the previous loop but this is simpler.
            for voice in &mut self.voices {
                if let Some(v) = voice {
                    if v.releasing && v.amp_envelope.get_state() == ADSREnvelopeState::Idle {
                        context.send_event(NoteEvent::VoiceTerminated {
                            timing: block_end as u32,
                            voice_id: Some(v.voice_id),
                            channel: v.channel,
                            note: v.note,
                        });
                        *voice = None;
                    }
                }
            }

            // And then just keep processing blocks until we've run out of buffer to fill
            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl FishSynth {
    fn get_voice_idx(&mut self, voice_id: i32) -> Option<usize> {
        self.voices
            .iter_mut()
            .position(|voice| matches!(voice, Some(voice) if voice.voice_id == voice_id))
    }

    fn construct_envelopes(
        &self,
        sample_rate: f32,
        velocity: f32,
    ) -> (ADSREnvelope, ADSREnvelope, ADSREnvelope) {
        let tension = self.params.tension.value();
        let amp_attack = Self::ms_to_seconds(self.params.amp_attack_ms.value());
        let amp_decay = Self::ms_to_seconds(self.params.amp_decay_ms.value());
        let amp_release = Self::ms_to_seconds(self.params.amp_release_ms.value());
        let cut_attack = Self::ms_to_seconds(self.params.filter_cut_attack_ms.value());
        let cut_decay = Self::ms_to_seconds(self.params.filter_cut_decay_ms.value());
        let cut_release = Self::ms_to_seconds(self.params.filter_cut_release_ms.value());
        let res_attack = Self::ms_to_seconds(self.params.filter_res_attack_ms.value());
        let res_decay = Self::ms_to_seconds(self.params.filter_res_decay_ms.value());
        let res_release = Self::ms_to_seconds(self.params.filter_res_release_ms.value());
        (
            ADSREnvelope::new(
                amp_attack,
                self.params.amp_envelope_level.value(),
                amp_decay,
                self.params.amp_sustain_level.value(),
                amp_release,
                sample_rate,
                velocity,
                tension,
            ),
            ADSREnvelope::new(
                cut_attack,
                self.params.filter_cut_envelope_level.value(),
                cut_decay,
                self.params.filter_cut_sustain_ms.value(),
                cut_release,
                sample_rate,
                velocity,
                tension,
            ),
            ADSREnvelope::new(
                res_attack,
                self.params.filter_res_envelope_level.value(),
                res_decay,
                self.params.filter_res_sustain_ms.value(),
                res_release,
                sample_rate,
                velocity,
                tension,
            ),
        )
    }

    fn ms_to_seconds(ms: f32) -> f32 {
        (ms / 1000.0).max(0.0)
    }

    fn select_sf2_zone(&self, note: u8, velocity: f32) -> Option<Arc<Sf2Zone>> {
        let bank = self.sf2_bank.as_ref()?;
        let program = self.current_program.clamp(0, 127) as u8;
        let vel = (velocity.clamp(0.0, 1.0) * 127.0).round() as u8;
        bank.select_zone(program, note, vel)
    }

    fn apply_sf2_env(amp_envelope: &mut ADSREnvelope, env: Sf2Env) {
        amp_envelope.set_attack(env.attack);
        amp_envelope.set_decay(env.decay);
        amp_envelope.set_sustain(env.sustain);
        amp_envelope.set_release(env.release);
    }

    fn start_voice(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
        pan: f32,
        pressure: f32,
        brightness: f32,
        expression: f32,
        vibrato: f32,
        tuning: f32,
        vib_mod: Modulator,
        trem_mod: Modulator,
        amp_envelope: ADSREnvelope,
        filter_cut_envelope: ADSREnvelope,
        filter_res_envelope: ADSREnvelope,
        filter: FilterType,
        sample_rate: f32,
    ) -> &mut Voice {
        // Use the passed envelopes instead of creating new ones
        let new_voice = Voice {
            voice_id: voice_id.unwrap_or_else(|| compute_fallback_voice_id(note, channel)),
            internal_voice_id: self.next_internal_voice_id,
            channel,
            note,
            velocity,
            velocity_sqrt: velocity.sqrt(),
            pan,
            pressure,
            brightness,
            expression,
            vibrato,
            tuning,
            phase: 0.0,
            phase_delta: 0.0,
            releasing: false,
            amp_envelope,
            voice_gain: None,
            filter_cut_envelope,
            filter_res_envelope,
            filter: Some(filter),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, sample_rate),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, sample_rate),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, sample_rate),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, sample_rate),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, sample_rate),
            vib_mod,
            trem_mod,
            modulator_phase: 0.0,
            modulator_phase_delta: 0.0,
            sub_phase: 0.0,
            sub_phase_delta: 0.0,
            sf2_zone: None,
            sf2_phase: 0.0,
            sf2_phase_delta: 0.0,
        };

        self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);

        if let Some(free_voice_idx) = self.voices.iter().position(|voice| voice.is_none()) {
            let voice = &mut self.voices[free_voice_idx];
            if voice.is_none() {
                *voice = Some(new_voice);
                let voice = voice.as_mut().unwrap();
                voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.vib_mod.trigger();
                voice.trem_mod.trigger();
            }
            voice.as_mut().unwrap()
        } else {
            let oldest_voice = self
                .voices
                .iter_mut()
                .min_by_key(|voice| voice.as_ref().unwrap().internal_voice_id)
                .unwrap();
            let oldest_voice = oldest_voice.as_mut().unwrap();
    
            if oldest_voice.amp_envelope.get_state() == ADSREnvelopeState::Idle ||
                oldest_voice.amp_envelope.get_state() == ADSREnvelopeState::Release
            {
                // If the oldest voice's amp envelope is already idle or releasing, no need to send a voice terminated event
                *oldest_voice = new_voice;
                oldest_voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                oldest_voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                oldest_voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                oldest_voice.releasing = false; // Reset the releasing flag
                oldest_voice.vib_mod.trigger();
                oldest_voice.trem_mod.trigger();
            } else {
                context.send_event(NoteEvent::VoiceTerminated {
                    timing: sample_offset,
                    voice_id: Some(oldest_voice.voice_id),
                    channel: oldest_voice.channel,
                    note: oldest_voice.note,
                });
    
                *oldest_voice = new_voice;
            }
    
            oldest_voice
        }
    }

    fn start_release_for_voices(
        &mut self,
        _sample_rate: f32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        for voice in &mut self.voices {
            if let Some(voice) = voice {
                if voice_id == Some(voice.voice_id) || (channel == voice.channel && note == voice.note) {
                    voice.releasing = true;
                    voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                }
            }
        }
    }

    fn _find_voice(&mut self, voice_id: Option<i32>, channel: u8, note: u8) -> Option<&mut Voice> {
        self.voices
            .iter_mut()
            .find(|voice| {
                let voice_id = voice_id.clone(); // Clone the voice_id for comparison inside the closure
                if let Some(voice) = voice {
                    voice.voice_id == voice_id.unwrap_or(voice.voice_id)
                        && voice.channel == channel
                        && voice.note == note
                } else {
                    false
                }
            })
            .map(|voice| voice.as_mut().unwrap())
    }

    fn compute_fallback_voice_id(note: u8, channel: u8, next_voice_id: i32) -> i32 {
        // Fallback voice ID computation...
        // Modify this function to generate a unique voice ID based on note, channel, and next_voice_id.
        // Example implementation:
        (note as i32) + (channel as i32) + next_voice_id
    }

    fn find_or_create_voice(
        &mut self,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        pan: f32,
        pressure:f32,
        brightness: f32,
        expression: f32,
        tuning: f32,
        vibrato: f32,
        amp_envelope: ADSREnvelope,
        filter_cut_envelope: ADSREnvelope,
        filter_res_envelope: ADSREnvelope,
        vib_mod: Modulator,
        trem_mod: Modulator,
    ) -> &mut Voice {
        // Search for an existing voice with the given voice_id
        if let Some(existing_index) = self.voices.iter().position(|voice| {
            voice
                .as_ref()
                .map(|voice_ref| {
                    voice_ref.voice_id == voice_id.unwrap_or(voice_ref.voice_id)
                        && voice_ref.channel == channel
                        && voice_ref.note == note
                })
                .unwrap_or(false)
        }) {
            return self.voices[existing_index].as_mut().unwrap();
        }

        // If no existing voice found, create a new voice
        let new_voice_id = voice_id.unwrap_or_else(|| {
            // Generate a fallback voice ID
            self.next_voice_index += 1;
            Self::compute_fallback_voice_id(
                note,
                channel,
                self.next_voice_index.try_into().unwrap(),
            )
        });

        // If no existing voice found, create a new voice
        let (amp_envelope, filter_cut_envelope, filter_res_envelope) =
            self.construct_envelopes(192000.0, 1.0);
        let mut new_voice = Voice {
            voice_id: new_voice_id,
            channel,
            note,
            internal_voice_id: self.next_internal_voice_id,
            velocity: 0.0,
            velocity_sqrt: 0.0,
            phase: 0.0,
            phase_delta: 0.0,
            releasing: false,
            amp_envelope,
            voice_gain: None,
            filter_cut_envelope,
            filter_res_envelope,
            filter: Some(self.params.filter_type.value()),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, 192000.0),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, 192000.0),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, 192000.0),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, 192000.0),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, 192000.0),
            pan,
            pressure,
            brightness,
            expression,
            tuning,
            vibrato,
            vib_mod,
            trem_mod,
            modulator_phase: 0.0,
            modulator_phase_delta: 0.0,
            sub_phase: 0.0,
            sub_phase_delta: 0.0,
            sf2_zone: None,
            sf2_phase: 0.0,
            sf2_phase_delta: 0.0,
        };
        new_voice.amp_envelope.trigger();
        new_voice.filter_cut_envelope.trigger();
        new_voice.filter_res_envelope.trigger();
        new_voice.vib_mod.trigger();
        new_voice.trem_mod.trigger();
        // Find the next available slot for a new voice
        let mut next_voice_index = self.next_voice_index;
        while self.voices[next_voice_index].is_some() {
            next_voice_index = (next_voice_index + 1) % NUM_VOICES;
            if next_voice_index == self.next_voice_index {
                panic!("No available slots for new voices");
            }
        }

        // Store the new voice in the found slot
        self.voices[next_voice_index] = Some(new_voice);

        // Update the next available slot index
        self.next_voice_index = next_voice_index;

        // Return a mutable reference to the newly created voice
        self.voices[next_voice_index].as_mut().unwrap()

    }

    fn handle_poly_event(
        &mut self,
        _timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        gain: f32,
        pan: f32,
        brightness: f32,
        expression: f32,
        tuning: f32,
        pressure: f32,
        vibrato: f32,
        amp_envelope: Option<&ADSREnvelope>,
        filter_cut_envelope: Option<&ADSREnvelope>,
        filter_res_envelope: Option<&ADSREnvelope>,
        vibrato_modulator: Option<&Modulator>,
        tremolo_modulator: Option<&Modulator>,
    ) {
        let voice = self.find_or_create_voice(
            voice_id,
            channel,
            note,
            pan,
            pressure,
            brightness,
            expression,
            tuning,
            vibrato,
            amp_envelope.cloned().unwrap(),
            filter_cut_envelope.cloned().unwrap(),
            filter_res_envelope.cloned().unwrap(),
            vibrato_modulator.cloned().unwrap(),
            tremolo_modulator.cloned().unwrap(),
        );
        voice.velocity = gain;
        voice.velocity_sqrt = gain.sqrt();
        if let Some(amp_envelope) = amp_envelope {
            voice.amp_envelope = amp_envelope.clone();
            voice.amp_envelope.set_velocity(gain);
        }
    }
    
    

    fn choke_voices(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        sample_offset: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
    ) {
        for voice in self.voices.iter_mut() {
            match voice {
                Some(Voice {
                    voice_id: candidate_voice_id,
                    channel: candidate_channel,
                    note: candidate_note,
                    ..
                }) if voice_id == Some(*candidate_voice_id)
                    || (channel == *candidate_channel && note == *candidate_note) =>
                {
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: sample_offset,
                        voice_id: Some(*candidate_voice_id),
                        channel,
                        note,
                    });
                    *voice = None;

                    if voice_id.is_some() {
                        return;
                    }
                }
                _ => (),
            }
        }
    }
    pub fn clip(input: f32, limit: f32) -> f32 {
        if input > limit {
            limit
        } else if input < -limit {
            -limit
        } else {
            input
        }
    }
    pub fn poly_blep(t: f32, dt: f32) -> f32 {
        if t < dt {
            let t = t / dt;
            // 2 * (t - t^2/2 - 0.5)
            return t + t - t * t - 1.0;
        } else if t > 1.0 - dt {
            let t = (t - 1.0) / dt;
            // 2 * (t^2/2 + t + 0.5)
            return t * t + t + t + 1.0;
        }
        0.0
    }
}

const fn compute_fallback_voice_id(note: u8, channel: u8) -> i32 {
    note as i32 | ((channel as i32) << 16)
}

impl ClapPlugin for FishSynth {
    const CLAP_ID: &'static str = "art.taellinglin.fishsynth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A Polyphonic Subtractive Synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];

    const CLAP_POLY_MODULATION_CONFIG: Option<PolyModulationConfig> = Some(PolyModulationConfig {
        max_voice_capacity: NUM_VOICES as u32,
        supports_overlapping_voices: true,
    });
}

impl Vst3Plugin for FishSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"FishSynthLing01!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
    const VST3_PROGRAM_LIST_NAME: &'static str = "FishSynth GM";
    const VST3_PROGRAM_NAMES: &'static [&'static str] = &presets::GM_PRESETS;
}

nih_export_clap!(FishSynth);
nih_export_vst3!(FishSynth);
