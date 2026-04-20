pub mod params;
pub mod voice;

mod chorus;
mod delay;
mod distortion;
mod editor;
mod envelope;
mod eq;
mod filter;
mod limiter;
mod modulator;
mod multi_filter;
mod output_saturation;
mod reverb;
mod waveform;

use nih_plug::prelude::*;
use rand::Rng;
use rand_pcg::Pcg32;
use std::sync::Arc;

use modulator::Modulator;
use chorus::Chorus;
use delay::StereoDelay;
use envelope::{ADSREnvelope, ADSREnvelopeState};
use filter::FilterType;
use limiter::Limiter;
use multi_filter::MultiStageFilter;
use waveform::{generate_additive_sample_advanced, generate_waveform, WavetableBank, Waveform};
use eq::ThreeBandEq;
use distortion::Distortion;
use output_saturation::{OutputSaturation, OutputSaturationType};

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
    factory_presets: Vec<editor::PresetEntry>,
    current_preset_index: usize,
    last_preset_param: i32,
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
    morph_a_snapshot: Option<editor::PresetData>,
    morph_b_snapshot: Option<editor::PresetData>,
    morph_a_json: Option<String>,
    morph_b_json: Option<String>,
    morph_last_amount: f32,
    seq_phase: f32,
    last_note_phase_delta: f32,
    last_note_active: bool,
    sample_rate: f32,
}

impl Default for SubSynth {
    fn default() -> Self {
        let params = Arc::new(SubSynthParams::default());
        let factory_presets = editor::load_presets(&params);
        let last_preset_param = params.preset_index.value();
        Self {
            params,
            prng: Pcg32::new(420, 1337),
            voices: [0; NUM_VOICES as usize].map(|_| None),
            next_internal_voice_id: 0,
            next_voice_index: 0,
            factory_presets,
            current_preset_index: 0,
            last_preset_param,
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
            morph_a_snapshot: None,
            morph_b_snapshot: None,
            morph_a_json: None,
            morph_b_json: None,
            morph_last_amount: -1.0,
            seq_phase: 0.0,
            last_note_phase_delta: 0.0,
            last_note_active: false,
            sample_rate: 44100.0,
        }
    }
}

impl Plugin for SubSynth {
    const NAME: &'static str = "MiceSynth";
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

    fn params(&self) -> Arc<dyn Params> { self.params.clone() }
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, _buffer_config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = _buffer_config.sample_rate;
        self.refresh_custom_wavetable();
        self.apply_preset(self.params.preset_index.value() as usize, _buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.prng = Pcg32::new(420, 1337);
        self.voices.fill(None);
        self.next_internal_voice_id = 0;
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

        // 1. Housekeeping
        let preset_param = self.params.preset_index.value();
        if preset_param != self.last_preset_param {
            self.apply_preset(preset_param as usize, sample_rate);
        }
        self.update_module_sample_rates(sample_rate);
        self.refresh_custom_wavetable();
        
        let morph_changed = self.refresh_morph_snapshots();
        let morph_amount = self.params.morph_amount.value().clamp(0.0, 1.0);
        if let (Some(a), Some(b)) = (&self.morph_a_snapshot, &self.morph_b_snapshot) {
            if morph_changed || (morph_amount - self.morph_last_amount).abs() > 0.0005 {
                let blended = editor::PresetData::lerp(a, b, morph_amount);
                blended.apply_direct(&self.params);
                self.morph_last_amount = morph_amount;
            }
        }

        let mut next_event = context.next_event();
        let mut block_start: usize = 0;
        
        while block_start < num_samples {
            let block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);
            let mut this_block_end = block_end;

            if let Some(event) = next_event {
                if (event.timing() as usize) < this_block_end {
                    this_block_end = (event.timing() as usize).max(block_start);
                }
            }

            let this_sample_internal_voice_id_start = self.next_internal_voice_id;
            while let Some(event) = next_event {
                if (event.timing() as usize) <= block_start {
                    self.handle_event(event, context, sample_rate, this_sample_internal_voice_id_start);
                    next_event = context.next_event();
                } else { break; }
            }

            let block_len = this_block_end - block_start;
            if block_len > 0 {
                let p = self.get_block_params();
                output[0][block_start..this_block_end].fill(0.0);
                output[1][block_start..this_block_end].fill(0.0);

                let mut fx_left = [0.0f32; MAX_BLOCK_SIZE];
                let mut fx_right = [0.0f32; MAX_BLOCK_SIZE];

                self.render_block_voices(&mut fx_left, &mut fx_right, block_start, this_block_end, output, sample_rate, &p, context);
                self.apply_block_fx(output, &mut fx_left, &mut fx_right, block_start, this_block_end, sample_rate, &p);
                self.terminate_finished_voices(context, this_block_end as u32);
            }
            block_start = this_block_end;
        }

        self.last_note_active = self.voices.iter().any(|v| v.is_some());
        ProcessStatus::Normal
    }
}

pub(crate) struct BlockParams {
    waveform: Waveform,
    osc_routing: OscRouting,
    osc_blend: f32,
    wavetable_position: f32,
    wavetable_distortion: f32,
    classic_drive: f32,
    additive_mix: f32,
    additive_partials: f32,
    additive_tilt: f32,
    additive_inharm: f32,
    additive_morph: f32,
    additive_decay: f32,
    additive_drift: f32,
    custom_wavetable_enable: bool,
    analog_enable: bool,
    analog_drive: f32,
    analog_noise: f32,
    analog_drift: f32,
    breath_enable: bool,
    breath_amount: f32,
    breath_tone: f32,
    sub_level: f32,
    unison_voices: UnisonVoices,
    unison_detune: f32,
    unison_spread: f32,
    glide_time: f32,
    vibrato_intensity: f32,
    filter_type: FilterType,
    filter_cut: f32,
    filter_res: f32,
    filter_amount: f32,
    amp_env_level: f32,
    fm_enable: bool,
    fm_source: FmSource,
    fm_target: FmTarget,
    fm_amount: f32,
    fm_ratio: f32,
    fm_feedback: f32,
    fm_env_amount: f32,
    dist_env_amount: f32,
    chorus_enable: bool,
    chorus_rate: f32,
    chorus_depth: f32,
    chorus_mix: f32,
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
    dist_enable: bool,
    dist_drive: f32,
    dist_tone: f32,
    dist_magic: f32,
    dist_mix: f32,
    eq_enable: bool,
    eq_low_gain: f32,
    eq_mid_gain: f32,
    eq_mid_freq: f32,
    eq_mid_q: f32,
    eq_high_gain: f32,
    eq_mix: f32,
    output_sat_enable: bool,
    output_sat_type: OutputSaturationType,
    output_sat_drive: f32,
    output_sat_mix: f32,
    multi_filter_enable: bool,
    multi_filter_routing: FilterRouting,
    multi_filter_morph: f32,
    multi_filter_parallel_ab: f32,
    multi_filter_parallel_c: f32,
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
    }

    fn handle_event(&mut self, event: NoteEvent<()>, context: &mut impl ProcessContext<Self>, sample_rate: f32, this_sample_internal_voice_id_start: u64) {
        match event {
            NoteEvent::NoteOn { timing, voice_id, channel, note, velocity } => {
                let initial_phase: f32 = self.prng.gen();
                let (amp_envelope, cutoff_envelope, resonance_envelope) = self.construct_envelopes(sample_rate, velocity);
                let fm_envelope = ADSREnvelope::new(Self::ms_to_s(self.params.fm_env_attack_ms.value()), self.params.fm_env_amount.value(), Self::ms_to_s(self.params.fm_env_decay_ms.value()), self.params.fm_env_sustain_level.value(), Self::ms_to_s(self.params.fm_env_release_ms.value()), sample_rate, velocity, 0.0);
                let dist_envelope = ADSREnvelope::new(Self::ms_to_s(self.params.dist_env_attack_ms.value()), self.params.dist_env_amount.value(), Self::ms_to_s(self.params.dist_env_decay_ms.value()), self.params.dist_env_sustain_level.value(), Self::ms_to_s(self.params.dist_env_release_ms.value()), sample_rate, velocity, 0.0);
                let breath_envelope = self.construct_breath_envelope(sample_rate, velocity);

                let voice = self.start_voice(context, timing, voice_id, channel, note, velocity, 0.5, 1.0, 1.0, 1.0, 0.0, 0.0, 
                    Modulator::new(self.params.vibrato_rate.value(), self.params.vibrato_intensity.value(), self.params.vibrato_attack.value(), self.params.vibrato_shape.value()),
                    Modulator::new(self.params.tremolo_rate.value(), self.params.tremolo_intensity.value(), self.params.tremolo_attack.value(), self.params.tremolo_shape.value()),
                    Modulator::new(self.params.lfo1_rate.value(), 1.0, self.params.lfo1_attack.value(), self.params.lfo1_shape.value()),
                    Modulator::new(self.params.lfo2_rate.value(), 1.0, self.params.lfo2_attack.value(), self.params.lfo2_shape.value()),
                    amp_envelope, cutoff_envelope, resonance_envelope, fm_envelope, dist_envelope, breath_envelope, self.params.filter_type.value(), sample_rate);
                
                voice.phase = initial_phase;
                voice.unison_phases = [initial_phase; 6];
                
                let pitch = util::midi_note_to_freq(note);
                let target_phase_delta = pitch / sample_rate;
                let use_glide = match self.params.glide_mode.value() {
                    GlideMode::Off => false,
                    GlideMode::Always => true,
                    GlideMode::Legato => self.last_note_active,
                };
                voice.phase_delta = if use_glide && self.last_note_active { self.last_note_phase_delta } else { target_phase_delta };
                voice.target_phase_delta = target_phase_delta;
                self.last_note_phase_delta = target_phase_delta;
            }
            NoteEvent::NoteOff { voice_id, channel, note, .. } => {
                self.start_release_for_voices(sample_rate, voice_id, channel, note);
            }
            NoteEvent::Choke { timing, voice_id, channel, note } => {
                self.choke_voices(context, timing, voice_id, channel, note);
            }
            NoteEvent::PolyModulation { voice_id, poly_modulation_id, normalized_offset, .. } => {
                if let Some(voice_idx) = self.get_voice_idx(voice_id) {
                    let voice = self.voices[voice_idx].as_mut().unwrap();
                    if poly_modulation_id == GAIN_POLY_MOD_ID {
                        let target = self.params.gain.preview_modulated(normalized_offset);
                        let (_, smoother) = voice.voice_gain.get_or_insert_with(|| (normalized_offset, self.params.gain.smoothed.clone()));
                        if voice.internal_voice_id >= this_sample_internal_voice_id_start {
                            smoother.reset(target);
                        } else {
                            smoother.set_target(sample_rate, target);
                        }
                    }
                }
            }
            NoteEvent::MonoAutomation { poly_modulation_id, normalized_value, .. } => {
                for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                    if poly_modulation_id == GAIN_POLY_MOD_ID {
                        if let Some((offset, smoother)) = voice.voice_gain.as_mut() {
                            let target = self.params.gain.preview_plain(normalized_value + *offset);
                            smoother.set_target(sample_rate, target);
                        }
                    }
                }
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
            classic_drive: self.params.classic_drive.value(),
            additive_mix: self.params.additive_mix.value(),
            additive_partials: self.params.additive_partials.value(),
            additive_tilt: self.params.additive_tilt.value(),
            additive_inharm: self.params.additive_inharm.value(),
            additive_morph: self.params.additive_morph.value(),
            additive_decay: self.params.additive_decay.value(),
            additive_drift: self.params.additive_drift.value(),
            custom_wavetable_enable: self.params.custom_wavetable_enable.value(),
            analog_enable: self.params.analog_enable.value(),
            analog_drive: self.params.analog_drive.value(),
            analog_noise: self.params.analog_noise.value(),
            analog_drift: self.params.analog_drift.value(),
            breath_enable: self.params.breath_enable.value(),
            breath_amount: self.params.breath_amount.value(),
            breath_tone: self.params.breath_tone.value(),
            sub_level: self.params.sub_level.value(),
            unison_voices: self.params.unison_voices.value(),
            unison_detune: self.params.unison_detune.value(),
            unison_spread: self.params.unison_spread.value(),
            glide_time: self.params.glide_time_ms.value(),
            vibrato_intensity: self.params.vibrato_intensity.value(),
            filter_type: self.params.filter_type.value(),
            filter_cut: self.params.filter_cut.value(),
            filter_res: self.params.filter_res.value(),
            filter_amount: self.params.filter_amount.value(),
            amp_env_level: self.params.amp_envelope_level.value(),
            fm_enable: self.params.fm_enable.value(),
            fm_source: self.params.fm_source.value(),
            fm_target: self.params.fm_target.value(),
            fm_amount: self.params.fm_amount.value(),
            fm_ratio: self.params.fm_ratio.value(),
            fm_feedback: self.params.fm_feedback.value(),
            fm_env_amount: self.params.fm_env_amount.value(),
            dist_env_amount: self.params.dist_env_amount.value(),
            chorus_enable: self.params.chorus_enable.value(),
            chorus_rate: self.params.chorus_rate.value(),
            chorus_depth: self.params.chorus_depth.value(),
            chorus_mix: self.params.chorus_mix.value(),
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
            output_sat_enable: self.params.output_sat_enable.value(),
            output_sat_type: self.params.output_sat_type.value(),
            output_sat_drive: self.params.output_sat_drive.value(),
            output_sat_mix: self.params.output_sat_mix.value(),
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

    fn render_block_voices(&mut self, fx_left: &mut [f32; MAX_BLOCK_SIZE], fx_right: &mut [f32; MAX_BLOCK_SIZE], block_start: usize, block_end: usize, output: &mut [&mut [f32]], sample_rate: f32, p: &BlockParams, context: &mut impl ProcessContext<Self>) {
        let block_len = block_end - block_start;
        let mut gain = [0.0f32; MAX_BLOCK_SIZE];
        self.params.gain.smoothed.next_block(&mut gain[..block_len], block_len);
        let mut v_gain_buf = [0.0f32; MAX_BLOCK_SIZE];

        let unison_count = match p.unison_voices { UnisonVoices::One => 1, UnisonVoices::Two => 2, UnisonVoices::Four => 4, UnisonVoices::Six => 6 };
        let detune_cents = p.unison_detune * 30.0;
        let offsets: &[f32] = match unison_count { 1 => &[0.0], 2 => &[-0.5, 0.5], 4 => &[-0.75, -0.25, 0.25, 0.75], _ => &[-1.0, -0.6, -0.2, 0.2, 0.6, 1.0] };

        for (v_idx, s_idx) in (block_start..block_end).enumerate() {
            let (seq_gate, seq_cut, seq_res, seq_wt, seq_dist, seq_fm) = if p.seq_enable {
                let tempo = context.transport().tempo.unwrap_or(120.0) as f32;
                let step_rate = (tempo / 60.0) * p.seq_rate;
                let step_idx = (self.seq_phase.floor() as usize) % 32;
                let gate = self.params.seq_lanes[0].steps[step_idx].value.value();
                let cut = self.params.seq_lanes[1].steps[step_idx].value.value();
                let res = self.params.seq_lanes[2].steps[step_idx].value.value();
                let wt = self.params.seq_lanes[3].steps[step_idx].value.value();
                let dist = self.params.seq_lanes[4].steps[step_idx].value.value();
                let fm = self.params.seq_lanes[5].steps[step_idx].value.value();
                self.seq_phase = (self.seq_phase + step_rate / sample_rate).fract() * 32.0;
                let gate_val = (1.0 - p.seq_gate_amount) + p.seq_gate_amount * (gate * 0.5 + 0.5).clamp(0.0, 1.0);
                (gate_val, cut * p.seq_cut_amount, res * p.seq_res_amount, wt * p.seq_wt_amount, dist * p.seq_dist_amount, fm * p.seq_fm_amount)
            } else { (1.0, 0.0, 0.0, 0.0, 0.0, 0.0) };

            for voice in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
                let v_gain = match &voice.voice_gain { Some((_, s)) => { s.next_block(&mut v_gain_buf[..block_len], block_len); v_gain_buf[v_idx] }, None => gain[v_idx] };
                let (l, r, fl, fr) = self.render_voice_sample(voice, sample_rate, unison_count, detune_cents, offsets, p, seq_gate, seq_cut, seq_res, seq_wt, seq_dist, seq_fm);
                let amp = v_gain * voice.velocity_sqrt;
                output[0][s_idx] += l * amp;
                output[1][s_idx] += r * amp;
                fx_left[v_idx] += fl * amp;
                fx_right[v_idx] += fr * amp;
            }
        }
    }

    fn render_voice_sample(&mut self, voice: &mut Voice, sample_rate: f32, unison_count: usize, detune_cents: f32, offsets: &[f32], p: &BlockParams, seq_gate: f32, seq_cut: f32, seq_res: f32, seq_wt: f32, seq_dist: f32, seq_fm: f32) -> (f32, f32, f32, f32) {
        if p.glide_time > 0.0 {
            let coeff = (-1.0_f32 / (p.glide_time * 0.001 * sample_rate)).exp();
            voice.phase_delta = voice.phase_delta * coeff + voice.target_phase_delta * (1.0 - coeff);
        } else { voice.phase_delta = voice.target_phase_delta; }

        let vibrato = voice.vib_mod.get_modulation(sample_rate);
        let mut d_pd = voice.phase_delta * (1.0 + (p.vibrato_intensity * vibrato));
        if p.analog_enable && p.analog_drift > 0.0 {
            voice.drift_offset = (voice.drift_offset + (self.prng.gen::<f32>() - 0.5) * p.analog_drift * 0.0005).clamp(-0.02, 0.02);
        }
        d_pd *= 1.0 + voice.drift_offset;

        voice.amp_envelope.advance(); voice.filter_cut_envelope.advance(); voice.filter_res_envelope.advance();
        voice.fm_envelope.advance(); voice.dist_envelope.advance(); voice.breath_envelope.advance();
        let amp_v = voice.amp_envelope.get_value();
        let lfo1 = voice.mod_lfo1.get_modulation(sample_rate);
        let lfo2 = voice.mod_lfo2.get_modulation(sample_rate);
        
        let m = self.apply_voice_modulations(voice, lfo1, lfo2, amp_v, voice.filter_cut_envelope.get_value(), sample_rate);
        let fm_amt = (p.fm_amount + m.11 + seq_fm + voice.fm_envelope.get_value() * p.fm_env_amount).clamp(-1.0, 1.0);
        let fm_fb = (p.fm_feedback + m.13).clamp(0.0, 0.99);

        let bank = if p.custom_wavetable_enable { self.custom_wavetable.as_ref().unwrap_or(&self.factory_wavetable) } else { &self.factory_wavetable };
        let fm_sig = if p.fm_enable {
            let mod_p = (voice.unison_phases[0] * (p.fm_ratio + m.12) + voice.fm_feedback_state * fm_fb).fract();
            let mod_s = match p.fm_source {
                FmSource::Classic => generate_waveform(p.waveform, mod_p),
                FmSource::Wavetable => bank.sample(mod_p, (p.wavetable_position + m.0 + seq_wt).clamp(0.0, 1.0)),
                FmSource::Sub => (2.0 * std::f32::consts::PI * mod_p).sin(),
            };
            voice.fm_feedback_state = mod_s;
            mod_s * fm_amt * 0.25
        } else { 0.0 };

        let mut c_sum = 0.0; let mut w_sum = 0.0; let mut a_sum = 0.0;
        let wt_pos = (p.wavetable_position + m.0 + seq_wt).clamp(0.0, 1.0);
        let wt_dist = (p.wavetable_distortion + seq_dist + voice.dist_envelope.get_value() * p.dist_env_amount).clamp(0.0, 1.0);

        for i in 0..unison_count {
            let ratio = 2.0_f32.powf(detune_cents * offsets[i] / 1200.0);
            let phase = voice.unison_phases[i];
            let c_ph = if p.fm_enable && matches!(p.fm_target, FmTarget::Classic | FmTarget::Both) { (phase + fm_sig).fract() } else { phase };
            let w_ph = if p.fm_enable && matches!(p.fm_target, FmTarget::Wavetable | FmTarget::Both) { (phase + fm_sig).fract() } else { phase };

            let mut c_s = generate_waveform(p.waveform, c_ph);
            c_s = SubSynth::wavefold(c_s, p.classic_drive);
            c_s -= SubSynth::poly_blep(phase, d_pd * ratio);

            let mut w_s = bank.sample(w_ph, wt_pos);
            w_s = SubSynth::wavefold(w_s, wt_dist);

            let a_s = generate_additive_sample_advanced(c_ph, p.additive_partials.round() as usize, p.additive_tilt + m.2, p.additive_inharm, p.additive_morph + m.3, p.additive_decay + m.4, p.additive_drift + m.5, amp_v, voice.internal_voice_id, i as u32);

            c_sum += c_s; w_sum += w_s; a_sum += a_s;
            voice.unison_phases[i] = (phase + d_pd * ratio).fract();
        }

        let mix_a = (p.osc_blend + p.additive_mix + m.1).clamp(0.0, 1.0);
        let mut mixed = match p.osc_routing {
            OscRouting::ClassicOnly => c_sum,
            OscRouting::WavetableOnly => w_sum,
            OscRouting::Blend => c_sum * (1.0 - mix_a) + w_sum * mix_a,
        } + a_sum * mix_a * 0.5;

        mixed += (2.0 * std::f32::consts::PI * (voice.unison_phases[0] * 0.5).fract()).sin() * p.sub_level;
        if p.breath_enable {
            let noise = (self.prng.gen::<f32>() - 0.5) * 2.0;
            voice.breath_filter.set_bandpass(sample_rate, p.breath_tone, 0.1);
            mixed += voice.breath_filter.process(noise) * p.breath_amount * voice.breath_envelope.get_value();
        }

        let cut = (p.filter_cut + m.6 + seq_cut + voice.filter_cut_envelope.get_value() * p.filter_amount).clamp(20.0, 20000.0);
        let res = (p.filter_res + m.7 + seq_res).clamp(0.0, 1.0);
        mixed = self.apply_voice_filter(voice, mixed, sample_rate, p.filter_type, cut, res);

        let pan = (voice.pan + voice.pan_mod.get_modulation(sample_rate) + m.9).clamp(0.0, 1.0);
        let l_amp = (1.0 - pan).sqrt();
        let r_amp = pan.sqrt();
        let out_amp = seq_gate * p.amp_env_level * amp_v * 0.5 * (voice.trem_mod.get_modulation(sample_rate) + 1.0) * (1.0 + m.10).clamp(0.0, 2.0);

        (mixed * l_amp * out_amp, mixed * r_amp * out_amp, mixed * l_amp, mixed * r_amp)
    }

    #[allow(clippy::type_complexity)]
    fn apply_voice_modulations(&mut self, voice: &mut Voice, lfo1: f32, lfo2: f32, amp_env: f32, filter_env: f32, sample_rate: f32) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) {
        let mut m = [0.0f32; 14];
        let mut apply = |slot: usize, source: ModSource, target: ModTarget, amount: f32, smooth: f32| {
            let src = match source { ModSource::Lfo1 => lfo1, ModSource::Lfo2 => lfo2, ModSource::AmpEnv => amp_env, ModSource::FilterEnv => filter_env };
            let val = src * amount;
            let val = if smooth > 0.0 {
                let coeff = (-1.0 / (smooth * 0.001 * sample_rate)).exp();
                voice.mod_smooth[slot] = voice.mod_smooth[slot] * coeff + val * (1.0 - coeff);
                voice.mod_smooth[slot]
            } else { voice.mod_smooth[slot] = val; val };
            match target {
                ModTarget::WavetablePos => m[0] += val,
                ModTarget::AdditiveMix => m[1] += val,
                ModTarget::AdditiveTilt => m[2] += val,
                ModTarget::AdditiveMorph => m[3] += val,
                ModTarget::AdditiveDecay => m[4] += val,
                ModTarget::AdditiveDrift => m[5] += val,
                ModTarget::FilterCut => m[6] += val,
                ModTarget::FilterRes => m[7] += val,
                ModTarget::FilterAmount => m[8] += val,
                ModTarget::Pan => m[9] += val,
                ModTarget::Gain => m[10] += val,
                ModTarget::FmAmount => m[11] += val,
                ModTarget::FmRatio => m[12] += val,
                ModTarget::FmFeedback => m[13] += val,
            }
        };
        apply(0, self.params.mod1_source.value(), self.params.mod1_target.value(), self.params.mod1_amount.value(), self.params.mod1_smooth_ms.value());
        apply(1, self.params.mod2_source.value(), self.params.mod2_target.value(), self.params.mod2_amount.value(), self.params.mod2_smooth_ms.value());
        apply(2, self.params.mod3_source.value(), self.params.mod3_target.value(), self.params.mod3_amount.value(), self.params.mod3_smooth_ms.value());
        apply(3, self.params.mod4_source.value(), self.params.mod4_target.value(), self.params.mod4_amount.value(), self.params.mod4_smooth_ms.value());
        apply(4, self.params.mod5_source.value(), self.params.mod5_target.value(), self.params.mod5_amount.value(), self.params.mod5_smooth_ms.value());
        apply(5, self.params.mod6_source.value(), self.params.mod6_target.value(), self.params.mod6_amount.value(), self.params.mod6_smooth_ms.value());
        (m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10], m[11], m[12], m[13])
    }

    fn apply_voice_filter(&self, voice: &mut Voice, sample: f32, sample_rate: f32, filter_type: FilterType, cutoff: f32, resonance: f32) -> f32 {
        match filter_type {
            FilterType::Lowpass => { voice.lowpass_filter.set_lowpass(sample_rate, cutoff, resonance); voice.lowpass_filter.process(sample) }
            FilterType::Highpass => { voice.highpass_filter.set_highpass(sample_rate, cutoff, resonance); voice.highpass_filter.process(sample) }
            FilterType::Bandpass => { voice.bandpass_filter.set_bandpass(sample_rate, cutoff, resonance); voice.bandpass_filter.process(sample) }
            FilterType::Notch => { voice.notch_filter.set_notch(sample_rate, cutoff, resonance); voice.notch_filter.process(sample) }
            _ => sample
        }
    }

    fn apply_block_fx(&mut self, output: &mut [&mut [f32]], fx_left: &mut [f32; MAX_BLOCK_SIZE], fx_right: &mut [f32; MAX_BLOCK_SIZE], block_start: usize, block_end: usize, sample_rate: f32, p: &BlockParams) {
        let block_len = block_end - block_start;
        if p.chorus_enable { for i in 0..block_len { let (l, r) = self.chorus.process(fx_left[i], fx_right[i], p.chorus_rate, p.chorus_depth, p.chorus_mix); fx_left[i] = l; fx_right[i] = r; } }
        if p.multi_filter_enable { for i in 0..block_len { let (l, r) = self.multi_filter.process(fx_left[i], fx_right[i], p.multi_filter_routing, p.multi_filter_a_type, p.multi_filter_a_cut, p.multi_filter_a_res, p.multi_filter_a_amt, p.multi_filter_b_type, p.multi_filter_b_cut, p.multi_filter_b_res, p.multi_filter_b_amt, p.multi_filter_c_type, p.multi_filter_c_cut, p.multi_filter_c_res, p.multi_filter_c_amt, p.multi_filter_morph, p.multi_filter_parallel_ab, p.multi_filter_parallel_c); fx_left[i] = l; fx_right[i] = r; } }
        if p.dist_enable { self.distortion.set_tone(p.dist_tone); for i in 0..block_len { fx_left[i] = self.distortion.process_sample(0, fx_left[i], p.dist_drive, p.dist_magic, p.dist_mix); fx_right[i] = self.distortion.process_sample(1, fx_right[i], p.dist_drive, p.dist_magic, p.dist_mix); } }
        if p.eq_enable { self.eq.set_params(p.eq_low_gain, p.eq_mid_gain, p.eq_mid_freq, p.eq_mid_q, p.eq_high_gain); for i in 0..block_len { let l = self.eq.process_sample(0, fx_left[i]); let r = self.eq.process_sample(1, fx_right[i]); fx_left[i] = fx_left[i] * (1.0 - p.eq_mix) + l * p.eq_mix; fx_right[i] = fx_right[i] * (1.0 - p.eq_mix) + r * p.eq_mix; } }
        if p.delay_enable { for i in 0..block_len { let (l, r) = self.delay.process(fx_left[i], fx_right[i], p.delay_time, p.delay_feedback, p.delay_mix); fx_left[i] = l; fx_right[i] = r; } }
        if p.reverb_enable { for i in 0..block_len { let (l, r) = self.reverb.process(fx_left[i], fx_right[i], p.reverb_size, p.reverb_damp, p.reverb_diffusion, p.reverb_shimmer, p.reverb_mix); fx_left[i] = l; fx_right[i] = r; } }
        for i in 0..block_len { output[0][block_start + i] += fx_left[i]; output[1][block_start + i] += fx_right[i]; }
    }

    fn terminate_finished_voices(&mut self, context: &mut impl ProcessContext<Self>, timing: u32) {
        for voice in self.voices.iter_mut() {
            if let Some(v) = voice {
                if v.releasing && v.amp_envelope.is_finished() {
                    context.send_event(NoteEvent::VoiceTerminated { timing, voice_id: Some(v.voice_id), channel: v.channel, note: v.note });
                    *voice = None;
                }
            }
        }
    }

    fn ms_to_s(ms: f32) -> f32 { ms.max(0.0) * 0.001 }
    fn get_voice_idx(&mut self, voice_id: i32) -> Option<usize> { self.voices.iter().position(|v| matches!(v, Some(v) if v.voice_id == voice_id)) }
    fn refresh_custom_wavetable(&mut self) {
        if let Ok(mut data) = self.params.custom_wavetable_data.try_write() { if let Some(table) = data.take() { self.custom_wavetable = Some(WavetableBank::from_table(table)); if let Ok(path) = self.params.custom_wavetable_path.read() { self.custom_wavetable_path = (*path).clone(); } } }
        if self.custom_wavetable.is_none() { if let Ok(path) = self.params.custom_wavetable_path.read() { if let Some(path) = (*path).as_ref() { if self.custom_wavetable_path.as_deref() != Some(path.as_str()) { if let Ok(table) = waveform::load_wavetable_from_file(std::path::Path::new(path)) { self.custom_wavetable = Some(WavetableBank::from_table(table)); self.custom_wavetable_path = Some(path.clone()); } } } } }
    }
    fn refresh_morph_snapshots(&mut self) -> bool {
        let mut changed = false;
        if let Ok(snapshot) = self.params.morph_a_snapshot.read() { if self.morph_a_json != *snapshot { self.morph_a_json = snapshot.clone(); self.morph_a_snapshot = snapshot.as_deref().and_then(|json| serde_json::from_str(json).ok()); changed = true; } }
        if let Ok(snapshot) = self.params.morph_b_snapshot.read() { if self.morph_b_json != *snapshot { self.morph_b_json = snapshot.clone(); self.morph_b_snapshot = snapshot.as_deref().and_then(|json| serde_json::from_str(json).ok()); changed = true; } }
        changed
    }
    fn apply_preset(&mut self, index: usize, sample_rate: f32) {
        if self.factory_presets.is_empty() { return; }
        let idx = index.min(self.factory_presets.len() - 1);
        self.current_preset_index = idx;
        self.factory_presets[idx].data.apply_direct(&self.params);
        unsafe { self.params.gain.as_ptr().update_smoother(sample_rate, false); }
        self.last_preset_param = idx as i32;
    }
    fn construct_envelopes(&self, sample_rate: f32, vel: f32) -> (ADSREnvelope, ADSREnvelope, ADSREnvelope) {
        (ADSREnvelope::new(Self::ms_to_s(self.params.amp_attack_ms.value()), 0.0, Self::ms_to_s(self.params.amp_decay_ms.value()), self.params.amp_sustain_level.value(), Self::ms_to_s(self.params.amp_release_ms.value()), sample_rate, vel, self.params.amp_tension.value()),
         ADSREnvelope::new(Self::ms_to_s(self.params.filter_cut_attack_ms.value()), 0.0, Self::ms_to_s(self.params.filter_cut_decay_ms.value()), self.params.filter_cut_sustain_ms.value(), Self::ms_to_s(self.params.filter_cut_release_ms.value()), sample_rate, vel, self.params.filter_cut_tension.value()),
         ADSREnvelope::new(Self::ms_to_s(self.params.filter_res_attack_ms.value()), 0.0, Self::ms_to_s(self.params.filter_res_decay_ms.value()), self.params.filter_res_sustain_ms.value(), Self::ms_to_s(self.params.filter_res_release_ms.value()), sample_rate, vel, self.params.filter_res_tension.value()))
    }
    fn construct_breath_envelope(&self, sample_rate: f32, vel: f32) -> ADSREnvelope {
        let atk = Self::ms_to_s(self.params.breath_attack_ms.value()); let dec = Self::ms_to_s(self.params.breath_decay_ms.value());
        ADSREnvelope::new(atk, 0.0, dec, 0.0, dec * 0.5, sample_rate, vel, 0.0)
    }
    fn start_voice(&mut self, context: &mut impl ProcessContext<Self>, timing: u32, voice_id: Option<i32>, channel: u8, note: u8, velocity: f32, pan: f32, pressure: f32, brightness: f32, expression: f32, vibrato: f32, tuning: f32, vib_mod: Modulator, trem_mod: Modulator, mod_lfo1: Modulator, mod_lfo2: Modulator, amp_envelope: ADSREnvelope, filter_cut_envelope: ADSREnvelope, filter_res_envelope: ADSREnvelope, fm_envelope: ADSREnvelope, dist_envelope: ADSREnvelope, breath_envelope: ADSREnvelope, filter: FilterType, sample_rate: f32) -> &mut Voice {
        let new_voice = Voice {
            voice_id: voice_id.unwrap_or_else(|| (note as i32) + (channel as i32) + self.next_voice_index as i32),
            internal_voice_id: self.next_internal_voice_id, channel, note, velocity, velocity_sqrt: velocity.sqrt(), pan, pressure, brightness, expression, vibrato, tuning, phase: 0.0, phase_delta: 0.0, target_phase_delta: 0.0, releasing: false, amp_envelope, voice_gain: None, filter_cut_envelope, filter_res_envelope, fm_envelope, dist_envelope, breath_envelope, filter: Some(filter),
            lowpass_filter: filter::LowpassFilter::new(1000.0, 0.5, sample_rate), highpass_filter: filter::HighpassFilter::new(1000.0, 0.5, sample_rate), bandpass_filter: filter::BandpassFilter::new(1000.0, 0.5, sample_rate), notch_filter: filter::NotchFilter::new(1000.0, 1.0, sample_rate), statevariable_filter: filter::StatevariableFilter::new(1000.0, 0.5, sample_rate), comb_filter: filter::CombFilter::new(sample_rate), rainbow_comb_filter: filter::RainbowCombFilter::new(sample_rate), diode_ladder_lp_filter: filter::DiodeLadderFilter::new_lowpass(sample_rate), diode_ladder_hp_filter: filter::DiodeLadderFilter::new_highpass(sample_rate), ms20_filter: filter::Ms20Filter::new(sample_rate), formant_morph_filter: filter::FormantMorphFilter::new(sample_rate), phaser_filter: filter::PhaserFilter::new(sample_rate), comb_allpass_filter: filter::CombAllpassFilter::new(sample_rate), bitcrush_lp_filter: filter::BitcrushLpFilter::new(sample_rate), breath_filter: filter::BandpassFilter::new(2400.0, 0.6, sample_rate),
            vib_mod, trem_mod, mod_lfo1, mod_lfo2, pan_mod: Modulator::new(self.params.pan_lfo_rate.value(), self.params.pan_lfo_intensity.value(), self.params.pan_lfo_attack.value(), self.params.pan_lfo_shape.value()),
            drift_offset: 0.0, mod_smooth: [0.0; 6], fm_feedback_state: 0.0, unison_phases: [0.0; 6], stereo_prev: 0.0, dc_blocker: filter::DCBlocker::new(),
        };
        self.next_internal_voice_id = self.next_internal_voice_id.wrapping_add(1);
        let idx = self.voices.iter().position(|v| v.is_none()).unwrap_or_else(|| {
            let (old_idx, old_v) = self.voices.iter().enumerate().min_by_key(|(_, v)| v.as_ref().map(|v| v.internal_voice_id).unwrap_or(u64::MAX)).unwrap();
            let old_v = old_v.as_ref().unwrap();
            context.send_event(NoteEvent::VoiceTerminated { timing, voice_id: Some(old_v.voice_id), channel: old_v.channel, note: old_v.note });
            old_idx
        });
        self.voices[idx] = Some(new_voice);
        let v = self.voices[idx].as_mut().unwrap();
        v.amp_envelope.trigger(); v.filter_cut_envelope.trigger(); v.filter_res_envelope.trigger(); v.fm_envelope.trigger(); v.dist_envelope.trigger(); v.breath_envelope.trigger(); v.vib_mod.trigger(); v.trem_mod.trigger(); v.mod_lfo1.trigger(); v.mod_lfo2.trigger();
        self.next_voice_index = (idx + 1) % NUM_VOICES;
        v
    }
    fn start_release_for_voices(&mut self, _sr: f32, voice_id: Option<i32>, channel: u8, note: u8) {
        for v in self.voices.iter_mut().filter_map(|v| v.as_mut()) {
            if voice_id == Some(v.voice_id) || (channel == v.channel && note == v.note) {
                v.releasing = true; v.amp_envelope.release(); v.filter_cut_envelope.release(); v.filter_res_envelope.release(); v.fm_envelope.release(); v.dist_envelope.release(); v.breath_envelope.release();
            }
        }
    }
    fn choke_voices(&mut self, context: &mut impl ProcessContext<Self>, timing: u32, voice_id: Option<i32>, channel: u8, note: u8) {
        for v in self.voices.iter_mut() {
            if let Some(v_inner) = v {
                if voice_id == Some(v_inner.voice_id) || (channel == v_inner.channel && note == v_inner.note) {
                    context.send_event(NoteEvent::VoiceTerminated { timing, voice_id: Some(v_inner.voice_id), channel, note });
                    *v = None; if voice_id.is_some() { return; }
                }
            }
        }
    }
    pub fn poly_blep(t: f32, dt: f32) -> f32 {
        if t < dt { let t = t / dt; t + t - t * t - 1.0 }
        else if t > 1.0 - dt { let t = (t - 1.0) / dt; t * t + t + t + 1.0 }
        else { 0.0 }
    }
    pub fn wavefold(sample: f32, amount: f32) -> f32 {
        if amount <= 0.0 { return sample; }
        let x = sample * (1.0 + amount * 8.0);
        let mut folded = (x + 1.0).rem_euclid(4.0);
        if folded > 2.0 { folded = 4.0 - folded; }
        folded - 1.0
    }
}

impl ClapPlugin for SubSynth {
    const CLAP_ID: &'static str = "art.micesynth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A knitty gritty additive/wavetable synthesis engine");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::Synthesizer, ClapFeature::Stereo];
    const CLAP_POLY_MODULATION_CONFIG: Option<PolyModulationConfig> = Some(PolyModulationConfig { max_voice_capacity: NUM_VOICES as u32, supports_overlapping_voices: true });
}

impl Vst3Plugin for SubSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"MiceSynthLingA01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth, Vst3SubCategory::Stereo];
}

nih_export_clap!(SubSynth);
nih_export_vst3!(SubSynth);
