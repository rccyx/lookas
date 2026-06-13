#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(target_os = "linux"))]
mod stub;

#[cfg(target_os = "linux")]
pub use linux::{SystemHandle, start_system};

#[cfg(not(target_os = "linux"))]
pub use stub::{SystemHandle, start_system};
