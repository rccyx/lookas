use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, StreamConfig};

#[allow(clippy::disallowed_methods)]
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
