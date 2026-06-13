use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{
    Device, StreamConfig, SupportedStreamConfig,
    SupportedStreamConfigRange,
};

pub fn pick_input_device() -> Result<Device> {
    cpal::default_host()
        .default_input_device()
        .context("No default input device")
}

pub fn best_config_for(device: &Device) -> Result<StreamConfig> {
    Ok(best_supported_config_for(device)?.config())
}

pub(super) fn best_supported_config_for(
    device: &Device,
) -> Result<SupportedStreamConfig> {
    let default = device.default_input_config()?;
    let default_format = default.sample_format();
    let configs = device.supported_input_configs()?;

    let ranges = configs
        .filter(|range| range.sample_format() == default_format)
        .collect::<Vec<_>>();

    for rate in [48_000, 44_100] {
        if let Some(config) = config_with_rate(&ranges, rate) {
            return Ok(config);
        }
    }

    Ok(default)
}

fn config_with_rate(
    ranges: &[SupportedStreamConfigRange],
    rate: u32,
) -> Option<SupportedStreamConfig> {
    ranges
        .iter()
        .find_map(|range| (*range).try_with_sample_rate(rate))
}
