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
        let len = self.spec_pow_smooth.len().min(spec_pow.len());
        for (i, &pow) in spec_pow.iter().enumerate().take(len) {
            let incoming = pow.max(1e-12);
            if let Some(prev_val) = self.spec_pow_smooth.get_mut(i) {
                let prev = *prev_val;
                // constant attack if energy rose, snap immediately;
                // smooth release if energy fell, decay via EMA.
                *prev_val = if incoming >= prev {
                    incoming
                } else {
                    ema_tc(prev, incoming, tau_spec, dt_s)
                };
            }
        }
    }

    /// compute per-band dB values and map them to normalised bar targets.
    #[allow(clippy::cognitive_complexity)]
    pub fn analyze_bands(&mut self, dt_s: f32, gate_open: bool) {
        let filters_len = self.filters.len();

        // --- Phase 1 --- //
        for (i, tri) in self.filters.iter().enumerate() {
            let mut acc = 0.0f32;
            for &(idx, wgt) in &tri.taps {
                if let Some(&val) = self.spec_pow_smooth.get(idx) {
                    acc += val * wgt;
                }
            }
            let amp = acc.sqrt();

            // Perceptually accurate frequency-sensitivity curve that peaks at
            // ~3–4 kHz and rolls off at both ends.
            let amp_weighted = amp * a_weighting(tri.center_hz);

            if let Some(eq) = self.eq_ref.get_mut(i) {
                *eq = ema_tc(*eq, amp_weighted, 6.0, dt_s).max(1e-9);
                let rel = amp_weighted / *eq;
                if let Some(target) = self.bars_target.get_mut(i) {
                    *target = 20.0 * rel.max(1e-12).log10();
                }
            }
        }

        // --- Phase 2 ---
        if let (Some(dst), Some(src)) = (
            self.sort_scratch.get_mut(..filters_len),
            self.bars_target.get(..filters_len),
        ) {
            dst.copy_from_slice(src);
        }
        self.update_db_range(filters_len, dt_s);

        // --- Phase 3 ---
        let low = self.db_low - 3.0;
        let high = self.db_high + 6.0;
        let range_inv = 1.0 / (high - low).max(12.0);

        if gate_open {
            for i in 0..filters_len {
                if let Some(target) = self.bars_target.get_mut(i) {
                    let mut v = (*target - low) * range_inv;
                    v = v.clamp(0.0, 1.0).powf(0.85);
                    *target = 1.0 - (1.0 - v).powf(1.6);
                }
            }
        } else if let Some(targets) =
            self.bars_target.get_mut(..filters_len)
        {
            for t in targets {
                *t = 0.0;
            }
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn update_db_range(&mut self, len: usize, dt_s: f32) {
        if len == 0 {
            return;
        }

        // sort_scratch already holds a copy of the raw dB values (set by analyze_bands).
        if let Some(slice) = self.sort_scratch.get_mut(..len) {
            slice.sort_by(f32::total_cmp);
        }

        let len_f = len as f32;
        let idx_low =
            ((len_f - 1.0) * 0.10).round().max(0.0) as usize;
        let idx_high =
            ((len_f - 1.0) * 0.90).round().max(0.0) as usize;

        if let (Some(&q10), Some(&q90)) = (
            self.sort_scratch.get(idx_low),
            self.sort_scratch.get(idx_high),
        ) {
            self.db_low = ema_tc(self.db_low, q10, 0.30, dt_s);
            self.db_high = ema_tc(self.db_high, q90, 0.50, dt_s);
        }
    }

    /// Advance bar positions by one frame using lateral energy diffusion followed
    /// by a spring-damper integration.  Reads targets from `self.bars_target`
    /// (written by `analyze_bands`)
    #[allow(clippy::cognitive_complexity)]
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
                if let Some(y) = self.bars_y.get_mut(i) {
                    *y *= a;
                    if *y < 0.001 {
                        *y = 0.0;
                    }
                }
                if let Some(v) = self.bars_v.get_mut(i) {
                    *v = 0.0;
                }
            }
            return;
        }

        // lateral energy diffusion into pre-allocated scratch.
        for i in 0..n {
            let cur = *self.bars_y.get(i).unwrap_or(&0.0);
            let left = if i > 0 {
                *self.bars_y.get(i.saturating_sub(1)).unwrap_or(&cur)
            } else {
                cur
            };
            let right = if i.saturating_add(1) < n {
                *self.bars_y.get(i.saturating_add(1)).unwrap_or(&cur)
            } else {
                cur
            };
            let flow =
                params.flow_k * 2.0f32.mul_add(-cur, left + right);
            if let Some(scratch) = self.flowed_scratch.get_mut(i) {
                *scratch = (*self.bars_target.get(i).unwrap_or(&0.0)
                    + flow)
                    .clamp(0.0, 1.0);
            }
        }

        let c = 2.0 * params.spr_k.sqrt() * params.spr_zeta;

        for i in 0..n {
            if let (Some(y), Some(v), Some(scratch)) = (
                self.bars_y.get_mut(i),
                self.bars_v.get_mut(i),
                self.flowed_scratch.get(i),
            ) {
                let a = params.spr_k.mul_add(scratch - *y, -(c * *v));
                *v = a.mul_add(dt_s, *v);
                *y = (*v).mul_add(dt_s, *y).clamp(0.0, 1.0);
            }
        }
    }
}
