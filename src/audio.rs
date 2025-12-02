use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Sample, SizedSample, StreamConfig};
use rustfft::num_traits::ToPrimitive;

use crate::buffer::SharedBuf;
use std::sync::{Arc, Mutex};

fn score_device_name(name: &str) -> i32 {
    let n = name.to_lowercase();
    let mut score = 0;

    // prefer obvious sink monitors
    if n.contains("monitor of") {
        score += 200;
    }
    if n.contains(".monitor") {
        score += 160;
    }
    if n.contains("monitor") {
        score += 100;
    }

    // extra hints that it is an output path
    if n.contains("sink")
        || n.contains("output")
        || n.contains("speaker")
    {
        score += 40;
    }

    // penalize obvious mics and webcams
    if n.contains("mic") || n.contains("microphone") {
        score -= 80;
    }
    if n.contains("webcam") || n.contains("camera") {
        score -= 80;
    }
    if n.contains("headset") || n.contains("headphone") {
        score -= 40;
    }
    if n.contains("input") {
        score -= 20;
    }

    score
}

pub fn pick_input_device() -> Result<Device> {
    let host = cpal::default_host();

    // manual override: LOOKAS_DEVICE=substring
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

    // automatic scoring based on device name
    let mut best: Option<(Device, i32)> = None;

    if let Ok(devices) = host.input_devices() {
        for dev in devices {
            let name = match dev.name() {
                Ok(n) => n,
                Err(_) => continue,
            };

            let score = score_device_name(&name);
            if score <= 0 {
                continue;
            }

            match best {
                Some((_, best_score)) if score <= best_score => {}
                _ => {
                    best = Some((dev, score));
                }
            }
        }
    }

    if let Some((dev, _)) = best {
        return Ok(dev);
    }

    // fallback: whatever CPAL thinks is the default input
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
    let err_fn = |e| eprintln!("Stream error: {}", e);

    let stream = device.build_input_stream(
        &cfg,
        move |data: &[T], _| {
            if let Ok(mut buf) = shared.try_lock() {
                let frames = data.chunks_exact(ch);
                for frame in frames {
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
