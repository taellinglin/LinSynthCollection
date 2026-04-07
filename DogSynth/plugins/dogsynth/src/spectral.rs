use nih_plug::util::stft::{StftInput, StftInputMut};
use nih_plug::util::{window, StftHelper};
use num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub struct SpectralShaper {
    stft: StftHelper,
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    scratch: Vec<Complex32>,
    block_size: usize,
    overlap: usize,
    sample_rate: f32,
}

struct StereoSlices<'a> {
    channels: [&'a mut [f32]; 2],
}

impl<'a> StereoSlices<'a> {
    fn new(left: &'a mut [f32], right: &'a mut [f32]) -> Self {
        Self { channels: [left, right] }
    }
}

impl StftInput for StereoSlices<'_> {
    fn num_samples(&self) -> usize {
        self.channels[0].len()
    }

    fn num_channels(&self) -> usize {
        2
    }

    unsafe fn get_sample_unchecked(&self, channel: usize, sample_idx: usize) -> f32 {
        *self.channels.get_unchecked(channel).get_unchecked(sample_idx)
    }
}

impl StftInputMut for StereoSlices<'_> {
    unsafe fn get_sample_unchecked_mut(&mut self, channel: usize, sample_idx: usize) -> &mut f32 {
        self.channels
            .get_unchecked_mut(channel)
            .get_unchecked_mut(sample_idx)
    }
}

impl SpectralShaper {
    pub fn new(sample_rate: f32, block_size: usize, overlap: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(block_size);
        let ifft = planner.plan_fft_inverse(block_size);
        let window = window::hann(block_size);
        let scratch = vec![Complex32::new(0.0, 0.0); block_size];

        Self {
            stft: StftHelper::new(2, block_size, 0),
            fft,
            ifft,
            window,
            scratch,
            block_size,
            overlap: overlap.max(1),
            sample_rate: sample_rate.max(1.0),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn latency_samples(&self) -> u32 {
        self.stft.latency_samples()
    }

    pub fn process_block(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        amount: f32,
        tilt: f32,
        formant: f32,
    ) {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return;
        }

        let tilt = tilt.clamp(-1.0, 1.0);
        let formant = formant.clamp(0.0, 1.0);
        let mut channels = StereoSlices::new(left, right);
        let nyquist = self.sample_rate * 0.5;
        let block_size = self.block_size;
        let window = &self.window;

        self.stft
            .process_overlap_add(&mut channels, self.overlap, |_, real| {
                if real.len() < block_size {
                    return;
                }
                let real = &mut real[..block_size];
                window::multiply_with_window(real, window);

                for (idx, sample) in real.iter().enumerate() {
                    self.scratch[idx] = Complex32::new(*sample, 0.0);
                }

                self.fft.process(&mut self.scratch);

                for (bin, value) in self.scratch.iter_mut().enumerate() {
                    let freq = (bin as f32 / block_size as f32) * nyquist;
                    let mut mag = value.norm();
                    let phase = value.arg();

                    let tilt_gain = if freq <= 1.0 {
                        1.0
                    } else {
                        let norm = (freq / nyquist).clamp(0.0, 1.0);
                        (1.0 + tilt * 1.6 * (norm - 0.4)).clamp(0.35, 2.5)
                    };

                    let vowel_peak = (formant * 2.0 - 1.0).abs();
                    let formant_gain = 1.0 + formant * (0.8 - 0.6 * vowel_peak);

                    let drive = 1.0 + amount * 2.2;
                    mag = (mag * drive).powf(1.0 - amount * 0.45) * tilt_gain * formant_gain;

                    *value = Complex32::from_polar(mag, phase);
                }

                self.ifft.process(&mut self.scratch);

                let norm = 1.0 / block_size as f32;
                for (idx, sample) in real.iter_mut().enumerate() {
                    *sample = self.scratch[idx].re * norm;
                }

                window::multiply_with_window(real, window);
            });
    }
}
