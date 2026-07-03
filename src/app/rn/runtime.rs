use anyhow::Result;
use crossterm::event::KeyCode;
use lookas::{
    audio::{AudioController, AudioError, AudioMode},
    buffer::SharedBuf,
    config::Config,
};
use std::sync::{Arc, Mutex};

use super::super::input::{InputContext, handle_key};

pub enum InputAction {
    Continue,
    Quit,
    AudioChanged,
}

pub enum StartupCapture {
    System,
    MicFallback { system_error: AudioError },
}

pub struct RuntimeDiagnostics {
    pub startup_capture: StartupCapture,
}

pub struct Runtime {
    fft_size: usize,
    audio: AudioController,
    mic_shared: Arc<Mutex<SharedBuf>>,
    sys_shared: Arc<Mutex<SharedBuf>>,
    sr: f32,
    cap: usize,
    sr_u32: u32,
    diagnostics: RuntimeDiagnostics,
}

impl Runtime {
    pub fn new(cfg: &Config) -> Result<Self> {
        let fft_size = cfg.fft_size;
        let cap = ring_cap(fft_size);
        let mic_shared = Arc::new(Mutex::new(SharedBuf::new(cap)));
        let sys_shared = Arc::new(Mutex::new(SharedBuf::new(cap)));

        let mut audio = AudioController::new();
        let startup_capture = match audio.start(
            AudioMode::System,
            mic_shared.clone(),
            sys_shared.clone(),
        ) {
            Ok(()) => StartupCapture::System,
            Err(system_error) => {
                audio.start(
                    AudioMode::Mic,
                    mic_shared.clone(),
                    sys_shared.clone(),
                )?;

                StartupCapture::MicFallback { system_error }
            }
        };

        let diagnostics = RuntimeDiagnostics { startup_capture };

        let sr_u32 = audio.info().sample_rate;
        #[allow(clippy::cast_precision_loss)]
        let sr = sr_u32 as f32;

        Ok(Self {
            fft_size,
            audio,
            mic_shared,
            sys_shared,
            sr,
            cap,
            sr_u32,
            diagnostics,
        })
    }

    pub fn handle_key(
        &mut self,
        code: KeyCode,
    ) -> Result<InputAction> {
        let mut ctx = InputContext {
            audio: &mut self.audio,
            mic_shared: &self.mic_shared,
            sys_shared: &self.sys_shared,
            ring_cap: self.cap,
        };

        if handle_key(code, &mut ctx)? {
            return Ok(InputAction::Quit);
        }

        if self.update_sample_rate() {
            return Ok(InputAction::AudioChanged);
        }

        Ok(InputAction::Continue)
    }

    pub const fn fft_size(&self) -> usize {
        self.fft_size
    }

    pub const fn sample_rate(&self) -> f32 {
        self.sr
    }

    pub const fn mode(&self) -> AudioMode {
        self.audio.mode()
    }

    pub const fn diagnostics(&self) -> &RuntimeDiagnostics {
        &self.diagnostics
    }

    pub fn copy_mic_tail(&self, tail: &mut Vec<f32>) -> bool {
        self.mic_shared
            .try_lock()
            .ok()
            .is_some_and(|b| b.copy_last_n_into(self.fft_size, tail))
    }

    pub fn copy_system_tail(&self, tail: &mut Vec<f32>) -> bool {
        self.sys_shared
            .try_lock()
            .ok()
            .is_some_and(|b| b.copy_last_n_into(self.fft_size, tail))
    }

    fn update_sample_rate(&mut self) -> bool {
        let new_sr = self.audio.info().sample_rate;
        if new_sr == self.sr_u32 {
            return false;
        }

        self.sr_u32 = new_sr;
        #[allow(clippy::cast_precision_loss)]
        {
            self.sr = self.sr_u32 as f32;
        }
        true
    }
}

#[allow(clippy::arithmetic_side_effects)]
fn ring_cap(fft_size: usize) -> usize {
    ((48_000usize / 10).max(fft_size * 3))
        .max(fft_size * 6)
        .next_power_of_two()
}
