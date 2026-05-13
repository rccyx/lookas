use lookas::dsp::ema_tc;

pub struct GateState {
    pub pow_ema: f32,
    pub open: bool,
    pub below_s: f32,
    pub attack_s: f32,
    pub release_s: f32,
    pub open_db: f32,
    pub close_db: f32,
    pub confirm_s: f32,
}

impl GateState {
    pub fn reset(&mut self) {
        self.pow_ema = 0.0;
        self.open = false;
        self.below_s = 0.0;
    }

    pub fn tick(&mut self, rms: f32, dt_s: f32) {
        if self.pow_ema == 0.0 {
            self.pow_ema = rms;
        } else {
            let tau = if rms > self.pow_ema {
                self.attack_s
            } else {
                self.release_s
            };
            self.pow_ema = ema_tc(self.pow_ema, rms, tau, dt_s);
        }

        let rms_db = 10.0 * self.pow_ema.max(1e-12).log10();

        if self.open {
            if rms_db < self.close_db {
                self.below_s += dt_s;
                if self.below_s >= self.confirm_s {
                    self.open = false;
                    self.below_s = 0.0;
                }
            } else {
                self.below_s = 0.0;
            }
        } else {
            self.below_s = 0.0;
            if rms_db > self.open_db {
                self.open = true;
            }
        }
    }
}
