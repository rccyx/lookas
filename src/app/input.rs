use anyhow::Result;
use crossterm::event::KeyCode;
use lookas::{
    audio::{AudioController, AudioMode},
    buffer::SharedBuf,
};
use std::sync::{Arc, Mutex};

pub struct InputContext<'a> {
    pub audio: &'a mut AudioController,
    pub mic_shared: &'a Arc<Mutex<SharedBuf>>,
    pub sys_shared: &'a Arc<Mutex<SharedBuf>>,
    pub ring_cap: usize,
}

pub fn handle_key(
    code: KeyCode,
    ctx: &mut InputContext<'_>,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('1') => switch_mode(AudioMode::Mic, ctx)?,
        KeyCode::Char('2') => switch_mode(AudioMode::System, ctx)?,
        KeyCode::Char('3') => switch_mode(AudioMode::Both, ctx)?,
        KeyCode::Char('r') => {
            reset_bufs(ctx);
            ctx.audio.reset(
                ctx.mic_shared.clone(),
                ctx.sys_shared.clone(),
            )?;
        }
        _ => {}
    }
    Ok(false)
}

fn switch_mode(
    mode: AudioMode,
    ctx: &mut InputContext<'_>,
) -> Result<()> {
    reset_bufs(ctx);
    ctx.audio.start(
        mode,
        ctx.mic_shared.clone(),
        ctx.sys_shared.clone(),
    )
}

fn reset_bufs(ctx: &InputContext<'_>) {
    reset_one(ctx.mic_shared, ctx.ring_cap);
    reset_one(ctx.sys_shared, ctx.ring_cap);
}

fn reset_one(shared: &Arc<Mutex<SharedBuf>>, cap: usize) {
    if let Ok(mut b) = shared.lock() {
        *b = SharedBuf::new(cap);
    }
}
