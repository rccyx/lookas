mod device;
mod mic;
mod stream;
mod system;

pub use device::{best_config_for, pick_input_device};
pub use stream::build_stream;
pub use system::SystemHandle;

use crate::buffer::SharedBuf;
use anyhow::Result;
use std::sync::{Arc, Mutex};

use mic::start_mic;
use system::start_system;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AudioMode {
    Mic,
    System,
    Both,
}

pub struct CaptureInfo {
    pub label: String,
    pub sample_rate: u32,
}

pub struct AudioController {
    mode: AudioMode,
    mic: Option<mic::MicHandle>,
    sys: Option<system::SystemHandle>,
    info: CaptureInfo,
}

impl Default for AudioController {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioController {
    #[must_use]
    pub fn new() -> Self {
        Self {
            mode: AudioMode::Mic,
            mic: None,
            sys: None,
            info: CaptureInfo {
                label: "mic".into(),
                sample_rate: 48_000,
            },
        }
    }

    #[must_use]
    pub const fn mode(&self) -> AudioMode {
        self.mode
    }

    #[must_use]
    pub const fn info(&self) -> &CaptureInfo {
        &self.info
    }

    pub fn start(
        &mut self,
        mode: AudioMode,
        mic_shared: Arc<Mutex<SharedBuf>>,
        sys_shared: Arc<Mutex<SharedBuf>>,
    ) -> Result<()> {
        self.stop();

        match mode {
            AudioMode::Mic => {
                let mic = start_mic(mic_shared)?;
                self.info = CaptureInfo {
                    label: mic.label.clone(),
                    sample_rate: mic.sample_rate,
                };
                self.mic = Some(mic);
                self.mode = mode;
                Ok(())
            }
            AudioMode::System => {
                let sys = start_system(sys_shared, 48_000)?;
                self.info = CaptureInfo {
                    label: sys.label.clone(),
                    sample_rate: sys.sample_rate,
                };
                self.sys = Some(sys);
                self.mode = mode;
                Ok(())
            }
            AudioMode::Both => {
                let mic = start_mic(mic_shared)?;
                let sys = start_system(sys_shared, mic.sample_rate)?;
                self.info = CaptureInfo {
                    label: format!("{} + {}", mic.label, sys.label),
                    sample_rate: mic.sample_rate,
                };
                self.mic = Some(mic);
                self.sys = Some(sys);
                self.mode = mode;
                Ok(())
            }
        }
    }

    pub fn reset(
        &mut self,
        mic_shared: Arc<Mutex<SharedBuf>>,
        sys_shared: Arc<Mutex<SharedBuf>>,
    ) -> Result<()> {
        let mode = self.mode;
        self.start(mode, mic_shared, sys_shared)
    }

    pub fn stop(&mut self) {
        self.sys.take();
        self.mic.take();
    }
}
