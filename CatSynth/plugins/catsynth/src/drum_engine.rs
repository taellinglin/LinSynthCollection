use crate::drum_model::{ExciterType, InstrumentType, MaterialType, NoiseType, ResonatorType};
use crate::drum_params::{DrumOrganicKitPreset, DrumSlotParams, DRUM_SLOTS};

const MODE_COUNT: usize = 8;
const HARMONIC_RATIOS: [f32; MODE_COUNT] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
const CYMBAL_RATIOS: [f32; MODE_COUNT] = [1.0, 1.37, 1.92, 2.45, 2.93, 3.58, 4.29, 5.11];
const TONE_LOW_CUTOFF_HZ: f32 = 220.0;
const TONE_HIGH_CUTOFF_HZ: f32 = 2400.0;

#[derive(Clone, Copy, Debug)]
struct ModeState {
    coeff1: f32,
    coeff2: f32,
    gain: f32,
    y1: f32,
    y2: f32,
}

impl ModeState {
    fn idle() -> Self {
        Self {
            coeff1: 0.0,
            coeff2: 0.0,
            gain: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ExciterState {
    env: f32,
    decay: f32,
    impulse: bool,
}

impl ExciterState {
    fn idle() -> Self {
        Self {
            env: 0.0,
            decay: 0.0,
            impulse: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DrumVoice {
    instrument: InstrumentType,
    exciter: ExciterType,
    exciter_mix: f32,
    resonator: ResonatorType,
    material: MaterialType,
    velocity: f32,
    level: f32,
    pan: f32,
    drive: f32,
    transient: f32,
    body_mix: f32,
    velocity_sensitivity: f32,
    tone_low_gain: f32,
    tone_mid_gain: f32,
    tone_high_gain: f32,
    tone_low_state: f32,
    tone_high_state: f32,
    tone_low_alpha: f32,
    tone_high_alpha: f32,
    noise: f32,
    diffusion: f32,
    noise_color: f32,
    noise_density: f32,
    noise_type: NoiseType,
    resonator_mix: f32,
    mode_tilt: f32,
    cymbal_shape: f32,
    snare_wire: f32,
    snare_wire_decay: f32,
    noise_color_alpha: f32,
    noise_color_lp: f32,
    noise_pink_1: f32,
    noise_pink_2: f32,
    noise_metal_lp: f32,
    noise_metal_ap1_x1: f32,
    noise_metal_ap1_y1: f32,
    noise_metal_ap2_x1: f32,
    noise_metal_ap2_y1: f32,
    spread: f32,
    pan_lfo_rate: f32,
    pan_lfo_depth: f32,
    pan_lfo_phase: f32,
    strike_position: f32,
    strike_hardness: f32,
    attack: f32,
    pitch_env: f32,
    pitch_decay: f32,
    pitch_env_state: f32,
    pitch_phase: f32,
    pitch_base: f32,
    body_env: f32,
    body_decay: f32,
    body_phase: f32,
    noise_env: f32,
    noise_decay: f32,
    decay: f32,
    damping: f32,
    active: bool,
    exciter_state: ExciterState,
    modes: [ModeState; MODE_COUNT],
    rng: u32,
    stereo_sign: f32,
    noise_lp1: f32,
    noise_lp2: f32,
    snare_lp: f32,
    snare_wire_lp: f32,
    snare_low_alpha: f32,
    snare_high_alpha: f32,
    ap1_x1: f32,
    ap1_y1: f32,
    ap2_x1: f32,
    ap2_y1: f32,
}

impl DrumVoice {
    fn idle(seed: u32) -> Self {
        Self {
            instrument: InstrumentType::Kick,
            exciter: ExciterType::Mallet,
            exciter_mix: 0.4,
            resonator: ResonatorType::Membrane,
            material: MaterialType::Skin,
            velocity: 0.0,
            level: 0.9,
            pan: 0.5,
            drive: 0.0,
            transient: 0.4,
            body_mix: 0.8,
            velocity_sensitivity: 0.85,
            tone_low_gain: 1.0,
            tone_mid_gain: 1.0,
            tone_high_gain: 1.0,
            tone_low_state: 0.0,
            tone_high_state: 0.0,
            tone_low_alpha: 0.0,
            tone_high_alpha: 0.0,
            noise: 0.2,
            diffusion: 0.5,
            noise_color: 0.5,
            noise_density: 0.5,
            noise_type: NoiseType::White,
            resonator_mix: 0.7,
            mode_tilt: 0.2,
            cymbal_shape: 0.5,
            snare_wire: 0.6,
            snare_wire_decay: 0.5,
            noise_color_alpha: 0.0,
            noise_color_lp: 0.0,
            noise_pink_1: 0.0,
            noise_pink_2: 0.0,
            noise_metal_lp: 0.0,
            noise_metal_ap1_x1: 0.0,
            noise_metal_ap1_y1: 0.0,
            noise_metal_ap2_x1: 0.0,
            noise_metal_ap2_y1: 0.0,
            spread: 0.0,
            pan_lfo_rate: 1.2,
            pan_lfo_depth: 0.0,
            pan_lfo_phase: 0.0,
            strike_position: 0.5,
            strike_hardness: 0.6,
            attack: 0.4,
            pitch_env: 0.35,
            pitch_decay: 0.4,
            pitch_env_state: 0.0,
            pitch_phase: 0.0,
            pitch_base: 48.0,
            body_env: 0.0,
            body_decay: 0.0,
            body_phase: 0.0,
            noise_env: 0.0,
            noise_decay: 0.0,
            decay: 0.6,
            damping: 0.5,
            active: false,
            exciter_state: ExciterState::idle(),
            modes: std::array::from_fn(|_| ModeState::idle()),
            rng: seed,
            stereo_sign: if seed & 1 == 0 { 1.0 } else { -1.0 },
            noise_lp1: 0.0,
            noise_lp2: 0.0,
            snare_lp: 0.0,
            snare_wire_lp: 0.0,
            snare_low_alpha: 0.0,
            snare_high_alpha: 0.0,
            ap1_x1: 0.0,
            ap1_y1: 0.0,
            ap2_x1: 0.0,
            ap2_y1: 0.0,
        }
    }
}

pub struct DrumEngine {
    sample_rate: f32,
    slots: [DrumVoice; DRUM_SLOTS],
}

pub struct AuxOutput<'a> {
    pub left: &'a mut [f32],
    pub right: &'a mut [f32],
}

impl DrumEngine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            slots: std::array::from_fn(|index| DrumVoice::idle(0x1234_5678 ^ index as u32)),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn trigger(
        &mut self,
        slot: usize,
        slot_params: &DrumSlotParams,
        velocity: f32,
        kit: DrumOrganicKitPreset,
    ) {
        if let Some(voice) = self.slots.get_mut(slot) {
            let sample_rate = self.sample_rate;
            voice.instrument = slot_params.instrument.value();
            voice.exciter = slot_params.exciter.value();
            voice.exciter_mix = slot_params.exciter_mix.value().clamp(0.0, 1.0);
            voice.resonator = slot_params.resonator.value();
            voice.material = slot_params.material.value();
            voice.velocity = velocity.clamp(0.0, 1.0);
            voice.level = slot_params.level.value().clamp(0.0, 1.0);
            voice.pan = slot_params.pan.value().clamp(0.0, 1.0);
            voice.drive = slot_params.drive.value().clamp(0.0, 1.0);
            voice.transient = slot_params.transient.value().clamp(0.0, 1.0);
            voice.body_mix = slot_params.body.value().clamp(0.0, 1.0);
            voice.velocity_sensitivity = slot_params.velocity_sensitivity.value().clamp(0.0, 1.0);
            voice.tone_low_gain = db_to_gain(slot_params.tone_low.value());
            voice.tone_mid_gain = db_to_gain(slot_params.tone_mid.value());
            voice.tone_high_gain = db_to_gain(slot_params.tone_high.value());
            voice.tone_low_alpha = Self::one_pole_alpha(TONE_LOW_CUTOFF_HZ, sample_rate);
            voice.tone_high_alpha = Self::one_pole_alpha(TONE_HIGH_CUTOFF_HZ, sample_rate);
            voice.tone_low_state = 0.0;
            voice.tone_high_state = 0.0;
            voice.noise = slot_params.noise.value().clamp(0.0, 1.0);
            voice.diffusion = slot_params.diffusion.value().clamp(0.0, 1.0);
            voice.noise_color = slot_params.noise_color.value().clamp(0.0, 1.0);
            voice.noise_density = slot_params.noise_density.value().clamp(0.0, 1.0);
            voice.noise_type = slot_params.noise_type.value();
            voice.resonator_mix = slot_params.resonator_mix.value().clamp(0.0, 1.0);
            voice.mode_tilt = slot_params.mode_tilt.value().clamp(-1.0, 1.0);
            voice.cymbal_shape = slot_params.cymbal_shape.value().clamp(0.0, 1.0);
            voice.snare_wire = slot_params.snare_wire.value().clamp(0.0, 1.0);
            voice.snare_wire_decay = slot_params.snare_wire_decay.value().clamp(0.0, 1.0);
            let noise_cutoff = 800.0 + voice.noise_color * 9000.0;
            voice.noise_color_alpha = Self::one_pole_alpha(noise_cutoff, sample_rate);
            voice.noise_color_lp = 0.0;
            voice.noise_pink_1 = 0.0;
            voice.noise_pink_2 = 0.0;
            voice.noise_metal_lp = 0.0;
            voice.noise_metal_ap1_x1 = 0.0;
            voice.noise_metal_ap1_y1 = 0.0;
            voice.noise_metal_ap2_x1 = 0.0;
            voice.noise_metal_ap2_y1 = 0.0;
            voice.spread = slot_params.spread.value().clamp(0.0, 1.0);
            voice.pan_lfo_rate = slot_params.pan_lfo_rate.value().clamp(0.1, 12.0);
            voice.pan_lfo_depth = slot_params.pan_lfo_depth.value().clamp(0.0, 1.0);
            voice.pan_lfo_phase = 0.0;
            voice.strike_position = slot_params.strike_position.value().clamp(0.0, 1.0);
            voice.strike_hardness = slot_params.strike_hardness.value().clamp(0.0, 1.0);
            voice.attack = slot_params.attack.value().clamp(0.0, 1.0);
            voice.pitch_env = slot_params.pitch_env.value().clamp(0.0, 1.0);
            voice.pitch_decay = slot_params.pitch_decay.value().clamp(0.0, 1.0);
            voice.decay = slot_params.decay.value().clamp(0.01, 1.0);
            voice.damping = slot_params.damping.value().clamp(0.0, 1.0);
            let decay_curve = Self::decay_curve(voice.decay);
            let base = Self::base_frequency(voice.instrument)
                * 2.0_f32.powf(slot_params.tune.value() / 12.0);
            voice.pitch_base = base.max(1.0);
            voice.pitch_env_state = 1.0;
            voice.pitch_phase = 0.0;
            let body_time = match voice.instrument {
                InstrumentType::Kick => 0.06 + decay_curve * 2.0,
                InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh => {
                    0.04 + decay_curve * 1.1
                }
                _ => 0.02,
            };
            voice.body_env = if matches!(
                voice.instrument,
                InstrumentType::Kick
                    | InstrumentType::TomLow
                    | InstrumentType::TomMid
                    | InstrumentType::TomHigh
            ) {
                1.0
            } else {
                0.0
            };
            voice.body_decay = (-1.0 / (body_time.max(0.01) * sample_rate)).exp();
            voice.body_phase = 0.0;
            let noise_time = match voice.instrument {
                InstrumentType::HatClosed => 0.02 + decay_curve * 0.18,
                InstrumentType::HatOpen => 0.05 + decay_curve * 0.55,
                InstrumentType::HatPedal => 0.015 + decay_curve * 0.12,
                InstrumentType::Ride => 0.12 + decay_curve * 0.8,
                InstrumentType::Crash => 0.16 + decay_curve * 1.1,
                InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot => {
                    (0.03 + decay_curve * 0.25) * (0.6 + voice.snare_wire_decay * 1.2)
                }
                _ => 0.02,
            };
            voice.noise_env = 1.0;
            voice.noise_decay = (-1.0 / (noise_time.max(0.005) * sample_rate)).exp();
            if matches!(
                voice.instrument,
                InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot
            ) {
                let density = Self::material_density(voice.material);
                let tension = 0.5 + voice.strike_hardness * 0.5;
                let low_cut = 1200.0 + voice.diffusion * 1400.0 + density * 600.0;
                let high_cut = 5000.0 + voice.diffusion * 5000.0 + tension * 2000.0;
                voice.snare_low_alpha = Self::one_pole_alpha(low_cut, sample_rate);
                voice.snare_high_alpha = Self::one_pole_alpha(high_cut, sample_rate);
                voice.snare_lp = 0.0;
                voice.snare_wire_lp = 0.0;
            }
            Self::apply_organic_kit(voice, kit);
            Self::initialize_modes(sample_rate, voice, base);
            voice.active = true;
        }
    }

    pub fn process(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        mut aux_outputs: Option<&mut [AuxOutput]>,
    ) {
        let frames = left.len().min(right.len());
        for idx in 0..frames {
            let mut left_sum = 0.0;
            let mut right_sum = 0.0;
            for (slot_index, voice) in self.slots.iter_mut().enumerate() {
                if !voice.active {
                    continue;
                }
                let exciter = Self::next_exciter_sample(voice);
                let mut resonated = Self::process_modes(voice, exciter);
                let mut noise_layer = Self::instrument_noise(voice);
                if Self::is_cymbal(voice.instrument) {
                    let resonator_mix =
                        0.08 + (1.0 - voice.noise) * (0.12 + (1.0 - voice.cymbal_shape) * 0.18);
                    resonated *= resonator_mix * voice.resonator_mix;
                    noise_layer *= (1.1 + voice.noise * 0.9) * (0.7 + voice.noise_density * 0.8);
                } else {
                    resonated *= voice.resonator_mix;
                }
                let pitch_layer = Self::pitch_env_sample(voice, self.sample_rate);
                let body_layer = Self::body_sample(voice, self.sample_rate);
                let tonal = resonated + pitch_layer + body_layer;
                let driven_tonal = Self::apply_drive(tonal, voice.drive);
                let driven_noise = if voice.drive > 0.0 {
                    Self::apply_drive(noise_layer, voice.drive)
                } else {
                    noise_layer
                };
                let toned = if Self::is_noise_sound(voice.instrument, voice.exciter) {
                    Self::apply_tone(voice, driven_tonal) + driven_noise
                } else {
                    Self::apply_tone(voice, driven_tonal + driven_noise)
                };
                let velocity = 1.0 - voice.velocity_sensitivity + voice.velocity_sensitivity * voice.velocity;
                let norm = Self::instrument_level_norm(voice.instrument);
                let mono = toned * voice.level * velocity * norm;
                let pan = Self::pan_with_lfo(voice, self.sample_rate);
                let left_gain = (1.0 - pan).sqrt();
                let right_gain = pan.sqrt();
                let width = voice.spread * 0.4 * voice.stereo_sign;
                let left_sample = mono * (left_gain + width);
                let right_sample = mono * (right_gain - width);
                left_sum += left_sample;
                right_sum += right_sample;
                if let Some(aux_outputs) = aux_outputs.as_mut() {
                    if !aux_outputs.is_empty() {
                        let aux_index = slot_index % aux_outputs.len();
                        let aux = &mut aux_outputs[aux_index];
                        if idx < aux.left.len() && idx < aux.right.len() {
                            aux.left[idx] += left_sample;
                            aux.right[idx] += right_sample;
                        }
                    }
                }
                if voice.exciter_state.env < 1.0e-4 && toned.abs() < 1.0e-4 {
                    voice.active = false;
                }
            }
            left[idx] += left_sum;
            right[idx] += right_sum;
        }
    }

    fn apply_drive(sample: f32, drive: f32) -> f32 {
        if drive <= 0.0 {
            return sample;
        }
        let amount = 1.0 + drive * 6.0;
        (sample * amount).tanh() / amount
    }

    fn apply_noise_color(voice: &mut DrumVoice, sample: f32) -> f32 {
        let alpha = voice.noise_color_alpha;
        voice.noise_color_lp += alpha * (sample - voice.noise_color_lp);
        let lp = voice.noise_color_lp;
        let hp = sample - lp;
        let mix = voice.noise_color;
        let shaped = lp * (1.0 - mix) + hp * mix;
        if Self::is_cymbal(voice.instrument) {
            shaped * (0.7 + mix * 0.9)
        } else {
            shaped
        }
    }

    fn apply_tone(voice: &mut DrumVoice, sample: f32) -> f32 {
        let low = voice.tone_low_state + voice.tone_low_alpha * (sample - voice.tone_low_state);
        voice.tone_low_state = low;
        let high_lp =
            voice.tone_high_state + voice.tone_high_alpha * (sample - voice.tone_high_state);
        voice.tone_high_state = high_lp;
        let high = sample - high_lp;
        let mid = sample - low - high;
        low * voice.tone_low_gain + mid * voice.tone_mid_gain + high * voice.tone_high_gain
    }

    fn pan_with_lfo(voice: &mut DrumVoice, sample_rate: f32) -> f32 {
        if voice.pan_lfo_depth <= 0.0 {
            return voice.pan;
        }
        let phase = voice.pan_lfo_phase + voice.pan_lfo_rate / sample_rate;
        voice.pan_lfo_phase = phase.fract();
        let lfo = (2.0 * std::f32::consts::PI * voice.pan_lfo_phase).sin();
        (voice.pan + lfo * voice.pan_lfo_depth * 0.5).clamp(0.0, 1.0)
    }

    fn one_pole_alpha(cutoff: f32, sample_rate: f32) -> f32 {
        let cutoff = cutoff.max(20.0).min(sample_rate * 0.45);
        cutoff / (cutoff + sample_rate)
    }

    fn decay_curve(value: f32) -> f32 {
        value.clamp(0.0, 1.0).powf(1.6)
    }

    fn next_exciter_sample(voice: &mut DrumVoice) -> f32 {
        if voice.exciter_state.env <= 0.0 {
            return 0.0;
        }
        let impulse = if voice.exciter_state.impulse {
            voice.exciter_state.impulse = false;
            1.0 + voice.transient * 1.6
        } else {
            0.0
        };
        let noise = if Self::is_hat(voice.instrument) {
            Self::hat_noise_sample(voice)
        } else {
            Self::next_noise_typed(voice)
        } * voice.noise * 1.25;
        let mix = voice.exciter_mix.clamp(0.0, 1.0);
        let impulse = impulse * (1.0 - mix);
        let noise = noise * mix;
        let base = match voice.exciter {
            ExciterType::Mallet => impulse + noise * 0.3,
            ExciterType::Stick => impulse + noise * 0.6,
            ExciterType::Brush => noise * 0.8,
            ExciterType::Noise => noise,
            ExciterType::Impulse => impulse,
        };
        let transient_boost = 0.7 + voice.transient * 0.6;
        let out = base * voice.exciter_state.env * transient_boost;
        voice.exciter_state.env *= voice.exciter_state.decay;
        out
    }

    fn next_noise(voice: &mut DrumVoice) -> f32 {
        voice.rng ^= voice.rng << 13;
        voice.rng ^= voice.rng >> 17;
        voice.rng ^= voice.rng << 5;
        let v = (voice.rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        v
    }

    fn next_noise_typed(voice: &mut DrumVoice) -> f32 {
        let white = Self::next_noise(voice);
        match voice.noise_type {
            NoiseType::White => white,
            NoiseType::Pink => {
                voice.noise_pink_1 = 0.995 * voice.noise_pink_1 + 0.06 * white;
                voice.noise_pink_2 = 0.985 * voice.noise_pink_2 + 0.03 * white;
                (voice.noise_pink_1 + voice.noise_pink_2 + white * 0.2) * 0.6
            }
            NoiseType::Metallic => {
                voice.noise_metal_lp += 0.18 * (white - voice.noise_metal_lp);
                let mut metallic = white - voice.noise_metal_lp;
                metallic = Self::allpass(
                    metallic,
                    0.72,
                    &mut voice.noise_metal_ap1_x1,
                    &mut voice.noise_metal_ap1_y1,
                );
                let metallic = Self::allpass(
                    metallic,
                    0.53,
                    &mut voice.noise_metal_ap2_x1,
                    &mut voice.noise_metal_ap2_y1,
                );
                let metallic_level = voice.cymbal_shape.clamp(0.0, 1.0);
                metallic * metallic_level
            }
        }
    }

    fn hat_noise_sample(voice: &mut DrumVoice) -> f32 {
        let density = voice.noise_density.clamp(0.0, 1.0);
        let diffusion = (voice.diffusion.clamp(0.0, 1.0) * (0.7 + density * 0.6)).min(1.0);
        let density = Self::material_density(voice.material);
        let noise = Self::next_noise_typed(voice);
        let lp_fast = 0.09 + diffusion * 0.12 + (1.0 - density) * 0.05;
        let lp_slow = 0.02 + diffusion * 0.04 + (1.0 - density) * 0.02;
        let lp1 = voice.noise_lp1 + lp_fast * (noise - voice.noise_lp1);
        let lp2 = voice.noise_lp2 + lp_slow * (noise - voice.noise_lp2);
        voice.noise_lp1 = lp1;
        voice.noise_lp2 = lp2;
        let hp = noise - lp2;
        let shape = voice.cymbal_shape.clamp(0.0, 1.0);
        let mix = 0.12 + diffusion * 0.34 + density * 0.08 + shape * 0.08;
        let mut diffused = hp + lp1 * mix;
        let g1 = 0.35 + diffusion * 0.45 + density * 0.1;
        let g2 = 0.3 + diffusion * 0.5 + density * 0.1;
        diffused = Self::allpass(diffused, g1, &mut voice.ap1_x1, &mut voice.ap1_y1);
        diffused = Self::allpass(diffused, g2, &mut voice.ap2_x1, &mut voice.ap2_y1);
        diffused * (0.8 + voice.noise_density * 0.6)
    }

    fn snare_noise_sample(voice: &mut DrumVoice) -> f32 {
        let noise = Self::next_noise_typed(voice);
        voice.snare_lp += voice.snare_low_alpha * (noise - voice.snare_lp);
        voice.snare_wire_lp += voice.snare_high_alpha * (noise - voice.snare_wire_lp);
        let band = voice.snare_wire_lp - voice.snare_lp;
        let buzz = (band * (1.1 + voice.diffusion * 0.8)).tanh() * voice.snare_wire;
        let air = noise - voice.snare_wire_lp;
        let tension = 0.6 + voice.strike_hardness * 0.6;
        buzz * tension + air * (0.2 + voice.noise_density * 0.2)
    }

    fn instrument_noise(voice: &mut DrumVoice) -> f32 {
        let env = voice.noise_env;
        if voice.noise <= 0.0 || env <= 0.0 {
            return 0.0;
        }
        let out = match voice.instrument {
            InstrumentType::HatClosed
            | InstrumentType::HatOpen
            | InstrumentType::HatPedal
            | InstrumentType::Ride
            | InstrumentType::Crash => {
                let noise = Self::hat_noise_sample(voice);
                Self::apply_noise_color(voice, noise) * voice.noise * env
            }
            InstrumentType::Snare | InstrumentType::Clap | InstrumentType::Rimshot => {
                let noise = Self::snare_noise_sample(voice);
                Self::apply_noise_color(voice, noise) * voice.noise * env * 1.2
            }
            _ => 0.0,
        };
        voice.noise_env *= voice.noise_decay;
        out * 1.5
    }

    fn pitch_env_sample(voice: &mut DrumVoice, sample_rate: f32) -> f32 {
        if voice.pitch_env <= 0.0 || voice.pitch_env_state <= 1.0e-4 {
            return 0.0;
        }
        let use_pitch = matches!(
            voice.instrument,
            InstrumentType::Kick | InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh
        );
        if !use_pitch {
            return 0.0;
        }
        let decay_time = 0.015 + voice.pitch_decay * 0.25;
        let decay = (-1.0 / (decay_time * sample_rate)).exp();
        let pitch_mod = 1.0 + voice.pitch_env * 3.0 * voice.pitch_env_state;
        let freq = (voice.pitch_base * pitch_mod).min(sample_rate * 0.45);
        voice.pitch_phase = (voice.pitch_phase + freq / sample_rate).fract();
        let amp = voice.pitch_env * voice.pitch_env_state;
        voice.pitch_env_state *= decay;
        (2.0 * std::f32::consts::PI * voice.pitch_phase).sin() * amp
    }

    fn body_sample(voice: &mut DrumVoice, sample_rate: f32) -> f32 {
        if voice.body_env <= 1.0e-4 {
            return 0.0;
        }
        let pitch_mod = 1.0 + voice.pitch_env * 2.0 * voice.pitch_env_state;
        let freq = (voice.pitch_base * pitch_mod).min(sample_rate * 0.45);
        voice.body_phase = (voice.body_phase + freq / sample_rate).fract();
        let amp = voice.body_env * voice.body_mix;
        voice.body_env *= voice.body_decay;
        (2.0 * std::f32::consts::PI * voice.body_phase).sin() * amp
    }

    fn allpass(input: f32, g: f32, x1: &mut f32, y1: &mut f32) -> f32 {
        let output = -g * input + *x1 + g * *y1;
        *x1 = input;
        *y1 = output;
        output
    }

    fn is_hat(instrument: InstrumentType) -> bool {
        matches!(
            instrument,
            InstrumentType::HatClosed | InstrumentType::HatOpen | InstrumentType::HatPedal
        )
    }

    fn is_cymbal(instrument: InstrumentType) -> bool {
        matches!(
            instrument,
            InstrumentType::HatClosed
                | InstrumentType::HatOpen
                | InstrumentType::HatPedal
                | InstrumentType::Ride
                | InstrumentType::Crash
        )
    }

    fn is_noise_sound(instrument: InstrumentType, exciter: ExciterType) -> bool {
        if matches!(exciter, ExciterType::Noise | ExciterType::Brush) {
            return true;
        }
        matches!(
            instrument,
            InstrumentType::HatClosed
                | InstrumentType::HatOpen
                | InstrumentType::HatPedal
                | InstrumentType::Ride
                | InstrumentType::Crash
                | InstrumentType::Snare
                | InstrumentType::Clap
                | InstrumentType::Rimshot
                | InstrumentType::Perc1
                | InstrumentType::Perc2
        )
    }

    fn instrument_level_norm(instrument: InstrumentType) -> f32 {
        match instrument {
            InstrumentType::Kick => 1.05,
            InstrumentType::Snare => 0.95,
            InstrumentType::HatClosed => 0.75,
            InstrumentType::HatOpen => 0.8,
            InstrumentType::HatPedal => 0.7,
            InstrumentType::Ride => 0.8,
            InstrumentType::Crash => 0.85,
            InstrumentType::Rimshot => 0.9,
            InstrumentType::Clap => 0.9,
            InstrumentType::TomLow => 0.95,
            InstrumentType::TomMid => 0.9,
            InstrumentType::TomHigh => 0.85,
            InstrumentType::Perc1 => 0.85,
            InstrumentType::Perc2 => 0.85,
            InstrumentType::Fx1 => 0.8,
            InstrumentType::Fx2 => 0.8,
        }
    }

    fn process_modes(voice: &mut DrumVoice, exciter: f32) -> f32 {
        let mut sum = 0.0;
        for mode in voice.modes.iter_mut() {
            if mode.gain == 0.0 {
                continue;
            }
            let input = exciter * mode.gain;
            let y = mode.coeff1 * mode.y1 - mode.coeff2 * mode.y2 + input;
            mode.y2 = mode.y1;
            mode.y1 = y;
            sum += y;
        }
        sum
    }

    fn initialize_modes(sample_rate: f32, voice: &mut DrumVoice, base: f32) {
        let ratios = Self::mode_ratios_for_instrument(voice.instrument, voice.resonator);
        let decay_scale = match voice.instrument {
            InstrumentType::HatClosed => 0.18,
            InstrumentType::HatOpen => 0.45,
            InstrumentType::HatPedal => 0.12,
            InstrumentType::Ride => 0.55,
            InstrumentType::Crash => 0.7,
            _ => 1.0,
        };
        let decay_seconds = (0.03 + voice.decay * 1.6) * decay_scale;
        let material_scale = Self::material_decay_scale(voice.material);
        let position = voice.strike_position.max(0.02);
        let hardness = voice.strike_hardness;
        let exciter_time =
            (0.001 + (1.0 - hardness) * (0.02 + voice.transient * 0.02)) * (0.4 + voice.attack * 1.4);
        voice.exciter_state.env = 1.0;
        voice.exciter_state.decay = (-1.0 / (exciter_time * sample_rate)).exp();
        voice.exciter_state.impulse = true;

        for (index, mode) in voice.modes.iter_mut().enumerate() {
            let mut ratio = ratios[index % ratios.len()];
            if Self::is_cymbal(voice.instrument) {
                ratio *= 0.9 + voice.cymbal_shape * 0.25;
            }
            let mut freq = base * ratio;
            if Self::is_cymbal(voice.instrument) {
                let mut seed = voice.rng ^ (index as u32 * 0x9e37_79b9);
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let detune = 0.92 + (seed as f32 / u32::MAX as f32) * 0.16;
                freq *= detune;
            }
            let freq = freq.min(sample_rate * 0.45);
            let tau = (decay_seconds * material_scale) / (1.0 + index as f32 * voice.damping);
            let r = (-1.0 / (tau * sample_rate)).exp();
            let w = 2.0 * std::f32::consts::PI * freq / sample_rate;
            mode.coeff1 = 2.0 * r * w.cos();
            mode.coeff2 = r * r;
            let pos_weight =
                (std::f32::consts::PI * position * (index as f32 + 1.0)).sin().abs();
            let tilt = match voice.instrument {
                InstrumentType::HatClosed
                | InstrumentType::HatOpen
                | InstrumentType::HatPedal
                | InstrumentType::Ride
                | InstrumentType::Crash => {
                    (0.22 + (1.0 - voice.cymbal_shape) * 0.14) / (1.0 + index as f32 * 0.7)
                }
                _ => 1.0 / (1.0 + index as f32 * 0.4),
            };
            let tilt_pos = index as f32 / (MODE_COUNT as f32 - 1.0);
            let tilt_curve = (tilt_pos * 2.0 - 1.0) * voice.mode_tilt;
            let tilt_gain = (1.0 + tilt_curve).clamp(0.2, 2.0);
            mode.gain = pos_weight * tilt * tilt_gain;
            mode.y1 = 0.0;
            mode.y2 = 0.0;
        }
    }

    fn base_frequency(instrument: InstrumentType) -> f32 {
        match instrument {
            InstrumentType::Kick => 48.0,
            InstrumentType::Snare => 180.0,
            InstrumentType::HatClosed => 4200.0,
            InstrumentType::HatOpen => 3200.0,
            InstrumentType::HatPedal => 3800.0,
            InstrumentType::Ride => 2200.0,
            InstrumentType::Crash => 2600.0,
            InstrumentType::Rimshot => 900.0,
            InstrumentType::Clap => 1400.0,
            InstrumentType::TomLow => 110.0,
            InstrumentType::TomMid => 180.0,
            InstrumentType::TomHigh => 260.0,
            InstrumentType::Perc1 => 540.0,
            InstrumentType::Perc2 => 780.0,
            InstrumentType::Fx1 => 600.0,
            InstrumentType::Fx2 => 900.0,
        }
    }

    fn material_decay_scale(material: MaterialType) -> f32 {
        match material {
            MaterialType::Skin => 1.0,
            MaterialType::Plastic => 0.9,
            MaterialType::Metal => 1.4,
            MaterialType::Wood => 0.8,
            MaterialType::Composite => 1.1,
        }
    }

    fn material_density(material: MaterialType) -> f32 {
        match material {
            MaterialType::Skin => 0.6,
            MaterialType::Plastic => 0.75,
            MaterialType::Metal => 1.0,
            MaterialType::Wood => 0.55,
            MaterialType::Composite => 0.85,
        }
    }

    fn mode_ratios(resonator: ResonatorType) -> &'static [f32] {
        match resonator {
            ResonatorType::Membrane => &[1.0, 1.59, 2.14, 2.30, 2.65, 2.92, 3.16, 3.60],
            ResonatorType::Plate => &[1.0, 2.0, 3.0, 4.2, 5.4, 6.8, 8.0, 9.5],
            ResonatorType::Metallic => &[1.0, 1.41, 2.23, 2.90, 3.60, 4.10, 5.00, 6.20],
            ResonatorType::Tube => &[1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0],
        }
    }

    fn mode_ratios_for_instrument(
        instrument: InstrumentType,
        resonator: ResonatorType,
    ) -> &'static [f32] {
        match instrument {
            InstrumentType::Kick | InstrumentType::TomLow | InstrumentType::TomMid | InstrumentType::TomHigh => {
                &HARMONIC_RATIOS
            }
            InstrumentType::Snare | InstrumentType::Rimshot | InstrumentType::Clap => {
                Self::mode_ratios(resonator)
            }
            InstrumentType::HatClosed
            | InstrumentType::HatOpen
            | InstrumentType::HatPedal
            | InstrumentType::Ride
            | InstrumentType::Crash => &CYMBAL_RATIOS,
            _ => Self::mode_ratios(resonator),
        }
    }

    fn apply_organic_kit(voice: &mut DrumVoice, kit: DrumOrganicKitPreset) {
        let (body_mul, noise_mul, decay_mul, damp_mul, drive_add, tone_low_add, tone_mid_add, tone_high_add) =
            match kit {
                DrumOrganicKitPreset::NaturalStudio => (1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0),
                DrumOrganicKitPreset::DryStudio => (0.9, 0.85, 0.8, 1.1, 0.0, 0.5, 0.0, -0.4),
                DrumOrganicKitPreset::WarmTape => (1.05, 0.9, 1.05, 0.95, 0.08, 0.9, 0.3, -1.2),
                DrumOrganicKitPreset::VintageSixties => (1.0, 0.8, 0.95, 1.05, 0.03, 1.2, -0.4, -1.5),
                DrumOrganicKitPreset::VintageSeventies => (1.05, 0.85, 1.0, 1.0, 0.04, 0.8, 0.2, -1.0),
                DrumOrganicKitPreset::VintageEighties => (1.0, 0.95, 0.85, 1.05, 0.06, 0.0, 0.9, 0.8),
                DrumOrganicKitPreset::TightFunk => (0.95, 0.9, 0.78, 1.15, 0.04, 0.6, 0.7, -0.1),
                DrumOrganicKitPreset::SoulPocket => (1.08, 0.9, 1.15, 0.9, 0.03, 1.1, 0.5, -0.6),
                DrumOrganicKitPreset::MotownLite => (1.1, 0.82, 1.05, 1.0, 0.02, 1.6, -0.3, -1.8),
                DrumOrganicKitPreset::IndieRoom => (1.02, 1.05, 1.2, 0.9, 0.05, 0.2, 0.1, 0.2),
                DrumOrganicKitPreset::LoFiDust => (0.95, 0.7, 0.82, 1.08, 0.1, 1.8, -1.2, -2.0),
                DrumOrganicKitPreset::BrushKit => (0.85, 1.2, 1.05, 0.95, 0.0, -0.2, 0.7, 0.8),
                DrumOrganicKitPreset::JazzClub => (0.98, 1.05, 1.12, 0.92, 0.01, 0.2, 0.7, 0.5),
                DrumOrganicKitPreset::FolkWood => (1.08, 0.82, 1.02, 0.98, 0.01, 1.6, -0.4, -1.4),
                DrumOrganicKitPreset::PercussionWood => (1.0, 0.95, 0.92, 1.05, 0.03, 0.9, 0.2, -0.5),
                DrumOrganicKitPreset::HybridOrganic => (1.04, 1.0, 1.0, 0.97, 0.05, 0.5, 0.4, 0.3),
                DrumOrganicKitPreset::PunchyRock => (1.2, 0.9, 0.88, 1.12, 0.1, 1.0, 1.2, 0.4),
                DrumOrganicKitPreset::BigRoom => (1.1, 1.1, 1.28, 0.85, 0.07, 0.3, 0.6, 0.8),
                DrumOrganicKitPreset::ArenaLive => (1.18, 1.12, 1.35, 0.82, 0.09, 0.1, 0.9, 1.0),
                DrumOrganicKitPreset::DarkCinematic => (1.1, 0.92, 1.22, 0.9, 0.06, 2.0, -0.6, -2.2),
                DrumOrganicKitPreset::BrightPop => (0.98, 1.05, 0.92, 1.02, 0.05, -0.3, 0.9, 1.8),
                DrumOrganicKitPreset::AmbientAir => (0.92, 1.25, 1.42, 0.8, 0.02, -0.8, 0.3, 1.5),
                DrumOrganicKitPreset::BrokenToy => (0.88, 1.18, 0.72, 1.2, 0.12, -0.5, 0.5, 1.6),
                DrumOrganicKitPreset::ExperimentalFoley => (0.9, 1.3, 1.18, 0.9, 0.08, -0.7, 0.9, 1.7),
            };

        voice.body_mix = (voice.body_mix * body_mul).clamp(0.0, 1.0);
        voice.noise = (voice.noise * noise_mul).clamp(0.0, 1.0);
        voice.decay = (voice.decay * decay_mul).clamp(0.01, 1.0);
        voice.damping = (voice.damping * damp_mul).clamp(0.0, 1.0);
        voice.drive = (voice.drive + drive_add).clamp(0.0, 1.0);
        voice.tone_low_gain *= db_to_gain_safe(tone_low_add);
        voice.tone_mid_gain *= db_to_gain_safe(tone_mid_add);
        voice.tone_high_gain *= db_to_gain_safe(tone_high_add);

        if Self::is_cymbal(voice.instrument) {
            voice.cymbal_shape = (voice.cymbal_shape + tone_high_add * 0.03).clamp(0.0, 1.0);
            voice.noise_density = (voice.noise_density + noise_mul * 0.08 - 0.08).clamp(0.0, 1.0);
        }

        // Guard against restored state containing NaN/Inf values.
        voice.level = finite_or(voice.level, 0.8).clamp(0.0, 1.0);
        voice.pan = finite_or(voice.pan, 0.5).clamp(0.0, 1.0);
        voice.tone_low_gain = finite_or(voice.tone_low_gain, 1.0).max(0.0001);
        voice.tone_mid_gain = finite_or(voice.tone_mid_gain, 1.0).max(0.0001);
        voice.tone_high_gain = finite_or(voice.tone_high_gain, 1.0).max(0.0001);
    }
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

fn db_to_gain_safe(db: f32) -> f32 {
    db_to_gain(db.clamp(-24.0, 24.0))
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}
