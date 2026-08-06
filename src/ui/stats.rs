//! Stats widget - displays session statistics
//!
//! Shows session state, duration, messages, tool calls, compressions, and cloud status.

use std::io::{Stdout, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use super::sparkline::render_sparkline;
use super::time::{format_duration_compact, format_elapsed_age};
use super::utils::strip_ansi_len;
use super::{WidgetArea, COMPLETION_ICON, PROMPT_ICON};
use crate::cloud::CloudStatus;
use crate::hooks::SessionStats;
use crate::platforms::SessionState;
use crate::terminal::escape::{self, color, fg, RESET};

/// Braille spinner frames for the thinking animation
const THROBBER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const CORE_STATS_ROWS: u16 = 7;
const COMPACT_STATS_ROWS: u16 = 4;

/// Get current throbber frame index based on time (for hash comparison)
pub fn throbber_frame_index() -> usize {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    (millis / 100) as usize % THROBBER.len()
}

/// Get throbber character for the current time
fn throbber_char() -> char {
    THROBBER[throbber_frame_index()]
}

/// Calculate idle seconds from idle_since timestamp
fn idle_seconds(idle_since: Option<f64>) -> Option<u64> {
    let since = idle_since?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let secs = (now - since) as u64;
    if secs >= 60 {
        Some(secs)
    } else {
        None
    }
}

/// Format elapsed time since a timestamp
fn format_elapsed(timestamp: Option<f64>) -> String {
    format_elapsed_age(timestamp)
        .map(|age| format!(" {}", age))
        .unwrap_or_default()
}

/// Format the state indicator for the header row
fn format_state_indicator(state: SessionState) -> String {
    match state {
        SessionState::Ready => {
            format!("{}○ Ready{}", fg(color::GRAY), RESET)
        }
        SessionState::Thinking => {
            format!("{}{}{}", fg(color::GREEN), throbber_char(), RESET)
        }
        SessionState::Permission => {
            format!("{}» ? «{} Perm", fg(color::YELLOW), RESET)
        }
        SessionState::Question => {
            format!("{}» ? «{} Ask", fg(color::ORANGE), RESET)
        }
        SessionState::Complete => {
            format!("{}✓ Complete{}", fg(color::PURPLE), RESET)
        }
        SessionState::Interrupted => {
            format!("{}⊘ Interrupted{}", fg(color::RED), RESET)
        }
    }
}

/// Number of content rows the stats widget would render in normal mode,
/// including the header. The Idle row only appears when the session is in
/// an idle state and the idle timer has crossed the 60-second threshold.
fn stats_natural_rows(stats: &SessionStats) -> u16 {
    let is_idle_state = matches!(
        stats.effective_state(),
        SessionState::Complete | SessionState::Question | SessionState::Interrupted
    );
    if is_idle_state && idle_seconds(stats.platform_stats.idle_since).is_some() {
        CORE_STATS_ROWS + 1
    } else {
        CORE_STATS_ROWS
    }
}

/// Use the compact grid whenever the normal stats list would lose a row.
pub fn stats_use_compact_layout(available_rows: u16) -> bool {
    available_rows < CORE_STATS_ROWS
}

/// Number of rows stats will use at the current height.
pub fn stats_render_rows(available_rows: u16, stats: &SessionStats) -> u16 {
    if stats_use_compact_layout(available_rows) {
        COMPACT_STATS_ROWS
    } else {
        stats_natural_rows(stats)
    }
}

/// Draw the stats widget at the given position
pub fn draw_stats_widget(
    stdout: &mut Stdout,
    area: WidgetArea,
    stats: &SessionStats,
    cloud_status: Option<&CloudStatus>,
    is_paired: bool,
    pairing_code: Option<&str>,
) -> Result<()> {
    write!(
        stdout,
        "{}",
        escape::cursor_to(area.pty_rows + 1 + area.row, area.col + 1)
    )?;
    // 1-col left margin so content doesn't sit flush against the separator/edge.
    write!(stdout, " ")?;

    // `area.height` includes the separator above the widget data. Switch to the
    // compact grid as soon as the normal list would be clipped, not only at the
    // minimum terminal height.
    let compact = stats_use_compact_layout(area.height.saturating_sub(1));
    let inner_width = area.width.saturating_sub(2);

    let content = if compact {
        draw_compact_row(
            area.row,
            inner_width,
            stats,
            cloud_status,
            is_paired,
            pairing_code,
        )
    } else {
        draw_normal_row(
            area.row,
            inner_width,
            stats,
            cloud_status,
            is_paired,
            pairing_code,
        )
    };

    write!(stdout, "{}", content)?;
    let content_len = strip_ansi_len(&content);
    let pad = (inner_width as usize).saturating_sub(content_len);
    write!(stdout, "{:pad$}", "", pad = pad)?;
    // 1-col right margin.
    write!(stdout, " ")?;

    Ok(())
}

/// Format cloud status as header text.
///
/// No leading space — the widget wrapper applies a uniform 1-col left margin,
/// so an extra space here would push the label in by 2 cols instead of 1.
fn format_cloud_header(cloud_status: Option<&CloudStatus>, is_paired: bool) -> String {
    match cloud_status {
        Some(status) if status.connected => {
            if is_paired {
                // In debug builds, show first 8 chars of session ID
                #[cfg(debug_assertions)]
                if let Some(ref session_id) = status.session_id {
                    let short_id = &session_id[..8.min(session_id.len())];
                    return format!(
                        "{}Streaming {}{}{}",
                        fg(color::GREEN),
                        fg(color::CYAN),
                        short_id,
                        RESET
                    );
                }
                format!("{}Streaming{}", fg(color::GREEN), RESET)
            } else {
                format!("{}Waiting to pair{}", fg(color::YELLOW), RESET)
            }
        }
        Some(status) if status.reconnect_attempts > 0 => {
            format!("{}Retry{}", fg(color::ORANGE), RESET)
        }
        Some(_) => {
            format!("{}Offline{}", fg(color::RED), RESET)
        }
        None => {
            format!("{}Local{}", fg(color::GRAY), RESET)
        }
    }
}

/// Build the dim "· <pair-text>" suffix shown next to the streaming label.
/// The leading " · " is rendered outside the OSC hyperlink (per design — the
/// link should only cover the meaningful pair text), and three width tiers
/// degrade gracefully:
///   - **Full**:      ` · Pair: ABC-DEF-GHI` (preferred)
///   - **No label**:  ` · ABC-DEF-GHI`        (drops the "Pair: " prefix)
///   - **Truncated**: ` · ABC-DEF…`           (drops trailing characters,
///     ellipsis on the last cell — the URL still carries the full code)
///
/// Returns `Some((visible_width, ansi_string))` for the widest tier that
/// fits in `available`, or `None` if not even the truncated tier (with at
/// least 2 visible code chars) can fit.
fn build_pair_suffix(code: &str, available: usize) -> Option<(usize, String)> {
    if code.is_empty() {
        return None;
    }
    let url = format!("https://drinkcrabigator.com/dashboard?setup={}", code);
    let dim = fg(color::DARK_GRAY);
    // " · " is 3 visible columns and is rendered *outside* the hyperlink so
    // clicking only triggers on the pair text itself.
    let prefix_visible: usize = 3;
    let prefix = format!(" {}·{} ", dim, RESET);

    let code_chars: Vec<char> = code.chars().collect();
    let code_len = code_chars.len();

    let make = |link_text: String, visible_text_width: usize| -> (usize, String) {
        let inner = format!("{}{}{}", dim, link_text, RESET);
        let linked = escape::hyperlink(&url, &inner);
        let visible_width = prefix_visible + visible_text_width;
        let ansi = format!("{}{}", prefix, linked);
        (visible_width, ansi)
    };

    // Tier 1: " · Pair: <code>"
    let label = "Pair: ";
    let full_text_width = label.chars().count() + code_len;
    if prefix_visible + full_text_width <= available {
        let link_text = format!("{}{}", label, code);
        return Some(make(link_text, full_text_width));
    }

    // Tier 2: " · <code>"
    if prefix_visible + code_len <= available {
        return Some(make(code.to_string(), code_len));
    }

    // Tier 3: " · <truncated…>" — keep at least 2 code chars + ellipsis.
    let max_visible_code = available.saturating_sub(prefix_visible);
    if max_visible_code < 3 {
        return None;
    }
    // Reserve the last cell for the ellipsis; the URL still carries the full
    // code so a click takes the user to the right setup link.
    let keep = max_visible_code - 1;
    let truncated: String = code_chars.iter().take(keep).collect::<String>() + "…";
    let truncated_width = truncated.chars().count();
    Some(make(truncated, truncated_width))
}

/// Build the full left-side header content: cloud status + optional pair-code
/// suffix sized to whatever room is left between the cloud label and the
/// right-aligned state indicator (with 2 cols of breathing room reserved).
fn build_header_left(
    cloud_status: Option<&CloudStatus>,
    is_paired: bool,
    pairing_code: Option<&str>,
    width: usize,
    state_width: usize,
) -> (usize, String) {
    let cloud = format_cloud_header(cloud_status, is_paired);
    let cloud_width = strip_ansi_len(&cloud);
    let Some(code) = pairing_code else {
        return (cloud_width, cloud);
    };
    let available_for_pair = width
        .saturating_sub(cloud_width)
        .saturating_sub(state_width)
        .saturating_sub(2);
    if let Some((pair_width, pair_text)) = build_pair_suffix(code, available_for_pair) {
        (cloud_width + pair_width, format!("{}{}", cloud, pair_text))
    } else {
        (cloud_width, cloud)
    }
}

fn compact_metric_cell(labels: &[String], value: &str, width: usize) -> String {
    let value_len = strip_ansi_len(value);
    let value_gap = usize::from(!value.is_empty());
    let label = labels
        .iter()
        .find(|label| strip_ansi_len(label) + value_gap + value_len <= width)
        .unwrap_or_else(|| labels.last().expect("compact metric needs a label"));
    let label_len = strip_ansi_len(label);
    let gap = if value.is_empty() {
        width.saturating_sub(label_len)
    } else {
        width.saturating_sub(label_len + value_len).max(1)
    };

    format!("{}{:gap$}{}", label, "", value, gap = gap)
}

fn compact_elapsed(timestamp: Option<f64>) -> String {
    format_elapsed_age(timestamp)
        .map(|elapsed| format!("{}{}{}", fg(color::GRAY), elapsed, RESET))
        .unwrap_or_default()
}

fn compact_count_labels(
    icon: &str,
    full_label: &str,
    short_label: &str,
    count: u32,
    count_color: u8,
) -> [String; 3] {
    let label = |text: &str| {
        let space = if text.is_empty() { "" } else { " " };
        format!(
            "{}{icon}{space}{text}{} {}{count}{}",
            fg(color::GRAY),
            RESET,
            fg(count_color),
            RESET,
        )
    };

    [label(full_label), label(short_label), label("")]
}

fn compact_tools_cell(stats: &SessionStats, width: usize) -> String {
    let full_label = format!("{}⚒ Tools{}", fg(color::GRAY), RESET);
    let short_label = format!("{}⚒{}", fg(color::GRAY), RESET);
    let label = if strip_ansi_len(&full_label) + 2 <= width {
        full_label
    } else {
        short_label
    };
    let label_len = strip_ansi_len(&label);
    let gap = usize::from(label_len < width);
    let sparkline_width = width.saturating_sub(label_len + gap);
    let bins = stats.tool_usage_bins(sparkline_width);
    let sparkline = render_sparkline(&bins, sparkline_width);

    format!("{}{:gap$}{}", label, "", sparkline, gap = gap)
}

fn compact_columns(left: String, right: String, width: usize) -> String {
    let content_width = width.saturating_sub(1);
    let left_width = content_width / 2;
    let right_width = content_width.saturating_sub(left_width);
    let left_len = strip_ansi_len(&left);
    let right_len = strip_ansi_len(&right);

    format!(
        "{}{:left_pad$}{}│{}{}{:right_pad$}",
        left,
        "",
        fg(color::DARK_GRAY),
        RESET,
        right,
        "",
        left_pad = left_width.saturating_sub(left_len),
        right_pad = right_width.saturating_sub(right_len),
    )
}

/// Draw a row in compact mode (two metrics per row, each with label and value)
fn draw_compact_row(
    row: u16,
    width: u16,
    stats: &SessionStats,
    cloud_status: Option<&CloudStatus>,
    is_paired: bool,
    pairing_code: Option<&str>,
) -> String {
    let content_width = (width as usize).saturating_sub(1);
    let left_width = content_width / 2;
    let right_width = content_width.saturating_sub(left_width);

    match row {
        1 => {
            // Header: cloud status (+ optional pair code) on left, state on right
            let state = format_state_indicator(stats.effective_state());
            let state_len = strip_ansi_len(&state);
            let (header_len, header) = build_header_left(
                cloud_status,
                is_paired,
                pairing_code,
                width as usize,
                state_len,
            );
            let gap = (width as usize).saturating_sub(header_len + state_len);
            format!("{}{:gap$}{}", header, "", state, gap = gap)
        }
        2 => {
            let session_labels = [
                format!("{}◉ Session{}", fg(color::GRAY), RESET),
                format!("{}◉ Sess{}", fg(color::GRAY), RESET),
                format!("{}◉{}", fg(color::GRAY), RESET),
            ];
            let session_value = format!("{}{}{}", fg(color::BLUE), stats.format_work(), RESET);
            let session = compact_metric_cell(&session_labels, &session_value, left_width);

            let thinking_val = stats.format_thinking().unwrap_or_else(|| "—".to_string());
            let thinking_labels = [
                format!("{}◐ Thinking{}", fg(color::GRAY), RESET),
                format!("{}◐ Thnk{}", fg(color::GRAY), RESET),
                format!("{}◐{}", fg(color::GRAY), RESET),
            ];
            let thinking_value = format!("{}{}{}", fg(color::GREEN), thinking_val, RESET);
            let thinking = compact_metric_cell(&thinking_labels, &thinking_value, right_width);

            compact_columns(session, thinking, width as usize)
        }
        3 => {
            let prompt_labels = compact_count_labels(
                PROMPT_ICON,
                "Prompts",
                "Pmt",
                stats.platform_stats.prompts,
                color::LIGHT_BLUE,
            );
            let prompts = compact_metric_cell(
                &prompt_labels,
                &compact_elapsed(stats.prompts_changed_at),
                left_width,
            );
            let completion_labels = compact_count_labels(
                COMPLETION_ICON,
                "Completions",
                "Fin",
                stats.platform_stats.completions,
                color::LIGHT_BLUE,
            );
            let completions = compact_metric_cell(
                &completion_labels,
                &compact_elapsed(stats.completions_changed_at),
                right_width,
            );

            compact_columns(prompts, completions, width as usize)
        }
        4 => {
            let tools = compact_tools_cell(stats, left_width);
            let compaction_labels = compact_count_labels(
                "⊜",
                "Compactions",
                "Cmp",
                stats.platform_stats.compressions,
                color::PINK,
            );
            let compactions = compact_metric_cell(
                &compaction_labels,
                &compact_elapsed(stats.compressions_changed_at),
                right_width,
            );

            compact_columns(tools, compactions, width as usize)
        }
        _ => String::new(),
    }
}

/// Draw a row in normal mode (full labels, single column)
fn draw_normal_row(
    row: u16,
    width: u16,
    stats: &SessionStats,
    cloud_status: Option<&CloudStatus>,
    is_paired: bool,
    pairing_code: Option<&str>,
) -> String {
    match row {
        1 => {
            // Header: cloud status (+ optional pair code) on left, state on right
            let state = format_state_indicator(stats.effective_state());
            let state_len = strip_ansi_len(&state);
            let (header_len, header) = build_header_left(
                cloud_status,
                is_paired,
                pairing_code,
                width as usize,
                state_len,
            );
            let gap = (width as usize).saturating_sub(header_len + state_len);
            format!("{}{:gap$}{}", header, "", state, gap = gap)
        }
        2 => {
            // Session/work time (right-aligned)
            let label = format!("{}◉ Session{}", fg(color::GRAY), RESET);
            let value = format!("{}{}{}", fg(color::BLUE), stats.format_work(), RESET);
            let label_len = strip_ansi_len(&label);
            let value_len = strip_ansi_len(&value);
            let gap = (width as usize).saturating_sub(label_len + value_len);
            format!("{}{:gap$}{}", label, "", value, gap = gap)
        }
        3 => {
            // Thinking time (always show, with dash when no thinking yet)
            let label = format!("{}◐ Thinking{}", fg(color::GRAY), RESET);
            let thinking_value = stats.format_thinking().unwrap_or_else(|| "—".to_string());
            let value = format!("{}{}{}", fg(color::GREEN), thinking_value, RESET);
            let label_len = strip_ansi_len(&label);
            let value_len = strip_ansi_len(&value);
            let gap = (width as usize).saturating_sub(label_len + value_len);
            format!("{}{:gap$}{}", label, "", value, gap = gap)
        }
        4 => {
            // Prompts: count left-aligned after label, timer right-aligned
            let label = format!(
                "{}{PROMPT_ICON} Prompts{} {}{}{}",
                fg(color::GRAY),
                RESET,
                fg(color::LIGHT_BLUE),
                stats.platform_stats.prompts,
                RESET
            );
            let elapsed = format_elapsed(stats.prompts_changed_at);
            let timer = format!("{}{}{}", fg(color::GRAY), elapsed, RESET);
            let label_len = strip_ansi_len(&label);
            let timer_len = strip_ansi_len(&timer);
            let gap = (width as usize).saturating_sub(label_len + timer_len);
            format!("{}{:gap$}{}", label, "", timer, gap = gap)
        }
        5 => {
            // Completions: count left-aligned after label, timer right-aligned
            let label = format!(
                "{}{COMPLETION_ICON} Completions{} {}{}{}",
                fg(color::GRAY),
                RESET,
                fg(color::LIGHT_BLUE),
                stats.platform_stats.completions,
                RESET
            );
            let elapsed = format_elapsed(stats.completions_changed_at);
            let timer = format!("{}{}{}", fg(color::GRAY), elapsed, RESET);
            let label_len = strip_ansi_len(&label);
            let timer_len = strip_ansi_len(&timer);
            let gap = (width as usize).saturating_sub(label_len + timer_len);
            format!("{}{:gap$}{}", label, "", timer, gap = gap)
        }
        6 => {
            // Tool usage sparkline (spans from after label to right edge)
            let label = format!("{}⚒ Tools{} ", fg(color::GRAY), RESET);
            let label_len = strip_ansi_len(&label);
            let sparkline_width = (width as usize).saturating_sub(label_len);
            let bins = stats.tool_usage_bins(sparkline_width);
            let sparkline = render_sparkline(&bins, sparkline_width);
            format!("{}{}", label, sparkline)
        }
        7 => {
            // Compactions (always show, like prompts/completions)
            let compressions = stats.platform_stats.compressions;
            let label = format!(
                "{}⊜ Compactions{} {}{}{}",
                fg(color::GRAY),
                RESET,
                fg(color::PINK),
                compressions,
                RESET
            );
            if compressions > 0 {
                let elapsed = format_elapsed(stats.compressions_changed_at);
                let timer = format!("{}{}{}", fg(color::GRAY), elapsed, RESET);
                let label_len = strip_ansi_len(&label);
                let timer_len = strip_ansi_len(&timer);
                let gap = (width as usize).saturating_sub(label_len + timer_len);
                format!("{}{:gap$}{}", label, "", timer, gap = gap)
            } else {
                label
            }
        }
        8 => {
            // Idle time (only show when complete/question/interrupted state and idle > 60s)
            let is_idle_state = matches!(
                stats.effective_state(),
                SessionState::Complete | SessionState::Question | SessionState::Interrupted
            );
            if is_idle_state {
                if let Some(secs) = idle_seconds(stats.platform_stats.idle_since) {
                    format!(
                        "{}◌ Idle{} {}{}{}",
                        fg(color::GRAY),
                        RESET,
                        fg(color::GRAY),
                        format_duration_compact(secs),
                        RESET
                    )
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible(s: &str) -> usize {
        crate::ui::utils::strip_ansi_len(s)
    }

    fn osc_link_target(s: &str) -> Option<String> {
        // OSC 8 hyperlink format: \x1b]8;;<url>\x07<text>\x1b]8;;\x07
        let start = s.find("\x1b]8;;")? + "\x1b]8;;".len();
        let end = s[start..].find('\x07')? + start;
        Some(s[start..end].to_string())
    }

    #[test]
    fn compact_layout_starts_before_normal_rows_are_clipped() {
        let stats = SessionStats::default();

        assert!(stats_use_compact_layout(6));
        assert!(!stats_use_compact_layout(7));
        assert_eq!(stats_render_rows(6, &stats), 4);
        assert_eq!(stats_render_rows(7, &stats), 7);
    }

    #[test]
    fn compact_grid_pairs_related_metrics() {
        let stats = SessionStats::default();
        let width = 70;

        let first = draw_compact_row(2, width, &stats, None, false, None);
        let second = draw_compact_row(3, width, &stats, None, false, None);
        let third = draw_compact_row(4, width, &stats, None, false, None);

        assert!(first.contains("Session"));
        assert!(first.contains("Thinking"));
        assert!(!first.contains("Prompts"));
        assert!(second.contains("Prompts"));
        assert!(second.contains("Completions"));
        assert!(third.contains("Tools"));
        assert!(third.contains("Compactions"));
        assert_eq!(visible(&first), width as usize);
        assert_eq!(visible(&second), width as usize);
        assert_eq!(visible(&third), width as usize);
    }

    #[test]
    fn pair_suffix_full_tier_renders_with_label() {
        let (width, ansi) = build_pair_suffix("ABC-DEF-GHI", 64).expect("full tier should render");
        // " · Pair: ABC-DEF-GHI" = 3 + 6 + 11 = 20 cols
        assert_eq!(width, 20);
        assert_eq!(visible(&ansi), 20);
        // The OSC link still points to the full code regardless of tier.
        assert_eq!(
            osc_link_target(&ansi).as_deref(),
            Some("https://drinkcrabigator.com/dashboard?setup=ABC-DEF-GHI")
        );
    }

    #[test]
    fn pair_suffix_drops_label_when_tight() {
        // 14 cols fits " · ABC-DEF-GHI" (3 + 11 = 14) but not the full tier (20).
        let (width, ansi) = build_pair_suffix("ABC-DEF-GHI", 14).expect("no-label tier");
        assert_eq!(width, 14);
        assert!(!ansi.contains("Pair:"));
    }

    #[test]
    fn pair_suffix_truncates_code_when_very_tight() {
        // 8 cols → " · " + 5 visible code chars (4 + ellipsis).
        let (width, ansi) = build_pair_suffix("ABC-DEF-GHI", 8).expect("truncated tier");
        assert_eq!(width, 8);
        assert!(ansi.contains('…'));
        // URL still carries the full code.
        assert_eq!(
            osc_link_target(&ansi).as_deref(),
            Some("https://drinkcrabigator.com/dashboard?setup=ABC-DEF-GHI")
        );
    }

    #[test]
    fn pair_suffix_returns_none_when_no_room() {
        // " · " plus at least 2 code chars + ellipsis needs 6 cols. Below that, hide.
        assert!(build_pair_suffix("ABC-DEF-GHI", 5).is_none());
        assert!(build_pair_suffix("ABC-DEF-GHI", 0).is_none());
    }

    #[test]
    fn pair_suffix_link_excludes_leading_separator() {
        // The " · " before the link should sit OUTSIDE the OSC sequence so
        // clicks only trigger on the meaningful pair text.
        let (_, ansi) = build_pair_suffix("ABC-DEF-GHI", 64).unwrap();
        let osc_open = ansi.find("\x1b]8;;").unwrap();
        // Everything before the opening OSC must be just the prefix decoration:
        // a single space, the dim middot (with its own SGR), space, RESET.
        let before = &ansi[..osc_open];
        assert!(before.starts_with(' '));
        assert!(before.contains('·'));
        assert!(!before.contains("Pair"));
    }
}
