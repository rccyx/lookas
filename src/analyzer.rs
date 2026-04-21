use crate::dsp::{a_weighting, ema_tc};
use crate::filterbank::Tri;

pub struct SpectrumAnalyzer {
    pub spec_pow_smooth: Vec<f32>,
    pub filters: Vec<Tri>,
    pub bars_y: Vec<f32>,
    pub bars_v: Vec<f32>,
    pub eq_ref: Vec<f32>,
    pub db_low: f32,
    pub db_high: f32,
    /// holds raw dB values (phase 1), then a sorted copy for percentile tracking (phase 2).
    sort_scratch: Vec<f32>,
    /// holds dB values temporarily (phase 1–2), then final normalised bar targets (phase 3).
    pub bars_target: Vec<f32>,
    /// holds lateral-flow intermediate values for the spring-damper step.
    flowed_scratch: Vec<f32>,
}

pub struct FlowSpringParams {
    pub flow_k: f32,
    pub spr_k: f32,
    pub spr_zeta: f32,
}

impl SpectrumAnalyzer {
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
        let len = self.spec_pow_smooth.len().min(spec_pow.len());
        for (i, &pow) in spec_pow.iter().enumerate().take(len) {
            let incoming = pow.max(1e-12);
            let prev = self.spec_pow_smooth[i];

            // constant attack if energy rose, snap immediately;
            // smooth release if energy fell, decay via EMA.
            self.spec_pow_smooth[i] = if incoming >= prev {
                incoming
            } else {
                ema_tc(prev, incoming, tau_spec, dt_s)
            };
        }
    }

    /// compute per-band dB values and map them to normalised bar targets.
    pub fn analyze_bands(&mut self, dt_s: f32, gate_open: bool) {
        let filters_len = self.filters.len();

        // --- Phase 1 --- //
        for (i, tri) in self.filters.iter().enumerate() {
            let mut acc = 0.0f32;
            for &(idx, wgt) in &tri.taps {
                if idx < self.spec_pow_smooth.len() {
                    acc += self.spec_pow_smooth[idx] * wgt;
                }
            }
            let amp = acc.sqrt();

            // Perceptually accurate frequency-sensitivity curve that peaks at
            // ~3–4 kHz and rolls off at both ends.
            let amp_weighted = amp * a_weighting(tri.center_hz);

            self.eq_ref[i] =
                ema_tc(self.eq_ref[i], amp_weighted, 6.0, dt_s)
                    .max(1e-9);
            let rel = amp_weighted / self.eq_ref[i];

            self.bars_target[i] = 20.0 * rel.max(1e-12).log10();
        }

        // --- Phase 2 ---
        self.sort_scratch[..filters_len]
            .copy_from_slice(&self.bars_target[..filters_len]);
        self.update_db_range(filters_len, dt_s);

        // --- Phase 3 ---
        let low = self.db_low - 3.0;
        let high = self.db_high + 6.0;
        let range_inv = 1.0 / (high - low).max(12.0);

        if gate_open {
            for i in 0..filters_len {
                let mut v = (self.bars_target[i] - low) * range_inv;
                v = v.clamp(0.0, 1.0).powf(0.85);
                self.bars_target[i] = 1.0 - (1.0 - v).powf(1.6);
            }
        } else {
            for t in &mut self.bars_target[..filters_len] {
                *t = 0.0;
            }
        }
    }

    fn update_db_range(&mut self, len: usize, dt_s: f32) {
        if len == 0 {
            return;
        }

        // sort_scratch already holds a copy of the raw dB values (set by analyze_bands).
        self.sort_scratch[..len].sort_by(|a, b| a.total_cmp(b));

        let idx_low = ((len - 1) as f32 * 0.10).round() as usize;
        let idx_high = ((len - 1) as f32 * 0.90).round() as usize;

        let q10 = self.sort_scratch[idx_low];
        let q90 = self.sort_scratch[idx_high];

        self.db_low = ema_tc(self.db_low, q10, 0.30, dt_s);
        self.db_high = ema_tc(self.db_high, q90, 0.50, dt_s);
    }

    /// Advance bar positions by one frame using lateral energy diffusion followed
    /// by a spring-damper integration.  Reads targets from `self.bars_target`
    /// (written by `analyze_bands`)
    pub fn apply_flow_and_spring(
        &mut self,
        params: &FlowSpringParams,
        dt_s: f32,
        gate_open: bool,
    ) {
        let n = self.bars_y.len();
        if n == 0 {
            return;
        }

        if !gate_open {
            let tau_silence = 0.22f32;
            let a = (-dt_s / tau_silence).exp();

            for i in 0..n {
                self.bars_y[i] *= a;
                self.bars_v[i] = 0.0;
                if self.bars_y[i] < 0.001 {
                    self.bars_y[i] = 0.0;
                }
            }
            return;
        }

        // lateral energy diffusion into pre-allocated scratch.
        for i in 0..n {
            let left = if i > 0 {
                self.bars_y[i - 1]
            } else {
                self.bars_y[i]
            };
            let right = if i + 1 < n {
                self.bars_y[i + 1]
            } else {
                self.bars_y[i]
            };
            let flow =
                params.flow_k * (left + right - 2.0 * self.bars_y[i]);
            self.flowed_scratch[i] =
                (self.bars_target[i] + flow).clamp(0.0, 1.0);
        }

        let c = 2.0 * params.spr_k.sqrt() * params.spr_zeta;

        for i in 0..n {
            let a = params.spr_k
                * (self.flowed_scratch[i] - self.bars_y[i])
                - c * self.bars_v[i];
            self.bars_v[i] += a * dt_s;
            self.bars_y[i] = (self.bars_y[i] + self.bars_v[i] * dt_s)
                .clamp(0.0, 1.0);
        }
    }
}
