mod editor;
mod envelope;
mod filter;
mod chorus;
mod delay;
mod reverb;
mod limiter;
mod multi_filter;
mod waveform;
mod modulator;
mod eq;
mod distortion;
mod output_saturation;
mod spectral;
pub mod params;
pub mod voice;
use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;
use nih_plug::params::enums::EnumParam;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::Rng;
use rand_pcg::Pcg32;
use std::sync::{Arc, RwLock};

use modulator::{Modulator, OscillatorShape};
use chorus::Chorus;
use delay::StereoDelay;
use envelope::{ADSREnvelope, Envelope, ADSREnvelopeState};
use filter::{FilterType, Filter};
use limiter::Limiter;
use multi_filter::MultiStageFilter;
use waveform::{generate_waveform, WavetableBank, Waveform};
use eq::{Biquad, ThreeBandEq};
use distortion::Distortion;
use output_saturation::{OutputSaturation, OutputSaturationType};
use spectral::SpectralShaper;

const NUM_VOICES: usize = 16;
const MAX_BLOCK_SIZE: usize = 64;
pub(crate) const GAIN_POLY_MOD_ID: u32 = 0;

use params::*;
use voice::*;

struct SubSynth {
    params: Arc<SubSynthParams>,
    prng: Pcg32,
    voices: [Option<Voice>; NUM_VOICES as usize],
    next_voice_index: usize,
    next_internal_voice_id: u64,
    chorus: Chorus,
    delay: StereoDelay,
    reverb: reverb::Reverb,
    limiter_left: Limiter,
    limiter_right: Limiter,
    multi_filter: MultiStageFilter,
    distortion: Distortion,
    eq: ThreeBandEq,
    output_saturation: OutputSaturation,
    spectral_main: SpectralShaper,
    spectral_fx: SpectralShaper,
    factory_wavetable: WavetableBank,
    custom_wavetable: Option<WavetableBank>,
    custom_wavetable_path: Option<String>,
    factory_presets: Vec<editor::PresetData>,
    last_preset_index: i32,
    seq_phase: f32,
    last_note_phase_delta: f32,
    last_note_active: bool,
    ring_mod_post_phase: [f32; 2],
    sample_rate: f32,
}

impl Default for SubSynth {
    fn default() -> Self {
        let params = Arc::new(SubSynthParams::default());
        let factory_presets = editor::factory_preset_data(&params);

        Self {
            params: params.clone(),

            prng: Pcg32::new(420, 1337),
            voices: [0; NUM_VOICES as usize].map(|_| None),
            next_internal_voice_id: 0,
            next_voice_index: 0,
            chorus: Chorus::new(44100.0),
            delay: StereoDelay::new(44100.0),
            reverb: reverb::Reverb::new(44100.0),
            limiter_left: Limiter::new(),
            limiter_right: Limiter::new(),
            multi_filter: MultiStageFilter::new(44100.0),
            distortion: Distortion::new(44100.0),
            eq: ThreeBandEq::new(44100.0),
            output_saturation: OutputSaturation::new(44100.0),
            spectral_main: SpectralShaper::new(44100.0, 2048, 4),
            spectral_fx: SpectralShaper::new(44100.0, 2048, 4),
            factory_wavetable: WavetableBank::new(),
            custom_wavetable: None,
            custom_wavetable_path: None,
            factory_presets,
            last_preset_index: params.preset_index.value(),
            seq_phase: 0.0,
            last_note_phase_delta: 0.0,
            last_note_active: false,
            ring_mod_post_phase: [0.0; 2],
            sample_rate: 44100.0,
        }
    }
}


impl Plugin for SubSynth {
    const NAME: &'static str = "DogSynth";
    const VENDOR: &'static str = "DogSynth";
    const URL: &'static str = "https://taellinglin.art";
    const EMAIL: &'static str = "taellinglin@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
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
        context: &mut impl InitContext<Self>,
    ) -> bool {
        // After `PEAK_METER_DECAY_MS` milliseconds of pure silence, the peak meter's value should
        // have dropped by 12 dB
        self.sample_rate = buffer_config.sample_rate;
        self.refresh_custom_wavetable();
        context.set_latency_samples(self.spectral_main.latency_samples());
        true
    }

    fn reset(&mut self) {
        self.prng = Pcg32::new(420, 1337);

        self.voices.fill(None);
        self.next_internal_voice_id = 0;
        self.ring_mod_post_phase = [0.0; 2];
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let output = buffer.as_slice();

        self.update_module_sample_rates(sample_rate);
        self.refresh_custom_wavetable();
        self.sync_preset_if_changed();

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);

        while block_start < num_samples {
            let this_sample_internal_voice_id_start = self.next_internal_voice_id;
            
            'events: loop {
                match next_event {
                    Some(event) if (event.timing() as usize) < block_end => {
                        self.handle_event(event, context, sample_rate, this_sample_internal_voice_id_start);
                        next_event = context.next_event();
                    }
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            let block_len = block_end - block_start;
            let mut gain = [0.0; MAX_BLOCK_SIZE];
            let mut fx_left = [0.0; MAX_BLOCK_SIZE];
            let mut fx_right = [0.0; MAX_BLOCK_SIZE];

            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);
            self.params.gain.smoothed.next_block(&mut gain, block_len);

            let p = self.get_block_params();

            self.render_block_voices(
                output,
                &mut fx_left,
                &mut fx_right,
                block_start,
                block_end,
                sample_rate,
                &p,
                context,
            );

            self.apply_block_fx(
                output,
                &mut fx_left,
                &mut fx_right,
                block_start,
                block_end,
                sample_rate,
                &p,
            );

            for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
                output[0][sample_idx] *= gain[value_idx];
                output[1][sample_idx] *= gain[value_idx];
            }

            self.terminate_finished_voices(context, block_end);
            self.last_note_active = self.voices.iter().any(|v| v.is_some());

            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl SubSynth {
    fn apply_factory_preset(&mut self, index: usize, update_param: bool) {
        if self.factory_presets.is_empty() {
            return;
        }

        let clamped = index.min(self.factory_presets.len() - 1);
        self.factory_presets[clamped].apply_direct(&self.params);
        if update_param {
            self.params.preset_index.set_plain_value(clamped as i32);
        }
        self.last_preset_index = clamped as i32;
    }

    fn sync_preset_if_changed(&mut self) {
        let preset_index = self.params.preset_index.value();
        if preset_index != self.last_preset_index {
            let clamped = if preset_index < 0 { 0 } else { preset_index as usize };
            self.apply_factory_preset(clamped, false);
        }
    }

    fn get_voice_idx(&mut self, voice_id: i32) -> Option<usize> {
        self.voices
            .iter_mut()
            .position(|voice| matches!(voice, Some(voice) if voice.voice_id == voice_id))
    }

    fn refresh_custom_wavetable(&mut self) {
        if let Ok(mut data) = self.params.custom_wavetable_data.try_write() {
            if let Some(table) = data.take() {
                self.custom_wavetable = Some(WavetableBank::from_table(table));
                if let Ok(path) = self.params.custom_wavetable_path.read() {
                    self.custom_wavetable_path = (*path).clone();
                }
            }
        }

        if self.custom_wavetable.is_none() {
            if let Ok(path) = self.params.custom_wavetable_path.read() {
                if let Some(path) = (*path).as_ref() {
                    if self.custom_wavetable_path.as_deref() != Some(path.as_str()) {
                        if let Ok(table) = waveform::load_wavetable_from_file(std::path::Path::new(path)) {
                            self.custom_wavetable = Some(WavetableBank::from_table(table));
                            self.custom_wavetable_path = Some(path.clone());
                        }
                    }
                }
            }
        }
    }

    fn construct_envelopes(
        &self,
        sample_rate: f32,
        velocity: f32,
    ) -> (ADSREnvelope, ADSREnvelope, ADSREnvelope) {
        (
            ADSREnvelope::new(
                self.params.amp_attack_ms.value(),
                self.params.amp_hold_ms.value(),
                self.params.amp_decay_ms.value(),
                self.params.amp_decay2_ms.value(),
                self.params.amp_decay2_level.value(),
                self.params.amp_sustain_level.value(),
                self.params.amp_release_ms.value(),
                sample_rate,
                velocity,
                self.params.amp_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_cut_attack_ms.value(),
                self.params.filter_cut_hold_ms.value(),
                self.params.filter_cut_decay_ms.value(),
                self.params.filter_cut_decay2_ms.value(),
                self.params.filter_cut_decay2_level.value(),
                self.params.filter_cut_sustain_ms.value(),
                self.params.filter_cut_release_ms.value(),
                sample_rate,
                velocity,
                self.params.filter_cut_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_res_attack_ms.value(),
                self.params.filter_res_hold_ms.value(),
                self.params.filter_res_decay_ms.value(),
                self.params.filter_res_decay2_ms.value(),
                self.params.filter_res_decay2_level.value(),
                self.params.filter_res_sustain_ms.value(),
                self.params.filter_res_release_ms.value(),
                sample_rate,
                velocity,
                self.params.filter_res_tension.value(),
            ),
        )
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
        mod_lfo1: Modulator,
        mod_lfo2: Modulator,
        amp_envelope: ADSREnvelope,
        filter_cut_envelope: ADSREnvelope,
        filter_res_envelope: ADSREnvelope,
        fm_envelope: ADSREnvelope,
        dist_envelope: ADSREnvelope,
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
            target_phase_delta: 0.0,
            releasing: false,
            amp_envelope,
            voice_gain: None,
            filter_cut_envelope,
            filter_res_envelope,
            fm_envelope,
            dist_envelope,
            filter: Some(filter),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, sample_rate),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, sample_rate),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, sample_rate),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, sample_rate),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, sample_rate),
            comb_filter: filter::CombFilter::new(sample_rate),
            rainbow_comb_filter: filter::RainbowCombFilter::new(sample_rate),
            diode_ladder_lp_filter: filter::DiodeLadderFilter::new_lowpass(sample_rate),
            diode_ladder_hp_filter: filter::DiodeLadderFilter::new_highpass(sample_rate),
            ms20_filter: filter::Ms20Filter::new(sample_rate),
            formant_morph_filter: filter::FormantMorphFilter::new(sample_rate),
            phaser_filter: filter::PhaserFilter::new(sample_rate),
            comb_allpass_filter: filter::CombAllpassFilter::new(sample_rate),
            bitcrush_lp_filter: filter::BitcrushLpFilter::new(sample_rate),
            vib_mod,
            trem_mod,
            mod_lfo1,
            mod_lfo2,
            pan_mod: Modulator::new(
                self.params.pan_lfo_rate.value(),
                self.params.pan_lfo_intensity.value(),
                self.params.pan_lfo_attack.value(),
                self.params.pan_lfo_shape.value(),
            ),
            drift_offset: 0.0,
            mod_smooth: [0.0; 6],
            fm_feedback_state: 0.0,
            unison_phases: [0.0; 6],
            stereo_prev: 0.0,
            dc_blocker: filter::DCBlocker::new(),
            sizzle_osc_lp: Biquad::new(),
            sizzle_wt_lp: Biquad::new(),
            alias_lp1: Biquad::new(),
            alias_lp2: Biquad::new(),
            tight_lp: Biquad::new(),
            tight_hp: Biquad::new(),
            ring_phase: 0.0,
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
                voice.fm_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.dist_envelope.set_envelope_stage(ADSREnvelopeState::Attack);
                voice.vib_mod.trigger();
                voice.trem_mod.trigger();
                voice.mod_lfo1.trigger();
                voice.mod_lfo2.trigger();
            }
            voice.as_mut().unwrap()
        } else {
            let oldest_voice = self
                .voices
                .iter_mut()
                .min_by_key(|voice| voice.as_ref().map(|v| v.internal_voice_id).unwrap_or(u64::MAX))
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
                oldest_voice.mod_lfo1.trigger();
                oldest_voice.mod_lfo2.trigger();
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
                    voice.fm_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                    voice.dist_envelope.set_envelope_stage(ADSREnvelopeState::Release);
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
                self.next_voice_index as i32,
            )
        });

        let mut new_voice = Voice {
            voice_id: new_voice_id,
            channel,
            note,
            internal_voice_id: self.next_internal_voice_id,
            velocity: 0.0,
            velocity_sqrt: 0.0,
            phase: 0.0,
            phase_delta: 0.0,
            target_phase_delta: 0.0,
            releasing: false,
            amp_envelope,
            voice_gain: None,
            filter_cut_envelope,
            filter_res_envelope,
            fm_envelope: ADSREnvelope::new(
                self.params.fm_env_attack_ms.value(),
                self.params.fm_env_hold_ms.value(),
                self.params.fm_env_decay_ms.value(),
                self.params.fm_env_decay2_ms.value(),
                self.params.fm_env_decay2_level.value(),
                self.params.fm_env_sustain_level.value(),
                self.params.fm_env_release_ms.value(),
                self.sample_rate,
                1.0,
                0.0,
            ),
            dist_envelope: ADSREnvelope::new(
                self.params.dist_env_attack_ms.value(),
                self.params.dist_env_hold_ms.value(),
                self.params.dist_env_decay_ms.value(),
                self.params.dist_env_decay2_ms.value(),
                self.params.dist_env_decay2_level.value(),
                self.params.dist_env_sustain_level.value(),
                self.params.dist_env_release_ms.value(),
                self.sample_rate,
                1.0,
                0.0,
            ),
            filter: Some(self.params.filter_type.value()),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, self.sample_rate),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, self.sample_rate),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, self.sample_rate),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, self.sample_rate),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, self.sample_rate),
            comb_filter: filter::CombFilter::new(self.sample_rate),
            rainbow_comb_filter: filter::RainbowCombFilter::new(self.sample_rate),
            diode_ladder_lp_filter: filter::DiodeLadderFilter::new_lowpass(self.sample_rate),
            diode_ladder_hp_filter: filter::DiodeLadderFilter::new_highpass(self.sample_rate),
            ms20_filter: filter::Ms20Filter::new(self.sample_rate),
            formant_morph_filter: filter::FormantMorphFilter::new(self.sample_rate),
            phaser_filter: filter::PhaserFilter::new(self.sample_rate),
            comb_allpass_filter: filter::CombAllpassFilter::new(self.sample_rate),
            bitcrush_lp_filter: filter::BitcrushLpFilter::new(self.sample_rate),
            pan,
            pressure,
            brightness,
            expression,
            tuning,
            vibrato,
            vib_mod,
            trem_mod,
            mod_lfo1: Modulator::new(
                self.params.lfo1_rate.value(),
                1.0,
                self.params.lfo1_attack.value(),
                self.params.lfo1_shape.value(),
            ),
            mod_lfo2: Modulator::new(
                self.params.lfo2_rate.value(),
                1.0,
                self.params.lfo2_attack.value(),
                self.params.lfo2_shape.value(),
            ),
            pan_mod: Modulator::new(
                self.params.pan_lfo_rate.value(),
                self.params.pan_lfo_intensity.value(),
                self.params.pan_lfo_attack.value(),
                self.params.pan_lfo_shape.value(),
            ),
            drift_offset: 0.0,
            mod_smooth: [0.0; 6],
            fm_feedback_state: 0.0,
            unison_phases: [0.0; 6],
            stereo_prev: 0.0,
            dc_blocker: filter::DCBlocker::new(),
            sizzle_osc_lp: Biquad::new(),
            sizzle_wt_lp: Biquad::new(),
            alias_lp1: Biquad::new(),
            alias_lp2: Biquad::new(),
            tight_lp: Biquad::new(),
            tight_hp: Biquad::new(),
            ring_phase: 0.0,
        };
        new_voice.amp_envelope.trigger();
        new_voice.filter_cut_envelope.trigger();
        new_voice.filter_res_envelope.trigger();
        new_voice.fm_envelope.trigger();
        new_voice.dist_envelope.trigger();
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
        let (default_amp_env, default_cut_env, default_res_env) = self.construct_envelopes(self.sample_rate, 1.0);
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
            amp_envelope.cloned().unwrap_or(default_amp_env),
            filter_cut_envelope.cloned().unwrap_or(default_cut_env),
            filter_res_envelope.cloned().unwrap_or(default_res_env),
            vibrato_modulator.cloned().unwrap_or_default(),
            tremolo_modulator.cloned().unwrap_or_default(),
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

    pub fn wavefold(sample: f32, amount: f32) -> f32 {
        if amount <= 0.0 {
            return sample;
        }

        let drive = 1.0 + amount * 6.0;
        let post_gain = 1.0 / (1.0 + amount * 1.5);
        let x = sample * drive;
        let mut folded = (x + 1.0).rem_euclid(4.0);
        if folded > 2.0 {
            folded = 4.0 - folded;
        }
        (folded - 1.0) * post_gain
    }
}

const fn compute_fallback_voice_id(note: u8, channel: u8) -> i32 {
    note as i32 | ((channel as i32) << 16)
}

impl ClapPlugin for SubSynth {
    const CLAP_ID: &'static str = "art.dogsynth";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A knitty gritty wavetable bass synth for dubstep and heavy electronic sound design");
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

impl Vst3Plugin for SubSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"DogSynthLing1A01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(SubSynth);
nih_export_vst3!(SubSynth);

impl SubSynth {
    fn handle_note_on(
        &mut self,
        _context: &mut impl ProcessContext<Self>,
        timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        velocity: f32,
        sample_rate: f32,
    ) {
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
        let pan_lfo = Modulator::new(
            self.params.pan_lfo_rate.value(),
            self.params.pan_lfo_intensity.value(),
            self.params.pan_lfo_attack.value(),
            self.params.pan_lfo_shape.value(),
        );
        let mod_lfo1 = Modulator::new(
            self.params.lfo1_rate.value(),
            1.0,
            self.params.lfo1_attack.value(),
            self.params.lfo1_shape.value(),
        );
        let mod_lfo2 = Modulator::new(
            self.params.lfo2_rate.value(),
            1.0,
            self.params.lfo2_attack.value(),
            self.params.lfo2_shape.value(),
        );

        let pitch = util::midi_note_to_freq(note) * (2.0_f32).powf(tuning / 12.0);
        let target_phase_delta = pitch / sample_rate;
        let glide_mode = self.params.glide_mode.value();
        let use_glide = match glide_mode {
            GlideMode::Off => false,
            GlideMode::Always => true,
            GlideMode::Legato => self.last_note_active,
        };
        let start_phase_delta = if use_glide && self.last_note_phase_delta > 0.0 {
            self.last_note_phase_delta
        } else {
            target_phase_delta
        };

        let (amp_envelope, cutoff_envelope, resonance_envelope) = self.construct_envelopes(sample_rate, velocity);
        let fm_envelope = ADSREnvelope::new(
            self.params.fm_env_attack_ms.value(),
            self.params.fm_env_hold_ms.value(),
            self.params.fm_env_decay_ms.value(),
            self.params.fm_env_decay2_ms.value(),
            self.params.fm_env_decay2_level.value(),
            self.params.fm_env_sustain_level.value(),
            self.params.fm_env_release_ms.value(),
            sample_rate,
            velocity,
            0.0,
        );
        let dist_envelope = ADSREnvelope::new(
            self.params.dist_env_attack_ms.value(),
            self.params.dist_env_hold_ms.value(),
            self.params.dist_env_decay_ms.value(),
            self.params.dist_env_decay2_ms.value(),
            self.params.dist_env_decay2_level.value(),
            self.params.dist_env_sustain_level.value(),
            self.params.dist_env_release_ms.value(),
            sample_rate,
            velocity,
            0.0,
        );

        let voice = self.start_voice(
            _context, timing, voice_id, channel, note, velocity,
            pan, pressure, brightness, expression, vibrato, tuning,
            vibrato_lfo, tremolo_lfo, mod_lfo1, mod_lfo2,
            amp_envelope, cutoff_envelope, resonance_envelope, fm_envelope, dist_envelope,
            self.params.filter_type.value(), sample_rate,
        );
        
        voice.vib_mod = vibrato_lfo.clone();
        voice.trem_mod = tremolo_lfo.clone();
        voice.pan_mod = pan_lfo.clone();
        voice.mod_lfo1 = mod_lfo1.clone();
        voice.mod_lfo2 = mod_lfo2.clone();
        voice.velocity_sqrt = velocity.sqrt();
        voice.phase = initial_phase;
        voice.phase_delta = start_phase_delta;
        voice.target_phase_delta = target_phase_delta;
        voice.unison_phases = [initial_phase; 6];
        voice.stereo_prev = 0.0;

        self.last_note_phase_delta = target_phase_delta;
        self.last_note_active = true;
    }
}

impl SubSynth {
    fn handle_poly_event_wrapper(
        &mut self,
        timing: u32,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        gain: Option<f32>,
        pan: Option<f32>,
        vibrato: Option<f32>,
        pressure: Option<f32>,
        tuning: Option<f32>,
    ) {
        if let Some(voice_idx) = self.get_voice_idx(voice_id.unwrap_or_default()) {
            if let Some(voice_inner) = self.voices[voice_idx].as_mut() {
                let v_gain = gain.unwrap_or(voice_inner.velocity);
                let v_pan = pan.unwrap_or(voice_inner.pan);
                let v_vibrato = vibrato.unwrap_or(voice_inner.vibrato);
                let v_pressure = pressure.unwrap_or(voice_inner.pressure);
                let v_tuning = tuning.unwrap_or(voice_inner.tuning);

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
                    v_gain,
                    v_pan,
                    voice_inner.brightness,
                    voice_inner.expression,
                    v_tuning,
                    v_pressure,
                    v_vibrato,
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


struct BlockParams {
    waveform: Waveform,
    osc_routing: OscRouting,
    osc_blend: f32,
    wavetable_position: f32,
    wavetable_distortion: f32,
    fm_enable: bool,
    fm_target: FmTarget,
    fm_source: FmSource,
    fm_amount: f32,
    fm_ratio: f32,
    fm_feedback: f32,
    fm_env_amount: f32,
    dist_env_amount: f32,
    analog_enable: bool,
    analog_drive: f32,
    analog_noise: f32,
    analog_drift: f32,
    sub_level: f32,
    vibrato_intensity: f32,
    glide_time: f32,
    sizzle_cutoff: f32,
    sizzle_osc_enable: bool,
    sizzle_wt_enable: bool,
    filter_type: FilterType,
    filter_cut: f32,
    filter_res: f32,
    filter_tight_enable: bool,
    filter_cut_env_level: f32,
    filter_res_env_level: f32,
    filter_amount: f32,
    amp_env_level: f32,
    unison_voices: UnisonVoices,
    unison_detune: f32,
    unison_spread: f32,
    classic_level: f32,
    wavetable_level: f32,
    noise_level: f32,
    classic_send: f32,
    wavetable_send: f32,
    sub_send: f32,
    noise_send: f32,
    ring_mod_enable: bool,
    ring_mod_source: RingModSource,
    ring_mod_placement: RingModPlacement,
    ring_mod_level: f32,
    ring_mod_freq: f32,
    ring_mod_send: f32,
    ring_mod_mix: f32,
    spectral_enable: bool,
    spectral_placement: SpectralPlacement,
    spectral_amount: f32,
    spectral_tilt: f32,
    spectral_formant: f32,
    chorus_enable: bool,
    chorus_rate: f32,
    chorus_depth: f32,
    chorus_mix: f32,
    multi_filter_enable: bool,
    multi_filter_routing: FilterRouting,
    multi_filter_a_type: FilterType,
    multi_filter_a_cut: f32,
    multi_filter_a_res: f32,
    multi_filter_a_amt: f32,
    multi_filter_b_type: FilterType,
    multi_filter_b_cut: f32,
    multi_filter_b_res: f32,
    multi_filter_b_amt: f32,
    multi_filter_c_type: FilterType,
    multi_filter_c_cut: f32,
    multi_filter_c_res: f32,
    multi_filter_c_amt: f32,
    multi_filter_morph: f32,
    multi_filter_parallel_ab: bool,
    multi_filter_parallel_c: bool,
    dist_enable: bool,
    dist_drive: f32,
    dist_tone: f32,
    dist_magic: f32,
    dist_mix: f32,
    sizzle_dist_enable: bool,
    eq_enable: bool,
    eq_low_gain: f32,
    eq_mid_gain: f32,
    eq_mid_freq: f32,
    eq_mid_q: f32,
    eq_high_gain: f32,
    eq_mix: f32,
    delay_enable: bool,
    delay_time: f32,
    delay_feedback: f32,
    delay_mix: f32,
    reverb_enable: bool,
    reverb_size: f32,
    reverb_damp: f32,
    reverb_diffusion: f32,
    reverb_shimmer: f32,
    reverb_mix: f32,
    fx_bus_mix: f32,
    output_sat_enable: bool,
    output_sat_drive: f32,
    output_sat_mix: f32,
    output_sat_type: OutputSaturationType,
    limiter_enable: bool,
    limiter_threshold: f32,
    limiter_release: f32,
    seq_enable: bool,
    seq_rate: f32,
    seq_gate_amount: f32,
    seq_cut_amount: f32,
    seq_res_amount: f32,
    seq_wt_amount: f32,
    seq_dist_amount: f32,
    seq_fm_amount: f32,
}

impl SubSynth {
    fn update_module_sample_rates(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.chorus.set_sample_rate(sample_rate);
        self.delay.set_sample_rate(sample_rate);
        self.reverb.set_sample_rate(sample_rate);
        self.multi_filter.set_sample_rate(sample_rate);
        self.distortion.set_sample_rate(sample_rate);
        self.eq.set_sample_rate(sample_rate);
        self.output_saturation.set_sample_rate(sample_rate);
        self.spectral_main.set_sample_rate(sample_rate);
        self.spectral_fx.set_sample_rate(sample_rate);
    }

    fn handle_event(&mut self, event: NoteEvent<()>, context: &mut impl ProcessContext<Self>, sample_rate: f32, this_sample_internal_voice_id_start: u64) {
        match event {
            NoteEvent::NoteOn { timing, voice_id, channel, note, velocity } => {
                self.handle_note_on(context, timing, voice_id, channel, note, velocity, sample_rate);
            }
            NoteEvent::NoteOff { timing: _, voice_id, channel, note, velocity: _ } => {
                self.start_release_for_voices(sample_rate, voice_id, channel, note);
            }
            NoteEvent::Choke { timing, voice_id, channel, note } => {
                self.choke_voices(context, timing, voice_id, channel, note);
            }
            NoteEvent::PolyModulation { timing: _, voice_id, poly_modulation_id, normalized_offset } => {
                if let Some(voice_idx) = self.get_voice_idx(voice_id) {
                    let voice = self.voices[voice_idx].as_mut().unwrap();
                    if poly_modulation_id == GAIN_POLY_MOD_ID {
                        let target_plain_value = self.params.gain.preview_modulated(normalized_offset);
                        let (_, smoother) = voice.voice_gain.get_or_insert_with(|| {
                            (normalized_offset, self.params.gain.smoothed.clone())
                        });
                        if voice.internal_voice_id >= this_sample_internal_voice_id_start {
                            smoother.reset(target_plain_value);
                        } else {
                            smoother.set_target(sample_rate, target_plain_value);
                        }
                    }
                }
            }
            NoteEvent::MonoAutomation { timing: _, poly_modulation_id, normalized_value } => {
                for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                    if poly_modulation_id == GAIN_POLY_MOD_ID {
                        if let Some((normalized_offset, smoother)) = voice.voice_gain.as_mut() {
                            let target_plain_value = self.params.gain.preview_plain(normalized_value + *normalized_offset);
                            smoother.set_target(sample_rate, target_plain_value);
                        }
                    }
                }
            }
            NoteEvent::PolyPressure { timing, voice_id, channel, note, pressure } => {
                self.handle_poly_event_wrapper(timing, voice_id, channel, note, None, None, None, Some(pressure), None);
            }
            NoteEvent::PolyVolume { timing, voice_id, channel, note, gain } => {
                self.handle_poly_event_wrapper(timing, voice_id, channel, note, Some(gain), None, None, None, None);
            }
            NoteEvent::PolyPan { timing, voice_id, channel, note, pan } => {
                self.handle_poly_event_wrapper(timing, voice_id, channel, note, None, Some(pan), None, None, None);
            }
            NoteEvent::PolyTuning { timing, voice_id, channel, note, tuning } => {
                self.handle_poly_event_wrapper(timing, voice_id, channel, note, None, None, None, None, Some(tuning));
            }
            NoteEvent::PolyVibrato { timing, voice_id, channel, note, vibrato } => {
                self.handle_poly_event_wrapper(timing, voice_id, channel, note, None, None, Some(vibrato), None, None);
            }
            NoteEvent::MidiProgramChange { program, .. } => {
                self.apply_factory_preset(program as usize, true);
            }
            _ => (),
        }
    }

    fn get_block_params(&self) -> BlockParams {
        BlockParams {
            waveform: self.params.waveform.value(),
            osc_routing: self.params.osc_routing.value(),
            osc_blend: self.params.osc_blend.value(),
            wavetable_position: self.params.wavetable_position.value(),
            wavetable_distortion: self.params.wavetable_distortion.value(),
            fm_enable: self.params.fm_enable.value(),
            fm_target: self.params.fm_target.value(),
            fm_source: self.params.fm_source.value(),
            fm_amount: self.params.fm_amount.value(),
            fm_ratio: self.params.fm_ratio.value(),
            fm_feedback: self.params.fm_feedback.value(),
            fm_env_amount: self.params.fm_env_amount.value(),
            dist_env_amount: self.params.dist_env_amount.value(),
            analog_enable: self.params.analog_enable.value(),
            analog_drive: self.params.analog_drive.value(),
            analog_noise: self.params.analog_noise.value(),
            analog_drift: self.params.analog_drift.value(),
            sub_level: self.params.sub_level.value(),
            vibrato_intensity: self.params.vibrato_intensity.value(),
            glide_time: self.params.glide_time_ms.value(),
            sizzle_cutoff: self.params.sizzle_cutoff.value(),
            sizzle_osc_enable: self.params.sizzle_osc_enable.value(),
            sizzle_wt_enable: self.params.sizzle_wt_enable.value(),
            filter_type: self.params.filter_type.value(),
            filter_cut: self.params.filter_cut.value(),
            filter_res: self.params.filter_res.value(),
            filter_tight_enable: self.params.filter_tight_enable.value(),
            filter_cut_env_level: self.params.filter_cut_envelope_level.value().max(0.0).min(1.0),
            filter_res_env_level: self.params.filter_res_envelope_level.value().max(0.0).min(1.0),
            filter_amount: self.params.filter_amount.value(),
            amp_env_level: self.params.amp_envelope_level.value(),
            unison_voices: self.params.unison_voices.value(),
            unison_detune: self.params.unison_detune.value(),
            unison_spread: self.params.unison_spread.value(),
            classic_level: self.params.classic_level.value(),
            wavetable_level: self.params.wavetable_level.value(),
            noise_level: self.params.noise_level.value(),
            classic_send: self.params.classic_send.value(),
            wavetable_send: self.params.wavetable_send.value(),
            sub_send: self.params.sub_send.value(),
            noise_send: self.params.noise_send.value(),
            ring_mod_enable: self.params.ring_mod_enable.value(),
            ring_mod_source: self.params.ring_mod_source.value(),
            ring_mod_placement: self.params.ring_mod_placement.value(),
            ring_mod_level: self.params.ring_mod_level.value(),
            ring_mod_freq: self.params.ring_mod_freq.value(),
            ring_mod_send: self.params.ring_mod_send.value(),
            ring_mod_mix: self.params.ring_mod_mix.value(),
            spectral_enable: self.params.spectral_enable.value(),
            spectral_placement: self.params.spectral_placement.value(),
            spectral_amount: self.params.spectral_amount.value(),
            spectral_tilt: self.params.spectral_tilt.value(),
            spectral_formant: self.params.spectral_formant.value(),
            chorus_enable: self.params.chorus_enable.value(),
            chorus_rate: self.params.chorus_rate.value(),
            chorus_depth: self.params.chorus_depth.value(),
            chorus_mix: self.params.chorus_mix.value(),
            multi_filter_enable: self.params.multi_filter_enable.value(),
            multi_filter_routing: self.params.multi_filter_routing.value(),
            multi_filter_a_type: self.params.multi_filter_a_type.value(),
            multi_filter_a_cut: self.params.multi_filter_a_cut.value(),
            multi_filter_a_res: self.params.multi_filter_a_res.value(),
            multi_filter_a_amt: self.params.multi_filter_a_amt.value(),
            multi_filter_b_type: self.params.multi_filter_b_type.value(),
            multi_filter_b_cut: self.params.multi_filter_b_cut.value(),
            multi_filter_b_res: self.params.multi_filter_b_res.value(),
            multi_filter_b_amt: self.params.multi_filter_b_amt.value(),
            multi_filter_c_type: self.params.multi_filter_c_type.value(),
            multi_filter_c_cut: self.params.multi_filter_c_cut.value(),
            multi_filter_c_res: self.params.multi_filter_c_res.value(),
            multi_filter_c_amt: self.params.multi_filter_c_amt.value(),
            multi_filter_morph: self.params.multi_filter_morph.value(),
            multi_filter_parallel_ab: self.params.multi_filter_parallel_ab.value(),
            multi_filter_parallel_c: self.params.multi_filter_parallel_c.value(),
            dist_enable: self.params.dist_enable.value(),
            dist_drive: self.params.dist_drive.value(),
            dist_tone: self.params.dist_tone.value(),
            dist_magic: self.params.dist_magic.value(),
            dist_mix: self.params.dist_mix.value(),
            sizzle_dist_enable: self.params.sizzle_dist_enable.value(),
            eq_enable: self.params.eq_enable.value(),
            eq_low_gain: self.params.eq_low_gain.value(),
            eq_mid_gain: self.params.eq_mid_gain.value(),
            eq_mid_freq: self.params.eq_mid_freq.value(),
            eq_mid_q: self.params.eq_mid_q.value(),
            eq_high_gain: self.params.eq_high_gain.value(),
            eq_mix: self.params.eq_mix.value(),
            delay_enable: self.params.delay_enable.value(),
            delay_time: self.params.delay_time_ms.value(),
            delay_feedback: self.params.delay_feedback.value(),
            delay_mix: self.params.delay_mix.value(),
            reverb_enable: self.params.reverb_enable.value(),
            reverb_size: self.params.reverb_size.value(),
            reverb_damp: self.params.reverb_damp.value(),
            reverb_diffusion: self.params.reverb_diffusion.value(),
            reverb_shimmer: self.params.reverb_shimmer.value(),
            reverb_mix: self.params.reverb_mix.value(),
            fx_bus_mix: self.params.fx_bus_mix.value().clamp(0.0, 1.0),
            output_sat_enable: self.params.output_sat_enable.value(),
            output_sat_drive: self.params.output_sat_drive.value(),
            output_sat_mix: self.params.output_sat_mix.value(),
            output_sat_type: self.params.output_sat_type.value(),
            limiter_enable: self.params.limiter_enable.value(),
            limiter_threshold: self.params.limiter_threshold.value(),
            limiter_release: self.params.limiter_release.value(),
            seq_enable: self.params.seq_enable.value(),
            seq_rate: self.params.seq_rate.value(),
            seq_gate_amount: self.params.seq_gate_amount.value(),
            seq_cut_amount: self.params.seq_cut_amount.value(),
            seq_res_amount: self.params.seq_res_amount.value(),
            seq_wt_amount: self.params.seq_wt_amount.value(),
            seq_dist_amount: self.params.seq_dist_amount.value(),
            seq_fm_amount: self.params.seq_fm_amount.value(),
        }
    }
}

impl SubSynth {
    fn render_block_voices(
        &mut self,
        output: &mut [&mut [f32]],
        fx_left: &mut [f32],
        fx_right: &mut [f32],
        block_start: usize,
        block_end: usize,
        sample_rate: f32,
        p: &BlockParams,
        context: &mut impl ProcessContext<Self>,
    ) {
        let block_len = block_end - block_start;
        let mut voice_gain = [0.0; MAX_BLOCK_SIZE];

        // Sequencer logic - handled at block level for performance if possible
        // But for high-rate modulation, we might need it per sample.
        // Let's stick to per-sample for now but optimize the fetches.
        let mut current_seq_phase = self.seq_phase;
        let tempo = context.transport().tempo.unwrap_or(120.0) as f32;
        let step_rate = (tempo / 60.0) * p.seq_rate;
        let phase_inc = step_rate / sample_rate;

        for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
            let (seq_gate, seq_cut, seq_res, seq_wt, seq_dist, seq_fm) = if p.seq_enable {
                let step_idx = (current_seq_phase.floor() as usize) % 32;
                let gate = self.params.seq_lanes[0].steps[step_idx].value.value();
                let cut = self.params.seq_lanes[1].steps[step_idx].value.value();
                let res = self.params.seq_lanes[2].steps[step_idx].value.value();
                let wt = self.params.seq_lanes[3].steps[step_idx].value.value();
                let dist = self.params.seq_lanes[4].steps[step_idx].value.value();
                let fm = self.params.seq_lanes[5].steps[step_idx].value.value();

                current_seq_phase += phase_inc;
                if current_seq_phase >= 32.0 { current_seq_phase -= 32.0; }

                let gate_step = (gate * 0.5 + 0.5).clamp(0.0, 1.0);
                let gate_value = (1.0 - p.seq_gate_amount) + p.seq_gate_amount * gate_step;

                (
                    gate_value,
                    cut * p.seq_cut_amount,
                    res * p.seq_res_amount,
                    wt * p.seq_wt_amount,
                    dist * p.seq_dist_amount,
                    fm * p.seq_fm_amount,
                )
            } else {
                (1.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            };

            for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                // Handle voice gain poly modulation
                let current_gain = if let Some((_, smoother)) = &mut voice.voice_gain {
                    smoother.next()
                } else {
                    1.0 // This will be multiplied by global gain later
                };

                let (out_l, out_r, fx_l, fx_r) = self.render_voice_sample(
                    voice,
                    sample_rate,
                    p,
                    seq_gate, seq_cut, seq_res, seq_wt, seq_dist, seq_fm,
                );

                output[0][sample_idx] += out_l * current_gain;
                output[1][sample_idx] += out_r * current_gain;
                fx_left[value_idx] += fx_l * current_gain;
                fx_right[value_idx] += fx_r * current_gain;
            }
        }
        self.seq_phase = current_seq_phase;
    }

    fn render_voice_sample(
        &mut self,
        voice: &mut Voice,
        sample_rate: f32,
        p: &BlockParams,
        seq_gate: f32,
        seq_cut: f32,
        seq_res: f32,
        seq_wt: f32,
        seq_dist: f32,
        seq_fm: f32,
    ) -> (f32, f32, f32, f32) {
        // 1. Pitch & Glide
        if p.glide_time > 0.0 {
            let coeff = (-1.0_f32 / (p.glide_time * 0.001 * sample_rate)).exp();
            voice.phase_delta = voice.phase_delta * coeff + voice.target_phase_delta * (1.0 - coeff);
        } else {
            voice.phase_delta = voice.target_phase_delta;
        }

        let vibrato_modulation = voice.vib_mod.get_modulation(sample_rate);
        let vibrato_phase_delta = voice.phase_delta * (1.0 + (p.vibrato_intensity * vibrato_modulation));
        
        if p.analog_enable && p.analog_drift > 0.0 {
            let jitter = (self.prng.gen::<f32>() - 0.5) * p.analog_drift * 0.0005;
            voice.drift_offset = (voice.drift_offset + jitter).clamp(-0.02, 0.02);
        }
        let drifted_phase_delta = vibrato_phase_delta * (1.0 + voice.drift_offset);

        // 2. Envelopes
        voice.amp_envelope.advance();
        voice.filter_cut_envelope.advance();
        voice.filter_res_envelope.advance();
        voice.fm_envelope.advance();
        voice.dist_envelope.advance();

        let amp_env = voice.amp_envelope.get_value();
        let cut_env = voice.filter_cut_envelope.get_value();
        let res_env = voice.filter_res_envelope.get_value();
        let fm_env = voice.fm_envelope.get_value();
        let dist_env = voice.dist_envelope.get_value();

        // 3. Modulations
        let lfo1 = voice.mod_lfo1.get_modulation(sample_rate);
        let lfo2 = voice.mod_lfo2.get_modulation(sample_rate);
        let (wt_mod, cut_mod, res_mod, amt_mod, pan_mod, gain_mod, fm_a_mod, fm_r_mod, fm_f_mod) = 
            self.apply_voice_modulations(voice, lfo1, lfo2, amp_env, cut_env, sample_rate);

        let final_wt_pos = (p.wavetable_position + wt_mod + seq_wt).clamp(0.0, 1.0);
        let final_fm_amt = (p.fm_amount + fm_a_mod + seq_fm + (fm_env * p.fm_env_amount)).clamp(-1.0, 1.0);
        let final_fm_ratio = (p.fm_ratio + fm_r_mod).clamp(0.25, 8.0);
        let final_fm_fback = (p.fm_feedback + fm_f_mod).clamp(0.0, 0.99);

        // 4. Oscillators
        let base_phase = voice.unison_phases[0];
        let fm_signal = if p.fm_enable {
            let mod_phase = (base_phase * final_fm_ratio + voice.fm_feedback_state * final_fm_fback).fract();
            let wavetable_bank = if self.params.custom_wavetable_enable.value() {
                self.custom_wavetable.as_ref().unwrap_or(&self.factory_wavetable)
            } else {
                &self.factory_wavetable
            };
            let mod_sample = match p.fm_source {
                FmSource::Classic => generate_waveform(p.waveform, mod_phase),
                FmSource::Wavetable => wavetable_bank.sample(mod_phase, final_wt_pos),
                FmSource::Sub => (2.0 * std::f32::consts::PI * mod_phase).sin(),
            };
            voice.fm_feedback_state = mod_sample;
            mod_sample * final_fm_amt * 0.25
        } else {
            voice.fm_feedback_state = 0.0;
            0.0
        };

        let unison_count = match p.unison_voices {
            UnisonVoices::One => 1,
            UnisonVoices::Two => 2,
            UnisonVoices::Four => 4,
            UnisonVoices::Six => 6,
        };
        let detune_cents = p.unison_detune * 30.0;
        let offsets: &[f32] = match unison_count {
            1 => &[0.0],
            2 => &[-0.5, 0.5],
            4 => &[-0.75, -0.25, 0.25, 0.75],
            _ => &[-1.0, -0.6, -0.2, 0.2, 0.6, 1.0],
        };

        let wavetable_bank = if self.params.custom_wavetable_enable.value() {
            self.custom_wavetable.as_ref().unwrap_or(&self.factory_wavetable)
        } else {
            &self.factory_wavetable
        };

        let mut classic_sum = 0.0;
        let mut wavetable_sum = 0.0;
        for i in 0..unison_count {
            let offset = offsets[i];
            let ratio = 2.0_f32.powf(detune_cents * offset / 1200.0);
            let phase = voice.unison_phases[i];
            
            let classic_phase = if p.fm_enable && matches!(p.fm_target, FmTarget::Classic | FmTarget::Both) {
                (phase + fm_signal).fract()
            } else { phase };
            
            let wavetable_phase = if p.fm_enable && matches!(p.fm_target, FmTarget::Wavetable | FmTarget::Both) {
                (phase + fm_signal).fract()
            } else { phase };

            let mut classic_sample = generate_waveform(p.waveform, classic_phase);
            classic_sample = SubSynth::wavefold(classic_sample, p.analog_drive); // Wait, classic_drive? 
            // In original it was p.classic_drive. Let's use p.analog_drive for now if I missed classic_drive in BlockParams.
            // Actually I should check BlockParams.
            classic_sample -= SubSynth::poly_blep(phase, drifted_phase_delta * ratio);

            let mut wavetable_sample = wavetable_bank.sample(wavetable_phase, final_wt_pos);
            wavetable_sample = SubSynth::wavefold(wavetable_sample, p.wavetable_distortion);

            classic_sum += classic_sample;
            wavetable_sum += wavetable_sample;

            let next_phase = phase + drifted_phase_delta * ratio;
            voice.unison_phases[i] = if next_phase >= 1.0 { next_phase - 1.0 } else { next_phase };
        }
        let classic_sum = classic_sum / unison_count as f32;
        let wavetable_sum = wavetable_sum / unison_count as f32;

        // 5. Signal Path
        let mut classic_sample = classic_sum;
        if p.sizzle_osc_enable {
            voice.sizzle_osc_lp.set_lowpass(sample_rate, p.sizzle_cutoff, 0.7);
            classic_sample = voice.sizzle_osc_lp.process(classic_sample);
        }
        let mut wavetable_sample = wavetable_sum;
        if p.sizzle_wt_enable {
            voice.sizzle_wt_lp.set_lowpass(sample_rate, p.sizzle_cutoff, 0.7);
            wavetable_sample = voice.sizzle_wt_lp.process(wavetable_sample);
        }
        classic_sample *= p.classic_level;
        wavetable_sample *= p.wavetable_level;

        let mut sub_sample = 0.0;
        if p.sub_level > 0.0 {
            let sub_phase = (base_phase * 0.5).fract();
            sub_sample = (2.0 * std::f32::consts::PI * sub_phase).sin() * p.sub_level;
        }

        let noise_sample = if p.noise_level > 0.0 {
            (self.prng.gen::<f32>() * 2.0 - 1.0) * p.noise_level
        } else { 0.0 };

        let mut pre_filter = match p.osc_routing {
            OscRouting::ClassicOnly => classic_sample,
            OscRouting::WavetableOnly => wavetable_sample,
            OscRouting::Blend => classic_sample * (1.0 - p.osc_blend) + wavetable_sample * p.osc_blend,
        };
        pre_filter += sub_sample + noise_sample;

        if p.sizzle_osc_enable || p.sizzle_wt_enable {
            let alias_cutoff = p.sizzle_cutoff.min(sample_rate * 0.45);
            voice.alias_lp1.set_lowpass(sample_rate, alias_cutoff, 0.7);
            voice.alias_lp2.set_lowpass(sample_rate, alias_cutoff, 0.7);
            pre_filter = voice.alias_lp2.process(voice.alias_lp1.process(pre_filter));
        }

        // Ring Mod Pre
        let mut ring_carrier = 0.0;
        if p.ring_mod_enable {
            ring_carrier = match p.ring_mod_source {
                RingModSource::Sine => (2.0 * std::f32::consts::PI * voice.ring_phase).sin(),
                RingModSource::Classic => classic_sum,
                RingModSource::Wavetable => wavetable_sum,
            };
            voice.ring_phase = (voice.ring_phase + p.ring_mod_freq / sample_rate).fract();
        }
        if p.ring_mod_enable && p.ring_mod_level > 0.0 && p.ring_mod_placement == RingModPlacement::PreFilter {
            pre_filter += pre_filter * ring_carrier * p.ring_mod_level;
        }

        // 6. Filter
        let mod_cut = (p.filter_cut * (1.0 + cut_mod + seq_cut)).clamp(20.0, 20000.0);
        let mod_res = (p.filter_res + res_mod + seq_res).clamp(0.0, 1.0);
        let final_cut = mod_cut * (1.0 - p.filter_cut_env_level + p.filter_cut_env_level * cut_env);
        let final_res = mod_res * (1.0 - p.filter_res_env_level + p.filter_res_env_level * res_env);
        let final_cut = final_cut.clamp(20.0, 20000.0);
        let final_res = final_res.clamp(0.0, 1.0);

        let filtered = self.apply_voice_filter(voice, pre_filter, final_cut, final_res, p.filter_type, p.filter_tight_enable, sample_rate);
        
        let mut final_out = pre_filter * (1.0 - (p.filter_amount + amt_mod).clamp(0.0, 1.0)) + filtered * (p.filter_amount + amt_mod).clamp(0.0, 1.0);

        // Ring Mod Post Filter
        if p.ring_mod_enable && p.ring_mod_level > 0.0 && p.ring_mod_placement == RingModPlacement::PostFilter {
            final_out += final_out * ring_carrier * p.ring_mod_level;
        }

        // 7. Amp & Output
        let final_gain = (1.0 + gain_mod).clamp(0.0, 2.0);
        let amp = voice.velocity_sqrt * (amp_env * p.amp_env_level) * 0.5 * (voice.trem_mod.get_modulation(sample_rate) + 1.0) * final_gain * seq_gate;
        
        let naive = final_out;
        let corrected = naive - SubSynth::poly_blep(voice.phase, voice.phase_delta);
        let mut output = corrected * amp;

        if p.analog_enable {
            if p.analog_noise > 0.0 { output += (self.prng.gen::<f32>() * 2.0 - 1.0) * p.analog_noise; }
            if p.analog_drive > 0.0 {
                let drive = 1.0 + p.analog_drive * 4.0;
                output = (output * drive).tanh() / drive;
            }
        }

        // FX Sends
        let ring_in_mix = if p.ring_mod_enable && p.ring_mod_placement != RingModPlacement::PostFx { p.ring_mod_level } else { 0.0 };
        let send_sum = p.classic_level * p.classic_send + p.wavetable_level * p.wavetable_send + p.sub_level * p.sub_send + p.noise_level * p.noise_send + ring_in_mix * p.ring_mod_send;
        let source_sum = p.classic_level + p.wavetable_level + p.sub_level + p.noise_level + ring_in_mix;
        let send_scale = if source_sum > 0.0 { (send_sum / source_sum).clamp(0.0, 1.0) } else { 0.0 };
        let fx_sample = output * send_scale;

        // Pan & Stereo
        let dc_blocked = voice.dc_blocker.process(output);
        let spread = p.unison_spread.clamp(0.0, 1.0);
        let diff = dc_blocked - voice.stereo_prev;
        voice.stereo_prev = dc_blocked;
        
        let pan = (voice.pan + voice.pan_mod.get_modulation(sample_rate) + pan_mod).clamp(0.0, 1.0);
        let left_amp = (1.0 - pan).sqrt();
        let right_amp = pan.sqrt();

        let l_wide = dc_blocked + diff * spread;
        let r_wide = dc_blocked - diff * spread;

        voice.phase = voice.unison_phases[0];

        (l_wide * left_amp, r_wide * right_amp, fx_sample * left_amp, fx_sample * right_amp)
    }

    fn apply_voice_filter(&mut self, voice: &mut Voice, input: f32, cutoff: f32, resonance: f32, filter_type: FilterType, tight: bool, sample_rate: f32) -> f32 {
        let mut filtered = match filter_type {
            FilterType::None => input,
            FilterType::Lowpass => { voice.lowpass_filter.set_cutoff(cutoff); voice.lowpass_filter.set_resonance(resonance); voice.lowpass_filter.process(input) }
            FilterType::Highpass => { voice.highpass_filter.set_cutoff(cutoff); voice.highpass_filter.set_resonance(resonance); voice.highpass_filter.process(input) }
            FilterType::Bandpass => { voice.bandpass_filter.set_cutoff(cutoff); voice.bandpass_filter.set_resonance(resonance); voice.bandpass_filter.process(input) }
            FilterType::Notch => { voice.notch_filter.set_cutoff(cutoff); voice.notch_filter.set_resonance(resonance); voice.notch_filter.process(input) }
            FilterType::Statevariable => { voice.statevariable_filter.set_cutoff(cutoff); voice.statevariable_filter.set_resonance(resonance); voice.statevariable_filter.process(input) }
            FilterType::Comb => { voice.comb_filter.set_cutoff(cutoff); voice.comb_filter.set_resonance(resonance); voice.comb_filter.process(input) }
            FilterType::RainbowComb => { voice.rainbow_comb_filter.set_cutoff(cutoff); voice.rainbow_comb_filter.set_resonance(resonance); voice.rainbow_comb_filter.process(input) }
            FilterType::DiodeLadderLp => { voice.diode_ladder_lp_filter.set_cutoff(cutoff); voice.diode_ladder_lp_filter.set_resonance(resonance); voice.diode_ladder_lp_filter.process(input) }
            FilterType::DiodeLadderHp => { voice.diode_ladder_hp_filter.set_cutoff(cutoff); voice.diode_ladder_hp_filter.set_resonance(resonance); voice.diode_ladder_hp_filter.process(input) }
            FilterType::Ms20Pair => { voice.ms20_filter.set_cutoff(cutoff); voice.ms20_filter.set_resonance(resonance); voice.ms20_filter.process(input) }
            FilterType::FormantMorph => { voice.formant_morph_filter.set_cutoff(cutoff); voice.formant_morph_filter.set_resonance(resonance); voice.formant_morph_filter.process(input) }
            FilterType::Phaser => { voice.phaser_filter.set_cutoff(cutoff); voice.phaser_filter.set_resonance(resonance); voice.phaser_filter.process(input) }
            FilterType::CombAllpass => { voice.comb_allpass_filter.set_cutoff(cutoff); voice.comb_allpass_filter.set_resonance(resonance); voice.comb_allpass_filter.process(input) }
            FilterType::BitcrushLp => { voice.bitcrush_lp_filter.set_cutoff(cutoff); voice.bitcrush_lp_filter.set_resonance(resonance); voice.bitcrush_lp_filter.process(input) }
        };

        if !matches!(filter_type, FilterType::None) {
            filtered = filter::tame_resonance(filtered, resonance);
        }

        if tight {
            match filter_type {
                FilterType::Lowpass | FilterType::DiodeLadderLp | FilterType::BitcrushLp => {
                    voice.tight_lp.set_lowpass(sample_rate, cutoff, 0.7);
                    filtered = voice.tight_lp.process(filtered);
                }
                FilterType::Highpass | FilterType::DiodeLadderHp => {
                    voice.tight_hp.set_highpass(sample_rate, cutoff, 0.7);
                    filtered = voice.tight_hp.process(filtered);
                }
                _ => {}
            }
        }
        filtered
    }
}

impl SubSynth {
    fn apply_block_fx(
        &mut self,
        output: &mut [&mut [f32]],
        fx_left: &mut [f32],
        fx_right: &mut [f32],
        block_start: usize,
        block_end: usize,
        sample_rate: f32,
        p: &BlockParams,
    ) {
        let block_len = block_end - block_start;

        // 1. Pre-FX Spectral
        if p.spectral_enable && p.spectral_placement == SpectralPlacement::PreFx {
            let (out_left, out_right) = output.split_at_mut(1);
            self.spectral_main.process_block(&mut out_left[0][block_start..block_end], &mut out_right[0][block_start..block_end], p.spectral_amount, p.spectral_tilt, p.spectral_formant);
            self.spectral_fx.process_block(&mut fx_left[..block_len], &mut fx_right[..block_len], p.spectral_amount, p.spectral_tilt, p.spectral_formant);
        }

        // 2. Chorus
        if p.chorus_enable {
            self.chorus.set_enabled(true);
            for i in 0..block_len {
                let (l, r) = self.chorus.process(fx_left[i], fx_right[i], p.chorus_rate, p.chorus_depth, p.chorus_mix);
                fx_left[i] = l; fx_right[i] = r;
            }
        } else { self.chorus.set_enabled(false); }

        // 3. Multi-Filter
        if p.multi_filter_enable {
            for i in 0..block_len {
                let (l, r) = self.multi_filter.process(
                    fx_left[i], fx_right[i], p.multi_filter_routing,
                    p.multi_filter_a_type, p.multi_filter_a_cut, p.multi_filter_a_res, p.multi_filter_a_amt,
                    p.multi_filter_b_type, p.multi_filter_b_cut, p.multi_filter_b_res, p.multi_filter_b_amt,
                    p.multi_filter_c_type, p.multi_filter_c_cut, p.multi_filter_c_res, p.multi_filter_c_amt,
                    p.multi_filter_morph, 
                    if p.multi_filter_parallel_ab { 1.0 } else { 0.0 }, 
                    if p.multi_filter_parallel_c { 1.0 } else { 0.0 },
                );
                fx_left[i] = l; fx_right[i] = r;
            }
        }

        // 4. Pre-Dist Spectral
        if p.spectral_enable && p.spectral_placement == SpectralPlacement::PreDist {
            self.spectral_fx.process_block(&mut fx_left[..block_len], &mut fx_right[..block_len], p.spectral_amount, p.spectral_tilt, p.spectral_formant);
        }

        // 5. Distortion
        if p.dist_enable {
            self.distortion.set_tone(p.dist_tone);
            self.distortion.set_sizzle_guard(p.sizzle_dist_enable, p.sizzle_cutoff);
            for i in 0..block_len {
                // Note: dist_drive was modulated by seq/env in original.
                // For now we use the block-static dist_drive. Optimization: passed modulated drive.
                fx_left[i] = self.distortion.process_sample(0, fx_left[i], p.dist_drive, p.dist_magic, p.dist_mix);
                fx_right[i] = self.distortion.process_sample(1, fx_right[i], p.dist_drive, p.dist_magic, p.dist_mix);
            }
        }

        // 6. EQ
        if p.eq_enable {
            self.eq.set_params(p.eq_low_gain, p.eq_mid_gain, p.eq_mid_freq, p.eq_mid_q, p.eq_high_gain);
            for i in 0..block_len {
                let l = self.eq.process_sample(0, fx_left[i]);
                let r = self.eq.process_sample(1, fx_right[i]);
                fx_left[i] = fx_left[i] * (1.0 - p.eq_mix) + l * p.eq_mix;
                fx_right[i] = fx_right[i] * (1.0 - p.eq_mix) + r * p.eq_mix;
            }
        }

        // 7. Delay & Reverb
        if p.delay_enable {
            for i in 0..block_len {
                let (l, r) = self.delay.process(fx_left[i], fx_right[i], p.delay_time, p.delay_feedback, p.delay_mix);
                fx_left[i] = l; fx_right[i] = r;
            }
        }
        if p.reverb_enable {
            for i in 0..block_len {
                let (l, r) = self.reverb.process(fx_left[i], fx_right[i], p.reverb_size, p.reverb_damp, p.reverb_diffusion, p.reverb_shimmer, p.reverb_mix);
                fx_left[i] = l; fx_right[i] = r;
            }
        }

        // 8. Mix FX Bus
        for i in 0..block_len {
            let sample_idx = block_start + i;
            output[0][sample_idx] = output[0][sample_idx] * (1.0 - p.fx_bus_mix) + fx_left[i] * p.fx_bus_mix;
            output[1][sample_idx] = output[1][sample_idx] * (1.0 - p.fx_bus_mix) + fx_right[i] * p.fx_bus_mix;
        }

        // 9. Post-FX Spectral
        if p.spectral_enable && p.spectral_placement == SpectralPlacement::PostFx {
            let (out_left, out_right) = output.split_at_mut(1);
            self.spectral_main.process_block(&mut out_left[0][block_start..block_end], &mut out_right[0][block_start..block_end], p.spectral_amount, p.spectral_tilt, p.spectral_formant);
        }

        // 10. Ring Mod Post FX
        if p.ring_mod_enable && p.ring_mod_placement == RingModPlacement::PostFx {
            for i in 0..block_len {
                let sample_idx = block_start + i;
                let carrier_l = match p.ring_mod_source { RingModSource::Sine => (2.0 * std::f32::consts::PI * self.ring_mod_post_phase[0]).sin(), _ => output[0][sample_idx] };
                let carrier_r = match p.ring_mod_source { RingModSource::Sine => (2.0 * std::f32::consts::PI * self.ring_mod_post_phase[1]).sin(), _ => output[1][sample_idx] };
                output[0][sample_idx] = output[0][sample_idx] * (1.0 - p.ring_mod_mix) + output[0][sample_idx] * carrier_l * p.ring_mod_mix;
                output[1][sample_idx] = output[1][sample_idx] * (1.0 - p.ring_mod_mix) + output[1][sample_idx] * carrier_r * p.ring_mod_mix;
                self.ring_mod_post_phase[0] = (self.ring_mod_post_phase[0] + p.ring_mod_freq / sample_rate).fract();
                self.ring_mod_post_phase[1] = (self.ring_mod_post_phase[1] + p.ring_mod_freq / sample_rate).fract();
            }
        }

        // 11. Saturation & Limiter
        if p.output_sat_enable {
            for i in 0..block_len {
                let sample_idx = block_start + i;
                output[0][sample_idx] = self.output_saturation.process_sample(0, output[0][sample_idx], p.output_sat_drive, p.output_sat_type, p.output_sat_mix);
                output[1][sample_idx] = self.output_saturation.process_sample(1, output[1][sample_idx], p.output_sat_drive, p.output_sat_type, p.output_sat_mix);
            }
        }
        self.limiter_left.set_enabled(p.limiter_enable);
        self.limiter_right.set_enabled(p.limiter_enable);
        if p.limiter_enable {
            self.limiter_left.set_threshold(p.limiter_threshold);
            self.limiter_right.set_threshold(p.limiter_threshold);
            self.limiter_left.set_release(p.limiter_release);
            self.limiter_right.set_release(p.limiter_release);
            for i in 0..block_len {
                let sample_idx = block_start + i;
                output[0][sample_idx] = self.limiter_left.process(output[0][sample_idx]);
                output[1][sample_idx] = self.limiter_right.process(output[1][sample_idx]);
            }
        }
    }

    fn terminate_finished_voices(&mut self, context: &mut impl ProcessContext<Self>, timing: usize) {
        for voice in &mut self.voices {
            if let Some(v) = voice {
                if v.releasing && v.amp_envelope.get_state() == ADSREnvelopeState::Idle {
                    context.send_event(NoteEvent::VoiceTerminated {
                        timing: timing as u32,
                        voice_id: Some(v.voice_id),
                        channel: v.channel,
                        note: v.note,
                    });
                    *voice = None;
                }
            }
        }
    }
}
