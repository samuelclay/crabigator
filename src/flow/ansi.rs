//! A frame's cells as rows of text with 24-bit colour codes (flow's
//! `pi/ansi.ts`): every row exactly `columns` cells wide.

use super::cells::{Cells, DEFAULT_COLOR};

const RESET: &str = "\x1b[0m";

fn rgb(c: u32) -> String {
    format!("{};{};{}", (c >> 16) & 255, (c >> 8) & 255, c & 255)
}

pub fn grid_to_ansi(grid: &Cells) -> Vec<String> {
    let mut lines = Vec::with_capacity(grid.rows);
    for r in 0..grid.rows {
        let mut line = String::new();
        let (mut fg, mut bg) = (DEFAULT_COLOR, DEFAULT_COLOR);
        for x in 0..grid.columns {
            let i = r * grid.columns + x;
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
                    line.push_str(&format!("\x1b[38;2;{}m", rgb(f)));
                }
                if b != bg {
                    line.push_str(&format!("\x1b[48;2;{}m", rgb(b)));
                }
                fg = f;
                bg = b;
            }
            line.push(char::from_u32(grid.code_point(i)).unwrap_or(' '));
        }
        if fg != DEFAULT_COLOR || bg != DEFAULT_COLOR {
            line.push_str(RESET);
        }
        lines.push(line);
    }
    lines
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
