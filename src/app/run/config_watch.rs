use anyhow::Result;
use lookas::config::Config;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

const CONFIG_WATCH_INTERVAL: Duration = Duration::from_millis(125);

pub struct ConfigWatch {
    rx: Receiver<Result<Config>>,
}

impl ConfigWatch {
    #[must_use]
    pub fn spawn(initial_cfg: Config) -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut current_cfg = initial_cfg;

            loop {
                thread::sleep(CONFIG_WATCH_INTERVAL);

                match Config::load() {
                    Ok(next_cfg) => {
                        if next_cfg == current_cfg {
                            continue;
                        }

                        current_cfg = next_cfg.clone();
                        if tx.send(Ok(next_cfg)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        break;
                    }
                }
            }
        });

        Self { rx }
    }

    pub fn latest(&self) -> Result<Option<Config>> {
        let mut latest = None;

        loop {
            match self.rx.try_recv() {
                Ok(Ok(cfg)) => latest = Some(cfg),
                Ok(Err(error)) => return Err(error),
                Err(TryRecvError::Empty) => return Ok(latest),
                Err(TryRecvError::Disconnected) => {
                    anyhow::bail!("config watcher stopped");
                }
            }
        }
    }
}
