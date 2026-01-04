use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, SizedSample, StreamConfig};
use rustfft::num_traits::ToPrimitive;

use crate::buffer::SharedBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AudioMode {
    Mic,
    System,
    Both,
}

pub struct CaptureInfo {
    pub label: String,
    pub sample_rate: u32,
}

pub struct AudioController {
    mode: AudioMode,
    mic: Option<MicHandle>,
    sys: Option<SystemHandle>,
    info: CaptureInfo,
}

impl Default for AudioController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioController {
    pub fn new() -> Self {
        Self {
            mode: AudioMode::Mic,
            mic: None,
            sys: None,
            info: CaptureInfo {
                label: "mic".into(),
                sample_rate: 48_000,
            },
        }
    }

    pub fn mode(&self) -> AudioMode {
        self.mode
    }

    pub fn info(&self) -> &CaptureInfo {
        &self.info
    }

    pub fn start(
        &mut self,
        mode: AudioMode,
        mic_shared: Arc<Mutex<SharedBuf>>,
        sys_shared: Arc<Mutex<SharedBuf>>,
    ) -> Result<()> {
        self.stop();

        match mode {
            AudioMode::Mic => {
                let mic = start_mic(mic_shared)?;
                self.info = CaptureInfo {
                    label: mic.label.clone(),
                    sample_rate: mic.sample_rate,
                };
                self.mic = Some(mic);
                self.mode = mode;
                Ok(())
            }
            AudioMode::System => {
                let sys = start_system(sys_shared, 48_000)?;
                self.info = CaptureInfo {
                    label: sys.label.clone(),
                    sample_rate: sys.sample_rate,
                };
                self.sys = Some(sys);
                self.mode = mode;
                Ok(())
            }
            AudioMode::Both => {
                let mic = start_mic(mic_shared)?;
                let sys = start_system(sys_shared, mic.sample_rate)?;
                self.info = CaptureInfo {
                    label: format!("{} + {}", mic.label, sys.label),
                    sample_rate: mic.sample_rate,
                };
                self.mic = Some(mic);
                self.sys = Some(sys);
                self.mode = mode;
                Ok(())
            }
        }
    }

    pub fn reset(
        &mut self,
        mic_shared: Arc<Mutex<SharedBuf>>,
        sys_shared: Arc<Mutex<SharedBuf>>,
    ) -> Result<()> {
        let mode = self.mode;
        self.start(mode, mic_shared, sys_shared)
    }

    pub fn stop(&mut self) {
        self.sys.take();
        self.mic.take();
    }
}

pub fn pick_input_device() -> Result<Device> {
    let host = cpal::default_host();

    if let Ok(filter) = std::env::var("LOOKAS_DEVICE") {
        let filter = filter.to_lowercase();
        if let Ok(devices) = host.input_devices() {
            for dev in devices {
                if let Ok(name) = dev.name() {
                    if name.to_lowercase().contains(&filter) {
                        return Ok(dev);
                    }
                }
            }
        }
    }

    host.default_input_device()
        .context("No default input device")
}

pub fn best_config_for(device: &Device) -> Result<StreamConfig> {
    let mut cfg = device.default_input_config()?.config();
    cfg.sample_rate.0 = cfg.sample_rate.0.clamp(44_100, 48_000);
    Ok(cfg)
}

pub fn build_stream<T>(
    device: Device,
    cfg: StreamConfig,
    shared: Arc<Mutex<SharedBuf>>,
) -> Result<cpal::Stream>
where
    T: Sample + SizedSample + ToPrimitive,
{
    let ch = cfg.channels as usize;
    let err_fn = |_| {};

    let stream = device.build_input_stream(
        &cfg,
        move |data: &[T], _| {
            if let Ok(mut buf) = shared.try_lock() {
                for frame in data.chunks_exact(ch) {
                    let mut acc = 0.0f32;
                    for &s in frame {
                        acc += s.to_f32().unwrap_or(0.0);
                    }
                    buf.push(acc / ch as f32);
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

pub struct MicHandle {
    _stream: cpal::Stream,
    label: String,
    sample_rate: u32,
}

fn start_mic(shared: Arc<Mutex<SharedBuf>>) -> Result<MicHandle> {
    let device = pick_input_device()?;
    let label = device.name().unwrap_or_else(|_| "mic".into());
    let cfg = best_config_for(&device)?;
    let sample_rate = cfg.sample_rate.0;

    let stream = match device.default_input_config()?.sample_format()
    {
        SampleFormat::F32 => {
            build_stream::<f32>(device, cfg.clone(), shared)?
        }
        SampleFormat::I16 => {
            build_stream::<i16>(device, cfg.clone(), shared)?
        }
        SampleFormat::U16 => {
            build_stream::<u16>(device, cfg.clone(), shared)?
        }
        _ => anyhow::bail!("Unsupported sample format"),
    };

    stream.play()?;

    Ok(MicHandle {
        _stream: stream,
        label,
        sample_rate,
    })
}

pub struct SystemHandle {
    #[cfg(target_os = "linux")]
    child: std::process::Child,
    #[cfg(target_os = "linux")]
    join: Option<std::thread::JoinHandle<()>>,
    label: String,
    sample_rate: u32,
}

#[cfg(target_os = "linux")]
fn cmd_out(mut cmd: Command) -> Result<String> {
    let out = cmd.output().context("failed to run command")?;
    if !out.status.success() {
        anyhow::bail!("command failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(target_os = "linux")]
fn pactl(args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("pactl");
    cmd.args(args);
    cmd_out(cmd).context(
        "pactl failed (install pulseaudio-utils, and ensure pipewire-pulse or pulseaudio is running)",
    )
}

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct SourceInfo {
    name: String,
    channels: usize,
    rate: u32,
    state: String,
}

#[cfg(target_os = "linux")]
fn pulse_sources() -> Result<Vec<SourceInfo>> {
    let s = pactl(&["list", "short", "sources"])?;
    let mut out = Vec::new();

    for line in s.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // typical:
        // id name driver fmt 4ch 48000Hz STATE
        if parts.len() < 7 {
            continue;
        }

        let name = parts[1].to_string();
        let ch_tok = parts[4];
        let rate_tok = parts[5];
        let state = parts[6].to_string();

        let channels = ch_tok
            .strip_suffix("ch")
            .and_then(|x| x.parse().ok())
            .unwrap_or(2);
        let rate = rate_tok
            .strip_suffix("Hz")
            .and_then(|x| x.parse().ok())
            .unwrap_or(48_000);

        out.push(SourceInfo {
            name,
            channels,
            rate,
            state,
        });
    }

    Ok(out)
}

#[cfg(target_os = "linux")]
fn resolve_monitor_source() -> Result<SourceInfo> {
    let sources = pulse_sources()?;

    if let Ok(want) = std::env::var("LOOKAS_SYS_DEVICE") {
        let w = want.to_lowercase();
        if let Some(hit) = sources
            .iter()
            .find(|s| s.name.to_lowercase().contains(&w))
        {
            return Ok(hit.clone());
        }
        anyhow::bail!("LOOKAS_SYS_DEVICE was set but no pulse source matched it");
    }

    // prefer a running monitor, because default sink can be wrong on multi-device setups
    let mut best: Option<SourceInfo> = None;
    let mut best_score: i32 = -1;

    for s in sources.iter().filter(|s| s.name.contains(".monitor")) {
        let mut score = 0;
        if s.state == "RUNNING" {
            score += 200;
        }
        score += 100;
        if score > best_score {
            best_score = score;
            best = Some(s.clone());
        }
    }

    if let Some(hit) = best {
        return Ok(hit);
    }

    // fallback: default sink monitor if no monitor was picked above
    if let Ok(sink) = pactl(&["get-default-sink"]) {
        if !sink.is_empty() {
            let mon = format!("{}.monitor", sink);
            let sources = pulse_sources()?;
            if let Some(hit) =
                sources.into_iter().find(|s| s.name == mon)
            {
                return Ok(hit);
            }
        }
    }

    anyhow::bail!("no monitor source found (no .monitor sources in pactl list short sources)")
}

#[cfg(target_os = "linux")]
fn start_system(
    shared: Arc<Mutex<SharedBuf>>,
    rate: u32,
) -> Result<SystemHandle> {
    use std::io::Read;
    use std::thread;

    let src = resolve_monitor_source()?;
    let channels = src.channels.max(1);
    let use_rate = src.rate.clamp(8_000, 192_000).max(rate);

    let mut child = Command::new("parec")
        .args([
            "--device",
            &src.name,
            "--format=float32le",
            "--rate",
            &use_rate.to_string(),
            "--channels",
            &channels.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!("failed to spawn parec on {}", src.name)
        })?;

    let mut stdout =
        child.stdout.take().context("parec stdout missing")?;

    let join = thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        let mut carry: Vec<u8> = Vec::with_capacity(32 * 1024);
        let frame_bytes = channels * 4;

        loop {
            let n = match stdout.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };

            carry.extend_from_slice(&buf[..n]);

            let frames = carry.len() / frame_bytes;
            if frames == 0 {
                continue;
            }

            let take = frames * frame_bytes;

            if let Ok(mut ring) = shared.try_lock() {
                for f in 0..frames {
                    let base = f * frame_bytes;
                    let mut acc = 0.0f32;
                    for c in 0..channels {
                        let off = base + c * 4;
                        let x = f32::from_le_bytes([
                            carry[off],
                            carry[off + 1],
                            carry[off + 2],
                            carry[off + 3],
                        ]);
                        acc += x;
                    }
                    ring.push(acc / channels as f32);
                }
            }

            carry.drain(..take);
        }
    });

    Ok(SystemHandle {
        child,
        join: Some(join),
        label: format!("system:{} ({}ch)", src.name, channels),
        sample_rate: use_rate,
    })
}

#[cfg(not(target_os = "linux"))]
fn start_system(
    _: Arc<Mutex<SharedBuf>>,
    _: u32,
) -> Result<SystemHandle> {
    anyhow::bail!("system capture is linux-only in this build")
}

impl Drop for SystemHandle {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some(j) = self.join.take() {
                let _ = j.join();
            }
        }
    }
}
