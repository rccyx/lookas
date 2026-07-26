use anyhow::Result;
use crossterm::event::{self, Event};
use std::time::Duration;

use crate::app::rn::{
    frame::Frame,
    runtime::{InputAction, Runtime},
};

pub enum TerminalAction {
    Continue,
    Quit,
    Refresh,
}

pub struct TerminalEventContext<'a> {
    pub runtime: &'a mut Runtime,
    pub frame: &'a mut Frame,
}

pub fn handle_terminal_event(
    ctx: &mut TerminalEventContext<'_>,
) -> Result<TerminalAction> {
    if !event::poll(Duration::ZERO)? {
        return Ok(TerminalAction::Continue);
    }

    match event::read()? {
        Event::Resize(w, h) => {
            ctx.frame.resize(w, h);
            return Ok(TerminalAction::Refresh);
        }
        Event::Key(key) => {
            return handle_key_event(ctx, key.code);
        }
        _ => {}
    }

    Ok(TerminalAction::Continue)
}

fn handle_key_event(
    ctx: &mut TerminalEventContext<'_>,
    code: event::KeyCode,
) -> Result<TerminalAction> {
    match ctx.runtime.handle_key(code)? {
        InputAction::Quit => return Ok(TerminalAction::Quit),
        InputAction::AudioChanged => {
            ctx.frame.clear_filters();
        }
        InputAction::Continue => {}
    }

    ctx.frame.reset_gate();
    Ok(TerminalAction::Continue)
}
