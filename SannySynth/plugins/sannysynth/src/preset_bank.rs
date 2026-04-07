use nih_plug::prelude::{ProgramList, ProgramPreset, Param};
use nih_plug_vizia::vizia::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    FilterStyle, FilterType, ModSource, ModTarget, OscRouting, SubSynthParams, Waveform,
};

pub(crate) const GM_PROGRAM_NAMES: [&str; 128] = [
    "Acoustic Grand Piano",
    "Bright Acoustic Piano",
    "Electric Grand Piano",
    "Honky-tonk Piano",
    "Electric Piano 1",
    "Electric Piano 2",
    "Harpsichord",
    "Clavinet",
    "Celesta",
    "Glockenspiel",
    "Music Box",
    "Vibraphone",
    "Marimba",
    "Xylophone",
    "Tubular Bells",
    "Dulcimer",
    "Drawbar Organ",
    "Percussive Organ",
    "Rock Organ",
    "Church Organ",
    "Reed Organ",
    "Accordion",
    "Harmonica",
    "Tango Accordion",
    "Acoustic Guitar (nylon)",
    "Acoustic Guitar (steel)",
    "Electric Guitar (jazz)",
    "Electric Guitar (clean)",
    "Electric Guitar (muted)",
    "Overdriven Guitar",
    "Distortion Guitar",
    "Guitar Harmonics",
    "Acoustic Bass",
    "Electric Bass (finger)",
    "Electric Bass (pick)",
    "Fretless Bass",
    "Slap Bass 1",
    "Slap Bass 2",
    "Synth Bass 1",
    "Synth Bass 2",
    "Violin",
    "Viola",
    "Cello",
    "Contrabass",
    "Tremolo Strings",
    "Pizzicato Strings",
    "Orchestral Harp",
    "Timpani",
    "String Ensemble 1",
    "String Ensemble 2",
    "Synth Strings 1",
    "Synth Strings 2",
    "Choir Aahs",
    "Voice Oohs",
    "Synth Voice",
    "Orchestra Hit",
    "Trumpet",
    "Trombone",
    "Tuba",
    "Muted Trumpet",
    "French Horn",
    "Brass Section",
    "Synth Brass 1",
    "Synth Brass 2",
    "Soprano Sax",
    "Alto Sax",
    "Tenor Sax",
    "Baritone Sax",
    "Oboe",
    "English Horn",
    "Bassoon",
    "Clarinet",
    "Piccolo",
    "Flute",
    "Recorder",
    "Pan Flute",
    "Blown Bottle",
    "Shakuhachi",
    "Whistle",
    "Ocarina",
    "Lead 1 (square)",
    "Lead 2 (sawtooth)",
    "Lead 3 (calliope)",
    "Lead 4 (chiff)",
    "Lead 5 (charang)",
    "Lead 6 (voice)",
    "Lead 7 (fifths)",
    "Lead 8 (bass + lead)",
    "Pad 1 (new age)",
    "Pad 2 (warm)",
    "Pad 3 (polysynth)",
    "Pad 4 (choir)",
    "Pad 5 (bowed)",
    "Pad 6 (metallic)",
    "Pad 7 (halo)",
    "Pad 8 (sweep)",
    "FX 1 (rain)",
    "FX 2 (soundtrack)",
    "FX 3 (crystal)",
    "FX 4 (atmosphere)",
    "FX 5 (brightness)",
    "FX 6 (goblins)",
    "FX 7 (echoes)",
    "FX 8 (sci-fi)",
    "Sitar",
    "Banjo",
    "Shamisen",
    "Koto",
    "Kalimba",
    "Bag pipe",
    "Fiddle",
    "Shanai",
    "Tinkle Bell",
    "Agogo",
    "Steel Drums",
    "Woodblock",
    "Taiko Drum",
    "Melodic Tom",
    "Synth Drum",
    "Reverse Cymbal",
    "Guitar Fret Noise",
    "Breath Noise",
    "Seashore",
    "Bird Tweet",
    "Telephone Ring",
    "Helicopter",
    "Applause",
    "Gunshot",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PresetData {
    pub(crate) gain: f32,
    pub(crate) amp_attack_ms: f32,
    pub(crate) amp_release_ms: f32,
    pub(crate) waveform: f32,
    pub(crate) osc_routing: f32,
    pub(crate) osc_blend: f32,
    pub(crate) wavetable_position: f32,
    pub(crate) custom_wavetable_enable: f32,
    pub(crate) analog_enable: f32,
    pub(crate) analog_drive: f32,
    pub(crate) analog_noise: f32,
    pub(crate) analog_drift: f32,
    pub(crate) sub_level: f32,
    pub(crate) lfo1_rate: f32,
    pub(crate) lfo1_attack: f32,
    pub(crate) lfo1_shape: f32,
    pub(crate) lfo2_rate: f32,
    pub(crate) lfo2_attack: f32,
    pub(crate) lfo2_shape: f32,
    pub(crate) mod1_source: f32,
    pub(crate) mod1_target: f32,
    pub(crate) mod1_amount: f32,
    pub(crate) mod2_source: f32,
    pub(crate) mod2_target: f32,
    pub(crate) mod2_amount: f32,
    pub(crate) amp_decay_ms: f32,
    pub(crate) amp_sustain_level: f32,
    pub(crate) filter_type: f32,
    #[serde(default)]
    pub(crate) filter_style: f32,
    #[serde(default)]
    pub(crate) filter_vintage_drive: f32,
    #[serde(default)]
    pub(crate) filter_vintage_curve: f32,
    #[serde(default)]
    pub(crate) filter_vintage_mix: f32,
    #[serde(default)]
    pub(crate) filter_vintage_trim: f32,
    pub(crate) filter_cut: f32,
    pub(crate) filter_res: f32,
    pub(crate) filter_amount: f32,
    pub(crate) filter_cut_attack_ms: f32,
    pub(crate) filter_cut_decay_ms: f32,
    pub(crate) filter_cut_sustain_ms: f32,
    pub(crate) filter_cut_release_ms: f32,
    pub(crate) filter_res_attack_ms: f32,
    pub(crate) filter_res_decay_ms: f32,
    pub(crate) filter_res_sustain_ms: f32,
    pub(crate) filter_res_release_ms: f32,
    pub(crate) amp_envelope_level: f32,
    pub(crate) filter_cut_envelope_level: f32,
    pub(crate) filter_res_envelope_level: f32,
    pub(crate) vibrato_attack: f32,
    pub(crate) vibrato_intensity: f32,
    pub(crate) vibrato_rate: f32,
    pub(crate) tremolo_attack: f32,
    pub(crate) tremolo_intensity: f32,
    pub(crate) tremolo_rate: f32,
    pub(crate) vibrato_shape: f32,
    pub(crate) tremolo_shape: f32,
    pub(crate) filter_cut_env_polarity: f32,
    pub(crate) filter_res_env_polarity: f32,
    pub(crate) filter_cut_tension: f32,
    pub(crate) filter_res_tension: f32,
    pub(crate) cutoff_lfo_attack: f32,
    pub(crate) res_lfo_attack: f32,
    pub(crate) pan_lfo_attack: f32,
    pub(crate) cutoff_lfo_intensity: f32,
    pub(crate) cutoff_lfo_rate: f32,
    pub(crate) cutoff_lfo_shape: f32,
    pub(crate) res_lfo_intensity: f32,
    pub(crate) res_lfo_rate: f32,
    pub(crate) res_lfo_shape: f32,
    pub(crate) pan_lfo_intensity: f32,
    pub(crate) pan_lfo_rate: f32,
    pub(crate) pan_lfo_shape: f32,
    pub(crate) chorus_enable: f32,
    pub(crate) chorus_rate: f32,
    pub(crate) chorus_depth: f32,
    pub(crate) chorus_mix: f32,
    pub(crate) delay_enable: f32,
    pub(crate) delay_time_ms: f32,
    pub(crate) delay_feedback: f32,
    pub(crate) delay_mix: f32,
    pub(crate) reverb_enable: f32,
    pub(crate) reverb_size: f32,
    pub(crate) reverb_damp: f32,
    pub(crate) reverb_diffusion: f32,
    pub(crate) reverb_shimmer: f32,
    pub(crate) reverb_mix: f32,
    #[serde(default)]
    pub(crate) resonator_enable: f32,
    #[serde(default)]
    pub(crate) resonator_mix: f32,
    #[serde(default)]
    pub(crate) resonator_tone: f32,
    #[serde(default)]
    pub(crate) resonator_shape: f32,
    #[serde(default)]
    pub(crate) resonator_timbre: f32,
    #[serde(default)]
    pub(crate) resonator_damping: f32,
    pub(crate) multi_filter_enable: f32,
    pub(crate) multi_filter_routing: f32,
    pub(crate) multi_filter_morph: f32,
    pub(crate) multi_filter_parallel_ab: f32,
    pub(crate) multi_filter_parallel_c: f32,
    pub(crate) multi_filter_a_type: f32,
    #[serde(default)]
    pub(crate) multi_filter_a_style: f32,
    #[serde(default)]
    pub(crate) multi_filter_a_drive: f32,
    #[serde(default)]
    pub(crate) multi_filter_a_curve: f32,
    #[serde(default)]
    pub(crate) multi_filter_a_mix: f32,
    #[serde(default)]
    pub(crate) multi_filter_a_trim: f32,
    pub(crate) multi_filter_a_cut: f32,
    pub(crate) multi_filter_a_res: f32,
    pub(crate) multi_filter_a_amt: f32,
    pub(crate) multi_filter_b_type: f32,
    #[serde(default)]
    pub(crate) multi_filter_b_style: f32,
    #[serde(default)]
    pub(crate) multi_filter_b_drive: f32,
    #[serde(default)]
    pub(crate) multi_filter_b_curve: f32,
    #[serde(default)]
    pub(crate) multi_filter_b_mix: f32,
    #[serde(default)]
    pub(crate) multi_filter_b_trim: f32,
    pub(crate) multi_filter_b_cut: f32,
    pub(crate) multi_filter_b_res: f32,
    pub(crate) multi_filter_b_amt: f32,
    pub(crate) multi_filter_c_type: f32,
    #[serde(default)]
    pub(crate) multi_filter_c_style: f32,
    #[serde(default)]
    pub(crate) multi_filter_c_drive: f32,
    #[serde(default)]
    pub(crate) multi_filter_c_curve: f32,
    #[serde(default)]
    pub(crate) multi_filter_c_mix: f32,
    #[serde(default)]
    pub(crate) multi_filter_c_trim: f32,
    pub(crate) multi_filter_c_cut: f32,
    pub(crate) multi_filter_c_res: f32,
    pub(crate) multi_filter_c_amt: f32,
    pub(crate) limiter_enable: f32,
    pub(crate) limiter_threshold: f32,
    pub(crate) limiter_release: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct PresetEntry {
    pub(crate) name: String,
    pub(crate) data: PresetData,
    #[allow(dead_code)]
    pub(crate) user: bool,
}

pub(crate) fn normalized<P: Param>(param: &P, plain: P::Plain) -> f32 {
    param.preview_normalized(plain)
}

impl PresetData {
    pub(crate) fn from_params(params: &SubSynthParams) -> Self {
        Self {
            gain: params.gain.unmodulated_normalized_value(),
            amp_attack_ms: params.amp_attack_ms.unmodulated_normalized_value(),
            amp_release_ms: params.amp_release_ms.unmodulated_normalized_value(),
            waveform: params.waveform.unmodulated_normalized_value(),
            osc_routing: params.osc_routing.unmodulated_normalized_value(),
            osc_blend: params.osc_blend.unmodulated_normalized_value(),
            wavetable_position: params.wavetable_position.unmodulated_normalized_value(),
            custom_wavetable_enable: params.custom_wavetable_enable.unmodulated_normalized_value(),
            analog_enable: params.analog_enable.unmodulated_normalized_value(),
            analog_drive: params.analog_drive.unmodulated_normalized_value(),
            analog_noise: params.analog_noise.unmodulated_normalized_value(),
            analog_drift: params.analog_drift.unmodulated_normalized_value(),
            sub_level: params.sub_level.unmodulated_normalized_value(),
            lfo1_rate: params.lfo1_rate.unmodulated_normalized_value(),
            lfo1_attack: params.lfo1_attack.unmodulated_normalized_value(),
            lfo1_shape: params.lfo1_shape.unmodulated_normalized_value(),
            lfo2_rate: params.lfo2_rate.unmodulated_normalized_value(),
            lfo2_attack: params.lfo2_attack.unmodulated_normalized_value(),
            lfo2_shape: params.lfo2_shape.unmodulated_normalized_value(),
            mod1_source: params.mod1_source.unmodulated_normalized_value(),
            mod1_target: params.mod1_target.unmodulated_normalized_value(),
            mod1_amount: params.mod1_amount.unmodulated_normalized_value(),
            mod2_source: params.mod2_source.unmodulated_normalized_value(),
            mod2_target: params.mod2_target.unmodulated_normalized_value(),
            mod2_amount: params.mod2_amount.unmodulated_normalized_value(),
            amp_decay_ms: params.amp_decay_ms.unmodulated_normalized_value(),
            amp_sustain_level: params.amp_sustain_level.unmodulated_normalized_value(),
            filter_type: params.filter_type.unmodulated_normalized_value(),
            filter_style: params.filter_style.unmodulated_normalized_value(),
            filter_vintage_drive: params.filter_vintage_drive.unmodulated_normalized_value(),
            filter_vintage_curve: params.filter_vintage_curve.unmodulated_normalized_value(),
            filter_vintage_mix: params.filter_vintage_mix.unmodulated_normalized_value(),
            filter_vintage_trim: params.filter_vintage_trim.unmodulated_normalized_value(),
            filter_cut: params.filter_cut.unmodulated_normalized_value(),
            filter_res: params.filter_res.unmodulated_normalized_value(),
            filter_amount: params.filter_amount.unmodulated_normalized_value(),
            filter_cut_attack_ms: params.filter_cut_attack_ms.unmodulated_normalized_value(),
            filter_cut_decay_ms: params.filter_cut_decay_ms.unmodulated_normalized_value(),
            filter_cut_sustain_ms: params.filter_cut_sustain_ms.unmodulated_normalized_value(),
            filter_cut_release_ms: params.filter_cut_release_ms.unmodulated_normalized_value(),
            filter_res_attack_ms: params.filter_res_attack_ms.unmodulated_normalized_value(),
            filter_res_decay_ms: params.filter_res_decay_ms.unmodulated_normalized_value(),
            filter_res_sustain_ms: params.filter_res_sustain_ms.unmodulated_normalized_value(),
            filter_res_release_ms: params.filter_res_release_ms.unmodulated_normalized_value(),
            amp_envelope_level: params.amp_envelope_level.unmodulated_normalized_value(),
            filter_cut_envelope_level: params.filter_cut_envelope_level.unmodulated_normalized_value(),
            filter_res_envelope_level: params.filter_res_envelope_level.unmodulated_normalized_value(),
            vibrato_attack: params.vibrato_attack.unmodulated_normalized_value(),
            vibrato_intensity: params.vibrato_intensity.unmodulated_normalized_value(),
            vibrato_rate: params.vibrato_rate.unmodulated_normalized_value(),
            tremolo_attack: params.tremolo_attack.unmodulated_normalized_value(),
            tremolo_intensity: params.tremolo_intensity.unmodulated_normalized_value(),
            tremolo_rate: params.tremolo_rate.unmodulated_normalized_value(),
            vibrato_shape: params.vibrato_shape.unmodulated_normalized_value(),
            tremolo_shape: params.tremolo_shape.unmodulated_normalized_value(),
            filter_cut_env_polarity: params.filter_cut_env_polarity.unmodulated_normalized_value(),
            filter_res_env_polarity: params.filter_res_env_polarity.unmodulated_normalized_value(),
            filter_cut_tension: params.filter_cut_tension.unmodulated_normalized_value(),
            filter_res_tension: params.filter_res_tension.unmodulated_normalized_value(),
            cutoff_lfo_attack: params.cutoff_lfo_attack.unmodulated_normalized_value(),
            res_lfo_attack: params.res_lfo_attack.unmodulated_normalized_value(),
            pan_lfo_attack: params.pan_lfo_attack.unmodulated_normalized_value(),
            cutoff_lfo_intensity: params.cutoff_lfo_intensity.unmodulated_normalized_value(),
            cutoff_lfo_rate: params.cutoff_lfo_rate.unmodulated_normalized_value(),
            cutoff_lfo_shape: params.cutoff_lfo_shape.unmodulated_normalized_value(),
            res_lfo_intensity: params.res_lfo_intensity.unmodulated_normalized_value(),
            res_lfo_rate: params.res_lfo_rate.unmodulated_normalized_value(),
            res_lfo_shape: params.res_lfo_shape.unmodulated_normalized_value(),
            pan_lfo_intensity: params.pan_lfo_intensity.unmodulated_normalized_value(),
            pan_lfo_rate: params.pan_lfo_rate.unmodulated_normalized_value(),
            pan_lfo_shape: params.pan_lfo_shape.unmodulated_normalized_value(),
            chorus_enable: params.chorus_enable.unmodulated_normalized_value(),
            chorus_rate: params.chorus_rate.unmodulated_normalized_value(),
            chorus_depth: params.chorus_depth.unmodulated_normalized_value(),
            chorus_mix: params.chorus_mix.unmodulated_normalized_value(),
            delay_enable: params.delay_enable.unmodulated_normalized_value(),
            delay_time_ms: params.delay_time_ms.unmodulated_normalized_value(),
            delay_feedback: params.delay_feedback.unmodulated_normalized_value(),
            delay_mix: params.delay_mix.unmodulated_normalized_value(),
            reverb_enable: params.reverb_enable.unmodulated_normalized_value(),
            reverb_size: params.reverb_size.unmodulated_normalized_value(),
            reverb_damp: params.reverb_damp.unmodulated_normalized_value(),
            reverb_diffusion: params.reverb_diffusion.unmodulated_normalized_value(),
            reverb_shimmer: params.reverb_shimmer.unmodulated_normalized_value(),
            reverb_mix: params.reverb_mix.unmodulated_normalized_value(),
            resonator_enable: params.resonator_enable.unmodulated_normalized_value(),
            resonator_mix: params.resonator_mix.unmodulated_normalized_value(),
            resonator_tone: params.resonator_tone.unmodulated_normalized_value(),
            resonator_shape: params.resonator_shape.unmodulated_normalized_value(),
            resonator_timbre: params.resonator_timbre.unmodulated_normalized_value(),
            resonator_damping: params.resonator_damping.unmodulated_normalized_value(),
            multi_filter_enable: params.multi_filter_enable.unmodulated_normalized_value(),
            multi_filter_routing: params.multi_filter_routing.unmodulated_normalized_value(),
            multi_filter_morph: params.multi_filter_morph.unmodulated_normalized_value(),
            multi_filter_parallel_ab: params.multi_filter_parallel_ab.unmodulated_normalized_value(),
            multi_filter_parallel_c: params.multi_filter_parallel_c.unmodulated_normalized_value(),
            multi_filter_a_type: params.multi_filter_a_type.unmodulated_normalized_value(),
            multi_filter_a_style: params.multi_filter_a_style.unmodulated_normalized_value(),
            multi_filter_a_drive: params.multi_filter_a_drive.unmodulated_normalized_value(),
            multi_filter_a_curve: params.multi_filter_a_curve.unmodulated_normalized_value(),
            multi_filter_a_mix: params.multi_filter_a_mix.unmodulated_normalized_value(),
            multi_filter_a_trim: params.multi_filter_a_trim.unmodulated_normalized_value(),
            multi_filter_a_cut: params.multi_filter_a_cut.unmodulated_normalized_value(),
            multi_filter_a_res: params.multi_filter_a_res.unmodulated_normalized_value(),
            multi_filter_a_amt: params.multi_filter_a_amt.unmodulated_normalized_value(),
            multi_filter_b_type: params.multi_filter_b_type.unmodulated_normalized_value(),
            multi_filter_b_style: params.multi_filter_b_style.unmodulated_normalized_value(),
            multi_filter_b_drive: params.multi_filter_b_drive.unmodulated_normalized_value(),
            multi_filter_b_curve: params.multi_filter_b_curve.unmodulated_normalized_value(),
            multi_filter_b_mix: params.multi_filter_b_mix.unmodulated_normalized_value(),
            multi_filter_b_trim: params.multi_filter_b_trim.unmodulated_normalized_value(),
            multi_filter_b_cut: params.multi_filter_b_cut.unmodulated_normalized_value(),
            multi_filter_b_res: params.multi_filter_b_res.unmodulated_normalized_value(),
            multi_filter_b_amt: params.multi_filter_b_amt.unmodulated_normalized_value(),
            multi_filter_c_type: params.multi_filter_c_type.unmodulated_normalized_value(),
            multi_filter_c_style: params.multi_filter_c_style.unmodulated_normalized_value(),
            multi_filter_c_drive: params.multi_filter_c_drive.unmodulated_normalized_value(),
            multi_filter_c_curve: params.multi_filter_c_curve.unmodulated_normalized_value(),
            multi_filter_c_mix: params.multi_filter_c_mix.unmodulated_normalized_value(),
            multi_filter_c_trim: params.multi_filter_c_trim.unmodulated_normalized_value(),
            multi_filter_c_cut: params.multi_filter_c_cut.unmodulated_normalized_value(),
            multi_filter_c_res: params.multi_filter_c_res.unmodulated_normalized_value(),
            multi_filter_c_amt: params.multi_filter_c_amt.unmodulated_normalized_value(),
            limiter_enable: params.limiter_enable.unmodulated_normalized_value(),
            limiter_threshold: params.limiter_threshold.unmodulated_normalized_value(),
            limiter_release: params.limiter_release.unmodulated_normalized_value(),
        }
    }

    pub(crate) fn apply(&self, cx: &mut EventContext, params: &SubSynthParams) {
        apply_param(cx, &params.gain, self.gain);
        apply_param(cx, &params.amp_attack_ms, self.amp_attack_ms);
        apply_param(cx, &params.amp_release_ms, self.amp_release_ms);
        apply_param(cx, &params.waveform, self.waveform);
        apply_param(cx, &params.osc_routing, self.osc_routing);
        apply_param(cx, &params.osc_blend, self.osc_blend);
        apply_param(cx, &params.wavetable_position, self.wavetable_position);
        apply_param(cx, &params.custom_wavetable_enable, self.custom_wavetable_enable);
        apply_param(cx, &params.analog_enable, self.analog_enable);
        apply_param(cx, &params.analog_drive, self.analog_drive);
        apply_param(cx, &params.analog_noise, self.analog_noise);
        apply_param(cx, &params.analog_drift, self.analog_drift);
        apply_param(cx, &params.sub_level, self.sub_level);
        apply_param(cx, &params.lfo1_rate, self.lfo1_rate);
        apply_param(cx, &params.lfo1_attack, self.lfo1_attack);
        apply_param(cx, &params.lfo1_shape, self.lfo1_shape);
        apply_param(cx, &params.lfo2_rate, self.lfo2_rate);
        apply_param(cx, &params.lfo2_attack, self.lfo2_attack);
        apply_param(cx, &params.lfo2_shape, self.lfo2_shape);
        apply_param(cx, &params.mod1_source, self.mod1_source);
        apply_param(cx, &params.mod1_target, self.mod1_target);
        apply_param(cx, &params.mod1_amount, self.mod1_amount);
        apply_param(cx, &params.mod2_source, self.mod2_source);
        apply_param(cx, &params.mod2_target, self.mod2_target);
        apply_param(cx, &params.mod2_amount, self.mod2_amount);
        apply_param(cx, &params.amp_decay_ms, self.amp_decay_ms);
        apply_param(cx, &params.amp_sustain_level, self.amp_sustain_level);
        apply_param(cx, &params.filter_type, self.filter_type);
        apply_param(cx, &params.filter_style, self.filter_style);
        apply_param(cx, &params.filter_vintage_drive, self.filter_vintage_drive);
        apply_param(cx, &params.filter_vintage_curve, self.filter_vintage_curve);
        apply_param(cx, &params.filter_vintage_mix, self.filter_vintage_mix);
        apply_param(cx, &params.filter_vintage_trim, self.filter_vintage_trim);
        apply_param(cx, &params.filter_cut, self.filter_cut);
        apply_param(cx, &params.filter_res, self.filter_res);
        apply_param(cx, &params.filter_amount, self.filter_amount);
        apply_param(cx, &params.filter_cut_attack_ms, self.filter_cut_attack_ms);
        apply_param(cx, &params.filter_cut_decay_ms, self.filter_cut_decay_ms);
        apply_param(cx, &params.filter_cut_sustain_ms, self.filter_cut_sustain_ms);
        apply_param(cx, &params.filter_cut_release_ms, self.filter_cut_release_ms);
        apply_param(cx, &params.filter_res_attack_ms, self.filter_res_attack_ms);
        apply_param(cx, &params.filter_res_decay_ms, self.filter_res_decay_ms);
        apply_param(cx, &params.filter_res_sustain_ms, self.filter_res_sustain_ms);
        apply_param(cx, &params.filter_res_release_ms, self.filter_res_release_ms);
        apply_param(cx, &params.amp_envelope_level, self.amp_envelope_level);
        apply_param(cx, &params.filter_cut_envelope_level, self.filter_cut_envelope_level);
        apply_param(cx, &params.filter_res_envelope_level, self.filter_res_envelope_level);
        apply_param(cx, &params.vibrato_attack, self.vibrato_attack);
        apply_param(cx, &params.vibrato_intensity, self.vibrato_intensity);
        apply_param(cx, &params.vibrato_rate, self.vibrato_rate);
        apply_param(cx, &params.tremolo_attack, self.tremolo_attack);
        apply_param(cx, &params.tremolo_intensity, self.tremolo_intensity);
        apply_param(cx, &params.tremolo_rate, self.tremolo_rate);
        apply_param(cx, &params.vibrato_shape, self.vibrato_shape);
        apply_param(cx, &params.tremolo_shape, self.tremolo_shape);
        apply_param(cx, &params.filter_cut_env_polarity, self.filter_cut_env_polarity);
        apply_param(cx, &params.filter_res_env_polarity, self.filter_res_env_polarity);
        apply_param(cx, &params.filter_cut_tension, self.filter_cut_tension);
        apply_param(cx, &params.filter_res_tension, self.filter_res_tension);
        apply_param(cx, &params.cutoff_lfo_attack, self.cutoff_lfo_attack);
        apply_param(cx, &params.res_lfo_attack, self.res_lfo_attack);
        apply_param(cx, &params.pan_lfo_attack, self.pan_lfo_attack);
        apply_param(cx, &params.cutoff_lfo_intensity, self.cutoff_lfo_intensity);
        apply_param(cx, &params.cutoff_lfo_rate, self.cutoff_lfo_rate);
        apply_param(cx, &params.cutoff_lfo_shape, self.cutoff_lfo_shape);
        apply_param(cx, &params.res_lfo_intensity, self.res_lfo_intensity);
        apply_param(cx, &params.res_lfo_rate, self.res_lfo_rate);
        apply_param(cx, &params.res_lfo_shape, self.res_lfo_shape);
        apply_param(cx, &params.pan_lfo_intensity, self.pan_lfo_intensity);
        apply_param(cx, &params.pan_lfo_rate, self.pan_lfo_rate);
        apply_param(cx, &params.pan_lfo_shape, self.pan_lfo_shape);
        apply_param(cx, &params.chorus_enable, self.chorus_enable);
        apply_param(cx, &params.chorus_rate, self.chorus_rate);
        apply_param(cx, &params.chorus_depth, self.chorus_depth);
        apply_param(cx, &params.chorus_mix, self.chorus_mix);
        apply_param(cx, &params.delay_enable, self.delay_enable);
        apply_param(cx, &params.delay_time_ms, self.delay_time_ms);
        apply_param(cx, &params.delay_feedback, self.delay_feedback);
        apply_param(cx, &params.delay_mix, self.delay_mix);
        apply_param(cx, &params.reverb_enable, self.reverb_enable);
        apply_param(cx, &params.reverb_size, self.reverb_size);
        apply_param(cx, &params.reverb_damp, self.reverb_damp);
        apply_param(cx, &params.reverb_diffusion, self.reverb_diffusion);
        apply_param(cx, &params.reverb_shimmer, self.reverb_shimmer);
        apply_param(cx, &params.reverb_mix, self.reverb_mix);
        apply_param(cx, &params.resonator_enable, self.resonator_enable);
        apply_param(cx, &params.resonator_mix, self.resonator_mix);
        apply_param(cx, &params.resonator_tone, self.resonator_tone);
        apply_param(cx, &params.resonator_shape, self.resonator_shape);
        apply_param(cx, &params.resonator_timbre, self.resonator_timbre);
        apply_param(cx, &params.resonator_damping, self.resonator_damping);
        apply_param(cx, &params.multi_filter_enable, self.multi_filter_enable);
        apply_param(cx, &params.multi_filter_routing, self.multi_filter_routing);
        apply_param(cx, &params.multi_filter_morph, self.multi_filter_morph);
        apply_param(cx, &params.multi_filter_parallel_ab, self.multi_filter_parallel_ab);
        apply_param(cx, &params.multi_filter_parallel_c, self.multi_filter_parallel_c);
        apply_param(cx, &params.multi_filter_a_type, self.multi_filter_a_type);
        apply_param(cx, &params.multi_filter_a_style, self.multi_filter_a_style);
        apply_param(cx, &params.multi_filter_a_drive, self.multi_filter_a_drive);
        apply_param(cx, &params.multi_filter_a_curve, self.multi_filter_a_curve);
        apply_param(cx, &params.multi_filter_a_mix, self.multi_filter_a_mix);
        apply_param(cx, &params.multi_filter_a_trim, self.multi_filter_a_trim);
        apply_param(cx, &params.multi_filter_a_cut, self.multi_filter_a_cut);
        apply_param(cx, &params.multi_filter_a_res, self.multi_filter_a_res);
        apply_param(cx, &params.multi_filter_a_amt, self.multi_filter_a_amt);
        apply_param(cx, &params.multi_filter_b_type, self.multi_filter_b_type);
        apply_param(cx, &params.multi_filter_b_style, self.multi_filter_b_style);
        apply_param(cx, &params.multi_filter_b_drive, self.multi_filter_b_drive);
        apply_param(cx, &params.multi_filter_b_curve, self.multi_filter_b_curve);
        apply_param(cx, &params.multi_filter_b_mix, self.multi_filter_b_mix);
        apply_param(cx, &params.multi_filter_b_trim, self.multi_filter_b_trim);
        apply_param(cx, &params.multi_filter_b_cut, self.multi_filter_b_cut);
        apply_param(cx, &params.multi_filter_b_res, self.multi_filter_b_res);
        apply_param(cx, &params.multi_filter_b_amt, self.multi_filter_b_amt);
        apply_param(cx, &params.multi_filter_c_type, self.multi_filter_c_type);
        apply_param(cx, &params.multi_filter_c_style, self.multi_filter_c_style);
        apply_param(cx, &params.multi_filter_c_drive, self.multi_filter_c_drive);
        apply_param(cx, &params.multi_filter_c_curve, self.multi_filter_c_curve);
        apply_param(cx, &params.multi_filter_c_mix, self.multi_filter_c_mix);
        apply_param(cx, &params.multi_filter_c_trim, self.multi_filter_c_trim);
        apply_param(cx, &params.multi_filter_c_cut, self.multi_filter_c_cut);
        apply_param(cx, &params.multi_filter_c_res, self.multi_filter_c_res);
        apply_param(cx, &params.multi_filter_c_amt, self.multi_filter_c_amt);
        apply_param(cx, &params.limiter_enable, self.limiter_enable);
        apply_param(cx, &params.limiter_threshold, self.limiter_threshold);
        apply_param(cx, &params.limiter_release, self.limiter_release);
    }

    pub(crate) fn program_preset(&self, name: String) -> ProgramPreset {
        let mut values = Vec::new();
        push_value(&mut values, "gain", self.gain);
        push_value(&mut values, "amp_atk", self.amp_attack_ms);
        push_value(&mut values, "amp_rel", self.amp_release_ms);
        push_value(&mut values, "waveform", self.waveform);
        push_value(&mut values, "osc_route", self.osc_routing);
        push_value(&mut values, "osc_blend", self.osc_blend);
        push_value(&mut values, "wt_pos", self.wavetable_position);
        push_value(&mut values, "wt_custom", self.custom_wavetable_enable);
        push_value(&mut values, "analog_en", self.analog_enable);
        push_value(&mut values, "analog_drive", self.analog_drive);
        push_value(&mut values, "analog_noise", self.analog_noise);
        push_value(&mut values, "analog_drift", self.analog_drift);
        push_value(&mut values, "sub_level", self.sub_level);
        push_value(&mut values, "lfo1_rate", self.lfo1_rate);
        push_value(&mut values, "lfo1_atk", self.lfo1_attack);
        push_value(&mut values, "lfo1_shape", self.lfo1_shape);
        push_value(&mut values, "lfo2_rate", self.lfo2_rate);
        push_value(&mut values, "lfo2_atk", self.lfo2_attack);
        push_value(&mut values, "lfo2_shape", self.lfo2_shape);
        push_value(&mut values, "mod1_src", self.mod1_source);
        push_value(&mut values, "mod1_tgt", self.mod1_target);
        push_value(&mut values, "mod1_amt", self.mod1_amount);
        push_value(&mut values, "mod2_src", self.mod2_source);
        push_value(&mut values, "mod2_tgt", self.mod2_target);
        push_value(&mut values, "mod2_amt", self.mod2_amount);
        push_value(&mut values, "amp_dec", self.amp_decay_ms);
        push_value(&mut values, "amp_sus", self.amp_sustain_level);
        push_value(&mut values, "filter_type", self.filter_type);
        push_value(&mut values, "filter_style", self.filter_style);
        push_value(&mut values, "filter_drive", self.filter_vintage_drive);
        push_value(&mut values, "filter_curve", self.filter_vintage_curve);
        push_value(&mut values, "filter_mix", self.filter_vintage_mix);
        push_value(&mut values, "filter_trim", self.filter_vintage_trim);
        push_value(&mut values, "filter_cut", self.filter_cut);
        push_value(&mut values, "filter_res", self.filter_res);
        push_value(&mut values, "filter_amount", self.filter_amount);
        push_value(&mut values, "filter_cut_atk", self.filter_cut_attack_ms);
        push_value(&mut values, "filter_cut_dec", self.filter_cut_decay_ms);
        push_value(&mut values, "filter_cut_sus", self.filter_cut_sustain_ms);
        push_value(&mut values, "filter_cut_rel", self.filter_cut_release_ms);
        push_value(&mut values, "filter_res_atk", self.filter_res_attack_ms);
        push_value(&mut values, "filter_res_dec", self.filter_res_decay_ms);
        push_value(&mut values, "filter_res_sus", self.filter_res_sustain_ms);
        push_value(&mut values, "filter_res_rel", self.filter_res_release_ms);
        push_value(&mut values, "amp_env_level", self.amp_envelope_level);
        push_value(&mut values, "filter_cut_env_level", self.filter_cut_envelope_level);
        push_value(&mut values, "filter_res_env_level", self.filter_res_envelope_level);
        push_value(&mut values, "vibrato_atk", self.vibrato_attack);
        push_value(&mut values, "vibrato_int", self.vibrato_intensity);
        push_value(&mut values, "vibrato_rate", self.vibrato_rate);
        push_value(&mut values, "tremolo_atk", self.tremolo_attack);
        push_value(&mut values, "tremolo_int", self.tremolo_intensity);
        push_value(&mut values, "tremolo_rate", self.tremolo_rate);
        push_value(&mut values, "vibrato_shape", self.vibrato_shape);
        push_value(&mut values, "tremolo_shape", self.tremolo_shape);
        push_value(&mut values, "filter_cut_env_pol", self.filter_cut_env_polarity);
        push_value(&mut values, "filter_res_env_pol", self.filter_res_env_polarity);
        push_value(&mut values, "filter_cut_tension", self.filter_cut_tension);
        push_value(&mut values, "filter_res_tension", self.filter_res_tension);
        push_value(&mut values, "cutoff_lfo_attack", self.cutoff_lfo_attack);
        push_value(&mut values, "res_lfo_attack", self.res_lfo_attack);
        push_value(&mut values, "pan_lfo_attack", self.pan_lfo_attack);
        push_value(&mut values, "cutoff_lfo_int", self.cutoff_lfo_intensity);
        push_value(&mut values, "cutoff_lfo_rate", self.cutoff_lfo_rate);
        push_value(&mut values, "cutoff_lfo_shape", self.cutoff_lfo_shape);
        push_value(&mut values, "res_lfo_int", self.res_lfo_intensity);
        push_value(&mut values, "res_lfo_rate", self.res_lfo_rate);
        push_value(&mut values, "res_lfo_shape", self.res_lfo_shape);
        push_value(&mut values, "pan_lfo_int", self.pan_lfo_intensity);
        push_value(&mut values, "pan_lfo_rate", self.pan_lfo_rate);
        push_value(&mut values, "pan_lfo_shape", self.pan_lfo_shape);
        push_value(&mut values, "chorus_enable", self.chorus_enable);
        push_value(&mut values, "chorus_rate", self.chorus_rate);
        push_value(&mut values, "chorus_depth", self.chorus_depth);
        push_value(&mut values, "chorus_mix", self.chorus_mix);
        push_value(&mut values, "delay_en", self.delay_enable);
        push_value(&mut values, "delay_time", self.delay_time_ms);
        push_value(&mut values, "delay_fb", self.delay_feedback);
        push_value(&mut values, "delay_mix", self.delay_mix);
        push_value(&mut values, "rev_en", self.reverb_enable);
        push_value(&mut values, "rev_size", self.reverb_size);
        push_value(&mut values, "rev_damp", self.reverb_damp);
        push_value(&mut values, "rev_diff", self.reverb_diffusion);
        push_value(&mut values, "rev_shim", self.reverb_shimmer);
        push_value(&mut values, "rev_mix", self.reverb_mix);
        push_value(&mut values, "res_en", self.resonator_enable);
        push_value(&mut values, "res_mix", self.resonator_mix);
        push_value(&mut values, "res_tone", self.resonator_tone);
        push_value(&mut values, "res_shape", self.resonator_shape);
        push_value(&mut values, "res_map", self.resonator_timbre);
        push_value(&mut values, "res_damp", self.resonator_damping);
        push_value(&mut values, "mf_en", self.multi_filter_enable);
        push_value(&mut values, "mf_route", self.multi_filter_routing);
        push_value(&mut values, "mf_morph", self.multi_filter_morph);
        push_value(&mut values, "mf_par_ab", self.multi_filter_parallel_ab);
        push_value(&mut values, "mf_par_c", self.multi_filter_parallel_c);
        push_value(&mut values, "mf_a_type", self.multi_filter_a_type);
        push_value(&mut values, "mf_a_style", self.multi_filter_a_style);
        push_value(&mut values, "mf_a_drive", self.multi_filter_a_drive);
        push_value(&mut values, "mf_a_curve", self.multi_filter_a_curve);
        push_value(&mut values, "mf_a_mix", self.multi_filter_a_mix);
        push_value(&mut values, "mf_a_trim", self.multi_filter_a_trim);
        push_value(&mut values, "mf_a_cut", self.multi_filter_a_cut);
        push_value(&mut values, "mf_a_res", self.multi_filter_a_res);
        push_value(&mut values, "mf_a_amt", self.multi_filter_a_amt);
        push_value(&mut values, "mf_b_type", self.multi_filter_b_type);
        push_value(&mut values, "mf_b_style", self.multi_filter_b_style);
        push_value(&mut values, "mf_b_drive", self.multi_filter_b_drive);
        push_value(&mut values, "mf_b_curve", self.multi_filter_b_curve);
        push_value(&mut values, "mf_b_mix", self.multi_filter_b_mix);
        push_value(&mut values, "mf_b_trim", self.multi_filter_b_trim);
        push_value(&mut values, "mf_b_cut", self.multi_filter_b_cut);
        push_value(&mut values, "mf_b_res", self.multi_filter_b_res);
        push_value(&mut values, "mf_b_amt", self.multi_filter_b_amt);
        push_value(&mut values, "mf_c_type", self.multi_filter_c_type);
        push_value(&mut values, "mf_c_style", self.multi_filter_c_style);
        push_value(&mut values, "mf_c_drive", self.multi_filter_c_drive);
        push_value(&mut values, "mf_c_curve", self.multi_filter_c_curve);
        push_value(&mut values, "mf_c_mix", self.multi_filter_c_mix);
        push_value(&mut values, "mf_c_trim", self.multi_filter_c_trim);
        push_value(&mut values, "mf_c_cut", self.multi_filter_c_cut);
        push_value(&mut values, "mf_c_res", self.multi_filter_c_res);
        push_value(&mut values, "mf_c_amt", self.multi_filter_c_amt);
        push_value(&mut values, "limiter_enable", self.limiter_enable);
        push_value(&mut values, "limiter_threshold", self.limiter_threshold);
        push_value(&mut values, "limiter_release", self.limiter_release);

        ProgramPreset { name, values }
    }
}

pub(crate) fn factory_presets(params: &SubSynthParams) -> Vec<PresetEntry> {
    gm_presets(params)
        .into_iter()
        .map(|(name, data)| PresetEntry {
            name,
            data,
            user: false,
        })
        .collect()
}

pub(crate) fn program_list(params: &SubSynthParams) -> ProgramList {
    let programs = gm_presets(params)
        .into_iter()
        .map(|(name, data)| data.program_preset(name))
        .collect();

    ProgramList {
        name: "GM/PSR-128".to_string(),
        programs,
    }
}

fn push_value(values: &mut Vec<(String, f32)>, id: &str, value: f32) {
    values.push((id.to_string(), value));
}

fn variation(index: usize, salt: u32) -> f32 {
    let raw = (index as u32 * 37 + salt * 101 + 13) % 100;
    raw as f32 / 100.0
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
    min + (max - min) * t
}

enum GmGroup {
    Piano,
    Chromatic,
    Organ,
    Guitar,
    Bass,
    Strings,
    Ensemble,
    Brass,
    Reed,
    Pipe,
    SynthLead,
    SynthPad,
    SynthFx,
    Ethnic,
    Percussive,
    Sfx,
}

fn group_for(index: usize) -> GmGroup {
    match index / 8 {
        0 => GmGroup::Piano,
        1 => GmGroup::Chromatic,
        2 => GmGroup::Organ,
        3 => GmGroup::Guitar,
        4 => GmGroup::Bass,
        5 => GmGroup::Strings,
        6 => GmGroup::Ensemble,
        7 => GmGroup::Brass,
        8 => GmGroup::Reed,
        9 => GmGroup::Pipe,
        10 => GmGroup::SynthLead,
        11 => GmGroup::SynthPad,
        12 => GmGroup::SynthFx,
        13 => GmGroup::Ethnic,
        14 => GmGroup::Percussive,
        _ => GmGroup::Sfx,
    }
}

fn gm_presets(params: &SubSynthParams) -> Vec<(String, PresetData)> {
    let mut presets = Vec::with_capacity(128);

    for index in 0..128 {
        let name = GM_PROGRAM_NAMES[index].to_string();
        let mut data = PresetData::from_params(params);
        let group = group_for(index);
        let flavor = variation(index, 3);
        let motion = variation(index, 7);
        let tone = variation(index, 11);
        let vintage = index % 2 == 0;

        data.custom_wavetable_enable = normalized(&params.custom_wavetable_enable, false);
        data.analog_enable = normalized(&params.analog_enable, vintage);
        data.analog_drive = normalized(&params.analog_drive, lerp(0.05, 0.35, tone));
        data.analog_noise = normalized(&params.analog_noise, lerp(0.0, 0.2, motion));
        data.analog_drift = normalized(&params.analog_drift, lerp(0.05, 0.35, motion));
        data.filter_style = normalized(
            &params.filter_style,
            if vintage { FilterStyle::Vintage } else { FilterStyle::Digital },
        );
        data.filter_vintage_drive = normalized(&params.filter_vintage_drive, lerp(0.2, 0.7, tone));
        data.filter_vintage_curve = normalized(&params.filter_vintage_curve, lerp(0.25, 0.75, flavor));
        data.filter_vintage_mix = normalized(&params.filter_vintage_mix, lerp(0.6, 1.0, motion));
        data.filter_vintage_trim = normalized(&params.filter_vintage_trim, lerp(0.9, 1.15, tone));

        match group {
            GmGroup::Piano => {
                data.waveform = normalized(&params.waveform, Waveform::Triangle);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.3, 0.7, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.35, 0.75, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(2.0, 6.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(8.0, 25.0, tone));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.6, 0.85, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(6.0, 15.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(900.0, 4200.0, flavor));
                data.filter_res = normalized(&params.filter_res, lerp(0.15, 0.45, motion));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.2, 0.4, motion));
            }
            GmGroup::Chromatic => {
                data.waveform = normalized(&params.waveform, Waveform::Pulse);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.6, 1.0, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.1, 0.9, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.2, 2.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(2.0, 10.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.2, 0.6, flavor));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(2.0, 6.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(1200.0, 5200.0, tone));
                data.filter_res = normalized(&params.filter_res, lerp(0.35, 0.7, motion));
                data.delay_enable = normalized(&params.delay_enable, true);
                data.delay_time_ms = normalized(&params.delay_time_ms, lerp(120.0, 480.0, flavor));
                data.delay_mix = normalized(&params.delay_mix, lerp(0.15, 0.35, motion));
            }
            GmGroup::Organ => {
                data.waveform = normalized(&params.waveform, Waveform::Square);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.4, 0.6, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.2, 0.7, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.3, 1.2, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(2.0, 6.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.8, 0.95, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(4.0, 9.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Highpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(300.0, 1400.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.45, flavor));
                data.chorus_enable = normalized(&params.chorus_enable, true);
                data.chorus_mix = normalized(&params.chorus_mix, lerp(0.2, 0.5, motion));
            }
            GmGroup::Guitar => {
                data.waveform = normalized(&params.waveform, Waveform::Sawtooth);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.25, 0.6, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.2, 0.8, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.4, 2.5, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(5.0, 15.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.3, 0.7, flavor));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(3.0, 8.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(650.0, 2400.0, tone));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.6, motion));
                data.delay_enable = normalized(&params.delay_enable, true);
                data.delay_time_ms = normalized(&params.delay_time_ms, lerp(160.0, 420.0, flavor));
                data.delay_mix = normalized(&params.delay_mix, lerp(0.12, 0.35, motion));
            }
            GmGroup::Bass => {
                data.waveform = normalized(&params.waveform, Waveform::Sawtooth);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.1, 0.4, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.2, 0.65, tone));
                data.sub_level = normalized(&params.sub_level, lerp(0.35, 0.8, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.4, 2.0, flavor));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(4.0, 12.0, tone));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.55, 0.85, flavor));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(2.0, 6.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(120.0, 520.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.5, flavor));
                data.reverb_enable = normalized(&params.reverb_enable, false);
            }
            GmGroup::Strings => {
                data.waveform = normalized(&params.waveform, Waveform::Triangle);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.7, 1.0, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.35, 0.9, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(8.0, 18.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(12.0, 28.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.65, 0.9, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(10.0, 22.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(500.0, 2400.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.45, flavor));
                data.chorus_enable = normalized(&params.chorus_enable, true);
                data.chorus_mix = normalized(&params.chorus_mix, lerp(0.35, 0.6, motion));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.3, 0.6, tone));
            }
            GmGroup::Ensemble => {
                data.waveform = normalized(&params.waveform, Waveform::Sawtooth);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.45, 0.85, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.25, 0.85, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(6.0, 14.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(10.0, 24.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.6, 0.9, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(9.0, 18.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(650.0, 2800.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.5, flavor));
                data.chorus_enable = normalized(&params.chorus_enable, true);
                data.chorus_mix = normalized(&params.chorus_mix, lerp(0.3, 0.55, tone));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.35, 0.65, motion));
            }
            GmGroup::Brass => {
                data.waveform = normalized(&params.waveform, Waveform::Square);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.3, 0.6, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.25, 0.75, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(2.0, 6.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(8.0, 18.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.5, 0.85, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(5.0, 12.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(500.0, 2200.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.3, 0.65, flavor));
                data.delay_enable = normalized(&params.delay_enable, true);
                data.delay_time_ms = normalized(&params.delay_time_ms, lerp(90.0, 280.0, motion));
                data.delay_mix = normalized(&params.delay_mix, lerp(0.1, 0.25, flavor));
            }
            GmGroup::Reed => {
                data.waveform = normalized(&params.waveform, Waveform::Pulse);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.2, 0.55, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.2, 0.65, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(1.5, 4.5, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(6.0, 16.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.55, 0.85, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(4.0, 10.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(800.0, 2800.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.3, 0.7, flavor));
                data.vibrato_intensity = normalized(&params.vibrato_intensity, lerp(0.1, 0.35, motion));
                data.vibrato_rate = normalized(&params.vibrato_rate, lerp(4.0, 7.5, tone));
            }
            GmGroup::Pipe => {
                data.waveform = normalized(&params.waveform, Waveform::Sine);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.2, 0.45, tone));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.1, 0.4, flavor));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(3.0, 7.5, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(8.0, 18.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.65, 0.9, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(6.0, 14.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Highpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(200.0, 1400.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.5, flavor));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.35, 0.6, tone));
            }
            GmGroup::SynthLead => {
                data.waveform = normalized(&params.waveform, Waveform::Sawtooth);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.55, 1.0, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.4, 0.95, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.4, 1.4, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(4.0, 10.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.45, 0.8, flavor));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(2.5, 6.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Notch);
                data.filter_cut = normalized(&params.filter_cut, lerp(1200.0, 4200.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.25, 0.65, flavor));
                data.delay_enable = normalized(&params.delay_enable, true);
                data.delay_time_ms = normalized(&params.delay_time_ms, lerp(140.0, 520.0, tone));
                data.delay_feedback = normalized(&params.delay_feedback, lerp(0.2, 0.5, motion));
                data.delay_mix = normalized(&params.delay_mix, lerp(0.2, 0.45, flavor));
            }
            GmGroup::SynthPad => {
                data.waveform = normalized(&params.waveform, Waveform::Triangle);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.7, 1.0, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.2, 0.95, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(10.0, 22.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(12.0, 24.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.65, 0.9, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(12.0, 28.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Lowpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(400.0, 1800.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.2, 0.45, flavor));
                data.chorus_enable = normalized(&params.chorus_enable, true);
                data.chorus_mix = normalized(&params.chorus_mix, lerp(0.4, 0.7, tone));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.45, 0.75, motion));
            }
            GmGroup::SynthFx => {
                data.waveform = normalized(&params.waveform, Waveform::Noise);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.4, 0.8, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.1, 0.9, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(2.0, 8.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(6.0, 18.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.3, 0.7, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(4.0, 12.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Statevariable);
                data.filter_cut = normalized(&params.filter_cut, lerp(800.0, 5000.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.35, 0.85, flavor));
                data.lfo1_rate = normalized(&params.lfo1_rate, lerp(0.15, 1.4, motion));
                data.lfo1_attack = normalized(&params.lfo1_attack, lerp(0.5, 4.0, tone));
                data.mod1_source = normalized(&params.mod1_source, ModSource::Lfo1);
                data.mod1_target = normalized(&params.mod1_target, ModTarget::FilterCut);
                data.mod1_amount = normalized(&params.mod1_amount, lerp(0.2, 0.7, motion));
                data.delay_enable = normalized(&params.delay_enable, true);
                data.delay_time_ms = normalized(&params.delay_time_ms, lerp(240.0, 720.0, tone));
                data.delay_mix = normalized(&params.delay_mix, lerp(0.2, 0.5, flavor));
            }
            GmGroup::Ethnic => {
                data.waveform = normalized(&params.waveform, Waveform::Pulse);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::Blend);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.4, 0.7, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.25, 0.85, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(1.5, 5.5, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(6.0, 14.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.45, 0.8, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(4.0, 10.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(500.0, 2600.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.3, 0.6, flavor));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.25, 0.5, motion));
            }
            GmGroup::Percussive => {
                data.waveform = normalized(&params.waveform, Waveform::Noise);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::ClassicOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.2, 0.5, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.1, 0.6, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.1, 1.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(1.0, 6.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.05, 0.3, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(1.0, 4.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Bandpass);
                data.filter_cut = normalized(&params.filter_cut, lerp(900.0, 3200.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.4, 0.75, flavor));
                data.delay_enable = normalized(&params.delay_enable, true);
                data.delay_time_ms = normalized(&params.delay_time_ms, lerp(80.0, 240.0, flavor));
                data.delay_mix = normalized(&params.delay_mix, lerp(0.1, 0.25, motion));
            }
            GmGroup::Sfx => {
                data.waveform = normalized(&params.waveform, Waveform::Noise);
                data.osc_routing = normalized(&params.osc_routing, OscRouting::WavetableOnly);
                data.osc_blend = normalized(&params.osc_blend, lerp(0.5, 1.0, flavor));
                data.wavetable_position = normalized(&params.wavetable_position, lerp(0.0, 1.0, tone));
                data.amp_attack_ms = normalized(&params.amp_attack_ms, lerp(0.2, 4.0, tone));
                data.amp_decay_ms = normalized(&params.amp_decay_ms, lerp(2.0, 12.0, flavor));
                data.amp_sustain_level = normalized(&params.amp_sustain_level, lerp(0.15, 0.6, tone));
                data.amp_release_ms = normalized(&params.amp_release_ms, lerp(3.0, 10.0, tone));
                data.filter_type = normalized(&params.filter_type, FilterType::Statevariable);
                data.filter_cut = normalized(&params.filter_cut, lerp(1000.0, 7200.0, motion));
                data.filter_res = normalized(&params.filter_res, lerp(0.35, 0.9, flavor));
                data.lfo2_rate = normalized(&params.lfo2_rate, lerp(0.1, 1.0, motion));
                data.lfo2_attack = normalized(&params.lfo2_attack, lerp(0.5, 5.0, tone));
                data.mod2_source = normalized(&params.mod2_source, ModSource::Lfo2);
                data.mod2_target = normalized(&params.mod2_target, ModTarget::Pan);
                data.mod2_amount = normalized(&params.mod2_amount, lerp(0.25, 0.7, motion));
                data.reverb_enable = normalized(&params.reverb_enable, true);
                data.reverb_mix = normalized(&params.reverb_mix, lerp(0.35, 0.7, tone));
            }
        }

        data.cutoff_lfo_rate = normalized(&params.cutoff_lfo_rate, lerp(0.05, 0.8, motion));
        data.cutoff_lfo_intensity = normalized(&params.cutoff_lfo_intensity, lerp(0.0, 0.4, flavor));
        data.pan_lfo_intensity = normalized(&params.pan_lfo_intensity, lerp(0.0, 0.35, tone));
        data.pan_lfo_rate = normalized(&params.pan_lfo_rate, lerp(0.05, 0.5, motion));

        presets.push((name, data));
    }

    presets
}

fn apply_param<P: Param>(cx: &mut EventContext, param: &P, normalized: f32) {
    cx.emit(nih_plug_vizia::widgets::ParamEvent::BeginSetParameter(param).upcast());
    cx.emit(
        nih_plug_vizia::widgets::ParamEvent::SetParameterNormalized(param, normalized).upcast(),
    );
    cx.emit(nih_plug_vizia::widgets::ParamEvent::EndSetParameter(param).upcast());
}
