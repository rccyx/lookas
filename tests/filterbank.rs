use lookas::filterbank::{build_filterbank, FilterbankParams};

const SR: f32 = 44_100.0;
const FFT: usize = 2048;
const FMIN: f32 = 30.0;
const FMAX: f32 = 16_000.0;

#[test]
fn filterbank_correct_band_count() {
    for bands in [8, 16, 32, 64] {
        let fb = build_filterbank(FilterbankParams {
            sr: SR,
            fft_size: FFT,
            bands,
            fmin: FMIN,
            fmax: FMAX,
        });
        assert_eq!(
            fb.len(),
            bands,
            "expected {bands} filters, got {}",
            fb.len()
        );
    }
}

#[test]
fn filterbank_center_frequencies_in_range() {
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });
    for tri in &fb {
        assert!(
            tri.center_hz >= FMIN * 0.9
                && tri.center_hz <= FMAX * 1.1,
            "center_hz {} out of expected range [{}, {}]",
            tri.center_hz,
            FMIN,
            FMAX
        );
    }
}

#[test]
fn filterbank_center_frequencies_monotone() {
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });
    let centers: Vec<f32> = fb.iter().map(|t| t.center_hz).collect();
    for w in centers.windows(2) {
        let (Some(&prev), Some(&curr)) = (w.first(), w.get(1)) else {
            continue;
        };
        assert!(
            curr > prev,
            "center frequencies should be monotonically increasing: {prev} -> {curr}",
        );
    }
}

// ---------------------------------------------------------------------------
// tap integrity
// ---------------------------------------------------------------------------

#[test]
fn filterbank_taps_have_valid_indices() {
    let half = FFT / 2;
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });
    for (i, tri) in fb.iter().enumerate() {
        for &(idx, _) in &tri.taps {
            assert!(
                idx < half,
                "filter {i}: tap index {idx} out of half-spectrum range [0, {half})"
            );
        }
    }
}

#[test]
fn filterbank_tap_weights_non_negative() {
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });
    for (i, tri) in fb.iter().enumerate() {
        for &(_, wgt) in &tri.taps {
            assert!(
                wgt >= 0.0,
                "filter {i}: negative tap weight {wgt}"
            );
        }
    }
}

#[test]
fn filterbank_tap_weights_normalised() {
    // Each filter is normalised so its tap weights sum to 1.0.
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });
    for (i, tri) in fb.iter().enumerate() {
        let sum: f32 = tri.taps.iter().map(|(_, w)| w).sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "filter {i}: tap weights sum to {sum}, expected ~1.0"
        );
    }
}

#[test]
fn filterbank_each_filter_has_taps() {
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });
    for (i, tri) in fb.iter().enumerate() {
        assert!(!tri.taps.is_empty(), "filter {i} has no taps");
    }
}

// ---------------------------------------------------------------------------
// edge cases
// ---------------------------------------------------------------------------

#[test]
fn filterbank_single_band() {
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 1,
        fmin: FMIN,
        fmax: FMAX,
    });
    assert_eq!(fb.len(), 1);
    assert!(
        fb.first().is_some_and(|tri| !tri.taps.is_empty()),
        "single band filter has no taps"
    );
}

#[test]
fn filterbank_different_fft_sizes() {
    for fft_size in [512, 1024, 2048, 4096] {
        let half = fft_size / 2;
        let fb = build_filterbank(FilterbankParams {
            sr: SR,
            fft_size,
            bands: 16,
            fmin: FMIN,
            fmax: FMAX,
        });
        assert_eq!(fb.len(), 16);
        for tri in &fb {
            for &(idx, _) in &tri.taps {
                assert!(
                    idx < half,
                    "fft_size={fft_size}: idx {idx} >= half {half}"
                );
            }
        }
    }
}

#[test]
fn filterbank_different_sample_rates() {
    for sr in [44_100.0f32, 48_000.0] {
        let fb = build_filterbank(FilterbankParams {
            sr,
            fft_size: FFT,
            bands: 24,
            fmin: FMIN,
            fmax: FMAX,
        });
        assert_eq!(fb.len(), 24, "sr={sr}");
    }
}

// ---------------------------------------------------------------------------
// energy pass-through sanity
// ---------------------------------------------------------------------------

#[test]
fn filterbank_flat_spectrum_produces_positive_output() {
    // A flat all-ones power spectrum should give positive accumulated energy
    // in every band.
    let half = FFT / 2;
    let flat = vec![1.0f32; half];
    let fb = build_filterbank(FilterbankParams {
        sr: SR,
        fft_size: FFT,
        bands: 32,
        fmin: FMIN,
        fmax: FMAX,
    });

    for (i, tri) in fb.iter().enumerate() {
        let acc: f32 = tri
            .taps
            .iter()
            .map(|&(idx, wgt)| {
                flat.get(idx).copied().unwrap_or(0.0) * wgt
            })
            .sum();
        assert!(
            acc > 0.0,
            "filter {i}: zero energy from flat spectrum"
        );
    }
}
