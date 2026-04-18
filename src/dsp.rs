pub fn hann(n: usize) -> Vec<f32> {
    let den = (n.max(2) - 1) as f32;
    let pi2_div_den = 2.0 * std::f32::consts::PI / den;
    (0..n)
        .map(|i| 0.5 - 0.5 * f32::cos(pi2_div_den * i as f32))
        .collect()
}

#[inline]
pub fn ema_tc(prev: f32, x: f32, tau_s: f32, dt_s: f32) -> f32 {
    let a = (-dt_s / tau_s).exp();
    a * prev + (1.0 - a) * x
}

#[inline]
pub fn hz_to_mel(f: f32) -> f32 {
    2595.0 * (1.0 + f / 700.0).log10()
}

#[inline]
pub fn mel_to_hz(m: f32) -> f32 {
    700.0 * (10f32.powf(m / 2595.0) - 1.0)
}

pub fn prepare_fft_input(
    samples: &[f32],
    window: &[f32],
) -> Vec<f32> {
    samples
        .iter()
        .zip(window.iter())
        .map(|(&s, &w)| s * w)
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

// weighting amplitude multiplier per IEC 61672:2003.
/// Returns a linear scale factor (not dB) that, when applied to a
/// linear amplitude value, weights it according to the human ear's
/// frequency sensitivity. The curve peaks near 3–4 kHz and rolls off
/// steeply below ~500 Hz and above ~10 kHz.
#[inline]
pub fn a_weighting(hz: f32) -> f32 {
    // clamped to avoid division-by-zero or meaningless sub-1 Hz input.
    let f = hz.max(10.0);
    let f2 = f * f;
    let f4 = f2 * f2;

    const P1_SQ: f32 = 20.6_f32 * 20.6_f32; // 424.36
    const P2_SQ: f32 = 107.7_f32 * 107.7_f32; // 11_599.29
    const P3_SQ: f32 = 737.9_f32 * 737.9_f32; // 544_496.41
    const P4_SQ: f32 = 12_194.0_f32 * 12_194.0_f32; // 148_692_836.0

    // Unnormalised Ra(f).
    let num = P4_SQ * f4;
    let den = (f2 + P1_SQ)
        * ((f2 + P2_SQ) * (f2 + P3_SQ)).sqrt()
        * (f2 + P4_SQ);

    let ra = num / den;

    // IEC 61672 normalises so that A(1 kHz) = 0 dB ,the standard adds +2.0 dB to the raw Ra curve, i.e. multiplies
    // by 10^(2/20) ≈ 1.2589.  This makes Ra(1000) * NORM ≈ 1.0.
    const NORM: f32 = 1.258_925_4; // 10^(2/20)
    ra * NORM
}
