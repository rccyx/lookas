use anyhow::Result;
use crossterm::{
    cursor, event, execute, queue,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use lookas::{config::Config, utils::scopeguard};
use std::{
    io::{BufWriter, Write, stdout},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::rn::{
    frame::Frame,
    runtime::{
        InputAction, Runtime, RuntimeDiagnostics, StartupCapture,
    },
};

const CONFIG_WATCH_INTERVAL: Duration = Duration::from_millis(125);

pub fn run() -> Result<()> {
    let cfg = Config::load()?;

    let (color_tx, color_rx) = mpsc::channel();
    let initial_color = cfg.color;
    thread::spawn(move || {
        let mut current_color = initial_color;

        loop {
            thread::sleep(CONFIG_WATCH_INTERVAL);

            let Ok(next_cfg) = Config::load() else {
                continue;
            };

            if next_cfg.color != current_color {
                current_color = next_cfg.color;
                if color_tx.send(current_color).is_err() {
                    break;
                }
            }
        }
    });

    let mut out = BufWriter::with_capacity(1024 * 1024, stdout());
    terminal::enable_raw_mode()?;
    execute!(
        out,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All),
        SetForegroundColor(Color::Rgb {
            r: cfg.color.r,
            g: cfg.color.g,
            b: cfg.color.b,
        }),
    )?;
    out.flush()?;

    let _cleanup = scopeguard::guard((), |()| {
        let mut o = stdout();
        let _ = execute!(
            o,
            ResetColor,
            cursor::Show,
            terminal::LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    });

    let mut runtime = Runtime::new(&cfg)?;
    report_diagnostics(runtime.diagnostics());

    let (w, h) = terminal::size()?;
    let mut frame = Frame::new(&cfg, &runtime, w, h);
    let target_dt = Duration::from_millis(cfg.frame_ms);
    let mut last = Instant::now();

    loop {
        let mut layout_dirty = false;

        if event::poll(Duration::ZERO)? {
            match event::read()? {
                event::Event::Resize(nw, nh) => {
                    frame.resize(nw, nh);
                    layout_dirty = true;
                }
                event::Event::Key(k) => {
                    match runtime.handle_key(k.code)? {
                        InputAction::Quit => return Ok(()),
                        InputAction::AudioChanged => {
                            frame.clear_filters();
                        }
                        InputAction::Continue => {}
                    }
                    frame.reset_gate();
                }
                _ => {}
            }
        }

        let mut next_color = None;
        while let Ok(color) = color_rx.try_recv() {
            next_color = Some(color);
        }

        if let Some(color) = next_color {
            queue!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: color.r,
                    g: color.g,
                    b: color.b,
                }),
            )?;
            out.flush()?;
        }

        let now = Instant::now();
        let dt = now.duration_since(last);
        if dt < target_dt {
            if let Some(diff) = target_dt.checked_sub(dt) {
                thread::sleep(diff);
            }
        }

        let now = Instant::now();
        let dt_s = now.duration_since(last).as_secs_f32();
        last = now;

        if layout_dirty {
            queue!(out, terminal::Clear(ClearType::All),)?;
            out.flush()?;
        }

        frame.ensure_filterbank(&runtime);
        frame.set_delta(dt_s);
        frame.tick(&runtime, &mut out)?;
    }
}

fn report_diagnostics(diagnostics: &RuntimeDiagnostics) {
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