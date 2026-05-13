use crate::dsp::mel_to_hz;

#[allow(
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#[must_use]
pub fn calculate_bin_points(
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
