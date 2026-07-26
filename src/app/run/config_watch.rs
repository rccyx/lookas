use lookas::config::{Config, RgbColor};
use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

const CONFIG_WATCH_INTERVAL: Duration = Duration::from_millis(125);

pub struct ColorWatch {
    rx: Receiver<RgbColor>,
}

impl ColorWatch {
    #[must_use]
    pub fn spawn(initial_color: RgbColor) -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut current_color = initial_color;

            loop {
                thread::sleep(CONFIG_WATCH_INTERVAL);

                let Ok(next_cfg) = Config::load() else {
                    continue;
                };

                if next_cfg.color == current_color {
                    continue;
                }

                current_color = next_cfg.color;
                if tx.send(current_color).is_err() {
                    break;
                }
            }
        });

        Self { rx }
    }

    pub fn latest(&self) -> Option<RgbColor> {
        let mut next_color = None;

        while let Ok(color) = self.rx.try_recv() {
            next_color = Some(color);
        }

        next_color
    }
}
