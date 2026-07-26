use lookas::dsp::prepare_fft_input_inplace;
use realfft::RealToComplex;
use realfft::num_complex::Complex;
use std::sync::Arc;

pub struct FftContext<'a> {
    pub tail: &'a [f32],
    pub window: &'a [f32],
    pub buf: &'a mut Vec<f32>,
    pub fft_out: &'a mut [Complex<f32>],
    pub fft: &'a Arc<dyn RealToComplex<f32>>,
    pub spec_pow: &'a mut [f32],
    pub half: usize,
    pub fft_size: usize,
}

pub fn compute_spectrum(ctx: &mut FftContext<'_>) {
    prepare_fft_input_inplace(ctx.tail, ctx.window, ctx.buf);

    if let Err(e) = ctx.fft.process(ctx.buf, ctx.fft_out) {
        eprintln!("[lookas] FFT processing error: {e}");
        return;
    }

    #[allow(clippy::cast_precision_loss)]
    let norm_inv =
        1.0 / ((ctx.fft_size as f32) * (ctx.fft_size as f32));
    #[allow(clippy::indexing_slicing)]
    for i in 0..ctx.half {
        let re = ctx.fft_out[i].re;
        let im = ctx.fft_out[i].im;
        ctx.spec_pow[i] = re.mul_add(re, im * im) * norm_inv;
    }
}
