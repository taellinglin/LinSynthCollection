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
mod drum_engine;
mod drum_model;
mod drum_params;
mod drum_sequencer;
mod midi_map;
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
use eq::ThreeBandEq;
use distortion::Distortion;
use output_saturation::{OutputSaturation, OutputSaturationType};
use drum_engine::{AuxOutput, DrumEngine};
use drum_params::{DrumSynthParams, DRUM_SLOTS, DRUM_STEPS};
use drum_sequencer::DrumSequencer;

const NUM_VOICES: usize = 16;
const MAX_BLOCK_SIZE: usize = 64;
const GAIN_POLY_MOD_ID: u32 = 0;

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
    factory_wavetable: WavetableBank,
    custom_wavetable: Option<WavetableBank>,
    custom_wavetable_path: Option<String>,
    seq_phase: f32,
    last_note_phase_delta: f32,
    last_note_active: bool,
    sample_rate: f32,
}

impl Default for SubSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(SubSynthParams::default()),

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

const MAX_BLOCK_SIZE: usize = 64;

struct BlockParams {
    gain: f32,
    waveform: Waveform,
    osc_routing: OscRouting,
    osc_blend: f32,
    wavetable_position: f32,
    wavetable_distortion: f32,
    classic_drive: f32,
    custom_wavetable_enable: bool,
    analog_enable: bool,
    analog_drive: f32,
    analog_noise: f32,
    analog_drift: f32,
    sub_level: f32,
    unison_voices: UnisonVoices,
    unison_detune: f32,
    unison_spread: f32,
    glide_mode: GlideMode,
    glide_time_ms: f32,
    filter_type: FilterType,
    filter_cut: f32,
    filter_res: f32,
    filter_amount: f32,
    filter_cut_envelope_level: f32,
    filter_res_envelope_level: f32,
    amp_envelope_level: f32,
    fm_enable: bool,
    fm_source: FmSource,
    fm_target: FmTarget,
    fm_amount: f32,
    fm_ratio: f32,
    fm_feedback: f32,
    vibrato_intensity: f32,
    vibrato_rate: f32,
    vibrato_attack: f32,
    vibrato_shape: OscillatorShape,
    tremolo_intensity: f32,
    tremolo_rate: f32,
    tremolo_attack: f32,
    tremolo_shape: OscillatorShape,
    lfo1_rate: f32,
    lfo1_attack: f32,
    lfo1_shape: OscillatorShape,
    lfo2_rate: f32,
    lfo2_attack: f32,
    lfo2_shape: OscillatorShape,
    mod1_source: ModSource,
    mod1_target: ModTarget,
    mod1_amount: f32,
    mod1_smooth_ms: f32,
    mod2_source: ModSource,
    mod2_target: ModTarget,
    mod2_amount: f32,
    mod2_smooth_ms: f32,
    mod3_source: ModSource,
    mod3_target: ModTarget,
    mod3_amount: f32,
    mod3_smooth_ms: f32,
    mod4_source: ModSource,
    mod4_target: ModTarget,
    mod4_amount: f32,
    mod4_smooth_ms: f32,
    mod5_source: ModSource,
    mod5_target: ModTarget,
    mod5_amount: f32,
    mod5_smooth_ms: f32,
    mod6_source: ModSource,
    mod6_target: ModTarget,
    mod6_amount: f32,
    mod6_smooth_ms: f32,
    seq_enable: bool,
    seq_rate: f32,
    seq_gate_amount: f32,
    seq_cut_amount: f32,
    seq_res_amount: f32,
    seq_wt_amount: f32,
    seq_dist_amount: f32,
    seq_fm_amount: f32,
    pan_lfo_rate: f32,
    pan_lfo_intensity: f32,
    pan_lfo_attack: f32,
    pan_lfo_shape: OscillatorShape,
}

impl Plugin for SubSynth {
    const NAME: &'static str = "CatSynth";
    const VENDOR: &'static str = "CatSynth";
    const URL: &'static str = "https://taellinglin.art";
    const EMAIL: &'static str = "taellinglin@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
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
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // After `PEAK_METER_DECAY_MS` milliseconds of pure silence, the peak meter's value should
        // have dropped by 12 dB
        self.sample_rate = buffer_config.sample_rate;
        self.refresh_custom_wavetable();
        true
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            *voice = None;
        }
        self.next_voice_index = 0;
        self.next_internal_voice_id = 0;
    }

    fn handle_event(
        &mut self,
        context: &mut impl ProcessContext<Self>,
        event: NoteEvent<Self>,
        this_sample_internal_voice_id_start: u64,
        sample_rate: f32,
    ) {
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
                let pitch = util::midi_note_to_freq(note) * (2.0_f32).powf((tuning + 0.0) / 12.0);
                let target_phase_delta = pitch / sample_rate;
                let glide_mode = self.params.glide_mode.value();
                let last_note_active = self.last_note_active;
                let last_note_phase_delta = self.last_note_phase_delta;
                let use_glide = match glide_mode {
                    GlideMode::Off => false,
                    GlideMode::Always => true,
                    GlideMode::Legato => last_note_active,
                };
                let start_phase_delta = if use_glide && last_note_phase_delta > 0.0 {
                    last_note_phase_delta
                } else {
                    target_phase_delta
                };

                let (amp_envelope, cutoff_envelope, resonance_envelope) =
                    self.construct_envelopes(sample_rate, velocity);
                let fm_envelope = ADSREnvelope::new(
                    self.params.fm_env_attack_ms.value(),
                    self.params.fm_env_amount.value(),
                    self.params.fm_env_decay_ms.value(),
                    self.params.fm_env_sustain_level.value(),
                    self.params.fm_env_release_ms.value(),
                    sample_rate,
                    velocity,
                    0.0,
                );
                let dist_envelope = ADSREnvelope::new(
                    self.params.dist_env_attack_ms.value(),
                    self.params.dist_env_amount.value(),
                    self.params.dist_env_decay_ms.value(),
                    self.params.dist_env_sustain_level.value(),
                    self.params.dist_env_release_ms.value(),
                    sample_rate,
                    velocity,
                    0.0,
                );

                {
                    let voice = self.start_voice(
                        context,
                        timing,
                        voice_id,
                        channel,
                        note,
                        velocity,
                        pan,
                        pressure,
                        brightness,
                        expression,
                        vibrato,
                        tuning,
                        vibrato_lfo,
                        tremolo_lfo,
                        mod_lfo1,
                        mod_lfo2,
                        amp_envelope,
                        cutoff_envelope,
                        resonance_envelope,
                        fm_envelope,
                        dist_envelope,
                        self.params.filter_type.value(),
                        sample_rate,
                    );

                    voice.vib_mod = vibrato_lfo.clone();
                    voice.trem_mod = tremolo_lfo.clone();
                    voice.pan_mod = pan_lfo.clone();
                    voice.mod_lfo1 = mod_lfo1.clone();
                    voice.mod_lfo2 = mod_lfo2.clone();
                    voice.velocity_sqrt = velocity.sqrt();
                    voice.phase = initial_phase;
                    voice.vib_mod.trigger();
                    voice.trem_mod.trigger();
                    voice.pan_mod.trigger();
                    voice.mod_lfo1.trigger();
                    voice.mod_lfo2.trigger();
                    voice.phase_delta = start_phase_delta;
                    voice.target_phase_delta = target_phase_delta;
                    voice.amp_envelope = amp_envelope;
                    voice.filter_cut_envelope = cutoff_envelope;
                    voice.filter_res_envelope = resonance_envelope;
                    voice.fm_envelope = fm_envelope;
                    voice.dist_envelope = dist_envelope;
                    voice.velocity = velocity;
                    voice.pan = pan;
                    voice.unison_phases = [initial_phase; 6];
                    voice.stereo_prev = 0.0;
                }

                self.last_note_phase_delta = target_phase_delta;
                self.last_note_active = true;
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
                if let Some(voice_idx) = self.get_voice_idx(voice_id) {
                    let voice = self.voices[voice_idx].as_mut().unwrap();

                    match poly_modulation_id {
                        GAIN_POLY_MOD_ID => {
                            let target_plain_value =
                                self.params.gain.preview_modulated(normalized_offset);
                            let (_, smoother) = voice.voice_gain.get_or_insert_with(|| {
                                (normalized_offset, self.params.gain.smoothed.clone())
                            });

                            if voice.internal_voice_id >= this_sample_internal_voice_id_start {
                                smoother.reset(target_plain_value);
                            } else {
                                smoother.set_target(sample_rate, target_plain_value);
                            }
                        }
                        n => nih_debug_assert_failure!(
                            "Polyphonic modulation sent for unknown poly modulation ID {}",
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
                for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                    match poly_modulation_id {
                        GAIN_POLY_MOD_ID => {
                            let (normalized_offset, smoother) = match voice.voice_gain.as_mut() {
                                Some((o, s)) => (o, s),
                                None => continue,
                            };
                            let target_plain_value = self
                                .params
                                .gain
                                .preview_plain(normalized_value + *normalized_offset);
                            smoother.set_target(sample_rate, target_plain_value);
                        }
                        n => nih_debug_assert_failure!(
                            "Automation event sent for unknown poly modulation ID {}",
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
                        if let Some(_voice_inner) = voice.as_mut() {
                            self.handle_poly_event(
                                timing,
                                voice_id,
                                channel,
                                note,
                                None,
                                None,
                                None,
                                None,
                                None,
                                Some(pressure),
                                None,
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
                        if let Some(_voice_inner) = voice {
                            self.handle_poly_event(
                                timing,
                                voice_id,
                                channel,
                                note,
                                Some(gain),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
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
                        if let Some(_voice_inner) = voice {
                            self.handle_poly_event(
                                timing,
                                voice_id,
                                channel,
                                note,
                                None,
                                Some(pan),
                                None,
                                None,
                                None,
                                None,
                                None,
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
                        if let Some(_voice_inner) = voice {
                            self.handle_poly_event(
                                timing,
                                voice_id,
                                channel,
                                note,
                                None,
                                None,
                                None,
                                None,
                                Some(tuning),
                                None,
                                None,
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
                        if let Some(_voice_inner) = voice {
                            self.handle_poly_event(
                                timing,
                                voice_id,
                                channel,
                                note,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                Some(vibrato),
                            );
                        }
                    }
                }
            }
            _ => (),
        };
    }

    fn render_block_voices(
        &mut self,
        block_start: usize,
        block_end: usize,
        output: &mut [&mut [f32]],
        params: &BlockParams,
        sample_rate: f32,
        tempo: f32,
        gain: &[f32],
        voice_gain: &mut [f32],
        seq_gate_values: &mut [f32],
        seq_dist_values: &mut [f32],
        dist_env_values: &mut [f32],
    ) {
        let block_len = block_end - block_start;

        for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
            let (seq_gate, seq_cut, seq_res, seq_wt, seq_dist, seq_fm) = if params.seq_enable {
                let step_rate = (tempo / 60.0) * params.seq_rate;
                let step_idx = (self.seq_phase.floor() as usize) % 32;
                let gate = self.params.seq_lanes[0].steps[step_idx].value.value();
                let cut = self.params.seq_lanes[1].steps[step_idx].value.value();
                let res = self.params.seq_lanes[2].steps[step_idx].value.value();
                let wt = self.params.seq_lanes[3].steps[step_idx].value.value();
                let dist = self.params.seq_lanes[4].steps[step_idx].value.value();
                let fm = self.params.seq_lanes[5].steps[step_idx].value.value();

                let phase_inc = step_rate / sample_rate;
                self.seq_phase += phase_inc;
                if self.seq_phase >= 32.0 {
                    self.seq_phase -= 32.0;
                }

                let gate_step = (gate * 0.5 + 0.5).clamp(0.0, 1.0);
                let gate_value = (1.0 - params.seq_gate_amount) + params.seq_gate_amount * gate_step;

                (
                    gate_value,
                    cut * params.seq_cut_amount,
                    res * params.seq_res_amount,
                    wt * params.seq_wt_amount,
                    dist * params.seq_dist_amount,
                    fm * params.seq_fm_amount,
                )
            } else {
                (1.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            };

            seq_gate_values[value_idx] = seq_gate;
            seq_dist_values[value_idx] = seq_dist;

            for voice in self.voices.iter_mut() {
                if let Some(voice) = voice {
                    let _v_gain = match &voice.voice_gain {
                        Some((_, smoother)) => {
                            smoother.next_block(voice_gain, block_len);
                            &voice_gain[value_idx]
                        }
                        None => &gain[value_idx],
                    };

                    voice.filter = Some(params.filter_type);
                    let pan_lfo = voice.pan_mod.get_modulation(sample_rate);
                    
                    if params.glide_time_ms > 0.0 {
                        let coeff = (-1.0_f32 / (params.glide_time_ms * 0.001 * sample_rate)).exp();
                        voice.phase_delta = voice.phase_delta * coeff
                            + voice.target_phase_delta * (1.0 - coeff);
                    } else {
                        voice.phase_delta = voice.target_phase_delta;
                    }

                    let vibrato_modulation = voice.vib_mod.get_modulation(sample_rate);
                    let vibrato_phase_delta =
                        voice.phase_delta * (1.0 + (params.vibrato_intensity * vibrato_modulation));
                    if params.analog_enable && params.analog_drift > 0.0 {
                        let jitter = (self.prng.gen::<f32>() - 0.5) * params.analog_drift * 0.0005;
                        voice.drift_offset = (voice.drift_offset + jitter).clamp(-0.02, 0.02);
                    }
                    let drifted_phase_delta = vibrato_phase_delta * (1.0 + voice.drift_offset);

                    voice.amp_envelope.advance();
                    voice.filter_cut_envelope.advance();
                    voice.filter_res_envelope.advance();
                    voice.fm_envelope.advance();
                    voice.dist_envelope.advance();

                    let amp_env_value = voice.amp_envelope.get_value();
                    let filter_cut_env_value = voice.filter_cut_envelope.get_value();
                    let filter_res_env_value = voice.filter_res_envelope.get_value();
                    let fm_env_value = voice.fm_envelope.get_value();
                    let dist_env_value = voice.dist_envelope.get_value();

                    let lfo1_mod = voice.mod_lfo1.get_modulation(sample_rate);
                    let lfo2_mod = voice.mod_lfo2.get_modulation(sample_rate);
                    
                    let mut wavetable_pos_mod = 0.0;
                    let mut filter_cut_mod = 0.0;
                    let mut filter_res_mod = 0.0;
                    let mut filter_amount_mod = 0.0;
                    let mut pan_mod_extra = 0.0;
                    let mut gain_mod = 0.0;
                    let mut fm_amount_mod = 0.0;
                    let mut fm_ratio_mod = 0.0;
                    let mut fm_feedback_mod = 0.0;

                    let mut apply_mod = |slot: usize, src: ModSource, tgt: ModTarget, amt: f32, smooth: f32| {
                        let source_value = match src {
                            ModSource::Lfo1 => lfo1_mod,
                            ModSource::Lfo2 => lfo2_mod,
                            ModSource::AmpEnv => amp_env_value,
                            ModSource::FilterEnv => filter_cut_env_value,
                        };
                        let mod_value = source_value * amt;
                        let mod_value = if smooth > 0.0 {
                            let coeff = (-1.0 / (smooth * 0.001 * sample_rate)).exp();
                            let prev = voice.mod_smooth[slot];
                            let smoothed = prev * coeff + mod_value * (1.0 - coeff);
                            voice.mod_smooth[slot] = smoothed;
                            smoothed
                        } else {
                            voice.mod_smooth[slot] = mod_value;
                            mod_value
                        };
                        match tgt {
                            ModTarget::WavetablePos => wavetable_pos_mod += mod_value,
                            ModTarget::FilterCut => filter_cut_mod += mod_value,
                            ModTarget::FilterRes => filter_res_mod += mod_value,
                            ModTarget::FilterAmount => filter_amount_mod += mod_value,
                            ModTarget::Pan => pan_mod_extra += mod_value,
                            ModTarget::Gain => gain_mod += mod_value,
                            ModTarget::FmAmount => fm_amount_mod += mod_value,
                            ModTarget::FmRatio => fm_ratio_mod += mod_value,
                            ModTarget::FmFeedback => fm_feedback_mod += mod_value,
                        }
                    };

                    apply_mod(0, params.mod1_source, params.mod1_target, params.mod1_amount, params.mod1_smooth_ms);
                    apply_mod(1, params.mod2_source, params.mod2_target, params.mod2_amount, params.mod2_smooth_ms);
                    apply_mod(2, params.mod3_source, params.mod3_target, params.mod3_amount, params.mod3_smooth_ms);
                    apply_mod(3, params.mod4_source, params.mod4_target, params.mod4_amount, params.mod4_smooth_ms);
                    apply_mod(4, params.mod5_source, params.mod5_target, params.mod5_amount, params.mod5_smooth_ms);
                    apply_mod(5, params.mod6_source, params.mod6_target, params.mod6_amount, params.mod6_smooth_ms);

                    wavetable_pos_mod += seq_wt;
                    filter_cut_mod += seq_cut;
                    filter_res_mod += seq_res;
                    fm_amount_mod += seq_fm;
                    fm_amount_mod += fm_env_value * params.fm_env_amount;
                    dist_env_values[value_idx] += dist_env_value * params.dist_env_amount;

                    let wavetable_position = (params.wavetable_position + wavetable_pos_mod).clamp(0.0, 1.0);
                    let pan = (voice.pan + pan_lfo + pan_mod_extra).clamp(0.0, 1.0);
                    let left_amp = (1.0 - pan).sqrt() as f32;
                    let right_amp = pan.sqrt() as f32;

                    let fm_amount = (params.fm_amount + fm_amount_mod).clamp(-1.0, 1.0);
                    let fm_ratio = (params.fm_ratio + fm_ratio_mod).clamp(0.25, 8.0);
                    let fm_feedback = (params.fm_feedback + fm_feedback_mod).clamp(0.0, 0.99);
                    
                    let base_phase = voice.unison_phases[0];
                    let fm_signal = if params.fm_enable {
                        let mod_phase = (base_phase * fm_ratio + voice.fm_feedback_state * fm_feedback).fract();
                        let mod_sample = match params.fm_source {
                            FmSource::Classic => generate_waveform(params.waveform, mod_phase),
                            FmSource::Wavetable => {
                                let bank = if params.custom_wavetable_enable {
                                    self.custom_wavetable.as_ref().unwrap_or(&self.factory_wavetable)
                                } else {
                                    &self.factory_wavetable
                                };
                                bank.sample(mod_phase, wavetable_position)
                            }
                            FmSource::Sub => (2.0 * std::f32::consts::PI * mod_phase).sin(),
                        };
                        voice.fm_feedback_state = mod_sample;
                        mod_sample * fm_amount * 0.25
                    } else {
                        voice.fm_feedback_state = 0.0;
                        0.0
                    };

                    let unison_count = match params.unison_voices {
                        UnisonVoices::One => 1,
                        UnisonVoices::Two => 2,
                        UnisonVoices::Four => 4,
                        UnisonVoices::Six => 6,
                    };
                    let detune_cents = params.unison_detune * 30.0;
                    let offsets: &[f32] = match unison_count {
                        1 => &[0.0],
                        2 => &[-0.5, 0.5],
                        4 => &[-0.75, -0.25, 0.25, 0.75],
                        _ => &[-1.0, -0.6, -0.2, 0.2, 0.6, 1.0],
                    };

                    let wavetable_bank = if params.custom_wavetable_enable {
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
                        
                        let classic_phase = if params.fm_enable && matches!(params.fm_target, FmTarget::Classic | FmTarget::Both) {
                            (phase + fm_signal).fract()
                        } else {
                            phase
                        };
                        let wavetable_phase = if params.fm_enable && matches!(params.fm_target, FmTarget::Wavetable | FmTarget::Both) {
                            (phase + fm_signal).fract()
                        } else {
                            phase
                        };

                        let mut classic_sample = generate_waveform(params.waveform, classic_phase);
                        classic_sample = SubSynth::wavefold(classic_sample, params.classic_drive);
                        classic_sample -= SubSynth::poly_blep(phase, drifted_phase_delta * ratio);

                        let mut wavetable_sample = wavetable_bank.sample(wavetable_phase, wavetable_position);
                        wavetable_sample = SubSynth::wavefold(wavetable_sample, params.wavetable_distortion);

                        classic_sum += classic_sample;
                        wavetable_sum += wavetable_sample;

                        let next_phase = phase + drifted_phase_delta * ratio;
                        voice.unison_phases[i] = if next_phase >= 1.0 { next_phase - 1.0 } else { next_phase };
                    }

                    let classic_sum = classic_sum / unison_count as f32;
                    let wavetable_sum = wavetable_sum / unison_count as f32;

                    let mut generated_sample = match params.osc_routing {
                        OscRouting::ClassicOnly => classic_sum,
                        OscRouting::WavetableOnly => wavetable_sum,
                        OscRouting::Blend => classic_sum * (1.0 - params.osc_blend) + wavetable_sum * params.osc_blend,
                    };
                    if params.sub_level > 0.0 {
                        let sub_phase = (base_phase * 0.5).fract();
                        generated_sample += (2.0 * std::f32::consts::PI * sub_phase).sin() * params.sub_level;
                    }

                    let cutoff_base = (params.filter_cut * (1.0 + filter_cut_mod)).clamp(20.0, 20000.0);
                    let resonance_base = (params.filter_res + filter_res_mod).clamp(0.0, 1.0);
                    let modulated_cutoff = (cutoff_base * (1.0 - params.filter_cut_envelope_level + params.filter_cut_envelope_level * filter_cut_env_value)).max(20.0).min(20000.0);
                    let modulated_resonance = (resonance_base * (1.0 - params.filter_res_envelope_level + params.filter_res_envelope_level * filter_res_env_value)).max(0.0).min(1.0);

                    let filtered_sample = match voice.filter.unwrap_or(FilterType::None) {
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
                        FilterType::Comb => {
                            voice.comb_filter.set_cutoff(modulated_cutoff);
                            voice.comb_filter.set_resonance(modulated_resonance);
                            voice.comb_filter.process(generated_sample)
                        }
                        FilterType::RainbowComb => {
                            voice.rainbow_comb_filter.set_cutoff(modulated_cutoff);
                            voice.rainbow_comb_filter.set_resonance(modulated_resonance);
                            voice.rainbow_comb_filter.process(generated_sample)
                        }
                        FilterType::DiodeLadderLp => {
                            voice.diode_ladder_lp_filter.set_cutoff(modulated_cutoff);
                            voice.diode_ladder_lp_filter.set_resonance(modulated_resonance);
                            voice.diode_ladder_lp_filter.process(generated_sample)
                        }
                        FilterType::DiodeLadderHp => {
                            voice.diode_ladder_hp_filter.set_cutoff(modulated_cutoff);
                            voice.diode_ladder_hp_filter.set_resonance(modulated_resonance);
                            voice.diode_ladder_hp_filter.process(generated_sample)
                        }
                        FilterType::Ms20Pair => {
                            voice.ms20_filter.set_cutoff(modulated_cutoff);
                            voice.ms20_filter.set_resonance(modulated_resonance);
                            voice.ms20_filter.process(generated_sample)
                        }
                        FilterType::FormantMorph => {
                            voice.formant_morph_filter.set_cutoff(modulated_cutoff);
                            voice.formant_morph_filter.set_resonance(modulated_resonance);
                            voice.formant_morph_filter.process(generated_sample)
                        }
                        FilterType::Phaser => {
                            voice.phaser_filter.set_cutoff(modulated_cutoff);
                            voice.phaser_filter.set_resonance(modulated_resonance);
                            voice.phaser_filter.process(generated_sample)
                        }
                        FilterType::CombAllpass => {
                            voice.comb_allpass_filter.set_cutoff(modulated_cutoff);
                            voice.comb_allpass_filter.set_resonance(modulated_resonance);
                            voice.comb_allpass_filter.process(generated_sample)
                        }
                        FilterType::BitcrushLp => {
                            voice.bitcrush_lp_filter.set_cutoff(modulated_cutoff);
                            voice.bitcrush_lp_filter.set_resonance(modulated_resonance);
                            voice.bitcrush_lp_filter.process(generated_sample)
                        }
                    };

                    let filtered_sample = if matches!(voice.filter.unwrap_or(FilterType::None), FilterType::None) {
                        filtered_sample
                    } else {
                        filter::tame_resonance(filtered_sample, modulated_resonance)
                    };

                    let filter_amount = (params.filter_amount + filter_amount_mod).clamp(0.0, 1.0);
                    let final_sample = generated_sample * (1.0 - filter_amount) + filtered_sample * filter_amount;

                    let gain_mod_val = (1.0 + gain_mod).clamp(0.0, 2.0);
                    let amp = voice.velocity_sqrt
                        * (amp_env_value * params.amp_envelope_level)
                        * 0.5
                        * (voice.trem_mod.get_modulation(sample_rate) + 1.0)
                        * gain_mod_val
                        * seq_gate;

                    let naive_waveform = final_sample;
                    let corrected_waveform = naive_waveform - SubSynth::poly_blep(voice.phase, voice.phase_delta);
                    let mut processed_sample = corrected_waveform * amp;
                    if params.analog_enable {
                        if params.analog_noise > 0.0 {
                            processed_sample += (self.prng.gen::<f32>() * 2.0 - 1.0) * params.analog_noise;
                        }
                        if params.analog_drive > 0.0 {
                            let drive = 1.0 + params.analog_drive * 6.0;
                            processed_sample = (processed_sample * drive).tanh() / drive;
                        }
                    }

                    let dc_blocked_sample = voice.dc_blocker.process(processed_sample);
                    let spread = params.unison_spread.clamp(0.0, 1.0);
                    let diff = dc_blocked_sample - voice.stereo_prev;
                    voice.stereo_prev = dc_blocked_sample;
                    let left_wide = dc_blocked_sample + diff * spread;
                    let right_wide = dc_blocked_sample - diff * spread;
                    
                    output[0][sample_idx] += left_amp * left_wide;
                    output[1][sample_idx] += right_amp * right_wide;
                    voice.phase = voice.unison_phases[0];
                }
            }
        }
    }

    fn apply_block_fx(
        &mut self,
        block_start: usize,
        block_end: usize,
        output: &mut [&mut [f32]],
        params: &BlockParams,
        sample_rate: f32,
        gain: &[f32],
        seq_dist_values: &[f32],
        dist_env_values: &[f32],
    ) {
        if params.chorus_enable {
            self.chorus.set_enabled(true);
            self.chorus.set_sample_rate(sample_rate);
            for sample_idx in block_start..block_end {
                let (left, right) = self.chorus.process(
                    output[0][sample_idx],
                    output[1][sample_idx],
                    params.chorus_rate,
                    params.chorus_depth,
                    params.chorus_mix,
                );
                output[0][sample_idx] = left;
                output[1][sample_idx] = right;
            }
        } else {
            self.chorus.set_enabled(false);
        }

        if params.multi_filter_enable {
            for sample_idx in block_start..block_end {
                let (left, right) = self.multi_filter.process(
                    output[0][sample_idx],
                    output[1][sample_idx],
                    params.multi_filter_routing,
                    params.multi_filter_a_type,
                    params.multi_filter_a_cut,
                    params.multi_filter_a_res,
                    params.multi_filter_a_amt,
                    params.multi_filter_b_type,
                    params.multi_filter_b_cut,
                    params.multi_filter_b_res,
                    params.multi_filter_b_amt,
                    params.multi_filter_c_type,
                    params.multi_filter_c_cut,
                    params.multi_filter_c_res,
                    params.multi_filter_c_amt,
                    params.multi_filter_morph,
                    params.multi_filter_parallel_ab,
                    params.multi_filter_parallel_c,
                );
                output[0][sample_idx] = left;
                output[1][sample_idx] = right;
            }
        }

        if params.dist_enable {
            self.distortion.set_tone(params.dist_tone);
            for sample_idx in block_start..block_end {
                let value_idx = sample_idx - block_start;
                let dist_drive = (params.dist_drive + seq_dist_values[value_idx] + dist_env_values[value_idx]).clamp(0.0, 1.0);
                let left = self.distortion.process_sample(0, output[0][sample_idx], dist_drive, params.dist_magic, params.dist_mix);
                let right = self.distortion.process_sample(1, output[1][sample_idx], dist_drive, params.dist_magic, params.dist_mix);
                output[0][sample_idx] = left;
                output[1][sample_idx] = right;
            }
        }

        if params.eq_enable {
            self.eq.set_params(params.eq_low_gain, params.eq_mid_gain, params.eq_mid_freq, params.eq_mid_q, params.eq_high_gain);
            for sample_idx in block_start..block_end {
                let left = self.eq.process_sample(0, output[0][sample_idx]);
                let right = self.eq.process_sample(1, output[1][sample_idx]);
                output[0][sample_idx] = output[0][sample_idx] * (1.0 - params.eq_mix) + left * params.eq_mix;
                output[1][sample_idx] = output[1][sample_idx] * (1.0 - params.eq_mix) + right * params.eq_mix;
            }
        }

        if params.delay_enable {
            for sample_idx in block_start..block_end {
                let (left, right) = self.delay.process(
                    output[0][sample_idx],
                    output[1][sample_idx],
                    params.delay_time_ms,
                    params.delay_feedback,
                    params.delay_mix,
                );
                output[0][sample_idx] = left;
                output[1][sample_idx] = right;
            }
        }

        if params.reverb_enable {
            for sample_idx in block_start..block_end {
                let (left, right) = self.reverb.process(
                    output[0][sample_idx],
                    output[1][sample_idx],
                    params.reverb_size,
                    params.reverb_damp,
                    params.reverb_diffusion,
                    params.reverb_shimmer,
                    params.reverb_mix,
                );
                output[0][sample_idx] = left;
                output[1][sample_idx] = right;
            }
        }

        if params.output_sat_enable {
            for sample_idx in block_start..block_end {
                let left = self.output_saturation.process_sample(0, output[0][sample_idx], params.output_sat_drive, params.output_sat_type, params.output_sat_mix);
                let right = self.output_saturation.process_sample(1, output[1][sample_idx], params.output_sat_drive, params.output_sat_type, params.output_sat_mix);
                output[0][sample_idx] = left;
                output[1][sample_idx] = right;
            }
        }

        self.limiter_left.set_enabled(params.limiter_enable);
        self.limiter_right.set_enabled(params.limiter_enable);
        self.limiter_left.set_threshold(params.limiter_threshold);
        self.limiter_right.set_threshold(params.limiter_threshold);
        self.limiter_left.set_release(params.limiter_release);
        self.limiter_right.set_release(params.limiter_release);

        if params.limiter_enable {
            for sample_idx in block_start..block_end {
                output[0][sample_idx] = self.limiter_left.process(output[0][sample_idx]);
                output[1][sample_idx] = self.limiter_right.process(output[1][sample_idx]);
            }
        }

        for (value_idx, sample_idx) in (block_start..block_end).enumerate() {
            output[0][sample_idx] *= gain[value_idx];
            output[1][sample_idx] *= gain[value_idx];
        }
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
        if output.len() < 2 {
            for channel in output.iter_mut() {
                channel.fill(0.0);
            }
            return ProcessStatus::Normal;
        }

        self.delay.set_sample_rate(sample_rate);
        self.reverb.set_sample_rate(sample_rate);
        self.multi_filter.set_sample_rate(sample_rate);
        self.distortion.set_sample_rate(sample_rate);
        self.eq.set_sample_rate(sample_rate);
        self.output_saturation.set_sample_rate(sample_rate);
        self.refresh_custom_wavetable();

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);
        while block_start < num_samples {
            let this_sample_internal_voice_id_start = self.next_internal_voice_id;
            'events: loop {
                match next_event {
                    Some(event) if (event.timing() as usize) < block_end => {
                        self.handle_event(context, event, this_sample_internal_voice_id_start, sample_rate);
                        next_event = context.next_event();
                    }
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            output[0][block_start..block_end].fill(0.0);
            output[1][block_start..block_end].fill(0.0);

            let block_len = block_end - block_start;
            let mut gain = [0.0; MAX_BLOCK_SIZE];
            let mut voice_gain = [0.0; MAX_BLOCK_SIZE];
            let mut seq_gate_values = [1.0; MAX_BLOCK_SIZE];
            let mut seq_dist_values = [0.0; MAX_BLOCK_SIZE];
            let mut dist_env_values = [0.0; MAX_BLOCK_SIZE];
            self.params.gain.smoothed.next_block(&mut gain, block_len);

            let block_params = BlockParams {
                gain: self.params.gain.value(),
                waveform: self.params.waveform.value(),
                osc_routing: self.params.osc_routing.value(),
                osc_blend: self.params.osc_blend.value(),
                wavetable_position: self.params.wavetable_position.value(),
                wavetable_distortion: self.params.wavetable_distortion.value(),
                classic_drive: self.params.classic_drive.value(),
                custom_wavetable_enable: self.params.custom_wavetable_enable.value(),
                analog_enable: self.params.analog_enable.value(),
                analog_drive: self.params.analog_drive.value(),
                analog_noise: self.params.analog_noise.value(),
                analog_drift: self.params.analog_drift.value(),
                sub_level: self.params.sub_level.value(),
                unison_voices: self.params.unison_voices.value(),
                unison_detune: self.params.unison_detune.value(),
                unison_spread: self.params.unison_spread.value(),
                glide_mode: self.params.glide_mode.value(),
                glide_time_ms: self.params.glide_time_ms.value(),
                filter_type: self.params.filter_type.value(),
                filter_cut: self.params.filter_cut.value(),
                filter_res: self.params.filter_res.value(),
                filter_amount: self.params.filter_amount.value(),
                filter_cut_envelope_level: self.params.filter_cut_envelope_level.value(),
                filter_res_envelope_level: self.params.filter_res_envelope_level.value(),
                amp_envelope_level: self.params.amp_envelope_level.value(),
                fm_enable: self.params.fm_enable.value(),
                fm_source: self.params.fm_source.value(),
                fm_target: self.params.fm_target.value(),
                fm_amount: self.params.fm_amount.value(),
                fm_ratio: self.params.fm_ratio.value(),
                fm_feedback: self.params.fm_feedback.value(),
                vibrato_intensity: self.params.vibrato_intensity.value(),
                vibrato_rate: self.params.vibrato_rate.value(),
                vibrato_attack: self.params.vibrato_attack.value(),
                vibrato_shape: self.params.vibrato_shape.value(),
                tremolo_intensity: self.params.tremolo_intensity.value(),
                tremolo_rate: self.params.tremolo_rate.value(),
                tremolo_attack: self.params.tremolo_attack.value(),
                tremolo_shape: self.params.tremolo_shape.value(),
                lfo1_rate: self.params.lfo1_rate.value(),
                lfo1_attack: self.params.lfo1_attack.value(),
                lfo1_shape: self.params.lfo1_shape.value(),
                lfo2_rate: self.params.lfo2_rate.value(),
                lfo2_attack: self.params.lfo2_attack.value(),
                lfo2_shape: self.params.lfo2_shape.value(),
                mod1_source: self.params.mod1_source.value(),
                mod1_target: self.params.mod1_target.value(),
                mod1_amount: self.params.mod1_amount.value(),
                mod1_smooth_ms: self.params.mod1_smooth_ms.value(),
                mod2_source: self.params.mod2_source.value(),
                mod2_target: self.params.mod2_target.value(),
                mod2_amount: self.params.mod2_amount.value(),
                mod2_smooth_ms: self.params.mod2_smooth_ms.value(),
                mod3_source: self.params.mod3_source.value(),
                mod3_target: self.params.mod3_target.value(),
                mod3_amount: self.params.mod3_amount.value(),
                mod3_smooth_ms: self.params.mod3_smooth_ms.value(),
                mod4_source: self.params.mod4_source.value(),
                mod4_target: self.params.mod4_target.value(),
                mod4_amount: self.params.mod4_amount.value(),
                mod4_smooth_ms: self.params.mod4_smooth_ms.value(),
                mod5_source: self.params.mod5_source.value(),
                mod5_target: self.params.mod5_target.value(),
                mod5_amount: self.params.mod5_amount.value(),
                mod5_smooth_ms: self.params.mod5_smooth_ms.value(),
                mod6_source: self.params.mod6_source.value(),
                mod6_target: self.params.mod6_target.value(),
                mod6_amount: self.params.mod6_amount.value(),
                mod6_smooth_ms: self.params.mod6_smooth_ms.value(),
                seq_enable: self.params.seq_enable.value(),
                seq_rate: self.params.seq_rate.value(),
                seq_gate_amount: self.params.seq_gate_amount.value(),
                seq_cut_amount: self.params.seq_cut_amount.value(),
                seq_res_amount: self.params.seq_res_amount.value(),
                seq_wt_amount: self.params.seq_wt_amount.value(),
                seq_dist_amount: self.params.seq_dist_amount.value(),
                seq_fm_amount: self.params.seq_fm_amount.value(),
                pan_lfo_rate: self.params.pan_lfo_rate.value(),
                pan_lfo_intensity: self.params.pan_lfo_intensity.value(),
                pan_lfo_attack: self.params.pan_lfo_attack.value(),
                pan_lfo_shape: self.params.pan_lfo_shape.value(),
                
                chorus_enable: self.params.chorus_enable.value(),
                chorus_rate: self.params.chorus_rate.value(),
                chorus_depth: self.params.chorus_depth.value(),
                chorus_mix: self.params.chorus_mix.value(),
                multi_filter_enable: self.params.multi_filter_enable.value(),
                multi_filter_routing: self.params.multi_filter_routing.value(),
                multi_filter_morph: self.params.multi_filter_morph.value(),
                multi_filter_parallel_ab: self.params.multi_filter_parallel_ab.value(),
                multi_filter_parallel_c: self.params.multi_filter_parallel_c.value(),
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
                dist_enable: self.params.dist_enable.value(),
                dist_drive: self.params.dist_drive.value(),
                dist_tone: self.params.dist_tone.value(),
                dist_magic: self.params.dist_magic.value(),
                dist_mix: self.params.dist_mix.value(),
                eq_enable: self.params.eq_enable.value(),
                eq_low_gain: self.params.eq_low_gain.value(),
                eq_mid_gain: self.params.eq_mid_gain.value(),
                eq_mid_freq: self.params.eq_mid_freq.value(),
                eq_mid_q: self.params.eq_mid_q.value(),
                eq_high_gain: self.params.eq_high_gain.value(),
                eq_mix: self.params.eq_mix.value(),
                delay_enable: self.params.delay_enable.value(),
                delay_time_ms: self.params.delay_time_ms.value(),
                delay_feedback: self.params.delay_feedback.value(),
                delay_mix: self.params.delay_mix.value(),
                reverb_enable: self.params.reverb_enable.value(),
                reverb_size: self.params.reverb_size.value(),
                reverb_damp: self.params.reverb_damp.value(),
                reverb_diffusion: self.params.reverb_diffusion.value(),
                reverb_shimmer: self.params.reverb_shimmer.value(),
                reverb_mix: self.params.reverb_mix.value(),
                output_sat_enable: self.params.output_sat_enable.value(),
                output_sat_type: self.params.output_sat_type.value(),
                output_sat_drive: self.params.output_sat_drive.value(),
                output_sat_mix: self.params.output_sat_mix.value(),
                limiter_enable: self.params.limiter_enable.value(),
                limiter_threshold: self.params.limiter_threshold.value(),
                limiter_release: self.params.limiter_release.value(),
            };

            let tempo = context.transport().tempo.unwrap_or(120.0) as f32;
            let (l_slice, r_slice) = output.split_at_mut(1);
            let mut out_block = [l_slice[0], r_slice[0]];

            self.render_block_voices(
                block_start,
                block_end,
                &mut out_block,
                &block_params,
                sample_rate,
                tempo,
                &gain,
                &mut voice_gain,
                &mut seq_gate_values,
                &mut seq_dist_values,
                &mut dist_env_values,
            );

            self.apply_block_fx(
                block_start,
                block_end,
                &mut out_block,
                &block_params,
                sample_rate,
                &gain,
                &seq_dist_values,
                &dist_env_values,
            );

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
            self.last_note_active = self.voices.iter().any(|v| v.is_some());

            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl SubSynth {
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
                self.params.amp_envelope_level.value(),
                self.params.amp_decay_ms.value(),
                self.params.amp_sustain_level.value(),
                self.params.amp_release_ms.value(),
                sample_rate,
                velocity,
                self.params.amp_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_cut_attack_ms.value(),
                self.params.filter_cut_envelope_level.value(),
                self.params.filter_cut_decay_ms.value(),
                self.params.filter_cut_sustain_ms.value(),
                self.params.filter_cut_release_ms.value(),
                sample_rate,
                velocity,
                self.params.filter_cut_tension.value(),
            ),
            ADSREnvelope::new(
                self.params.filter_res_attack_ms.value(),
                self.params.filter_res_envelope_level.value(),
                self.params.filter_res_decay_ms.value(),
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
                self.params.fm_env_amount.value(),
                self.params.fm_env_decay_ms.value(),
                self.params.fm_env_sustain_level.value(),
                self.params.fm_env_release_ms.value(),
                self.sample_rate,
                1.0,
                0.0,
            ),
            dist_envelope: ADSREnvelope::new(
                self.params.dist_env_attack_ms.value(),
                self.params.dist_env_amount.value(),
                self.params.dist_env_decay_ms.value(),
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
                break;
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
        gain: Option<f32>,
        pan: Option<f32>,
        brightness: Option<f32>,
        expression: Option<f32>,
        tuning: Option<f32>,
        pressure: Option<f32>,
        vibrato: Option<f32>,
    ) {
        let existing_index = self.voices.iter().position(|voice| {
            voice
                .as_ref()
                .map(|voice_ref| {
                    voice_id == Some(voice_ref.voice_id)
                        || (voice_ref.channel == channel && voice_ref.note == note)
                })
                .unwrap_or(false)
        });

        let Some(existing_index) = existing_index else {
            return;
        };

        let Some(voice) = self.voices[existing_index].as_mut() else {
            return;
        };

        if let Some(gain) = gain {
            voice.velocity = gain;
            voice.velocity_sqrt = gain.sqrt();
            voice.amp_envelope.set_velocity(gain);
        }
        if let Some(pan) = pan {
            voice.pan = pan;
        }
        if let Some(brightness) = brightness {
            voice.brightness = brightness;
        }
        if let Some(expression) = expression {
            voice.expression = expression;
        }
        if let Some(tuning) = tuning {
            voice.tuning = tuning;
        }
        if let Some(pressure) = pressure {
            voice.pressure = pressure;
        }
        if let Some(vibrato) = vibrato {
            voice.vibrato = vibrato;
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

        let drive = 1.0 + amount * 8.0;
        let x = sample * drive;
        let mut folded = (x + 1.0).rem_euclid(4.0);
        if folded > 2.0 {
            folded = 4.0 - folded;
        }
        folded - 1.0
    }
}

impl ClapPlugin for SubSynth {
    const CLAP_ID: &'static str = "art.catsynth";
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
    const VST3_CLASS_ID: [u8; 16] = *b"CatSynthLing1A01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

struct DrumBlockParams {
    master_gain: f32,
    master_drive: f32,
    master_comp: f32,
    master_clip: f32,
    kit_preset: DrumOrganicKitPreset,
    seq_enabled: bool,
    seq_rate: f32,
    seq_swing: f32,
}


struct DrumSynth {
    params: Arc<DrumSynthParams>,
    engine: DrumEngine,
    sequencer: DrumSequencer,
    pad_triggers: [f32; DRUM_SLOTS],
    comp_env: f32,
}

const DRUM_AUX_OUTPUT_COUNT: usize = 16;
const DRUM_AUX_OUTPUT_PORTS: [NonZeroU32; DRUM_AUX_OUTPUT_COUNT] =
    [new_nonzero_u32(2); DRUM_AUX_OUTPUT_COUNT];
const DRUM_AUX_OUTPUT_NAMES: [&str; DRUM_AUX_OUTPUT_COUNT] = [
    "Pad 01",
    "Pad 02",
    "Pad 03",
    "Pad 04",
    "Pad 05",
    "Pad 06",
    "Pad 07",
    "Pad 08",
    "Pad 09",
    "Pad 10",
    "Pad 11",
    "Pad 12",
    "Pad 13",
    "Pad 14",
    "Pad 15",
    "Pad 16",
];

fn collect_aux_outputs<'a>(
    aux: &'a mut AuxiliaryBuffers,
    range: std::ops::Range<usize>,
) -> Vec<AuxOutput<'a>> {
    if aux.outputs.is_empty() {
        return Vec::new();
    }
    let mut outputs = Vec::with_capacity(aux.outputs.len());
    for output in aux.outputs.iter_mut() {
        let channels = output.as_slice();
        if channels.len() < 2 {
            continue;
        }
        let (left_channels, right_channels) = channels.split_at_mut(1);
        let max_len = left_channels[0].len().min(right_channels[0].len());
        if range.start >= max_len {
            continue;
        }
        let end = range.end.min(max_len);
        outputs.push(AuxOutput {
            left: &mut left_channels[0][range.start..end],
            right: &mut right_channels[0][range.start..end],
        });
    }
    outputs
}

impl Default for DrumSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(DrumSynthParams::default()),
            engine: DrumEngine::new(),
            sequencer: DrumSequencer::new(),
            pad_triggers: [0.0; DRUM_SLOTS],
            comp_env: 0.0,
        }
    }
}

impl Plugin for DrumSynth {
    const NAME: &'static str = "CatSynth Drums";
    const VENDOR: &'static str = "CatSynth";
    const URL: &'static str = "https://taellinglin.art";
    const EMAIL: &'static str = "taellinglin@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        aux_output_ports: &DRUM_AUX_OUTPUT_PORTS,
        names: PortNames {
            main_output: Some("Main"),
            aux_outputs: &DRUM_AUX_OUTPUT_NAMES,
            ..PortNames::const_default()
        },
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
        editor::create_drum(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        self.sequencer.reset();
        true
    }

    fn reset(&mut self) {
        self.sequencer.reset();
        self.comp_env = 0.0;
    }

    fn handle_event(
        &mut self,
        _context: &mut impl ProcessContext<Self>,
        event: NoteEvent<Self>,
        kit: DrumOrganicKitPreset,
    ) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } => {
                if let Some(slot) = self
                    .params
                    .slots
                    .iter()
                    .position(|slot| slot.midi_note.value() as u8 == note)
                {
                    let slot_params = &self.params.slots[slot];
                    self.engine.trigger(slot, slot_params, velocity, kit);
                }
            }
            _ => {}
        }
    }

    fn handle_pad_triggers(&mut self, kit: DrumOrganicKitPreset) {
        for (slot, slot_params) in self.params.slots.iter().enumerate() {
            let current = slot_params.pad_trigger.value();
            if (current - self.pad_triggers[slot]).abs() > 1.0e-5 {
                self.pad_triggers[slot] = current;
                self.engine.trigger(slot, slot_params, 1.0, kit);
            }
        }
    }

    fn render_block_drums(
        &mut self,
        block_start: usize,
        block_end: usize,
        left: &mut [f32],
        right: &mut [f32],
        aux: &mut AuxiliaryBuffers,
        params: &DrumBlockParams,
        tempo: f32,
        sample_rate: f32,
    ) {
        let block_left = &mut left[block_start..block_end];
        let block_right = &mut right[block_start..block_end];

        if params.seq_enabled {
            let mut segment_start = 0usize;
            let num_samples = block_left.len();
            for sample_idx in 0..num_samples {
                if let Some(step) = self.sequencer.advance(sample_rate, tempo, params.seq_rate, params.seq_swing) {
                    if sample_idx > segment_start {
                        let range = (block_start + segment_start)..(block_start + sample_idx);
                        let mut aux_outputs = collect_aux_outputs(aux, range.clone());
                        let aux_slice = if aux_outputs.is_empty() { None } else { Some(aux_outputs.as_mut_slice()) };
                        self.engine.process(&mut left[range.clone()], &mut right[range], aux_slice);
                    }
                    let step_index = step.min(DRUM_STEPS - 1);
                    for slot in 0..DRUM_SLOTS {
                        let lane = &self.params.sequencer.lanes[slot];
                        let step_params = &lane.steps[step_index];
                        if step_params.gate.value() {
                            let slot_params = &self.params.slots[slot];
                            let velocity = step_params.velocity.value().clamp(0.0, 1.0);
                            self.engine.trigger(slot, slot_params, velocity, params.kit_preset);
                        }
                    }
                    segment_start = sample_idx;
                }
            }
            if segment_start < num_samples {
                let range = (block_start + segment_start)..(block_start + num_samples);
                let mut aux_outputs = collect_aux_outputs(aux, range.clone());
                let aux_slice = if aux_outputs.is_empty() { None } else { Some(aux_outputs.as_mut_slice()) };
                self.engine.process(&mut left[range.clone()], &mut right[range], aux_slice);
            }
        } else {
            let range = block_start..block_end;
            let mut aux_outputs = collect_aux_outputs(aux, range.clone());
            let aux_slice = if aux_outputs.is_empty() { None } else { Some(aux_outputs.as_mut_slice()) };
            self.engine.process(block_left, block_right, aux_slice);
        }
    }

    fn apply_block_fx(
        &mut self,
        block_start: usize,
        block_end: usize,
        left: &mut [f32],
        right: &mut [f32],
        params: &DrumBlockParams,
        sample_rate: f32,
    ) {
        if params.master_gain != 1.0 || params.master_drive > 0.0 || params.master_comp > 0.0 || params.master_clip > 0.0 {
            let drive_amount = 1.0 + params.master_drive * 6.0;
            let clip_amount = 1.0 + params.master_clip * 10.0;
            let threshold_db = -18.0 + params.master_comp * 10.0;
            let threshold = util::db_to_gain(threshold_db);
            let ratio = 1.5 + params.master_comp * 5.0;
            let attack = (-1.0 / (0.005 * sample_rate)).exp();
            let release = (-1.0 / (0.08 * sample_rate)).exp();

            let block_left = &mut left[block_start..block_end];
            let block_right = &mut right[block_start..block_end];

            for idx in 0..block_left.len() {
                let mut left_sample = block_left[idx] * params.master_gain;
                let mut right_sample = block_right[idx] * params.master_gain;

                if params.master_comp > 0.0 {
                    let detector = left_sample.abs().max(right_sample.abs());
                    if detector > self.comp_env {
                        self.comp_env = self.comp_env * attack + detector * (1.0 - attack);
                    } else {
                        self.comp_env = self.comp_env * release + detector * (1.0 - release);
                    }
                    if self.comp_env > threshold {
                        let gain = (threshold + (self.comp_env - threshold) / ratio) / self.comp_env;
                        left_sample *= gain;
                        right_sample *= gain;
                    }
                }

                if params.master_drive > 0.0 {
                    left_sample = (left_sample * drive_amount).tanh() / drive_amount;
                    right_sample = (right_sample * drive_amount).tanh() / drive_amount;
                }

                if params.master_clip > 0.0 {
                    left_sample = (left_sample * clip_amount).tanh() / clip_amount;
                    right_sample = (right_sample * clip_amount).tanh() / clip_amount;
                }

                block_left[idx] = left_sample;
                block_right[idx] = right_sample;
            }
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();
        let sample_rate = context.transport().sample_rate;
        let output = buffer.as_slice();
        if output.len() < 2 {
            for channel in output.iter_mut() {
                channel.fill(0.0);
            }
            return ProcessStatus::Normal;
        }

        let (left_slice, right_slice) = output.split_at_mut(1);
        let left = &mut left_slice[0];
        let right = &mut right_slice[0];
        left.fill(0.0);
        right.fill(0.0);

        if !aux.outputs.is_empty() {
            for output in aux.outputs.iter_mut() {
                for channel in output.as_slice().iter_mut() {
                    channel.fill(0.0);
                }
            }
        }

        let kit = self.params.kit_preset.value();
        self.handle_pad_triggers(kit);

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        let mut block_end: usize = MAX_BLOCK_SIZE.min(num_samples);

        while block_start < num_samples {
            'events: loop {
                match next_event {
                    Some(event) if (event.timing() as usize) < block_end => {
                        self.handle_event(context, event, kit);
                        next_event = context.next_event();
                    }
                    Some(event) if (event.timing() as usize) < block_end => {
                        block_end = event.timing() as usize;
                        break 'events;
                    }
                    _ => break 'events,
                }
            }

            let block_params = DrumBlockParams {
                master_gain: self.params.master_gain.value().clamp(0.0, 1.0),
                master_drive: self.params.master_drive.value().clamp(0.0, 1.0),
                master_comp: self.params.master_comp.value().clamp(0.0, 1.0),
                master_clip: self.params.master_clip.value().clamp(0.0, 1.0),
                kit_preset: kit,
                seq_enabled: self.params.sequencer.enabled.value(),
                seq_rate: self.params.sequencer.rate.value(),
                seq_swing: self.params.sequencer.swing.value(),
            };

            let tempo = context.transport().tempo.unwrap_or(120.0) as f32;

            self.render_block_drums(
                block_start,
                block_end,
                left,
                right,
                aux,
                &block_params,
                tempo,
                sample_rate,
            );

            self.apply_block_fx(
                block_start,
                block_end,
                left,
                right,
                &block_params,
                sample_rate,
            );

            block_start = block_end;
            block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for DrumSynth {
    const CLAP_ID: &'static str = "art.catsynth.drums";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Physical modeling drum synth with 16-slot sequencer");
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

nih_export_clap!(DrumSynth);
nih_export_vst3!(DrumSynth);
