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
    // When incoming power is HIGHER than the smoothed value, the bin should
    // snap to the new value immediately (no EMA lag on the attack path).
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
    // Now when incoming power is LOWER, the EMA should run and the result should
    // be strictly between prev and incoming (not a snap).
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

#[test]
fn update_spectrum_release_decays_toward_target() {
    // after many frames with zero input the smoothed value should approach 0.
    let mut sa = make_analyzer(2);
    sa.spec_pow_smooth = vec![1.0, 1.0];
    let silence = vec![0.0f32; 2];

    for _ in 0..500 {
        sa.update_spectrum(&silence, TAU, DT);
    }

    for (i, &v) in sa.spec_pow_smooth.iter().enumerate() {
        assert!(
            v < 1e-3,
            "bin {i}: should decay close to zero after 500 frames, got {v}"
        );
    }
}

#[test]
fn update_spectrum_attack_then_release() {
    // simulate a drum hit, (burst of energy followed by silence).
    // The attack should snap instantly & the tail should decay smoothly.
    let mut sa = make_analyzer(1);
    sa.spec_pow_smooth = vec![0.0];

    // HIT
    sa.update_spectrum(&[1.0], TAU, DT);
    assert!(
        (sa.spec_pow_smooth[0] - 1.0).abs() < 1e-6,
        "attack did not snap: {}",
        sa.spec_pow_smooth[0]
    );

    // decay for a handful of frames
    let v_after_1 = {
        sa.update_spectrum(&[0.0], TAU, DT);
        sa.spec_pow_smooth[0]
    };
    let v_after_2 = {
        sa.update_spectrum(&[0.0], TAU, DT);
        sa.spec_pow_smooth[0]
    };

    assert!(v_after_1 < 1.0, "should start decaying: {v_after_1}");
    assert!(
        v_after_2 < v_after_1,
        "should continue decaying: {v_after_2}"
    );
}

#[test]
fn update_spectrum_equal_power_snaps() {
    // Incoming == current falls on the >= branch and should just hold value.
    let mut sa = make_analyzer(1);
    sa.spec_pow_smooth = vec![0.5];
    sa.update_spectrum(&[0.5], TAU, DT);
    assert!(
        (sa.spec_pow_smooth[0] - 0.5).abs() < 1e-6,
        "equal power should hold steady"
    );
}

#[test]
fn update_spectrum_floor_clamp() {
    // 0 input should be clamped to 1e-12 rather than causing log issues.
    let mut sa = make_analyzer(2);
    sa.spec_pow_smooth = vec![0.0, 0.0];
    sa.update_spectrum(&[0.0, 0.0], TAU, DT);
    for &v in &sa.spec_pow_smooth {
        assert!(
            v.is_finite() && v > 0.0,
            "expected finite positive floor, got {v}"
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
    let half = fft_size / 2;
    let mut sa = SpectrumAnalyzer::new(half);
    sa.filters =
        build_filterbank(sr, fft_size, bands, 30.0, 16_000.0);
    sa.resize(bands);
    sa
}

#[test]
fn analyze_bands_returns_correct_length() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 32);
    let targets = sa.analyze_bands(DT, true);
    assert_eq!(targets.len(), 32);
}

#[test]
fn analyze_bands_gate_closed_returns_zeros() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 16);
    // pump some energy in so eq_ref is warm.
    sa.spec_pow_smooth.fill(1.0);
    sa.analyze_bands(DT, true);

    // now close the gate -- targets must all be 0.
    let targets = sa.analyze_bands(DT, false);
    for (i, &v) in targets.iter().enumerate() {
        assert_eq!(
            v, 0.0,
            "band {i} should be 0 when gate is closed, got {v}"
        );
    }
}

#[test]
fn analyze_bands_outputs_in_unit_range() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 24);
    sa.spec_pow_smooth.fill(0.01);

    for _ in 0..30 {
        let targets = sa.analyze_bands(DT, true);
        for (i, &v) in targets.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&v),
                "band {i} target out of [0,1]: {v}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// update_db_range
// ---------------------------------------------------------------------------

#[test]
fn db_range_tracks_input_over_time() {
    let mut sa = make_analyzer(4);
    let input = vec![-40.0f32, -30.0, -20.0, -10.0];

    // After enough frames, db_low should approach the p10 value and
    // db_high should approach the p90 value.
    for _ in 0..300 {
        sa.update_db_range(&input, DT);
    }

    // p10 ~ -40, p90 ~ -10 (from a 4-element sorted array)
    assert!(
        sa.db_low < -30.0,
        "db_low should track toward lower values, got {}",
        sa.db_low
    );
    assert!(
        sa.db_high > -20.0,
        "db_high should track toward higher values, got {}",
        sa.db_high
    );
}

// ---------------------------------------------------------------------------
// apply_flow_and_spring
// ---------------------------------------------------------------------------

fn default_params() -> FlowSpringParams {
    FlowSpringParams {
        flow_k: 0.18,
        spr_k: 60.0,
        spr_zeta: 1.0, // critical damping
    }
}

#[test]
fn spring_moves_toward_target() {
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 4);
    let target = vec![1.0f32; 4];

    for _ in 0..200 {
        sa.apply_flow_and_spring(
            &target,
            &default_params(),
            DT,
            true,
        );
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
    let target = vec![0.0f32; 4];

    for _ in 0..200 {
        sa.apply_flow_and_spring(
            &target,
            &default_params(),
            DT,
            false,
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
fn spring_output_stays_in_unit_range() {
    // yes, even w/ extreme targets
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 8);
    let target = vec![999.0f32; 8]; // absurd target

    for _ in 0..10 {
        sa.apply_flow_and_spring(
            &target,
            &default_params(),
            DT,
            true,
        );
        for (i, &y) in sa.bars_y.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&y),
                "bar {i} out of [0,1]: {y}"
            );
        }
    }
}

#[test]
fn spring_critically_damped_no_overshoot() {
    // w/ zeta = 1.0 (critical damping) bars should not overshoot the target.
    let mut sa = make_analyzer_with_filters(44_100.0, 2048, 4);
    let target = vec![0.5f32; 4];
    let mut max_y = 0.0f32;

    for _ in 0..300 {
        sa.apply_flow_and_spring(
            &target,
            &default_params(),
            DT,
            true,
        );
        for &y in &sa.bars_y {
            max_y = max_y.max(y);
        }
    }

    // allow a very small overshoot margin due to discrete integration.
    assert!(
        max_y < 0.52,
        "critically damped spring should not overshoot significantly: max_y = {max_y}"
    );
}
