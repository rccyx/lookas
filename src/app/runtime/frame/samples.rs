use lookas::audio::AudioMode;

use super::Runtime;

pub struct FrameSamples {
    mic_tail: Vec<f32>,
    sys_tail: Vec<f32>,
    mix: Vec<f32>,
}

struct AudioReady {
    mic: bool,
    system: bool,
}

impl FrameSamples {
    pub fn new(fft_size: usize) -> Self {
        Self {
            mic_tail: Vec::with_capacity(fft_size),
            sys_tail: Vec::with_capacity(fft_size),
            mix: vec![0.0f32; fft_size],
        }
    }

    pub fn len(&self) -> usize {
        self.mix.len()
    }

    pub fn mix(&self) -> &[f32] {
        &self.mix
    }

    pub fn resize(&mut self, fft_size: usize) {
        self.mic_tail = Vec::with_capacity(fft_size);
        self.sys_tail = Vec::with_capacity(fft_size);
        self.mix = vec![0.0; fft_size];
    }

    pub fn prepare(&mut self, runtime: &Runtime) -> bool {
        let ready = self.copy_tails(runtime);

        match runtime.mode() {
            AudioMode::Mic => self.copy_mic(ready.mic),
            AudioMode::System => self.copy_system(ready.system),
            AudioMode::Both => {
                self.mix_samples(runtime.fft_size(), &ready)
            }
        }
    }

    fn copy_tails(&mut self, runtime: &Runtime) -> AudioReady {
        AudioReady {
            mic: runtime.copy_mic_tail(&mut self.mic_tail),
            system: runtime.copy_system_tail(&mut self.sys_tail),
        }
    }

    fn copy_mic(&mut self, ready: bool) -> bool {
        if ready {
            self.mix.copy_from_slice(&self.mic_tail);
        }
        ready
    }

    fn copy_system(&mut self, ready: bool) -> bool {
        if ready {
            self.mix.copy_from_slice(&self.sys_tail);
        }
        ready
    }

    fn mix_samples(
        &mut self,
        fft_size: usize,
        ready: &AudioReady,
    ) -> bool {
        if !ready.mic || !ready.system {
            return false;
        }

        #[allow(clippy::indexing_slicing)]
        for i in 0..fft_size {
            self.mix[i] = (self.mic_tail[i] + self.sys_tail[i]) * 0.5;
        }

        true
    }
}
