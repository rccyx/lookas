use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
}

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
    pub color: RgbColor,
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
            color: RgbColor::WHITE,
        }
    }

    pub fn load() -> Result<Self> {
        let mut cfg = Self::defaults();

        if let Some(file_cfg) = load_file_config()? {
            cfg.apply_file(&file_cfg)?;
        }

        cfg.sanitize();

        Ok(cfg)
    }

    fn apply_file(&mut self, fc: &FileConfig) -> Result<()> {
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
        if let Some(v) = fc.color.as_deref() {
            self.color = parse_hex_color(v)?;
        }

        Ok(())
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

#[derive(Debug, Deserialize, Default, Clone)]
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
    pub color: Option<String>,
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

fn parse_hex_color(value: &str) -> Result<RgbColor> {
    let value = value.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);
    let bytes = hex.as_bytes();

    let &[r_hi, r_lo, g_hi, g_lo, b_hi, b_lo] = bytes else {
        return Err(invalid_color(value));
    };

    let r = parse_hex_channel(r_hi, r_lo)
        .ok_or_else(|| invalid_color(value))?;
    let g = parse_hex_channel(g_hi, g_lo)
        .ok_or_else(|| invalid_color(value))?;
    let b = parse_hex_channel(b_hi, b_lo)
        .ok_or_else(|| invalid_color(value))?;

    Ok(RgbColor { r, g, b })
}

fn invalid_color(value: &str) -> anyhow::Error {
    anyhow::anyhow!("invalid color `{value}`: expected `#RRGGBB`")
}

#[allow(clippy::arithmetic_side_effects)]
const fn parse_hex_channel(high: u8, low: u8) -> Option<u8> {
    let Some(high) = parse_hex_digit(high) else {
        return None;
    };
    let Some(low) = parse_hex_digit(low) else {
        return None;
    };

    Some((high << 4) | low)
}

#[allow(clippy::arithmetic_side_effects)]
const fn parse_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{RgbColor, parse_hex_color};

    #[test]
    fn parses_hash_prefixed_hex_color() {
        let Ok(color) = parse_hex_color("#7CCEA7") else {
            panic!("valid color should parse");
        };

        assert_eq!(
            color,
            RgbColor {
                r: 124,
                g: 206,
                b: 167,
            }
        );
    }

    #[test]
    fn parses_unprefixed_hex_color() {
        let Ok(color) = parse_hex_color("7ccea7") else {
            panic!("valid color should parse");
        };

        assert_eq!(
            color,
            RgbColor {
                r: 124,
                g: 206,
                b: 167,
            }
        );
    }

    #[test]
    fn rejects_invalid_hex_color() {
        assert!(parse_hex_color("green").is_err());
        assert!(parse_hex_color("#12FG00").is_err());
        assert!(parse_hex_color("#12345678").is_err());
    }
}