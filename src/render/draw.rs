use std::io::Write;

use super::{Layout, BAR_W, GAP_W};

// Pre-encoded UTF-8 byte sequences for each block character.
// Space is ASCII (1 byte); the 8 block elements are all 3-byte U+2580..U+2588.
// Entries that are 1-byte sequences store the byte in index 0; the write path
// selects the correct slice length via VBLOCKS_LEN.
const VBLOCKS_ENCODED: [[u8; 3]; 9] = [
    [b' ', 0, 0],       // ' '  U+0020  1 byte
    [0xE2, 0x96, 0x81], // '▁'  U+2581
    [0xE2, 0x96, 0x82], // '▂'  U+2582
    [0xE2, 0x96, 0x83], // '▃'  U+2583
    [0xE2, 0x96, 0x84], // '▄'  U+2584
    [0xE2, 0x96, 0x85], // '▅'  U+2585
    [0xE2, 0x96, 0x86], // '▆'  U+2586
    [0xE2, 0x96, 0x87], // '▇'  U+2587
    [0xE2, 0x96, 0x88], // '█'  U+2588
];

const VBLOCKS_LEN: [usize; 9] = [1, 3, 3, 3, 3, 3, 3, 3, 3];

// Index of the full-block character (U+2588) in the table.
const FULL_BLOCK: usize = 8;

#[inline]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn v_partial_idx(frac: f32) -> usize {
    let f = frac.clamp(0.0, 0.9999);
    f.mul_add(8.0, 0.5).floor() as usize
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
    fulls: &mut [usize],
    fracs: &mut [f32],
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
        .min(cols.checked_div(per).map_or(1, |v| v.max(1)))
        .min(fulls.len())
        .min(fracs.len());

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
            let idx = if row < f_val {
                FULL_BLOCK
            } else if row == f_val
                && fracs.get(i).copied().unwrap_or(0.0) > 0.0
            {
                v_partial_idx(fracs.get(i).copied().unwrap_or(0.0))
            } else {
                0
            };

            let enc = VBLOCKS_ENCODED
                .get(idx)
                .unwrap_or(&VBLOCKS_ENCODED[0]);
            let len = VBLOCKS_LEN.get(idx).copied().unwrap_or(1);
            let bytes = enc.get(..len).unwrap_or(enc.as_slice());
            for _ in 0..BAR_W {
                out.write_all(bytes)?;
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
