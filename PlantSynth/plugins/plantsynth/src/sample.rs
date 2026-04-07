use std::path::Path;

#[derive(Clone, Debug)]
pub struct SampleBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: f32,
}

pub fn load_sample_from_file(path: &Path) -> Result<SampleBuffer, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    if channels == 0 {
        return Err("WAV has no channels".to_string());
    }
    let sample_rate = spec.sample_rate as f32;
    let mut mono_samples = Vec::new();
    let mut accum = 0.0_f32;
    let mut count = 0usize;

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let sample = sample.map_err(|e| e.to_string())?;
                accum += sample;
                count += 1;
                if count == channels {
                    mono_samples.push(accum / channels as f32);
                    accum = 0.0;
                    count = 0;
                }
            }
        }
        hound::SampleFormat::Int => {
            let max = (1u64 << (spec.bits_per_sample - 1)) as f32;
            for sample in reader.samples::<i32>() {
                let sample = sample.map_err(|e| e.to_string())?;
                accum += (sample as f32) / max;
                count += 1;
                if count == channels {
                    mono_samples.push(accum / channels as f32);
                    accum = 0.0;
                    count = 0;
                }
            }
        }
    }

    Ok(SampleBuffer {
        samples: mono_samples,
        sample_rate,
    })
}
