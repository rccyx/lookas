<h1 align="center">Lookas</h1>

<p align="center">
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/Rust-1.70+-black?logo=rust&logoColor=white&style=for-the-badge" alt="Rust"/>
  </a>
  <a href="https://opensource.org/licenses/MIT">
    <img src="https://img.shields.io/badge/License-MIT-blue?logo=open-source-initiative&logoColor=white&style=for-the-badge" alt="License: MIT"/>
  </a>
  <a href="https://crates.io/crates/lookas">
    <img src="https://img.shields.io/crates/v/lookas?color=orange&logo=rust&style=for-the-badge" alt="Crates.io"/>
  </a>
</p>

A high-performance, terminal-based audio spectrum visualizer written in Rust.

## Overview

<img width="1913" height="1080" alt="image" src="https://github.com/user-attachments/assets/89f5cac5-e6c5-48bb-a029-d29172765200" />

## What It Does

Lookas captures **microphone input, system audio, or both**, converts the signal into frequency bands using a mel-scale FFT, and renders it as smooth, physics-driven bars directly in the terminal.

Adaptive gain control and noise gating keep the display clean. A spring-damper animation model gives the bars weight and continuity instead of raw jitter. Rendering is optimized for terminal throughput and stable frame pacing rather than flashy redraw tricks.

The result is a visualizer that feels connected to the sound, not a noisy oscilloscope clone.

## Installation

```bash
cargo install lookas
```

### System Audio (Linux)

For system audio capture, Lookas relies on PulseAudio / PipeWire utilities.

If not installed already, run:

```bash
sudo apt install pulseaudio-utils
```

This works on both PulseAudio and PipeWire setups via `pipewire-pulse`.

## Basic Usage

Run with defaults:

```bash
lookas
```

By default, Lookas will attempt to start with system audio. If that fails, it falls back to microphone input.

## Controls

- `1` – Microphone input
- `2` – System audio (loopback / monitor)
- `3` – Microphone + system mix
- `r` – Restart audio pipeline
- `q` – Quit

## Configuration

Lookas is configured entirely through environment variables.

### Frequency & Resolution

| Variable          | Description                    | Default | Range            |
| ----------------- | ------------------------------ | ------- | ---------------- |
| `LOOKAS_FMIN`     | Minimum frequency (Hz)         | 30.0    | 10.0 – 1000.0    |
| `LOOKAS_FMAX`     | Maximum frequency (Hz)         | 16000.0 | 1000.0 – 24000.0 |
| `LOOKAS_FFT_SIZE` | FFT window size (power of two) | 2048    | 512 – 4096       |

### Performance & Layout

| Variable               | Description            | Default | Range             |
| ---------------------- | ---------------------- | ------- | ----------------- |
| `LOOKAS_TARGET_FPS_MS` | Target frame time (ms) | 16      | 8 – 50            |
| `LOOKAS_MODE`          | Horizontal layout mode | "rows"  | "rows", "columns" |

### Audio Processing

| Variable            | Description                 | Default | Range         |
| ------------------- | --------------------------- | ------- | ------------- |
| `LOOKAS_TAU_SPEC`   | Spectrum smoothing constant | 0.06    | 0.01 – 0.20   |
| `LOOKAS_GATE_DB`    | Noise gate threshold (dB)   | -55.0   | -80.0 – -30.0 |
| `LOOKAS_TILT_ALPHA` | Frequency response tilt     | 0.30    | 0.0 – 1.0     |

### Animation Physics

| Variable          | Description                 | Default | Range        |
| ----------------- | --------------------------- | ------- | ------------ |
| `LOOKAS_FLOW_K`   | Horizontal energy diffusion | 0.18    | 0.0 – 1.0    |
| `LOOKAS_SPR_K`    | Spring stiffness            | 60.0    | 10.0 – 200.0 |
| `LOOKAS_SPR_ZETA` | Spring damping              | 1.0     | 0.1 – 2.0    |

## Example Configurations

High frequency resolution:

```bash
LOOKAS_FFT_SIZE=4096 LOOKAS_FMIN=20 LOOKAS_FMAX=20000 lookas
```

Lower CPU usage:

```bash
LOOKAS_TARGET_FPS_MS=33 LOOKAS_FFT_SIZE=1024 lookas
```

Bass-focused music:

```bash
LOOKAS_FMIN=20 LOOKAS_FMAX=8000 LOOKAS_TILT_ALPHA=0.1 lookas
```

Smoother classical dynamics:

```bash
LOOKAS_FMIN=40 LOOKAS_FMAX=12000 LOOKAS_TAU_SPEC=0.12 lookas
```

## How It Works

Lookas runs a low-latency audio pipeline designed for visual stability first.

Audio is captured from the microphone, system loopback, or both. The signal is windowed with a Hann function to reduce spectral leakage, then transformed via FFT into frequency bins. These bins are remapped onto a mel-scale filterbank so the visualization aligns with human loudness perception rather than linear frequency spacing.

Dynamic range is managed continuously using percentile tracking instead of fixed scaling. A noise gate suppresses background hiss, while frequency tilt prevents low or high bands from dominating the display.

Animation is driven by a spring-damper model rather than raw amplitude changes. Energy diffuses laterally between neighboring bands, producing motion that feels fluid instead of twitchy. All hot-path operations are allocation-free, cache-friendly, and designed to maintain consistent frame pacing.

Rendering uses dense Unicode block characters to achieve smooth gradients without expensive redraws. The terminal is only cleared once per frame, layout is recomputed only when geometry changes, and output is written in large contiguous chunks to avoid flicker.

On modern Linux systems, this yields a stable 60+ FPS experience with audio-to-visual latency low enough to feel immediate.

## License

MIT © [@rccyx](https://rccyx.com)
