#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(target_os = "linux"))]
mod stub;

#[cfg(target_os = "linux")]
pub use linux::{start_system, SystemHandle};

#[cfg(not(target_os = "linux"))]
pub use stub::{start_system, SystemHandle};
