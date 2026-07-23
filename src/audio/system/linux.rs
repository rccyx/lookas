use anyhow::{Context, Result};
use std::fmt;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::buffer::SharedBuf;

pub struct SystemHandle {
    pub(in crate::audio) label: String,
    pub(in crate::audio) sample_rate: u32,
    child: std::process::Child,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for SystemHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

struct ParecConfig {
    device: String,
    rate: u32,
    channels: usize,
    latency_ms: u32,
    process_ms: u32,
}

#[derive(Debug)]
enum CommandError {
    MissingCommand {
        cmd: &'static str,
        source: std::io::Error,
    },
    CommandFailed {
        cmd: &'static str,
        args: String,
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
}

pub fn start_system(
    shared: Arc<Mutex<SharedBuf>>,
    rate: u32,
) -> Result<SystemHandle> {
    let src = resolve_monitor_source()?;
    let pcfg = ParecConfig {
        device: src.name.clone(),
        rate,
        channels: src.channels.max(1),
        latency_ms: 15,
        process_ms: 5,
    };

    let mut child = spawn_parec(&pcfg)?;
    let stdout =
        child.stdout.take().context("parec stdout missing")?;
    let join = thread::spawn(move || {
        read_parec_loop(stdout, &shared, pcfg.channels);
    });

    Ok(SystemHandle {
        label: format!(
            "system:{} ({}ch, lat={}ms proc={}ms)",
            src.name, pcfg.channels, pcfg.latency_ms, pcfg.process_ms
        ),
        sample_rate: pcfg.rate,
        child,
        join: Some(join),
    })
}

fn spawn_parec(cfg: &ParecConfig) -> Result<std::process::Child> {
    Command::new("parec")
        .args([
            "--device",
            &cfg.device,
            "--format=float32le",
            &format!("--latency-msec={}", cfg.latency_ms),
            &format!("--process-time-msec={}", cfg.process_ms),
            "--rate",
            &cfg.rate.to_string(),
            "--channels",
            &cfg.channels.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!("failed to spawn parec on {}", cfg.device)
        })
}

#[allow(clippy::arithmetic_side_effects)]
fn read_parec_loop(
    mut stdout: std::process::ChildStdout,
    shared: &Arc<Mutex<SharedBuf>>,
    channels: usize,
) {
    let mut raw = [0u8; 16 * 1024];
    let mut carry: Vec<u8> = Vec::with_capacity(32 * 1024);
    let frame_bytes = channels * 4;

    loop {
        let n = match stdout.read(&mut raw) {
            Ok(0) | Err(_) => break,
            Ok(v) => v,
        };

        if let Some(slice) = raw.get(..n) {
            carry.extend_from_slice(slice);
        }

        let frames = carry.len() / frame_bytes;
        if frames == 0 {
            continue;
        }
        let take = frames * frame_bytes;

        if let Ok(mut ring) = shared.try_lock() {
            push_frames(&carry, frames, channels, &mut ring);
        }

        carry.drain(..take);
    }
}

#[allow(clippy::arithmetic_side_effects, clippy::cast_precision_loss)]
fn push_frames(
    carry: &[u8],
    frames: usize,
    channels: usize,
    ring: &mut SharedBuf,
) {
    let frame_bytes = channels * 4;
    for f in 0..frames {
        let base = f * frame_bytes;
        let mut acc = 0.0f32;
        for c in 0..channels {
            let off = base + c * 4;
            if let Some(bytes) = carry.get(off..off.saturating_add(4))
            {
                let mut chunk = [0u8; 4];
                chunk.copy_from_slice(bytes);
                acc += f32::from_le_bytes(chunk);
            }
        }
        ring.push(acc / channels as f32);
    }
}

fn cmd_out(cmd: &'static str, args: &[&str]) -> Result<String> {
    let out =
        Command::new(cmd).args(args).output().map_err(|source| {
            CommandError::MissingCommand { cmd, source }
        })?;

    if !out.status.success() {
        return Err(CommandError::CommandFailed {
            cmd,
            args: args.join(" "),
            status: out.status,
            stdout: String::from_utf8_lossy(&out.stdout)
                .trim()
                .to_string(),
            stderr: String::from_utf8_lossy(&out.stderr)
                .trim()
                .to_string(),
        }
        .into());
    }

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn pactl(args: &[&str]) -> Result<String> {
    cmd_out("pactl", args).context(
        "pactl failed (install pulseaudio-utils, and ensure pipewire-pulse or pulseaudio is running)",
    )
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand { cmd, source } => {
                write!(f, "missing command `{cmd}`: {source}")
            }
            Self::CommandFailed {
                cmd,
                args,
                status,
                stdout,
                stderr,
            } => write!(
                f,
                "command `{cmd} {args}` failed with {status}; stdout: {stdout}; stderr: {stderr}"
            ),
        }
    }
}

impl std::error::Error for CommandError {}

#[derive(Clone)]
struct SourceInfo {
    name: String,
    channels: usize,
    state: String,
}

fn pulse_sources() -> Result<Vec<SourceInfo>> {
    let s = pactl(&["list", "short", "sources"])?;
    let mut out = Vec::new();

    for line in s.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 7 {
            continue;
        }

        let name =
            parts.get(1).map_or_else(String::new, |&x| x.to_string());
        let ch_tok = parts.get(4).copied().unwrap_or("");
        let state =
            parts.get(6).map_or_else(String::new, |&x| x.to_string());

        let channels = ch_tok
            .strip_suffix("ch")
            .and_then(|x| x.parse().ok())
            .unwrap_or(2);

        out.push(SourceInfo {
            name,
            channels,
            state,
        });
    }

    Ok(out)
}

fn resolve_monitor_source() -> Result<SourceInfo> {
    let sources = pulse_sources()?;

    if let Some(hit) = sources
        .iter()
        .filter(|s| s.name.contains(".monitor"))
        .find(|s| s.state == "RUNNING")
    {
        return Ok(hit.clone());
    }

    if let Ok(sink) = pactl(&["get-default-sink"]) {
        if !sink.is_empty() {
            let mon = format!("{sink}.monitor");
            if let Some(hit) = sources.iter().find(|s| s.name == mon)
            {
                return Ok(hit.clone());
            }
        }
    }

    if let Some(hit) =
        sources.iter().find(|s| s.name.contains(".monitor"))
    {
        return Ok(hit.clone());
    }

    anyhow::bail!(
        "no monitor source found (no .monitor sources in pactl list short sources)"
    )
}
