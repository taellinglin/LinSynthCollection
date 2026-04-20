pub mod params;
pub mod voice;

use nih_plug::prelude::*;
use rand::Rng;
use rand_pcg::Pcg32;
use std::sync::Arc;

use crate::chorus::Chorus;
use crate::common::*;
use crate::delay::StereoDelay;
use crate::distortion::Distortion;
use crate::envelope::{ADSREnvelope, ADSREnvelopeState};
use crate::eq::ThreeBandEq;
use crate::filter::{self, FilterType};
use crate::multi_filter::MultiStageFilter;
use crate::limiter::Limiter;
use crate::modulator::{Modulator, OscillatorShape};
use crate::output_saturation::OutputSaturation;
use crate::reverb::Reverb;
use crate::sub_synth::params::{SubSynthBlockParams, SubSynthParams, SEQ_LANE_COUNT, GAIN_POLY_MOD_ID};
use crate::sub_synth::voice::Voice;
use crate::waveform::{WavetableBank, Waveform};
use crate::util;

pub const NUM_VOICES: usize = 16;
pub const MAX_BLOCK_SIZE: usize = 64;

pub struct SubSynth {
    pub params: Arc<SubSynthParams>,
    pub prng: Pcg32,
    pub voices: [Option<Voice>; NUM_VOICES],
    pub next_voice_index: usize,
    pub next_internal_voice_id: u64,
    pub chorus: Chorus,
    pub delay: StereoDelay,
    pub reverb: Reverb,
    pub limiter_left: Limiter,
    pub limiter_right: Limiter,
    pub multi_filter: MultiStageFilter,
    pub distortion: Distortion,
    pub eq: ThreeBandEq,
    pub output_saturation: OutputSaturation,
    pub factory_wavetable: WavetableBank,
    pub custom_wavetable: Option<WavetableBank>,
    pub custom_wavetable_path: Option<String>,
    pub seq_phase: f32,
    pub last_note_phase_delta: f32,
    pub last_note_active: bool,
    pub sample_rate: f32,
}

impl Default for SubSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(SubSynthParams::default()),
            prng: Pcg32::new(420, 1337),
            voices: std::array::from_fn(|_| None),
            next_internal_voice_id: 0,
            next_voice_index: 0,
            chorus: Chorus::new(44100.0),
            delay: StereoDelay::new(44100.0),
            reverb: Reverb::new(44100.0),
            limiter_left: Limiter::new(),
            limiter_right: Limiter::new(),
            multi_filter: MultiStageFilter::new(44100.0),
            distortion: Distortion::new(44100.0),
            eq: ThreeBandEq::new(44100.0),
            output_saturation: OutputSaturation::new(44100.0),
            factory_wavetable: WavetableBank::new(),
            custom_wavetable: None,
            custom_wavetable_path: None,
            seq_phase: 0.0,
            last_note_phase_delta: 0.0,
            last_note_active: false,
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

    pub fn wavefold(sample: f32, drive: f32) -> f32 {
        if drive <= 0.0 {
            return sample;
        }
        let s = sample * (1.0 + drive * 4.0);
        if s > 1.0 {
            2.0 - s
        } else if s < -1.0 {
            -2.0 - s
        } else {
            s
        }
    }

    fn handle_event(&mut self, event: NoteEvent<()>, sample_rate: f32, context: &mut impl ProcessContext<Self>, this_sample_id: u64) {
        match event {
            NoteEvent::NoteOn { timing, voice_id, channel, note, velocity } => {
                let initial_phase: f32 = self.prng.gen();
                let pitch = util::midi_note_to_freq(note);
                let target_phase_delta = pitch / sample_rate;
                
                let (amp_env, cut_env, res_env) = self.construct_envelopes(sample_rate, velocity);
                let fm_env = ADSREnvelope::new(
                    self.params.fm_env_attack_ms.value(),
                    self.params.fm_env_amount.value(),
                    self.params.fm_env_decay_ms.value(),
                    self.params.fm_env_sustain_level.value(),
                    self.params.fm_env_release_ms.value(),
                    sample_rate, velocity, 0.0,
                );
                let dist_env = ADSREnvelope::new(
                    self.params.dist_env_attack_ms.value(),
                    self.params.dist_env_amount.value(),
                    self.params.dist_env_decay_ms.value(),
                    self.params.dist_env_sustain_level.value(),
                    self.params.dist_env_release_ms.value(),
                    sample_rate, velocity, 0.0,
                );

                let glide_mode = self.params.glide_mode.value();
                let last_note_active = self.last_note_active;
                let last_note_phase_delta = self.last_note_phase_delta;

                let voice_idx = self.start_voice(
                    context, timing, voice_id, channel, note, velocity,
                    amp_env, cut_env, res_env, fm_env, dist_env,
                    this_sample_id, sample_rate
                );
                
                {
                    let voice = self.voices[voice_idx].as_mut().unwrap();
                    voice.phase = initial_phase;
                    voice.unison_phases = [initial_phase; 6];
                    voice.target_phase_delta = target_phase_delta;
                    
                    let use_glide = match glide_mode {
                        GlideMode::Off => false,
                        GlideMode::Always => true,
                        GlideMode::Legato => last_note_active,
                    };
                    voice.phase_delta = if use_glide && last_note_phase_delta > 0.0 {
                        last_note_phase_delta
                    } else {
                        target_phase_delta
                    };
                }

                self.last_note_phase_delta = target_phase_delta;
                self.last_note_active = true;
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
                self.params.amp_release_ms.value(), sample_rate, velocity, self.params.amp_tension.value(),
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
        fm_env: ADSREnvelope,
        dist_env: ADSREnvelope,
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
            fm_envelope: fm_env,
            dist_envelope: dist_env,
            filter: Some(self.params.filter_type.value()),
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
            pressure: 1.0, pan: 0.5, tuning: 0.0, vibrato: 0.0, expression: 1.0, brightness: 1.0,
            vib_mod: Modulator::new(self.params.vibrato_rate.value(), self.params.vibrato_intensity.value(), self.params.vibrato_attack.value(), self.params.vibrato_shape.value()),
            trem_mod: Modulator::new(self.params.tremolo_rate.value(), self.params.tremolo_intensity.value(), self.params.tremolo_attack.value(), self.params.tremolo_shape.value()),
            pan_mod: Modulator::new(self.params.pan_lfo_rate.value(), self.params.pan_lfo_intensity.value(), self.params.pan_lfo_attack.value(), self.params.pan_lfo_shape.value()),
            mod_lfo1: Modulator::new(self.params.lfo1_rate.value(), 1.0, self.params.lfo1_attack.value(), self.params.lfo1_shape.value()),
            mod_lfo2: Modulator::new(self.params.lfo2_rate.value(), 1.0, self.params.lfo2_attack.value(), self.params.lfo2_shape.value()),
            drift_offset: 0.0, mod_smooth: [0.0; 6], fm_feedback_state: 0.0, unison_phases: [0.0; 6], stereo_prev: 0.0, dc_blocker: filter::DCBlocker::new(),
            prng: rand_pcg::Pcg32::new(self.prng.gen(), self.prng.gen()),
        };

        if let Some(idx) = self.voices.iter().position(|v| v.is_none()) {
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
                voice.fm_envelope.set_envelope_stage(ADSREnvelopeState::Release);
                voice.dist_envelope.set_envelope_stage(ADSREnvelopeState::Release);
            }
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
                    self.custom_wavetable_path = (*path).clone();
                }
            }
        }
    }
}

impl Plugin for SubSynth {
    const NAME: &'static str = "PlantSynth";
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

            // Handle events
            while let Some(event) = context.next_event() {
                if (event.timing() as usize) < block_end {
                    self.handle_event(event, sample_rate, context, self.next_internal_voice_id);
                    self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);
                } else {
                    break;
                }
            }

            // Cache parameters
            let bp = SubSynthBlockParams::cache(&self.params);

            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);

            // Sequencer logic
            let mut seq_values = [(1.0, 0.0, 0.0, 0.0, 0.0, 0.0); MAX_BLOCK_SIZE];
            if self.params.seq_enable.value() {
                let tempo = context.transport().tempo.unwrap_or(120.0) as f32;
                let step_rate = (tempo / 60.0) * self.params.seq_rate.value();
                let gate_amount = self.params.seq_gate_amount.value();

                for i in 0..block_len {
                    let step_idx = (self.seq_phase.floor() as usize) % 32;
                    let gate = (self.params.seq_lanes[0].steps[step_idx].value.value() * 0.5 + 0.5).clamp(0.0, 1.0);
                    let gate_val = (1.0 - gate_amount) + gate_amount * gate;
                    
                    seq_values[i] = (
                        gate_val,
                        self.params.seq_lanes[1].steps[step_idx].value.value() * self.params.seq_cut_amount.value(),
                        self.params.seq_lanes[2].steps[step_idx].value.value() * self.params.seq_res_amount.value(),
                        self.params.seq_lanes[3].steps[step_idx].value.value() * self.params.seq_wt_amount.value(),
                        self.params.seq_lanes[4].steps[step_idx].value.value() * self.params.seq_dist_amount.value(),
                        self.params.seq_lanes[5].steps[step_idx].value.value() * self.params.seq_fm_amount.value(),
                    );
                    
                    self.seq_phase = (self.seq_phase + step_rate / sample_rate).fract() * 32.0;
                }
            }

            // Render voices
            for voice in self.voices.iter_mut().flatten() {
                for i in 0..block_len {
                    let (s_gate, s_cut, s_res, s_wt, s_dist, s_fm) = seq_values[i];
                    let (l, r) = voice.render_sample(
                                    &bp, &self.factory_wavetable, self.custom_wavetable.as_ref(), sample_rate,
                                    s_gate, s_cut, s_res, s_wt, s_dist, s_fm
                                );
                    output[0][block_start + i] += l;
                    output[1][block_start + i] += r;
                }
            }

            // Global FX
            if bp.dist_enable {
                self.distortion.set_tone(bp.dist_tone);
                for i in 0..block_len {
                    output[0][block_start + i] = self.distortion.process_sample(0, output[0][block_start + i], bp.dist_drive, bp.dist_magic, bp.dist_mix);
                    output[1][block_start + i] = self.distortion.process_sample(1, output[1][block_start + i], bp.dist_drive, bp.dist_magic, bp.dist_mix);
                }
            }

            if bp.chorus_enable {
                for i in 0..block_len {
                    let (left, right) = self.chorus.process(output[0][block_start + i], output[1][block_start + i], bp.chorus_rate, bp.chorus_depth, bp.chorus_mix);
                    output[0][block_start + i] = left;
                    output[1][block_start + i] = right;
                }
            }

            if bp.delay_enable {
                for i in 0..block_len {
                    let (left, right) = self.delay.process(output[0][block_start + i], output[1][block_start + i], bp.delay_time_ms, bp.delay_feedback, bp.delay_mix);
                    output[0][block_start + i] = left;
                    output[1][block_start + i] = right;
                }
            }

            if bp.reverb_enable {
                for i in 0..block_len {
                    let (left, right) = self.reverb.process(output[0][block_start + i], output[1][block_start + i], bp.reverb_size, bp.reverb_damp, bp.reverb_diffusion, bp.reverb_shimmer, bp.reverb_mix);
                    output[0][block_start + i] = left;
                    output[1][block_start + i] = right;
                }
            }

            if bp.eq_enable {
                self.eq.set_params(bp.eq_low_gain, bp.eq_mid_gain, bp.eq_mid_freq, bp.eq_mid_q, bp.eq_high_gain);
                for i in 0..block_len {
                    output[0][block_start + i] = self.eq.process_sample(0, output[0][block_start + i]);
                    output[1][block_start + i] = self.eq.process_sample(1, output[1][block_start + i]);
                }
            }

            if bp.output_sat_enable {
                for i in 0..block_len {
                    output[0][block_start + i] = self.output_saturation.process_sample(0, output[0][block_start + i], bp.output_sat_drive, bp.output_sat_type, bp.output_sat_mix);
                    output[1][block_start + i] = self.output_saturation.process_sample(1, output[1][block_start + i], bp.output_sat_drive, bp.output_sat_type, bp.output_sat_mix);
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

            // Final gain
            for i in 0..block_len {
                output[0][block_start + i] *= bp.gain;
                output[1][block_start + i] *= bp.gain;
            }

            // Voice termination
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

        self.last_note_active = self.voices.iter().any(|v| v.is_some());
        ProcessStatus::Normal
    }
}

impl ClapPlugin for SubSynth {
    const CLAP_ID: &'static str = "art.plantsynth";
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
    const VST3_CLASS_ID: [u8; 16] = *b"PlantSynthLingA1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}
