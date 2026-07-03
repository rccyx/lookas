use anyhow::Result;
use crossterm::{cursor, queue};
use lookas::{
    analyzer::{FlowSpringParams, SpectrumAnalyzer},
    audio::AudioMode,
    config::Config,
    dsp::hann,
    filterbank::{build_filterbank, FilterbankParams},
    render::{draw_blocks_vertical, layout_for, Layout},
};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::{io::Write, sync::Arc};

use super::super::{
    fft::{compute_spectrum, FftContext},
    gate::GateState,
    mix::compute_power,
};
use super::runtime::Runtime;

pub struct Frame {
    cfg: FrameConfig,
    analyzer: SpectrumAnalyzer,
    gate: GateState,
    window: Vec<f32>,
    half: usize,
    fft: Arc<dyn RealToComplex<f32>>,
    buf: Vec<f32>,
    fft_out: Vec<Complex<f32>>,
    spec_pow: Vec<f32>,
    mic_tail: Vec<f32>,
    sys_tail: Vec<f32>,
    mix: Vec<f32>,
    lay: Layout,
    render: Vec<u8>,
    w: u16,
    h: u16,
    top_pad: u16,
    dt_s: f32,
}

struct FrameConfig {
    tau_spec: f32,
    flow_k: f32,
    spr_k: f32,
    spr_zeta: f32,
    fmin: f32,
    fmax: f32,
}

struct AudioReady {
    mic: bool,
    system: bool,
}

impl Frame {
    pub fn new(
        cfg: &Config,
        runtime: &Runtime,
        w: u16,
        h: u16,
    ) -> Self {
        let fft_size = runtime.fft_size();
        let top_pad: u16 = 0;
        let half = fft_size / 2;
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let buf = fft.make_input_vec();
        let fft_out = fft.make_output_vec();

        #[allow(clippy::arithmetic_side_effects)]
        let frame_cap = (w as usize * h as usize * 4).max(64 * 1024);

        Self {
            cfg: FrameConfig {
                tau_spec: cfg.tau_spec,
                flow_k: cfg.flow_k,
                spr_k: cfg.spr_k,
                spr_zeta: cfg.spr_zeta,
                fmin: cfg.fmin,
                fmax: cfg.fmax,
            },
            analyzer: SpectrumAnalyzer::new(half),
            gate: GateState {
                power_ema: 0.0,
                open: false,
                below_s: 0.0,
                attack_s: 0.012,
                release_s: 0.22,
                open_db: cfg.gate_db,
                close_db: (cfg.gate_db - 3.0).max(-80.0),
                confirm_s: 0.12,
            },
            window: hann(fft_size),
            half,
            fft,
            buf,
            fft_out,
            spec_pow: vec![0.0f32; half],
            mic_tail: Vec::with_capacity(fft_size),
            sys_tail: Vec::with_capacity(fft_size),
            mix: vec![0.0f32; fft_size],
            lay: layout_for(w, h, top_pad),
            render: Vec::with_capacity(frame_cap),
            w,
            h,
            top_pad,
            dt_s: 0.0,
        }
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.w = w;
        self.h = h;
        self.lay = layout_for(self.w, self.h, self.top_pad);
    }

    pub fn reset_gate(&mut self) {
        self.gate.reset();
    }

    pub fn clear_filters(&mut self) {
        self.analyzer.filters.clear();
    }

    pub fn ensure_filterbank(&mut self, runtime: &Runtime) {
        if self.analyzer.filters.len() == self.lay.bars {
            return;
        }

        self.analyzer.filters = build_filterbank(FilterbankParams {
            sr: runtime.sample_rate(),
            fft_size: runtime.fft_size(),
            bands: self.lay.bars,
            fmin: self.cfg.fmin,
            fmax: self.cfg.fmax,
        });
        self.analyzer.resize(self.lay.bars);
    }

    pub fn set_delta(&mut self, dt_s: f32) {
        self.dt_s = dt_s;
    }

    pub fn tick<W: Write>(
        &mut self,
        runtime: &Runtime,
        out: &mut W,
    ) -> Result<()> {
        if !self.prepare_samples(runtime) {
            return Ok(());
        }

        self.gate.tick(
            compute_power(&self.mix, runtime.fft_size()),
            self.dt_s,
        );
        self.cs(runtime.fft_size());
        self.analyze();
        self.draw(out)
    }

    fn cs(&mut self, fft_size: usize) {
        compute_spectrum(&mut FftContext {
            tail: &self.mix,
            window: &self.window,
            buf: &mut self.buf,
            fft_out: &mut self.fft_out,
            fft: &self.fft,
            spec_pow: &mut self.spec_pow,
            half: self.half,
            fft_size,
        });
    }

    fn analyze(&mut self) {
        self.analyzer.update_spectrum(
            &self.spec_pow,
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

    fn draw<W: Write>(&mut self, out: &mut W) -> Result<()> {
        queue!(out, cursor::MoveTo(0, self.top_pad))?;
        self.render.clear();
        draw_blocks_vertical(
            &mut self.render,
            &self.analyzer.bars_y,
            self.w,
            self.h,
            &self.lay,
            &mut self.analyzer.render_fulls,
            &mut self.analyzer.render_fracs,
        )?;
        out.write_all(&self.render)?;
        out.flush()?;
        Ok(())
    }

    fn prepare_samples(&mut self, runtime: &Runtime) -> bool {
        let ready = self.copy_tails(runtime);

        match runtime.mode() {
            AudioMode::Mic => self.copy_mic(ready.mic),
            AudioMode::System => self.copy_system(ready.system),
            AudioMode::Both => {
                self.mix_samples(runtime.fft_size(), &ready)
            }
        }
    }

    fn copy_tails(&mut self, runtime: &Runtime) -> AudioReady {
        AudioReady {
            mic: runtime.copy_mic_tail(&mut self.mic_tail),
            system: runtime.copy_system_tail(&mut self.sys_tail),
        }
    }

    fn copy_mic(&mut self, ready: bool) -> bool {
        if ready {
            self.mix.copy_from_slice(&self.mic_tail);
        }
        ready
    }

    fn copy_system(&mut self, ready: bool) -> bool {
        if ready {
            self.mix.copy_from_slice(&self.sys_tail);
        }
        ready
    }

    fn mix_samples(
        &mut self,
        fft_size: usize,
        ready: &AudioReady,
    ) -> bool {
        if !ready.mic || !ready.system {
            return false;
        }

        #[allow(clippy::indexing_slicing)]
        for i in 0..fft_size {
            self.mix[i] = (self.mic_tail[i] + self.sys_tail[i]) * 0.5;
        }

        true
    }
}
