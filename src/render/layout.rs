use super::{BAR_W, GAP_W};

pub struct Layout {
    pub bars: usize,
    pub left_pad: u16,
    pub right_pad: u16,
    pub top_pad: u16,
}

#[inline]
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects
)]
pub fn layout_for(w: u16, _h: u16, top_pad: u16) -> Layout {
    let left_pad = 1u16;
    let right_pad = 2u16;
    let usable_cols = w.saturating_sub(left_pad + right_pad);

    let per = (BAR_W + GAP_W) as u16;
    let bars = usable_cols
        .checked_div(per)
        .map_or(1, |v| (v as usize).max(1));

    Layout {
        bars,
        left_pad,
        right_pad,
        top_pad,
    }
}
