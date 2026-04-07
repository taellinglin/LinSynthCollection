use crate::filter::FilterType;
use crate::modulator::OscillatorShape;
use crate::util;
use crate::waveform::Waveform;
use crate::FishSynthParams;
use nih_plug::prelude::ParamMut;

pub struct PresetCategory {
    pub name: &'static str,
    pub start: usize,
}

pub const GM_CATEGORIES: [PresetCategory; 16] = [
    PresetCategory { name: "Piano", start: 0 },
    PresetCategory {
        name: "Chromatic Percussion",
        start: 8,
    },
    PresetCategory { name: "Organ", start: 16 },
    PresetCategory { name: "Guitar", start: 24 },
    PresetCategory { name: "Bass", start: 32 },
    PresetCategory { name: "Strings", start: 40 },
    PresetCategory { name: "Ensemble", start: 48 },
    PresetCategory { name: "Brass", start: 56 },
    PresetCategory { name: "Reed", start: 64 },
    PresetCategory { name: "Pipe", start: 72 },
    PresetCategory { name: "Synth Lead", start: 80 },
    PresetCategory { name: "Synth Pad", start: 88 },
    PresetCategory { name: "Synth Effects", start: 96 },
    PresetCategory { name: "Ethnic", start: 104 },
    PresetCategory { name: "Percussive", start: 112 },
    PresetCategory {
        name: "Sound Effects",
        start: 120,
    },
];

pub const GM_PRESETS: [&str; 128] = [
    "Acoustic Grand Piano",
    "Bright Acoustic Piano",
    "Electric Grand Piano",
    "Honky-tonk Piano",
    "Electric Piano 1",
    "Electric Piano 2",
    "Harpsichord",
    "Clavi",
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
    "Guitar harmonics",
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
    "SynthStrings 1",
    "SynthStrings 2",
    "Choir Aahs",
    "Voice Oohs",
    "Synth Voice",
    "Orchestral Hit",
    "Trumpet",
    "Trombone",
    "Tuba",
    "Muted Trumpet",
    "French Horn",
    "Brass Section",
    "SynthBrass 1",
    "SynthBrass 2",
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

#[derive(Clone, Copy)]
pub struct PresetValues {
    pub gain: f32,
    pub amp_attack_ms: f32,
    pub amp_decay_ms: f32,
    pub amp_sustain_level: f32,
    pub amp_release_ms: f32,
    pub waveform: Waveform,
    pub sub_waveform: Waveform,
    pub modulator_waveform: Waveform,
    pub fm_enable: bool,
    pub fm_ratio: f32,
    pub fm_amount: f32,
    pub sub_mix: f32,
    pub noise_mix: f32,
    pub filter_type: FilterType,
    pub filter_cut: f32,
    pub filter_res: f32,
    pub filter_amount: f32,
    pub filter_cut_attack_ms: f32,
    pub filter_cut_decay_ms: f32,
    pub filter_cut_sustain_ms: f32,
    pub filter_cut_release_ms: f32,
    pub filter_res_attack_ms: f32,
    pub filter_res_decay_ms: f32,
    pub filter_res_sustain_ms: f32,
    pub filter_res_release_ms: f32,
    pub filter_cut_envelope_level: f32,
    pub filter_res_envelope_level: f32,
    pub amp_envelope_level: f32,
    pub vibrato_rate: f32,
    pub vibrato_intensity: f32,
    pub vibrato_attack: f32,
    pub vibrato_shape: OscillatorShape,
    pub tremolo_rate: f32,
    pub tremolo_intensity: f32,
    pub tremolo_attack: f32,
    pub tremolo_shape: OscillatorShape,
    pub tension: f32,
    pub chorus_enable: bool,
    pub chorus_rate: f32,
    pub chorus_depth: f32,
    pub chorus_mix: f32,
}

fn dry_defaults() -> PresetValues {
    PresetValues {
        gain: util::db_to_gain(-12.0),
        amp_attack_ms: 1.0,
        amp_decay_ms: 40.0,
        amp_sustain_level: 0.7,
        amp_release_ms: 4.0,
        waveform: Waveform::Sine,
        sub_waveform: Waveform::Sine,
        modulator_waveform: Waveform::Sine,
        fm_enable: false,
        fm_ratio: 1.0,
        fm_amount: 0.0,
        sub_mix: 0.0,
        noise_mix: 0.0,
        filter_type: FilterType::None,
        filter_cut: 12000.0,
        filter_res: 0.1,
        filter_amount: 0.4,
        filter_cut_attack_ms: 1.0,
        filter_cut_decay_ms: 30.0,
        filter_cut_sustain_ms: 0.6,
        filter_cut_release_ms: 4.0,
        filter_res_attack_ms: 1.0,
        filter_res_decay_ms: 20.0,
        filter_res_sustain_ms: 0.2,
        filter_res_release_ms: 4.0,
        filter_cut_envelope_level: 0.4,
        filter_res_envelope_level: 0.2,
        amp_envelope_level: 1.0,
        vibrato_rate: 5.0,
        vibrato_intensity: 0.0,
        vibrato_attack: 1.0,
        vibrato_shape: OscillatorShape::Sine,
        tremolo_rate: 5.0,
        tremolo_intensity: 0.0,
        tremolo_attack: 1.0,
        tremolo_shape: OscillatorShape::Sine,
        tension: 0.0,
        chorus_enable: false,
        chorus_rate: 0.5,
        chorus_depth: 10.0,
        chorus_mix: 0.0,
    }
}

fn base_for_category(category: usize) -> PresetValues {
    let mut values = dry_defaults();
    match category {
        0 => {
            values.amp_attack_ms = 0.8;
            values.amp_decay_ms = 70.0;
            values.amp_sustain_level = 0.25;
            values.amp_release_ms = 6.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 9000.0;
            values.filter_amount = 0.7;
            values.filter_cut_envelope_level = 0.5;
            values.sub_mix = 0.4;
            values.sub_waveform = Waveform::Sine;
        }
        1 => {
            values.amp_attack_ms = 0.4;
            values.amp_decay_ms = 60.0;
            values.amp_sustain_level = 0.0;
            values.amp_release_ms = 4.0;
            values.fm_enable = true;
            values.fm_ratio = 3.0;
            values.fm_amount = 4.5;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 14000.0;
            values.filter_amount = 0.5;
            values.noise_mix = 0.05;
            values.tremolo_rate = 6.0;
            values.tremolo_intensity = 0.1;
        }
        2 => {
            values.amp_attack_ms = 0.6;
            values.amp_decay_ms = 30.0;
            values.amp_sustain_level = 1.0;
            values.amp_release_ms = 5.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 12000.0;
            values.filter_amount = 0.2;
        }
        3 => {
            values.amp_attack_ms = 0.4;
            values.noise_mix = 0.2;
            values.amp_decay_ms = 80.0;
            values.amp_sustain_level = 0.2;
            values.amp_release_ms = 5.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 7000.0;
            values.filter_amount = 0.7;
            values.filter_cut_envelope_level = 0.6;
        }
        4 => {
            values.amp_attack_ms = 0.4;
            values.amp_decay_ms = 50.0;
            values.amp_sustain_level = 0.6;
            values.amp_release_ms = 4.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 1800.0;
            values.filter_res = 0.2;
            values.filter_amount = 0.8;
        }
        5 => {
            values.amp_attack_ms = 4.0;
            values.noise_mix = 0.3;
            values.amp_decay_ms = 60.0;
            values.amp_sustain_level = 0.85;
            values.amp_release_ms = 7.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 6000.0;
            values.filter_amount = 0.6;
            values.vibrato_rate = 5.5;
            values.vibrato_intensity = 0.08;
        }
        6 => {
            values.amp_attack_ms = 5.0;
            values.amp_decay_ms = 70.0;
            values.amp_sustain_level = 0.8;
            values.amp_release_ms = 7.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 5500.0;
            values.filter_amount = 0.55;
            values.vibrato_rate = 5.2;
            values.vibrato_intensity = 0.1;
        }
        7 => {
            values.amp_attack_ms = 2.5;
            values.amp_decay_ms = 50.0;
            values.amp_sustain_level = 0.7;
            values.amp_release_ms = 4.5;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 5000.0;
            values.filter_res = 0.3;
            values.filter_amount = 0.7;
        }
        8 => {
            values.amp_attack_ms = 2.0;
            values.amp_decay_ms = 45.0;
            values.amp_sustain_level = 0.75;
            values.amp_release_ms = 4.5;
            values.filter_type = FilterType::Bandpass;
            values.filter_cut = 3500.0;
            values.filter_res = 0.4;
            values.filter_amount = 0.8;
        }
        9 => {
            values.amp_attack_ms = 4.0;
            values.amp_decay_ms = 30.0;
            values.amp_sustain_level = 1.0;
            values.amp_release_ms = 5.5;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 13000.0;
            values.filter_amount = 0.3;
        }
        10 => {
            values.amp_attack_ms = 0.7;
            values.amp_decay_ms = 35.0;
            values.amp_sustain_level = 0.85;
            values.amp_release_ms = 2.5;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 7000.0;
            values.filter_res = 0.25;
            values.filter_amount = 0.6;
            values.vibrato_rate = 6.5;
            values.vibrato_intensity = 0.12;
        }
        11 => {
            values.amp_attack_ms = 8.0;
            values.amp_decay_ms = 80.0;
            values.amp_sustain_level = 0.9;
            values.amp_release_ms = 8.5;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 4500.0;
            values.filter_amount = 0.6;
            values.vibrato_rate = 4.5;
            values.vibrato_intensity = 0.07;
        }
        12 => {
            values.amp_attack_ms = 1.0;
            values.amp_decay_ms = 40.0;
            values.amp_sustain_level = 0.4;
            values.amp_release_ms = 6.0;
            values.filter_type = FilterType::Bandpass;
            values.filter_cut = 4800.0;
            values.filter_res = 0.6;
            values.filter_amount = 0.8;
            values.fm_enable = true;
            values.fm_ratio = 4.0;
            values.fm_amount = 5.0;
            values.vibrato_rate = 6.0;
            values.vibrato_intensity = 0.2;
        }
        13 => {
            values.amp_attack_ms = 1.2;
            values.amp_decay_ms = 70.0;
            values.amp_sustain_level = 0.4;
            values.amp_release_ms = 6.0;
            values.filter_type = FilterType::Lowpass;
            values.filter_cut = 7000.0;
            values.filter_amount = 0.7;
        }
        14 => {
            values.amp_attack_ms = 0.3;
            values.amp_decay_ms = 35.0;
            values.amp_sustain_level = 0.0;
            values.amp_release_ms = 3.0;
            values.filter_type = FilterType::Bandpass;
            values.filter_cut = 4000.0;
            values.filter_res = 0.5;
            values.filter_amount = 0.9;
            values.fm_enable = true;
            values.fm_ratio = 2.5;
            values.fm_amount = 3.5;
        }
        _ => {
            values.amp_attack_ms = 0.8;
            values.amp_decay_ms = 50.0;
            values.amp_sustain_level = 0.3;
            values.amp_release_ms = 5.0;
            values.filter_type = FilterType::Highpass;
            values.filter_cut = 1200.0;
            values.filter_res = 0.6;
            values.filter_amount = 0.9;
            values.fm_enable = true;
            values.fm_ratio = 5.0;
            values.fm_amount = 6.0;
            values.vibrato_rate = 7.0;
            values.vibrato_intensity = 0.3;
            values.noise_mix = 0.35;
        }
    }

    values
}

fn apply_variant(values: &mut PresetValues, category: usize, variant: usize) {
    let t = variant as f32 / 7.0;

    match category {
        0 => {
            let waves = [
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Square,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Square,
                Waveform::Pulse,
            ];
            values.waveform = waves[variant];
            if variant >= 4 {
                values.fm_enable = true;
                values.fm_ratio = 2.0 + t * 2.0;
                values.fm_amount = 1.5 + t * 2.0;
            }
            values.filter_cut = (values.filter_cut * (0.8 + 0.4 * t)).min(20000.0);
        }
        1 => {
            let waves = [
                Waveform::Sine,
                Waveform::Sine,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Sine,
            ];
            values.waveform = waves[variant];
            values.fm_ratio = 2.5 + t * 6.0;
            values.fm_amount = 3.0 + t * 5.0;
            values.tremolo_intensity = 0.12 + 0.18 * t;
        }
        2 => {
            let waves = [
                Waveform::Square,
                Waveform::Square,
                Waveform::Sawtooth,
                Waveform::Square,
                Waveform::Pulse,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Triangle,
            ];
            values.waveform = waves[variant];
            values.filter_cut = (9000.0 + 4000.0 * t).min(20000.0);
        }
        3 => {
            let waves = [
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Square,
                Waveform::Sawtooth,
                Waveform::Square,
                Waveform::Sine,
            ];
            values.waveform = waves[variant];
            if variant >= 6 {
                values.fm_enable = true;
                values.fm_ratio = 4.0;
                values.fm_amount = 2.0 + t * 2.0;
            }
            values.filter_cut = (6000.0 + 2500.0 * t).min(20000.0);
        }
        4 => {
            let waves = [
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Square,
                Waveform::Triangle,
                Waveform::Square,
                Waveform::Square,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
            ];
            values.waveform = waves[variant];
            values.sub_mix = 0.3 + 0.2 * t;
            if variant >= 6 {
                values.fm_enable = true;
                values.fm_ratio = 1.5 + t * 3.0;
                values.fm_amount = 1.0 + t * 2.5;
            }
            values.filter_cut = (1200.0 + 1200.0 * t).min(20000.0);
        }
        5 => {
            let waves = [
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Pulse,
                Waveform::Triangle,
                Waveform::Triangle,
            ];
            values.waveform = waves[variant];
            values.filter_cut = (4500.0 + 2500.0 * t).min(20000.0);
            values.vibrato_intensity = 0.06 + 0.08 * t;
        }
        6 => {
            let waves = [
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
            ];
            values.waveform = waves[variant];
            if variant >= 4 {
                values.fm_enable = true;
                values.fm_ratio = 2.0 + t * 2.0;
                values.fm_amount = 0.8 + t * 1.2;
            }
        }
        7 => {
            let waves = [
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Square,
                Waveform::Square,
            ];
            values.waveform = waves[variant];
            values.filter_cut = (3800.0 + 2000.0 * t).min(20000.0);
            values.vibrato_intensity = 0.08 + 0.06 * t;
        }
        8 => {
            let waves = [
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Triangle,
            ];
            values.waveform = waves[variant];
            values.filter_cut = (3000.0 + 1500.0 * t).min(20000.0);
            values.vibrato_intensity = 0.1 + 0.08 * t;
        }
        9 => {
            let waves = [
                Waveform::Sine,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Sine,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Sine,
            ];
            values.waveform = waves[variant];
            values.filter_cut = (11000.0 + 3000.0 * t).min(20000.0);
            values.vibrato_intensity = 0.05 + 0.08 * t;
        }
        10 => {
            let waves = [
                Waveform::Square,
                Waveform::Sawtooth,
                Waveform::Square,
                Waveform::Pulse,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Square,
            ];
            values.waveform = waves[variant];
            if variant == 4 || variant == 7 {
                values.fm_enable = true;
                values.fm_ratio = 2.0;
                values.fm_amount = 2.0 + t * 2.0;
            }
            values.filter_cut = (6000.0 + 2500.0 * t).min(20000.0);
        }
        11 => {
            let waves = [
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Sawtooth,
            ];
            values.waveform = waves[variant];
            values.filter_cut = (3800.0 + 2000.0 * t).min(20000.0);
            values.amp_attack_ms = (6.0 + 3.0 * t).min(10.0);
        }
        12 => {
            let waves = [
                Waveform::Noise,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Square,
                Waveform::Sine,
                Waveform::Noise,
            ];
            values.waveform = waves[variant];
            values.fm_ratio = 3.0 + t * 6.0;
            values.fm_amount = 4.0 + t * 4.0;
            values.tremolo_intensity = 0.2 + 0.3 * t;
            values.noise_mix = (values.noise_mix + 0.15 * t).min(1.0);
        }
        13 => {
            let waves = [
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Sine,
                Waveform::Sawtooth,
                Waveform::Triangle,
                Waveform::Sawtooth,
            ];
            values.waveform = waves[variant];
            if variant == 4 {
                values.fm_enable = true;
                values.fm_ratio = 4.0;
                values.fm_amount = 2.5;
            }
            values.filter_cut = (6000.0 + 2500.0 * t).min(20000.0);
        }
        14 => {
            let waves = [
                Waveform::Triangle,
                Waveform::Triangle,
                Waveform::Square,
                Waveform::Noise,
                Waveform::Triangle,
                Waveform::Square,
                Waveform::Noise,
                Waveform::Triangle,
            ];
            values.waveform = waves[variant];
            values.fm_ratio = 2.0 + t * 3.0;
            values.fm_amount = 2.0 + t * 3.0;
            values.tremolo_intensity = 0.15 + 0.2 * t;
            values.noise_mix = (values.noise_mix + 0.1 * t).min(1.0);
        }
        _ => {
            let waves = [
                Waveform::Noise,
                Waveform::Noise,
                Waveform::Noise,
                Waveform::Sine,
                Waveform::Noise,
                Waveform::Noise,
                Waveform::Noise,
                Waveform::Square,
            ];
            values.waveform = waves[variant];
            values.fm_ratio = 4.0 + t * 6.0;
            values.fm_amount = 4.0 + t * 5.0;
            values.tremolo_intensity = 0.2 + 0.4 * t;
            values.noise_mix = (values.noise_mix + 0.2 * t).min(1.0);
        }
    }

    values.filter_res = values.filter_res.clamp(0.0, 0.99);
    values.filter_cut = values.filter_cut.clamp(20.0, 20000.0);
    values.filter_amount = values.filter_amount.clamp(0.0, 1.0);
    values.vibrato_intensity = values.vibrato_intensity.clamp(0.0, 1.0);
    values.tremolo_intensity = values.tremolo_intensity.clamp(-1.0, 1.0);
    values.sub_mix = values.sub_mix.clamp(0.0, 1.0);
    values.noise_mix = values.noise_mix.clamp(0.0, 1.0);
}

pub fn preset_values_for(index: usize) -> PresetValues {
    let preset_index = index.min(GM_PRESETS.len() - 1);
    let category = preset_index / 8;
    let variant = preset_index % 8;

    let mut values = base_for_category(category);
    apply_variant(&mut values, category, variant);

    values
}

pub(crate) fn apply_preset_to_params(params: &FishSynthParams, values: PresetValues) {
    params.gain.set_plain_value(values.gain);
    params.amp_attack_ms.set_plain_value(values.amp_attack_ms);
    params.amp_decay_ms.set_plain_value(values.amp_decay_ms);
    params.amp_sustain_level.set_plain_value(values.amp_sustain_level);
    params.amp_release_ms.set_plain_value(values.amp_release_ms);
    params.waveform.set_plain_value(values.waveform);
    params.sub_waveform.set_plain_value(values.sub_waveform);
    params.modulator_waveform.set_plain_value(values.modulator_waveform);
    params.fm_enable.set_plain_value(values.fm_enable);
    params.fm_ratio.set_plain_value(values.fm_ratio);
    params.fm_amount.set_plain_value(values.fm_amount);
    params.sub_mix.set_plain_value(values.sub_mix);
    params.noise_mix.set_plain_value(values.noise_mix);
    params.filter_type.set_plain_value(values.filter_type);
    params.filter_cut.set_plain_value(values.filter_cut);
    params.filter_res.set_plain_value(values.filter_res);
    params.filter_amount.set_plain_value(values.filter_amount);
    params.filter_cut_attack_ms
        .set_plain_value(values.filter_cut_attack_ms);
    params
        .filter_cut_decay_ms
        .set_plain_value(values.filter_cut_decay_ms);
    params
        .filter_cut_sustain_ms
        .set_plain_value(values.filter_cut_sustain_ms);
    params
        .filter_cut_release_ms
        .set_plain_value(values.filter_cut_release_ms);
    params
        .filter_res_attack_ms
        .set_plain_value(values.filter_res_attack_ms);
    params
        .filter_res_decay_ms
        .set_plain_value(values.filter_res_decay_ms);
    params
        .filter_res_sustain_ms
        .set_plain_value(values.filter_res_sustain_ms);
    params
        .filter_res_release_ms
        .set_plain_value(values.filter_res_release_ms);
    params
        .filter_cut_envelope_level
        .set_plain_value(values.filter_cut_envelope_level);
    params
        .filter_res_envelope_level
        .set_plain_value(values.filter_res_envelope_level);
    params
        .amp_envelope_level
        .set_plain_value(values.amp_envelope_level);
    params.vibrato_rate.set_plain_value(values.vibrato_rate);
    params
        .vibrato_intensity
        .set_plain_value(values.vibrato_intensity);
    params.vibrato_attack.set_plain_value(values.vibrato_attack);
    params
        .vibrato_shape
        .set_plain_value(values.vibrato_shape);
    params.tremolo_rate.set_plain_value(values.tremolo_rate);
    params
        .tremolo_intensity
        .set_plain_value(values.tremolo_intensity);
    params.tremolo_attack.set_plain_value(values.tremolo_attack);
    params
        .tremolo_shape
        .set_plain_value(values.tremolo_shape);
    params.tension.set_plain_value(values.tension);
    params.chorus_enable.set_plain_value(values.chorus_enable);
    params.chorus_rate.set_plain_value(values.chorus_rate);
    params.chorus_depth.set_plain_value(values.chorus_depth);
    params.chorus_mix.set_plain_value(values.chorus_mix);
}
