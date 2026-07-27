use lookas::config::Config;

pub struct FrameConfig {
    pub tau_spec: f32,
    pub flow_k: f32,
    pub spr_k: f32,
    pub spr_zeta: f32,
    pub fmin: f32,
    pub fmax: f32,
}

impl FrameConfig {
    pub const fn new(cfg: &Config) -> Self {
        Self {
            tau_spec: cfg.tau_spec,
            flow_k: cfg.flow_k,
            spr_k: cfg.spr_k,
            spr_zeta: cfg.spr_zeta,
            fmin: cfg.fmin,
            fmax: cfg.fmax,
        }
    }

    pub fn filterbank_changed(&self, cfg: &Config) -> bool {
        self.fmin.to_bits() != cfg.fmin.to_bits()
            || self.fmax.to_bits() != cfg.fmax.to_bits()
    }

    pub fn apply(&mut self, cfg: &Config) {
        self.tau_spec = cfg.tau_spec;
        self.flow_k = cfg.flow_k;
        self.spr_k = cfg.spr_k;
        self.spr_zeta = cfg.spr_zeta;
        self.fmin = cfg.fmin;
        self.fmax = cfg.fmax;
    }
}
