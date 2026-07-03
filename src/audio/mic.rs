use anyhow::Result;
use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, StreamTrait};

use crate::buffer::SharedBuf;
use std::sync::{Arc, Mutex};

use super::device::{best_config_for, pick_input_device};
use super::stream::build_stream;

pub struct MicHandle {
    pub(super) _stream: cpal::Stream,
    pub(super) label: String,
    pub(super) sample_rate: u32,
}

pub(super) fn start_mic(
    shared: Arc<Mutex<SharedBuf>>,
) -> Result<MicHandle> {
    let device = pick_input_device()?;
    let label = device.name().unwrap_or_else(|_| "mic".into());
    let supported_cfg = best_config_for(&device)?;
    let cfg = supported_cfg.config();
    let sample_rate = cfg.sample_rate.0;

    let stream = match supported_cfg.sample_format() {
        SampleFormat::F32 => {
            build_stream::<f32>(&device, &cfg, shared)?
        }
        SampleFormat::I16 => {
            build_stream::<i16>(&device, &cfg, shared)?
        }
        SampleFormat::U16 => {
            build_stream::<u16>(&device, &cfg, shared)?
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
