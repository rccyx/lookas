use crate::app::runtime::{RuntimeDiagnostics, StartupCapture};

pub fn report_runtime_diagnostics(diagnostics: &RuntimeDiagnostics) {
    match &diagnostics.startup_capture {
        StartupCapture::System => {}
        StartupCapture::MicFallback { system_error } => {
            eprintln!(
                "[lookas] system capture failed: {system_error}"
            );
            eprintln!("[lookas] fallback active: using mic");
        }
    }
}
