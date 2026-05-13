use std::io::Write;

use super::{Layout, BAR_W, GAP_W};

const VBLOCKS: [char; 9] =
    [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn v_partial(frac: f32) -> char {
    let f = frac.clamp(0.0, 0.9999);
    let idx = f.mul_add(8.0, 0.5).floor() as usize;
    *VBLOCKS.get(idx.min(8)).unwrap_or(&' ')
}

#[inline]
fn write_spaces<W: Write>(
    out: &mut W,
    mut n: usize,
) -> std::io::Result<()> {
    const BLANK: [u8; 64] = [b' '; 64];
    while n >= BLANK.len() {
        out.write_all(&BLANK)?;
        n = n.saturating_sub(BLANK.len());
    }
    if n > 0 {
        if let Some(slice) = BLANK.get(..n) {
            out.write_all(slice)?;
        }
    }
    Ok(())
}

#[inline]
#[allow(
    clippy::too_many_arguments,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    clippy::cognitive_complexity
)]
pub fn draw_blocks_vertical<W: Write>(
    out: &mut W,
    bars: &[f32],
    w: u16,
    h: u16,
    lay: &Layout,
) -> std::io::Result<()> {
    let rows = h.saturating_sub(lay.top_pad) as usize;
    let cols = w
        .saturating_sub(lay.left_pad.saturating_add(lay.right_pad))
        as usize;
    if rows == 0 || cols == 0 {
        return Ok(());
    }

    let per = BAR_W + GAP_W;
    let n = bars
        .len()
        .min(cols.checked_div(per).map_or(1, |v| v.max(1)));

    let mut fulls = vec![0usize; n];
    let mut fracs = vec![0f32; n];
    for i in 0..n {
        let height =
            bars.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0)
                * rows as f32;
        if let Some(f) = fulls.get_mut(i) {
            *f = height.floor() as usize;
        }
        if let Some(fr) = fracs.get_mut(i) {
            *fr =
                height - (fulls.get(i).copied().unwrap_or(0) as f32);
        }
    }

    for y in 0..rows {
        let row = rows.saturating_sub(1).saturating_sub(y);
        write_spaces(out, lay.left_pad as usize)?;

        for i in 0..n {
            let f_val = fulls.get(i).copied().unwrap_or(0);
            let ch = if row < f_val {
                '█'
            } else if row == f_val
                && fracs.get(i).copied().unwrap_or(0.0) > 0.0
            {
                v_partial(fracs.get(i).copied().unwrap_or(0.0))
            } else {
                ' '
            };

            for _ in 0..BAR_W {
                out.write_all(
                    ch.encode_utf8(&mut [0; 4]).as_bytes(),
                )?;
            }
            write_spaces(out, GAP_W)?;
        }

        let used = n * per;
        if cols > used {
            write_spaces(out, cols.saturating_sub(used))?;
        }
        write_spaces(out, lay.right_pad as usize)?;

        if y.saturating_add(1) < rows {
            out.write_all(b"\r\n")?;
        }
    }

    Ok(())
}
