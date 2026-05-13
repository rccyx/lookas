use anyhow::Result;

pub struct SystemHandle {
    pub(in crate::audio) label: String,
    pub(in crate::audio) sample_rate: u32,
}

pub fn start_system(
    _shared: std::sync::Arc<
        std::sync::Mutex<crate::buffer::SharedBuf>,
    >,
    _rate: u32,
) -> Result<SystemHandle> {
    anyhow::bail!("system audio capture is only supported on Linux")
}
