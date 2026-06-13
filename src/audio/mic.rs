use anyhow::Result;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{I24, SampleFormat, U24};

use crate::buffer::SharedBuf;
use std::sync::{Arc, Mutex};

use super::device::{best_supported_config_for, pick_input_device};
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
    let label = device.description().map_or_else(
        |_| "mic".into(),
        |desc| desc.name().to_string(),
    );

    let supported_cfg = best_supported_config_for(&device)?;
    let cfg = supported_cfg.config();
    let sample_rate = cfg.sample_rate;

    let stream = match supported_cfg.sample_format() {
        SampleFormat::F32 => {
            build_stream::<f32>(&device, cfg, shared)?
        }
        SampleFormat::F64 => {
            build_stream::<f64>(&device, cfg, shared)?
        }
        SampleFormat::I8 => build_stream::<i8>(&device, cfg, shared)?,
        SampleFormat::I16 => {
            build_stream::<i16>(&device, cfg, shared)?
        }
        SampleFormat::I24 => {
            build_stream::<I24>(&device, cfg, shared)?
        }
        SampleFormat::I32 => {
            build_stream::<i32>(&device, cfg, shared)?
        }
        SampleFormat::I64 => {
            build_stream::<i64>(&device, cfg, shared)?
        }
        SampleFormat::U8 => build_stream::<u8>(&device, cfg, shared)?,
        SampleFormat::U16 => {
            build_stream::<u16>(&device, cfg, shared)?
        }
        SampleFormat::U24 => {
            build_stream::<U24>(&device, cfg, shared)?
        }
        SampleFormat::U32 => {
            build_stream::<u32>(&device, cfg, shared)?
        }
        SampleFormat::U64 => {
            build_stream::<u64>(&device, cfg, shared)?
        }
        _ => anyhow::bail!(
            "Unsupported sample format: {}",
            supported_cfg.sample_format()
        ),
    };

    stream.play()?;

    Ok(MicHandle {
        _stream: stream,
        label,
        sample_rate,
    })
}
