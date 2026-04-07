use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;
use std::f32::consts::PI;
use std::path::Path;

pub const WAVETABLE_SIZE: usize = 2048;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum Waveform {
    Sine,
    Triangle,
    Sawtooth,
    Square,
    Pulse,
    Noise,
}

#[derive(Debug, Clone)]
pub struct WavetableBank {
    tables: Vec<Vec<f32>>,
}

pub fn generate_waveform(waveform: Waveform, phase: f32) -> f32 {
    match waveform {
        Waveform::Sine => ((phase % 1.0) * 2.0 * std::f32::consts::PI).sin(),
        Waveform::Triangle => (2.0 * (phase - 0.5)).abs() * 2.0 - 1.0,
        Waveform::Sawtooth => 1.0 - phase * 2.0,
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Pulse => {
            if phase < 0.25 || phase >= 0.75 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Noise => rand::random::<f32>() * 2.0 - 1.0,
    }
}

impl WavetableBank {
    pub fn new() -> Self {
        let tables = vec![
            build_additive_table(WAVETABLE_SIZE, &[(1, 1.0), (2, 0.3), (3, 0.2)]),
            build_additive_table(WAVETABLE_SIZE, &[(1, 1.0), (3, 0.6), (5, 0.35)]),
            build_additive_table(WAVETABLE_SIZE, &[(1, 0.9), (4, 0.35), (7, 0.25), (10, 0.2)]),
            build_additive_table(WAVETABLE_SIZE, &[(1, 0.7), (2, 0.5), (9, 0.25), (13, 0.18)]),
            build_additive_table(
                WAVETABLE_SIZE,
                &[(1, 1.0), (2, 0.85), (3, 0.65), (5, 0.45), (7, 0.3), (9, 0.2)],
            ),
            build_additive_table(
                WAVETABLE_SIZE,
                &[(1, 1.0), (3, 0.8), (5, 0.6), (7, 0.4), (9, 0.25), (11, 0.2)],
            ),
            build_additive_table(
                WAVETABLE_SIZE,
                &[(1, 0.9), (2, 0.7), (4, 0.5), (6, 0.35), (8, 0.25), (12, 0.2)],
            ),
        ];

        Self { tables }
    }

    pub fn from_table(table: Vec<f32>) -> Self {
        let mut tables = Vec::new();
        if table.is_empty() {
            tables.push(vec![0.0; WAVETABLE_SIZE]);
        } else {
            tables.push(table);
        }
        Self { tables }
    }

    pub fn sample(&self, phase: f32, position: f32) -> f32 {
        let table_count = self.tables.len().max(1);
        let clamped_pos = position.clamp(0.0, 1.0);
        let table_pos = clamped_pos * (table_count as f32 - 1.0);
        let table_idx = table_pos.floor() as usize;
        let table_next = (table_idx + 1).min(table_count - 1);
        let table_blend = table_pos - table_idx as f32;

        let sample_a = sample_table(&self.tables[table_idx], phase);
        let sample_b = sample_table(&self.tables[table_next], phase);

        sample_a + (sample_b - sample_a) * table_blend
    }
}

pub fn load_wavetable_from_file(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;

    let mut mono_samples = Vec::new();
    let mut accum = 0.0f32;
    let mut channel_index = 0usize;

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let sample = sample.map_err(|e| e.to_string())?;
                accum += sample;
                channel_index += 1;
                if channel_index == channels {
                    mono_samples.push(accum / channels as f32);
                    accum = 0.0;
                    channel_index = 0;
                }
            }
        }
        hound::SampleFormat::Int => {
            let max = (1u64 << (spec.bits_per_sample - 1)) as f32;
            for sample in reader.samples::<i32>() {
                let sample = sample.map_err(|e| e.to_string())? as f32 / max;
                accum += sample;
                channel_index += 1;
                if channel_index == channels {
                    mono_samples.push(accum / channels as f32);
                    accum = 0.0;
                    channel_index = 0;
                }
            }
        }
    }

    if mono_samples.len() < 2 {
        return Err("WAV file is too short".to_string());
    }

    Ok(build_wavetable_from_samples(&mono_samples))
}

pub fn build_wavetable_from_samples(samples: &[f32]) -> Vec<f32> {
    let len = samples.len().max(2);
    let mut table = vec![0.0; WAVETABLE_SIZE];

    for i in 0..WAVETABLE_SIZE {
        let pos = i as f32 / WAVETABLE_SIZE as f32 * (len - 1) as f32;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f32;
        let next_idx = (idx + 1).min(len - 1);
        let a = samples[idx];
        let b = samples[next_idx];
        table[i] = a + (b - a) * frac;
    }

    normalize_table(&mut table);
    table
}

fn build_additive_table(size: usize, harmonics: &[(usize, f32)]) -> Vec<f32> {
    let mut table = vec![0.0; size];

    for i in 0..size {
        let phase = i as f32 / size as f32;
        let mut sample = 0.0;
        for (harmonic, amplitude) in harmonics {
            sample += amplitude * (2.0 * PI * (*harmonic as f32) * phase).sin();
        }
        table[i] = sample;
    }

    normalize_table(&mut table);
    table
}

fn normalize_table(table: &mut [f32]) {
    let mut peak: f32 = 0.0;
    for value in table.iter() {
        peak = peak.max(value.abs());
    }

    if peak > 0.0 {
        for value in table.iter_mut() {
            *value /= peak;
        }
    }
}

fn sample_table(table: &[f32], phase: f32) -> f32 {
    let pos = phase.fract() * table.len() as f32;
    let idx = pos.floor() as usize;
    let frac = pos - idx as f32;
    let next_idx = (idx + 1) % table.len();

    let a = table[idx];
    let b = table[next_idx];

    a + (b - a) * frac
}
