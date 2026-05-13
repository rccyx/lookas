#[must_use]
#[allow(clippy::cast_precision_loss, clippy::arithmetic_side_effects)]
pub fn hann(n: usize) -> Vec<f32> {
    let n_max_2 = n.max(2);
    let den = (n_max_2 - 1) as f32;
    let pi2_div_den = 2.0 * std::f32::consts::PI / den;
    (0..n)
        .map(|i| {
            let cos_val = f32::cos(pi2_div_den * i as f32);
            0.5f32.mul_add(-cos_val, 0.5)
        })
        .collect()
}

#[inline]
pub fn prepare_fft_input_inplace(
    samples: &[f32],
    window: &[f32],
    buf: &mut Vec<f32>,
) {
    buf.clear();
    buf.extend(
        samples.iter().zip(window.iter()).map(|(&s, &w)| s * w),
    );
}
