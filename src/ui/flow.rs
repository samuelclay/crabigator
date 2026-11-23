//! The flow column: an ambient scene from flow (github.com/robdmac/flow)
//! that moves with the session's work, drawn at the right of the widgets.
//!
//! The scene's frames come from the flow sidecar (`crate::flow`) as rows of
//! text with colour codes. Every row is fitted to the column's exact width
//! before it is written, because the column ends at the terminal's right
//! edge, where one extra cell would wrap the bottom line.

use std::io::Write;

use anyhow::Result;

use crate::flow::{Cells, DEFAULT_COLOR};
use crate::terminal::escape;

use super::utils::fit_ansi_to_width;

#[inline]
fn push_u8_bytes(buf: &mut Vec<u8>, n: u8) {
    if n >= 100 {
        buf.push(b'0' + n / 100);
        buf.push(b'0' + (n / 10) % 10);
        buf.push(b'0' + n % 10);
    } else if n >= 10 {
        buf.push(b'0' + n / 10);
        buf.push(b'0' + n % 10);
    } else {
        buf.push(b'0' + n);
    }
}

#[inline]
fn push_u16_bytes(buf: &mut Vec<u8>, mut n: u16) {
    let mut tmp = [0u8; 5];
    let mut len = 0;
    loop {
        tmp[len] = b'0' + (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for i in (0..len).rev() {
        buf.push(tmp[i]);
    }
}

#[inline]
fn push_rgb_params(buf: &mut Vec<u8>, c: u32) {
    push_u8_bytes(buf, ((c >> 16) & 255) as u8);
    buf.push(b';');
    push_u8_bytes(buf, ((c >> 8) & 255) as u8);
    buf.push(b';');
    push_u8_bytes(buf, (c & 255) as u8);
}

/// Emit cell-run differential ANSI directly from `cells` into `buf`.
///
/// Compares every cell `(code_point, fg, bg)` against `prev` (when provided
/// and matching dimensions) and emits cursor moves + combined truecolor SGR
/// only for changed cells. Tracks active `(fg, bg)` across runs so consecutive
/// changed cells with matching colors skip redundant SGR sequences.
pub(crate) fn write_flow_cells_diff(
    buf: &mut Vec<u8>,
    rect: FlowRect,
    cells: &Cells,
    prev: Option<&Cells>,
    sync_started: &mut bool,
) {
    let cols = rect.width as usize;
    let rows = rect.rows as usize;
    if cells.columns != cols || cells.rows != rows {
        return;
    }
    let prev_words = prev
        .filter(|p| p.columns == cols && p.rows == rows && p.words.len() == cells.words.len())
        .map(|p| p.words.as_slice());

    let base_row = rect.widget_pty_rows + 2; // 1-based first row below separator
    let base_col = rect.col + 1; // 1-based left column of flow
    let mut cur_fg = DEFAULT_COLOR;
    let mut cur_bg = DEFAULT_COLOR;
    let mut cur_row: Option<usize> = None;
    let mut cur_col: usize = 0;
    let mut utf8_buf = [0u8; 4];

    for r in 0..rows {
        let row_offset = r * cols;
        for x in 0..cols {
            let idx = (row_offset + x) * 3;
            let cp = cells.words[idx];
            let f = cells.words[idx + 1];
            let b = cells.words[idx + 2];
            if let Some(pw) = prev_words {
                if pw[idx] == cp && pw[idx + 1] == f && pw[idx + 2] == b {
                    continue;
                }
            }

            if !*sync_started {
                buf.extend_from_slice(escape::SYNC_BEGIN.as_bytes());
                *sync_started = true;
            }

            if cur_row != Some(r) || cur_col != x {
                if cur_row == Some(r) && x > cur_col {
                    // CUF (Cursor Forward): 3-5 bytes vs 9-10 bytes for CUP.
                    let delta = (x - cur_col) as u16;
                    buf.extend_from_slice(b"\x1b[");
                    if delta > 1 {
                        push_u16_bytes(buf, delta);
                    }
                    buf.push(b'C');
                } else {
                    buf.extend_from_slice(b"\x1b[");
                    push_u16_bytes(buf, base_row + r as u16);
                    buf.push(b';');
                    push_u16_bytes(buf, base_col + x as u16);
                    buf.push(b'H');
                }
                cur_row = Some(r);
                cur_col = x;
            }

            if f != cur_fg || b != cur_bg {
                if (f == DEFAULT_COLOR && cur_fg != DEFAULT_COLOR)
                    || (b == DEFAULT_COLOR && cur_bg != DEFAULT_COLOR)
                {
                    buf.extend_from_slice(escape::RESET.as_bytes());
                    cur_fg = DEFAULT_COLOR;
                    cur_bg = DEFAULT_COLOR;
                }
                let need_fg = f != cur_fg;
                let need_bg = b != cur_bg;
                if need_fg && need_bg {
                    buf.extend_from_slice(b"\x1b[38;2;");
                    push_rgb_params(buf, f);
                    buf.extend_from_slice(b";48;2;");
                    push_rgb_params(buf, b);
                    buf.push(b'm');
                } else if need_fg {
                    buf.extend_from_slice(b"\x1b[38;2;");
                    push_rgb_params(buf, f);
                    buf.push(b'm');
                } else if need_bg {
                    buf.extend_from_slice(b"\x1b[48;2;");
                    push_rgb_params(buf, b);
                    buf.push(b'm');
                }
                cur_fg = f;
                cur_bg = b;
            }

            let ch = char::from_u32(cp).unwrap_or(' ');
            buf.extend_from_slice(ch.encode_utf8(&mut utf8_buf).as_bytes());
            cur_col += 1;
        }
    }

    if cur_fg != DEFAULT_COLOR || cur_bg != DEFAULT_COLOR {
        buf.extend_from_slice(escape::RESET.as_bytes());
    }
}

/// Repaint only the changed cells of the flow column from its `Cells` grid,
/// in a single buffered write with zero intermediate `String` allocations.
pub(crate) fn draw_flow_cells_diff(
    stdout: &mut dyn Write,
    buf: &mut Vec<u8>,
    rect: FlowRect,
    cells: &Cells,
    last_cells: &mut Option<(FlowRect, Cells)>,
    cursor_position: Option<(u16, u16)>,
    child_attrs: &[u8],
) -> Result<()> {
    buf.clear();
    let mut sync_started = false;
    let prev = last_cells
        .as_ref()
        .filter(|(r, _)| *r == rect)
        .map(|(_, c)| c);
    write_flow_cells_diff(buf, rect, cells, prev, &mut sync_started);

    // Update cached cells in place without reallocating when dimensions match.
    match last_cells {
        Some((r, c)) if *r == rect && c.words.len() == cells.words.len() => {
            c.columns = cells.columns;
            c.rows = cells.rows;
            c.words.copy_from_slice(&cells.words);
        }
        slot => {
            *slot = Some((rect, cells.clone()));
        }
    }

    if !sync_started {
        return Ok(());
    }
    if let Some((row, col)) = cursor_position {
        buf.extend_from_slice(b"\x1b[");
        push_u16_bytes(buf, row + 1);
        buf.push(b';');
        push_u16_bytes(buf, col + 1);
        buf.push(b'H');
    }
    buf.extend_from_slice(child_attrs);
    buf.extend_from_slice(escape::SYNC_END.as_bytes());
    stdout.write_all(buf)?;
    stdout.flush()?;
    Ok(())
}

/// What the status bar shows of the flow column, and the label on the
/// separator above it.
#[derive(Clone, Copy, Debug)]
pub enum FlowView<'a> {
    /// The frame's rows, under the scene's name and the key that cycles scenes.
    Scene { rows: &'a [String], scene: &'a str },
    /// Off: no column, only the label naming the key that turns it on.
    Off,
}

/// The key that cycles scenes, Settings and Off, as the label shows it.
pub const NEXT_SCENE_KEY: &str = "⌃]";

/// The scene and shortcut, or just the shortcut when the name will not fit.
pub fn flow_label(view: FlowView, width: u16) -> String {
    let scene = match view {
        FlowView::Scene { scene, .. } => scene,
        FlowView::Off => "flow off",
    };
    let full = format!(" {scene}  {NEXT_SCENE_KEY} ");
    if full.chars().count() < usize::from(width) {
        full
    } else {
        format!(" {NEXT_SCENE_KEY} ")
    }
}

/// Write the label into the separator row above the column, right-aligned.
pub fn draw_flow_label(
    stdout: &mut dyn Write,
    separator_row: u16,
    total_cols: u16,
    width: u16,
    view: FlowView,
) -> Result<()> {
    let label = flow_label(view, width);
    let len = label.chars().count() as u16;
    if len + 1 > width {
        return Ok(());
    }
    write!(
        stdout,
        "{}{}{}{}{}",
        escape::cursor_to(separator_row, total_cols - len),
        escape::bg(escape::color::BG_DARK),
        escape::fg(escape::color::GRAY),
        label,
        escape::RESET
    )?;
    Ok(())
}

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

/// Format one flow row to exact `width` cells, skipping `fit_ansi_to_width`
/// when the caller guarantees the row came from `grid_to_ansi` at that exact width.
pub(crate) fn format_flow_row_text(row: Option<&str>, width: usize, exact_width: bool) -> String {
    match row {
        Some(text) if exact_width => text.to_string(),
        Some(text) => fit_ansi_to_width(text, width),
        None => format!("{}{:width$}", escape::RESET, "", width = width),
    }
}

/// Repaint only the flow column, for a new frame between full status-bar draws.
///
/// The cursor goes back to where the assistant left it (`cursor_position`,
/// 0-based, from the vt100 parser), and `child_attrs` restores the assistant's
/// text style, which the column's colour resets would otherwise clear.
#[cfg(test)]
pub fn draw_flow_column(
    stdout: &mut dyn Write,
    rect: FlowRect,
    rows: &[String],
    cursor_position: Option<(u16, u16)>,
    child_attrs: &[u8],
) -> Result<()> {
    let mut buf = Vec::with_capacity(usize::from(rect.rows) * usize::from(rect.width) * 10 + 64);
    write!(buf, "{}", escape::SYNC_BEGIN)?;
    for row in 1..=rect.rows {
        write!(
            buf,
            "{}{}",
            escape::cursor_to(rect.widget_pty_rows + 1 + row, rect.col + 1),
            fit_ansi_to_width(
                rows.get(row as usize - 1).map_or("", String::as_str),
                rect.width as usize
            )
        )?;
    }
    if let Some((row, col)) = cursor_position {
        write!(buf, "{}", escape::cursor_to(row + 1, col + 1))?;
    }
    buf.extend_from_slice(child_attrs);
    write!(buf, "{}", escape::SYNC_END)?;
    stdout.write_all(&buf)?;
    stdout.flush()?;
    Ok(())
}

/// Repaint only the changed rows of the flow column into a reusable byte buffer,
/// writing the entire synchronized update in a single `write_all` call.
///
/// `prev_rows` holds the last-painted exact-width strings for `rect` and is
/// updated in place. When `exact_width` is true, non-empty rows from `rows`
/// are written directly without re-parsing through `fit_ansi_to_width`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_flow_column_diff(
    stdout: &mut dyn Write,
    buf: &mut Vec<u8>,
    rect: FlowRect,
    rows: &[String],
    exact_width: bool,
    prev_rows: &mut Vec<String>,
    cursor_position: Option<(u16, u16)>,
    child_attrs: &[u8],
) -> Result<()> {
    let row_count = rect.rows as usize;
    let width = rect.width as usize;
    let col = rect.col + 1;
    let base_row = rect.widget_pty_rows + 1;

    buf.clear();
    let mut any_changed = false;
    let mut prev_matches_len = prev_rows.len() == row_count;
    if !prev_matches_len {
        prev_rows.clear();
        prev_rows.resize(row_count, String::new());
    }

    for idx in 0..row_count {
        let raw = rows.get(idx).map(String::as_str);
        // Fast path: when exact_width is true and a row is present, compare
        // directly against `prev_rows[idx]` with zero allocations.
        if exact_width {
            if let Some(text) = raw {
                if prev_matches_len && prev_rows[idx] == text {
                    continue;
                }
                if !any_changed {
                    buf.extend_from_slice(escape::SYNC_BEGIN.as_bytes());
                    any_changed = true;
                }
                write!(buf, "\x1b[{};{}H", base_row + idx as u16 + 1, col)?;
                buf.extend_from_slice(text.as_bytes());
                prev_rows[idx].clear();
                prev_rows[idx].push_str(text);
                continue;
            }
        }

        let formatted = format_flow_row_text(raw, width, false);
        if prev_matches_len && prev_rows[idx] == formatted {
            continue;
        }
        if !any_changed {
            buf.extend_from_slice(escape::SYNC_BEGIN.as_bytes());
            any_changed = true;
        }
        write!(buf, "\x1b[{};{}H", base_row + idx as u16 + 1, col)?;
        buf.extend_from_slice(formatted.as_bytes());
        prev_rows[idx] = formatted;
    }

    // Mark length valid for subsequent diffs even if every row was skipped.
    let _ = &mut prev_matches_len;

    if !any_changed {
        return Ok(());
    }
    if let Some((row, col)) = cursor_position {
        write!(buf, "\x1b[{};{}H", row + 1, col + 1)?;
    }
    buf.extend_from_slice(child_attrs);
    buf.extend_from_slice(escape::SYNC_END.as_bytes());
    stdout.write_all(buf)?;
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
    fn the_label_names_the_scene_and_the_key_or_just_the_key() {
        let scene = |scene| FlowView::Scene { rows: &[], scene };
        assert_eq!(flow_label(scene("surf"), 18), " surf  ⌃] ");
        assert_eq!(flow_label(scene("engine"), 15), " engine  ⌃] ");
        assert_eq!(flow_label(scene("starship"), 17), " starship  ⌃] ");
        assert_eq!(flow_label(scene("starship"), 12), " ⌃] ");
        assert_eq!(flow_label(FlowView::Off, 15), " flow off  ⌃] ");
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
