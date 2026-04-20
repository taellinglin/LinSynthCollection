mod editor;
mod presets;

use atomic_float::AtomicF32;
use enum_iterator::Sequence;
use nih_plug::params::enums::{Enum, EnumParam};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use presets::{apply_preset, PRESETS};

const NUM_BANKS: usize = 6;
const MAX_DELAY_SECONDS: f32 = 2.5;
const SCOPE_SIZE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Sequence)]
pub(crate) enum DelayNote {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    EighthTriplet,
    SixteenthTriplet,
    DottedQuarter,
    DottedEighth,
    DottedSixteenth,
}

impl DelayNote {
    fn beats(self) -> f32 {
        match self {
            DelayNote::Whole => 4.0,
            DelayNote::Half => 2.0,
            DelayNote::Quarter => 1.0,
            DelayNote::Eighth => 0.5,
            DelayNote::Sixteenth => 0.25,
            DelayNote::ThirtySecond => 0.125,
            DelayNote::EighthTriplet => 1.0 / 3.0,
            DelayNote::SixteenthTriplet => 1.0 / 6.0,
            DelayNote::DottedQuarter => 1.5,
            DelayNote::DottedEighth => 0.75,
            DelayNote::DottedSixteenth => 0.375,
        }
    }
}

pub(crate) struct ScopeBuffer {
    data: Vec<AtomicF32>,
    index: AtomicUsize,
}

impl ScopeBuffer {
    fn new(size: usize) -> Self {
        Self {
            data: (0..size).map(|_| AtomicF32::new(0.0)).collect(),
            index: AtomicUsize::new(0),
        }
    }

    fn push(&self, sample: f32) {
        let len = self.data.len();
        if len == 0 {
            return;
        }

        let idx = self.index.fetch_add(1, Ordering::Relaxed) % len;
        self.data[idx].store(sample, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self, out: &mut Vec<f32>) {
        let len = self.data.len();
        if len == 0 {
            out.clear();
            return;
        }

        out.clear();
        out.reserve(len);
        let start = self.index.load(Ordering::Relaxed) % len;
        for i in 0..len {
            let idx = (start + i) % len;
            out.push(self.data[idx].load(Ordering::Relaxed));
        }
    }
}

#[derive(Params)]
pub struct DelayBankParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    scope: Arc<ScopeBuffer>,

    #[id = "preset"]
    preset: IntParam,
    #[id = "mix"]
    mix: FloatParam,
    #[id = "input_trim"]
    input_trim: FloatParam,
    #[id = "output_trim"]
    output_trim: FloatParam,

    #[id = "hp_cut"]
    hp_cut: FloatParam,
    #[id = "lp_cut"]
    lp_cut: FloatParam,

    #[id = "crush_depth"]
    crush_depth: IntParam,
    #[id = "crush_rate"]
    crush_rate: FloatParam,
    #[id = "crush_mix"]
    crush_mix: FloatParam,

    #[id = "b1_on"]
    bank1_enable: BoolParam,
    #[id = "b1_sync"]
    bank1_sync: BoolParam,
    #[id = "b1_note"]
    bank1_time_note: EnumParam<DelayNote>,
    #[id = "b1_time"]
    bank1_time_ms: FloatParam,
    #[id = "b1_fb"]
    bank1_feedback: FloatParam,
    #[id = "b1_lvl"]
    bank1_level: FloatParam,
    #[id = "b1_pan"]
    bank1_pan: FloatParam,

    #[id = "b2_on"]
    bank2_enable: BoolParam,
    #[id = "b2_sync"]
    bank2_sync: BoolParam,
    #[id = "b2_note"]
    bank2_time_note: EnumParam<DelayNote>,
    #[id = "b2_time"]
    bank2_time_ms: FloatParam,
    #[id = "b2_fb"]
    bank2_feedback: FloatParam,
    #[id = "b2_lvl"]
    bank2_level: FloatParam,
    #[id = "b2_pan"]
    bank2_pan: FloatParam,

    #[id = "b3_on"]
    bank3_enable: BoolParam,
    #[id = "b3_sync"]
    bank3_sync: BoolParam,
    #[id = "b3_note"]
    bank3_time_note: EnumParam<DelayNote>,
    #[id = "b3_time"]
    bank3_time_ms: FloatParam,
    #[id = "b3_fb"]
    bank3_feedback: FloatParam,
    #[id = "b3_lvl"]
    bank3_level: FloatParam,
    #[id = "b3_pan"]
    bank3_pan: FloatParam,

    #[id = "b4_on"]
    bank4_enable: BoolParam,
    #[id = "b4_sync"]
    bank4_sync: BoolParam,
    #[id = "b4_note"]
    bank4_time_note: EnumParam<DelayNote>,
    #[id = "b4_time"]
    bank4_time_ms: FloatParam,
    #[id = "b4_fb"]
    bank4_feedback: FloatParam,
    #[id = "b4_lvl"]
    bank4_level: FloatParam,
    #[id = "b4_pan"]
    bank4_pan: FloatParam,

    #[id = "b5_on"]
    bank5_enable: BoolParam,
    #[id = "b5_sync"]
    bank5_sync: BoolParam,
    #[id = "b5_note"]
    bank5_time_note: EnumParam<DelayNote>,
    #[id = "b5_time"]
    bank5_time_ms: FloatParam,
    #[id = "b5_fb"]
    bank5_feedback: FloatParam,
    #[id = "b5_lvl"]
    bank5_level: FloatParam,
    #[id = "b5_pan"]
    bank5_pan: FloatParam,

    #[id = "b6_on"]
    bank6_enable: BoolParam,
    #[id = "b6_sync"]
    bank6_sync: BoolParam,
    #[id = "b6_note"]
    bank6_time_note: EnumParam<DelayNote>,
    #[id = "b6_time"]
    bank6_time_ms: FloatParam,
    #[id = "b6_fb"]
    bank6_feedback: FloatParam,
    #[id = "b6_lvl"]
    bank6_level: FloatParam,
    #[id = "b6_pan"]
    bank6_pan: FloatParam,
}

pub(crate) struct BankBlockParams {
    pub(crate) enabled: bool,
    pub(crate) sync: bool,
    pub(crate) note: DelayNote,
    pub(crate) time_ms: f32,
    pub(crate) feedback: f32,
    pub(crate) level: f32,
    pub(crate) pan: f32,
}

pub(crate) struct BlockParams {
    pub(crate) mix: f32,
    pub(crate) input_gain: f32,
    pub(crate) output_gain: f32,
    pub(crate) hp_cut: f32,
    pub(crate) lp_cut: f32,
    pub(crate) crush_bits: u32,
    pub(crate) crush_rate: f32,
    pub(crate) crush_mix: f32,
    pub(crate) banks: [BankBlockParams; NUM_BANKS],
}

impl Default for DelayBankParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            scope: Arc::new(ScopeBuffer::new(SCOPE_SIZE)),
            preset: IntParam::new(
                "Preset",
                0,
                IntRange::Linear {
                    min: 0,
                    max: (PRESETS.len() - 1) as i32,
                },
            ),
            mix: FloatParam::new("Mix", 0.65, FloatRange::Linear { min: 0.0, max: 1.0 }),
            input_trim: FloatParam::new(
                "Input",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit("dB"),
            output_trim: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit("dB"),
            hp_cut: FloatParam::new(
                "HP Cut",
                80.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 8000.0,
                    factor: 0.5,
                },
            )
            .with_unit("Hz"),
            lp_cut: FloatParam::new(
                "LP Cut",
                12000.0,
                FloatRange::Skewed {
                    min: 200.0,
                    max: 20000.0,
                    factor: 0.5,
                },
            )
            .with_unit("Hz"),
            crush_depth: IntParam::new("Crush Bits", 8, IntRange::Linear { min: 1, max: 16 }),
            crush_rate: FloatParam::new(
                "Crush Rate",
                12000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 48000.0,
                    factor: 0.5,
                },
            )
            .with_unit("Hz"),
            crush_mix: FloatParam::new("Crush Mix", 0.2, FloatRange::Linear { min: 0.0, max: 1.0 }),

            bank1_enable: BoolParam::new("B1 On", true),
            bank1_sync: BoolParam::new("B1 Sync", false),
            bank1_time_note: EnumParam::new("B1 Note", DelayNote::Quarter),
            bank1_time_ms: FloatParam::new(
                "B1 Time",
                220.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1500.0,
                    factor: 0.5,
                },
            )
            .with_unit("ms"),
            bank1_feedback: FloatParam::new("B1 Feedback", 0.35, FloatRange::Linear { min: 0.0, max: 0.95 }),
            bank1_level: FloatParam::new("B1 Level", 0.6, FloatRange::Linear { min: 0.0, max: 1.0 }),
            bank1_pan: FloatParam::new("B1 Pan", -0.2, FloatRange::Linear { min: -1.0, max: 1.0 }),

            bank2_enable: BoolParam::new("B2 On", true),
            bank2_sync: BoolParam::new("B2 Sync", false),
            bank2_time_note: EnumParam::new("B2 Note", DelayNote::Eighth),
            bank2_time_ms: FloatParam::new(
                "B2 Time",
                360.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1500.0,
                    factor: 0.5,
                },
            )
            .with_unit("ms"),
            bank2_feedback: FloatParam::new("B2 Feedback", 0.4, FloatRange::Linear { min: 0.0, max: 0.95 }),
            bank2_level: FloatParam::new("B2 Level", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            bank2_pan: FloatParam::new("B2 Pan", 0.2, FloatRange::Linear { min: -1.0, max: 1.0 }),

            bank3_enable: BoolParam::new("B3 On", false),
            bank3_sync: BoolParam::new("B3 Sync", false),
            bank3_time_note: EnumParam::new("B3 Note", DelayNote::Sixteenth),
            bank3_time_ms: FloatParam::new(
                "B3 Time",
                480.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1500.0,
                    factor: 0.5,
                },
            )
            .with_unit("ms"),
            bank3_feedback: FloatParam::new("B3 Feedback", 0.35, FloatRange::Linear { min: 0.0, max: 0.95 }),
            bank3_level: FloatParam::new("B3 Level", 0.4, FloatRange::Linear { min: 0.0, max: 1.0 }),
            bank3_pan: FloatParam::new("B3 Pan", 0.0, FloatRange::Linear { min: -1.0, max: 1.0 }),

            bank4_enable: BoolParam::new("B4 On", false),
            bank4_sync: BoolParam::new("B4 Sync", false),
            bank4_time_note: EnumParam::new("B4 Note", DelayNote::DottedEighth),
            bank4_time_ms: FloatParam::new(
                "B4 Time",
                620.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1500.0,
                    factor: 0.5,
                },
            )
            .with_unit("ms"),
            bank4_feedback: FloatParam::new("B4 Feedback", 0.45, FloatRange::Linear { min: 0.0, max: 0.95 }),
            bank4_level: FloatParam::new("B4 Level", 0.35, FloatRange::Linear { min: 0.0, max: 1.0 }),
            bank4_pan: FloatParam::new("B4 Pan", -0.3, FloatRange::Linear { min: -1.0, max: 1.0 }),

            bank5_enable: BoolParam::new("B5 On", false),
            bank5_sync: BoolParam::new("B5 Sync", false),
            bank5_time_note: EnumParam::new("B5 Note", DelayNote::DottedQuarter),
            bank5_time_ms: FloatParam::new(
                "B5 Time",
                820.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1500.0,
                    factor: 0.5,
                },
            )
            .with_unit("ms"),
            bank5_feedback: FloatParam::new("B5 Feedback", 0.4, FloatRange::Linear { min: 0.0, max: 0.95 }),
            bank5_level: FloatParam::new("B5 Level", 0.3, FloatRange::Linear { min: 0.0, max: 1.0 }),
            bank5_pan: FloatParam::new("B5 Pan", 0.3, FloatRange::Linear { min: -1.0, max: 1.0 }),

            bank6_enable: BoolParam::new("B6 On", false),
            bank6_sync: BoolParam::new("B6 Sync", false),
            bank6_time_note: EnumParam::new("B6 Note", DelayNote::Whole),
            bank6_time_ms: FloatParam::new(
                "B6 Time",
                980.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 1500.0,
                    factor: 0.5,
                },
            )
            .with_unit("ms"),
            bank6_feedback: FloatParam::new("B6 Feedback", 0.35, FloatRange::Linear { min: 0.0, max: 0.95 }),
            bank6_level: FloatParam::new("B6 Level", 0.25, FloatRange::Linear { min: 0.0, max: 1.0 }),
            bank6_pan: FloatParam::new("B6 Pan", -0.1, FloatRange::Linear { min: -1.0, max: 1.0 }),
        }
    }
}

struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
}

impl DelayLine {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            write_pos: 0,
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    fn process(&mut self, input: f32, delay_samples: f32, feedback: f32) -> f32 {
        let len = self.buffer.len();
        let delay_samples = delay_samples.clamp(1.0, (len - 2) as f32);
        let delay_int = delay_samples.floor() as usize;
        let frac = delay_samples - delay_int as f32;

        let read_pos = (self.write_pos + len - delay_int) % len;
        let read_pos_2 = (read_pos + len - 1) % len;
        let s1 = self.buffer[read_pos];
        let s2 = self.buffer[read_pos_2];
        let delayed = s1 + (s2 - s1) * frac;

        self.buffer[self.write_pos] = input + delayed * feedback;
        self.write_pos = (self.write_pos + 1) % len;

        delayed
    }
}

struct OnePoleLowpass {
    a: f32,
    y: f32,
}

impl OnePoleLowpass {
    fn new() -> Self {
        Self { a: 0.0, y: 0.0 }
    }

    fn set_cutoff(&mut self, cutoff: f32, sample_rate: f32) {
        let cutoff = cutoff.clamp(20.0, sample_rate * 0.49);
        self.a = (-2.0 * PI * cutoff / sample_rate).exp();
    }

    fn process(&mut self, input: f32) -> f32 {
        self.y = (1.0 - self.a) * input + self.a * self.y;
        self.y
    }
}

struct OnePoleHighpass {
    a: f32,
    y: f32,
    x_prev: f32,
}

impl OnePoleHighpass {
    fn new() -> Self {
        Self {
            a: 0.0,
            y: 0.0,
            x_prev: 0.0,
        }
    }

    fn set_cutoff(&mut self, cutoff: f32, sample_rate: f32) {
        let cutoff = cutoff.clamp(20.0, sample_rate * 0.49);
        self.a = (-2.0 * PI * cutoff / sample_rate).exp();
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.a * (self.y + input - self.x_prev);
        self.x_prev = input;
        self.y = output;
        output
    }
}

struct BitCrusherState {
    held: f32,
    counter: f32,
}

impl BitCrusherState {
    fn new() -> Self {
        Self { held: 0.0, counter: 0.0 }
    }

    fn reset(&mut self) {
        self.held = 0.0;
        self.counter = 0.0;
    }

    fn process(&mut self, input: f32, sample_rate: f32, target_rate: f32, bits: u32) -> f32 {
        let target_rate = target_rate.clamp(1000.0, 48000.0);
        let hold = (sample_rate / target_rate).max(1.0);
        self.counter += 1.0;
        if self.counter >= hold {
            self.counter -= hold;
            self.held = input;
        }
        quantize(self.held, bits)
    }
}

struct DelayBankState {
    delay_l: DelayLine,
    delay_r: DelayLine,
}

impl DelayBankState {
    fn new(size: usize) -> Self {
        Self {
            delay_l: DelayLine::new(size),
            delay_r: DelayLine::new(size),
        }
    }

    fn reset(&mut self) {
        self.delay_l.reset();
        self.delay_r.reset();
    }
}

pub struct DelayBank {
    params: Arc<DelayBankParams>,
    sample_rate: f32,
    current_preset: i32,
    banks: [DelayBankState; NUM_BANKS],
    lp_l: OnePoleLowpass,
    lp_r: OnePoleLowpass,
    hp_l: OnePoleHighpass,
    hp_r: OnePoleHighpass,
    crush_l: BitCrusherState,
    crush_r: BitCrusherState,
    scope: Arc<ScopeBuffer>,
    scope_decim: usize,
    scope_counter: usize,
}

impl Default for DelayBank {
    fn default() -> Self {
        let delay_samples = (MAX_DELAY_SECONDS * 44100.0) as usize;
        let params = Arc::new(DelayBankParams::default());
        let scope = params.scope.clone();
        Self {
            params,
            sample_rate: 44100.0,
            current_preset: 0,
            banks: [
                DelayBankState::new(delay_samples),
                DelayBankState::new(delay_samples),
                DelayBankState::new(delay_samples),
                DelayBankState::new(delay_samples),
                DelayBankState::new(delay_samples),
                DelayBankState::new(delay_samples),
            ],
            lp_l: OnePoleLowpass::new(),
            lp_r: OnePoleLowpass::new(),
            hp_l: OnePoleHighpass::new(),
            hp_r: OnePoleHighpass::new(),
            crush_l: BitCrusherState::new(),
            crush_r: BitCrusherState::new(),
            scope,
            scope_decim: 256,
            scope_counter: 0,
        }
    }
}

impl Plugin for DelayBank {
    const NAME: &'static str = "DelayBank";
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
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        let delay_samples = (MAX_DELAY_SECONDS * self.sample_rate) as usize;
        self.banks = [
            DelayBankState::new(delay_samples),
            DelayBankState::new(delay_samples),
            DelayBankState::new(delay_samples),
            DelayBankState::new(delay_samples),
            DelayBankState::new(delay_samples),
            DelayBankState::new(delay_samples),
        ];
        self.scope_decim = (self.sample_rate / 120.0).round().max(1.0) as usize;
        self.scope_counter = 0;
        true
    }

    fn reset(&mut self) {
        for bank in &mut self.banks {
            bank.reset();
        }
        self.lp_l = OnePoleLowpass::new();
        self.lp_r = OnePoleLowpass::new();
        self.hp_l = OnePoleHighpass::new();
        self.hp_r = OnePoleHighpass::new();
        self.crush_l.reset();
        self.crush_r.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let preset = self.params.preset.value();
        if preset != self.current_preset {
            apply_preset(&self.params, preset as usize);
            self.current_preset = preset;
        }

        let block_params = BlockParams {
            mix: self.params.mix.value(),
            input_gain: db_to_gain(self.params.input_trim.value()),
            output_gain: db_to_gain(self.params.output_trim.value()),
            hp_cut: self.params.hp_cut.value(),
            lp_cut: self.params.lp_cut.value(),
            crush_bits: self.params.crush_depth.value() as u32,
            crush_rate: self.params.crush_rate.value(),
            crush_mix: self.params.crush_mix.value(),
            banks: [
                BankBlockParams {
                    enabled: self.params.bank1_enable.value(),
                    sync: self.params.bank1_sync.value(),
                    note: self.params.bank1_time_note.value(),
                    time_ms: self.params.bank1_time_ms.value(),
                    feedback: self.params.bank1_feedback.value(),
                    level: self.params.bank1_level.value(),
                    pan: self.params.bank1_pan.value(),
                },
                BankBlockParams {
                    enabled: self.params.bank2_enable.value(),
                    sync: self.params.bank2_sync.value(),
                    note: self.params.bank2_time_note.value(),
                    time_ms: self.params.bank2_time_ms.value(),
                    feedback: self.params.bank2_feedback.value(),
                    level: self.params.bank2_level.value(),
                    pan: self.params.bank2_pan.value(),
                },
                BankBlockParams {
                    enabled: self.params.bank3_enable.value(),
                    sync: self.params.bank3_sync.value(),
                    note: self.params.bank3_time_note.value(),
                    time_ms: self.params.bank3_time_ms.value(),
                    feedback: self.params.bank3_feedback.value(),
                    level: self.params.bank3_level.value(),
                    pan: self.params.bank3_pan.value(),
                },
                BankBlockParams {
                    enabled: self.params.bank4_enable.value(),
                    sync: self.params.bank4_sync.value(),
                    note: self.params.bank4_time_note.value(),
                    time_ms: self.params.bank4_time_ms.value(),
                    feedback: self.params.bank4_feedback.value(),
                    level: self.params.bank4_level.value(),
                    pan: self.params.bank4_pan.value(),
                },
                BankBlockParams {
                    enabled: self.params.bank5_enable.value(),
                    sync: self.params.bank5_sync.value(),
                    note: self.params.bank5_time_note.value(),
                    time_ms: self.params.bank5_time_ms.value(),
                    feedback: self.params.bank5_feedback.value(),
                    level: self.params.bank5_level.value(),
                    pan: self.params.bank5_pan.value(),
                },
                BankBlockParams {
                    enabled: self.params.bank6_enable.value(),
                    sync: self.params.bank6_sync.value(),
                    note: self.params.bank6_time_note.value(),
                    time_ms: self.params.bank6_time_ms.value(),
                    feedback: self.params.bank6_feedback.value(),
                    level: self.params.bank6_level.value(),
                    pan: self.params.bank6_pan.value(),
                },
            ],
        };

        self.hp_l.set_cutoff(block_params.hp_cut, self.sample_rate);
        self.hp_r.set_cutoff(block_params.hp_cut, self.sample_rate);
        self.lp_l.set_cutoff(block_params.lp_cut, self.sample_rate);
        self.lp_r.set_cutoff(block_params.lp_cut, self.sample_rate);

        let tempo = context.transport().tempo.map(|tempo| tempo as f32);
        let num_samples = buffer.samples();
        let output = buffer.as_slice();
        if output.is_empty() {
            return ProcessStatus::Normal;
        }

        const MAX_BLOCK_SIZE: usize = 64;
        let mut block_start: usize = 0;
        let has_stereo = output.len() >= 2;

        while block_start < num_samples {
            let block_end = (block_start + MAX_BLOCK_SIZE).min(num_samples);

            for i in block_start..block_end {
                let in_l = output[0][i] * block_params.input_gain;
                let in_r = if has_stereo { output[1][i] * block_params.input_gain } else { in_l };
                let input_mono = 0.5 * (in_l + in_r);

                let (mut wet_l, mut wet_r) = process_banks(
                    &mut self.banks,
                    &block_params,
                    input_mono,
                    self.sample_rate,
                    tempo,
                );

                wet_l = self.lp_l.process(self.hp_l.process(wet_l));
                wet_r = self.lp_r.process(self.hp_r.process(wet_r));

                let crushed_l = self.crush_l.process(
                    wet_l,
                    self.sample_rate,
                    block_params.crush_rate,
                    block_params.crush_bits,
                );
                let crushed_r = self.crush_r.process(
                    wet_r,
                    self.sample_rate,
                    block_params.crush_rate,
                    block_params.crush_bits,
                );

                wet_l = lerp(wet_l, crushed_l, block_params.crush_mix);
                wet_r = lerp(wet_r, crushed_r, block_params.crush_mix);

                let out_l = lerp(in_l, wet_l, block_params.mix) * block_params.output_gain;
                let out_r = lerp(in_r, wet_r, block_params.mix) * block_params.output_gain;

                output[0][i] = out_l;
                if has_stereo {
                    output[1][i] = out_r;
                }

                self.scope_counter += 1;
                if self.scope_counter >= self.scope_decim {
                    self.scope_counter = 0;
                    self.scope.push(0.5 * (out_l + out_r));
                }
            }

            block_start = block_end;
        }

        ProcessStatus::Normal
    }
}

fn process_banks(
    banks: &mut [DelayBankState; NUM_BANKS],
    params: &BlockParams,
    input_mono: f32,
    sample_rate: f32,
    tempo: Option<f32>,
) -> (f32, f32) {
    let mut wet_l = 0.0;
    let mut wet_r = 0.0;

    for bank_idx in 0..NUM_BANKS {
        let b = &params.banks[bank_idx];
        let (enabled, sync, note, time_ms, feedback, level, pan) = (
            b.enabled, b.sync, b.note, b.time_ms, b.feedback, b.level, b.pan
        );

        if !enabled {
            continue;
        }

        let delay_ms = if sync {
            tempo
                .map(|tempo| 60000.0 * note.beats() / tempo)
                .unwrap_or(time_ms)
        } else {
            time_ms
        };

        let delay_samples = delay_ms * sample_rate / 1000.0;
        let delayed_l = banks[bank_idx]
            .delay_l
            .process(input_mono, delay_samples, feedback);
        let delayed_r = banks[bank_idx]
            .delay_r
            .process(input_mono, delay_samples, feedback);

        let (pan_l, pan_r) = pan_gains(pan);
        wet_l += delayed_l * level * pan_l;
        wet_r += delayed_r * level * pan_r;
    }

    (wet_l, wet_r)
}

impl ClapPlugin for DelayBank {
    const CLAP_ID: &'static str = "art.taellinglin.delaybank";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Multi-bank delay effect");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for DelayBank {
    const VST3_CLASS_ID: [u8; 16] = *b"DelayBankFX0001!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(DelayBank);
nih_export_vst3!(DelayBank);

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

fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    let angle = (pan + 1.0) * 0.25 * PI;
    (angle.cos(), angle.sin())
}
