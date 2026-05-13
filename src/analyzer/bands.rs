use crate::dsp::{a_weighting, ema_tc};

use super::SpectrumAnalyzer;

pub fn analyze_bands(
    sa: &mut SpectrumAnalyzer,
    dt_s: f32,
    gate_open: bool,
) {
    let filters_len = sa.filters.len();
    accumulate_band_db(sa, dt_s, filters_len);
    update_db_range(sa, filters_len, dt_s);
    normalise_targets(sa, filters_len, gate_open);
}

fn accumulate_band_db(
    sa: &mut SpectrumAnalyzer,
    dt_s: f32,
    filters_len: usize,
) {
    for (i, tri) in sa.filters.iter().enumerate().take(filters_len) {
        let mut acc = 0.0f32;
        for &(idx, wgt) in &tri.taps {
            if let Some(&val) = sa.spec_pow_smooth.get(idx) {
                acc += val * wgt;
            }
        }
        let amp_weighted = acc.sqrt() * a_weighting(tri.center_hz);

        if let Some(eq) = sa.eq_ref.get_mut(i) {
            *eq = ema_tc(*eq, amp_weighted, 6.0, dt_s).max(1e-9);
            let rel = amp_weighted / *eq;
            if let Some(target) = sa.bars_target.get_mut(i) {
                *target = 20.0 * rel.max(1e-12).log10();
            }
        }
    }

    if let (Some(dst), Some(src)) = (
        sa.sort_scratch.get_mut(..filters_len),
        sa.bars_target.get(..filters_len),
    ) {
        dst.copy_from_slice(src);
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn update_db_range(sa: &mut SpectrumAnalyzer, len: usize, dt_s: f32) {
    if len == 0 {
        return;
    }

    if let Some(slice) = sa.sort_scratch.get_mut(..len) {
        slice.sort_by(f32::total_cmp);
    }

    let len_f = len as f32;
    let idx_low = ((len_f - 1.0) * 0.10).round().max(0.0) as usize;
    let idx_high = ((len_f - 1.0) * 0.90).round().max(0.0) as usize;

    if let (Some(&q10), Some(&q90)) =
        (sa.sort_scratch.get(idx_low), sa.sort_scratch.get(idx_high))
    {
        sa.db_low = ema_tc(sa.db_low, q10, 0.30, dt_s);
        sa.db_high = ema_tc(sa.db_high, q90, 0.50, dt_s);
    }
}

fn normalise_targets(
    sa: &mut SpectrumAnalyzer,
    filters_len: usize,
    gate_open: bool,
) {
    let low = sa.db_low - 3.0;
    let high = sa.db_high + 6.0;
    let range_inv = 1.0 / (high - low).max(12.0);

    if gate_open {
        for i in 0..filters_len {
            if let Some(target) = sa.bars_target.get_mut(i) {
                let mut v = (*target - low) * range_inv;
                v = v.clamp(0.0, 1.0).powf(0.85);
                *target = 1.0 - (1.0 - v).powf(1.6);
            }
        }
    } else if let Some(targets) =
        sa.bars_target.get_mut(..filters_len)
    {
        for t in targets {
            *t = 0.0;
        }
    }
}
