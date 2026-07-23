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
  <a href="https://www.youtube.com/watch?v=lVDFpoCkvh8">
    <img src="./assets/demo.gif" alt="Lookas Demo" width="100%">
  </a>
</p>

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

Updated the full configuration section, including `color`, and corrected the existing mismatches such as `target_fps_ms` → `frame_ms`, the FFT range, and the gate default.

## Configuration (Optional)

This works without a config file. Configuration exists only for changing how the visualizer looks and reacts to sound.

It reads:

`~/.config/lookas.toml`

To create the file:

```bash
mkdir -p ~/.config
cat > ~/.config/lookas.toml <<'TOML'
color = "#7CCEA7"

fmin = 30.0
fmax = 16000.0
frame_ms = 16
fft_size = 2048
tau_spec = 0.06
gate_db = -65.0
flow_k = 0.18
spr_k = 60.0
spr_zeta = 1.0
TOML
```

> [!NOTE]
> Every setting is optional. Any omitted value uses the built-in default.

### Color

The `color` value controls the foreground color used to render the bouncing bars.

It accepts a 6 digit RGB hex color with or without the leading `#`:

```toml
color = "#7CCEA7"
```

```toml
color = "7CCEA7"
```

The default is white:

```toml
color = "#FFFFFF"
```

### Frequency Boundaries

The `fmin` and `fmax` values determine the lowest and highest frequencies rendered by the visualizer, measured in Hertz.

`fmin` defaults to `30.0` and is restricted to `10.0` through `1000.0`.

`fmax` defaults to `16000.0` and is restricted to `1000.0` through `24000.0`.

If `fmin` is equal to or greater than `fmax`, Lookas restores both values to their defaults.

> [!WARNING]
> Pushing `fmax` too high can leave empty bars on the right side of the spectrum when the audio source contains little high-frequency energy.

### Spectrum Resolution

The `fft_size` value controls the number of samples processed by each Fast Fourier Transform window.

It defaults to `2048` and is restricted to `512` through `4096`.

Lower values react faster but provide less frequency detail. Higher values provide finer separation at the cost of additional latency and processing work.

### Frame Pacing

The `frame_ms` value controls the target duration of each rendered frame in milliseconds.

It defaults to `16`, which targets roughly 60 frames per second.

The accepted range is `8` through `50`. Lower values render more frequently and require more terminal throughput. Higher values reduce CPU use but make the animation less responsive.

### Spectrum Smoothing

The `tau_spec` value controls how quickly spectrum energy decays after a transient.

Attacks remain immediate. This value only affects the release.

It defaults to `0.06` and is restricted to `0.01` through `0.20`. Lower values decay faster. Higher values produce a longer visual tail.

### Noise Gate

The `gate_db` value controls the silence threshold in decibels.

It defaults to `-65.0` and is restricted to `-80.0` through `-30.0`.

More negative values make Lookas more sensitive to quiet audio. Less negative values suppress more background noise.

### Motion Coupling

The `flow_k` value controls how strongly energy diffuses between neighboring bars.

It defaults to `0.18` and is restricted to `0.0` through `1.0`.

A value of `0.0` makes every frequency band move independently. Higher values make neighboring bars move more like a continuous fluid surface.

### Spring-Damper Animation

The `spr_k` and `spr_zeta` values control the spring model used to animate the bars.

`spr_k` controls spring stiffness. It defaults to `60.0` and is restricted to `10.0` through `200.0`.

`spr_zeta` controls damping. It defaults to `1.0` and is restricted to `0.1` through `2.0`.

Values below `1.0` allow overshoot and bounce. A value of `1.0` is critically damped. Values above `1.0` produce a slower, heavier response.

## License

MIT © [@rccyx](https://rccyx.com)
