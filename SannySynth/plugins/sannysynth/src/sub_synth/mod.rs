pub mod params;
pub mod voice;

use nih_plug::prelude::*;
use rand::Rng;
use rand_pcg::Pcg32;
use std::sync::Arc;

use crate::chorus::Chorus;
use crate::common::*;
use crate::delay::StereoDelay;
use crate::envelope::{ADSREnvelope, ADSREnvelopeState};
use crate::filter::{self, FilterType};
use crate::multi_filter::MultiStageFilter;
use crate::limiter::Limiter;
use crate::modulator::{Modulator, OscillatorShape};
use crate::resonator::ResonatorBank;
use crate::sub_synth::params::{SubSynthBlockParams, SubSynthParams};
use crate::sub_synth::voice::Voice;
use crate::waveform::{WavetableBank, Waveform};
use crate::util;

pub const NUM_VOICES: usize = 16;
pub const MAX_BLOCK_SIZE: usize = 64;

pub struct SubSynth {
    pub params: Arc<SubSynthParams>,
    pub prng: Pcg32,
    pub voices: [Option<Voice>; NUM_VOICES],
    pub next_internal_voice_id: u64,
    pub chorus: Chorus,
    pub delay: StereoDelay,
    pub reverb: crate::reverb::Reverb,
    pub limiter_left: Limiter,
    pub limiter_right: Limiter,
    pub multi_filter: MultiStageFilter,
    pub factory_wavetable: WavetableBank,
    pub custom_wavetable: Option<WavetableBank>,
    pub custom_wavetable_path: Option<String>,
    pub sample_rate: f32,
}

impl Default for SubSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(SubSynthParams::default()),
            prng: Pcg32::new(420, 1337),
            voices: std::array::from_fn(|_| None),
            next_internal_voice_id: 0,
            chorus: Chorus::new(44100.0),
            delay: StereoDelay::new(44100.0),
            reverb: crate::reverb::Reverb::new(44100.0),
            limiter_left: Limiter::new(),
            limiter_right: Limiter::new(),
            multi_filter: MultiStageFilter::new(44100.0),
            factory_wavetable: WavetableBank::new(),
            custom_wavetable: None,
            custom_wavetable_path: None,
            sample_rate: 44100.0,
        }
    }
}

impl SubSynth {
    pub fn poly_blep(t: f32, dt: f32) -> f32 {
        if t < dt {
            let t = t / dt;
            2.0 * t - t * t - 1.0
        } else if t > 1.0 - dt {
            let t = (t - 1.0) / dt;
            t * t + 2.0 * t + 1.0
        } else {
            0.0
        }
    }

    fn handle_event(&mut self, event: NoteEvent<()>, sample_rate: f32, context: &mut impl ProcessContext<Self>, this_sample_id: u64) {
        match event {
            NoteEvent::NoteOn { timing, voice_id, channel, note, velocity } => {
                let initial_phase: f32 = self.prng.gen();
                let pitch = util::midi_note_to_freq(note);
                
                let (amp_env, cut_env, res_env) = self.construct_envelopes(sample_rate, velocity);
                
                let voice_idx = self.start_voice(
                    context, timing, voice_id, channel, note, velocity,
                    amp_env, cut_env, res_env,
                    this_sample_id, sample_rate
                );
                
                let voice = self.voices[voice_idx].as_mut().unwrap();
                voice.phase = initial_phase;
                voice.phase_delta = pitch / sample_rate;
            }
            NoteEvent::NoteOff { voice_id, channel, note, .. } => {
                self.start_release_for_voices(voice_id, channel, note);
            }
            NoteEvent::Choke { timing, voice_id, channel, note } => {
                self.choke_voices(context, timing, voice_id, channel, note);
            }
            _ => {}
        }
    }

    fn construct_envelopes(&self, sample_rate: f32, velocity: f32) -> (ADSREnvelope, ADSREnvelope, ADSREnvelope) {
        (
            ADSREnvelope::new(
                self.params.amp_attack_ms.value(), self.params.amp_envelope_level.value(),
                self.params.amp_decay_ms.value(), self.params.amp_sustain_level.value(),
                self.params.amp_release_ms.value(), sample_rate, velocity, 0.0,
            ),
            ADSREnvelope::new(
                self.params.filter_cut_attack_ms.value(), self.params.filter_cut_envelope_level.value(),
                self.params.filter_cut_decay_ms.value(), self.params.filter_cut_sustain_ms.value(),
                self.params.filter_cut_release_ms.value(), sample_rate, velocity, self.params.filter_cut_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_res_attack_ms.value(), self.params.filter_res_envelope_level.value(),
                self.params.filter_res_decay_ms.value(), self.params.filter_res_sustain_ms.value(),
                self.params.filter_res_release_ms.value(), sample_rate, velocity, self.params.filter_res_tension.value(),
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
        amp_env: ADSREnvelope,
        cut_env: ADSREnvelope,
        res_env: ADSREnvelope,
        internal_id: u64,
        sample_rate: f32,
    ) -> usize {
        let new_voice = Voice {
            voice_id: voice_id.unwrap_or(note as i32),
            internal_voice_id: internal_id,
            channel, note, velocity,
            velocity_sqrt: velocity.sqrt(),
            phase: 0.0, phase_delta: 0.0, target_phase_delta: 0.0,
            releasing: false,
            amp_envelope: amp_env,
            voice_gain: None,
            filter_cut_envelope: cut_env,
            filter_res_envelope: res_env,
            filter: Some(self.params.filter_type.value()),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, sample_rate),
            highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, sample_rate),
            bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, sample_rate),
            notch_filter: filter::NotchFilter::new(1000.0, 1.0, sample_rate),
            statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, sample_rate),
            pressure: 1.0, pan: 0.5, tuning: 0.0, vibrato: 0.0, expression: 1.0, brightness: 1.0,
            vib_mod: Modulator::new(self.params.vibrato_rate.value(), self.params.vibrato_intensity.value(), self.params.vibrato_attack.value(), self.params.vibrato_shape.value()),
            trem_mod: Modulator::new(self.params.tremolo_rate.value(), self.params.tremolo_intensity.value(), self.params.tremolo_attack.value(), self.params.tremolo_shape.value()),
            pan_mod: Modulator::new(self.params.pan_lfo_rate.value(), self.params.pan_lfo_intensity.value(), self.params.pan_lfo_attack.value(), self.params.pan_lfo_shape.value()),
            mod_lfo1: Modulator::new(self.params.lfo1_rate.value(), 1.0, self.params.lfo1_attack.value(), self.params.lfo1_shape.value()),
            mod_lfo2: Modulator::new(self.params.lfo2_rate.value(), 1.0, self.params.lfo2_attack.value(), self.params.lfo2_shape.value()),
            drift_offset: 0.0,
            resonator: ResonatorBank::new(sample_rate, util::midi_note_to_freq(note)),
            prng: rand_pcg::Pcg32::new(self.prng.gen(), self.prng.gen()),
        };        if let Some(idx) = self.voices.iter().position(|v| v.is_none()) {
            self.voices[idx] = Some(new_voice);
            idx
        } else {
            let (idx, oldest) = self.voices.iter_mut().enumerate().min_by_key(|(_, v)| v.as_ref().unwrap().internal_voice_id).unwrap();
            let v = oldest.as_mut().unwrap();
            context.send_event(NoteEvent::VoiceTerminated { timing: sample_offset, voice_id: Some(v.voice_id), channel: v.channel, note: v.note });
            *v = new_voice;
            idx
        }
    }

    fn start_release_for_voices(&mut self, voice_id: Option<i32>, channel: u8, note: u8) {
        for voice in self.voices.iter_mut().flatten() {
            if voice_id == Some(voice.voice_id) || (channel == voice.channel && note == voice.note) {
                voice.releasing = true;
                voice.amp_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                voice.filter_cut_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                voice.filter_res_envelope.set_envelope_stage(ADSREnvelopeState::Release);
            }
        }
    }

    fn find_or_create_voice(
        &mut self,
        voice_id: Option<i32>,
        channel: u8,
        note: u8,
        pan: f32,
        pressure: f32,
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
        if let Some(idx) = self.voices.iter().position(|v| {
            v.as_ref().map_or(false, |v| {
                voice_id == Some(v.voice_id) || (channel == v.channel && note == v.note)
            })
        }) {
            return self.voices[idx].as_mut().unwrap();
        }

        let voice_idx = self.start_voice(
            &mut crate::sub_synth::dummy::DummyProcessContext,
            0, voice_id, channel, note, 1.0,
            amp_envelope, filter_cut_envelope, filter_res_envelope,
            self.next_internal_voice_id, self.sample_rate
        );
        self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);
        let voice = self.voices[voice_idx].as_mut().unwrap();
        voice.pan = pan;
        voice.pressure = pressure;
        voice.brightness = brightness;
        voice.expression = expression;
        voice.tuning = tuning;
        voice.vibrato = vibrato;
        voice.vib_mod = vib_mod;
        voice.trem_mod = trem_mod;
        voice
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
        vib_mod: Option<&Modulator>,
        trem_mod: Option<&Modulator>,
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
            vib_mod.cloned().unwrap_or_default(),
            trem_mod.cloned().unwrap_or_default(),
        );
        voice.velocity = gain;
        voice.velocity_sqrt = gain.sqrt();
        if let Some(amp_env) = amp_envelope {
            voice.amp_envelope = amp_env.clone();
            voice.amp_envelope.set_velocity(gain);
        }
    }

    fn choke_voices(&mut self, context: &mut impl ProcessContext<Self>, timing: u32, voice_id: Option<i32>, channel: u8, note: u8) {
        for voice in self.voices.iter_mut() {
            if let Some(v) = voice {
                if voice_id == Some(v.voice_id) || (channel == v.channel && note == v.note) {
                    context.send_event(NoteEvent::VoiceTerminated { timing, voice_id: Some(v.voice_id), channel: v.channel, note: v.note });
                    *voice = None;
                }
            }
        }
    }

    fn refresh_custom_wavetable(&mut self) {
        if let Ok(mut data) = self.params.custom_wavetable_data.try_write() {
            if let Some(table) = data.take() {
                self.custom_wavetable = Some(WavetableBank::from_table(table));
                if let Ok(path) = self.params.custom_wavetable_path.read() {
                    self.custom_wavetable_path = path.clone();
                }
            }
        }
    }
}

pub mod dummy {
    use nih_plug::prelude::*;
    pub struct DummyProcessContext;
    impl<P: Plugin> ProcessContext<P> for DummyProcessContext {
        fn plugin_api(&self) -> PluginApi { PluginApi::Standalone }
        fn transport(&self) -> &Transport { unreachable!() }
        fn next_event(&mut self) -> Option<NoteEvent<P::SysExMessage>> { None }
        fn send_event(&mut self, _event: NoteEvent<P::SysExMessage>) {}
        fn set_latency_samples(&self, _samples: u32) {}
        fn execute_background(&self, _task: P::BackgroundTask) {}
        fn execute_gui(&self, _task: P::BackgroundTask) {}
        fn set_current_voice_capacity(&self, _capacity: u32) {}
    }
}

impl Plugin for SubSynth {
    const NAME: &'static str = "SannySynth";
    const VENDOR: &'static str = "Ling Lin";
    const URL: &'static str = "https://github.com/robbert-vdh/nih-plug";
    const EMAIL: &'static str = "taellinglin@gmail.com";
    const VERSION: &'static str = "0.1.0";

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

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let output = buffer.as_slice();

        self.refresh_custom_wavetable();
        
        let mut block_start: usize = 0;
        while block_start < num_samples {
            let block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
            let block_len = block_end - block_start;

            while let Some(event) = context.next_event() {
                if (event.timing() as usize) < block_end {
                    self.handle_event(event, sample_rate, context, self.next_internal_voice_id);
                    self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);
                } else {
                    break;
                }
            }

            let bp = SubSynthBlockParams::cache(&self.params);

            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);

            let mut gain = [0.0; MAX_BLOCK_SIZE];
            self.params.gain.smoothed.next_block(&mut gain, block_len);

            for voice in self.voices.iter_mut().flatten() {
                for i in 0..block_len {
                    let (l, r) = voice.render_sample(&bp, &self.factory_wavetable, self.custom_wavetable.as_ref(), sample_rate, gain[i]);
                    output[0][block_start + i] += l;
                    output[1][block_start + i] += r;
                }
            }

            if bp.multi_filter_enable {
                for i in 0..block_len {
                    let (l, r) = self.multi_filter.process(
                        output[0][block_start + i], output[1][block_start + i],
                        bp.multi_filter_routing,
                        bp.multi_filter_a_type, bp.multi_filter_a_style, bp.multi_filter_a_drive, bp.multi_filter_a_curve, bp.multi_filter_a_mix, bp.multi_filter_a_trim, bp.multi_filter_a_cut, bp.multi_filter_a_res, bp.multi_filter_a_amt,
                        bp.multi_filter_b_type, bp.multi_filter_b_style, bp.multi_filter_b_drive, bp.multi_filter_b_curve, bp.multi_filter_b_mix, bp.multi_filter_b_trim, bp.multi_filter_b_cut, bp.multi_filter_b_res, bp.multi_filter_b_amt,
                        bp.multi_filter_c_type, bp.multi_filter_c_style, bp.multi_filter_c_drive, bp.multi_filter_c_curve, bp.multi_filter_c_mix, bp.multi_filter_c_trim, bp.multi_filter_c_cut, bp.multi_filter_c_res, bp.multi_filter_c_amt,
                        bp.multi_filter_morph, bp.multi_filter_parallel_ab, bp.multi_filter_parallel_c
                    );
                    output[0][block_start + i] = l;
                    output[1][block_start + i] = r;
                }
            }

            if bp.chorus_enable {
                for i in 0..block_len {
                    let (l, r) = self.chorus.process(output[0][block_start + i], output[1][block_start + i], bp.chorus_rate, bp.chorus_depth, bp.chorus_mix);
                    output[0][block_start + i] = l;
                    output[1][block_start + i] = r;
                }
            }

            if bp.delay_enable {
                for i in 0..block_len {
                    let (l, r) = self.delay.process(output[0][block_start + i], output[1][block_start + i], bp.delay_time_ms, bp.delay_feedback, bp.delay_mix);
                    output[0][block_start + i] = l;
                    output[1][block_start + i] = r;
                }
            }

            if bp.reverb_enable {
                for i in 0..block_len {
                    let (l, r) = self.reverb.process(output[0][block_start + i], output[1][block_start + i], bp.reverb_size, bp.reverb_damp, bp.reverb_diffusion, bp.reverb_shimmer, bp.reverb_mix);
                    output[0][block_start + i] = l;
                    output[1][block_start + i] = r;
                }
            }

            if bp.limiter_enable {
                self.limiter_left.set_threshold(bp.limiter_threshold);
                self.limiter_right.set_threshold(bp.limiter_threshold);
                self.limiter_left.set_release(bp.limiter_release);
                self.limiter_right.set_release(bp.limiter_release);
                for i in 0..block_len {
                    output[0][block_start + i] = self.limiter_left.process(output[0][block_start + i]);
                    output[1][block_start + i] = self.limiter_right.process(output[1][block_start + i]);
                }
            }

            for voice in self.voices.iter_mut() {
                if let Some(v) = voice {
                    if v.releasing && v.amp_envelope.get_state() == ADSREnvelopeState::Idle {
                        context.send_event(NoteEvent::VoiceTerminated { timing: block_end as u32, voice_id: Some(v.voice_id), channel: v.channel, note: v.note });
                        *voice = None;
                    }
                }
            }

            block_start = block_end;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for SubSynth {
    const CLAP_ID: &'static str = "art.sannysynth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A resonator-based synthesizer for natural and organic sound design");
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
    const VST3_CLASS_ID: [u8; 16] = *b"SannySynthLing1A";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}
