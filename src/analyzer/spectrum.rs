use crate::dsp::ema_tc;

use super::SpectrumAnalyzer;

pub fn update_spectrum(
    sa: &mut SpectrumAnalyzer,
    spec_pow: &[f32],
    tau_spec: f32,
    dt_s: f32,
) {
    let len = sa.spec_pow_smooth.len().min(spec_pow.len());
    for (i, &pow) in spec_pow.iter().enumerate().take(len) {
        let incoming = pow.max(1e-12);
        if let Some(prev_val) = sa.spec_pow_smooth.get_mut(i) {
            let prev = *prev_val;
            *prev_val = if incoming >= prev {
                incoming
            } else {
                ema_tc(prev, incoming, tau_spec, dt_s)
            };
        }
    }
}
