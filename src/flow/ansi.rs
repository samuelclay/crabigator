//! A frame's cells as rows of text with 24-bit colour codes (flow's
//! `pi/ansi.ts`): every row exactly `columns` cells wide.

use super::cells::{Cells, DEFAULT_COLOR};

const RESET: &str = "\x1b[0m";

#[inline]
fn push_u8(out: &mut String, n: u8) {
    if n >= 100 {
        out.push((b'0' + n / 100) as char);
        out.push((b'0' + (n / 10) % 10) as char);
        out.push((b'0' + n % 10) as char);
    } else if n >= 10 {
        out.push((b'0' + n / 10) as char);
        out.push((b'0' + n % 10) as char);
    } else {
        out.push((b'0' + n) as char);
    }
}

#[inline]
fn push_rgb_sgr(out: &mut String, prefix: &str, c: u32) {
    out.push_str(prefix);
    push_u8(out, ((c >> 16) & 255) as u8);
    out.push(';');
    push_u8(out, ((c >> 8) & 255) as u8);
    out.push(';');
    push_u8(out, (c & 255) as u8);
    out.push('m');
}

#[allow(dead_code)]
pub fn grid_to_ansi(grid: &Cells) -> Vec<String> {
    let mut lines = Vec::with_capacity(grid.rows);
    grid_to_ansi_into(grid, &mut lines);
    lines
}

/// Format `grid` into `lines`, reusing existing row `String` allocations across frames.
pub fn grid_to_ansi_into(grid: &Cells, lines: &mut Vec<String>) {
    lines.truncate(grid.rows);
    let est_cap = grid.columns.saturating_mul(16);
    while lines.len() < grid.rows {
        lines.push(String::with_capacity(est_cap));
    }
    for r in 0..grid.rows {
        let line = &mut lines[r];
        line.clear();
        let (mut fg, mut bg) = (DEFAULT_COLOR, DEFAULT_COLOR);
        let row_start = r * grid.columns;
        for x in 0..grid.columns {
            let i = row_start + x;
            let (f, b) = (grid.foreground(i), grid.background(i));
            if f != fg || b != bg {
                // Going back to a terminal default needs a reset, then the other side again.
                if (f == DEFAULT_COLOR && fg != DEFAULT_COLOR)
                    || (b == DEFAULT_COLOR && bg != DEFAULT_COLOR)
                {
                    line.push_str(RESET);
                    fg = DEFAULT_COLOR;
                    bg = DEFAULT_COLOR;
                }
                if f != fg {
                    push_rgb_sgr(line, "\x1b[38;2;", f);
                }
                if b != bg {
                    push_rgb_sgr(line, "\x1b[48;2;", b);
                }
                fg = f;
                bg = b;
            }
            line.push(char::from_u32(grid.code_point(i)).unwrap_or(' '));
        }
        if fg != DEFAULT_COLOR || bg != DEFAULT_COLOR {
            line.push_str(RESET);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::utils::strip_ansi_len;

    #[test]
    fn rows_are_exactly_the_grid_width() {
        let mut grid = Cells::new(5, 2);
        for i in 0..10 {
            grid.blank(i);
        }
        grid.set(1, 0x2593, 0xff0000, DEFAULT_COLOR);
        grid.set(2, 0x2580, 0x00ff00, 0x0000ff);
        let rows = grid_to_ansi(&grid);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(strip_ansi_len(row), 5);
        }
        assert!(rows[0].contains("\x1b[38;2;255;0;0m▓"));
        assert!(rows[0].contains("\x1b[48;2;0;0;255m"));
        assert_eq!(rows[1], "     ");
    }
}
