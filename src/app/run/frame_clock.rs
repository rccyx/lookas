use std::{
    thread,
    time::{Duration, Instant},
};

pub struct FrameClock {
    target_dt: Duration,
    last: Instant,
}

impl FrameClock {
    #[must_use]
    pub fn new(frame_ms: u64) -> Self {
        Self {
            target_dt: Duration::from_millis(frame_ms),
            last: Instant::now(),
        }
    }

    pub fn set_frame_ms(&mut self, frame_ms: u64) {
        self.target_dt = Duration::from_millis(frame_ms);
    }

    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last);

        if let Some(diff) = self.target_dt.checked_sub(dt) {
            thread::sleep(diff);
        }

        let now = Instant::now();
        let dt_s = now.duration_since(self.last).as_secs_f32();
        self.last = now;

        dt_s
    }
}
