use nih_plug::prelude::ParamMut;

use crate::{DelayBankParams, DelayNote};

pub(crate) struct BankPreset {
    pub enabled: bool,
    pub time_ms: f32,
    pub time_note: DelayNote,
    pub sync: bool,
    pub feedback: f32,
    pub level: f32,
    pub pan: f32,
}

pub(crate) struct Preset {
    pub name: &'static str,
    pub mix: f32,
    pub hp_cut: f32,
    pub lp_cut: f32,
    pub crush_depth: i32,
    pub crush_rate: f32,
    pub crush_mix: f32,
    pub banks: [BankPreset; 6],
}

pub(crate) const PRESET_NAMES: [&str; 6] = [
    "Arcade Slap",
    "GB Ping",
    "Dungeon Echo",
    "CRT Wash",
    "Boss Arena",
    "Chiptape",
];

pub(crate) const PRESETS: [Preset; 6] = [
    Preset {
        name: PRESET_NAMES[0],
        mix: 0.55,
        hp_cut: 120.0,
        lp_cut: 8000.0,
        crush_depth: 8,
        crush_rate: 14000.0,
        crush_mix: 0.1,
        banks: [
            BankPreset { enabled: true, time_ms: 120.0, time_note: DelayNote::Eighth, sync: false, feedback: 0.3, level: 0.95, pan: -0.4 },
            BankPreset { enabled: true, time_ms: 200.0, time_note: DelayNote::Sixteenth, sync: false, feedback: 0.35, level: 0.8, pan: 0.4 },
            BankPreset { enabled: false, time_ms: 360.0, time_note: DelayNote::ThirtySecond, sync: false, feedback: 0.3, level: 0.4, pan: 0.0 },
            BankPreset { enabled: false, time_ms: 520.0, time_note: DelayNote::DottedEighth, sync: false, feedback: 0.35, level: 0.3, pan: -0.2 },
            BankPreset { enabled: false, time_ms: 720.0, time_note: DelayNote::DottedSixteenth, sync: false, feedback: 0.4, level: 0.3, pan: 0.2 },
            BankPreset { enabled: false, time_ms: 980.0, time_note: DelayNote::Quarter, sync: false, feedback: 0.45, level: 0.25, pan: 0.0 },
        ],
    },
    Preset {
        name: PRESET_NAMES[1],
        mix: 0.6,
        hp_cut: 180.0,
        lp_cut: 6000.0,
        crush_depth: 6,
        crush_rate: 9000.0,
        crush_mix: 0.2,
        banks: [
            BankPreset { enabled: true, time_ms: 80.0, time_note: DelayNote::Sixteenth, sync: false, feedback: 0.45, level: 0.95, pan: -0.3 },
            BankPreset { enabled: true, time_ms: 160.0, time_note: DelayNote::ThirtySecond, sync: false, feedback: 0.4, level: 0.85, pan: 0.3 },
            BankPreset { enabled: true, time_ms: 320.0, time_note: DelayNote::DottedSixteenth, sync: false, feedback: 0.35, level: 0.6, pan: 0.0 },
            BankPreset { enabled: false, time_ms: 520.0, time_note: DelayNote::DottedEighth, sync: false, feedback: 0.4, level: 0.3, pan: 0.1 },
            BankPreset { enabled: false, time_ms: 720.0, time_note: DelayNote::Quarter, sync: false, feedback: 0.5, level: 0.25, pan: -0.1 },
            BankPreset { enabled: false, time_ms: 980.0, time_note: DelayNote::DottedQuarter, sync: false, feedback: 0.6, level: 0.2, pan: 0.0 },
        ],
    },
    Preset {
        name: PRESET_NAMES[2],
        mix: 0.7,
        hp_cut: 90.0,
        lp_cut: 7000.0,
        crush_depth: 10,
        crush_rate: 12000.0,
        crush_mix: 0.12,
        banks: [
            BankPreset { enabled: true, time_ms: 320.0, time_note: DelayNote::Quarter, sync: true, feedback: 0.55, level: 0.9, pan: -0.5 },
            BankPreset { enabled: true, time_ms: 480.0, time_note: DelayNote::Eighth, sync: true, feedback: 0.6, level: 0.85, pan: 0.5 },
            BankPreset { enabled: true, time_ms: 720.0, time_note: DelayNote::Sixteenth, sync: true, feedback: 0.45, level: 0.65, pan: 0.0 },
            BankPreset { enabled: false, time_ms: 980.0, time_note: DelayNote::DottedEighth, sync: true, feedback: 0.4, level: 0.3, pan: -0.2 },
            BankPreset { enabled: false, time_ms: 1240.0, time_note: DelayNote::DottedQuarter, sync: true, feedback: 0.3, level: 0.2, pan: 0.2 },
            BankPreset { enabled: false, time_ms: 1480.0, time_note: DelayNote::Whole, sync: true, feedback: 0.25, level: 0.2, pan: 0.0 },
        ],
    },
    Preset {
        name: PRESET_NAMES[3],
        mix: 0.75,
        hp_cut: 60.0,
        lp_cut: 9000.0,
        crush_depth: 12,
        crush_rate: 18000.0,
        crush_mix: 0.08,
        banks: [
            BankPreset { enabled: true, time_ms: 260.0, time_note: DelayNote::Eighth, sync: true, feedback: 0.65, level: 0.9, pan: -0.4 },
            BankPreset { enabled: true, time_ms: 340.0, time_note: DelayNote::Sixteenth, sync: true, feedback: 0.7, level: 0.85, pan: 0.4 },
            BankPreset { enabled: true, time_ms: 520.0, time_note: DelayNote::DottedEighth, sync: true, feedback: 0.55, level: 0.7, pan: 0.0 },
            BankPreset { enabled: true, time_ms: 780.0, time_note: DelayNote::DottedQuarter, sync: true, feedback: 0.45, level: 0.55, pan: -0.2 },
            BankPreset { enabled: false, time_ms: 980.0, time_note: DelayNote::Quarter, sync: true, feedback: 0.4, level: 0.3, pan: 0.2 },
            BankPreset { enabled: false, time_ms: 1240.0, time_note: DelayNote::Whole, sync: true, feedback: 0.25, level: 0.2, pan: 0.0 },
        ],
    },
    Preset {
        name: PRESET_NAMES[4],
        mix: 0.8,
        hp_cut: 80.0,
        lp_cut: 10000.0,
        crush_depth: 8,
        crush_rate: 15000.0,
        crush_mix: 0.12,
        banks: [
            BankPreset { enabled: true, time_ms: 180.0, time_note: DelayNote::Half, sync: true, feedback: 0.5, level: 0.8, pan: -0.3 },
            BankPreset { enabled: true, time_ms: 360.0, time_note: DelayNote::Quarter, sync: true, feedback: 0.6, level: 0.9, pan: 0.3 },
            BankPreset { enabled: true, time_ms: 540.0, time_note: DelayNote::Eighth, sync: true, feedback: 0.65, level: 0.8, pan: -0.1 },
            BankPreset { enabled: true, time_ms: 720.0, time_note: DelayNote::Sixteenth, sync: true, feedback: 0.7, level: 0.7, pan: 0.1 },
            BankPreset { enabled: false, time_ms: 980.0, time_note: DelayNote::DottedEighth, sync: true, feedback: 0.4, level: 0.4, pan: -0.2 },
            BankPreset { enabled: false, time_ms: 1240.0, time_note: DelayNote::DottedQuarter, sync: true, feedback: 0.35, level: 0.3, pan: 0.2 },
        ],
    },
    Preset {
        name: PRESET_NAMES[5],
        mix: 0.65,
        hp_cut: 150.0,
        lp_cut: 7000.0,
        crush_depth: 5,
        crush_rate: 8000.0,
        crush_mix: 0.25,
        banks: [
            BankPreset { enabled: true, time_ms: 140.0, time_note: DelayNote::Sixteenth, sync: false, feedback: 0.45, level: 0.85, pan: -0.2 },
            BankPreset { enabled: true, time_ms: 280.0, time_note: DelayNote::ThirtySecond, sync: false, feedback: 0.5, level: 0.8, pan: 0.2 },
            BankPreset { enabled: true, time_ms: 420.0, time_note: DelayNote::DottedSixteenth, sync: false, feedback: 0.45, level: 0.65, pan: 0.0 },
            BankPreset { enabled: false, time_ms: 620.0, time_note: DelayNote::DottedEighth, sync: false, feedback: 0.3, level: 0.35, pan: -0.1 },
            BankPreset { enabled: false, time_ms: 820.0, time_note: DelayNote::Quarter, sync: false, feedback: 0.25, level: 0.3, pan: 0.1 },
            BankPreset { enabled: false, time_ms: 1080.0, time_note: DelayNote::DottedQuarter, sync: false, feedback: 0.2, level: 0.25, pan: 0.0 },
        ],
    },
];

pub(crate) fn apply_preset(params: &DelayBankParams, index: usize) {
    let preset = &PRESETS[index.min(PRESETS.len() - 1)];

    params.mix.set_plain_value(preset.mix);
    params.hp_cut.set_plain_value(preset.hp_cut);
    params.lp_cut.set_plain_value(preset.lp_cut);
    params.crush_depth.set_plain_value(preset.crush_depth);
    params.crush_rate.set_plain_value(preset.crush_rate);
    params.crush_mix.set_plain_value(preset.crush_mix);

    let banks = &preset.banks;
    params.bank1_enable.set_plain_value(banks[0].enabled);
    params.bank1_time_ms.set_plain_value(banks[0].time_ms);
    params.bank1_time_note.set_plain_value(banks[0].time_note);
    params.bank1_sync.set_plain_value(banks[0].sync);
    params.bank1_feedback.set_plain_value(banks[0].feedback);
    params.bank1_level.set_plain_value(banks[0].level);
    params.bank1_pan.set_plain_value(banks[0].pan);

    params.bank2_enable.set_plain_value(banks[1].enabled);
    params.bank2_time_ms.set_plain_value(banks[1].time_ms);
    params.bank2_time_note.set_plain_value(banks[1].time_note);
    params.bank2_sync.set_plain_value(banks[1].sync);
    params.bank2_feedback.set_plain_value(banks[1].feedback);
    params.bank2_level.set_plain_value(banks[1].level);
    params.bank2_pan.set_plain_value(banks[1].pan);

    params.bank3_enable.set_plain_value(banks[2].enabled);
    params.bank3_time_ms.set_plain_value(banks[2].time_ms);
    params.bank3_time_note.set_plain_value(banks[2].time_note);
    params.bank3_sync.set_plain_value(banks[2].sync);
    params.bank3_feedback.set_plain_value(banks[2].feedback);
    params.bank3_level.set_plain_value(banks[2].level);
    params.bank3_pan.set_plain_value(banks[2].pan);

    params.bank4_enable.set_plain_value(banks[3].enabled);
    params.bank4_time_ms.set_plain_value(banks[3].time_ms);
    params.bank4_time_note.set_plain_value(banks[3].time_note);
    params.bank4_sync.set_plain_value(banks[3].sync);
    params.bank4_feedback.set_plain_value(banks[3].feedback);
    params.bank4_level.set_plain_value(banks[3].level);
    params.bank4_pan.set_plain_value(banks[3].pan);

    params.bank5_enable.set_plain_value(banks[4].enabled);
    params.bank5_time_ms.set_plain_value(banks[4].time_ms);
    params.bank5_time_note.set_plain_value(banks[4].time_note);
    params.bank5_sync.set_plain_value(banks[4].sync);
    params.bank5_feedback.set_plain_value(banks[4].feedback);
    params.bank5_level.set_plain_value(banks[4].level);
    params.bank5_pan.set_plain_value(banks[4].pan);

    params.bank6_enable.set_plain_value(banks[5].enabled);
    params.bank6_time_ms.set_plain_value(banks[5].time_ms);
    params.bank6_time_note.set_plain_value(banks[5].time_note);
    params.bank6_sync.set_plain_value(banks[5].sync);
    params.bank6_feedback.set_plain_value(banks[5].feedback);
    params.bank6_level.set_plain_value(banks[5].level);
    params.bank6_pan.set_plain_value(banks[5].pan);
}
