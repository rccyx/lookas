
<h1 align="center">Lookas</h1>
<p align="center">
  <a href="https://github.com/rccyx/lookas/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/rccyx/lookas/ci.yml?style=for-the-badge&color=black&labelColor=111111&logo=githubactions&logoColor=white" alt="CI Status"/>
  </a>
  <a href="https://www.kernel.org">
    <img src="https://img.shields.io/badge/Platform-Linux-black?logo=linux&logoColor=white&style=for-the-badge" alt="Platform: Linux" />
  </a>
  <a href="https://crates.io/crates/lookas">
    <img src="https://img.shields.io/crates/v/lookas?color=black&logo=rust&logoColor=white&style=for-the-badge" alt="Crates.io" />
  </a>
  <a href="https://opensource.org/licenses/MIT">
    <img src="https://img.shields.io/badge/License-MIT-black?logo=open-source-initiative&logoColor=white&style=for-the-badge" alt="License: MIT" />
  </a>
</p>

<p align="center">
  <strong>Perception aligned terminal-based audio spectrum visualizer.</strong>
</p>

## Overview

### Ϟ

<p align="center">
  <a href="https://www.youtube.com/watch?v=lVDFpoCkvh8">
    <img src="./assets/demo.gif" alt="Lookas Demo" width="100%">
  </a>
</p>

### Ϝ

<div align="center">
  <video title"demo" src="https://github.com/user-attachments/assets/d46f74ad-77d3-4932-b02e-ff91df87d26b" width="100%" controls>
    Your browser does not support the video tag.
  </video>
</div>


**What it does:**

Lookas captures **microphone input, system audio, or both**, converts the signal into frequency bands using a mel-scale FFT, and renders it as smooth, physics-driven bars directly in the terminal.

**How it works:**

Audio is captured from the microphone, system loopback, or both. The signal is windowed with a Hann function to reduce spectral leakage, then transformed via FFT into frequency bins. These bins are then remapped onto a mel-scale filterbank so the visualization aligns with human loudness perception rather than linear frequency spacing.

Frequency balance is handled by [A-weighting](http://cdn.standards.iteh.ai/samples/10880/e138f40fd9e84af8906910f4b6d8a4df/IEC-61672-2-2003.pdf), which models the human ear's actual sensitivity curve across frequency. 

Dynamic range is managed continuously using percentile tracking instead of fixed scaling, with a noise gate suppressing background hiss.

The spectrum uses asymmetric smoothing: energy attacks instantly so every transient is captured at full resolution, while the decay tail uses an exponential moving average for a smooth release.

Animation is basically a spring damper model. Energy diffuses laterally between neighboring bands, which produces a fluid motion instead of twitchiness.

To have it rendered at 60+ FPS while keeping perf high: Unicode block characters to achieve smooth gradients without expensive redraws. The terminal is only cleared once per frame, layout is recomputed only when geometry changes, and output is written in large contiguous chunks to avoid flicker.

## CAVA Comparison

You're probably familiar with CAVA and similar tools.

<details><summary><b>How both differ?</b></summary>

<br/>

| Feature         | CAVA              | Lookas                   |
| :-------------- | :---------------- | :----------------------- |
| **Mapping**     | Logarithmic       | Mel-scale                |
| **Dynamics**    | Autosens / Manual | Percentile tracking      |
| **Noise**       | Basic reduction   | Explicit noise gate      |
| **Balance**     | Per-bar EQ        | A-weighting              |
| **Smoothing**   | Quadratic gravity | Asymmetric EMA           |
| **Physics**     | Gravity fall-off  | Spring-damper system     |
| **Interaction** | None              | Lateral energy diffusion |

</details>
<details><summary><b>Visual comparison</b></summary>

Both Lookas ( ← ) and CAVA ( → ) are running on default configs

<div align="center">
  <video src="https://github.com/user-attachments/assets/33b5d98b-a6e7-4d10-b093-77abfb25f255" width="100%" controls>
    Your browser does not support the video tag.
  </video>
</div>

</details>

## Installation

Simply run:

```bash
cargo install lookas
```

If your system already has working audio (microphone or system sound), it will just run.

> [!IMPORTANT]
> On very minimal Linux installs, you might be missing a couple of audio packages.
> If the program fails to start or can't capture audio, run:
>
> ```bash
> sudo apt install -y libasound2-dev pulseaudio-utils
> ```
>
> This works for both PulseAudio and PipeWire systems (via `pipewire-pulse`).

## Basic Usage

Just run:

```bash
lookas
```

It will attempt to start with system audio. If system audio isn't available, it automatically falls back to microphone input.

## Controls

- `1` – Microphone input
- `2` – System audio (loopback / monitor)
- `3` – Microphone + system mix
- `r` – Restart audio pipeline
- `q` – Quit

## Configuration (Optional)

Zero-config by default. It ships with sensible defaults and works out of the box. Configuration exists purely for customization.

If you want to tweak how it looks or reacts to sound, it reads a single TOML file and applies environment variables on top.

Precedence is:

Environment variables > config file > defaults

### Config File

Is here:

`~/.config/lookas.toml`

To create the file, simply copy-paste this into your terminal:

```bash
mkdir -p ~/.config
cat > ~/.config/lookas.toml <<'TOML'
fmin = 30.0
fmax = 16000.0
fft_size = 2048
target_fps_ms = 16
tau_spec = 0.06
gate_db = -55.0
flow_k = 0.18
spr_k = 60.0
spr_zeta = 1.0
TOML
```

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

The `tau_spec` controls the time constant (τ) governing how quickly the bars decay after a transient (read the underlying math [here.](https://ocw.mit.edu/courses/2-004-systems-modeling-and-control-ii-fall-2007/26a1e7459044ff2652c63c7c98138e4b_lecture06.pdf))

Attacks are always instant regardless of this value. `tau_spec` only affects the release: how long the energy lingers before fading. A minimal value close to 0.01 makes the decay almost immediate, while a value approaching 0.20 creates a long, heavy fade-out trail.

The default of 0.06 provides a polished decay that tracks fast transients without inducing visual fatigue.

### Noise Gate

The `gate_db` defines the absolute silence threshold in decibels. This dictates the point at which background noise is entirely suppressed. If your environment or microphone has a persistent hiss, you would raise this to a less negative number, like -30.0, which enforces strict silence and only allows distinctly loud audio to register.

If quiet nuances in your music are failing to appear on the visualizer, you drop this to a more negative extreme, something like -80.0, making the application highly sensitive. The 55.0 default is tuned to reject standard line noise while capturing quiet room ambiance.

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

## License

MIT © [@rccyx](https://rccyx.com)
