use crate::drum_model::{default_instrument_for_slot, instrument_label, InstrumentType};

pub const GM_NOTES: [u8; 32] = [
    36, 38, 42, 46, 44, 51, 49, 37, 39, 41, 45, 48, 50, 52, 55, 57, 36, 37, 38, 39, 40, 41,
    42, 43, 44, 45, 46, 47, 48, 49, 50, 51,
];

pub const GM_NAMES: [&str; 32] = [
    "Kick",
    "Snare",
    "Hat Closed",
    "Hat Open",
    "Hat Pedal",
    "Ride",
    "Crash",
    "Rimshot",
    "Clap",
    "Tom Low",
    "Tom Mid",
    "Tom High",
    "Perc 1",
    "Perc 2",
    "FX 1",
    "FX 2",
    "B01",
    "B02",
    "B03",
    "B04",
    "B05",
    "B06",
    "B07",
    "B08",
    "B09",
    "B10",
    "B11",
    "B12",
    "B13",
    "B14",
    "B15",
    "B16",
];

pub fn slot_for_note(note: u8) -> Option<usize> {
    GM_NOTES.iter().position(|mapped| *mapped == note)
}

pub fn default_note_name(note: u8) -> Option<&'static str> {
    slot_for_note(note).map(|idx| GM_NAMES[idx])
}

pub fn default_instrument_for_slot_index(index: usize) -> InstrumentType {
    default_instrument_for_slot(index)
}

pub fn default_instrument_label(index: usize) -> &'static str {
    instrument_label(default_instrument_for_slot(index))
}
