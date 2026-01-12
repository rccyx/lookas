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
<p align="center">
  Perception aligned terminal-based audio spectrum visualizer.
</p>

## Overview

### Immersive

<img width="1913" height="1080" alt="image" src="https://github.com/user-attachments/assets/89f5cac5-e6c5-48bb-a029-d29172765200" />

### Integrated

<img width="1920" height="1079" alt="image" src="https://github.com/user-attachments/assets/494c8b20-a512-4273-941b-89095e35e902" />

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
> If Lookas fails to start or can’t capture audio, run:
>
> ```bash
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

You do **not** need any configs to use this.

It ships with sensible defaults and works out of the box.  
Configuration exists purely for customization.

If you want to tweak how it looks or reacts to sound, it can read a single TOML file and then apply environment variables on top.

Precedence is:

Environment variables > config file > defaults

### Config file

The default location is:

- `~/.config/lookas.toml`

You can also point to a file explicitly:

```bash
LOOKAS_CONFIG=/path/to/lookas.toml lookas
```

<details> <summary><b>To create the config file, simply copy-paste into your terminal</b></summary>

```bash
mkdir -p ~/.configs
cat > ~/.config/lookas.toml <<'TOML'
# Lookas configuration file
# source: https://github.com/rccyx/lookas
#
# This file is optional. If missing, lookas runs with built-in defaults.
#
# --------------------------------------------------------------------
# Core concepts:
#
# FFT = Fast Fourier Transform.
# converts time-domain audio into frequency bins.
# canonical paper: Cooley & Tukey (1965)
# https://www.ams.org/mcom/1965-19-090/S0025-5718-1965-0178586-1/S0025-5718-1965-0178586-1.pdf
#
# τ (tau) = time constant.
# controls how quickly a value responds vs smooths.
# MIT OCW (control systems):
# https://ocw.mit.edu/courses/2-004-systems-modeling-and-control-ii-fall-2007/26a1e7459044ff2652c63c7c98138e4b_lecture06.pdf
#
# ζ (zeta) = damping ratio for a 2nd-order system.
# controls overshoot vs settle behavior.
# MIT OCW:
# https://ocw.mit.edu/courses/2-003-modeling-dynamics-and-control-i-spring-2005/57d44d83366ec969c16208c8fac3982d_notesinstalment2.pdf
#
# --------------------------------------------------------------------
# Quick fixes
#
# bars move in silence      -> raise gate_db (less negative)
# quiet audio not visible   -> lower gate_db (more negative)
# jittery / nervous motion  -> raise tau_spec or spr_zeta
# laggy / heavy feel        -> lower tau_spec or fft_size
# high cpu usage            -> raise target_fps_ms or lower fft_size


# ====================================================================
# frequency range (hz)
# ====================================================================

# minimum frequency shown (bass)
fmin = 30.0

# maximum frequency shown (treble)
fmax = 16000.0


# ====================================================================
# spectrum resolution (fft)
# ====================================================================

# fft_size = samples per FFT window (power of two)
#
# lower  -> faster response, less detail
# higher -> more detail, more cpu, slightly more latency
#
# common values: 1024, 2048, 4096
fft_size = 2048


# ====================================================================
# frame pacing
# ====================================================================

# target time per frame (ms)
# 16 ≈ 60 fps, 33 ≈ 30 fps
target_fps_ms = 16


# ====================================================================
# spectrum smoothing (τ)
# ====================================================================

# tau_spec = spectrum smoothing time constant (seconds)
#
# lower  -> snappier, more jitter
# higher -> smoother, heavier feel
tau_spec = 0.06


# ====================================================================
# noise gate
# ====================================================================

# gate_db = silence threshold
#
# less negative -> stricter silence
# more negative -> more sensitive
gate_db = -55.0


# ====================================================================
# frequency balance + motion coupling
# ====================================================================

# tilt_alpha = frequency balance compensation (0..1)
tilt_alpha = 0.30

# flow_k = sideways energy diffusion (0..1)
flow_k = 0.18


# ====================================================================
# spring-damper animation (k, ζ)
# ====================================================================

# spr_k = spring stiffness
spr_k = 60.0

# spr_zeta = damping ratio ζ
#
# < 1.0  -> bouncy
# = 1.0  -> fast settle, no overshoot
# > 1.0  -> heavy / slow settle
spr_zeta = 1.0
TOML
```

</details>

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
