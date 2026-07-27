use anyhow::Result;
use crossterm::terminal;
use lookas::config::Config;

mod config_watch;
mod diagnostics;
mod frame_clock;
mod terminal_event;
mod terminal_session;

use super::runtime::{Frame, Runtime};
use config_watch::ConfigWatch;
use diagnostics::report_runtime_diagnostics;
use frame_clock::FrameClock;
use terminal_event::{
    TerminalAction, TerminalEventContext, handle_terminal_event,
};
use terminal_session::TerminalSession;

pub fn run() -> Result<()> {
    let cfg = Config::load()?;
    let config_watch = ConfigWatch::spawn(cfg.clone());
    let mut terminal = TerminalSession::enter(cfg.color)?;
    let mut runtime = Runtime::new(&cfg)?;
    report_runtime_diagnostics(runtime.diagnostics());

    let (w, h) = terminal::size()?;
    let mut frame = Frame::new(&cfg, &runtime, w, h);
    let mut clock = FrameClock::new(cfg.frame_ms);

    loop {
        let mut event_ctx = TerminalEventContext {
            runtime: &mut runtime,
            frame: &mut frame,
        };

        match handle_terminal_event(&mut event_ctx)? {
            TerminalAction::Quit => return Ok(()),
            TerminalAction::Refresh => {
                terminal.clear()?;
            }
            TerminalAction::Continue => {}
        }

        if let Some(cfg) = config_watch.latest()? {
            terminal.set_color(cfg.color)?;
            runtime.set_fft_size(cfg.fft_size);
            frame.apply_config(&cfg, &runtime);
            clock.set_frame_ms(cfg.frame_ms);
        }

        frame.set_delta(clock.tick());
        frame.ensure_filterbank(&runtime);
        frame.tick(&runtime, terminal.writer())?;
    }
}
