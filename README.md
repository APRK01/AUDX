# AUDX

A beautiful, real-time audio visualizer for macOS.

![AUDX Preview](https://img.shields.io/badge/platform-macOS-black?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)

## Features

- **Real-time FFT analysis** with logarithmic frequency scaling
- **64 frequency bars** covering 20Hz - 20kHz
- **Smooth animations** with asymmetric attack/decay
- **Frequency compensation** for balanced visualization
- **Apple-inspired UI** with glassmorphism
- **Transparent window** - floats beautifully on your desktop
- **Draggable** - position it anywhere

## Tech Stack

- **Tauri 2** - Native desktop app framework
- **Rust** - Audio processing with `cpal` + `rustfft`
- **React + TypeScript** - UI rendering
- **Vite** - Fast builds

## Installation

```bash
# Clone the repo
git clone https://github.com/APRK01/AUDX.git
cd AUDX

# Install dependencies
npm install

# Run in development
npm run tauri dev

# Build for production
npm run tauri build
```

## Requirements

- macOS 11+
- Node.js 18+
- Rust 1.70+
- Microphone permission (for audio input)

## How It Works

1. **Audio Capture** - Uses `cpal` to capture system audio input
2. **FFT Processing** - Applies Hann window + FFT via `rustfft`
3. **Log Binning** - Maps FFT bins to 64 logarithmic frequency bands
4. **dB Scaling** - Converts to decibels for natural perception
5. **Smoothing** - Asymmetric smoothing (fast rise, slow fall)
6. **Rendering** - Canvas-based bars with gradient fills

## Configuration

Edit `src-tauri/src/lib.rs` to customize:

```rust
const NUM_BARS: usize = 64;        // Number of bars
const MIN_FREQ: f32 = 20.0;        // Lowest frequency
const MAX_FREQ: f32 = 20000.0;     // Highest frequency
const SENSITIVITY: f32 = 1.5;      // Overall gain
```

## Author

**APRK** (Advaith Praveen)

## License

MIT
