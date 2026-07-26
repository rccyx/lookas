use anyhow::Result;
use crossterm::{
    cursor, execute, queue,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use lookas::config::RgbColor;
use std::io::{BufWriter, Stdout, Write, stdout};

pub struct TerminalSession {
    out: BufWriter<Stdout>,
}

impl TerminalSession {
    pub fn enter(color: RgbColor) -> Result<Self> {
        let mut out = BufWriter::with_capacity(1024 * 1024, stdout());

        terminal::enable_raw_mode()?;
        execute!(
            out,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            terminal_color(color),
        )?;
        out.flush()?;

        Ok(Self { out })
    }

    pub fn writer(&mut self) -> &mut BufWriter<Stdout> {
        &mut self.out
    }

    pub fn set_color(&mut self, color: RgbColor) -> Result<()> {
        queue!(self.out, terminal_color(color),)?;
        self.out.flush()?;

        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        queue!(self.out, terminal::Clear(ClearType::All),)?;
        self.out.flush()?;

        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = execute!(
            out,
            ResetColor,
            cursor::Show,
            terminal::LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

const fn terminal_color(color: RgbColor) -> SetForegroundColor {
    SetForegroundColor(Color::Rgb {
        r: color.r,
        g: color.g,
        b: color.b,
    })
}
