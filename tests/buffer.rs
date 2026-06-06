use lookas::SharedBuf;

// ---------------------------------------------------------------------------
// basic state
// ---------------------------------------------------------------------------

#[test]
fn new_buf_is_empty() {
    let buf = SharedBuf::new(64);
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn push_increments_len() {
    let mut buf = SharedBuf::new(64);
    for i in 1_u8..=10 {
        buf.push(f32::from(i));
        assert_eq!(buf.len(), usize::from(i));
    }
}

#[test]
fn len_caps_at_capacity_after_wraparound() {
    let cap = 8;
    let mut buf = SharedBuf::new(cap);
    for i in 0_u8..20 {
        buf.push(f32::from(i));
    }
    assert_eq!(
        buf.len(),
        cap,
        "len should cap at capacity once filled"
    );
}

// ---------------------------------------------------------------------------
// copy_last_n_into
// ---------------------------------------------------------------------------

#[test]
fn copy_last_n_returns_false_when_not_enough_data() {
    let mut buf = SharedBuf::new(64);
    buf.push(1.0);
    buf.push(2.0);
    let mut out = Vec::new();
    assert!(
        !buf.copy_last_n_into(10, &mut out),
        "should return false when fewer than n samples available"
    );
}

#[test]
fn copy_last_n_returns_correct_samples_no_wraparound() {
    let mut buf = SharedBuf::new(64);
    for i in 0_u8..10 {
        buf.push(f32::from(i));
    }
    let mut out = Vec::new();
    assert!(buf.copy_last_n_into(5, &mut out));
    assert_eq!(out, vec![5.0, 6.0, 7.0, 8.0, 9.0]);
}

#[test]
fn copy_last_n_handles_wraparound() {
    // Fill a small buffer past its capacity so the ring wraps.
    let cap = 8;
    let mut buf = SharedBuf::new(cap);
    for i in 0_u8..12 {
        buf.push(f32::from(i)); // last 8 pushed are 4..11
    }
    let mut out = Vec::new();
    assert!(buf.copy_last_n_into(cap, &mut out));
    assert_eq!(out.len(), cap);
    assert_eq!(out, vec![4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]);
}

#[test]
fn copy_last_n_zero_request() {
    let mut buf = SharedBuf::new(64);
    buf.push(1.0);
    let mut out = vec![99.0f32];
    assert!(buf.copy_last_n_into(0, &mut out));
    assert!(out.is_empty());
}

// ---------------------------------------------------------------------------
// latest
// ---------------------------------------------------------------------------

#[test]
fn latest_empty_buf_returns_empty_vec() {
    let buf = SharedBuf::new(16);
    assert!(buf.latest().is_empty());
}

#[test]
fn latest_returns_all_samples_before_fill() {
    let mut buf = SharedBuf::new(16);
    buf.push(1.0);
    buf.push(2.0);
    buf.push(3.0);
    let v = buf.latest();
    assert_eq!(v, vec![1.0, 2.0, 3.0]);
}

#[test]
fn latest_returns_full_ring_after_wraparound() {
    let cap = 4;
    let mut buf = SharedBuf::new(cap);
    for i in 0_u8..7 {
        buf.push(f32::from(i)); // last 4: 3, 4, 5, 6
    }
    let v = buf.latest();
    assert_eq!(v.len(), cap);
    assert_eq!(v, vec![3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn non_power_of_two_capacity_wraps_in_order() {
    let mut buf = SharedBuf::new(3);
    for i in 0_u8..6 {
        buf.push(f32::from(i));
    }

    let mut out = Vec::new();
    assert!(buf.copy_last_n_into(3, &mut out));
    assert_eq!(out, vec![3.0, 4.0, 5.0]);
}
