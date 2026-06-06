use lookas::dsp::ema_tc;

pub struct GateState {
    pub power_ema: f32,
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
        self.power_ema = 0.0;
        self.open = false;
        self.below_s = 0.0;
    }

    pub fn tick(&mut self, power: f32, dt_s: f32) {
        if self.power_ema == 0.0 {
            self.power_ema = power;
        } else {
            let tau = if power > self.power_ema {
                self.attack_s
            } else {
                self.release_s
            };
            self.power_ema = ema_tc(self.power_ema, power, tau, dt_s);
        }

        let power_db = 10.0 * self.power_ema.max(1e-12).log10();

        if self.open {
            if power_db < self.close_db {
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
            if power_db > self.open_db {
                self.open = true;
            }
        }
    }
}
