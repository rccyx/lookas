use anyhow::Result;
use cpal::traits::DeviceTrait;
use cpal::{Device, Sample, SizedSample, StreamConfig};

use crate::buffer::SharedBuf;
use std::sync::{Arc, Mutex};

pub fn build_stream<T>(
    device: &Device,
    cfg: StreamConfig,
    shared: Arc<Mutex<SharedBuf>>,
) -> Result<cpal::Stream>
where
    T: Sample + SizedSample,
    f32: cpal::FromSample<T>,
{
    let ch = usize::from(cfg.channels);
    let err_fn = |err: cpal::Error| {
        eprintln!("[lookas] audio stream error: {err}");
    };

    let stream = device.build_input_stream(
        cfg,
        move |data: &[T], _| {
            if let Ok(mut buf) = shared.try_lock() {
                for frame in data.chunks_exact(ch) {
                    let mut acc = 0.0f32;
                    for &s in frame {
                        acc += s.to_sample::<f32>();
                    }
                    #[allow(clippy::cast_precision_loss)]
                    buf.push(acc / ch as f32);
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
