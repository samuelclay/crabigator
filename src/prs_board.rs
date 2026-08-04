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
fn aggregate(
    snapshots: &[SessionSnapshot],
    overrides: &HashMap<String, PrDisposition>,
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

    let mut out: Vec<BoardPr> = order
        .into_iter()
        .filter_map(|key| {
            let mut entry = merged.remove(&key)?;
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

/// Five-dot progress strip: draft → open → CI → review → merged.
fn checklist(pr: &SessionPr) -> String {
    let done = |on: bool| if on { "●" } else { "○" };
    let drafted = format!("{}●", fg(color::GREEN));
    let opened = if pr.state == "CLOSED" {
        format!("{}✗", fg(color::RED))
    } else {
        format!(
            "{}{}",
            fg(color::GREEN),
            done(!pr.is_draft || pr.state != "OPEN")
        )
    };
    let ci = if pr.checks_total == 0 {
        format!("{}○", fg(color::DARK_GRAY))
    } else if pr.checks_failed > 0 {
        format!("{}✗", fg(color::RED))
    } else if pr.checks_pending > 0 {
        format!("{}◐", fg(color::YELLOW))
    } else {
        format!("{}●", fg(color::GREEN))
    };
    let review = match pr.review_decision.as_str() {
        "APPROVED" => format!("{}●", fg(color::GREEN)),
        "CHANGES_REQUESTED" => format!("{}✗", fg(color::RED)),
        _ => format!("{}○", fg(color::DARK_GRAY)),
    };
    let merged = if pr.state == "MERGED" {
        format!("{}●", fg(color::PURPLE))
    } else {
        format!("{}○", fg(color::DARK_GRAY))
    };
    format!("{drafted}{opened}{ci}{review}{merged}{RESET_FG}")
}

fn format_age(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86399 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86400),
    }
}

/// Build one full frame as displayable lines (no trailing newline handling).
fn render(entries: &[BoardPr], width: u16) -> Vec<String> {
    let now_ms = (now_secs() * 1000.0) as u64;
    let session_count = entries
        .iter()
        .flat_map(|e| e.sessions.iter().map(|s| s.dir_name.as_str()))
        .collect::<HashSet<_>>()
        .len();

    let mut lines = Vec::new();
    lines.push(format!(
        "{}⑆ Crabigator PR board{}  {}{} PRs · {} sessions · q to quit{}",
        fg(color::PURPLE),
        RESET_FG,
        fg(color::DARK_GRAY),
        entries.len(),
        session_count,
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
        groups.entry(repo).or_default().push(entry);
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
            let sessions = entry
                .sessions
                .iter()
                .map(|s| format!("{}({})", s.dir_name, format_age(s.age_secs)))
                .collect::<Vec<_>>()
                .join(", ");
            let mention_age = if entry.pr.last_mentioned_at > 0 {
                let secs = now_ms.saturating_sub(entry.pr.last_mentioned_at) / 1000;
                format!(" · spoken {} ago", format_age(secs))
            } else {
                String::new()
            };
            let yours = if entry.pr.user_mentions > 0 {
                format!(" ({} yours)", entry.pr.user_mentions)
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
                "   {} {}{}{} · {}{}{}{} · {} mentions{}{}",
                checklist(&entry.pr),
                fg(stage.color),
                stage.label,
                RESET_FG,
                fg(color::GRAY),
                sessions,
                mention_age,
                RESET_FG,
                entry.pr.mentions,
                yours,
                uncommitted,
            ));

            // Row 3: the recap's judgment and Slack links, when we have them.
            let mut extras: Vec<String> = Vec::new();
            if !entry.pr.ai_note.is_empty() {
                let confidence = if entry.pr.ai_confidence.is_empty() {
                    String::new()
                } else {
                    format!(" ({} confidence it's done)", entry.pr.ai_confidence)
                };
                extras.push(format!(
                    "{}✦ {}{}{}",
                    fg(color::YELLOW),
                    entry.pr.ai_note,
                    confidence,
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
        let entries = aggregate(&gather()?, &overrides);
        for line in render(&entries, width) {
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

async fn board_loop(
    out: &mut std::io::Stdout,
    overrides: &mut HashMap<String, PrDisposition>,
    overrides_fetched: &mut Instant,
) -> Result<()> {
    let mut last_frame_hash = 0u64;
    let mut last_refresh = Instant::now() - REFRESH_INTERVAL;

    loop {
        if last_refresh.elapsed() >= REFRESH_INTERVAL {
            last_refresh = Instant::now();
            if overrides_fetched.elapsed() >= OVERRIDES_REFRESH {
                *overrides_fetched = Instant::now();
                if let Ok(fresh) = crate::cloud::fetch_pr_overrides_standalone().await {
                    *overrides = fresh;
                }
            }
            let (width, height) = terminal_size().unwrap_or((120, 40));
            let entries = aggregate(&gather()?, overrides);
            let lines = render(&entries, width);

            let hash = frame_hash(&lines);
            if hash != last_frame_hash {
                last_frame_hash = hash;
                write!(out, "{}", escape::CLEAR_SCREEN_HOME)?;
                for (i, line) in lines.iter().take(height as usize).enumerate() {
                    write!(out, "{}{}", escape::cursor_to(i as u16 + 1, 1), line)?;
                }
                out.flush()?;
            }
        }

        if poll(Duration::from_millis(250))? {
            if let Event::Key(key) = read()? {
                let quit = matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL));
                if quit {
                    return Ok(());
                }
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

        let entries = aggregate(&[snapshot("one", vec![pr])], &HashMap::new());
        let frame = render(&entries, 160).join("\n");
        assert!(frame.contains("o/portal"));
        assert!(frame.contains("CI green, awaiting review"));
        assert!(frame.contains("medium confidence"));
        assert!(frame.contains("slack origin"));
    }
}
