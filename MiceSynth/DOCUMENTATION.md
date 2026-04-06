# MiceSynth - Complete Documentation

[![Automated builds](https://github.com/taellinglin/MiceSynth/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/taellinglin/MiceSynth/actions/workflows/build.yml?query=branch%3Amaster)
[![Tests](https://github.com/taellinglin/MiceSynth/actions/workflows/test.yml/badge.svg?branch=master)](https://github.com/taellinglin/MiceSynth/actions/workflows/test.yml?query=branch%3Amaster)
[![Documentation](https://github.com/taellinglin/MiceSynth/actions/workflows/docs.yml/badge.svg?branch=master)](https://github.com/taellinglin/MiceSynth/actions/workflows/docs.yml?query=branch%3Amaster)

---

## Overview

**MiceSynth** is a knitty gritty wavetable bass synthesizer implemented as a VST3/CLAP plugin, built with the NIH-plug framework. It blends aggressive modulation, sub layering, and wavetable distortion to deliver dubstep-ready bass and heavy electronic sound design.

### Key Features

- ðŸŽ¹ **16-Voice Polyphony** - Smooth, responsive voice management
- ðŸŒŠ **Multiple Waveforms** - Sine, Square, Sawtooth, Triangle, Pulse, and Noise
- ðŸŽšï¸ **Full ADSR Envelopes** - Independent control for amplitude and filter parameters
- ðŸ”¥ **Wavetable Distortion** - Wavefolding drive for gritty harmonics
- ðŸ”Š **Bass-Focused Filtering** - Low-pass, High-pass, Band-pass, Notch, Statevariable
- ðŸ§± **Sub Layer** - Dedicated sub level for weight and punch
- ðŸŽ¨ **Modern UI** - Clean, intuitive interface with custom zCool font
- ðŸ”Œ **Universal Compatibility** - VST3 and CLAP plugin formats
- ðŸ’» **Cross-Platform** - Windows, macOS (Universal Binary), and Linux

---

## Installation

### System Requirements

- **Windows**: Windows 10 or later (64-bit)
- **macOS**: macOS 10.13 (High Sierra) or later
- **Linux**: Modern distribution with X11 or Wayland
- **DAW**: Any VST3 or CLAP compatible host

### Installation Instructions

#### Windows
1. Download the latest release from the [Releases page](https://github.com/taellinglin/MiceSynth/releases)
2. Extract the archive
3. Copy `MiceSynth.vst3` folder to: `C:\Program Files\Common Files\VST3\`
4. Copy `MiceSynth.clap` file to: `C:\Program Files\Common Files\CLAP\`
5. Restart your DAW and rescan plugins

#### macOS
1. Download the latest macOS release
2. Extract the archive
3. **Gatekeeper**: Right-click the plugin bundle and select "Open" to bypass security
   - Alternative: Visit [disable-gatekeeper.github.io](https://disable-gatekeeper.github.io/)
4. Copy `MiceSynth.vst3` to: `~/Library/Audio/Plug-Ins/VST3`
5. Copy `MiceSynth.clap` to: `~/Library/Audio/Plug-Ins/CLAP`
6. Restart your DAW

#### Linux
1. Download the Linux release
2. Extract and copy files:
   ```bash
   mkdir -p ~/.vst3 ~/.clap
   cp -r MiceSynth.vst3 ~/.vst3/
   cp MiceSynth.clap ~/.clap/
   chmod +x ~/.vst3/MiceSynth.vst3/Contents/x86_64-linux/MiceSynth.so
   chmod +x ~/.clap/MiceSynth.clap
   ```
3. Rescan plugins in your DAW

---

## Building from Source

### Prerequisites

**Required:**
- [Rust](https://rustup.rs/) 1.79.0 or later
- Git

**Platform-Specific Dependencies:**

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get install -y libasound2-dev libgl-dev libjack-dev \
  libx11-xcb-dev libxcb1-dev libxcb-dri2-0-dev libxcb-icccm4-dev \
  libxcursor-dev libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

**macOS:**
```bash
xcode-select --install
```

**Windows:**
No additional dependencies required.

### Build Instructions

```bash
# Clone the repository
git clone https://github.com/taellinglin/MiceSynth.git
cd MiceSynth

# Build release version
cargo xtask bundle MiceSynth --release
```

The compiled plugins will be located in `target/bundled/`:
- `MiceSynth.vst3` - VST3 plugin
- `MiceSynth.clap` - CLAP plugin

### Development Build

```bash
# Build debug version (faster compilation, slower runtime)
cargo xtask bundle MiceSynth

# Run tests
cargo test --workspace

# Generate documentation
cargo doc --no-deps --open
```

---

## User Interface

MiceSynth features a clean, organized interface using the custom **zCool XiaoWei** font for enhanced readability.

### Interface Layout

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                       MiceSynth                          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  Gain            â”‚  Filter Type     â”‚  Filter Cut Env   â”‚
â”‚  Waveform        â”‚  Filter Cutoff   â”‚  - Attack         â”‚
â”‚  Filter Type     â”‚  Filter Resonanceâ”‚  - Decay          â”‚
â”‚  Filter Cut      â”‚                  â”‚  - Sustain        â”‚
â”‚  Filter Res      â”‚  Amp Envelope    â”‚  - Release        â”‚
â”‚                  â”‚  - Attack        â”‚                   â”‚
â”‚                  â”‚  - Decay         â”‚  Filter Res Env   â”‚
â”‚                  â”‚  - Sustain       â”‚  - Attack         â”‚
â”‚                  â”‚  - Release       â”‚  - Decay          â”‚
â”‚                  â”‚                  â”‚  - Sustain        â”‚
â”‚                  â”‚                  â”‚  - Release        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

## Parameters Reference

### Amplitude Control

#### **Gain** (Master Output)
- **Range**: -30.0 dB to +30.0 dB
- **Default**: 0.0 dB
- **Description**: Controls the overall output volume of the synthesizer
- **Usage**: Set this to unity gain (0 dB) for standard operation, increase for louder output

### Oscillator Section

#### **Waveform**
Available waveforms with distinct tonal characteristics:

- **Sine** - Pure fundamental frequency, smooth and clean
  - Use for: Sub bass, pure tones, FM synthesis
  
- **Square** - Rich in odd harmonics, hollow sound
  - Use for: Leads, bass, retro game sounds
  
- **Sawtooth** - Full harmonic spectrum, bright and rich
  - Use for: Strings, brass, aggressive leads
  
- **Triangle** - Softer than square, contains odd harmonics
  - Use for: Flutes, mellow pads, soft leads

- **Pulse** - Narrow duty cycle, nasal and buzzy
  - Use for: Growls, chiptune leads, resonant bass
  
- **Noise** - Random signal, no pitch
  - Use for: Percussion, wind effects, texture

### Amplitude Envelope (ADSR)

#### **Attack** (amp_atk)
- **Range**: 0.0 ms to 2000.0 ms
- **Default**: 5.0 ms
- **Description**: Time for the note to reach peak volume after trigger
- **Tips**: 
  - Fast (0-10ms): Percussive sounds, plucks
  - Medium (10-100ms): Piano-like sounds
  - Slow (100-2000ms): Pads, strings, ambient textures

#### **Decay** (amp_dec)
- **Range**: 0.0 ms to 2000.0 ms
- **Default**: 100.0 ms
- **Description**: Time to transition from peak to sustain level
- **Tips**: Shorter decay for punchy sounds, longer for evolving tones

#### **Sustain** (amp_sus)
- **Range**: 0.0 to 1.0 (0% to 100%)
- **Default**: 0.7 (70%)
- **Description**: Level maintained while note is held
- **Tips**: 
  - Low (0-0.3): Percussion, plucks
  - Medium (0.3-0.7): General purpose
  - High (0.7-1.0): Pads, sustained notes

#### **Release** (amp_rel)
- **Range**: 0.0 ms to 5000.0 ms
- **Default**: 100.0 ms
- **Description**: Time for sound to fade after note release
- **Tips**:
  - Fast (0-50ms): Staccato, tight sounds
  - Medium (50-500ms): Natural decay
  - Slow (500-5000ms): Ambient tails, reverb-like effects

### Filter Section

#### **Filter Type**
Available filter modes:

- **None** - Bypass filter, unprocessed oscillator output
- **Low-pass** - Removes frequencies above cutoff
  - Use for: Warmth, removing brightness, classic subtractive synthesis
- **High-pass** - Removes frequencies below cutoff
  - Use for: Thin sounds, removing rumble, special effects
- **Band-pass** - Only allows frequencies around cutoff
  - Use for: Telephone effect, vocal-like sounds, special textures

#### **Filter Cutoff** (filter_cut)
- **Range**: 20.0 Hz to 20000.0 Hz
- **Default**: 20000.0 Hz (wide open)
- **Description**: Frequency where filter begins to take effect
- **Tips**:
  - Low-pass: Lower values = darker sound
  - High-pass: Higher values = thinner sound
  - Modulate with envelope for dynamic tonal changes

#### **Filter Resonance** (filter_res)
- **Range**: 0.0 to 1.0 (0% to 100%)
- **Default**: 0.0
- **Description**: Emphasizes frequencies near the cutoff point
- **Tips**:
  - Low (0-0.3): Subtle filtering, natural sound
  - Medium (0.3-0.7): Pronounced character, classic analog sound
  - High (0.7-1.0): Resonant peak, self-oscillation, screaming leads

### Filter Cutoff Envelope

Independent ADSR envelope for modulating filter cutoff frequency.

#### **Attack** (filter_cut_atk)
- **Range**: 0.0 ms to 2000.0 ms
- **Default**: 50.0 ms
- **Description**: How quickly the filter opens after note trigger

#### **Decay** (filter_cut_dec)
- **Range**: 0.0 ms to 2000.0 ms
- **Default**: 200.0 ms

#### **Sustain** (filter_cut_sus)
- **Range**: 0.0 to 1.0
- **Default**: 0.5

#### **Release** (filter_cut_rel)
- **Range**: 0.0 ms to 5000.0 ms
- **Default**: 200.0 ms

### Filter Resonance Envelope

Independent ADSR envelope for modulating filter resonance.

#### **Attack** (filter_res_atk)
- **Range**: 0.0 ms to 2000.0 ms
- **Default**: 50.0 ms

#### **Decay** (filter_res_dec)
- **Range**: 0.0 ms to 2000.0 ms
- **Default**: 200.0 ms

#### **Sustain** (filter_res_sus)
- **Range**: 0.0 to 1.0
- **Default**: 0.0

#### **Release** (filter_res_rel)
- **Range**: 0.0 ms to 5000.0 ms
- **Default**: 200.0 ms

---

## Sound Design Tips

### Classic Bass
- **Waveform**: Sawtooth or Square
- **Filter**: Low-pass, cutoff ~200-800 Hz
- **Amp Envelope**: Fast attack (5ms), short decay (50ms), high sustain (0.8), short release (100ms)
- **Filter Envelope**: Medium attack (50ms), quick decay (100ms), low sustain (0.2)

### Pluck/Bell
- **Waveform**: Sine or Triangle
- **Filter**: Low-pass, cutoff ~2000 Hz, medium resonance (0.5)
- **Amp Envelope**: Instant attack (0ms), fast decay (100ms), low sustain (0.1), short release (50ms)
- **Filter Envelope**: Fast attack (5ms), medium decay (300ms), low sustain (0.1)

### Pad/String
- **Waveform**: Sawtooth
- **Filter**: Low-pass, cutoff ~5000 Hz, low resonance (0.2)
- **Amp Envelope**: Slow attack (500ms), medium decay (500ms), high sustain (0.8), long release (1000ms)
- **Filter Envelope**: Slow attack (800ms), slow decay (1000ms), medium sustain (0.6)

### Lead Synth
- **Waveform**: Square or Sawtooth
- **Filter**: Low-pass, cutoff ~1500 Hz, high resonance (0.7)
- **Amp Envelope**: Fast attack (5ms), short decay (100ms), high sustain (0.9), short release (100ms)
- **Filter Envelope**: Medium attack (100ms), quick decay (150ms), high sustain (0.7)

---

## Technical Specifications

### Audio Processing
- **Polyphony**: 16 voices
- **Sample Rate**: Supports all standard rates (44.1 kHz - 192 kHz)
- **Bit Depth**: 32-bit floating point internal processing
- **Latency**: Zero-latency processing
- **Voice Stealing**: Oldest-first algorithm

### DSP Algorithms
- **Oscillators**: Bandlimited wavetable synthesis
- **Filters**: State-variable filter implementation
- **Envelopes**: Linear interpolation ADSR
- **Voice Management**: Optimized round-robin allocation

### MIDI Implementation
- **MIDI Channels**: Omni (responds to all channels)
- **Note On/Off**: Full velocity sensitivity
- **Pitch Bend**: Â±2 semitones (standard)
- **Sustain Pedal**: CC64 support
- **Modulation**: Supports standard MIDI CC

---

## Troubleshooting

### Plugin Not Appearing in DAW

**Windows:**
- Verify files are in correct location
- Check DAW plugin search paths
- Rescan plugins in DAW preferences
- Ensure VST3/CLAP folders exist

**macOS:**
- Bypass Gatekeeper security (right-click > Open)
- Check Console.app for load errors
- Verify bundle is not quarantined: `xattr -d com.apple.quarantine MiceSynth.vst3`

**Linux:**
- Check file permissions (chmod +x)
- Verify shared library dependencies: `ldd MiceSynth.so`
- Check JACK/ALSA configuration

### Audio Issues

**Crackling/Distortion:**
- Reduce buffer size in DAW
- Lower voice count if CPU limited
- Reduce filter resonance
- Check output gain level

**No Sound:**
- Check MIDI routing in DAW
- Verify track is armed/enabled
- Check master gain setting
- Ensure filter cutoff is not too low

---

## Development

### Project Structure

```
MiceSynth/
â”œâ”€â”€ plugins/MiceSynth/     # Main plugin code
â”‚   â””â”€â”€ src/
â”‚       â”œâ”€â”€ lib.rs        # Plugin implementation
â”‚       â”œâ”€â”€ editor.rs     # UI code
â”‚       â”œâ”€â”€ envelope.rs   # ADSR envelope
â”‚       â”œâ”€â”€ filter.rs     # Filter implementations
â”‚       â”œâ”€â”€ waveform.rs   # Oscillator waveforms
â”‚       â””â”€â”€ assets/       # zCool font files
â”œâ”€â”€ src/                  # NIH-plug framework
â”œâ”€â”€ nih_plug_vizia/      # VIZIA UI integration
â”œâ”€â”€ .github/workflows/   # CI/CD configuration
â””â”€â”€ target/bundled/      # Build output
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test --package MiceSynth
```

### Code Style

This project follows standard Rust formatting:
```bash
cargo fmt
cargo clippy
```

---

## Contributing

Contributions are welcome! Please follow these guidelines:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Guidelines
- Follow Rust style guide
- Add tests for new features
- Update documentation
- Keep commits atomic and well-described

---

## License

See [LICENSE](LICENSE) file for details.

---

## Credits

### Development
- Built with [NIH-plug](https://github.com/robbert-vdh/nih-plug) by Robbert van der Helm
- UI powered by [VIZIA](https://github.com/vizia/vizia)

### Font
- **zCool XiaoWei** - Used throughout the interface for enhanced readability

### Special Thanks
- The Rust audio community
- NIH-plug contributors
- All beta testers and users

---

## Links

- **GitHub Repository**: https://github.com/taellinglin/MiceSynth
- **Issues/Bug Reports**: https://github.com/taellinglin/MiceSynth/issues
- **Demo Track**: https://soundcloud.com/taellinglin/8kwealj94t22
- **NIH-plug**: https://github.com/robbert-vdh/nih-plug

---

## Version History

See [CHANGELOG.md](CHANGELOG.md) for detailed version history.

---

*Documentation generated for MiceSynth v0.1.0*
*Last updated: 2026å¹´1æœˆ4æ—¥*


