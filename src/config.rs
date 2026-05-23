use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub struct Config {
    pub fmin: f32,
    pub fmax: f32,
    /// target frame duration in milliseconds (e.g. 16 ≈ 60 FPS).
    pub frame_ms: u64,
    pub fft_size: usize,
    pub tau_spec: f32,
    pub gate_db: f32,
    pub flow_k: f32,
    pub spr_k: f32,
    pub spr_zeta: f32,
}

impl Config {
    #[must_use]
    pub const fn defaults() -> Self {
        Self {
            fmin: 30.0,
            fmax: 16_000.0,
            frame_ms: 16,
            fft_size: 2048,
            tau_spec: 0.06,
            gate_db: -65.0,
            flow_k: 0.18,
            spr_k: 60.0,
            spr_zeta: 1.0,
        }
    }

    pub fn load() -> Result<Self> {
        let mut cfg = Self::defaults();

        if let Some(file_cfg) = load_file_config()? {
            cfg.apply_file(&file_cfg);
        }

        cfg.sanitize();

        Ok(cfg)
    }

    fn apply_file(&mut self, fc: &FileConfig) {
        if let Some(v) = fc.fmin {
            self.fmin = v;
        }
        if let Some(v) = fc.fmax {
            self.fmax = v;
        }
        if let Some(v) = fc.frame_ms {
            self.frame_ms = v;
        }
        if let Some(v) = fc.fft_size {
            self.fft_size = v;
        }
        if let Some(v) = fc.tau_spec {
            self.tau_spec = v;
        }
        if let Some(v) = fc.gate_db {
            self.gate_db = v;
        }
        if let Some(v) = fc.flow_k {
            self.flow_k = v;
        }
        if let Some(v) = fc.spr_k {
            self.spr_k = v;
        }
        if let Some(v) = fc.spr_zeta {
            self.spr_zeta = v;
        }
    }

    fn sanitize(&mut self) {
        self.fmin = self.fmin.clamp(10.0, 1000.0);
        self.fmax = self.fmax.clamp(1000.0, 24_000.0);

        if self.fmin >= self.fmax {
            self.fmin = 30.0;
            self.fmax = 16_000.0;
        }

        self.frame_ms = self.frame_ms.clamp(8, 50);
        self.fft_size = self.fft_size.clamp(512, 4096);

        self.tau_spec = self.tau_spec.clamp(0.01, 0.20);
        self.gate_db = self.gate_db.clamp(-80.0, -30.0);

        self.flow_k = self.flow_k.clamp(0.0, 1.0);
        self.spr_k = self.spr_k.clamp(10.0, 200.0);
        self.spr_zeta = self.spr_zeta.clamp(0.1, 2.0);
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
struct FileConfig {
    pub fmin: Option<f32>,
    pub fmax: Option<f32>,
    pub frame_ms: Option<u64>,
    pub fft_size: Option<usize>,
    pub tau_spec: Option<f32>,
    pub gate_db: Option<f32>,
    pub flow_k: Option<f32>,
    pub spr_k: Option<f32>,
    pub spr_zeta: Option<f32>,
}

fn load_file_config() -> Result<Option<FileConfig>> {
    let path = dirs::config_dir()
        .context("failed to resolve config directory")?
        .join("lookas.toml");

    if path.exists() {
        return Ok(Some(read_toml(&path)?));
    }

    Ok(None)
}

fn read_toml(path: &Path) -> Result<FileConfig> {
    let s = fs::read_to_string(path).with_context(|| {
        format!("failed to read config: {}", path.display())
    })?;
    toml::from_str::<FileConfig>(&s).with_context(|| {
        format!("invalid TOML in {}", path.display())
    })
}