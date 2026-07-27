use anyhow::Result;
use crossterm::{cursor, queue};
use lookas::{
    analyzer::SpectrumAnalyzer,
    render::{Layout, draw_blocks_vertical, layout_for},
};
use std::io::Write;

pub struct FramePaint {
    lay: Layout,
    render: Vec<u8>,
    w: u16,
    h: u16,
    top_pad: u16,
}

impl FramePaint {
    pub fn new(w: u16, h: u16) -> Self {
        let top_pad: u16 = 0;

        #[allow(clippy::arithmetic_side_effects)]
        let frame_cap = (w as usize * h as usize * 4).max(64 * 1024);

        Self {
            lay: layout_for(w, h, top_pad),
            render: Vec::with_capacity(frame_cap),
            w,
            h,
            top_pad,
        }
    }

    pub const fn bars(&self) -> usize {
        self.lay.bars
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.w = w;
        self.h = h;
        self.lay = layout_for(self.w, self.h, self.top_pad);
    }

    pub fn draw<W: Write>(
        &mut self,
        analyzer: &mut SpectrumAnalyzer,
        out: &mut W,
    ) -> Result<()> {
        queue!(out, cursor::MoveTo(0, self.top_pad))?;
        self.render.clear();
        draw_blocks_vertical(
            &mut self.render,
            &analyzer.bars_y,
            self.w,
            self.h,
            &self.lay,
            &mut analyzer.render_fulls,
            &mut analyzer.render_fracs,
        )?;
        out.write_all(&self.render)?;
        out.flush()?;
        Ok(())
    }
}
