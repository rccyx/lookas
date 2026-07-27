use lookas::dsp::{hann, prepare_fft_input_inplace};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub struct FftState {
    pub window: Vec<f32>,
    pub half: usize,
    pub fft: Arc<dyn RealToComplex<f32>>,
    pub buf: Vec<f32>,
    pub fft_out: Vec<Complex<f32>>,
    pub spec_pow: Vec<f32>,
}

impl FftState {
    pub fn new(fft_size: usize) -> Self {
        let half = fft_size / 2;
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let buf = fft.make_input_vec();
        let fft_out = fft.make_output_vec();

        Self {
            window: hann(fft_size),
            half,
            fft,
            buf,
            fft_out,
            spec_pow: vec![0.0f32; half],
        }
    }

    pub fn resize(&mut self, fft_size: usize) {
        *self = Self::new(fft_size);
    }

    pub fn compute(&mut self, samples: &[f32], fft_size: usize) {
        prepare_fft_input_inplace(
            samples,
            &self.window,
            &mut self.buf,
        );

        if let Err(e) =
            self.fft.process(&mut self.buf, &mut self.fft_out)
        {
            eprintln!("[lookas] FFT processing error: {e}");
            return;
        }

        #[allow(clippy::cast_precision_loss)]
        let norm_inv = 1.0 / ((fft_size as f32) * (fft_size as f32));
        #[allow(clippy::indexing_slicing)]
        for i in 0..self.half {
            let re = self.fft_out[i].re;
            let im = self.fft_out[i].im;
            self.spec_pow[i] = re.mul_add(re, im * im) * norm_inv;
        }
    }
}
