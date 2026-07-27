use anyhow::Result;
use lookas::{
    analyzer::{FlowSpringParams, SpectrumAnalyzer},
    config::Config,
    filterbank::{FilterbankParams, build_filterbank},
};
use std::io::Write;

mod config;
mod fft;
mod paint;
mod samples;

use super::{Runtime, gate::GateState};
use config::FrameConfig;
use fft::FftState;
use paint::FramePaint;
use samples::FrameSamples;

pub struct Frame {
    cfg: FrameConfig,
    analyzer: SpectrumAnalyzer,
    gate: GateState,
    fft: FftState,
    samples: FrameSamples,
    paint: FramePaint,
    dt_s: f32,
}

impl Frame {
    pub fn new(
        cfg: &Config,
        runtime: &Runtime,
        w: u16,
        h: u16,
    ) -> Self {
        let fft_size = runtime.fft_size();
        let fft = FftState::new(fft_size);

        Self {
            cfg: FrameConfig::new(cfg),
            analyzer: SpectrumAnalyzer::new(fft.half),
            gate: make_gate(cfg),
            fft,
            samples: FrameSamples::new(fft_size),
            paint: FramePaint::new(w, h),
            dt_s: 0.0,
        }
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.paint.resize(w, h);
    }

    pub fn reset_gate(&mut self) {
        self.gate.reset();
    }

    pub fn apply_config(&mut self, cfg: &Config, runtime: &Runtime) {
        let filterbank_changed = self.cfg.filterbank_changed(cfg);
        let fft_size = runtime.fft_size();
        let fft_changed = self.samples.len() != fft_size;

        self.cfg.apply(cfg);
        self.gate.open_db = cfg.gate_db;
        self.gate.close_db = (cfg.gate_db - 3.0).max(-80.0);

        if fft_changed {
            self.fft.resize(fft_size);
            self.samples.resize(fft_size);
            self.analyzer.spec_pow_smooth = vec![0.0; self.fft.half];
            self.reset_gate();
        }

        if filterbank_changed || fft_changed {
            self.clear_filters();
            self.analyzer.eq_ref.fill(1e-6);
            self.analyzer.db_low = -60.0;
            self.analyzer.db_high = -20.0;
        }
    }

    pub fn clear_filters(&mut self) {
        self.analyzer.filters.clear();
    }

    pub fn ensure_filterbank(&mut self, runtime: &Runtime) {
        if self.analyzer.filters.len() == self.paint.bars() {
            return;
        }

        self.analyzer.filters = build_filterbank(FilterbankParams {
            sr: runtime.sample_rate(),
            fft_size: runtime.fft_size(),
            bands: self.paint.bars(),
            fmin: self.cfg.fmin,
            fmax: self.cfg.fmax,
        });
        self.analyzer.resize(self.paint.bars());
    }

    pub fn set_delta(&mut self, dt_s: f32) {
        self.dt_s = dt_s;
    }

    pub fn tick<W: Write>(
        &mut self,
        runtime: &Runtime,
        out: &mut W,
    ) -> Result<()> {
        if !self.samples.prepare(runtime) {
            return Ok(());
        }

        self.gate.tick(
            sample_power(self.samples.mix(), runtime.fft_size()),
            self.dt_s,
        );
        self.fft.compute(self.samples.mix(), runtime.fft_size());
        self.analyze();
        self.paint.draw(&mut self.analyzer, out)
    }

    fn analyze(&mut self) {
        self.analyzer.update_spectrum(
            &self.fft.spec_pow,
            self.cfg.tau_spec,
            self.dt_s,
        );
        self.analyzer.analyze_bands(self.dt_s, self.gate.open);
        self.analyzer.apply_flow_and_spring(
            &FlowSpringParams {
                flow_k: self.cfg.flow_k,
                spr_k: self.cfg.spr_k,
                spr_zeta: self.cfg.spr_zeta,
            },
            self.dt_s,
            self.gate.open,
        );
    }
}

fn make_gate(cfg: &Config) -> GateState {
    GateState {
        power_ema: 0.0,
        open: false,
        below_s: 0.0,
        attack_s: 0.012,
        release_s: 0.22,
        open_db: cfg.gate_db,
        close_db: (cfg.gate_db - 3.0).max(-80.0),
        confirm_s: 0.12,
    }
}

fn sample_power(tail: &[f32], fft_size: usize) -> f32 {
    let sum_sq = tail.iter().map(|&x| x * x).sum::<f32>();
    #[allow(clippy::cast_precision_loss)]
    {
        sum_sq / fft_size as f32
    }
}
