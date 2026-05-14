use super::{FlowSpringParams, SpectrumAnalyzer};

pub fn apply_flow_and_spring(
    sa: &mut SpectrumAnalyzer,
    params: &FlowSpringParams,
    dt_s: f32,
    gate_open: bool,
) {
    let n = sa.bars_y.len();
    if n == 0 {
        return;
    }

    if !gate_open {
        decay_to_silence(sa, n, dt_s);
        return;
    }

    diffuse_lateral_flow(sa, params, n);
    integrate_spring(sa, params, dt_s, n);
}

fn decay_to_silence(sa: &mut SpectrumAnalyzer, n: usize, dt_s: f32) {
    let tau_silence = 0.22f32;
    let a = (-dt_s / tau_silence).exp();

    for (y, v) in
        sa.bars_y.iter_mut().zip(sa.bars_v.iter_mut()).take(n)
    {
        *y *= a;
        if *y < 0.001 {
            *y = 0.0;
        }
        *v = 0.0;
    }
}

fn diffuse_lateral_flow(
    sa: &mut SpectrumAnalyzer,
    params: &FlowSpringParams,
    n: usize,
) {
    for i in 0..n {
        let cur = *sa.bars_y.get(i).unwrap_or(&0.0);
        let left = if i > 0 {
            *sa.bars_y.get(i.saturating_sub(1)).unwrap_or(&cur)
        } else {
            cur
        };
        let right = if i.saturating_add(1) < n {
            *sa.bars_y.get(i.saturating_add(1)).unwrap_or(&cur)
        } else {
            cur
        };
        let flow = params.flow_k * 2.0f32.mul_add(-cur, left + right);

        if let Some(scratch) = sa.flowed_scratch.get_mut(i) {
            *scratch = (*sa.bars_target.get(i).unwrap_or(&0.0)
                + flow)
                .clamp(0.0, 1.0);
        }
    }
}

fn integrate_spring(
    sa: &mut SpectrumAnalyzer,
    params: &FlowSpringParams,
    dt_s: f32,
    n: usize,
) {
    let c = 2.0 * params.spr_k.sqrt() * params.spr_zeta;

    for (y, (v, scratch)) in sa
        .bars_y
        .iter_mut()
        .zip(sa.bars_v.iter_mut().zip(sa.flowed_scratch.iter()))
        .take(n)
    {
        let a = params.spr_k.mul_add(*scratch - *y, -(c * *v));
        *v = a.mul_add(dt_s, *v);
        *y = (*v).mul_add(dt_s, *y).clamp(0.0, 1.0);
    }
}
