use lookas::dsp::{a_weighting, ema_tc, hann, hz_to_mel, mel_to_hz};

fn to_db(linear: f32) -> f32 {
    20.0 * linear.max(1e-12).log10()
}

#[test]
fn a_weighting_reference_values() {
    let cases: &[(f32, f32, f32)] = &[
        //  Hz     expected dB  tol
        (125.0, -16.1, 1.0),
        (250.0, -8.6, 1.0),
        (500.0, -3.2, 1.0),
        (1_000.0, 0.0, 0.1),
        (2_000.0, 1.2, 1.0),
        (4_000.0, 1.0, 1.0),
        (8_000.0, -1.1, 1.0),
        (16_000.0, -6.6, 1.5),
    ];

    for &(hz, expected_db, tol) in cases {
        let got_db = to_db(a_weighting(hz));
        assert!(
            (got_db - expected_db).abs() < tol,
            "a_weighting({hz} Hz): got {got_db:.2} dB, expected {expected_db:.2} dB (tol {tol})"
        );
    }
}

#[test]
fn a_weighting_1khz_is_unity() {
    // the curve is normalised so 1 kHz returns ~1.0 (0 dB).
    let w = a_weighting(1_000.0);
    assert!(
        (w - 1.0).abs() < 0.01,
        "a_weighting(1000) = {w}, expected ~1.0"
    );
}

#[test]
fn a_weighting_peaks_near_3_to_4_khz() {
    // the human ear is most sensitive around 3-4 kHz so weighting should be
    // above 1.0 there and above all adjacent octaves.
    let w_3k = a_weighting(3_150.0);
    let w_1k = a_weighting(1_000.0);
    let w_8k = a_weighting(8_000.0);
    assert!(
        w_3k > w_1k,
        "3.15 kHz should have higher weight than 1 kHz"
    );
    assert!(
        w_3k > w_8k,
        "3.15 kHz should have higher weight than 8 kHz"
    );
}

#[test]
fn a_weighting_monotone_rolloff_in_bass() {
    // below the peak, weight should fall as frequency drops.
    let freqs = [500.0f32, 250.0, 125.0, 63.0, 31.5];
    let weights: Vec<f32> =
        freqs.iter().map(|&f| a_weighting(f)).collect();
    for w in weights.windows(2) {
        assert!(
            w[0] > w[1],
            "expected monotone rolloff: a_weighting({}) = {} should be > a_weighting({}) = {}",
            freqs[0], w[0], freqs[1], w[1]
        );
    }
}

#[test]
fn a_weighting_floor_clamp() {
    // Values below 10 Hz are clamped to 10 Hz, result should be finite
    // and non-negative, not NaN/inf.
    let w = a_weighting(0.0);
    assert!(w.is_finite(), "a_weighting(0) should be finite");
    assert!(w >= 0.0, "a_weighting(0) should be non-negative");
}

// ---------------------------------------------------------------------------
// ema_tc
// ---------------------------------------------------------------------------

#[test]
fn ema_tc_converges_to_target() {
    let mut v = 0.0f32;
    let tau = 0.1;
    let dt = 0.01;
    // After many steps the EMA should approach the target closely.
    for _ in 0..1000 {
        v = ema_tc(v, 1.0, tau, dt);
    }
    assert!((v - 1.0).abs() < 1e-3, "EMA did not converge: got {v}");
}

#[test]
fn ema_tc_slow_tau_barely_moves() {
    // tau >> dt means the coefficient a = exp(-dt/tau) is close to 1,
    // so the output barely changes in one step.
    let prev = 0.0f32;
    let result = ema_tc(prev, 1.0, 10.0, 0.001); // tau = 10 s, dt = 1 ms
    assert!(
        result < 0.01,
        "with tau >> dt, EMA should barely move: got {result}"
    );
}

#[test]
fn ema_tc_fast_tau_snaps_quickly() {
    // tau << dt means the output almost equals the target after one step.
    let prev = 0.0f32;
    let result = ema_tc(prev, 1.0, 0.0001, 0.016); // tau = 0.1 ms, dt = 16 ms
    assert!(
        result > 0.99,
        "with tau << dt, EMA should snap to target: got {result}"
    );
}

#[test]
fn ema_tc_output_between_prev_and_target() {
    // Output must always be strictly between prev and target (or equal).
    let result = ema_tc(0.0, 1.0, 0.05, 0.016);
    assert!((0.0..=1.0).contains(&result));

    let result2 = ema_tc(1.0, 0.0, 0.05, 0.016);
    assert!((0.0..=1.0).contains(&result2));
}

// ---------------------------------------------------------------------------
// hann
// ---------------------------------------------------------------------------

#[test]
fn hann_length() {
    for n in [16, 64, 512, 2048] {
        assert_eq!(hann(n).len(), n);
    }
}

#[test]
fn hann_endpoints_are_zero() {
    let w = hann(1024);
    assert!(w[0].abs() < 1e-6, "hann[0] should be ~0, got {}", w[0]);
    assert!(
        w[1023].abs() < 1e-4,
        "hann[N-1] should be ~0, got {}",
        w[1023]
    );
}

#[test]
fn hann_peak_near_centre() {
    let n = 1024;
    let w = hann(n);
    let mid = n / 2;
    assert!(
        (w[mid] - 1.0).abs() < 1e-4,
        "hann centre should be ~1.0, got {}",
        w[mid]
    );
    assert!(w[mid] >= w[mid - 1]);
    assert!(w[mid] >= w[mid + 1]);
}

#[test]
fn hann_values_in_range() {
    for v in hann(512) {
        assert!(
            (0.0..=1.0).contains(&v),
            "hann value out of [0, 1]: {v}"
        );
    }
}

// ---------------------------------------------------------------------------
// hz_to_mel / mel_to_hz roundtrip
// ---------------------------------------------------------------------------

#[test]
fn mel_hz_roundtrip() {
    let freqs = [50.0f32, 200.0, 500.0, 1_000.0, 4_000.0, 12_000.0];
    for &hz in &freqs {
        let roundtripped = mel_to_hz(hz_to_mel(hz));
        assert!(
            (roundtripped - hz).abs() / hz < 1e-4,
            "roundtrip failed for {hz} Hz: got {roundtripped}"
        );
    }
}

#[test]
fn mel_scale_is_monotone() {
    let freqs = [100.0f32, 500.0, 1_000.0, 4_000.0, 10_000.0];
    let mels: Vec<f32> =
        freqs.iter().map(|&f| hz_to_mel(f)).collect();
    for m in mels.windows(2) {
        assert!(
            m[1] > m[0],
            "mel scale should be monotonically increasing"
        );
    }
}
