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
//! minute so dashboard toggles apply here too. Only sessions with a live
//! mirror under /tmp appear; the durable history lives on the web board.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{stdout, Write};
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
/// A mirror this old is a session that stopped updating; its rows dim.
const STALE_SESSION_SECS: f64 = 300.0;
/// How long merged/closed PRs linger by default; +/- adjusts at runtime.
const DEFAULT_LINGER_DAYS: u64 = 1;
/// Ceiling for the +key so the window can't run away unbounded.
const MAX_LINGER_DAYS: u64 = 90;

/// One live session's contribution to the board.
struct SessionSnapshot {
    dir_name: String,
    last_updated: f64,
    branch: String,
    uncommitted_files: usize,
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
    age_secs: u64,
}

/// Read every live mirror, one snapshot per session.
fn gather() -> Result<Vec<SessionSnapshot>> {
    let mut snapshots = Vec::new();
    for (_path, data) in crate::inspect::discover_instances(&None)? {
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
            prs,
        });
    }
    Ok(snapshots)
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Merge session snapshots into deduped board entries, honoring overrides.
/// `linger_days` bounds how long finished PRs stay visible (0 = open only).
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

            // Newest GitHub stats win, but the engagement counters belong to the
            // merge rather than to any one session's copy, so carry them over.
            if pr.refreshed_at > entry.pr.refreshed_at {
                let previous = std::mem::replace(&mut entry.pr, pr.clone());
                entry.pr.mentions = previous.mentions;
                entry.pr.user_mentions = previous.user_mentions;
                entry.pr.first_mentioned_at = previous.first_mentioned_at;
                entry.pr.last_mentioned_at = previous.last_mentioned_at;
                entry.pr.last_mention_prompt = previous.last_mention_prompt;
            }
            entry.pr.mentions += pr.mentions;
            entry.pr.user_mentions += pr.user_mentions;
            entry.pr.last_mentioned_at = entry.pr.last_mentioned_at.max(pr.last_mentioned_at);
            entry.pr.primary |= pr.primary;
            entry.pr.dismissed |= pr.dismissed;
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
                age_secs: session_age as u64,
            });
        }
    }

    let now_ms = (now * 1000.0) as u64;
    let mut out: Vec<BoardPr> = order
        .into_iter()
        .filter_map(|key| {
            let mut entry = merged.remove(&key)?;
            // A PR gh never confirmed is a scanning artifact — a doc example
            // like `o/r#500` or a line-wrapped `owner/repo#N` shorthand. The
            // session's own strip may show it mid-enrichment; the board waits
            // for verification.
            if entry.pr.refreshed_at == 0 {
                return None;
            }
            // Finished PRs age off after the linger window (0 = open only),
            // judged by the close time when known, else by the last time any
            // session spoke about them.
            if entry.pr.state != "OPEN" {
                let latest = entry.pr.closed_at.max(entry.pr.last_mentioned_at);
                if linger_days == 0
                    || latest == 0
                    || now_ms.saturating_sub(latest) > linger_days * 24 * 3600 * 1000
                {
                    return None;
                }
            }
            match overrides.get(&key) {
                Some(PrDisposition::Dismissed) => return None,
                Some(PrDisposition::Primary) => entry.pr.primary = true,
                Some(PrDisposition::Secondary) => entry.pr.primary = false,
                None => {}
            }
            (!entry.pr.dismissed).then_some(entry)
        })
        .collect();

    // Twins first (same non-empty head branch stays adjacent), then repo
    // grouping and stage ordering happen in render.
    out.sort_by(|a, b| {
        stage(&a.pr)
            .rank
            .cmp(&stage(&b.pr).rank)
            .then(b.pr.primary.cmp(&a.pr.primary))
            .then(b.pr.last_mentioned_at.cmp(&a.pr.last_mentioned_at))
    });
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
    let (rank, label, color) = if pr.state == "MERGED" {
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

/// Ten-cell progress bar in the stage color: `▓▓▓▓▓▓▓░░░  70%`.
fn progress_bar(pr: &SessionPr) -> String {
    let pct = progress_percent(pr);
    let stage = stage(pr);
    let filled = (usize::from(pct) + 5) / 10;
    format!(
        "{}{}{}{}{} {:>3}%{}",
        fg(stage.color),
        "▓".repeat(filled),
        fg(color::DARK_GRAY),
        "░".repeat(10 - filled),
        fg(stage.color),
        pct,
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
    let mut seen: Vec<(String, usize, u64)> = Vec::new(); // (name, count, min idle age)
    for session in sessions {
        match seen
            .iter_mut()
            .find(|(name, _, _)| *name == session.dir_name)
        {
            Some((_, count, min_age)) => {
                *count += 1;
                *min_age = (*min_age).min(session.age_secs);
            }
            None => seen.push((session.dir_name.clone(), 1, session.age_secs)),
        }
    }
    seen.iter()
        .map(|(name, count, min_age)| {
            let times = if *count > 1 {
                format!(" ×{count}")
            } else {
                String::new()
            };
            // A live mirror updates constantly; only a stopped one gets a tag.
            let idle = if *min_age as f64 > STALE_SESSION_SECS {
                format!(
                    " {}(idle {}){}",
                    fg(color::DARK_GRAY),
                    format_age(*min_age),
                    fg(color::GRAY)
                )
            } else {
                String::new()
            };
            format!("{name}{times}{idle}")
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

/// Build one full frame as displayable lines (no trailing newline handling).
fn render(entries: &[&BoardPr], width: u16, linger_days: u64) -> Vec<String> {
    let now_ms = (now_secs() * 1000.0) as u64;
    let session_count = entries
        .iter()
        .flat_map(|e| e.sessions.iter().map(|s| s.dir_name.as_str()))
        .collect::<HashSet<_>>()
        .len();
    let window = match linger_days {
        0 => "open only".to_string(),
        days => format!("done ≤ {days}d"),
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "{}⑆ Crabigator PR board{}  {}{} PRs · {} sessions · {} · ↑↓ scroll · / search · +/- days · q quit{}",
        fg(color::PURPLE),
        RESET_FG,
        fg(color::DARK_GRAY),
        entries.len(),
        session_count,
        window,
        RESET_FG,
    ));
    lines.push(String::new());

    if entries.is_empty() {
        lines.push(format!(
            "{}No PRs tracked by any live session.{}",
            fg(color::GRAY),
            RESET_FG
        ));
        return lines;
    }

    // Group by repo, preserving the attention ordering for group placement.
    let mut repo_order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<&BoardPr>> = HashMap::new();
    for entry in entries {
        let repo = format!("{}/{}", entry.pr.owner, entry.pr.repo);
        if !groups.contains_key(&repo) {
            repo_order.push(repo.clone());
        }
        groups.entry(repo).or_default().push(*entry);
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

        let refs: Vec<&SessionPr> = group.iter().map(|e| &e.pr).collect();
        let widths = PrColumnWidths::from_pr_refs(&refs, width as usize);
        for entry in group {
            let mut row = pr_row_text(width, &entry.pr, &widths);
            if entry.stale {
                row = format!("{}{row}", fg(color::DARK_GRAY));
            }
            lines.push(format!("{row}{RESET}"));

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
        let refs: Vec<&BoardPr> = entries.iter().collect();
        for line in render(&refs, width, DEFAULT_LINGER_DAYS) {
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
        " /{query}▏ {matched} match{} · type to filter · Esc clears ",
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
    let mut linger_days = DEFAULT_LINGER_DAYS;
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
            entries = aggregate(&gather()?, overrides, linger_days);
            needs_render = true;
        }

        let (width, height) = terminal_size().unwrap_or((120, 40));
        if needs_render {
            needs_render = false;
            let query = search.as_deref().unwrap_or("");
            let filtered: Vec<&BoardPr> = entries
                .iter()
                .filter(|entry| matches_search(&entry.pr, query))
                .collect();
            matched = filtered.len();
            let fresh = render(&filtered, width, linger_days);
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
                                scroll = 0;
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
                            // The filter runs during aggregation, so force a
                            // fresh pass rather than waiting out the tick.
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                linger_days = (linger_days + 1).min(MAX_LINGER_DAYS);
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                linger_days = linger_days.saturating_sub(1);
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

    fn render_frame(entries: &[BoardPr]) -> String {
        let refs: Vec<&BoardPr> = entries.iter().collect();
        render(&refs, 160, DEFAULT_LINGER_DAYS).join("\n")
    }

    fn now_ms() -> u64 {
        (now_secs() * 1000.0) as u64
    }

    fn snapshot(dir: &str, prs: Vec<SessionPr>) -> SessionSnapshot {
        SessionSnapshot {
            dir_name: dir.to_string(),
            last_updated: now_secs(),
            branch: String::new(),
            uncommitted_files: 0,
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
        // Progress reads as a bar and percent, not confidence prose.
        assert!(
            frame.contains('▓') && frame.contains('%'),
            "progress bar renders"
        );
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
            age_secs: 1_260,
        }];
        let text = sessions_text(&stale);
        assert!(text.contains("old-worktree"));
        assert!(text.contains("idle 21m"));
        assert!(!text.contains('s'), "no second-level ages: {text}");
    }

    /// Scanning artifacts (`o/r#500` doc examples, wrapped `owner/repo#N`
    /// shorthand) never enrich; the board hides anything gh hasn't confirmed.
    #[test]
    fn unverified_prs_stay_off_the_board() {
        let mut phantom = SessionPr::test_stub(500, "o", "r");
        phantom.state = "OPEN".to_string(); // refreshed_at stays 0
        let entries = aggregate(
            &[snapshot("one", vec![phantom])],
            &HashMap::new(),
            DEFAULT_LINGER_DAYS,
        );
        assert!(entries.is_empty());
    }

    /// Merged and closed PRs linger for a day, judged by close time when
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

        silent.dismissed = false;
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
        let snapshots = [snapshot("one", vec![merged])];

        assert!(aggregate(&snapshots, &HashMap::new(), 1).is_empty());
        assert_eq!(aggregate(&snapshots, &HashMap::new(), 3).len(), 1);
        // Zero shows open PRs only, no matter how fresh the merge.
        let mut fresh = board_pr(2, "portal");
        fresh.state = "MERGED".to_string();
        fresh.closed_at = now_ms();
        assert!(aggregate(&[snapshot("one", vec![fresh])], &HashMap::new(), 0).is_empty());
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
}
