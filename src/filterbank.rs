use crate::dsp::{hz_to_mel, mel_to_hz};

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

#[allow(
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn calculate_bin_points(
    mel_points: &[f32],
    hz_per_bin: f32,
    half: usize,
) -> Vec<usize> {
    let mut bin_points = Vec::with_capacity(mel_points.len());
    for &mel in mel_points {
        let hz = mel_to_hz(mel);
        let mut b = (hz / hz_per_bin).round() as isize;
        if b < 1 {
            b = 1;
        }
        if b as usize >= half {
            b = (half.saturating_sub(1)) as isize;
        }
        bin_points.push(b as usize);
    }

    for i in 1..bin_points.len() {
        let prev = *bin_points.get(i - 1).unwrap_or(&0);
        if let Some(curr) = bin_points.get_mut(i) {
            if *curr <= prev {
                *curr = (prev + 1).min(half.saturating_sub(1));
            }
        }
    }
    bin_points
}

#[allow(clippy::cast_precision_loss, clippy::arithmetic_side_effects)]
fn create_filters(
    bin_points: &[usize],
    bands: usize,
    hz_per_bin: f32,
) -> Vec<Tri> {
    let mut filters = Vec::with_capacity(bands);
    for b in 0..bands {
        let l = *bin_points.get(b).unwrap_or(&0);
        let c = *bin_points.get(b + 1).unwrap_or(&0);
        let r = *bin_points.get(b + 2).unwrap_or(&0);

        let mut taps = Vec::with_capacity(r.saturating_sub(l) + 1);

        let lc_diff = c.saturating_sub(l);
        if lc_diff > 0 {
            let lc_diff_f = lc_diff as f32;
            for i in l..=c {
                let w = (i.saturating_sub(l)) as f32 / lc_diff_f;
                taps.push((i, w));
            }
        }

        let cr_diff = r.saturating_sub(c);
        if cr_diff > 0 {
            let cr_diff_f = cr_diff as f32;
            for i in (c + 1)..=r {
                let w = 1.0
                    - (i.saturating_sub(c).saturating_sub(1)) as f32
                        / cr_diff_f;
                taps.push((i, w));
            }
        }

        let sumw =
            taps.iter().map(|(_, w)| *w).sum::<f32>().max(1e-6);
        let inv_sumw = 1.0 / sumw;
        for t in &mut taps {
            t.1 *= inv_sumw;
        }

        let center_hz = c as f32 * hz_per_bin;
        filters.push(Tri { taps, center_hz });
    }
    filters
}
