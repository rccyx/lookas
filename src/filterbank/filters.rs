use super::Tri;

#[allow(clippy::cast_precision_loss, clippy::arithmetic_side_effects)]
#[must_use]
pub fn create_filters(
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
