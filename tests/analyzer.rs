use lookas::analyzer::{FlowSpringParams, SpectrumAnalyzer};
use lookas::filterbank::build_filterbank;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn make_analyzer(bins: usize) -> SpectrumAnalyzer {
    SpectrumAnalyzer::new(bins)
}

const DT: f32 = 1.0 / 60.0; // 60 fps frame time
const TAU: f32 = 0.06; // default tau_spec from config

// ---------------------------------------------------------------------------
// update_spectrum -- asymmetric attack / release
// ---------------------------------------------------------------------------

#[test]
fn update_spectrum_instant_attack() {
    let mut sa = make_analyzer(4);
    sa.spec_pow_smooth = vec![0.01, 0.01, 0.01, 0.01];

    let high_pow = vec![1.0f32, 1.0, 1.0, 1.0];
    sa.update_spectrum(&high_pow, TAU, DT);

    for (i, &v) in sa.spec_pow_smooth.iter().enumerate() {
        assert!(
            (v - 1.0).abs() < 1e-6,
            "bin {i}: expected instant snap to 1.0, got {v}"
        );
    }
}

#[test]
fn update_spectrum_smooth_release() {
    let mut sa = make_analyzer(4);
    sa.spec_pow_smooth = vec![1.0, 1.0, 1.0, 1.0];

    let low_pow = vec![0.0f32; 4];
    sa.update_spectrum(&low_pow, TAU, DT);

    for (i, &v) in sa.spec_pow_smooth.iter().enumerate() {
        assert!(
            v > 1e-12 && v < 1.0,
            "bin {i}: EMA release should be between 0 and 1, got {v}"
        );
    }
}

// ---------------------------------------------------------------------------
// analyze_bands
// ---------------------------------------------------------------------------

fn make_analyzer_with_filters(
    sr: f32,
    fft_size: usize,
    bands: usize,
) -> SpectrumAnalyzer {
    use lookas::filterbank::FilterbankParams;
    let half = fft_size / 2;
    let mut sa = SpectrumAnalyzer::new(half);
    sa.filters = build_filterbank(FilterbankParams {
        sr,
        fft_size,
        bands,
        fmin: 30.0,
        fmax: 16_000.0,
    });
    sa.resize(bands);
    sa
}

#[test]
fn analyze_bands_updates_target_length() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 32);
    sa.analyze_bands(DT, true);
    assert_eq!(sa.bars_target.len(), 32);
}

#[test]
fn analyze_bands_gate_closed_sets_zeros() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 16);
    // pump some energy in
    sa.spec_pow_smooth.fill(1.0);
    sa.analyze_bands(DT, true);

    // now close the gate
    sa.analyze_bands(DT, false);
    for (i, &v) in sa.bars_target.iter().enumerate() {
        assert!(
            v.abs() < f32::EPSILON,
            "band {i} should be 0 when gate is closed, got {v}"
        );
    }
}

#[test]
fn analyze_bands_outputs_in_unit_range() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 24);
    sa.spec_pow_smooth.fill(0.01);

    for _ in 0..30 {
        sa.analyze_bands(DT, true);
        for (i, &v) in sa.bars_target.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&v),
                "band {i} target out of [0,1]: {v}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// apply_flow_and_spring
// ---------------------------------------------------------------------------

const fn default_params() -> FlowSpringParams {
    FlowSpringParams {
        flow_k: 0.18,
        spr_k: 60.0,
        spr_zeta: 1.0,
    }
}

#[test]
fn spring_moves_toward_target() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 4);
    // Manually set targets since we aren't pumping real FFT data here
    sa.bars_target = vec![1.0f32; 4];

    for _ in 0..200 {
        sa.apply_flow_and_spring(&default_params(), DT, true);
    }

    for (i, &y) in sa.bars_y.iter().enumerate() {
        assert!(
            y > 0.8,
            "bar {i} should converge toward 1.0 after 200 frames, got {y}"
        );
    }
}

#[test]
fn spring_gate_closed_decays_to_zero() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 4);
    sa.bars_y = vec![1.0; 4];
    sa.bars_target = vec![0.0f32; 4];

    for _ in 0..200 {
        sa.apply_flow_and_spring(
            &default_params(),
            DT,
            false, // gate closed
        );
    }

    for (i, &y) in sa.bars_y.iter().enumerate() {
        assert!(
            y < 0.01,
            "bar {i} should decay to ~0 when gate closed, got {y}"
        );
    }
}

#[test]
fn spring_critically_damped_no_overshoot() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 4);
    sa.bars_target = vec![0.5f32; 4];
    let mut max_y = 0.0f32;

    for _ in 0..300 {
        sa.apply_flow_and_spring(&default_params(), DT, true);
        for &y in &sa.bars_y {
            max_y = max_y.max(y);
        }
    }

    assert!(
        max_y < 0.52,
        "critically damped spring should not overshoot significantly: max_y = {max_y}"
    );
}
