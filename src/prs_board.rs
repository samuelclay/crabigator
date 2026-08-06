//! `crabigator prs` — the live cross-session PR board.
//!
//! Aggregates every tracked PR from every running session's `inspect.json`
//! mirror into one read-only, auto-refreshing view: grouped by repository,
//! ordered by how much attention each PR needs, with primaries starred,
//! cross-repo twins (same head branch) kept together, a progress checklist,
//! the latest recap's judgment, and clickable GitHub/Slack/action links.
//!
//! The board never talks to `gh` itself — it renders what the sessions
//! already know, with honest ages. Cloud dispositions are fetched once a
//! minute so dashboard toggles apply here too. The default view reads live
//! session mirrors under /tmp; `a` flips to the durable cloud record, which
//! includes ended sessions' PRs (tagged for resurrection) at ~1min lag.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size as terminal_size, EnterAlternateScreen,
    LeaveAlternateScreen,
};

use crate::pr::SessionPr;
use crate::pr_rank::PrDisposition;
use crate::terminal::escape::{self, color, fg, RESET, RESET_FG};
use crate::ui::pr_cells::{pr_row_text, PrColumnWidths};

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const OVERRIDES_REFRESH: Duration = Duration::from_secs(60);
/// The all-sessions view refetches the cloud board at this cadence.
const CLOUD_BOARD_REFRESH: Duration = Duration::from_secs(15);
/// A mirror this old is a session that stopped updating; its rows dim.
const STALE_SESSION_SECS: f64 = 300.0;
/// How long merged/closed primary PRs linger by default; +/- adjusts at runtime.
const DEFAULT_LINGER_DAYS: u64 = 1;
/// Ceiling for the +key so the window can't run away unbounded.
const MAX_LINGER_DAYS: u64 = 90;
/// Transcript search needs this many characters before it kicks in — one or
/// two letters match nearly every line and would light up the whole board.
const TRANSCRIPT_QUERY_MIN: usize = 3;
/// The expanded preview shows this many of the most recent matches.
const PREVIEW_MATCHES: usize = 3;
/// Transcript lines shown either side of a match in the expanded preview.
const PREVIEW_CONTEXT: usize = 2;
/// Detail levels `e`/`c` expand and collapse through: 0 = compact (one line
/// per PR), 1 = standard (progress + sessions + judgment), 2 = + each
/// session's terminal title, 3 = + each session's latest recap headline.
const MAX_DETAIL: u8 = 3;
const DEFAULT_DETAIL: u8 = 1;

/// Header name for a non-default detail level.
fn detail_name(detail: u8) -> &'static str {
    match detail {
        0 => "compact",
        2 => "titles",
        3 => "recaps",
        _ => "standard",
    }
}

/// The slice of a session's latest recap the detail view renders.
#[derive(Clone)]
struct RecapBrief {
    headline: String,
    /// Unix ms when the recap was generated; 0 when unknown.
    generated_at: u64,
    line_delta: crate::recap::TurnLineDelta,
}

/// One live session's contribution to the board.
struct SessionSnapshot {
    dir_name: String,
    /// The session's /tmp mirror directory, where scrollback.log lives.
    session_dir: PathBuf,
    last_updated: f64,
    branch: String,
    uncommitted_files: usize,
    /// The session's current terminal title (generated or OSC-published).
    title: String,
    /// The session's latest recap, when one has been generated.
    recap: Option<RecapBrief>,
    prs: Vec<SessionPr>,
}

/// One PR aggregated across every session that mentions it.
struct BoardPr {
    /// Freshest copy of the GitHub stats (newest `refreshed_at` wins).
    pr: SessionPr,
    sessions: Vec<SessionRef>,
    /// Uncommitted files in a live session sitting on this PR's branch.
    uncommitted: usize,
    stale: bool,
}

struct SessionRef {
    dir_name: String,
    /// Local mirror directory holding scrollback.log; None for cloud records,
    /// whose transcripts aren't reachable from this machine.
    session_dir: Option<PathBuf>,
    age_secs: u64,
    /// The session is gone (cloud record only) — a candidate to resurrect.
    ended: bool,
    /// The session's current terminal title (detail level 2+).
    title: String,
    /// The session's latest recap (detail level 3).
    recap: Option<RecapBrief>,
}

/// Read every live mirror, one snapshot per session.
fn gather() -> Result<Vec<SessionSnapshot>> {
    let mut snapshots = Vec::new();
    for (path, data) in crate::inspect::discover_instances(&None)? {
        let prs: Vec<SessionPr> = data
            .get("prs")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        if prs.is_empty() {
            continue;
        }
        let git = data.pointer("/widgets/git/data");
        snapshots.push(SessionSnapshot {
            session_dir: path.parent().map(Path::to_path_buf).unwrap_or_default(),
            dir_name: data
                .get("cwd")
                .and_then(|v| v.as_str())
                .and_then(|cwd| cwd.rsplit('/').next())
                .unwrap_or_default()
                .to_string(),
            last_updated: data
                .get("last_updated")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            branch: git
                .and_then(|g| g.get("branch"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            uncommitted_files: git
                .and_then(|g| g.get("files"))
                .and_then(|v| v.as_array())
                .map(|files| files.len())
                .unwrap_or(0),
            title: data
                .get("terminal_title")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    data.get("title_history")
                        .and_then(|v| v.as_array())
                        .and_then(|titles| titles.last())
                        .and_then(|v| v.as_str())
                })
                .unwrap_or_default()
                .to_string(),
            recap: data
                .pointer("/recap/latest")
                .or_else(|| {
                    data.get("recap_history")
                        .and_then(|v| v.as_array())
                        .and_then(|recaps| recaps.last())
                })
                .and_then(recap_brief_from),
            prs,
        });
    }
    Ok(snapshots)
}

/// Parse the fields the board needs out of a mirrored TurnRecap value.
fn recap_brief_from(recap: &serde_json::Value) -> Option<RecapBrief> {
    let headline = recap.get("headline")?.as_str()?;
    if headline.is_empty() {
        return None;
    }
    Some(RecapBrief {
        headline: headline.to_string(),
        generated_at: recap
            .get("generated_at")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        line_delta: crate::recap::TurnLineDelta {
            additions: recap
                .pointer("/line_delta/additions")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            deletions: recap
                .pointer("/line_delta/deletions")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        },
    })
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Merge session snapshots into deduped board entries, honoring overrides.
/// `linger_days` bounds how long finished primary PRs stay visible (0 = open only).
fn aggregate(
    snapshots: &[SessionSnapshot],
    overrides: &HashMap<String, PrDisposition>,
    linger_days: u64,
) -> Vec<BoardPr> {
    let now = now_secs();
    let mut merged: HashMap<String, BoardPr> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for session in snapshots {
        let session_age = (now - session.last_updated).max(0.0);
        for pr in &session.prs {
            let key = format!("{}/{}#{}", pr.owner, pr.repo, pr.number);
            let entry = merged.entry(key.clone()).or_insert_with(|| {
                order.push(key);
                // Engagement counters accumulate below, per contributing
                // session — start the merged copy from zero.
                let mut seed = pr.clone();
                seed.mentions = 0;
                seed.user_mentions = 0;
                seed.last_mentioned_at = 0;
                BoardPr {
                    pr: seed,
                    sessions: Vec::new(),
                    uncommitted: 0,
                    stale: true,
                }
            });

            // Newest GitHub stats win, but classification and engagement belong
            // to the aggregate rather than to any one session's copy.
            let aggregate_primary = entry.pr.primary || pr.primary;
            let aggregate_primary_source = combined_primary_source(&entry.pr, pr);
            let aggregate_dismissed = entry.pr.dismissed || pr.dismissed;
            if pr.refreshed_at > entry.pr.refreshed_at {
                let previous = std::mem::replace(&mut entry.pr, pr.clone());
                entry.pr.mentions = previous.mentions;
                entry.pr.user_mentions = previous.user_mentions;
                entry.pr.first_mentioned_at = previous.first_mentioned_at;
                entry.pr.last_mentioned_at = previous.last_mentioned_at;
                entry.pr.last_mention_prompt = previous.last_mention_prompt;
                if entry.pr.author_login.is_empty() {
                    entry.pr.author_login = previous.author_login;
                }
                if entry.pr.authored_by_viewer.is_none() {
                    entry.pr.authored_by_viewer = previous.authored_by_viewer;
                }
            }
            entry.pr.mentions += pr.mentions;
            entry.pr.user_mentions += pr.user_mentions;
            entry.pr.last_mentioned_at = entry.pr.last_mentioned_at.max(pr.last_mentioned_at);
            entry.pr.primary = aggregate_primary;
            entry.pr.primary_source = aggregate_primary_source;
            entry.pr.dismissed = aggregate_dismissed;
            if entry.pr.slack_origin_url.is_empty() {
                entry.pr.slack_origin_url = pr.slack_origin_url.clone();
            }
            for url in &pr.slack_comment_urls {
                if !entry.pr.slack_comment_urls.contains(url) {
                    entry.pr.slack_comment_urls.push(url.clone());
                }
            }
            if entry.pr.ai_note.is_empty() {
                entry.pr.ai_note = pr.ai_note.clone();
                entry.pr.ai_confidence = pr.ai_confidence.clone();
            }

            entry.stale &= session_age > STALE_SESSION_SECS;
            if !pr.branch.is_empty() && session.branch == pr.branch {
                entry.uncommitted = entry.uncommitted.max(session.uncommitted_files);
            }
            entry.sessions.push(SessionRef {
                dir_name: session.dir_name.clone(),
                session_dir: Some(session.session_dir.clone()),
                age_secs: session_age as u64,
                ended: false,
                title: session.title.clone(),
                recap: session.recap.clone(),
            });
        }
    }

    let now_ms = (now * 1000.0) as u64;
    let mut out: Vec<BoardPr> = order
        .into_iter()
        .filter_map(|key| {
            let mut entry = merged.remove(&key)?;
            match overrides.get(&key) {
                Some(PrDisposition::Dismissed) => return None,
                Some(PrDisposition::Primary) => {
                    entry.pr.primary = true;
                    entry.pr.primary_source = "override".to_string();
                }
                Some(PrDisposition::Secondary) => {
                    entry.pr.primary = false;
                    entry.pr.primary_source = "override".to_string();
                }
                None => {}
            }
            visible_pr(&entry.pr, linger_days, now_ms).then_some(entry)
        })
        .collect();

    sort_entries(&mut out);
    out
}

/// Keep the strongest source among the session copies that called this PR
/// primary. A newer metadata snapshot must not erase an older classification.
fn combined_primary_source(existing: &SessionPr, incoming: &SessionPr) -> String {
    match (existing.primary, incoming.primary) {
        (true, true) => {
            if primary_source_rank(&incoming.primary_source)
                > primary_source_rank(&existing.primary_source)
            {
                incoming.primary_source.clone()
            } else {
                existing.primary_source.clone()
            }
        }
        (true, false) => existing.primary_source.clone(),
        (false, true) => incoming.primary_source.clone(),
        (false, false) => String::new(),
    }
}

fn primary_source_rank(source: &str) -> u8 {
    match source {
        "override" => 3,
        "session" => 2,
        "auto" => 1,
        _ => 0,
    }
}

fn visible_pr(pr: &SessionPr, linger_days: u64, now_ms: u64) -> bool {
    if pr.dismissed {
        return false;
    }
    // Unverified references are usually scanning artifacts. A primary is
    // different: the session classifier has enough ownership evidence to keep
    // it visible as "fetching" while enrichment retries.
    if pr.refreshed_at == 0 {
        return pr.primary;
    }
    if pr.state == "OPEN" {
        return true;
    }
    // Finished secondaries disappear immediately. Finished primaries retain
    // the adjustable grace window, except foreign-authored PRs the user never
    // explicitly mentioned or promoted.
    if !pr.primary || foreign_without_explicit_interest(pr) {
        return false;
    }
    let latest = pr.closed_at.max(pr.last_mentioned_at);
    linger_days > 0 && latest > 0 && now_ms.saturating_sub(latest) <= linger_days * 24 * 3600 * 1000
}

fn foreign_without_explicit_interest(pr: &SessionPr) -> bool {
    pr.authored_by_viewer == Some(false)
        && pr.user_mentions == 0
        && !matches!(pr.primary_source.as_str(), "session" | "override")
}

/// Attention first, then primaries, then recency of discussion.
fn sort_entries(entries: &mut [BoardPr]) {
    entries.sort_by(|a, b| {
        stage(&a.pr)
            .rank
            .cmp(&stage(&b.pr).rank)
            .then(b.pr.primary.cmp(&a.pr.primary))
            .then(b.pr.last_mentioned_at.cmp(&a.pr.last_mentioned_at))
    });
}

/// Map the cloud board (durable D1 records, ended sessions included) into
/// the same shape the live aggregation produces. Overrides and the linger
/// window are already applied server-side.
fn cloud_entries_to_board(cloud: Vec<crate::cloud::CloudBoardEntry>) -> Vec<BoardPr> {
    let now = now_secs() as u64;
    let mut out: Vec<BoardPr> = cloud
        .into_iter()
        .map(|entry| {
            let any_active = entry.sessions.iter().any(|s| s.active);
            BoardPr {
                pr: entry.pr,
                sessions: entry
                    .sessions
                    .into_iter()
                    .map(|s| SessionRef {
                        dir_name: s.dir_name,
                        session_dir: None,
                        age_secs: now.saturating_sub(s.last_seen_at),
                        ended: !s.active,
                        title: s.title,
                        recap: s.recap.and_then(|r| {
                            (!r.headline.is_empty()).then_some(RecapBrief {
                                headline: r.headline,
                                generated_at: r.generated_at,
                                line_delta: crate::recap::TurnLineDelta {
                                    additions: r.additions,
                                    deletions: r.deletions,
                                },
                            })
                        }),
                    })
                    .collect(),
                uncommitted: 0,
                stale: !any_active,
            }
        })
        .collect();
    sort_entries(&mut out);
    out
}

/// Where a PR sits in the pipeline: its sort rank (most attention needed
/// first) plus the label and color the board renders for it.
struct Stage {
    rank: u8,
    label: &'static str,
    color: u8,
}

fn stage(pr: &SessionPr) -> Stage {
    // A PR with no state yet was never enriched: show the fetch, not "closed".
    let (rank, label, color) = if pr.state.is_empty() && !pr.fetch_error.is_empty() {
        (2, "fetch failed, retrying", color::RED)
    } else if pr.state.is_empty() {
        (2, "fetching", color::GRAY)
    } else if pr.state == "MERGED" {
        (6, "merged", color::PURPLE)
    } else if pr.state != "OPEN" {
        (7, "closed", color::DARK_GRAY)
    } else if pr.mergeable == "CONFLICTING" {
        (0, "conflicts", color::RED)
    } else if pr.checks_failed > 0 {
        (0, "CI failing", color::RED)
    } else if pr.review_decision == "CHANGES_REQUESTED" {
        (1, "changes requested", color::RED)
    } else if pr.is_draft {
        (2, "draft", color::GRAY)
    } else if pr.checks_pending > 0 {
        (3, "CI running", color::YELLOW)
    } else if pr.review_decision != "APPROVED" {
        (4, "awaiting review", color::YELLOW)
    } else {
        (5, "ready to merge", color::GREEN)
    };
    Stage { rank, label, color }
}

/// How far along the PR is, as a percentage. The objective pipeline
/// position sets the base; the recap's confidence nudges open PRs a step
/// either way. Merged is always 100% — no model opinion overrides a merge.
fn progress_percent(pr: &SessionPr) -> u8 {
    if pr.state.is_empty() {
        // Not enriched yet — nothing is known, so the bar stays near empty.
        return 5;
    }
    if pr.state != "OPEN" {
        return 100;
    }
    let base: i32 = match stage(pr).rank {
        0 => 35, // conflicts / CI failing
        1 => 40, // changes requested
        2 => 15, // draft
        3 => 55, // CI running
        4 => 70, // awaiting review
        _ => 90, // ready to merge
    };
    let nudge = match pr.ai_confidence.as_str() {
        "high" => 10,
        "low" => -10,
        _ => 0,
    };
    (base + nudge).clamp(5, 95) as u8
}

/// A muted variant of each stage color for the progress bar, so the gauge
/// reads at a glance without outshining the text around it.
fn dim_stage_color(stage_color: u8) -> u8 {
    match stage_color {
        color::RED => 131,    // muted brick
        color::YELLOW => 136, // dark goldenrod
        color::GREEN => 65,   // muted green
        color::PURPLE => 97,  // muted violet
        _ => 240,             // draft/closed grays fall back to dark gray
    }
}

/// Ten-cell progress bar in a muted stage color: `▓▓▓▓▓▓▓░░░`. The bar is an
/// approximation, so it shows no percentage — the shape is the signal.
fn progress_bar(pr: &SessionPr) -> String {
    let pct = progress_percent(pr);
    let stage = stage(pr);
    let filled = (usize::from(pct) + 5) / 10;
    format!(
        "{}{}{}{}{}",
        fg(dim_stage_color(stage.color)),
        "▓".repeat(filled),
        fg(color::DARK_GRAY),
        "░".repeat(10 - filled),
        RESET_FG
    )
}

/// Coarse ages only: second-level precision would tick on every repaint and
/// make the board look like it's thrashing.
fn format_age(secs: u64) -> String {
    match secs {
        0..=89 => "1m".to_string(),
        90..=3599 => format!("{}m", secs.div_ceil(60)),
        3600..=86399 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86400),
    }
}

/// The sessions that mention a PR, deduped by directory name (several
/// sessions often share a cwd), with a dim idle tag for stopped mirrors.
fn sessions_text(sessions: &[SessionRef]) -> String {
    // (name, count, min age, every ref ended)
    let mut seen: Vec<(String, usize, u64, bool)> = Vec::new();
    for session in sessions {
        match seen
            .iter_mut()
            .find(|(name, _, _, _)| *name == session.dir_name)
        {
            Some((_, count, min_age, ended)) => {
                *count += 1;
                *min_age = (*min_age).min(session.age_secs);
                *ended &= session.ended;
            }
            None => seen.push((session.dir_name.clone(), 1, session.age_secs, session.ended)),
        }
    }
    seen.iter()
        .map(|(name, count, min_age, ended)| {
            let times = if *count > 1 {
                format!(" ×{count}")
            } else {
                String::new()
            };
            // Ended sessions (cloud records) read as resurrection candidates;
            // a live mirror updates constantly, so only a stopped one gets a
            // tag.
            let tag = if *ended {
                format!(
                    " {}(ended {} ago){}",
                    fg(color::DARK_GRAY),
                    format_age(*min_age),
                    fg(color::GRAY)
                )
            } else if *min_age as f64 > STALE_SESSION_SECS {
                format!(
                    " {}(idle {}){}",
                    fg(color::DARK_GRAY),
                    format_age(*min_age),
                    fg(color::GRAY)
                )
            } else {
                String::new()
            };
            format!("{name}{times}{tag}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Case-insensitive substring filter across a PR's identifying text.
fn matches_search(pr: &SessionPr, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    pr.number.to_string().contains(&query)
        || pr.repo.to_lowercase().contains(&query)
        || pr.owner.to_lowercase().contains(&query)
        || pr.title.to_lowercase().contains(&query)
        || pr.branch.to_lowercase().contains(&query)
}

/// Byte range of the first case-insensitive occurrence of `needle` in
/// `haystack`, safe for text whose lowercase form changes length.
fn find_ci(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    let needle_lc: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if needle_lc.is_empty() {
        return None;
    }
    let chars: Vec<(usize, char)> = haystack.char_indices().collect();
    for start in 0..chars.len() {
        let mut matched = 0;
        let mut end = start;
        'walk: while matched < needle_lc.len() && end < chars.len() {
            for lc in chars[end].1.to_lowercase() {
                if matched >= needle_lc.len() || needle_lc[matched] != lc {
                    break 'walk;
                }
                matched += 1;
            }
            end += 1;
        }
        if matched == needle_lc.len() {
            let s = chars[start].0;
            let e = chars.get(end).map_or(haystack.len(), |(i, _)| *i);
            return Some((s, e));
        }
    }
    None
}

/// One session's scrollback, ANSI-stripped and split into lines, cached by
/// file size + mtime so the 2s tick doesn't reread unchanged transcripts.
struct CachedTranscript {
    len: u64,
    modified: SystemTime,
    lines: Vec<String>,
}

#[derive(Default)]
struct TranscriptCache {
    files: HashMap<PathBuf, CachedTranscript>,
}

impl TranscriptCache {
    /// The transcript lines for a session mirror directory, or None when the
    /// session has no readable scrollback.log.
    fn lines(&mut self, session_dir: &Path) -> Option<&[String]> {
        let path = session_dir.join("scrollback.log");
        let meta = std::fs::metadata(&path).ok()?;
        let len = meta.len();
        let modified = meta.modified().unwrap_or(UNIX_EPOCH);
        let fresh = self
            .files
            .get(&path)
            .is_some_and(|c| c.len == len && c.modified == modified);
        if !fresh {
            let raw = std::fs::read(&path).ok()?;
            let text = crate::parsers::strip_ansi_for_debug(&String::from_utf8_lossy(&raw));
            let lines = text.lines().map(str::to_string).collect();
            self.files.insert(
                path.clone(),
                CachedTranscript {
                    len,
                    modified,
                    lines,
                },
            );
        }
        self.files.get(&path).map(|c| c.lines.as_slice())
    }
}

/// Indices of transcript lines containing the query, case-insensitively.
fn transcript_match_lines(lines: &[String], query: &str) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| find_ci(line, query).is_some())
        .map(|(i, _)| i)
        .collect()
}

/// One row of a transcript preview, before styling.
struct PreviewRow {
    text: String,
    is_match: bool,
    /// This row starts a new context group — draw a `⋯` separator first.
    gap_before: bool,
}

/// Pick the preview rows for one session: the most recent matching line when
/// collapsed, or the last few matches with surrounding context when expanded.
fn preview_rows(lines: &[String], matches: &[usize], expanded: bool) -> Vec<PreviewRow> {
    if matches.is_empty() {
        return Vec::new();
    }
    if !expanded {
        let idx = *matches.last().unwrap();
        return vec![PreviewRow {
            text: lines[idx].clone(),
            is_match: true,
            gap_before: false,
        }];
    }
    // Context windows around the last few matches, merged where they touch.
    let recent = &matches[matches.len().saturating_sub(PREVIEW_MATCHES)..];
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &idx in recent {
        let start = idx.saturating_sub(PREVIEW_CONTEXT);
        let end = (idx + PREVIEW_CONTEXT).min(lines.len().saturating_sub(1));
        match ranges.last_mut() {
            Some((_, prev_end)) if start <= *prev_end + 1 => *prev_end = (*prev_end).max(end),
            _ => ranges.push((start, end)),
        }
    }
    let mut rows = Vec::new();
    for (group, (start, end)) in ranges.into_iter().enumerate() {
        for (idx, line) in lines.iter().enumerate().take(end + 1).skip(start) {
            rows.push(PreviewRow {
                text: line.clone(),
                is_match: matches.binary_search(&idx).is_ok(),
                gap_before: group > 0 && idx == start,
            });
        }
    }
    rows
}

/// Trim a matched line so the first occurrence of the query stays visible in
/// `max` display cells, ellipsizing the front when the match sits far right.
fn window_around_match(line: &str, query: &str, max: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let trimmed = line.trim();
    let Some((start, _)) = find_ci(trimmed, query) else {
        return crate::ui::pr_cells::truncate_to_width(trimmed, max);
    };
    let lead = 15usize.min(max / 3);
    if trimmed[..start].width() + lead <= max {
        return crate::ui::pr_cells::truncate_to_width(trimmed, max);
    }
    // Drop chars from the front until the match sits `lead` cells in.
    let mut cut = 0;
    for (i, _) in trimmed.char_indices() {
        if trimmed[i..start].width() <= lead {
            cut = i;
            break;
        }
    }
    let windowed = format!("…{}", &trimmed[cut..]);
    crate::ui::pr_cells::truncate_to_width(&windowed, max)
}

/// Wrap every occurrence of the query in a yellow highlight, restoring the
/// row's own color afterwards.
fn highlight_query(text: &str, query: &str, restore: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some((start, end)) = find_ci(rest, query) {
        out.push_str(&rest[..start]);
        out.push_str(&escape::bg(color::YELLOW));
        out.push_str(&fg(16));
        out.push_str(&rest[start..end]);
        out.push_str(RESET);
        out.push_str(restore);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Fully styled preview lines for one board entry: for each contributing
/// session whose transcript contains the query, an excerpt the search hit —
/// so a match can be confirmed without opening the session. Empty when the
/// query is too short or nothing matches.
fn build_previews(
    entry: &BoardPr,
    cache: &mut TranscriptCache,
    query: &str,
    width: u16,
    expanded: bool,
) -> Vec<String> {
    if query.len() < TRANSCRIPT_QUERY_MIN {
        return Vec::new();
    }
    let mut seen_dirs: HashSet<&Path> = HashSet::new();
    let mut out = Vec::new();
    for session in &entry.sessions {
        let Some(dir) = session.session_dir.as_deref() else {
            continue;
        };
        if !seen_dirs.insert(dir) {
            continue;
        }
        let Some(lines) = cache.lines(dir) else {
            continue;
        };
        let matches = transcript_match_lines(lines, query);
        if matches.is_empty() {
            continue;
        }
        let rows = preview_rows(lines, &matches, expanded);
        out.extend(styled_preview_lines(
            &session.dir_name,
            matches.len(),
            &rows,
            query,
            width,
            expanded,
        ));
    }
    out
}

/// Render one session's preview rows into display lines: a single inline
/// snippet when collapsed, or a header plus context block when expanded.
fn styled_preview_lines(
    dir_name: &str,
    total: usize,
    rows: &[PreviewRow],
    query: &str,
    width: u16,
    expanded: bool,
) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let matches_text = format!("{} match{}", total, if total == 1 { "" } else { "es" });
    if !expanded {
        let row = &rows[0];
        let suffix = format!(" · {matches_text}");
        let budget = (width as usize)
            .saturating_sub(3 + 2 + dir_name.width() + 3 + suffix.width())
            .max(10);
        let snippet = window_around_match(&row.text, query, budget);
        return vec![format!(
            "   {}⌕ {}{}{} · {}{}{}{}{}",
            fg(color::DARK_GRAY),
            fg(color::CYAN),
            dir_name,
            fg(color::DARK_GRAY),
            fg(color::GRAY),
            highlight_query(&snippet, query, &fg(color::GRAY)),
            fg(color::DARK_GRAY),
            suffix,
            RESET_FG,
        )];
    }
    let mut out = vec![format!(
        "   {}⌕ {}{}{} · {}{}",
        fg(color::DARK_GRAY),
        fg(color::CYAN),
        dir_name,
        fg(color::DARK_GRAY),
        matches_text,
        RESET_FG,
    )];
    let budget = (width as usize).saturating_sub(7).max(10);
    for row in rows {
        if row.gap_before {
            out.push(format!("     {}⋯{}", fg(color::DARK_GRAY), RESET_FG));
        }
        let text = crate::ui::pr_cells::truncate_to_width(row.text.trim_end(), budget);
        let line = if row.is_match {
            format!(
                "     {}│ {}{}{}",
                fg(color::DARK_GRAY),
                fg(color::GRAY),
                highlight_query(&text, query, &fg(color::GRAY)),
                RESET_FG,
            )
        } else {
            format!("     {}│ {}{}", fg(color::DARK_GRAY), text, RESET_FG)
        };
        out.push(line);
    }
    out
}

/// Detail lines for one PR at level 2+: each contributing session's terminal
/// title, and at level 3 its latest recap headline underneath. Sessions with
/// nothing to show are skipped; exact duplicates collapse (cloud records
/// often repeat a directory).
fn session_detail_lines(
    sessions: &[SessionRef],
    width: u16,
    detail: u8,
    now_ms: u64,
) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let mut seen: HashSet<(&str, &str, &str)> = HashSet::new();
    let mut out = Vec::new();
    for session in sessions {
        let recap = if detail >= 3 {
            session.recap.as_ref()
        } else {
            None
        };
        // A title that just repeats the directory name is the terminal's
        // default, not a generated one — it says nothing the row doesn't.
        let title = if session.title == session.dir_name {
            ""
        } else {
            session.title.as_str()
        };
        if title.is_empty() && recap.is_none() {
            continue;
        }
        let recap_key = recap.map_or("", |r| r.headline.as_str());
        if !seen.insert((session.dir_name.as_str(), title, recap_key)) {
            continue;
        }
        // The title is only as fresh as the session's last mirror update,
        // so that age answers "current as of when?".
        let title_age = format!(" · {} ago", format_age(session.age_secs));
        let mut header = format!(
            "   {}⌾ {}{}{}",
            fg(color::DARK_GRAY),
            fg(color::CYAN),
            session.dir_name,
            fg(color::DARK_GRAY),
        );
        if !title.is_empty() {
            let budget = (width as usize)
                .saturating_sub(5 + session.dir_name.width() + 3 + title_age.width())
                .max(10);
            header.push_str(&format!(
                " · {}{}{}",
                RESET_FG,
                crate::ui::pr_cells::truncate_to_width(title, budget),
                fg(color::DARK_GRAY),
            ));
        }
        header.push_str(&title_age);
        header.push_str(RESET_FG);
        out.push(header);
        if let Some(recap) = recap {
            let age = if recap.generated_at > 0 {
                format!(
                    " · {} ago",
                    format_age(now_ms.saturating_sub(recap.generated_at) / 1000)
                )
            } else {
                String::new()
            };
            // A zero delta says nothing here — leave it off rather than
            // rendering the handoff strip's `Δ ·` placeholder.
            let delta = if recap.line_delta.additions == 0 && recap.line_delta.deletions == 0 {
                String::new()
            } else {
                format!(
                    " · {}{}",
                    crate::ui::handoff::format_line_delta(recap.line_delta),
                    fg(color::DARK_GRAY)
                )
            };
            // The delta carries its own colors; budget by its visible form.
            let delta_width = crate::parsers::strip_ansi_for_debug(&delta).width();
            let budget = (width as usize)
                .saturating_sub(7 + delta_width + age.width())
                .max(10);
            out.push(format!(
                "     {}↪ {}{}{}{}{}{}",
                fg(color::DARK_GRAY),
                fg(color::GRAY),
                crate::ui::pr_cells::truncate_to_width(&recap.headline, budget),
                fg(color::DARK_GRAY),
                delta,
                age,
                RESET_FG,
            ));
        }
    }
    out
}

/// One board entry ready to draw: the PR plus any transcript preview lines
/// the active search produced for it.
struct BoardRow<'a> {
    entry: &'a BoardPr,
    preview_lines: Vec<String>,
}

/// Build one full frame as displayable lines (no trailing newline handling).
fn render(
    rows: &[BoardRow],
    width: u16,
    linger_days: u64,
    include_ended: bool,
    detail: u8,
) -> Vec<String> {
    let now_ms = (now_secs() * 1000.0) as u64;
    let session_count = rows
        .iter()
        .flat_map(|r| r.entry.sessions.iter().map(|s| s.dir_name.as_str()))
        .collect::<HashSet<_>>()
        .len();
    // The window reads gray at the default and yellow once adjusted, so an
    // unusual view never masquerades as the everyday one.
    let window_text = match linger_days {
        0 => "open only".to_string(),
        days => format!("primary done ≤ {days}d"),
    };
    let window = if linger_days == DEFAULT_LINGER_DAYS {
        window_text
    } else {
        format!(
            "{}{}{}",
            fg(color::YELLOW),
            window_text,
            fg(color::DARK_GRAY)
        )
    };
    // The source also reads yellow off its default, so the cloud view (which
    // lags live mirrors by up to a minute) never passes for the live one.
    let source = if include_ended {
        format!("{}all sessions{}", fg(color::YELLOW), fg(color::DARK_GRAY))
    } else {
        "live".to_string()
    };
    // Non-default detail levels announce themselves the same way.
    let detail_label = if detail == DEFAULT_DETAIL {
        String::new()
    } else {
        format!(
            " · {}{}{}",
            fg(color::YELLOW),
            detail_name(detail),
            fg(color::DARK_GRAY)
        )
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "{}⑆ Crabigator PR board{}  {}{} PRs · {} sessions · {} · {}{} · ↑↓ scroll · / search · +/- days · e/c detail · a all · q quit{}",
        fg(color::PURPLE),
        RESET_FG,
        fg(color::DARK_GRAY),
        rows.len(),
        session_count,
        source,
        window,
        detail_label,
        RESET_FG,
    ));
    lines.push(String::new());

    if rows.is_empty() {
        lines.push(format!(
            "{}No PRs tracked by any live session.{}",
            fg(color::GRAY),
            RESET_FG
        ));
        return lines;
    }

    // Group by repo, preserving the attention ordering for group placement.
    let mut repo_order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<&BoardRow>> = HashMap::new();
    for row in rows {
        let repo = format!("{}/{}", row.entry.pr.owner, row.entry.pr.repo);
        if !groups.contains_key(&repo) {
            repo_order.push(repo.clone());
        }
        groups.entry(repo).or_default().push(row);
    }

    for repo in repo_order {
        let group = &groups[&repo];
        lines.push(format!(
            "{}{}{}  {}{}{}",
            fg(color::CYAN),
            repo,
            RESET_FG,
            fg(color::DARK_GRAY),
            if group.len() == 1 {
                "1 PR".to_string()
            } else {
                format!("{} PRs", group.len())
            },
            RESET_FG,
        ));

        let refs: Vec<&SessionPr> = group.iter().map(|r| &r.entry.pr).collect();
        let widths = PrColumnWidths::from_pr_refs(&refs, width as usize);
        for board_row in group {
            let entry = board_row.entry;
            let mut row = pr_row_text(width, &entry.pr, &widths);
            if entry.stale {
                row = format!("{}{row}", fg(color::DARK_GRAY));
            }
            lines.push(format!("{row}{RESET}"));

            // Compact detail keeps only the identity row (search excerpts
            // still show, so an active filter stays explainable).
            if detail == 0 {
                lines.extend(board_row.preview_lines.iter().cloned());
                continue;
            }

            // Row 2: checklist, stage, sessions, engagement, local state.
            let stage = stage(&entry.pr);
            let mentions = if entry.pr.mentions > 0 {
                let yours = if entry.pr.user_mentions > 0 {
                    format!(" ({} yours)", entry.pr.user_mentions)
                } else {
                    String::new()
                };
                format!(" · {} mentions{}", entry.pr.mentions, yours)
            } else {
                String::new()
            };
            let mention_age = if entry.pr.last_mentioned_at > 0 {
                let secs = now_ms.saturating_sub(entry.pr.last_mentioned_at) / 1000;
                if secs < 60 {
                    " · spoken just now".to_string()
                } else {
                    format!(" · spoken {} ago", format_age(secs))
                }
            } else {
                String::new()
            };
            let uncommitted = if entry.uncommitted > 0 {
                format!(
                    " · {}⚠ {} uncommitted{}",
                    fg(color::YELLOW),
                    entry.uncommitted,
                    RESET_FG
                )
            } else {
                String::new()
            };
            lines.push(format!(
                "   {} {}{}{} · {}in {}{}{}{}{}",
                progress_bar(&entry.pr),
                fg(stage.color),
                stage.label,
                RESET_FG,
                fg(color::GRAY),
                sessions_text(&entry.sessions),
                mentions,
                mention_age,
                RESET_FG,
                uncommitted,
            ));

            // Row 3: the recap's judgment and Slack links, when we have them.
            // The judgment describes work in flight, so a merged or closed PR
            // doesn't show one — a merge outranks any stale model opinion.
            let mut extras: Vec<String> = Vec::new();
            if !entry.pr.ai_note.is_empty() && entry.pr.state == "OPEN" {
                extras.push(format!(
                    "{}✦ {}{}",
                    fg(color::YELLOW),
                    entry.pr.ai_note,
                    RESET_FG
                ));
            }
            if !entry.pr.slack_origin_url.is_empty() {
                extras.push(format!(
                    "{}{}{}",
                    fg(color::CYAN),
                    escape::hyperlink(&entry.pr.slack_origin_url, "⛓ slack origin"),
                    RESET_FG
                ));
            }
            for (i, url) in entry.pr.slack_comment_urls.iter().enumerate() {
                let label = if entry.pr.slack_comment_urls.len() == 1 {
                    "⛓ slack".to_string()
                } else {
                    format!("⛓ slack {}", i + 1)
                };
                extras.push(format!(
                    "{}{}{}",
                    fg(color::CYAN),
                    escape::hyperlink(url, &label),
                    RESET_FG
                ));
            }
            if !extras.is_empty() {
                lines.push(format!("   {}", extras.join("  ")));
            }
            // Row 4: per-session terminal titles (and recap headlines at the
            // deepest level), for the expanded detail views.
            if detail >= 2 {
                lines.extend(session_detail_lines(&entry.sessions, width, detail, now_ms));
            }
            // Row 5: transcript excerpts backing an active search hit.
            lines.extend(board_row.preview_lines.iter().cloned());
            lines.push(String::new());
        }
        // Compact rows sit flush; a single gap still separates repo groups.
        if detail == 0 {
            lines.push(String::new());
        }
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// Entry point for `crabigator prs`.
pub async fn run_prs_board(once: bool) -> Result<()> {
    let mut overrides = crate::cloud::fetch_pr_overrides_standalone()
        .await
        .unwrap_or_default();
    let mut overrides_fetched = Instant::now();

    if once {
        let width = terminal_size().map(|(w, _)| w).unwrap_or(120);
        let entries = aggregate(&gather()?, &overrides, DEFAULT_LINGER_DAYS);
        let rows: Vec<BoardRow> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        for line in render(&rows, width, DEFAULT_LINGER_DAYS, false, DEFAULT_DETAIL) {
            println!("{line}");
        }
        return Ok(());
    }

    let mut out = stdout();
    enable_raw_mode()?;
    execute!(out, EnterAlternateScreen)?;
    let result = board_loop(&mut out, &mut overrides, &mut overrides_fetched).await;
    let _ = execute!(out, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    result
}

/// Identity of one rendered frame, so an unchanged board isn't repainted.
fn frame_hash(lines: &[String]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lines.hash(&mut hasher);
    hasher.finish()
}

/// The sticky search banner: unmistakable yellow-on-black, pinned above the
/// scrolled content while a filter is active.
fn search_banner(query: &str, matched: usize, width: u16) -> String {
    let text = format!(
        " /{query}▏ {matched} match{} · type to filter · Tab context · Esc clears ",
        if matched == 1 { "" } else { "es" }
    );
    let padded = format!("{text:<width$}", width = width as usize);
    format!("{}{}{}{}", escape::bg(color::YELLOW), fg(16), padded, RESET)
}

async fn board_loop(
    out: &mut std::io::Stdout,
    overrides: &mut HashMap<String, PrDisposition>,
    overrides_fetched: &mut Instant,
) -> Result<()> {
    let mut last_frame_hash = 0u64;
    let mut last_refresh = Instant::now() - REFRESH_INTERVAL;
    let mut entries: Vec<BoardPr> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut matched = 0usize;
    let mut scroll: usize = 0;
    let mut search: Option<String> = None;
    // Transcript search state: cached scrollbacks plus the Tab-toggled
    // context view for the inline previews.
    let mut transcripts = TranscriptCache::default();
    let mut expanded = false;
    let mut detail = DEFAULT_DETAIL;
    let mut linger_days = DEFAULT_LINGER_DAYS;
    let mut include_ended = false;
    // Cloud fetches are throttled well below the local tick; toggling the
    // source or changing the day window forces one.
    let mut cloud_fetch_due = false;
    let mut cloud_fetched: Option<Instant> = None;
    let mut dirty = false;
    let mut needs_render = false;

    loop {
        if last_refresh.elapsed() >= REFRESH_INTERVAL {
            last_refresh = Instant::now();
            if overrides_fetched.elapsed() >= OVERRIDES_REFRESH {
                *overrides_fetched = Instant::now();
                if let Ok(fresh) = crate::cloud::fetch_pr_overrides_standalone().await {
                    *overrides = fresh;
                }
            }
            if include_ended {
                let stale = cloud_fetched.is_none_or(|at| at.elapsed() >= CLOUD_BOARD_REFRESH);
                if cloud_fetch_due || stale {
                    cloud_fetch_due = false;
                    cloud_fetched = Some(Instant::now());
                    match crate::cloud::fetch_pr_board_standalone(linger_days).await {
                        Ok(cloud) => entries = cloud_entries_to_board(cloud),
                        Err(_) => entries = aggregate(&gather()?, overrides, linger_days),
                    }
                }
            } else {
                entries = aggregate(&gather()?, overrides, linger_days);
            }
            needs_render = true;
        }

        let (width, height) = terminal_size().unwrap_or((120, 40));
        if needs_render {
            needs_render = false;
            let query = search.as_deref().unwrap_or("");
            // A PR stays visible when its metadata matches, or when any of
            // its sessions' transcripts contain the query — with the matched
            // excerpt shown inline so the hit can be confirmed.
            let filtered: Vec<BoardRow> = entries
                .iter()
                .filter_map(|entry| {
                    let preview_lines =
                        build_previews(entry, &mut transcripts, query, width, expanded);
                    (matches_search(&entry.pr, query) || !preview_lines.is_empty()).then_some(
                        BoardRow {
                            entry,
                            preview_lines,
                        },
                    )
                })
                .collect();
            matched = filtered.len();
            let fresh = render(&filtered, width, linger_days, include_ended, detail);
            let hash = frame_hash(&fresh);
            if hash != last_frame_hash {
                last_frame_hash = hash;
                lines = fresh;
                dirty = true;
            }
        }

        // The banner is pinned above the scrolled content while searching.
        let banner_rows = usize::from(search.is_some());
        let page = (height as usize).saturating_sub(banner_rows).max(1);
        let max_scroll = lines.len().saturating_sub(page);
        if scroll > max_scroll {
            scroll = max_scroll;
            dirty = true;
        }

        if dirty {
            dirty = false;
            write!(out, "{}", escape::CLEAR_SCREEN_HOME)?;
            if let Some(query) = &search {
                write!(
                    out,
                    "{}{}",
                    escape::cursor_to(1, 1),
                    search_banner(query, matched, width)
                )?;
            }
            for (i, line) in lines.iter().skip(scroll).take(page).enumerate() {
                write!(
                    out,
                    "{}{}",
                    escape::cursor_to((banner_rows + i) as u16 + 1, 1),
                    line
                )?;
            }
            out.flush()?;
        }

        if poll(Duration::from_millis(250))? {
            match read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(());
                    }
                    // While a filter is active, printable keys edit the query
                    // and Esc clears it; outside one, they're the shortcuts.
                    if let Some(query) = &mut search {
                        match key.code {
                            KeyCode::Esc => {
                                search = None;
                                expanded = false;
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            // Tab flips the transcript previews between one
                            // snippet and the surrounding context.
                            KeyCode::Tab => {
                                expanded = !expanded;
                                needs_render = true;
                                dirty = true;
                            }
                            KeyCode::Backspace => {
                                query.pop();
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                query.push(c);
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('/') => {
                                search = Some(String::new());
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            // Widen or narrow how long finished PRs linger.
                            // The filter runs during aggregation (server-side
                            // for the cloud view), so force a fresh pass
                            // rather than waiting out the tick.
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                linger_days = (linger_days + 1).min(MAX_LINGER_DAYS);
                                cloud_fetch_due = true;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                linger_days = linger_days.saturating_sub(1);
                                cloud_fetch_due = true;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            // Expand or collapse the detail level, one step
                            // at a time like +/- for days: compact ↔ standard
                            // ↔ titles ↔ recaps.
                            KeyCode::Char('e') => {
                                detail = (detail + 1).min(MAX_DETAIL);
                                needs_render = true;
                                dirty = true;
                            }
                            KeyCode::Char('c') => {
                                detail = detail.saturating_sub(1);
                                needs_render = true;
                                dirty = true;
                            }
                            // Flip between live mirrors and the durable cloud
                            // record, which includes ended sessions.
                            KeyCode::Char('a') => {
                                include_ended = !include_ended;
                                cloud_fetch_due = true;
                                scroll = 0;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            _ => {}
                        }
                    }
                    let target = match key.code {
                        KeyCode::Up => scroll.saturating_sub(1),
                        KeyCode::Down => scroll + 1,
                        KeyCode::PageUp => scroll.saturating_sub(page.saturating_sub(2)),
                        KeyCode::PageDown => scroll + page.saturating_sub(2),
                        KeyCode::Home if search.is_none() => 0,
                        KeyCode::End if search.is_none() => max_scroll,
                        KeyCode::Char('k') if search.is_none() => scroll.saturating_sub(1),
                        KeyCode::Char('j') if search.is_none() => scroll + 1,
                        KeyCode::Char('g') if search.is_none() => 0,
                        KeyCode::Char('G') if search.is_none() => max_scroll,
                        _ => scroll,
                    };
                    let target = target.min(max_scroll);
                    if target != scroll {
                        scroll = target;
                        dirty = true;
                    }
                }
                Event::Resize(..) => {
                    // Column widths depend on the terminal size; rebuild now.
                    last_refresh = Instant::now() - REFRESH_INTERVAL;
                    last_frame_hash = 0;
                    dirty = true;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_pr(number: u64, repo: &str) -> SessionPr {
        let mut pr = SessionPr::test_stub(number, "o", repo);
        pr.state = "OPEN".to_string();
        pr.refreshed_at = 1_000;
        pr
    }

    fn make_primary(pr: &mut SessionPr) {
        pr.primary = true;
        pr.primary_source = "auto".to_string();
    }

    fn render_frame_at(entries: &[BoardPr], detail: u8) -> String {
        let rows: Vec<BoardRow> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        render(&rows, 160, DEFAULT_LINGER_DAYS, false, detail).join("\n")
    }

    fn render_frame(entries: &[BoardPr]) -> String {
        render_frame_at(entries, DEFAULT_DETAIL)
    }

    fn now_ms() -> u64 {
        (now_secs() * 1000.0) as u64
    }

    fn snapshot(dir: &str, prs: Vec<SessionPr>) -> SessionSnapshot {
        SessionSnapshot {
            dir_name: dir.to_string(),
            session_dir: PathBuf::new(),
            last_updated: now_secs(),
            branch: String::new(),
            uncommitted_files: 0,
            title: String::new(),
            recap: None,
            prs,
        }
    }

    #[test]
    fn aggregation_merges_the_same_pr_across_sessions() {
        let mut a = board_pr(5, "portal");
        a.mentions = 10;
        a.user_mentions = 1;
        a.refreshed_at = 2_000;
        a.additions = 42;
        let mut b = board_pr(5, "portal");
        b.mentions = 3;
        b.refreshed_at = 1_000;
        b.slack_comment_urls = vec!["https://t.slack.com/archives/C1/p1".to_string()];

        let entries = aggregate(
            &[snapshot("one", vec![a]), snapshot("two", vec![b])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.pr.mentions, 13, "mentions sum across sessions");
        assert_eq!(entry.pr.additions, 42, "newest gh stats win");
        assert_eq!(entry.sessions.len(), 2);
        assert_eq!(entry.pr.slack_comment_urls.len(), 1);
    }

    #[test]
    fn aggregation_preserves_primary_from_an_older_placeholder() {
        let mut placeholder = SessionPr::test_stub(22, "o", "mcp");
        make_primary(&mut placeholder);
        placeholder.mentions = 7;

        let enriched = board_pr(22, "mcp");
        let entries = aggregate(
            &[
                snapshot("active", vec![placeholder]),
                snapshot("older", vec![enriched]),
            ],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );

        assert_eq!(entries.len(), 1);
        assert!(entries[0].pr.primary);
        assert_eq!(entries[0].pr.primary_source, "auto");
        assert_eq!(entries[0].pr.mentions, 7);
    }

    #[test]
    fn overrides_reshape_the_board() {
        let primary_key = "o/portal#5".to_string();
        let dismissed_key = "o/portal#6".to_string();
        let mut overrides = HashMap::new();
        overrides.insert(primary_key, PrDisposition::Primary);
        overrides.insert(dismissed_key, PrDisposition::Dismissed);

        let entries = aggregate(
            &[snapshot(
                "one",
                vec![board_pr(5, "portal"), board_pr(6, "portal")],
            )],
            &overrides,
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1, "dismissed PR is gone");
        assert!(entries[0].pr.primary, "override promotes");
    }

    #[test]
    fn attention_order_puts_failures_first() {
        let mut failing = board_pr(1, "portal");
        failing.checks_total = 3;
        failing.checks_failed = 1;
        let mut merged = board_pr(2, "portal");
        merged.state = "MERGED".to_string();
        merged.closed_at = now_ms();
        make_primary(&mut merged);
        let ready = {
            let mut pr = board_pr(3, "portal");
            pr.review_decision = "APPROVED".to_string();
            pr.checks_total = 2;
            pr.checks_passed = 2;
            pr
        };

        let entries = aggregate(
            &[snapshot("one", vec![merged, ready, failing])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        let numbers: Vec<u64> = entries.iter().map(|e| e.pr.number).collect();
        assert_eq!(numbers, vec![1, 3, 2], "failing → ready → merged");
    }

    #[test]
    fn render_groups_by_repo_and_shows_the_judgment() {
        let mut pr = board_pr(9, "portal");
        pr.title = "Fix the flow".to_string();
        pr.ai_note = "CI green, awaiting review".to_string();
        pr.ai_confidence = "medium".to_string();
        pr.slack_origin_url = "https://t.slack.com/archives/C1/p1".to_string();

        let entries = aggregate(
            &[snapshot("one", vec![pr])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        let frame = render_frame(&entries);
        assert!(frame.contains("o/portal"));
        assert!(frame.contains("CI green, awaiting review"));
        assert!(frame.contains("slack origin"));
        // Progress reads as a quiet bar — no percentage, no confidence prose.
        assert!(frame.contains('▓'), "progress bar renders");
        assert!(!frame.contains('%'), "bar carries no percentage");
        assert!(
            !frame.contains("confidence"),
            "confidence prose replaced by the bar"
        );
        // Old-binary sessions report no mention counts; don't render "0 mentions".
        assert!(!frame.contains("mentions"), "zero mentions stays silent");
    }

    #[test]
    fn mention_counts_render_only_when_present() {
        let mut pr = board_pr(9, "portal");
        pr.mentions = 12;
        pr.user_mentions = 3;
        let entries = aggregate(
            &[snapshot("one", vec![pr])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        let frame = render_frame(&entries);
        assert!(frame.contains("12 mentions (3 yours)"));
    }

    /// Several sessions often share a cwd; the board names each directory
    /// once with a multiplier instead of a flickering per-session age list.
    #[test]
    fn sessions_dedupe_by_directory_without_ages() {
        let pr = board_pr(9, "portal");
        let entries = aggregate(
            &[
                snapshot("developer-portal", vec![pr.clone()]),
                snapshot("developer-portal", vec![pr.clone()]),
                snapshot("builder-document-intent", vec![pr]),
            ],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        let text = sessions_text(&entries[0].sessions);
        assert_eq!(text, "developer-portal ×2, builder-document-intent");

        // A stopped mirror is tagged idle at minute granularity, never seconds.
        let stale = vec![SessionRef {
            dir_name: "old-worktree".to_string(),
            session_dir: None,
            age_secs: 1_260,
            ended: false,
            title: String::new(),
            recap: None,
        }];
        let text = sessions_text(&stale);
        assert!(text.contains("old-worktree"));
        assert!(text.contains("idle 21m"));
        assert!(!text.contains('s'), "no second-level ages: {text}");
    }

    /// Scanning artifacts stay hidden, while a classified primary remains
    /// visible so its failed or pending enrichment is actionable.
    #[test]
    fn only_primary_unverified_prs_reach_the_board() {
        let mut phantom = SessionPr::test_stub(500, "o", "r");
        phantom.state = "OPEN".to_string(); // refreshed_at stays 0
        let mut primary = SessionPr::test_stub(501, "o", "r");
        make_primary(&mut primary);
        let entries = aggregate(
            &[snapshot("one", vec![phantom, primary])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pr.number, 501);
    }

    /// Merged and closed primaries linger for a day, judged by close time when
    /// known, else the last mention; with neither signal they hide at once.
    #[test]
    fn finished_prs_age_off_after_a_day() {
        let day_ms: u64 = 24 * 3600 * 1000;
        let mut fresh = board_pr(1, "portal");
        fresh.state = "MERGED".to_string();
        fresh.closed_at = now_ms() - 3600 * 1000;
        let mut old = board_pr(2, "portal");
        old.state = "MERGED".to_string();
        old.closed_at = now_ms() - day_ms - 60_000;
        let mut recently_discussed = board_pr(3, "portal");
        recently_discussed.state = "CLOSED".to_string();
        recently_discussed.last_mentioned_at = now_ms() - 60_000;
        let mut silent = board_pr(4, "portal");
        silent.state = "CLOSED".to_string(); // no close time, no mentions

        for pr in [&mut fresh, &mut old, &mut recently_discussed, &mut silent] {
            make_primary(pr);
        }
        let entries = aggregate(
            &[snapshot(
                "one",
                vec![fresh, old, recently_discussed, silent],
            )],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        let numbers: Vec<u64> = entries.iter().map(|e| e.pr.number).collect();
        assert_eq!(
            numbers,
            vec![1, 3],
            "fresh merge and closed-but-discussed stay"
        );
    }

    /// +/- widen or narrow how far back finished PRs are pulled from.
    #[test]
    fn linger_window_scales_with_days() {
        let mut merged = board_pr(1, "portal");
        merged.state = "MERGED".to_string();
        merged.closed_at = now_ms() - 2 * 24 * 3600 * 1000; // two days ago
        make_primary(&mut merged);
        let snapshots = [snapshot("one", vec![merged])];

        assert!(aggregate(&snapshots, &HashMap::new(), 1).is_empty());
        assert_eq!(aggregate(&snapshots, &HashMap::new(), 3).len(), 1);
        // Zero shows open PRs only, no matter how fresh the merge.
        let mut fresh = board_pr(2, "portal");
        fresh.state = "MERGED".to_string();
        fresh.closed_at = now_ms();
        make_primary(&mut fresh);
        assert!(aggregate(&[snapshot("one", vec![fresh])], &HashMap::new(), 0).is_empty());
    }

    #[test]
    fn finished_secondaries_and_unmentioned_foreign_prs_hide_immediately() {
        let mut secondary = board_pr(1, "portal");
        secondary.state = "MERGED".to_string();
        secondary.closed_at = now_ms();

        let mut foreign = board_pr(2, "portal");
        foreign.state = "MERGED".to_string();
        foreign.closed_at = now_ms();
        foreign.authored_by_viewer = Some(false);
        make_primary(&mut foreign);

        let mut mentioned = foreign.clone();
        mentioned.number = 3;
        mentioned.url = "https://github.com/o/portal/pull/3".to_string();
        mentioned.user_mentions = 1;

        let entries = aggregate(
            &[snapshot("one", vec![secondary, foreign, mentioned])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pr.number, 3);
    }

    #[test]
    fn merged_prs_show_a_full_bar_regardless_of_ai_opinion() {
        let mut pr = board_pr(9, "portal");
        pr.state = "MERGED".to_string();
        pr.ai_confidence = "low".to_string();
        assert_eq!(progress_percent(&pr), 100);

        let mut open = board_pr(10, "portal");
        open.review_decision = "APPROVED".to_string();
        open.checks_total = 1;
        open.checks_passed = 1;
        assert_eq!(progress_percent(&open), 90, "ready to merge");
        open.ai_confidence = "low".to_string();
        assert_eq!(progress_percent(&open), 80, "low confidence nudges down");
    }

    #[test]
    fn search_filters_by_number_repo_title_and_branch() {
        let mut pr = board_pr(1089, "developer-portal");
        pr.title = "Slim the tool catalog".to_string();
        pr.branch = "sam/tool-diet".to_string();

        assert!(matches_search(&pr, "1089"));
        assert!(matches_search(&pr, "portal"));
        assert!(matches_search(&pr, "catalog"));
        assert!(matches_search(&pr, "TOOL-DIET"));
        assert!(matches_search(&pr, ""));
        assert!(!matches_search(&pr, "2557"));
    }

    #[test]
    fn find_ci_matches_case_insensitively_at_byte_boundaries() {
        assert_eq!(find_ci("Fix the WebGL shader", "webgl"), Some((8, 13)));
        assert_eq!(find_ci("nothing here", "webgl"), None);
        assert_eq!(find_ci("", "x"), None);
        assert_eq!(find_ci("text", ""), None);
        // Multibyte text before the match doesn't skew the byte range.
        let line = "⌕ búsqueda WebGL";
        let (start, end) = find_ci(line, "WEBGL").unwrap();
        assert_eq!(&line[start..end], "WebGL");
    }

    fn transcript() -> Vec<String> {
        (0..30)
            .map(|i| match i {
                5 => "> please fix the webgl shader".to_string(),
                6 => "Sure — looking at the WebGL setup now.".to_string(),
                20 => "The webgl fix is in PR #9.".to_string(),
                _ => format!("line {i}"),
            })
            .collect()
    }

    #[test]
    fn transcript_matches_find_every_line_containing_the_query() {
        let lines = transcript();
        assert_eq!(transcript_match_lines(&lines, "webgl"), vec![5, 6, 20]);
        assert!(transcript_match_lines(&lines, "nope").is_empty());
    }

    /// Collapsed shows only the most recent matching line; expanded shows the
    /// last few matches with context, merging windows that touch and marking
    /// the jump between distant groups.
    #[test]
    fn preview_rows_collapse_and_expand() {
        let lines = transcript();
        let matches = transcript_match_lines(&lines, "webgl");

        let collapsed = preview_rows(&lines, &matches, false);
        assert_eq!(collapsed.len(), 1);
        assert!(collapsed[0].text.contains("PR #9"));
        assert!(collapsed[0].is_match);

        let expanded = preview_rows(&lines, &matches, true);
        // Lines 3..=8 (5 and 6 merged) plus 18..=22 around line 20.
        assert_eq!(expanded.len(), 11);
        let match_count = expanded.iter().filter(|r| r.is_match).count();
        assert_eq!(match_count, 3);
        let gaps = expanded.iter().filter(|r| r.gap_before).count();
        assert_eq!(gaps, 1, "one separator between the two context groups");
        assert!(!expanded[0].is_match, "context precedes the first match");
    }

    #[test]
    fn window_keeps_a_far_right_match_visible() {
        let line = format!("{}needle at the end", "x".repeat(200));
        let windowed = window_around_match(&line, "needle", 40);
        assert!(windowed.starts_with('…'));
        assert!(windowed.contains("needle"));
    }

    #[test]
    fn highlight_wraps_every_occurrence() {
        let restore = fg(color::GRAY);
        let out = highlight_query("webgl and WebGL", "webgl", &restore);
        assert_eq!(out.matches(&escape::bg(color::YELLOW)).count(), 2);
        assert!(out.contains("and"));
    }

    /// A transcript hit keeps a PR visible even when its metadata doesn't
    /// match, and the excerpt renders inline with the session named.
    #[test]
    fn transcript_hits_surface_prs_with_inline_previews() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("scrollback.log"),
            "line one\nthe \x1b[32mwebgl\x1b[0m shader is fixed\nline three\n",
        )
        .unwrap();

        let mut entry_pr = board_pr(9, "portal");
        entry_pr.title = "Fix the flow".to_string();
        let entry = BoardPr {
            pr: entry_pr,
            sessions: vec![SessionRef {
                dir_name: "portal".to_string(),
                session_dir: Some(dir.path().to_path_buf()),
                age_secs: 10,
                ended: false,
                title: String::new(),
                recap: None,
            }],
            uncommitted: 0,
            stale: false,
        };

        let mut cache = TranscriptCache::default();
        assert!(!matches_search(&entry.pr, "webgl"), "metadata misses");
        let preview = build_previews(&entry, &mut cache, "webgl", 120, false);
        assert_eq!(preview.len(), 1, "one collapsed line per session");
        assert!(preview[0].contains("portal"), "session is named");
        assert!(preview[0].contains("shader is fixed"), "ANSI stripped");
        assert!(preview[0].contains("1 match"));

        // Below the minimum query length, transcripts stay out of it.
        assert!(build_previews(&entry, &mut cache, "we", 120, false).is_empty());
        // Cloud rows have no local transcript to search.
        let mut cloud = entry;
        cloud.sessions[0].session_dir = None;
        assert!(build_previews(&cloud, &mut cache, "webgl", 120, false).is_empty());
    }

    /// Sessions carrying a title and recap for the detail levels.
    fn titled_entries() -> Vec<BoardPr> {
        let mut with_title = snapshot("portal", vec![board_pr(9, "portal")]);
        with_title.title = "Wiring the PR board detail levels".to_string();
        with_title.recap = Some(RecapBrief {
            headline: "Added e-cycled detail to the prs board".to_string(),
            generated_at: now_ms() - 42 * 60 * 1000,
            line_delta: crate::recap::TurnLineDelta {
                additions: 120,
                deletions: 35,
            },
        });
        let bare = snapshot("other-dir", vec![board_pr(9, "portal")]);
        aggregate(&[with_title, bare], &HashMap::new(), DEFAULT_LINGER_DAYS)
    }

    /// `e` steps through four detail levels: compact keeps only the identity
    /// row; standard adds progress; titles adds each session's terminal
    /// title; recaps adds the latest recap headline.
    #[test]
    fn detail_levels_reveal_titles_then_recaps() {
        let entries = titled_entries();

        let compact = render_frame_at(&entries, 0);
        assert!(!compact.contains('▓'), "no progress bar at compact");
        assert!(!compact.contains("Wiring the PR board"), "no titles");
        assert!(compact.contains("#9"), "identity row stays");
        assert!(compact.contains("compact"), "header names the level");

        let standard = render_frame_at(&entries, 1);
        assert!(standard.contains('▓'), "progress bar returns");
        assert!(
            !standard.contains("Wiring the PR board"),
            "titles wait for level 2"
        );

        let titles = render_frame_at(&entries, 2);
        assert!(titles.contains("Wiring the PR board detail levels"));
        assert!(
            !titles.contains("Added e-cycled detail"),
            "recaps wait for level 3"
        );

        let recaps = render_frame_at(&entries, 3);
        assert!(recaps.contains("Wiring the PR board detail levels"));
        assert!(recaps.contains("Added e-cycled detail to the prs board"));
        // The recap line carries the turn's diff and its staleness.
        assert!(recaps.contains("+120"), "recap shows the line delta");
        assert!(recaps.contains("-35"));
        assert!(recaps.contains("42m ago"), "recap shows its age");
    }

    /// Sessions with neither a title nor a recap contribute no detail lines,
    /// and duplicate (dir, title, recap) triples collapse to one.
    #[test]
    fn session_detail_lines_skip_empty_and_dedupe() {
        let entries = titled_entries();
        let now = now_ms();
        let lines = session_detail_lines(&entries[0].sessions, 160, 3, now);
        assert_eq!(lines.len(), 2, "one title line + one recap line: {lines:?}");
        assert!(lines[0].contains("portal"));
        assert!(lines[0].contains("ago"), "title line carries its age");
        assert!(lines[1].contains("Added e-cycled detail"));

        let doubled: Vec<SessionRef> = entries[0]
            .sessions
            .iter()
            .chain(entries[0].sessions.iter())
            .map(|s| SessionRef {
                dir_name: s.dir_name.clone(),
                session_dir: s.session_dir.clone(),
                age_secs: s.age_secs,
                ended: s.ended,
                title: s.title.clone(),
                recap: s.recap.clone(),
            })
            .collect();
        assert_eq!(session_detail_lines(&doubled, 160, 3, now).len(), 2);

        // At level 2 a recap-only session shows nothing yet.
        let recap_only = vec![SessionRef {
            dir_name: "quiet".to_string(),
            session_dir: None,
            age_secs: 5,
            ended: false,
            title: String::new(),
            recap: Some(RecapBrief {
                headline: "Half-finished refactor".to_string(),
                generated_at: 0,
                line_delta: crate::recap::TurnLineDelta::default(),
            }),
        }];
        assert!(session_detail_lines(&recap_only, 160, 2, now).is_empty());
        let at_three = session_detail_lines(&recap_only, 160, 3, now);
        assert_eq!(at_three.len(), 2, "dir header plus recap line");
        assert!(at_three[1].contains("Half-finished refactor"));
    }
}
