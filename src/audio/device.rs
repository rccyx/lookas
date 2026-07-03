use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{
    Device, SampleRate, SupportedStreamConfig, SupportedStreamConfigRange,
};

const PREFERRED_INPUT_SAMPLE_RATES: [SampleRate; 2] = [
    SampleRate(48_000), 
    SampleRate(44_100),
];

pub fn pick_input_device() -> Result<Device> {
    cpal::default_host()
        .default_input_device()
        .context("No default input device")
}

pub fn best_config_for(device: &Device) -> Result<SupportedStreamConfig> {
    let default = device.default_input_config()?;
    let default_format = default.sample_format();

    let ranges = device
        .supported_input_configs()?
        .filter(|range| range.sample_format() == default_format)
        .collect::<Vec<_>>();

    for sample_rate in PREFERRED_INPUT_SAMPLE_RATES {
        if let Some(config) = config_with_rate(&ranges, sample_rate) {
            return Ok(config);
        }
    }

    Ok(default)
}

fn config_with_rate(
    ranges: &[SupportedStreamConfigRange],
    sample_rate: SampleRate,
) -> Option<SupportedStreamConfig> {
    ranges
        .iter()
        .find_map(|range| range.clone().try_with_sample_rate(sample_rate))
}