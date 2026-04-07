const COMB_TUNINGS_L: [usize; 4] = [1116, 1188, 1277, 1356];
const COMB_TUNINGS_R: [usize; 4] = [1139, 1211, 1300, 1379];
const ALLPASS_TUNINGS_L: [usize; 2] = [556, 441];
const ALLPASS_TUNINGS_R: [usize; 2] = [579, 464];

struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    filter_store: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            index: 0,
            filter_store: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32, shimmer: f32) -> f32 {
        let output = self.buffer[self.index];
        self.filter_store = output + (self.filter_store - output) * damp;
        let shimmer_component = output.abs() * shimmer;
        self.buffer[self.index] = input + self.filter_store * feedback + shimmer_component;
        self.index = (self.index + 1) % self.buffer.len().max(1);
        output
    }
}

struct AllpassFilter {
    buffer: Vec<f32>,
    index: usize,
}

impl AllpassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size.max(1)],
            index: 0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32) -> f32 {
        let buffer_sample = self.buffer[self.index];
        let output = -input + buffer_sample;
        self.buffer[self.index] = input + buffer_sample * feedback;
        self.index = (self.index + 1) % self.buffer.len().max(1);
        output
    }
}

pub struct Reverb {
    sample_rate: f32,
    combs_left: Vec<CombFilter>,
    combs_right: Vec<CombFilter>,
    allpass_left: Vec<AllpassFilter>,
    allpass_right: Vec<AllpassFilter>,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let mut reverb = Self {
            sample_rate,
            combs_left: Vec::new(),
            combs_right: Vec::new(),
            allpass_left: Vec::new(),
            allpass_right: Vec::new(),
        };
        reverb.rebuild();
        reverb
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() < f32::EPSILON {
            return;
        }
        self.sample_rate = sample_rate;
        self.rebuild();
    }

    pub fn process(
        &mut self,
        left: f32,
        right: f32,
        size: f32,
        damp: f32,
        diffusion: f32,
        shimmer: f32,
        mix: f32,
    ) -> (f32, f32) {
        let size = size.clamp(0.0, 1.0);
        let feedback = 0.2 + size * 0.75;
        let damp = damp.clamp(0.0, 1.0) * 0.5;
        let diffusion = diffusion.clamp(0.0, 1.0);
        let shimmer = shimmer.clamp(0.0, 1.0) * 0.35;
        let mix = mix.clamp(0.0, 1.0);

        let input = (left + right) * 0.5;

        let mut wet_left = 0.0;
        let mut wet_right = 0.0;

        for comb in self.combs_left.iter_mut() {
            wet_left += comb.process(input, feedback, damp, shimmer);
        }
        for comb in self.combs_right.iter_mut() {
            wet_right += comb.process(input, feedback, damp, shimmer);
        }

        let ap_feedback = 0.5 + diffusion * 0.4;
        for allpass in self.allpass_left.iter_mut() {
            wet_left = allpass.process(wet_left, ap_feedback);
        }
        for allpass in self.allpass_right.iter_mut() {
            wet_right = allpass.process(wet_right, ap_feedback);
        }

        wet_left *= 0.25;
        wet_right *= 0.25;

        let out_left = left * (1.0 - mix) + wet_left * mix;
        let out_right = right * (1.0 - mix) + wet_right * mix;

        (out_left, out_right)
    }

    fn rebuild(&mut self) {
        let scale = (self.sample_rate / 44100.0).max(0.1);
        self.combs_left = COMB_TUNINGS_L
            .iter()
            .map(|&tuning| CombFilter::new((tuning as f32 * scale) as usize))
            .collect();
        self.combs_right = COMB_TUNINGS_R
            .iter()
            .map(|&tuning| CombFilter::new((tuning as f32 * scale) as usize))
            .collect();
        self.allpass_left = ALLPASS_TUNINGS_L
            .iter()
            .map(|&tuning| AllpassFilter::new((tuning as f32 * scale) as usize))
            .collect();
        self.allpass_right = ALLPASS_TUNINGS_R
            .iter()
            .map(|&tuning| AllpassFilter::new((tuning as f32 * scale) as usize))
            .collect();
    }
}
