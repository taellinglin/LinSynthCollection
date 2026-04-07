use enum_iterator::Sequence;
use nih_plug::params::enums::Enum;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Enum, Sequence)]
pub enum Waveform {
    Sine,
    Triangle,
    Sawtooth,
    Square,
    Pulse,
    Noise,
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

/// Generate FM synthesis output
/// carrier_waveform: the carrier oscillator waveform
/// modulator_waveform: the modulator oscillator waveform
/// carrier_phase: the current phase of the carrier oscillator (0.0 to 1.0)
/// modulator_phase: the current phase of the modulator oscillator (0.0 to 1.0)
/// fm_amount: the amount of frequency modulation (modulation index)
pub fn generate_fm_waveform(
    carrier_waveform: Waveform,
    modulator_waveform: Waveform,
    carrier_phase: f32,
    modulator_phase: f32,
    fm_amount: f32,
) -> f32 {
    // Generate the modulator output
    let modulator_output = generate_waveform(modulator_waveform, modulator_phase);
    
    // Apply FM: modulate the carrier phase with the modulator
    // The modulator output is scaled by fm_amount (modulation index)
    let modulated_phase = (carrier_phase + modulator_output * fm_amount).fract();
    
    // Generate the carrier output with the modulated phase
    generate_waveform(carrier_waveform, modulated_phase)
}
