<h1 align="center">Lookas</h1>

<p align="center">
  <a href="https://www.kernel.org">
    <img src="https://img.shields.io/badge/Platform-Linux-black?logo=linux&logoColor=white&style=for-the-badge" alt="Platform: Linux"/>
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

The result is a visualizer that feels naturally connected to the sound, not a noisy oscilloscope clone.

## Installation

Simply run:

```bash
cargo install lookas
```

**You usually don't need to install anything else.**

If your system already has working audio (microphone or system sound),

Lookas will just run.

> [!NOTE]
> On very minimal Linux installs, you might be missing a couple of audio packages.
> If Lookas fails to start or can’t capture audio, install:
>
> ```bash
> sudo apt update
> sudo apt install -y libasound2-dev pulseaudio-utils
> ```
>
> This works for both PulseAudio and PipeWire systems (via `pipewire-pulse`).

## Basic Usage

Lookas is **zero-config by default**.

Just run:

```bash
lookas
```

It will attempt to start with system audio.  
If system audio isn’t available, it automatically falls back to microphone input.

## Controls

- `1` – Microphone input
- `2` – System audio (loopback / monitor)
- `3` – Microphone + system mix
- `r` – Restart audio pipeline
- `q` – Quit

## Configuration (Optional)

You do **not** need a config file to use this.

It ships with sensible defaults and works out of the box.  
Configuration exists purely for customization.

If you want to tweak how it looks or reacts to sound, it can read a single TOML file and then apply environment variables on top.

Precedence is:

environment variables > config file > defaults

### Config file location

The default location is:

- `~/.config/lookas.toml`

You can also point to a file explicitly:

```bash
LOOKAS_CONFIG=/path/to/lookas.toml lookas
```

### Creating a config file

Copy-paste into your terminal:

```bash
mkdir -p ~/.configs
cat > ~/.config/lookas.toml <<'TOML'
# Lookas configuration file @https://github.com/rccyx/lookas
#
# This file is optional.
# If it does not exist, Lookas runs with built-in defaults.
#
# Helpful references if you want to understand the terms:
# - FFT (Fast Fourier Transform): https://www.nti-audio.com/en/support/know-how/fast-fourier-transform-fft
# - Windowing & spectral leakage: https://brianmcfee.net/dstbook-site/content/ch06-dft-properties/Leakage.html
# - Mel scale (perceptual frequency spacing): https://www.fon.hum.uva.nl/praat/manual/MelSpectrogram.html
# - Decibels (dB): https://en.wikipedia.org/wiki/Decibel
# - Damping ratio (ζ): https://en.wikipedia.org/wiki/Damping
#
# Tuning tips:
# - Feels slow or heavy → lower fft_size or raise target_fps_ms
# - Feels twitchy → raise tau_spec or spr_zeta
# - Always active in silence → raise gate_db (less negative)
# - Never reacts → lower gate_db (more negative)

# Frequency range in Hz
# fmin: 10.0 .. 1000.0   (typical: 20..60)
# fmax: 1000.0 .. 24000.0 (typical: 12000..20000)
fmin = 30.0
fmax = 16000.0

# FFT window size (power of two)
# Range: 512 .. 4096
# Larger = more detail, more CPU, slightly more latency
fft_size = 2048

# Target frame time in milliseconds
# 16 ≈ 60 FPS, 33 ≈ 30 FPS
# Range: 8 .. 50
target_fps_ms = 16

# Spectrum smoothing time constant (seconds)
# Range: 0.01 .. 0.20
tau_spec = 0.06

# Noise gate threshold (dB)
# Range: -80.0 .. -30.0
gate_db = -55.0

# Frequency-response tilt (0..1)
tilt_alpha = 0.30

# Horizontal energy diffusion (0..1)
flow_k = 0.18

# Spring stiffness
# Range: 10.0 .. 200.0
spr_k = 60.0

# Spring damping ratio ζ
# Range: 0.1 .. 2.0
spr_zeta = 1.0
TOML
```

### Environment variable overrides

Every config value can be overridden temporarily using environment variables.

Examples:

```bash
LOOKAS_FFT_SIZE=4096 lookas
LOOKAS_GATE_DB=-60 lookas
LOOKAS_TARGET_FPS_MS=33 lookas
```

## How It Works

Lookas runs a low-latency audio pipeline designed for visual stability first.

Audio is captured from the microphone, system loopback, or both. The signal is windowed with a Hann function to reduce spectral leakage, then transformed via FFT into frequency bins. These bins are remapped onto a mel-scale filterbank so the visualization aligns with human loudness perception rather than linear frequency spacing.

Dynamic range is managed continuously using percentile tracking instead of fixed scaling. A noise gate suppresses background hiss, while frequency tilt prevents low or high bands from dominating the display.

Animation is driven by a spring-damper model rather than raw amplitude changes. Energy diffuses laterally between neighboring bands, producing motion that feels fluid instead of twitchy.

Rendering uses dense Unicode block characters to achieve smooth gradients without expensive redraws. The terminal is only cleared once per frame, layout is recomputed only when geometry changes, and output is written in large contiguous chunks to avoid flicker.

On modern Linux systems, this yields a stable 60+ FPS experience with audio-to-visual latency low enough to feel immediate.

## License

MIT © [@rccyx](https://rccyx.com)
