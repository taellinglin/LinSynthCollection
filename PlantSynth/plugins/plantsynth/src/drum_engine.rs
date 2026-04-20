use crate::drum_synth::params::{DrumSlotParams, DRUM_OUTPUT_PAIRS, DRUM_SLOTS};
use crate::sample::{load_sample_from_file, SampleBuffer};
use crate::util;
use std::sync::{Arc, RwLock};

const TONE_LOW_CUTOFF_HZ: f32 = 220.0;
const TONE_HIGH_CUTOFF_HZ: f32 = 2400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleEnvStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

#[derive(Clone, Debug)]
struct SampleVoice {
    sample: Option<Arc<SampleBuffer>>,
    position: f32,
    rate: f32,
    gain: f32,
    pan: f32,
    velocity: f32,
    velocity_sensitivity: f32,
    drive: f32,
    tone_low_gain: f32,
    tone_mid_gain: f32,
    tone_high_gain: f32,
    tone_low_state: f32,
    tone_high_state: f32,
    tone_low_alpha: f32,
    tone_high_alpha: f32,
    env_stage: SampleEnvStage,
    env_level: f32,
    attack_step: f32,
    decay_step: f32,
    sustain: f32,
    release_step: f32,
    output_bus: usize,
    active: bool,
}

impl SampleVoice {
    fn idle() -> Self {
        Self {
            sample: None,
            position: 0.0,
            rate: 1.0,
            gain: 1.0,
            pan: 0.5,
            velocity: 1.0,
            velocity_sensitivity: 1.0,
            drive: 0.0,
            tone_low_gain: 1.0,
            tone_mid_gain: 1.0,
            tone_high_gain: 1.0,
            tone_low_state: 0.0,
            tone_high_state: 0.0,
            tone_low_alpha: 0.0,
            tone_high_alpha: 0.0,
            env_stage: SampleEnvStage::Off,
            env_level: 0.0,
            attack_step: 1.0,
            decay_step: 0.0,
            sustain: 0.0,
            release_step: 0.0,
            output_bus: 0,
            active: false,
        }
    }
}

pub struct DrumEngine {
    sample_rate: f32,
    sample_buffers: [Option<Arc<SampleBuffer>>; DRUM_SLOTS],
    sample_voices: [SampleVoice; DRUM_SLOTS],
}

impl DrumEngine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            sample_buffers: std::array::from_fn(|_| None),
            sample_voices: std::array::from_fn(|_| SampleVoice::idle()),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn sync_sample_buffers(
        &mut self,
        sample_data: &Arc<RwLock<[Option<Arc<SampleBuffer>>; DRUM_SLOTS]>>,
        sample_paths: &Arc<RwLock<[Option<String>; DRUM_SLOTS]>>,
    ) {
        if let Ok(data) = sample_data.try_read() {
            for (index, entry) in data.iter().enumerate() {
                let needs_update = match (&self.sample_buffers[index], entry) {
                    (Some(existing), Some(incoming)) => !Arc::ptr_eq(existing, incoming),
                    (None, None) => false,
                    _ => true,
                };
                if needs_update {
                    self.sample_buffers[index] = entry.clone();
                }
            }
        }

        let mut to_load = Vec::new();
        if let Ok(paths) = sample_paths.try_read() {
            for (index, path) in paths.iter().enumerate() {
                if self.sample_buffers[index].is_none() {
                    if let Some(path) = path.as_ref() {
                        to_load.push((index, path.clone()));
                    }
                }
            }
        }

        if !to_load.is_empty() {
            for (index, path) in to_load {
                if let Ok(buffer) = load_sample_from_file(std::path::Path::new(&path)) {
                    let buffer = Arc::new(buffer);
                    self.sample_buffers[index] = Some(buffer.clone());
                    if let Ok(mut data) = sample_data.try_write() {
                        if let Some(entry) = data.get_mut(index) {
                            *entry = Some(buffer);
                        }
                    }
                }
            }
        }
    }

    pub fn trigger(
        &mut self,
        slot: usize,
        slot_params: &DrumSlotParams,
        velocity: f32,
        note: Option<u8>,
    ) {
        self.trigger_sample(slot, slot_params, velocity, note);
    }

    pub fn process(&mut self, outputs: &mut [&mut [f32]]) {
        let output_pairs = outputs.len() / 2;
        if output_pairs == 0 {
            return;
        }
        let samples = outputs[0].len();
        for idx in 0..samples {
            for sample_voice in &mut self.sample_voices {
                if !sample_voice.active {
                    continue;
                }
                if let Some(mono) = Self::next_sample_voice(sample_voice) {
                    let velocity = 1.0 - sample_voice.velocity_sensitivity
                        + sample_voice.velocity_sensitivity * sample_voice.velocity;
                    let mut value = mono * sample_voice.gain * velocity;
                    if sample_voice.drive > 0.0 {
                        value = Self::apply_drive(value, sample_voice.drive);
                    }
                    let value = Self::apply_tone_sample(sample_voice, value);
                    let pan = sample_voice.pan;
                    let left_gain = (1.0 - pan).sqrt();
                    let right_gain = pan.sqrt();
                    let bus = sample_voice.output_bus.min(output_pairs - 1);
                    let left_index = bus * 2;
                    let right_index = left_index + 1;
                    outputs[left_index][idx] += value * left_gain;
                    outputs[right_index][idx] += value * right_gain;
                }
            }
        }
    }

    fn trigger_sample(
        &mut self,
        slot: usize,
        slot_params: &DrumSlotParams,
        velocity: f32,
        note: Option<u8>,
    ) {
        let Some(buffer) = self.sample_buffers.get(slot).and_then(|entry| entry.clone()) else {
            return;
        };
        let voice = &mut self.sample_voices[slot];
        let base_note = slot_params.midi_note.value().clamp(0, 127) as i32;
        let note_offset = note.map(|note| note as i32 - base_note).unwrap_or(0);
        let pitch = (slot_params.tune.value() + note_offset as f32).clamp(-48.0, 48.0);
        let pitch_ratio = 2.0_f32.powf(pitch / 12.0);
        let rate = buffer.sample_rate / self.sample_rate * pitch_ratio;
        let attack_ms = Self::env_time_ms(slot_params.attack.value().clamp(0.0, 1.0), 1.0, 200.0);
        let decay_ms = Self::env_time_ms(slot_params.decay.value().clamp(0.0, 1.0), 10.0, 1200.0);
        let release_ms =
            Self::env_time_ms(slot_params.sample_env_release.value().clamp(0.0, 1.0), 10.0, 1500.0);
        let sustain = slot_params.sample_env_sustain.value().clamp(0.0, 1.0);

        voice.sample = Some(buffer);
        voice.position = 0.0;
        voice.rate = rate.max(0.01);
        voice.gain = slot_params.level.value().clamp(0.0, 1.0);
        voice.pan = slot_params.pan.value().clamp(0.0, 1.0);
        voice.velocity = velocity.clamp(0.0, 1.0);
        voice.velocity_sensitivity = slot_params.velocity_sensitivity.value().clamp(0.0, 1.0);
        voice.drive = slot_params.drive.value().clamp(0.0, 1.0);
        voice.tone_low_gain = util::db_to_gain(slot_params.tone_low.value());
        voice.tone_mid_gain = util::db_to_gain(slot_params.tone_mid.value());
        voice.tone_high_gain = util::db_to_gain(slot_params.tone_high.value());
        voice.tone_low_state = 0.0;
        voice.tone_high_state = 0.0;
        voice.tone_low_alpha = Self::one_pole_alpha(TONE_LOW_CUTOFF_HZ, self.sample_rate);
        voice.tone_high_alpha = Self::one_pole_alpha(TONE_HIGH_CUTOFF_HZ, self.sample_rate);
        voice.env_stage = SampleEnvStage::Attack;
        voice.env_level = 0.0;
        voice.attack_step = if attack_ms <= 0.0 {
            1.0
        } else {
            1.0 / (attack_ms * 0.001 * self.sample_rate)
        };
        voice.decay_step = if decay_ms <= 0.0 {
            1.0
        } else {
            (1.0 - sustain) / (decay_ms * 0.001 * self.sample_rate)
        };
        voice.sustain = sustain;
        voice.release_step = if release_ms <= 0.0 {
            1.0
        } else {
            1.0 / (release_ms * 0.001 * self.sample_rate)
        };
        voice.output_bus = slot_params
            .output_bus
            .value()
            .clamp(1, DRUM_OUTPUT_PAIRS as i32)
            .saturating_sub(1) as usize;
        voice.active = true;
    }

    fn env_time_ms(value: f32, min_ms: f32, max_ms: f32) -> f32 {
        let clamped = value.clamp(0.0, 1.0);
        min_ms + (max_ms - min_ms) * clamped
    }

    fn next_sample_voice(voice: &mut SampleVoice) -> Option<f32> {
        let sample = voice.sample.as_ref()?;
        if sample.samples.is_empty() {
            voice.active = false;
            return None;
        }
        let len = sample.samples.len() as f32;
        if voice.position >= len {
            if voice.env_stage != SampleEnvStage::Release && voice.env_stage != SampleEnvStage::Off {
                voice.env_stage = SampleEnvStage::Release;
            }
            if voice.env_stage == SampleEnvStage::Off {
                voice.active = false;
                return None;
            }
        }

        let idx = voice.position.floor() as usize;
        let frac = voice.position - idx as f32;
        let s0 = *sample.samples.get(idx).unwrap_or(&0.0);
        let s1 = *sample.samples.get(idx + 1).unwrap_or(&0.0);
        let mut value = s0 + (s1 - s0) * frac;
        voice.position += voice.rate;

        Self::advance_sample_env(voice);
        value *= voice.env_level;

        if voice.env_stage == SampleEnvStage::Off {
            voice.active = false;
        }

        if value.abs() < 1.0e-6 && voice.env_stage == SampleEnvStage::Off {
            voice.active = false;
            return None;
        }

        Some(value)
    }

    fn advance_sample_env(voice: &mut SampleVoice) {
        match voice.env_stage {
            SampleEnvStage::Attack => {
                voice.env_level += voice.attack_step;
                if voice.env_level >= 1.0 {
                    voice.env_level = 1.0;
                    voice.env_stage = SampleEnvStage::Decay;
                }
            }
            SampleEnvStage::Decay => {
                voice.env_level -= voice.decay_step;
                if voice.env_level <= voice.sustain {
                    voice.env_level = voice.sustain;
                    voice.env_stage = SampleEnvStage::Sustain;
                }
            }
            SampleEnvStage::Sustain => {}
            SampleEnvStage::Release => {
                voice.env_level -= voice.release_step;
                if voice.env_level <= 0.0 {
                    voice.env_level = 0.0;
                    voice.env_stage = SampleEnvStage::Off;
                }
            }
            SampleEnvStage::Off => {
                voice.env_level = 0.0;
            }
        }
    }

    fn apply_drive(sample: f32, drive: f32) -> f32 {
        if drive <= 0.0 {
            return sample;
        }
        let amount = 1.0 + drive * 6.0;
        (sample * amount).tanh() / amount
    }

    fn apply_tone_sample(voice: &mut SampleVoice, sample: f32) -> f32 {
        let low = voice.tone_low_state + voice.tone_low_alpha * (sample - voice.tone_low_state);
        voice.tone_low_state = low;
        let high_lp = voice.tone_high_state + voice.tone_high_alpha * (sample - voice.tone_high_state);
        voice.tone_high_state = high_lp;
        let high = sample - high_lp;
        let mid = sample - low - high;
        low * voice.tone_low_gain + mid * voice.tone_mid_gain + high * voice.tone_high_gain
    }

    fn one_pole_alpha(cutoff: f32, sample_rate: f32) -> f32 {
        let dt = 1.0 / sample_rate.max(1.0);
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff.max(1.0));
        dt / (dt + rc)
    }
}
