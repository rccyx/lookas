#[inline]
#[must_use]
pub fn ema_tc(prev: f32, x: f32, tau_s: f32, dt_s: f32) -> f32 {
    let a = (-dt_s / tau_s).exp();
    a.mul_add(prev, (1.0 - a) * x)
}

#[inline]
#[must_use]
pub fn ema_precomputed(prev: f32, x: f32, alpha: f32) -> f32 {
    alpha.mul_add(prev, (1.0 - alpha) * x)
}
