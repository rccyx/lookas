pub fn compute_rms(tail: &[f32], fft_size: usize) -> f32 {
    let sum_sq = tail.iter().map(|&x| x * x).sum::<f32>();
    #[allow(clippy::cast_precision_loss)]
    {
        sum_sq / fft_size as f32
    }
}
