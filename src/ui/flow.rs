//! The flow column: an ambient scene from flow (github.com/robdmac/flow)
//! that moves with the session's work, drawn at the right of the widgets.
//!
//! The scene's frames come from the flow sidecar (`crate::flow`) as rows of
//! text with colour codes. Every row is fitted to the column's exact width
//! before it is written, because the column ends at the terminal's right
//! edge, where one extra cell would wrap the bottom line.

use std::io::Write;

use anyhow::Result;

use crate::terminal::escape;

use super::utils::fit_ansi_to_width;
use super::WidgetArea;

/// Where the flow column sits on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowRect {
    /// Rows above the widget separator (the assistant's rows plus the handoff strip).
    pub widget_pty_rows: u16,
    /// First column of the scene, 0-based (the separator sits just left of it).
    pub col: u16,
    pub width: u16,
    /// Scene rows, below the widget separator.
    pub rows: u16,
}

impl FlowRect {
    fn area(self, row: u16) -> WidgetArea {
        WidgetArea {
            pty_rows: self.widget_pty_rows,
            col: self.col,
            row,
            width: self.width,
            height: self.rows + 1,
        }
    }
}

/// Draw one row of the scene (1-based `area.row`); no row yet draws blank.
pub fn draw_flow_row(stdout: &mut dyn Write, area: WidgetArea, row: Option<&str>) -> Result<()> {
    write!(
        stdout,
        "{}{}",
        escape::cursor_to(area.pty_rows + 1 + area.row, area.col + 1),
        fit_ansi_to_width(row.unwrap_or(""), area.width as usize)
    )?;
    Ok(())
}

/// Repaint only the flow column, for a new frame between full status-bar draws.
///
/// The cursor goes back to where the assistant left it (`cursor_position`,
/// 0-based, from the vt100 parser), and `child_attrs` restores the assistant's
/// text style, which the column's colour resets would otherwise clear.
pub fn draw_flow_column(
    stdout: &mut dyn Write,
    rect: FlowRect,
    rows: &[String],
    cursor_position: Option<(u16, u16)>,
    child_attrs: &[u8],
) -> Result<()> {
    write!(stdout, "{}", escape::SYNC_BEGIN)?;
    for row in 1..=rect.rows {
        draw_flow_row(
            stdout,
            rect.area(row),
            rows.get(row as usize - 1).map(String::as_str),
        )?;
    }
    if let Some((row, col)) = cursor_position {
        write!(stdout, "{}", escape::cursor_to(row + 1, col + 1))?;
    }
    stdout.write_all(child_attrs)?;
    write!(stdout, "{}", escape::SYNC_END)?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> FlowRect {
        FlowRect {
            widget_pty_rows: 30,
            col: 102,
            width: 18,
            rows: 3,
        }
    }

    #[test]
    fn column_draw_is_synchronized_positions_each_row_and_restores_the_cursor() {
        let mut out = Vec::new();
        let rows = vec!["\x1b[38;2;200;80;10m▓▓".to_string(), "x".repeat(40)];
        draw_flow_column(&mut out, rect(), &rows, Some((4, 7)), b"\x1b[1m").unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with(escape::SYNC_BEGIN));
        assert!(text.ends_with(&format!("\x1b[1m{}", escape::SYNC_END)));
        // Rows land below the separator (row 31) at column 103, 1-based.
        assert!(text.contains(&escape::cursor_to(32, 103)));
        assert!(text.contains(&escape::cursor_to(33, 103)));
        assert!(text.contains(&escape::cursor_to(34, 103)));
        // The long row is clipped to the width; the missing third row is blank.
        assert!(text.contains(&format!("{}{}", "x".repeat(18), escape::RESET)));
        assert!(text.contains(&format!(
            "{}{}{}",
            escape::cursor_to(34, 103),
            escape::RESET,
            " ".repeat(18)
        )));
        // The cursor goes back where the assistant left it.
        assert!(text.contains(&format!("{}\x1b[1m", escape::cursor_to(5, 8))));
    }

    #[test]
    fn a_frame_replayed_on_a_terminal_fills_the_column_exactly() {
        let mut parser = vt100::Parser::new(40, 120, 0);
        let mut out = Vec::new();
        let rows: Vec<String> = (0..3).map(|_| "░".repeat(18)).collect();
        draw_flow_column(&mut out, rect(), &rows, Some((0, 0)), b"").unwrap();
        parser.process(&out);
        let screen = parser.screen();
        for r in 31..34 {
            assert_eq!(screen.cell(r, 101).unwrap().contents(), "");
            for c in 102..120 {
                assert_eq!(
                    screen.cell(r, c).unwrap().contents(),
                    "░",
                    "row {r} col {c}"
                );
            }
        }
        assert_eq!(screen.cursor_position(), (0, 0));
    }
}
