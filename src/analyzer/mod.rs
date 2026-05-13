mod bands;
mod params;
mod spectrum;
mod spring;

pub use params::FlowSpringParams;

use crate::filterbank::Tri;

pub struct SpectrumAnalyzer {
    pub spec_pow_smooth: Vec<f32>,
    pub filters: Vec<Tri>,
    pub bars_y: Vec<f32>,
    pub bars_v: Vec<f32>,
    pub eq_ref: Vec<f32>,
    pub db_low: f32,
    pub db_high: f32,
    pub(crate) sort_scratch: Vec<f32>,
    pub bars_target: Vec<f32>,
    pub(crate) flowed_scratch: Vec<f32>,
}

impl SpectrumAnalyzer {
    #[must_use]
    pub fn new(half_fft_size: usize) -> Self {
        Self {
            spec_pow_smooth: vec![0.0; half_fft_size],
            filters: Vec::new(),
            bars_y: Vec::new(),
            bars_v: Vec::new(),
            eq_ref: Vec::new(),
            db_low: -60.0,
            db_high: -20.0,
            sort_scratch: Vec::new(),
            bars_target: Vec::new(),
            flowed_scratch: Vec::new(),
        }
    }

    #[inline]
    pub fn resize(&mut self, num_bars: usize) {
        if self.bars_y.len() != num_bars {
            self.bars_y = vec![0.0; num_bars];
            self.bars_v = vec![0.0; num_bars];
            self.eq_ref = vec![1e-6; num_bars];
            self.bars_target = vec![0.0; num_bars];
            self.sort_scratch = vec![0.0; num_bars];
            self.flowed_scratch = vec![0.0; num_bars];
        }
    }

    pub fn update_spectrum(
        &mut self,
        spec_pow: &[f32],
        tau_spec: f32,
        dt_s: f32,
    ) {
        spectrum::update_spectrum(self, spec_pow, tau_spec, dt_s);
    }

    pub fn analyze_bands(&mut self, dt_s: f32, gate_open: bool) {
        bands::analyze_bands(self, dt_s, gate_open);
    }

    pub fn apply_flow_and_spring(
        &mut self,
        params: &FlowSpringParams,
        dt_s: f32,
        gate_open: bool,
    ) {
        spring::apply_flow_and_spring(self, params, dt_s, gate_open);
    }
}
