#[inline]
#[must_use]
pub fn ema_tc(prev: f32, x: f32, tau_s: f32, dt_s: f32) -> f32 {
    let a = (-dt_s / tau_s).exp();
    a.mul_add(prev, (1.0 - a) * x)
}
