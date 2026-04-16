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

<p align="center">
  <a href="https://www.youtube.com/watch?v=lVDFpoCkvh8">
    <img src="./assets/demo.gif" alt="Lookas Demo" width="100%">
  </a>
</p>

## Explainers

<details> <summary><b>What</b></summary>
<br/>

Lookas captures **microphone input, system audio, or both**, converts the signal into frequency bands using a mel-scale FFT, and renders it as smooth, physics-driven bars directly in your terminal.

</details>
<details> <summary><b>Why</b></summary>
<br/>

Since I live rent free in the terminal, that space needs to feel [intentional](https://github.com/rccyx/osyx/commits/main/).

Aesthetics are a direct consequence of the logic. So, Lookas is an attempt to create a connection to the physical world that has actual weight. A way to align those pixels with the biological reality of how we experience sound.

</details>
<details> <summary><b>How</b></summary>
<br/>

It runs a low-latency audio pipeline designed for visual stability first.

Audio is captured from the microphone, system loopback, or both. The signal is windowed with a Hann function to reduce spectral leakage, then transformed via FFT into frequency bins. These bins are remapped onto a mel-scale filterbank so the visualization aligns with human loudness perception rather than linear frequency spacing.

Dynamic range is managed continuously using percentile tracking instead of fixed scaling. A noise gate suppresses background hiss, while frequency tilt prevents low or high bands from dominating the display.

Animation is driven by a spring-damper model rather than raw amplitude changes. Energy diffuses laterally between neighboring bands, which produces a fluid motion, instead of twitchiness.

Rendering uses dense Unicode block characters to achieve smooth gradients without expensive redraws. The terminal is only cleared once per frame, layout is recomputed only when geometry changes, and output is written in large contiguous chunks to avoid flicker.

On modern Linux, this yields a stable 60+ FPS experience with audio-to-visual latency low enough to feel immediate.

</details>

<details>
<summary><strong>Why not use CAVA?</strong></summary>
<br/>
On the surface it may look like "just another CAVA clone/reinvent the wheel situation."

**It’s not.**

They may look similar in static screenshots. But they feel completely different in motion.

Here’s the real difference:

**TL;DR**  
**CAVA** is the battle-tested, Swiss-army-knife visualizer (de facto standard, cross-platform, insanely configurable).

**Lookas** is the opinionated, perception-first tool that deliberately throws away raw linear-FFT twitchiness and replaces it with human-hearing-aligned physics so the bars _feel_ like real sound instead of a nervous digital meter.

### Philosophy

- **CAVA**: Makes pretty, responsive dancing bars that work everywhere with maximum flexibility. No deep claims about biology or physics. It’s aesthetic eye-candy tuned for low CPU and broad compatibility.

- **Lookas**: Fixes the core issue that most visualizers feel disconnected from how humans actually _hear_ sound. Goal = biological/perceptual alignment + physical weight. Mel-scale + spring-damper + lateral energy diffusion = a visualization that has _intentional_ heft instead of jitter.

### Audio Pipeline & Physics Difference

| Aspect              | CAVA (logarithmic FFTW)                    | Lookas (mel-scale + real physics)                            |
| ------------------- | ------------------------------------------ | ------------------------------------------------------------ |
| Frequency mapping   | Logarithmic binning (more bass resolution) | **Mel-scale filterbank** (matches human loudness perception) |
| Windowing           | Hann window                                | Hann window                                                  |
| Dynamic range       | Autosens + manual sensitivity              | **Continuous percentile tracking**                           |
| Noise handling      | Basic noise reduction                      | **Explicit noise gate** (`gate_db`)                          |
| Frequency balance   | Per-bar EQ (configurable)                  | **`tilt_alpha`** (compensates natural high-freq roll-off)    |
| Temporal smoothing  | Quadratic gravity + integral EMA           | **Exponential smoothing** (`tau_spec`)                       |
| Animation model     | Height changes with gravity fall-off       | **Second-order spring-damper system** (`spr_k` + `spr_zeta`) |
| Lateral interaction | None                                       | **Energy diffusion** (`flow_k`) between neighboring bars     |

### Input & Output

**Input**

- CAVA: Extremely flexible (PipeWire, PulseAudio, ALSA loopback, JACK, FIFO/MPD, Sndio, OSS, PortAudio, shared memory, Windows default). You can wire almost anything.

- Lookas: Deliberately minimal: Mic (1), System loopback (2), or Mix (3). Hotkey swap + auto-fallback. Low-latency pipeline tuned for stability, not maximum flexibility. Linux-only (Rust-only for now).

**Output**

- CAVA: Terminal (noncurses/ncurses), **SDL desktop window with GLSL shaders**, raw data (pipe to anything). Multiple orientations, bar widths, gradients, etc.

- Lookas: Terminal-only. Optimized Unicode-block renderer (single clear per frame + contiguous writes -> zero flicker, buttery-smooth gradients).

### Config & User Experience

CAVA: Huge, heavily commented INI file. You can tweak literally everything (bars, sensitivity, cutoffs, EQ, colors, framerate, sleep timer, etc.). Live reload with SIGUSR1/SIGUSR2.

Lookas: **Zero-config by default**. Tiny optional TOML (`~/.config/lookas.toml`) with only the perceptual knobs that actually matter. Environment-variable overrides. Keyboard controls built-in. Ships with sane defaults so it “just works”.

### Performance & Feel

Both run 60+ FPS on modern hardware and are very light on CPU.  
The difference is in the **_feel_**.

CAVA gives you crisp, responsive dancing bars.

Lookas gives you bars that have **mass, momentum, and perceptual weight**, they move like real sound propagating through air.

</details>

## Comparison

Both Lookas ( ↓ ) and CAVA ( ↑ ) are running on default configs

<div align="center">
  <video src="https://github.com/user-attachments/assets/557e377d-6eb3-47dc-ae94-8e507a2337ad" width="100%" controls>
    Your browser does not support the video tag.
  </video>
</div>

<br/>

> [!NOTE]
> Lookas as a standalone FOSS is an extension of [OSYX](https://github.com/rccyx/osyx).

## Installation

Simply run:

```bash
cargo install lookas
```

**You usually don't need to install anything else.**

If your system already has working audio (microphone or system sound),

Lookas will just run.

> [!IMPORTANT]
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

### Config File

The default location is:

- `~/.config/lookas.toml`

You can also point to a file explicitly:

```bash
LOOKAS_CONFIG=/path/to/lookas.toml lookas
```

To create the config file, simply copy-paste this into your terminal

```bash
mkdir -p ~/.config
cat > ~/.config/lookas.toml <<'TOML'
fmin = 30.0
fmax = 16000.0
fft_size = 2048
target_fps_ms = 16
tau_spec = 0.06
gate_db = -55.0
tilt_alpha = 0.30
flow_k = 0.18
spr_k = 60.0
spr_zeta = 1.0
TOML
```

### Options

### Frequency Boundaries

The `fmin` and `fmax` values determine the absolute lowest and highest frequencies rendered by the visualizer, measured in Hertz.

The `fmin` sets the bass cutoff, mapping to the lowest perceptible rumbles, with a practical minimum of around 20.0 and a maximum depending on your desired low-end focus. Setting it to 30.0 ignores inaudible sub-bass mud.

The `fmax` sets the treble ceiling, dictating the highest pitch the visualizer will display.

While human hearing maxes out around 20000.0, most meaningful audio content tapers off sooner, making 16000.0 a clean default.

> [!WARNING]
> Pushing `fmax` too high will result in empty bars on the right side of the spectrum if your audio source lacks high-frequency energy.

### Spectrum Resolution

The `fft_size` dictates the number of samples per Fast Fourier Transform window. This process relies on the canonical algorithm detailed in the [Cooley & Tukey (1965) paper.](https://www.ams.org/mcom/1965-19-090/S0025-5718-1965-0178586-1/S0025-5718-1965-0178586-1.pdf) This value must strictly be a power of two. The absolute minimum is 512, which gives an incredibly fast response time but terribly blurred frequency detail.

The maximum practical value is 8192, yielding extreme detail at the cost of high CPU utilization and noticeable input latency. The default of 2048 sits at the optimal intersection of performance, minimal latency, and distinct frequency separation.

### Frame Pacing

The `target_fps_ms` value establishes the render loop's resting heartbeat, defining the target time per frame in milliseconds.

Setting this to 16 forces the visualizer to update roughly 60 times a second. You can drop this down to 8 for an ultra-fluid 120 FPS experience if your terminal emulator can actually handle the throughput, or raise it to 33 to cap it at a relaxed 30 FPS, drastically cutting down on CPU overhead.

> [!WARNING]
> Going above 50 milliseconds will make the visualizer feel noticeably choppy and disconnected from the audio.

### Spectrum Smoothing

The `tau_spec` controls the time constant (τ), governing how rapidly the raw frequency data responds versus how much it relies on historical smoothing (read the underlying math [here.](https://ocw.mit.edu/courses/2-004-systems-modeling-and-control-ii-fall-2007/26a1e7459044ff2652c63c7c98138e4b_lecture06.pdf))

A minimal value close to 0.01 makes the visualizer snappy and instantly reactive, but also highly jittery. A maximal value approaching 0.5 turns the bars sluggish and heavy, creating a long fade-out trail.

The default of 0.06 provides a polished decay that tracks fast transients without inducing visual fatigue.

### Noise Gate

The `gate_db` defines the absolute silence threshold in decibels. This dictates the point at which background noise is entirely suppressed. If your environment or microphone has a persistent hiss, you would raise this to a less negative number, like -30.0, which enforces strict silence and only allows distinctly loud audio to register.

If quiet nuances in your music are failing to appear on the visualizer, you drop this to a more negative extreme, something like -80.0, making the application highly sensitive. The 55.0 default is tuned to reject standard line noise while capturing quiet room ambiance.

### Frequency Balance

The `tilt_alpha` compensates for the natural roll-off of high frequencies in most audio mixes, so that the treble bars aren't perpetually dwarfed by the bass.

This value is strictly between 0.0 and 1.0.

At a minimum of 0.0, no spectral compensation is applied, which results in a physically accurate but visually lopsided spectrum where the left side dominates.

Pushing it towards the maximum of 1.0 violently amplifies the high end.

The 0.30 sweet spot applies just enough tilt to create an evenly distributed visual landscape across all frequency bands.

### Motion Coupling

The `flow_k` parameter governs the lateral diffusion of energy, bleeding a calculated fraction of one bar's amplitude into its immediate neighbors.

It's restricted between 0.0 and 1.0.

Setting this to the absolute minimum of 0.0 severs all connections, meaning every single frequency band moves independently like a raw digital meter.

Approaching the maximum of 1.0 causes so much bleed that the visualizer becomes an undefined, solid block of moving color.

The 0.18 default injects just enough organic cohesion so the bars move together like a continuous fluid wave.

### Spring-Damper Animation

The `spr_k` and `spr_zeta` variables completely dictate the physical weight and momentum of the bars using a second-order system model.

The `spr_k` represents spring stiffness, ranging from a lethargic minimum of 10.0 to a violently rigid maximum of 200.0, determining the raw force pulling the bar to its target height.

The `spr_zeta` represents the damping ratio (ζ) (for a deep dive, read [this.](https://ocw.mit.edu/courses/2-003-modeling-dynamics-and-control-i-spring-2005/57d44d83366ec969c16208c8fac3982d_notesinstalment2.pdf))

A zeta value below 1.0 creates an underdamped system, causing the bars to bounce past their target before settling. A zeta value exactly at 1.0, the default, achieves critical damping, meaning the bar snaps to the target instantly with zero overshoot. Any value above 1.0 makes the system overdamped, resulting in a heavy, delayed crawl to the peak.

### Environment variable overrides

Every config value can be overridden temporarily using environment variables.

Examples:

```bash
LOOKAS_FFT_SIZE=4096 lookas
LOOKAS_GATE_DB=-60 lookas
LOOKAS_TARGET_FPS_MS=33 lookas
```

## Contributing & Issues

Please read [this](.github/CONTRIBUTING.md)

## License

MIT © [@rccyx](https://rccyx.com)
