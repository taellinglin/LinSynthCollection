use enum_iterator::Sequence;
use nih_plug::params::enums::EnumParam;
use nih_plug::prelude::*;
use std::sync::Arc;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Sequence)]
enum Mode {
    Nes,
    Gb,
    Snes,
    Md,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Sequence)]
enum PsgFilterType {
    Lowpass,
    Bandpass,
    Highpass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Sequence)]
enum DpcmMode {
    Dpcm,
    OneBit,
}

#[derive(Params)]
struct CartStackParams {
    #[id = "mode"]
    mode: EnumParam<Mode>,
    #[id = "mono"]
    mono: BoolParam,
    #[id = "mix"]
    mix: FloatParam,
    #[id = "cpu"]
    cpu: FloatParam,
    #[id = "input_trim"]
    input_trim: FloatParam,
    #[id = "output_trim"]
    output_trim: FloatParam,

    // Bit engine
    #[id = "bit_depth"]
    bit_depth: IntParam,
    #[id = "bit_rate"]
    bit_rate: FloatParam,
    #[id = "bit_jitter"]
    bit_jitter: FloatParam,
    #[id = "bit_pregain"]
    bit_pregain: FloatParam,

    // PWM shaper
    #[id = "pwm_duty"]
    pwm_duty: FloatParam,
    #[id = "pwm_depth"]
    pwm_depth: FloatParam,
    #[id = "pwm_rate"]
    pwm_rate: FloatParam,
    #[id = "pwm_audio_rate"]
    pwm_audio_rate: BoolParam,

    // PSG filter
    #[id = "psg_type"]
    psg_type: EnumParam<PsgFilterType>,
    #[id = "psg_cutoff"]
    psg_cutoff: FloatParam,
    #[id = "psg_res"]
    psg_res: FloatParam,
    #[id = "psg_steps"]
    psg_steps: IntParam,

    // Tracker gate
    #[id = "gate_steps"]
    gate_steps: IntParam,
    #[id = "gate_rate"]
    gate_rate: FloatParam,
    #[id = "gate_swing"]
    gate_swing: FloatParam,
    #[id = "gate_depth"]
    gate_depth: FloatParam,
    #[id = "gate_retrig"]
    gate_retrig: BoolParam,
    #[id = "gate_pattern"]
    gate_pattern: IntParam,

    // DPCM / 1-bit
    #[id = "dpcm_mode"]
    dpcm_mode: EnumParam<DpcmMode>,
    #[id = "dpcm_step"]
    dpcm_step: FloatParam,
    #[id = "dpcm_slew"]
    dpcm_slew: FloatParam,

    // Harmonizer
    #[id = "harm_int_a"]
    harm_interval_a: IntParam,
    #[id = "harm_int_b"]
    harm_interval_b: IntParam,
    #[id = "harm_detune"]
    harm_detune: FloatParam,
    #[id = "harm_mix"]
    harm_mix: FloatParam,
}

impl Default for CartStackParams {
    fn default() -> Self {
        Self {
            mode: EnumParam::new("Mode", Mode::Nes),
            mono: BoolParam::new("Mono", false),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            cpu: FloatParam::new("CPU", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            input_trim: FloatParam::new(
                "Input Trim",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit("dB"),
            output_trim: FloatParam::new(
                "Output Trim",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit("dB"),

            bit_depth: IntParam::new("Bit Depth", 6, IntRange::Linear { min: 1, max: 16 }),
            bit_rate: FloatParam::new(
                "Sample Rate",
                11025.0,
                FloatRange::Skewed { min: 1000.0, max: 48000.0, factor: 0.5 },
            )
            .with_unit("Hz"),
            bit_jitter: FloatParam::new("Jitter", 0.02, FloatRange::Linear { min: 0.0, max: 0.1 }),
            bit_pregain: FloatParam::new("Pre Gain", 0.0, FloatRange::Linear { min: -12.0, max: 12.0 })
                .with_unit("dB"),

            pwm_duty: FloatParam::new("PWM Duty", 0.5, FloatRange::Linear { min: 0.05, max: 0.95 }),
            pwm_depth: FloatParam::new("PWM Depth", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 }),
            pwm_rate: FloatParam::new(
                "PWM Rate",
                2.0,
                FloatRange::Skewed { min: 0.1, max: 20.0, factor: 0.5 },
            )
            .with_unit("Hz"),
            pwm_audio_rate: BoolParam::new("PWM Audio Rate", false),

            psg_type: EnumParam::new("PSG Type", PsgFilterType::Lowpass),
            psg_cutoff: FloatParam::new(
                "PSG Cutoff",
                2000.0,
                FloatRange::Skewed { min: 80.0, max: 12000.0, factor: 0.5 },
            )
            .with_unit("Hz"),
            psg_res: FloatParam::new("PSG Res", 0.2, FloatRange::Linear { min: 0.0, max: 0.9 }),
            psg_steps: IntParam::new("PSG Steps", 16, IntRange::Linear { min: 1, max: 128 }),

            gate_steps: IntParam::new("Gate Steps", 16, IntRange::Linear { min: 4, max: 32 }),
            gate_rate: FloatParam::new("Gate Rate", 8.0, FloatRange::Linear { min: 2.0, max: 64.0 }),
            gate_swing: FloatParam::new("Gate Swing", 0.0, FloatRange::Linear { min: 0.0, max: 0.6 }),
            gate_depth: FloatParam::new("Gate Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            gate_retrig: BoolParam::new("Gate Retrig", false),
            gate_pattern: IntParam::new("Gate Pattern", 0xAAAA, IntRange::Linear { min: 0, max: 65535 }),

            dpcm_mode: EnumParam::new("DPCM Mode", DpcmMode::Dpcm),
            dpcm_step: FloatParam::new("DPCM Step", 4.0, FloatRange::Linear { min: 1.0, max: 16.0 }),
            dpcm_slew: FloatParam::new("DPCM Slew", 0.0, FloatRange::Linear { min: 0.0, max: 30.0 })
                .with_unit("ms"),

            harm_interval_a: IntParam::new("Harm A", 7, IntRange::Linear { min: -12, max: 12 }),
            harm_interval_b: IntParam::new("Harm B", 12, IntRange::Linear { min: -12, max: 12 }),
            harm_detune: FloatParam::new("Harm Detune", 0.0, FloatRange::Linear { min: 0.0, max: 20.0 })
                .with_unit("cents"),
            harm_mix: FloatParam::new("Harm Mix", 0.2, FloatRange::Linear { min: 0.0, max: 0.5 }),
        }
    }
}

struct BitEngineState {
    held_l: f32,
    held_r: f32,
    counter: f32,
    rng: u32,
}

impl BitEngineState {
    fn new() -> Self {
        Self {
            held_l: 0.0,
            held_r: 0.0,
            counter: 0.0,
            rng: 0x12345678,
        }
    }

    fn next_noise(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn process(
        &mut self,
        input_l: f32,
        input_r: f32,
        sample_rate: f32,
        target_rate: f32,
        jitter: f32,
    ) -> (f32, f32) {
        let target_rate = target_rate.clamp(1000.0, 48000.0);
        let base_hold = (sample_rate / target_rate).max(1.0);
        let jitter_scale = 1.0 + self.next_noise() * jitter.clamp(0.0, 0.1);
        let hold = (base_hold * jitter_scale).max(1.0);

        self.counter += 1.0;
        if self.counter >= hold {
            self.counter -= hold;
            self.held_l = input_l;
            self.held_r = input_r;
        }

        (self.held_l, self.held_r)
    }
}

struct CartStack {
    params: Arc<CartStackParams>,
    sample_rate: f32,
    bit_state: BitEngineState,
}

impl Default for CartStack {
    fn default() -> Self {
        Self {
            params: Arc::new(CartStackParams::default()),
            sample_rate: 44100.0,
            bit_state: BitEngineState::new(),
        }
    }
}

impl Plugin for CartStack {
    const NAME: &'static str = "CartStack";
    const VENDOR: &'static str = "LingYue Synth";
    const URL: &'static str = "https://taellinglin.art";
    const EMAIL: &'static str = "taellinglin@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        self.bit_state = BitEngineState::new();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mix = self.params.mix.value().clamp(0.0, 1.0);
        let input_trim_db = self.params.input_trim.value();
        let output_trim_db = self.params.output_trim.value();
        let input_gain = db_to_gain(input_trim_db);
        let output_gain = db_to_gain(output_trim_db);

        let bit_depth = self.params.bit_depth.value() as u32;
        let bit_rate = self.params.bit_rate.value();
        let bit_jitter = self.params.bit_jitter.value();
        let bit_pregain = db_to_gain(self.params.bit_pregain.value());
        let mono = self.params.mono.value();

        let num_samples = buffer.samples();
        let output = buffer.as_slice();
        if output.is_empty() {
            return ProcessStatus::Normal;
        }

        if output.len() >= 2 {
            let (left, rest) = output.split_at_mut(1);
            let left = &mut left[0];
            let right = &mut rest[0];

            for i in 0..num_samples {
                let mut in_l = left[i] * input_gain;
                let mut in_r = right[i] * input_gain;
                if mono {
                    let mid = 0.5 * (in_l + in_r);
                    in_l = mid;
                    in_r = mid;
                }

                let (held_l, held_r) = self.bit_state.process(
                    in_l * bit_pregain,
                    in_r * bit_pregain,
                    self.sample_rate,
                    bit_rate,
                    bit_jitter,
                );

                let crushed_l = quantize(held_l, bit_depth);
                let crushed_r = quantize(held_r, bit_depth);

                let out_l = lerp(in_l, crushed_l, mix) * output_gain;
                let out_r = lerp(in_r, crushed_r, mix) * output_gain;

                left[i] = out_l;
                right[i] = out_r;
            }
        } else {
            let left = &mut output[0];
            for i in 0..num_samples {
                let in_l = left[i] * input_gain;

                let (held_l, _) = self.bit_state.process(
                    in_l * bit_pregain,
                    in_l * bit_pregain,
                    self.sample_rate,
                    bit_rate,
                    bit_jitter,
                );

                let crushed_l = quantize(held_l, bit_depth);
                let out_l = lerp(in_l, crushed_l, mix) * output_gain;
                left[i] = out_l;
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for CartStack {
    const VST3_CLASS_ID: [u8; 16] = *b"CartStackVST3!!!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Stereo,
    ];
}

impl ClapPlugin for CartStack {
    const CLAP_ID: &'static str = "art.taellinglin.cartstack";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("8-bit console-inspired multi-effect");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
    ];
}

nih_export_clap!(CartStack);
nih_export_vst3!(CartStack);

fn db_to_gain(db: f32) -> f32 {
    (10.0_f32).powf(db / 20.0)
}

fn quantize(sample: f32, bit_depth: u32) -> f32 {
    let bits = bit_depth.clamp(1, 16) as f32;
    let steps = (2.0_f32).powf(bits) - 1.0;
    let normalized = (sample * 0.5 + 0.5).clamp(0.0, 1.0);
    let quantized = (normalized * steps).round() / steps;
    (quantized * 2.0 - 1.0).clamp(-1.0, 1.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
