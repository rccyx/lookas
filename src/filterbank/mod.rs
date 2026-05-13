mod filters;
mod mel_bins;

use crate::dsp::hz_to_mel;

pub use filters::create_filters;
pub use mel_bins::calculate_bin_points;

#[derive(Clone)]
pub struct Tri {
    pub taps: Vec<(usize, f32)>,
    pub center_hz: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct FilterbankParams {
    pub sr: f32,
    pub fft_size: usize,
    pub bands: usize,
    pub fmin: f32,
    pub fmax: f32,
}

#[must_use]
#[allow(clippy::cast_precision_loss, clippy::arithmetic_side_effects)]
pub fn build_filterbank(params: FilterbankParams) -> Vec<Tri> {
    let half = params.fft_size / 2;
    let hz_per_bin = params.sr / params.fft_size as f32;
    let mmin = hz_to_mel(params.fmin.max(hz_per_bin));
    let mmax = hz_to_mel(
        params.fmax.min(params.sr.mul_add(0.5, -hz_per_bin)),
    );
    let mel_step = (mmax - mmin) / (params.bands as f32 + 1.0);

    let mut mel_points = Vec::with_capacity(params.bands + 2);
    for i in 0..(params.bands + 2) {
        mel_points.push((i as f32).mul_add(mel_step, mmin));
    }

    let bin_points =
        calculate_bin_points(&mel_points, hz_per_bin, half);
    create_filters(&bin_points, params.bands, hz_per_bin)
}
