//! `crabigator prs` — the live cross-session PR board.
//!
//! Aggregates every tracked PR from every running session's `inspect.json`
//! mirror into one read-only, auto-refreshing view: grouped first by activity
//! recency and then by repository, with cross-repo twins (same head branch)
//! kept together, with optional session titles, recaps, and clickable links.
//!
//! The board never talks to `gh` itself — it renders what the sessions
//! already know, with honest ages. Cloud dispositions are fetched once a
//! minute so dashboard toggles apply here too. The default view reads live
//! session mirrors under /tmp; `s` flips to the durable cloud record, which
//! includes ended sessions' PRs (tagged for resurrection) at ~1min lag.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{stdout, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{
    poll, read, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size as terminal_size, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use unicode_width::UnicodeWidthStr;

use crate::platforms::{PlatformKind, SessionState};
use crate::pr::{SessionPr, WatchAdd};
use crate::pr_rank::{attached_to_worktree, PrDisposition, ScopedOverrides};
use crate::slack::{SlackDirectory, SlackThread};
use crate::terminal::escape::{self, color, fg, RESET, RESET_FG, RESET_UNDERLINE, UNDERLINE};
use crate::ui::cooldown::Cooldowns;
use crate::ui::pr_cells::{
    board_detail_row_text, session_row_text_with_activity, PrCell, PrColumnWidths, BRANCH_PREFIX,
    PR_RIGHT_COLUMN_GAP,
};
use crate::ui::{COMPLETION_ICON, PROMPT_ICON};

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
/// Keep the first load visibly progressive without stretching it out.
const INITIAL_LOAD_BATCH: usize = 4;
/// `r` toggles between one titled row and the complete recap detail.
const MAX_DETAIL: u8 = 1;
const DEFAULT_DETAIL: u8 = 0;

/// Recency uses one cyan-blue hue at steadily lower intensities until old
/// activity becomes neutral gray after a day.
const RECENCY_1H: u8 = 51;
const RECENCY_3H: u8 = 45;
const RECENCY_6H: u8 = 39;
const RECENCY_9H: u8 = 33;
const RECENCY_12H: u8 = 27;
const RECENCY_24H: u8 = 25;
/// The event that matches the session's current state.
const ACTIVITY_BRIGHT_TEAL: u8 = 51;
/// The previous event in the prompt/completion pair.
const ACTIVITY_DARK_TEAL: u8 = 30;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RecencyBucket {
    LastHour,
    LastThreeHours,
    LastSixHours,
    LastNineHours,
    LastTwelveHours,
    LastDay,
    Older,
}

const DEFAULT_OLDEST_VISIBLE_BUCKET: RecencyBucket = RecencyBucket::Older;

impl RecencyBucket {
    fn from_age(age_secs: u64) -> Self {
        match age_secs {
            0..=3_599 => Self::LastHour,
            3_600..=10_799 => Self::LastThreeHours,
            10_800..=21_599 => Self::LastSixHours,
            21_600..=32_399 => Self::LastNineHours,
            32_400..=43_199 => Self::LastTwelveHours,
            43_200..=86_400 => Self::LastDay,
            _ => Self::Older,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::LastHour => "Last hour",
            Self::LastThreeHours => "1–3 hours",
            Self::LastSixHours => "3–6 hours",
            Self::LastNineHours => "6–9 hours",
            Self::LastTwelveHours => "9–12 hours",
            Self::LastDay => "12–24 hours",
            Self::Older => "Older",
        }
    }

    fn color(self) -> u8 {
        match self {
            Self::LastHour => RECENCY_1H,
            Self::LastThreeHours => RECENCY_3H,
            Self::LastSixHours => RECENCY_6H,
            Self::LastNineHours => RECENCY_9H,
            Self::LastTwelveHours => RECENCY_12H,
            Self::LastDay => RECENCY_24H,
            Self::Older => color::DARK_GRAY,
        }
    }

    fn heading_text_color(self) -> u8 {
        match self {
            Self::LastHour | Self::LastThreeHours | Self::LastSixHours | Self::LastNineHours => {
                color::BLACK
            }
            Self::LastTwelveHours | Self::LastDay | Self::Older => color::WHITE,
        }
    }

    fn next_cutoff(self) -> Self {
        match self {
            Self::Older => Self::LastHour,
            Self::LastHour => Self::LastThreeHours,
            Self::LastThreeHours => Self::LastSixHours,
            Self::LastSixHours => Self::LastNineHours,
            Self::LastNineHours => Self::LastTwelveHours,
            Self::LastTwelveHours => Self::LastDay,
            Self::LastDay => Self::Older,
        }
    }

    fn max_age_label(self) -> &'static str {
        match self {
            Self::LastHour => "1h",
            Self::LastThreeHours => "3h",
            Self::LastSixHours => "6h",
            Self::LastNineHours => "9h",
            Self::LastTwelveHours => "12h",
            Self::LastDay => "24h",
            Self::Older => "all",
        }
    }

    fn from_max_age_hours(hours: Option<u64>) -> Self {
        match hours {
            Some(0..=1) => Self::LastHour,
            Some(2..=3) => Self::LastThreeHours,
            Some(4..=6) => Self::LastSixHours,
            Some(7..=9) => Self::LastNineHours,
            Some(10..=12) => Self::LastTwelveHours,
            Some(13..=24) => Self::LastDay,
            Some(_) | None => Self::Older,
        }
    }

    fn max_age_hours(self) -> Option<u64> {
        match self {
            Self::LastHour => Some(1),
            Self::LastThreeHours => Some(3),
            Self::LastSixHours => Some(6),
            Self::LastNineHours => Some(9),
            Self::LastTwelveHours => Some(12),
            Self::LastDay => Some(24),
            Self::Older => None,
        }
    }
}

/// What one board block stands for: a primary PR with every touching session
/// beneath it (the default view), or a session with every PR it touches
/// beneath it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum BoardMode {
    Sessions,
    #[default]
    Prs,
}

impl BoardMode {
    fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("sessions") {
            Self::Sessions
        } else {
            Self::Prs
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Sessions => "sessions",
            Self::Prs => "prs",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BoardView {
    detail: u8,
    oldest_visible_bucket: RecencyBucket,
    mode: BoardMode,
}

impl BoardView {
    const fn new(detail: u8, oldest_visible_bucket: RecencyBucket) -> Self {
        Self {
            detail: if detail > MAX_DETAIL {
                MAX_DETAIL
            } else {
                detail
            },
            oldest_visible_bucket,
            mode: BoardMode::Prs,
        }
    }

    const fn with_mode(mut self, mode: BoardMode) -> Self {
        self.mode = mode;
        self
    }

    fn toggle_recap(&mut self) {
        self.detail = if self.detail == MAX_DETAIL {
            DEFAULT_DETAIL
        } else {
            MAX_DETAIL
        };
    }

    fn cycle_age(&mut self) {
        self.oldest_visible_bucket = self.oldest_visible_bucket.next_cutoff();
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            BoardMode::Sessions => BoardMode::Prs,
            BoardMode::Prs => BoardMode::Sessions,
        };
    }
}

impl Default for BoardView {
    fn default() -> Self {
        Self::new(DEFAULT_DETAIL, DEFAULT_OLDEST_VISIBLE_BUCKET)
    }
}

/// Header name for each detail level.
fn detail_name(detail: u8) -> &'static str {
    match detail {
        0 => "compact",
        1 => "recap",
        _ => "compact",
    }
}

/// One of the header's view toggles, as ` · <key> <state>`. A setting that is
/// off its default turns yellow so a changed view is obvious at a glance.
fn header_control(key: char, state: &str, changed: bool) -> String {
    if changed {
        format!(
            " · {}{UNDERLINE}{key}{RESET_UNDERLINE} {state}{}",
            fg(color::YELLOW),
            fg(color::DARK_GRAY)
        )
    } else {
        format!(" · {UNDERLINE}{key}{RESET_UNDERLINE} {state}")
    }
}

/// The slice of a session's latest recap the detail view renders.
#[derive(Clone)]
struct RecapBrief {
    headline: String,
    bullets: Vec<String>,
    next_prompt_notes: Vec<String>,
    artifacts: Vec<String>,
    /// Unix ms when the recap was generated; 0 when unknown.
    generated_at: u64,
    line_delta: crate::recap::TurnLineDelta,
}

/// One live session's contribution to the board.
struct SessionSnapshot {
    session_id: String,
    platform: PlatformKind,
    /// The session's full working directory, for matching path-scoped
    /// dispositions.
    cwd: String,
    /// The scope this session's PR dispositions use: 'path:<cwd>' in a linked
    /// worktree, else 'session:<id>'.
    pr_scope: String,
    dir_name: String,
    repo_owner: String,
    repo_name: String,
    /// The session's /tmp mirror directory, where scrollback.log lives.
    session_dir: PathBuf,
    last_updated: f64,
    branch: String,
    additions: i64,
    deletions: i64,
    /// The session's current terminal title (generated or OSC-published).
    title: String,
    /// Unix ms when the current title was set; 0 for legacy mirrors.
    title_set_at: u64,
    /// Slack thread metadata learned during this session.
    slack_threads: Vec<SlackThread>,
    /// The session's latest recap, when one has been generated.
    recap: Option<RecapBrief>,
    /// The session's current effective state.
    state: SessionState,
    /// Unix seconds when the prompt count last changed.
    prompted_at: u64,
    /// Unix seconds when the completion count last changed.
    completed_at: u64,
    prs: Vec<SessionPr>,
}

/// One PR aggregated across every session that mentions it.
#[derive(Clone)]
struct BoardPr {
    /// Freshest copy of the GitHub stats (newest `refreshed_at` wins).
    pr: SessionPr,
    sessions: Vec<SessionRef>,
    slack_threads: Vec<SlackThread>,
    stale: bool,
}

#[derive(Clone)]
struct SessionRef {
    session_id: String,
    platform: PlatformKind,
    /// The scope this session's PR dispositions use — its sub-row action
    /// links store dispositions there instead of group-wide.
    pr_scope: String,
    dir_name: String,
    /// Local mirror directory holding scrollback.log; None for cloud records,
    /// whose transcripts aren't reachable from this machine.
    session_dir: Option<PathBuf>,
    /// The session's current terminal title (detail level 1+).
    title: String,
    /// Unix ms when the current title was set; 0 when unknown.
    title_set_at: u64,
    /// The branch checked out in the session's worktree; empty when unknown.
    branch: String,
    /// The session's latest recap (detail level 2).
    recap: Option<RecapBrief>,
    /// The session's current effective state.
    state: SessionState,
    /// Unix seconds when the session last received a prompt.
    prompted_at: u64,
    /// Unix seconds when the session's completion count last changed.
    completed_at: u64,
    /// The session is over. Always false for live local mirrors; cloud
    /// records carry the durable answer.
    ended: bool,
}

/// One session-view block: a session with every PR it touches beneath it —
/// the transpose of a PR-view block.
#[derive(Clone)]
struct SessionEntry {
    session: SessionRef,
    /// The PR entries this session touches, in the board's attention order.
    prs: Vec<BoardPr>,
    stale: bool,
}

/// Active sessions in a repository/branch that has no matching visible PR row.
#[derive(Clone)]
struct WorkspaceEntry {
    repo_owner: String,
    repo_name: String,
    branch: String,
    session: SessionRef,
    additions: i64,
    deletions: i64,
}

type LocalBoard = (Vec<BoardPr>, Vec<WorkspaceEntry>, HashMap<String, PathBuf>);

impl WorkspaceEntry {
    fn sessions(&self) -> &[SessionRef] {
        std::slice::from_ref(&self.session)
    }
}

#[derive(Clone, Copy, Default)]
struct ActivityTimes {
    prompted_at: u64,
    completed_at: u64,
}

impl ActivityTimes {
    fn merge(&mut self, other: Self) {
        self.prompted_at = self.prompted_at.max(other.prompted_at);
        self.completed_at = self.completed_at.max(other.completed_at);
    }

    fn complete(self) -> bool {
        self.prompted_at > 0 && self.completed_at > 0
    }
}

#[derive(Clone, Copy, Default)]
struct CachedActivity {
    scanned_len: u64,
    times: ActivityTimes,
}

/// Older live mirrors do not contain the activity fields. Their hook history
/// and assistant transcript still do, so retain a small incremental cache
/// rather than rescanning large JSONL files every two seconds.
struct ActivityHistory {
    transcripts: HashMap<PathBuf, CachedActivity>,
    repositories: HashMap<PathBuf, (String, String)>,
    slack_transcripts: HashMap<PathBuf, u64>,
    slack_directory: SlackDirectory,
}

impl Default for ActivityHistory {
    fn default() -> Self {
        Self {
            transcripts: HashMap::new(),
            repositories: HashMap::new(),
            slack_transcripts: HashMap::new(),
            slack_directory: SlackDirectory::load(),
        }
    }
}

/// Read every live mirror, one snapshot per session.
fn gather(activity_history: &mut ActivityHistory) -> Result<Vec<SessionSnapshot>> {
    let mut snapshots = Vec::new();
    for (path, data) in crate::inspect::discover_instances(&None)? {
        if let Some(snapshot) = snapshot_from_instance(path, data, activity_history) {
            snapshots.push(snapshot);
        }
    }
    Ok(snapshots)
}

fn snapshot_from_instance(
    path: PathBuf,
    data: serde_json::Value,
    activity_history: &mut ActivityHistory,
) -> Option<SessionSnapshot> {
    let last_updated = data
        .get("last_updated")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if now_secs() - last_updated > STALE_SESSION_SECS {
        return None;
    }
    if let Some(transcript_path) = data
        .get("transcript_path")
        .and_then(|value| value.as_str())
        .filter(|path| !path.is_empty())
    {
        activity_history.scan_slack_metadata(Path::new(transcript_path));
    }
    let prs: Vec<SessionPr> = data
        .get("prs")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let git = data.pointer("/widgets/git/data");
    let files = git.and_then(|g| g.get("files")).and_then(|v| v.as_array());
    let activity = activity_times(&data, activity_history);
    let cwd = data.get("cwd").and_then(|v| v.as_str()).unwrap_or_default();
    let (repo_owner, repo_name) = activity_history.repository_identity(&data, cwd);
    let title = data
        .get("terminal_title")
        .and_then(|v| v.as_str())
        .or_else(|| {
            data.get("title_history")
                .and_then(|v| v.as_array())
                .and_then(|titles| titles.last())
                .and_then(|v| v.as_str())
        })
        .unwrap_or_default()
        .to_string();
    let latest_recap = data.pointer("/recap/latest").or_else(|| {
        data.get("recap_history")
            .and_then(|v| v.as_array())
            .and_then(|recaps| recaps.last())
    });
    let platform = mirror_platform(&data);
    let title_set_at = data
        .get("terminal_title_changed_at")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let mut slack_threads: Vec<SlackThread> = data
        .get("slack_threads")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    // The session enriches its PR-attached permalinks with channel and author
    // names it learned live; carry that metadata into the lookup list.
    let pr_slack_threads: Vec<SlackThread> = data
        .get("pr_slack_threads")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    for thread in pr_slack_threads {
        merge_slack_thread(&mut slack_threads, thread);
    }
    for pr in &prs {
        let urls = std::iter::once(pr.slack_origin_url.as_str())
            .chain(pr.slack_comment_urls.iter().map(String::as_str))
            .filter(|url| !url.is_empty());
        for url in urls {
            if slack_threads.iter().any(|thread| thread.url == url) {
                continue;
            }
            if let Some(thread) = crate::slack::extract_threads(url).into_iter().next() {
                slack_threads.push(thread);
            }
        }
    }
    for thread in &mut slack_threads {
        activity_history.slack_directory.enrich_thread(thread);
    }
    let session_id = data
        .get("cloud_session_id")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("session_id").and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_string();
    // Worktree sessions publish a path scope; everything else keys its
    // dispositions by session id.
    let pr_scope = data
        .get("pr_scope")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("session:{session_id}"));
    Some(SessionSnapshot {
        session_id,
        platform,
        cwd: cwd.to_string(),
        pr_scope,
        session_dir: path.parent().map(Path::to_path_buf).unwrap_or_default(),
        dir_name: cwd.rsplit('/').next().unwrap_or_default().to_string(),
        repo_owner,
        repo_name,
        last_updated,
        branch: git
            .and_then(|g| g.get("branch"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        additions: files.map_or(0, |files| {
            files
                .iter()
                .filter_map(|file| file.get("additions").and_then(|value| value.as_i64()))
                .sum()
        }),
        deletions: files.map_or(0, |files| {
            files
                .iter()
                .filter_map(|file| file.get("deletions").and_then(|value| value.as_i64()))
                .sum()
        }),
        title,
        title_set_at,
        slack_threads,
        recap: latest_recap.and_then(recap_brief_from),
        state: mirror_session_state(&data),
        prompted_at: activity.prompted_at,
        completed_at: activity.completed_at,
        prs,
    })
}

/// New mirrors publish the provider directly. Older mirrors can still be
/// identified from their transcript path or their existing Codex title marker.
fn mirror_platform(data: &serde_json::Value) -> PlatformKind {
    data.get("platform")
        .and_then(|value| value.as_str())
        .and_then(PlatformKind::parse)
        .or_else(|| {
            let path = data.get("transcript_path")?.as_str()?;
            path.split(['/', '\\'])
                .find_map(|component| match component {
                    ".codex" => Some(PlatformKind::Codex),
                    ".claude" => Some(PlatformKind::Claude),
                    ".grok" => Some(PlatformKind::Grok),
                    ".opencode" => Some(PlatformKind::Opencode),
                    _ => None,
                })
        })
        .or_else(|| {
            data.get("terminal_title")
                .and_then(|value| value.as_str())
                .and_then(crate::title::marked_title_platform)
        })
        .unwrap_or_default()
}

enum InitialLoadUpdate {
    Snapshot(Box<SessionSnapshot>),
    Complete(Box<ActivityHistory>),
}

fn start_initial_local_load() -> mpsc::Receiver<InitialLoadUpdate> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut history = ActivityHistory::default();
        let Ok(instances) = crate::inspect::discover_instances(&None) else {
            let _ = tx.send(InitialLoadUpdate::Complete(Box::new(history)));
            return;
        };
        for (path, data) in instances {
            if let Some(snapshot) = snapshot_from_instance(path, data, &mut history) {
                if tx
                    .send(InitialLoadUpdate::Snapshot(Box::new(snapshot)))
                    .is_err()
                {
                    return;
                }
            }
        }
        let _ = tx.send(InitialLoadUpdate::Complete(Box::new(history)));
    });
    rx
}

fn parse_session_state(value: &str) -> Option<SessionState> {
    match value.to_ascii_lowercase().as_str() {
        "ready" => Some(SessionState::Ready),
        "thinking" => Some(SessionState::Thinking),
        "permission" => Some(SessionState::Permission),
        "question" => Some(SessionState::Question),
        "complete" => Some(SessionState::Complete),
        "interrupted" => Some(SessionState::Interrupted),
        _ => None,
    }
}

fn mirror_session_state(data: &serde_json::Value) -> SessionState {
    data.pointer("/widgets/stats/data/state")
        .and_then(|value| value.as_str())
        .and_then(parse_session_state)
        .unwrap_or_default()
}

fn activity_times(data: &serde_json::Value, history: &mut ActivityHistory) -> ActivityTimes {
    let mut activity = ActivityTimes {
        prompted_at: mirror_activity_timestamp(data, "prompts_changed_at"),
        completed_at: mirror_activity_timestamp(data, "completions_changed_at"),
    };
    if activity.complete() {
        return activity;
    }

    activity.merge(hook_activity_times(data));
    if activity.complete() {
        return activity;
    }

    let Some(path) = data
        .get("transcript_path")
        .and_then(|value| value.as_str())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
    else {
        return activity;
    };
    history.transcript_activity(&path, activity)
}

fn hook_activity_times(data: &serde_json::Value) -> ActivityTimes {
    let Some(session_id) = data.get("session_id").and_then(|value| value.as_str()) else {
        return ActivityTimes::default();
    };
    let path = format!("/tmp/crabigator-stats-{session_id}.json");
    let Ok(stats) = std::fs::read_to_string(path) else {
        return ActivityTimes::default();
    };
    let Ok(stats) = serde_json::from_str::<serde_json::Value>(&stats) else {
        return ActivityTimes::default();
    };
    hook_activity_from_stats(&stats)
}

fn hook_activity_from_stats(stats: &serde_json::Value) -> ActivityTimes {
    let mut activity = ActivityTimes::default();
    for event in stats
        .get("event_history")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let timestamp = event
            .get("ts")
            .and_then(|value| value.as_f64())
            .map(activity_timestamp_secs)
            .unwrap_or(0);
        match event.get("event").and_then(|value| value.as_str()) {
            Some("UserPromptSubmit") => activity.prompted_at = activity.prompted_at.max(timestamp),
            Some("Stop") => activity.completed_at = activity.completed_at.max(timestamp),
            _ => {}
        }
    }
    activity
}

impl ActivityHistory {
    fn scan_slack_metadata(&mut self, path: &Path) {
        let Ok(file_len) = path.metadata().map(|metadata| metadata.len()) else {
            return;
        };
        let scanned_len = self
            .slack_transcripts
            .get(path)
            .copied()
            .unwrap_or_default();
        if file_len == scanned_len {
            return;
        }
        let offset = if file_len < scanned_len {
            0
        } else {
            scanned_len
        };
        let Ok(mut file) = File::open(path) else {
            return;
        };
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return;
        }
        let mut text = String::new();
        if file.read_to_string(&mut text).is_err() {
            return;
        }
        self.slack_directory.message_metadata(&text);
        self.slack_transcripts.insert(path.to_path_buf(), file_len);
    }

    fn enrich_slack_threads(&mut self, entries: &mut [BoardPr]) {
        for thread in entries
            .iter_mut()
            .flat_map(|entry| entry.slack_threads.iter_mut())
        {
            self.slack_directory.enrich_thread(thread);
        }
    }

    fn transcript_activity(&mut self, path: &Path, seed: ActivityTimes) -> ActivityTimes {
        let Ok(file_len) = path.metadata().map(|metadata| metadata.len()) else {
            return seed;
        };
        let cached = self.transcripts.entry(path.to_path_buf()).or_default();
        cached.times.merge(seed);

        if cached.scanned_len == 0 || file_len < cached.scanned_len {
            cached.times = scan_transcript_backwards(path, cached.times).unwrap_or(cached.times);
            cached.scanned_len = file_len;
        } else if file_len > cached.scanned_len {
            cached.times = scan_transcript_forward(path, cached.scanned_len, cached.times)
                .unwrap_or(cached.times);
            cached.scanned_len = file_len;
        }
        cached.times
    }

    fn repository_identity(&mut self, data: &serde_json::Value, cwd: &str) -> (String, String) {
        let owner = data
            .pointer("/widgets/git/data/repo_owner")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let name = data
            .pointer("/widgets/git/data/repo_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if !name.is_empty() {
            return (owner.to_string(), name.to_string());
        }

        let path = PathBuf::from(cwd);
        if let Some(identity) = self.repositories.get(&path) {
            return identity.clone();
        }
        let remote = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(&path)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| {
                crate::git::parse_remote_identity(String::from_utf8_lossy(&output.stdout).trim())
            });
        let identity = remote.unwrap_or_else(|| {
            let fallback = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            (String::new(), fallback)
        });
        self.repositories.insert(path, identity.clone());
        identity
    }
}

const TRANSCRIPT_SCAN_CHUNK: u64 = 64 * 1024;

fn scan_transcript_backwards(path: &Path, seed: ActivityTimes) -> std::io::Result<ActivityTimes> {
    let mut file = File::open(path)?;
    let mut position = file.metadata()?.len();
    let mut suffix = Vec::new();
    let mut activity = seed;

    while position > 0 && !activity.complete() {
        let chunk_len = position.min(TRANSCRIPT_SCAN_CHUNK);
        position -= chunk_len;
        file.seek(SeekFrom::Start(position))?;
        let mut data = vec![0; chunk_len as usize];
        file.read_exact(&mut data)?;
        data.extend_from_slice(&suffix);

        let complete_from = if position == 0 {
            0
        } else if let Some(newline) = data.iter().position(|byte| *byte == b'\n') {
            suffix = data[..newline].to_vec();
            newline + 1
        } else {
            suffix = data;
            continue;
        };

        for line in data[complete_from..].split(|byte| *byte == b'\n').rev() {
            update_activity_from_jsonl(line, &mut activity);
            if activity.complete() {
                break;
            }
        }
    }
    Ok(activity)
}

fn scan_transcript_forward(
    path: &Path,
    offset: u64,
    seed: ActivityTimes,
) -> std::io::Result<ActivityTimes> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let mut activity = seed;
    for line in data.split(|byte| *byte == b'\n') {
        update_activity_from_jsonl(line, &mut activity);
    }
    Ok(activity)
}

fn update_activity_from_jsonl(line: &[u8], activity: &mut ActivityTimes) {
    let Ok(line) = std::str::from_utf8(line) else {
        return;
    };
    if !line.contains("user_message")
        && !line.contains("agent_message")
        && !line.contains("\"type\":\"user\"")
        && !line.contains("\"type\":\"assistant\"")
    {
        return;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return;
    };
    let timestamp = value
        .get("timestamp")
        .and_then(|value| value.as_str())
        .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
        .and_then(|timestamp| u64::try_from(timestamp.timestamp()).ok())
        .unwrap_or(0);
    if timestamp == 0 {
        return;
    }

    match value.get("type").and_then(|value| value.as_str()) {
        Some("event_msg") => match value
            .pointer("/payload/type")
            .and_then(|value| value.as_str())
        {
            Some("user_message") => activity.prompted_at = activity.prompted_at.max(timestamp),
            Some("agent_message") => activity.completed_at = activity.completed_at.max(timestamp),
            _ => {}
        },
        Some("user")
            if value
                .pointer("/message/content")
                .is_some_and(|content| content.is_string()) =>
        {
            activity.prompted_at = activity.prompted_at.max(timestamp);
        }
        Some("assistant")
            if value
                .pointer("/message/stop_reason")
                .and_then(|value| value.as_str())
                == Some("end_turn") =>
        {
            activity.completed_at = activity.completed_at.max(timestamp);
        }
        _ => {}
    }
}

fn mirror_activity_timestamp(data: &serde_json::Value, field: &str) -> u64 {
    data.pointer(&format!("/widgets/stats/data/{field}"))
        .and_then(|value| value.as_f64())
        .map(activity_timestamp_secs)
        .unwrap_or(0)
}

fn activity_timestamp_secs(timestamp: f64) -> u64 {
    if timestamp.is_finite() && timestamp > 0.0 {
        timestamp as u64
    } else {
        0
    }
}

/// Parse the fields the board needs out of a mirrored TurnRecap value.
fn recap_brief_from(recap: &serde_json::Value) -> Option<RecapBrief> {
    let headline = recap.get("headline")?.as_str()?;
    if headline.is_empty() {
        return None;
    }
    Some(RecapBrief {
        headline: headline.to_string(),
        bullets: recap_string_array(recap, "bullets"),
        next_prompt_notes: recap_string_array(recap, "next_prompt_notes"),
        artifacts: recap_string_array(recap, "artifacts"),
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

fn recap_string_array(recap: &serde_json::Value, field: &str) -> Vec<String> {
    recap
        .get(field)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn merge_pr_slack_threads(
    merged: &mut Vec<SlackThread>,
    pr: &SessionPr,
    session_threads: &[SlackThread],
) {
    let urls = std::iter::once(pr.slack_origin_url.as_str())
        .chain(pr.slack_comment_urls.iter().map(String::as_str))
        .filter(|url| !url.is_empty());
    for url in urls {
        let Some(candidate) = session_threads
            .iter()
            .find(|thread| thread.url == url)
            .cloned()
            .or_else(|| crate::slack::extract_threads(url).into_iter().next())
        else {
            continue;
        };
        merge_slack_thread(merged, candidate);
    }
}

fn merge_slack_thread(merged: &mut Vec<SlackThread>, candidate: SlackThread) {
    if let Some(existing) = merged.iter_mut().find(|thread| thread.url == candidate.url) {
        if crate::slack::has_only_channel_id(existing)
            && !crate::slack::has_only_channel_id(&candidate)
        {
            existing.channel = candidate.channel.clone();
        }
        if existing.author.is_none() {
            existing.author = candidate.author;
        }
        if existing.text.is_none() {
            existing.text = candidate.text;
        }
    } else {
        merged.push(candidate);
    }
}

fn order_pr_slack_threads(entry: &mut BoardPr) {
    let origin = entry.pr.slack_origin_url.as_str();
    entry
        .slack_threads
        .sort_by_key(|thread| (origin.is_empty() || thread.url != origin, thread.posted_at));
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Merge session snapshots into deduped board entries, honoring overrides —
/// scoped ones against the session they cover, group-wide ones everywhere.
/// `linger_days` bounds how long finished primary PRs stay visible (0 = open only).
fn aggregate(
    snapshots: &[SessionSnapshot],
    overrides: &ScopedOverrides,
    linger_days: u64,
) -> Vec<BoardPr> {
    let now = now_secs();
    let mut merged: HashMap<String, BoardPr> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    // PRs a session holds a verified claim on, and same-organization primary
    // PRs that do not match its checkout. Resolve the latter after the merge
    // pass so a verified owner wins; otherwise every primary PR keeps the
    // session title, state, and activity that it shares.
    let mut verified_owner_keys: HashSet<String> = HashSet::new();
    let mut same_org_primary_candidates: Vec<(&SessionSnapshot, String)> = Vec::new();

    for session in snapshots {
        for stored_pr in &session.prs {
            let key = format!(
                "{}/{}#{}",
                stored_pr.owner, stored_pr.repo, stored_pr.number
            );
            let effective_pr = effective_session_pr(
                session,
                stored_pr,
                overrides.for_session(&key, &session.session_id, &session.cwd),
            );
            let pr = &effective_pr;
            let entry = merged.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                // Engagement counters accumulate below, per contributing
                // session — start the merged copy from zero.
                let mut seed = pr.clone();
                seed.mentions = 0;
                seed.user_mentions = 0;
                seed.last_mentioned_at = 0;
                BoardPr {
                    pr: seed,
                    sessions: Vec::new(),
                    slack_threads: Vec::new(),
                    stale: true,
                }
            });

            // Newest GitHub stats win, but classification and engagement belong
            // to the aggregate rather than to any one session's copy.
            let aggregate_primary = entry.pr.primary || pr.primary;
            let aggregate_primary_source = combined_primary_source(&entry.pr, pr);
            // A PR dismissed in one session can still be another session's
            // live work: the merged copy reads dismissed only when every
            // contribution does (group-wide dismissals are applied at the
            // entry level below).
            let aggregate_dismissed = entry.pr.dismissed && pr.dismissed;
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
                if entry.pr.slack_origin_url.is_empty() {
                    entry.pr.slack_origin_url = previous.slack_origin_url;
                }
                for url in previous.slack_comment_urls {
                    if !entry.pr.slack_comment_urls.contains(&url) {
                        entry.pr.slack_comment_urls.push(url);
                    }
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
            merge_pr_slack_threads(&mut entry.slack_threads, pr, &session.slack_threads);

            let represents_session = pr.primary
                && (session_created_pr(session, pr) || pr_attached_to_session(session, pr));
            if represents_session {
                attach_session(entry, session, now);
                verified_owner_keys.insert(key);
            } else if pr.primary
                && !pr.dismissed
                && !session.repo_owner.is_empty()
                && session.repo_owner.eq_ignore_ascii_case(&pr.owner)
            {
                // Same-org only: a session that merely discusses another
                // org's PR must not migrate into that repository's group.
                same_org_primary_candidates.push((session, key));
            }
        }
    }

    // A same-organization primary can live in a sibling repository while one
    // session works a paired change. Carry that session onto every such PR so
    // each block shows the shared title, state, prompt time, and completion
    // time. A verified claim from another session still wins.
    for (session, key) in same_org_primary_candidates {
        if verified_owner_keys.contains(&key) {
            continue;
        }
        if let Some(entry) = merged.get_mut(&key) {
            attach_session(entry, session, now);
        }
    }

    let now_ms = (now * 1000.0) as u64;
    let mut out: Vec<BoardPr> = order
        .into_iter()
        .filter_map(|key| {
            let mut entry = merged.remove(&key)?;
            order_pr_slack_threads(&mut entry);
            match overrides.group(&key) {
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

/// The active view's blocks: PR blocks, plus session blocks in session view.
struct ShapedEntries {
    /// PR view: one block per primary or watched PR. Session view: only the
    /// PRs no session touches (watches), still drawn as PR blocks.
    prs: Vec<BoardPr>,
    /// Session view only: one block per session, PRs beneath it.
    sessions: Vec<SessionEntry>,
}

/// Shape merged entries for the active view. PR view keeps one block per
/// primary PR, its sessions sorted live-first then freshest-first (the
/// sub-row and ←→ order). Session view transposes that: one block per
/// session — so the board lists sessions one-to-one — with every PR the
/// session touches beneath it.
fn entries_for_mode(entries: Vec<BoardPr>, mode: BoardMode) -> ShapedEntries {
    match mode {
        BoardMode::Sessions => session_view_entries(entries),
        BoardMode::Prs => ShapedEntries {
            prs: entries
                .into_iter()
                .filter(|entry| entry.pr.primary || entry.pr.watched)
                .map(|mut entry| {
                    entry.sessions.sort_by_key(|session| {
                        (session.ended, std::cmp::Reverse(session.prompted_at))
                    });
                    entry
                })
                .collect(),
            sessions: Vec::new(),
        },
    }
}

/// Regroup merged PR entries by session: every session becomes one block
/// carrying the PRs it touches, and a session that ended renders stale so a
/// live one stands out. PRs no session touches (watches) stay PR blocks.
fn session_view_entries(entries: Vec<BoardPr>) -> ShapedEntries {
    let mut prs = Vec::new();
    let mut sessions: Vec<SessionEntry> = Vec::new();
    for entry in entries {
        if entry.sessions.is_empty() {
            prs.push(entry);
            continue;
        }
        let BoardPr {
            pr,
            sessions: entry_sessions,
            slack_threads,
            stale,
        } = entry;
        for session in entry_sessions {
            let sub = BoardPr {
                pr: pr.clone(),
                sessions: Vec::new(),
                slack_threads: slack_threads.clone(),
                stale,
            };
            match sessions
                .iter_mut()
                .find(|block| session_key(&block.session) == session_key(&session))
            {
                Some(block) => {
                    block.stale &= stale || session.ended;
                    // Copies of one session are near-identical; keep one whose
                    // local mirror the quick look pane can open.
                    if block.session.session_dir.is_none() && session.session_dir.is_some() {
                        block.session = session;
                    }
                    block.prs.push(sub);
                }
                None => sessions.push(SessionEntry {
                    stale: stale || session.ended,
                    session,
                    prs: vec![sub],
                }),
            }
        }
    }
    for block in &mut sessions {
        sort_entries(&mut block.prs);
    }
    ShapedEntries { prs, sessions }
}

/// The clock a session block sorts and buckets by: the session's newest
/// prompt, falling back to its PRs' own events for sessions that never
/// prompted.
fn session_entry_recency(entry: &SessionEntry) -> u64 {
    if entry.session.prompted_at > 0 {
        return entry.session.prompted_at;
    }
    entry.prs.iter().map(entry_recency_time).max().unwrap_or(0)
}

/// The recency band a session block belongs to on that same clock.
fn session_entry_bucket(entry: &SessionEntry, now: u64) -> RecencyBucket {
    RecencyBucket::from_age(now.saturating_sub(session_entry_recency(entry)))
}

/// Apply current worktree ownership while reading a session snapshot. Older
/// Crabigator processes keep writing their original classification, so the
/// board derives this one fact from branch data that is already in the mirror.
fn effective_session_pr(
    session: &SessionSnapshot,
    stored: &SessionPr,
    disposition: Option<PrDisposition>,
) -> SessionPr {
    let mut pr = stored.clone();
    match disposition {
        Some(PrDisposition::Primary) => {
            pr.primary = true;
            pr.primary_source = "override".to_string();
            pr.dismissed = false;
        }
        Some(PrDisposition::Secondary) => {
            pr.primary = false;
            pr.primary_source = "override".to_string();
            pr.dismissed = false;
        }
        Some(PrDisposition::Dismissed) => {
            pr.primary = false;
            pr.primary_source = "override".to_string();
            pr.dismissed = true;
        }
        None => {
            let explicitly_secondary =
                !pr.primary && matches!(pr.primary_source.as_str(), "session" | "override");
            let attached = pr_attached_to_session(session, &pr);
            if attached
                && !explicitly_secondary
                && !pr.worktree_visit
                && !pr.dismissed
                && pr.state != "CLOSED"
            {
                pr.primary = true;
                pr.primary_source = "auto".to_string();
            }
        }
    }
    pr
}

/// Record a session on a PR row: its Slack threads join the row, and the row
/// stays stale only while every session on it is stale.
fn attach_session(entry: &mut BoardPr, session: &SessionSnapshot, now: f64) {
    for thread in &session.slack_threads {
        merge_slack_thread(&mut entry.slack_threads, thread.clone());
    }
    entry.stale &= (now - session.last_updated).max(0.0) > STALE_SESSION_SECS;
    entry.sessions.push(board_session_ref(session));
}

fn board_session_ref(session: &SessionSnapshot) -> SessionRef {
    SessionRef {
        session_id: session.session_id.clone(),
        platform: session.platform,
        pr_scope: session.pr_scope.clone(),
        dir_name: session.dir_name.clone(),
        session_dir: Some(session.session_dir.clone()),
        title: session.title.clone(),
        title_set_at: session.title_set_at,
        branch: session.branch.clone(),
        recap: session.recap.clone(),
        state: session.state,
        prompted_at: session.prompted_at,
        completed_at: session.completed_at,
        ended: false,
    }
}

fn pr_attached_to_session(session: &SessionSnapshot, pr: &SessionPr) -> bool {
    session_repository_matches_pr(session, pr)
        && attached_to_worktree(pr, &session.branch, &session.dir_name)
}

/// A PR the session opened itself is the session's own work even when it
/// lives in a sibling repository of the same organization — a fix paired with
/// the checkout's own PR. Other organizations still need a matching checkout.
fn session_created_pr(session: &SessionSnapshot, pr: &SessionPr) -> bool {
    pr.created_here
        && !session.repo_owner.is_empty()
        && session.repo_owner.eq_ignore_ascii_case(&pr.owner)
}

fn session_repository_matches_pr(session: &SessionSnapshot, pr: &SessionPr) -> bool {
    repository_matches(&session.repo_owner, &session.repo_name, &pr.owner, &pr.repo)
}

/// Keep one row for every active session no visible PR row represents.
/// Sessions remain distinct even when they share a branch.
fn local_workspaces(snapshots: &[SessionSnapshot], entries: &[BoardPr]) -> Vec<WorkspaceEntry> {
    let now = now_secs();
    let mut workspaces: Vec<WorkspaceEntry> = Vec::new();
    for snapshot in snapshots {
        let age_secs = (now - snapshot.last_updated).max(0.0) as u64;
        if age_secs > STALE_SESSION_SECS as u64 || session_has_matching_pr(snapshot, entries) {
            continue;
        }
        workspaces.push(WorkspaceEntry {
            repo_owner: snapshot.repo_owner.clone(),
            repo_name: snapshot.repo_name.clone(),
            branch: snapshot.branch.clone(),
            session: board_session_ref(snapshot),
            additions: snapshot.additions,
            deletions: snapshot.deletions,
        });
    }
    sort_workspaces(&mut workspaces);
    workspaces
}

fn session_has_matching_pr(snapshot: &SessionSnapshot, entries: &[BoardPr]) -> bool {
    entries_represent_session(
        &snapshot.repo_owner,
        &snapshot.repo_name,
        &snapshot.session_id,
        &snapshot.dir_name,
        entries,
    )
}

fn entries_represent_session(
    repo_owner: &str,
    repo_name: &str,
    session_id: &str,
    dir_name: &str,
    entries: &[BoardPr],
) -> bool {
    // Attachment can be cross-repo (a session working someone else's PR), so
    // an id match anywhere represents the session.
    if !session_id.is_empty() {
        return entries.iter().any(|entry| {
            entry
                .sessions
                .iter()
                .any(|session| session.session_id == session_id)
        });
    }
    // Without an id, a dir-name match only counts inside the session's own
    // repository.
    entries.iter().any(|entry| {
        repository_matches(repo_owner, repo_name, &entry.pr.owner, &entry.pr.repo)
            && entry
                .sessions
                .iter()
                .any(|session| session.session_id.is_empty() && session.dir_name == dir_name)
    })
}

fn repository_matches(owner: &str, repo: &str, other_owner: &str, other_repo: &str) -> bool {
    !repo.is_empty()
        && owner.eq_ignore_ascii_case(other_owner)
        && repo.eq_ignore_ascii_case(other_repo)
}

fn local_board(
    history: &mut ActivityHistory,
    overrides: &ScopedOverrides,
    linger_days: u64,
) -> Result<LocalBoard> {
    let snapshots = gather(history)?;
    let mut entries = aggregate(&snapshots, overrides, linger_days);
    history.enrich_slack_threads(&mut entries);
    let workspaces = local_workspaces(&snapshots, &entries);
    // Live mirror directories by session id (the cloud id once registered),
    // so the all-sessions view can point cloud rows back at local screens.
    let mirrors = snapshots
        .iter()
        .map(|snapshot| (snapshot.session_id.clone(), snapshot.session_dir.clone()))
        .collect();
    Ok((entries, workspaces, mirrors))
}

/// Point cloud-sourced rows at their local mirror when the session is still
/// running here. Cloud records carry no `session_dir`, which would make a
/// live session unselectable in the all-sessions view; sessions with no live
/// mirror are cleared so a dead row can't show a stale screen.
fn attach_live_mirrors(
    entries: &mut [BoardPr],
    workspaces: &mut [WorkspaceEntry],
    mirrors: &HashMap<String, PathBuf>,
) {
    let sessions = entries
        .iter_mut()
        .flat_map(|entry| entry.sessions.iter_mut())
        .chain(workspaces.iter_mut().map(|entry| &mut entry.session));
    for session in sessions {
        session.session_dir = mirrors.get(&session.session_id).cloned();
    }
}

fn sort_workspaces(workspaces: &mut [WorkspaceEntry]) {
    workspaces.sort_by_key(|entry| std::cmp::Reverse(activity_sort_time(entry.sessions())));
}

/// Overlay live mirrors on the durable session list. This keeps `a` mode's
/// historical PRs while still showing active no-PR sessions immediately,
/// including before the Worker has learned the session metadata.
fn merge_live_workspaces(
    workspaces: &mut Vec<WorkspaceEntry>,
    live_workspaces: Vec<WorkspaceEntry>,
    entries: &[BoardPr],
) {
    for live in live_workspaces {
        let live_session = &live.session;
        let existing = workspaces.iter().position(|entry| {
            let session = &entry.session;
            if !live_session.session_id.is_empty() {
                live_session.session_id == session.session_id
            } else {
                live.repo_owner.eq_ignore_ascii_case(&entry.repo_owner)
                    && live.repo_name.eq_ignore_ascii_case(&entry.repo_name)
                    && live.branch == entry.branch
                    && live_session.dir_name == session.dir_name
            }
        });
        if entries_represent_session(
            &live.repo_owner,
            &live.repo_name,
            &live_session.session_id,
            &live_session.dir_name,
            entries,
        ) {
            if let Some(index) = existing {
                workspaces.remove(index);
            }
            continue;
        }
        if let Some(index) = existing {
            workspaces[index] = live;
        } else {
            workspaces.push(live);
        }
    }
    sort_workspaces(workspaces);
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
    // it visible as "fetching" while enrichment retries — and a watched PR is
    // wanted by definition.
    let wanted = pr.primary || pr.watched;
    if pr.refreshed_at == 0 {
        return wanted;
    }
    if pr.state == "OPEN" {
        return true;
    }
    // Finished secondaries disappear immediately. Finished primaries and
    // watches retain the adjustable grace window; the foreign-author gate
    // never applies to a watch, which is explicit interest.
    if !wanted || (!pr.watched && foreign_without_explicit_interest(pr)) {
        return false;
    }
    let latest = pr.closed_at.max(pr.last_mentioned_at).max(pr.updated_at);
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
        attention_rank(&a.pr)
            .cmp(&attention_rank(&b.pr))
            .then(b.pr.primary.cmp(&a.pr.primary))
            .then(b.pr.last_mentioned_at.cmp(&a.pr.last_mentioned_at))
    });
}

/// Map the cloud board (durable D1 records, ended sessions included) into
/// the same shape the live aggregation produces. Overrides and the linger
/// window are already applied server-side.
fn cloud_entries_to_board(cloud: crate::cloud::CloudBoard) -> (Vec<BoardPr>, Vec<WorkspaceEntry>) {
    let mut represented = HashSet::new();
    let mut out: Vec<BoardPr> = Vec::new();
    for entry in cloud.prs {
        let pr = entry.pr;
        let mut slack_threads = Vec::new();
        merge_pr_slack_threads(&mut slack_threads, &pr, &[]);
        let sessions: Vec<SessionRef> = entry
            .sessions
            .into_iter()
            .map(|s| {
                represented.insert(s.session_id.clone());
                let active = s.active;
                let state = parse_session_state(&s.state).unwrap_or(if active {
                    SessionState::Ready
                } else {
                    SessionState::Complete
                });
                let recap = s.recap.and_then(|r| {
                    (!r.headline.is_empty()).then_some(RecapBrief {
                        headline: r.headline,
                        bullets: r.bullets,
                        next_prompt_notes: r.next_prompt_notes,
                        artifacts: r.artifacts,
                        generated_at: r.generated_at,
                        line_delta: crate::recap::TurnLineDelta {
                            additions: r.additions,
                            deletions: r.deletions,
                        },
                    })
                });
                SessionRef {
                    pr_scope: if s.pr_scope.is_empty() {
                        format!("session:{}", s.session_id)
                    } else {
                        s.pr_scope
                    },
                    session_id: s.session_id,
                    platform: cloud_session_platform(s.platform, &s.title),
                    dir_name: s.dir_name,
                    session_dir: None,
                    title_set_at: s.title_set_at,
                    title: s.title,
                    branch: s.branch,
                    recap,
                    state,
                    prompted_at: activity_timestamp_secs(s.prompts_changed_at),
                    completed_at: activity_timestamp_secs(s.completions_changed_at),
                    ended: !active,
                }
            })
            .collect();
        let stale = sessions.iter().all(|session| session.ended);
        out.push(BoardPr {
            pr,
            sessions,
            slack_threads,
            stale,
        });
    }
    sort_entries(&mut out);

    let mut workspaces: Vec<WorkspaceEntry> = Vec::new();
    for session in cloud.sessions {
        if !session.active || represented.contains(&session.session_id) {
            continue;
        }
        let repo_name = if session.repo_name.is_empty() {
            session.dir_name.clone()
        } else {
            session.repo_name.clone()
        };
        let recap = session.recap.and_then(|recap| {
            (!recap.headline.is_empty()).then_some(RecapBrief {
                headline: recap.headline,
                bullets: recap.bullets,
                next_prompt_notes: recap.next_prompt_notes,
                artifacts: recap.artifacts,
                generated_at: recap.generated_at,
                line_delta: crate::recap::TurnLineDelta {
                    additions: recap.additions,
                    deletions: recap.deletions,
                },
            })
        });
        let session_ref = SessionRef {
            state: parse_session_state(&session.state).unwrap_or(SessionState::Ready),
            pr_scope: if session.pr_scope.is_empty() {
                format!("session:{}", session.session_id)
            } else {
                session.pr_scope.clone()
            },
            session_id: session.session_id,
            platform: cloud_session_platform(session.platform, &session.title),
            dir_name: session.dir_name,
            session_dir: None,
            title_set_at: session.title_set_at,
            title: session.title,
            branch: session.branch.clone(),
            recap,
            prompted_at: activity_timestamp_secs(session.prompts_changed_at),
            completed_at: activity_timestamp_secs(session.completions_changed_at),
            ended: !session.active,
        };
        workspaces.push(WorkspaceEntry {
            repo_owner: session.repo_owner,
            repo_name,
            branch: session.branch,
            session: session_ref,
            additions: session.additions,
            deletions: session.deletions,
        });
    }
    sort_workspaces(&mut workspaces);
    (out, workspaces)
}

fn cloud_session_platform(platform: Option<PlatformKind>, title: &str) -> PlatformKind {
    platform
        .or_else(|| crate::title::marked_title_platform(title))
        .unwrap_or_default()
}

/// Sort rank for the GitHub state that needs the most attention.
fn attention_rank(pr: &SessionPr) -> u8 {
    if pr.state.is_empty() {
        2
    } else if pr.state == "MERGED" {
        6
    } else if pr.state != "OPEN" {
        7
    } else if pr.mergeable == "CONFLICTING" || pr.checks_failed > 0 {
        0
    } else if pr.review_decision == "CHANGES_REQUESTED" {
        1
    } else if pr.is_draft {
        2
    } else if pr.checks_pending > 0 {
        3
    } else if pr.review_decision != "APPROVED" {
        4
    } else {
        5
    }
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

struct ActivityCell {
    styled: String,
    visible: usize,
}

/// The newest prompt and completion among every session attached to a PR.
/// Thinking highlights the prompt; every settled state highlights completion.
fn activity_cell(
    sessions: &[SessionRef],
    now_ms: u64,
    throbber_frame: usize,
    cooldowns: &Cooldowns,
) -> ActivityCell {
    let now = now_ms / 1000;
    let badge_tint = cooldowns.hottest_tint(
        sessions
            .iter()
            .map(|session| session_cooldown_key(&session.session_id)),
        now_ms,
    );
    let prompted_at = sessions
        .iter()
        .map(|session| session.prompted_at)
        .max()
        .unwrap_or(0);
    let completed_at = sessions
        .iter()
        .map(|session| session.completed_at)
        .max()
        .unwrap_or(0);
    let activity_state = activity_state(sessions);
    let prompt_is_active = activity_state == Some(SessionState::Thinking);
    let prompt = activity_part(
        PROMPT_ICON,
        prompted_at,
        now,
        if prompt_is_active {
            ACTIVITY_BRIGHT_TEAL
        } else {
            ACTIVITY_DARK_TEAL
        },
    );
    let completion = activity_part(
        COMPLETION_ICON,
        completed_at,
        now,
        if prompt_is_active {
            ACTIVITY_DARK_TEAL
        } else {
            ACTIVITY_BRIGHT_TEAL
        },
    );
    let state = activity_state
        .map(|state| crate::ui::session_state_badge(state, throbber_frame, badge_tint))
        .unwrap_or_else(|| " ".repeat(crate::ui::STATE_BADGE_WIDTH));
    ActivityCell {
        styled: format!("{}  {}  {}", state, prompt.styled, completion.styled),
        visible: crate::ui::STATE_BADGE_WIDTH + 2 + prompt.visible + 2 + completion.visible,
    }
}

/// Pick the state that needs the user's attention when a PR represents more
/// than one session. Most PR rows have one session; this keeps aggregate rows
/// useful when they do not.
fn activity_state(sessions: &[SessionRef]) -> Option<SessionState> {
    sessions
        .iter()
        .map(|session| session.state)
        .max_by_key(|state| match state {
            SessionState::Permission => 5,
            SessionState::Question => 4,
            SessionState::Thinking => 3,
            SessionState::Interrupted => 2,
            SessionState::Ready => 1,
            SessionState::Complete => 0,
        })
}

/// Cooldown key for a session's state badge.
fn session_cooldown_key(session_id: &str) -> String {
    format!("session:{session_id}")
}

/// Record what every badge and status cell shows now, so cells that changed
/// since the last frame start their cooldown.
fn observe_cooldowns(
    cooldowns: &mut Cooldowns,
    entries: &[BoardPr],
    workspaces: &[WorkspaceEntry],
    now_ms: u64,
) {
    let sessions = entries
        .iter()
        .flat_map(|entry| entry.sessions.iter())
        .chain(workspaces.iter().map(|entry| &entry.session));
    for session in sessions {
        cooldowns.observe_session_state(
            session_cooldown_key(&session.session_id),
            session.state,
            now_ms,
        );
    }
    for entry in entries {
        cooldowns.observe_pr(&entry.pr, now_ms);
    }
}

fn board_has_thinking(entries: &[BoardPr], workspaces: &[WorkspaceEntry]) -> bool {
    entries
        .iter()
        .flat_map(|entry| entry.sessions.iter())
        .chain(workspaces.iter().map(|entry| &entry.session))
        .any(|session| session.state == SessionState::Thinking)
}

/// Sort each row by its newest prompt. Completions never move a row: the
/// board orders by when the user last spoke to a session, not by when the
/// assistant finished.
fn activity_sort_time(sessions: &[SessionRef]) -> u64 {
    sessions
        .iter()
        .map(|session| session.prompted_at)
        .max()
        .unwrap_or(0)
}

fn activity_bucket(sessions: &[SessionRef], now: u64) -> RecencyBucket {
    RecencyBucket::from_age(now.saturating_sub(activity_sort_time(sessions)))
}

/// The clock a PR row sorts and buckets by, in unix seconds: the newest
/// prompt among its sessions. A row with no prompt time — a watch, or
/// sessions that never prompted — falls back to the PR's own events: GitHub's
/// updatedAt (which moves on any push, comment, review, or merge), the close,
/// or its last mention here.
fn entry_recency_time(entry: &BoardPr) -> u64 {
    let prompted = activity_sort_time(&entry.sessions);
    if prompted > 0 {
        return prompted;
    }
    (entry.pr.updated_at / 1000)
        .max(entry.pr.closed_at / 1000)
        .max(entry.pr.last_mentioned_at / 1000)
}

/// The recency band a PR row belongs to on that same clock.
fn entry_bucket(entry: &BoardPr, now: u64) -> RecencyBucket {
    RecencyBucket::from_age(now.saturating_sub(entry_recency_time(entry)))
}

fn activity_part(label: &str, timestamp: u64, now: u64, event_color: u8) -> ActivityCell {
    let (plain, part_color) = if timestamp == 0 {
        (format!("{label} —"), color::DARK_GRAY)
    } else {
        let age = now.saturating_sub(timestamp);
        (format!("{label} {}", format_age(age)), event_color)
    };
    ActivityCell {
        visible: plain.width(),
        styled: format!("{}{}{}", fg(part_color), plain, RESET_FG),
    }
}

fn recency_color(age_secs: u64) -> u8 {
    RecencyBucket::from_age(age_secs).color()
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

/// A session block matches on its own title or on any of its PRs.
fn session_entry_matches_search(entry: &SessionEntry, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let title = entry.session.title.to_lowercase();
    title.contains(&query.to_lowercase())
        || entry.prs.iter().any(|sub| matches_search(&sub.pr, query))
}

fn workspace_matches_search(entry: &WorkspaceEntry, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    entry.repo_owner.to_lowercase().contains(&query)
        || entry.repo_name.to_lowercase().contains(&query)
        || entry.branch.to_lowercase().contains(&query)
        || entry.session.title.to_lowercase().contains(&query)
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
    build_session_previews(&entry.sessions, cache, query, width, expanded)
}

fn build_session_previews(
    sessions: &[SessionRef],
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
    for session in sessions {
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

fn provider_markers(sessions: &[SessionRef]) -> String {
    crate::title::combined_provider_title_marker(sessions.iter().map(|session| session.platform))
}

fn latest_session_title(sessions: &[SessionRef]) -> Option<String> {
    sessions
        .iter()
        .filter_map(|session| {
            let plain = crate::title::strip_provider_title_marker(&session.title);
            (!plain.is_empty() && plain != session.dir_name).then(|| {
                (
                    crate::title::mark_provider_title(session.platform, plain),
                    session.title_set_at,
                )
            })
        })
        .max_by_key(|(_, set_at)| *set_at)
        .map(|(title, _)| title)
}

fn timestamp_cell(timestamp_ms: u64, now_ms: u64, width: usize) -> ActivityCell {
    if width == 0 {
        return ActivityCell {
            styled: String::new(),
            visible: 0,
        };
    }
    let (plain, age_color) = if timestamp_ms == 0 {
        ("—".to_string(), color::DARK_GRAY)
    } else {
        let age_secs = now_ms.saturating_sub(timestamp_ms) / 1000;
        (format_age(age_secs), recency_color(age_secs))
    };
    let plain = crate::ui::pr_cells::truncate_to_width(&plain, width);
    let visible = plain.width();
    ActivityCell {
        styled: format!(
            "{}{}{}{}",
            " ".repeat(width.saturating_sub(visible)),
            fg(age_color),
            plain,
            RESET_FG
        ),
        visible: width,
    }
}

fn empty_cell(width: usize) -> ActivityCell {
    ActivityCell {
        styled: " ".repeat(width),
        visible: width,
    }
}

/// A Slack link right-aligned in `width` cells, so links end at the right
/// edge like the GitHub status above them.
fn slack_link_cell(thread: &SlackThread, width: usize) -> ActivityCell {
    let full_label = crate::slack::thread_identity_label(thread);
    let label = crate::ui::pr_cells::truncate_to_width(&full_label, width);
    if label.is_empty() {
        return empty_cell(0);
    }
    ActivityCell {
        styled: format!(
            "{}{}{}{}",
            " ".repeat(width.saturating_sub(label.width())),
            fg(color::CYAN),
            escape::hyperlink(&thread.url, &label),
            RESET_FG
        ),
        visible: width,
    }
}

fn slack_links_width(threads: &[SlackThread]) -> usize {
    unique_channel_threads(threads)
        .into_iter()
        .map(crate::slack::thread_identity_label)
        .map(|label| label.width())
        .max()
        .unwrap_or(0)
}

/// One row per Slack channel. A PR's comments often link the same channel
/// many times; readers only need each conversation once, labeled as fully as
/// any of its permalinks allows. Within a channel the best-known thread wins
/// (author known, then readable channel name, then newest), while the list
/// keeps its origin-first channel order.
fn unique_channel_threads(threads: &[SlackThread]) -> Vec<&SlackThread> {
    let mut channels: Vec<(String, &SlackThread)> = Vec::new();
    for thread in threads {
        let key = crate::slack::channel_key(thread);
        let rank = |thread: &SlackThread| {
            (
                thread.author.is_some(),
                !crate::slack::has_only_channel_id(thread),
                thread.posted_at,
            )
        };
        match channels.iter_mut().find(|(existing, _)| *existing == key) {
            Some((_, existing)) => {
                if rank(thread) > rank(existing) {
                    *existing = thread;
                }
            }
            None => channels.push((key, thread)),
        }
    }
    channels.into_iter().map(|(_, thread)| thread).collect()
}

/// The title a PR row leads with: GitHub's own title, falling back to the
/// newest session title and then the repository name when GitHub has none.
fn pr_row_title(entry: &BoardPr) -> String {
    let pr_title = entry.pr.title.trim();
    if !pr_title.is_empty() {
        return pr_title.to_string();
    }
    latest_session_title(&entry.sessions).unwrap_or_else(|| {
        let fallback = if entry.pr.repo.is_empty() {
            "PR"
        } else {
            entry.pr.repo.as_str()
        };
        format!("{}{fallback}", provider_markers(&entry.sessions))
    })
}

fn slack_detail_cells(entry: &BoardPr, span: usize) -> Vec<ActivityCell> {
    unique_channel_threads(&entry.slack_threads)
        .into_iter()
        .map(|thread| slack_link_cell(thread, span))
        .collect()
}

#[derive(Clone, Copy)]
enum RecapLineKind {
    Judgment,
    Recap,
    Bullet,
    Detail,
}

struct RecapDetailRow<'a> {
    kind: RecapLineKind,
    text: Cow<'a, str>,
    recap: Option<&'a RecapBrief>,
}

struct DetailLayout<'a> {
    width: u16,
    now_ms: u64,
    activity_width: usize,
    widths: &'a PrColumnWidths,
}

impl DetailLayout<'_> {
    /// The cells a Slack link may fill: the activity column, its gap, and the
    /// right cluster. Links only share a row with the judgment or stand
    /// alone, so the activity column beneath them is always empty.
    fn link_span(&self) -> usize {
        activity_span(self.activity_width) + self.widths.right_width()
    }

    /// A Slack link's row cells: the activity cell collapses so the link
    /// spans it along with the right cluster.
    fn link_cells(&self, link: &ActivityCell) -> (PrCell, PrCell) {
        (
            (String::new(), 0, 0),
            (link.styled.clone(), link.visible, self.link_span()),
        )
    }
}

/// The activity column plus the gap that separates it from the right cluster.
fn activity_span(activity_width: usize) -> usize {
    activity_width + usize::from(activity_width > 0) * PR_RIGHT_COLUMN_GAP
}

impl RecapLineKind {
    fn style(self) -> (&'static str, u8) {
        match self {
            Self::Judgment => ("✦", color::YELLOW),
            Self::Recap => ("↪", color::DARK_GRAY),
            Self::Bullet => ("•", color::DARK_GRAY),
            Self::Detail => (" ", color::DARK_GRAY),
        }
    }
}

fn recap_line(
    kind: RecapLineKind,
    headline: &str,
    recap: Option<&RecapBrief>,
    layout: &DetailLayout<'_>,
    link: Option<&ActivityCell>,
) -> String {
    let (icon, icon_color) = kind.style();
    let mut delta = recap.map_or_else(String::new, |recap| {
        if recap.line_delta.additions == 0 && recap.line_delta.deletions == 0 {
            String::new()
        } else {
            format!(
                " · {}{}",
                crate::ui::handoff::format_line_delta(recap.line_delta),
                fg(color::DARK_GRAY)
            )
        }
    });
    let mut delta_width = crate::parsers::strip_ansi_for_debug(&delta).width();
    let prefix_width = 2 + icon.width() + 1;
    if prefix_width + delta_width >= layout.widths.board_left_width() {
        delta.clear();
        delta_width = 0;
    }
    let headline = crate::ui::pr_cells::truncate_to_width(
        headline,
        layout
            .widths
            .board_left_width()
            .saturating_sub(prefix_width + delta_width),
    );
    let left_visible = prefix_width + headline.width() + delta_width;
    let left_styled = format!(
        "  {}{} {}{}{}{}",
        fg(icon_color),
        icon,
        fg(color::GRAY),
        headline,
        fg(color::DARK_GRAY),
        delta,
    );
    // A Slack link takes the activity column with it; without one the column
    // carries the recap's age.
    let (activity, right) = match link {
        Some(link) => layout.link_cells(link),
        None => {
            let age = recap.map_or_else(
                || empty_cell(layout.activity_width),
                |recap| timestamp_cell(recap.generated_at, layout.now_ms, layout.activity_width),
            );
            (
                (age.styled, age.visible, layout.activity_width),
                (String::new(), 0, layout.widths.right_width()),
            )
        }
    };
    format!(
        "{}{}",
        board_detail_row_text(
            layout.width,
            layout.widths,
            left_styled,
            left_visible,
            activity,
            right,
        ),
        RESET
    )
}

fn detail_lines(
    recap_rows: &[RecapDetailRow<'_>],
    slack_cells: &[ActivityCell],
    layout: &DetailLayout<'_>,
) -> Vec<String> {
    let height = recap_rows.len().max(slack_cells.len());
    (0..height)
        .map(|index| match recap_rows.get(index) {
            Some(row) => recap_line(
                row.kind,
                &row.text,
                row.recap,
                layout,
                slack_cells.get(index),
            ),
            None => {
                let (activity, right) = layout.link_cells(&slack_cells[index]);
                format!(
                    "{}{}",
                    board_detail_row_text(
                        layout.width,
                        layout.widths,
                        String::new(),
                        0,
                        activity,
                        right,
                    ),
                    RESET
                )
            }
        })
        .collect()
}

/// The ✦ judgment row, which only open PRs with a note carry.
fn judgment_row(pr: &SessionPr) -> Option<RecapDetailRow<'_>> {
    (!pr.ai_note.is_empty() && pr.state == "OPEN").then(|| RecapDetailRow {
        kind: RecapLineKind::Judgment,
        text: Cow::Borrowed(&pr.ai_note),
        recap: None,
    })
}

fn recap_detail_rows(recap: &RecapBrief) -> Vec<RecapDetailRow<'_>> {
    let mut rows = vec![RecapDetailRow {
        kind: RecapLineKind::Recap,
        text: Cow::Borrowed(&recap.headline),
        recap: Some(recap),
    }];
    rows.extend(recap_extra_rows(recap));
    rows
}

/// The recap's detail rows below the headline.
fn recap_extra_rows(recap: &RecapBrief) -> Vec<RecapDetailRow<'_>> {
    let mut rows = Vec::new();
    rows.extend(recap.bullets.iter().map(|bullet| RecapDetailRow {
        kind: RecapLineKind::Bullet,
        text: Cow::Borrowed(bullet),
        recap: None,
    }));
    rows.extend(recap.next_prompt_notes.iter().map(|note| RecapDetailRow {
        kind: RecapLineKind::Detail,
        text: labeled_recap_detail("Next:", note),
        recap: None,
    }));
    rows.extend(recap.artifacts.iter().map(|artifact| RecapDetailRow {
        kind: RecapLineKind::Detail,
        text: labeled_recap_detail("Artifact:", artifact),
        recap: None,
    }));
    rows
}

fn recap_detail_lines(
    recap: &RecapBrief,
    width: u16,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
) -> Vec<String> {
    let layout = DetailLayout {
        width,
        now_ms,
        activity_width,
        widths,
    };
    detail_lines(&recap_detail_rows(recap), &[], &layout)
}

fn labeled_recap_detail<'a>(label: &str, value: &'a str) -> Cow<'a, str> {
    let value = value.trim();
    if value
        .get(..label.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(label))
    {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(format!("{label} {value}"))
    }
}

/// One board entry ready to draw: the PR plus any transcript preview lines
/// the active search produced for it.
struct BoardRow<'a> {
    entry: &'a BoardPr,
    preview_lines: Vec<String>,
}

struct WorkspaceRow<'a> {
    entry: &'a WorkspaceEntry,
    preview_lines: Vec<String>,
}

/// One session-view block ready to draw: the session plus any transcript
/// preview lines the active search produced for it.
struct SessionBlockRow<'a> {
    entry: &'a SessionEntry,
    preview_lines: Vec<String>,
}

/// One board row's place in the rendered frame, for selection and quick look.
struct RowSpan {
    /// Stable identity across refreshes: the PR alone on a PR block, the
    /// session on a session block or a workspace row.
    key: String,
    /// First line of the row block, as an index into the rendered frame.
    start: usize,
    /// One past the row block's last line.
    end: usize,
    /// The live session mirrors this row can preview: one per live session on
    /// a PR block (freshest first), the block's own session otherwise.
    peeks: Vec<PeekTarget>,
}

/// A session whose live screen the quick look pane can show.
struct PeekTarget {
    title: String,
    dir_name: String,
    /// The session's /tmp mirror, holding screen.txt and scrollback.log.
    session_dir: PathBuf,
}

/// The identity a session contributes to a row key: the session id, or the
/// directory name for mirrors that predate ids.
fn session_key(session: &SessionRef) -> &str {
    if session.session_id.is_empty() {
        &session.dir_name
    } else {
        &session.session_id
    }
}

/// A row is previewable when its session has a local mirror with a live
/// screen snapshot; cloud records and `--no-capture` sessions are not.
fn peek_target(session: &SessionRef) -> Option<PeekTarget> {
    let dir = session.session_dir.as_ref()?;
    dir.join("screen.txt").exists().then(|| {
        let plain = crate::title::strip_provider_title_marker(&session.title);
        let title = if plain.is_empty() {
            session.dir_name.clone()
        } else {
            plain.to_string()
        };
        PeekTarget {
            title,
            dir_name: session.dir_name.clone(),
            session_dir: dir.clone(),
        }
    })
}

/// What every board row needs beyond its own entry: the frame's terminal
/// width and detail level, its clock, the shared column widths, the throbber
/// phase, and the cooldown tints.
#[derive(Clone, Copy)]
struct RowContext<'a> {
    width: u16,
    detail: u8,
    now_ms: u64,
    activity_width: usize,
    widths: &'a PrColumnWidths,
    throbber_frame: usize,
    cooldowns: &'a Cooldowns,
}

/// One PR-view block: the PR header row with its sessions' activity, diff,
/// file count, and GitHub status inline, the ✦ judgment and Slack links when
/// detail is on, then one sub-row per touching session with its recap
/// headline (and full recap at detail).
fn render_pr_view_block(board_row: &BoardRow<'_>, context: RowContext<'_>) -> Vec<String> {
    let RowContext {
        width,
        detail,
        now_ms,
        activity_width,
        widths,
        throbber_frame,
        cooldowns,
    } = context;
    let entry = board_row.entry;
    let title = pr_row_title(entry);
    let activity = pr_view_activity_cell(entry, now_ms, activity_width, throbber_frame, cooldowns);
    let mut row = crate::ui::pr_cells::pr_view_row_text(
        width,
        &entry.pr,
        widths,
        &title,
        (activity.styled, activity.visible, activity_width),
        cooldowns.pr_tints(&entry.pr, now_ms),
    );
    if entry.stale {
        row = format!("{}{row}", fg(color::DARK_GRAY));
    }
    let mut lines = vec![format!("{row}{RESET}")];

    let layout = DetailLayout {
        width,
        now_ms,
        activity_width,
        widths,
    };
    if detail > DEFAULT_DETAIL {
        let judgment: Vec<RecapDetailRow<'_>> = judgment_row(&entry.pr).into_iter().collect();
        let slack = slack_detail_cells(entry, layout.link_span());
        lines.extend(detail_lines(&judgment, &slack, &layout));
    }

    for session in &entry.sessions {
        lines.push(pr_view_session_row(session, width, activity_width, widths));
        if detail > DEFAULT_DETAIL {
            if let Some(recap) = session.recap.as_ref() {
                lines.extend(detail_lines(&recap_detail_rows(recap), &[], &layout));
            }
        }
    }
    lines.extend(board_row.preview_lines.iter().cloned());
    lines
}

/// The PR-view header's activity cell: the state and prompt/completion ages
/// aggregated across the PR's sessions. A PR no session touches (a watch)
/// shows the age of its latest GitHub event instead.
fn pr_view_activity_cell(
    entry: &BoardPr,
    now_ms: u64,
    activity_width: usize,
    throbber_frame: usize,
    cooldowns: &Cooldowns,
) -> ActivityCell {
    if entry.sessions.is_empty() {
        return timestamp_cell(entry_recency_time(entry) * 1000, now_ms, activity_width);
    }
    activity_cell(&entry.sessions, now_ms, throbber_frame, cooldowns)
}

/// One session beneath a PR-view header: `◆ title — recap headline`. The
/// state and activity ages live on the header row, so the sub-row leaves
/// that column empty. Ended sessions render entirely dim so the live ones
/// stand out.
fn pr_view_session_row(
    session: &SessionRef,
    width: u16,
    activity_width: usize,
    widths: &PrColumnWidths,
) -> String {
    let (left_styled, left_visible) = session_title_cell(session, "  ◆ ", session.ended, widths);
    format!(
        "{}{RESET}",
        board_detail_row_text(
            width,
            widths,
            left_styled,
            left_visible,
            (String::new(), 0, activity_width),
            (String::new(), 0, widths.right_width()),
        )
    )
}

/// The narrowest a session title may get before its row's branch is dropped
/// to make room.
const SESSION_TITLE_MIN: usize = 16;

/// The `◆ title ⎇  branch` left cell shared by the PR view's session
/// sub-rows and the session view's block headers, truncated to the left
/// columns. The branch renders whole or not at all — a clipped branch reads
/// as noise — and the title gives way down to [`SESSION_TITLE_MIN`] before
/// the branch is dropped. A dimmed cell renders entirely gray so live rows
/// stand out.
fn session_title_cell(
    session: &SessionRef,
    prefix: &str,
    dimmed: bool,
    widths: &PrColumnWidths,
) -> (String, usize) {
    let title = marked_session_title(session);
    let mut branch_text = if session.branch.is_empty() {
        String::new()
    } else {
        format!("{BRANCH_PREFIX}{}", session.branch)
    };
    let (title_color, text_color) = if dimmed {
        (color::DARK_GRAY, color::DARK_GRAY)
    } else {
        (color::LIGHT_BLUE, color::GRAY)
    };

    let budget = widths.board_left_width().saturating_sub(prefix.width());
    let mut separator = if branch_text.is_empty() { "" } else { " " };
    let title_reserved = title.width().min(SESSION_TITLE_MIN);
    if !branch_text.is_empty() && title_reserved + separator.width() + branch_text.width() > budget
    {
        branch_text.clear();
        separator = "";
    }
    let title_text = crate::ui::pr_cells::truncate_to_width(
        &title,
        budget.saturating_sub(branch_text.width() + separator.width()),
    );
    let visible = prefix.width() + title_text.width() + separator.width() + branch_text.width();
    let styled = format!(
        "{gray}{prefix}{}{title_text}{gray}{separator}{}{branch_text}{RESET_FG}",
        fg(title_color),
        fg(text_color),
        gray = fg(color::DARK_GRAY),
    );
    (styled, visible)
}

/// One session-view block: the session's title and recap headline with its
/// state and activity ages, then one sub-row per PR it touches — the PR-view
/// row anatomy with the activity column left empty, since the header carries
/// it. Detail adds the recap under the header and each PR's ✦ judgment and
/// Slack links under its row.
fn render_session_view_block(
    session_row: &SessionBlockRow<'_>,
    context: RowContext<'_>,
) -> Vec<String> {
    let RowContext {
        width,
        detail,
        now_ms,
        activity_width,
        widths,
        throbber_frame,
        cooldowns,
    } = context;
    let entry = session_row.entry;
    let session = &entry.session;
    let activity = activity_cell(
        std::slice::from_ref(session),
        now_ms,
        throbber_frame,
        cooldowns,
    );

    let (left_styled, left_visible) = session_title_cell(session, "◆ ", entry.stale, widths);
    let mut row = board_detail_row_text(
        width,
        widths,
        left_styled,
        left_visible,
        (activity.styled, activity.visible, activity_width),
        (String::new(), 0, widths.right_width()),
    );
    if entry.stale {
        row = format!("{}{row}", fg(color::DARK_GRAY));
    }
    let mut lines = vec![format!("{row}{RESET}")];

    let layout = DetailLayout {
        width,
        now_ms,
        activity_width,
        widths,
    };
    if detail > DEFAULT_DETAIL {
        if let Some(recap) = session.recap.as_ref() {
            lines.extend(detail_lines(&recap_detail_rows(recap), &[], &layout));
        }
    }

    for sub in &entry.prs {
        let title = pr_row_title(sub);
        // Actions on a session's sub-rows apply in that session's scope, so a
        // dismissal here leaves the group's other sessions alone.
        let mut row = crate::ui::pr_cells::session_view_pr_row_text(
            width,
            &sub.pr,
            widths,
            &title,
            activity_width,
            cooldowns.pr_tints(&sub.pr, now_ms),
            &session.pr_scope,
        );
        if entry.stale {
            row = format!("{}{row}", fg(color::DARK_GRAY));
        }
        lines.push(format!("{row}{RESET}"));
        if detail > DEFAULT_DETAIL {
            let judgment: Vec<RecapDetailRow<'_>> = judgment_row(&sub.pr).into_iter().collect();
            let slack = slack_detail_cells(sub, layout.link_span());
            lines.extend(detail_lines(&judgment, &slack, &layout));
        }
    }
    lines.extend(session_row.preview_lines.iter().cloned());
    lines
}

/// A session's own title carrying its provider marker, falling back to the
/// working directory when the assistant has not titled the session yet.
fn marked_session_title(session: &SessionRef) -> String {
    let plain = crate::title::strip_provider_title_marker(&session.title);
    let name = if plain.is_empty() {
        session.dir_name.as_str()
    } else {
        plain
    };
    crate::title::mark_provider_title(session.platform, name)
}

fn workspace_title(entry: &WorkspaceEntry) -> (String, u8) {
    (marked_session_title(&entry.session), color::PURPLE)
}

fn workspace_diff_text(entry: &WorkspaceEntry) -> String {
    if entry.additions == 0 && entry.deletions == 0 {
        String::new()
    } else {
        format!("+{} -{}", entry.additions, entry.deletions)
    }
}

fn workspace_branch_text(entry: &WorkspaceEntry) -> String {
    let branch = if entry.branch.is_empty() {
        "(no branch)"
    } else {
        entry.branch.as_str()
    };
    format!("{BRANCH_PREFIX}{branch}")
}

fn render_workspace_board_row(
    workspace_row: &WorkspaceRow<'_>,
    context: RowContext<'_>,
) -> Vec<String> {
    let RowContext {
        width,
        detail,
        now_ms,
        activity_width,
        widths,
        throbber_frame,
        cooldowns,
    } = context;
    let entry = workspace_row.entry;
    let (title, title_color) = workspace_title(entry);
    let branch = workspace_branch_text(entry);
    let activity = activity_cell(entry.sessions(), now_ms, throbber_frame, cooldowns);
    let row = session_row_text_with_activity(
        width,
        &title,
        title_color,
        entry.additions,
        entry.deletions,
        &branch,
        widths,
        activity.styled,
        activity.visible,
        activity_width,
    );
    let mut lines = vec![format!("{row}{RESET}")];
    if detail > DEFAULT_DETAIL {
        if let Some(recap) = entry.session.recap.as_ref() {
            lines.extend(recap_detail_lines(
                recap,
                width,
                now_ms,
                activity_width,
                widths,
            ));
        }
    }
    lines.extend(workspace_row.preview_lines.iter().cloned());
    lines
}

/// Build one full frame as displayable lines (no trailing newline handling).
fn render(
    rows: &[BoardRow],
    session_rows: &[SessionBlockRow],
    workspace_rows: &[WorkspaceRow],
    width: u16,
    linger_days: u64,
    include_ended: bool,
    view: BoardView,
) -> RenderedBoard {
    let cooldowns = Cooldowns::default();
    render_at(
        rows,
        session_rows,
        workspace_rows,
        width,
        linger_days,
        include_ended,
        view,
        RenderState {
            now_ms: (now_secs() * 1000.0) as u64,
            loading: false,
            cooldowns: &cooldowns,
        },
    )
}

#[derive(Clone, Copy)]
struct RenderState<'a> {
    now_ms: u64,
    loading: bool,
    /// Which badges and status cells changed recently, for their tints.
    cooldowns: &'a Cooldowns,
}

/// One rendered frame: the styled lines plus where each board row landed,
/// so selection and the quick look pane can address rows by frame position.
struct RenderedBoard {
    lines: Vec<String>,
    spans: Vec<RowSpan>,
}

#[allow(clippy::too_many_arguments)]
fn render_at(
    rows: &[BoardRow],
    session_rows: &[SessionBlockRow],
    workspace_rows: &[WorkspaceRow],
    width: u16,
    linger_days: u64,
    include_ended: bool,
    view: BoardView,
    state: RenderState<'_>,
) -> RenderedBoard {
    let RenderState {
        now_ms,
        loading,
        cooldowns,
    } = state;
    let BoardView {
        detail,
        oldest_visible_bucket,
        mode,
    } = view;
    let now = now_ms / 1000;
    let throbber_frame = crate::ui::throbber_frame_index();
    let visible_row_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (entry_bucket(row.entry, now) <= oldest_visible_bucket).then_some(index)
        })
        .collect();
    let visible_session_indices: Vec<usize> = session_rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (session_entry_bucket(row.entry, now) <= oldest_visible_bucket).then_some(index)
        })
        .collect();
    let visible_workspace_indices: Vec<usize> = workspace_rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (activity_bucket(row.entry.sessions(), now) <= oldest_visible_bucket).then_some(index)
        })
        .collect();
    let session_count = visible_row_indices
        .iter()
        .flat_map(|&index| rows[index].entry.sessions.iter())
        .chain(
            visible_session_indices
                .iter()
                .map(|&index| &session_rows[index].entry.session),
        )
        .chain(
            visible_workspace_indices
                .iter()
                .map(|&index| &workspace_rows[index].entry.session),
        )
        .map(|session| {
            if session.session_id.is_empty() {
                session.dir_name.as_str()
            } else {
                session.session_id.as_str()
            }
        })
        .collect::<HashSet<_>>()
        .len();
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
    let source = if include_ended {
        format!(
            "{}{UNDERLINE}s{RESET_UNDERLINE} all sessions{}",
            fg(color::YELLOW),
            fg(color::DARK_GRAY)
        )
    } else {
        format!("{UNDERLINE}s{RESET_UNDERLINE} live")
    };
    let mode_label = header_control('p', mode.label(), mode != BoardMode::default());
    let detail_label = header_control('r', detail_name(detail), detail != DEFAULT_DETAIL);
    let age_label = if oldest_visible_bucket == DEFAULT_OLDEST_VISIBLE_BUCKET {
        header_control('a', "all ages", false)
    } else {
        let label = format!("age ≤ {}", oldest_visible_bucket.max_age_label());
        header_control('a', &label, true)
    };

    // A PR touched by several session blocks is still one PR in the count.
    let pr_count = visible_row_indices
        .iter()
        .map(|&index| &rows[index].entry.pr)
        .chain(
            visible_session_indices
                .iter()
                .flat_map(|&index| session_rows[index].entry.prs.iter().map(|entry| &entry.pr)),
        )
        .map(|pr| (pr.owner.as_str(), pr.repo.as_str(), pr.number))
        .collect::<HashSet<_>>()
        .len();
    let mut spans: Vec<RowSpan> = Vec::new();
    let mut lines = vec![
        format!(
            "{}⑆ Crabigator PR board{}  {}{} PRs · {} sessions · {} · {}{}{}{} · {UNDERLINE}↑↓{RESET_UNDERLINE} select · {UNDERLINE}⏎{RESET_UNDERLINE} peek · {UNDERLINE}/{RESET_UNDERLINE} search · {UNDERLINE}w{RESET_UNDERLINE} watch · {UNDERLINE}+/-{RESET_UNDERLINE} days · {UNDERLINE}q{RESET_UNDERLINE} quit{}",
            fg(color::PURPLE),
            RESET_FG,
            fg(color::DARK_GRAY),
            pr_count,
            session_count,
            source,
            window,
            mode_label,
            detail_label,
            age_label,
            RESET_FG,
        ),
        String::new(),
    ];
    if loading {
        let spinner = crate::ui::session_state_icon(SessionState::Thinking, throbber_frame);
        lines.push(format!(
            "{} {}Loading live sessions… {} found{}",
            spinner,
            fg(color::CYAN),
            session_count,
            RESET_FG,
        ));
        lines.push(String::new());
    }
    if visible_row_indices.is_empty()
        && visible_session_indices.is_empty()
        && visible_workspace_indices.is_empty()
    {
        // No rows drawn, so `spans` is still empty: nothing to select.
        if !loading {
            lines.push(format!(
                "{}No live sessions or tracked PRs.{}",
                fg(color::GRAY),
                RESET_FG
            ));
        }
        return RenderedBoard { lines, spans };
    }

    let activity_width = visible_row_indices
        .iter()
        .map(|&index| {
            // A PR no session touches shows only an age stamp, capped at
            // four cells; the sessions set the real column width.
            pr_view_activity_cell(rows[index].entry, now * 1000, 4, throbber_frame, cooldowns)
                .visible
        })
        .chain(visible_session_indices.iter().map(|&index| {
            activity_cell(
                std::slice::from_ref(&session_rows[index].entry.session),
                now_ms,
                throbber_frame,
                cooldowns,
            )
            .visible
        }))
        .chain(visible_workspace_indices.iter().map(|&index| {
            activity_cell(
                workspace_rows[index].entry.sessions(),
                now_ms,
                throbber_frame,
                cooldowns,
            )
            .visible
        }))
        .max()
        .unwrap_or(0);
    let shared_width = (width as usize)
        .saturating_sub(activity_span(activity_width))
        .min(u16::MAX as usize) as u16;
    let visible_entries: Vec<&BoardPr> = visible_row_indices
        .iter()
        .map(|&index| rows[index].entry)
        .chain(
            visible_session_indices
                .iter()
                .flat_map(|&index| session_rows[index].entry.prs.iter()),
        )
        .collect();
    let pr_refs: Vec<&SessionPr> = visible_entries.iter().map(|entry| &entry.pr).collect();
    let mut widths = PrColumnWidths::from_pr_refs(&pr_refs, shared_width as usize);
    for entry in &visible_entries {
        let title = pr_row_title(entry);
        widths.include_board_identity(&entry.pr, &title, shared_width as usize);
    }
    for &index in &visible_workspace_indices {
        let entry = workspace_rows[index].entry;
        let (title, _) = workspace_title(entry);
        widths.include_board_row(
            &format!("◇ {title}"),
            &workspace_diff_text(entry),
            &workspace_branch_text(entry),
            shared_width as usize,
        );
    }
    if detail > DEFAULT_DETAIL {
        // Compact view never draws Slack links, so it reserves nothing for
        // them and the title keeps the room. In recap view a link spans the
        // empty activity column too; only the excess widens the right cluster.
        for entry in &visible_entries {
            widths.include_board_detail_right(
                slack_links_width(&entry.slack_threads)
                    .saturating_sub(activity_span(activity_width)),
                shared_width as usize,
            );
        }
    }
    widths.use_pr_view_layout(shared_width as usize);

    #[derive(Clone, Copy)]
    enum SectionRow {
        Pr(usize),
        Session(usize),
        Workspace(usize),
    }

    struct RepositorySection {
        key: String,
        name: String,
        rows: Vec<(u64, SectionRow)>,
    }

    struct RecencySection {
        bucket: RecencyBucket,
        repositories: Vec<RepositorySection>,
    }

    let mut sections: Vec<RecencySection> = Vec::new();
    {
        let mut add_row = |name: String, activity: u64, row: SectionRow| {
            let bucket = RecencyBucket::from_age(now.saturating_sub(activity));
            let section_index = sections
                .iter()
                .position(|section| section.bucket == bucket)
                .unwrap_or_else(|| {
                    sections.push(RecencySection {
                        bucket,
                        repositories: Vec::new(),
                    });
                    sections.len() - 1
                });
            let repositories = &mut sections[section_index].repositories;
            let key = name.to_ascii_lowercase();
            let repository_index = repositories
                .iter()
                .position(|repository| repository.key == key)
                .unwrap_or_else(|| {
                    repositories.push(RepositorySection {
                        key,
                        name,
                        rows: Vec::new(),
                    });
                    repositories.len() - 1
                });
            let repository = &mut repositories[repository_index];
            repository.rows.push((activity, row));
        };
        for &index in &visible_row_indices {
            let row = &rows[index];
            add_row(
                format!("{}/{}", row.entry.pr.owner, row.entry.pr.repo),
                entry_recency_time(row.entry),
                SectionRow::Pr(index),
            );
        }
        for &index in &visible_session_indices {
            let row = &session_rows[index];
            // The block sits in its lead PR's repository — the first after
            // the attention sort, so the primary's repo when there is one.
            let repo = row
                .entry
                .prs
                .first()
                .map(|entry| format!("{}/{}", entry.pr.owner, entry.pr.repo))
                .unwrap_or_default();
            add_row(
                repo,
                session_entry_recency(row.entry),
                SectionRow::Session(index),
            );
        }
        for &index in &visible_workspace_indices {
            let row = &workspace_rows[index];
            let repo = if row.entry.repo_owner.is_empty() {
                row.entry.repo_name.clone()
            } else {
                format!("{}/{}", row.entry.repo_owner, row.entry.repo_name)
            };
            add_row(
                repo,
                activity_sort_time(row.entry.sessions()),
                SectionRow::Workspace(index),
            );
        }
    }

    for section in &mut sections {
        for repository in &mut section.repositories {
            // PR view keeps sessions without a PR at the end of their repo
            // section; session view interleaves everything by recency.
            repository.rows.sort_by_key(|row| {
                let workspace_last =
                    mode == BoardMode::Prs && matches!(row.1, SectionRow::Workspace(_));
                (workspace_last, std::cmp::Reverse(row.0))
            });
        }
        section
            .repositories
            .sort_by(|a, b| b.rows[0].0.cmp(&a.rows[0].0));
    }
    sections.sort_by_key(|section| section.bucket);

    let context = RowContext {
        width,
        detail,
        now_ms,
        activity_width,
        widths: &widths,
        throbber_frame,
        cooldowns,
    };
    for (section_index, section) in sections.into_iter().enumerate() {
        if section_index > 0 {
            lines.push(String::new());
        }
        let heading = format!("● {}", section.bucket.label());
        lines.push(format!(
            "{}{}{heading:<width$}{}",
            escape::bg(section.bucket.color()),
            fg(section.bucket.heading_text_color()),
            RESET,
            width = width as usize,
        ));

        for (repository_index, repository) in section.repositories.into_iter().enumerate() {
            if repository_index > 0 {
                lines.push(String::new());
            }
            lines.push(format!(
                "{}{}{}",
                fg(color::YELLOW),
                repository.name,
                RESET_FG,
            ));

            for (_, row) in repository.rows {
                let start = lines.len();
                let (key, peeks) = match row {
                    SectionRow::Pr(index) => {
                        let board_row = &rows[index];
                        lines.extend(render_pr_view_block(board_row, context));
                        let pr = &board_row.entry.pr;
                        (
                            format!("{}/{}#{}", pr.owner, pr.repo, pr.number),
                            // Sub-row order: sessions were sorted live
                            // first, so ←→ walks them top to bottom.
                            board_row
                                .entry
                                .sessions
                                .iter()
                                .filter_map(peek_target)
                                .collect(),
                        )
                    }
                    SectionRow::Session(index) => {
                        let session_row = &session_rows[index];
                        lines.extend(render_session_view_block(session_row, context));
                        (
                            format!("sess:{}", session_key(&session_row.entry.session)),
                            peek_target(&session_row.entry.session)
                                .into_iter()
                                .collect(),
                        )
                    }
                    SectionRow::Workspace(index) => {
                        let workspace_row = &workspace_rows[index];
                        lines.extend(render_workspace_board_row(workspace_row, context));
                        (
                            format!("ws:{}", session_key(&workspace_row.entry.session)),
                            peek_target(&workspace_row.entry.session)
                                .into_iter()
                                .collect(),
                        )
                    }
                };
                spans.push(RowSpan {
                    key,
                    start,
                    end: lines.len(),
                    peeks,
                });
            }
        }
    }
    RenderedBoard { lines, spans }
}

struct PrBoardTerminalGuard;

impl Drop for PrBoardTerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = write!(out, "{}", escape::WRAP_ON);
        let _ = execute!(out, DisableBracketedPaste, LeaveAlternateScreen);
        let _ = write!(out, "{}", escape::CURSOR_SHOW);
        let _ = out.flush();
        let _ = disable_raw_mode();
    }
}

/// How often the open board re-runs `gh` for each open watched PR.
const WATCHED_REFRESH: Duration = Duration::from_secs(60);
/// Minimum spacing between relays of freshly fetched watched-PR stats.
const WATCH_RELAY_THROTTLE: Duration = Duration::from_secs(30);
/// A locally added watch survives cloud list refreshes at least this long,
/// covering the window before its own add POST lands.
const WATCH_ADD_GRACE: Duration = Duration::from_secs(120);

/// The board's watch list: cloud-synced membership, locally enriched stats.
/// The open board is the one place that runs `gh` for watched PRs; it relays
/// what it learns so the web board and other machines see the same stats.
#[derive(Default)]
struct WatchedBoard {
    /// Watched PRs by `owner/repo#number`, freshest stats winning.
    prs: HashMap<String, SessionPr>,
    /// Last local `gh` attempt per key, successful or not.
    attempted_at: HashMap<String, Instant>,
    /// In-flight enrichment jobs by key.
    pending: HashMap<String, mpsc::Receiver<Result<SessionPr, String>>>,
    /// Keys added on this board, kept through cloud refreshes while the add
    /// itself may still be in flight.
    added_locally: HashMap<String, Instant>,
    /// Fresh local stats waiting to be relayed to the cloud.
    relay_due: bool,
    last_relay: Option<Instant>,
}

fn watch_key(owner: &str, repo: &str, number: u64) -> String {
    format!("{owner}/{repo}#{number}")
}

/// One cloud watch row as the boards hold it: its key, and the stats a board
/// relayed last — a bare stub until the first enrichment lands.
fn cloud_watch_entry(row: crate::cloud::CloudWatchedPr) -> (String, SessionPr) {
    let key = watch_key(&row.owner, &row.repo, row.number);
    let mut pr = row
        .pr
        .unwrap_or_else(|| SessionPr::watched_stub(&row.owner, &row.repo, row.number));
    pr.watched = true;
    (key, pr)
}

impl WatchedBoard {
    /// Adopt the cloud list: new watches join with their relayed stats,
    /// fresher local enrichment is kept, and watches removed elsewhere leave
    /// (unless just added here, before the add could land).
    fn apply_cloud(&mut self, list: Vec<crate::cloud::CloudWatchedPr>) -> bool {
        let mut changed = false;
        let mut listed: HashSet<String> = HashSet::new();
        for row in list {
            let (key, incoming) = cloud_watch_entry(row);
            listed.insert(key.clone());
            match self.prs.get(&key) {
                Some(existing) if existing.refreshed_at >= incoming.refreshed_at => {}
                _ => {
                    self.prs.insert(key, incoming);
                    changed = true;
                }
            }
        }
        self.added_locally
            .retain(|_, added| added.elapsed() < WATCH_ADD_GRACE);
        let added_locally = &self.added_locally;
        let before = self.prs.len();
        self.prs
            .retain(|key, _| listed.contains(key) || added_locally.contains_key(key));
        changed || self.prs.len() != before
    }

    /// Watch one more PR right now, ahead of the cloud round trip.
    fn add(&mut self, add: &WatchAdd) {
        let key = watch_key(&add.owner, &add.repo, add.number);
        self.added_locally.insert(key.clone(), Instant::now());
        self.prs
            .entry(key.clone())
            .or_insert_with(|| SessionPr::watched_stub(&add.owner, &add.repo, add.number));
        self.attempted_at.remove(&key);
    }

    /// Start `gh` enrichment for every watched PR that is due: never-enriched
    /// ones immediately, open ones each minute. Finished PRs only linger, so
    /// they stop refreshing.
    fn spawn_due_refreshes(&mut self) {
        let due: Vec<(String, String)> = self
            .prs
            .iter()
            .filter(|(key, pr)| {
                !self.pending.contains_key(*key)
                    && (pr.refreshed_at == 0 || pr.state == "OPEN")
                    && self
                        .attempted_at
                        .get(*key)
                        .is_none_or(|at| at.elapsed() >= WATCHED_REFRESH)
            })
            .map(|(key, pr)| (key.clone(), pr.url.clone()))
            .collect();
        for (key, url) in due {
            self.attempted_at.insert(key.clone(), Instant::now());
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(crate::pr::fetch_watched_session_pr(&url));
            });
            self.pending.insert(key, rx);
        }
    }

    /// Collect finished enrichment jobs. Returns true when stats changed.
    fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut done = Vec::new();
        for (key, rx) in &self.pending {
            match rx.try_recv() {
                Ok(Ok(pr)) => done.push((key.clone(), Some(pr))),
                // A failed fetch or a dropped worker just retries on the next
                // cadence tick.
                Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                    done.push((key.clone(), None))
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        for (key, result) in done {
            self.pending.remove(&key);
            let Some(pr) = result else {
                continue;
            };
            // The watch may have been removed while the fetch ran.
            if let Some(existing) = self.prs.get_mut(&key) {
                if *existing != pr {
                    *existing = pr;
                    changed = true;
                    self.relay_due = true;
                }
            }
        }
        changed
    }

    /// Push freshly fetched stats to the cloud so other boards see them.
    fn maybe_relay(&mut self) {
        if !self.relay_due
            || self
                .last_relay
                .is_some_and(|at| at.elapsed() < WATCH_RELAY_THROTTLE)
        {
            return;
        }
        self.relay_due = false;
        self.last_relay = Some(Instant::now());
        let prs: Vec<SessionPr> = self
            .prs
            .values()
            .filter(|pr| pr.refreshed_at > 0)
            .cloned()
            .collect();
        if prs.is_empty() {
            return;
        }
        tokio::spawn(async move {
            let _ = crate::cloud::relay_watched_pr_stats_standalone(&prs).await;
        });
    }
}

/// Overlay the watch list on the board entries: session-tracked copies gain
/// the watch flag, watch-only PRs join as session-less entries, and fresher
/// local enrichment replaces relayed stats on those.
fn merge_watched_entries(
    entries: &mut Vec<BoardPr>,
    watched: &HashMap<String, SessionPr>,
    overrides: &ScopedOverrides,
    linger_days: u64,
) {
    let now_ms = (now_secs() * 1000.0) as u64;
    for (key, pr) in watched {
        // Watches are group-level, so only a group-wide dismissal hides one.
        if matches!(overrides.group(key), Some(PrDisposition::Dismissed)) {
            continue;
        }
        if let Some(entry) = entries.iter_mut().find(|entry| {
            entry.pr.number == pr.number
                && entry.pr.owner.eq_ignore_ascii_case(&pr.owner)
                && entry.pr.repo.eq_ignore_ascii_case(&pr.repo)
        }) {
            entry.pr.watched = true;
            // Watch-only cloud rows carry relayed stats; this board's own
            // enrichment is usually fresher.
            if entry.sessions.is_empty() && pr.refreshed_at > entry.pr.refreshed_at {
                entry.pr = pr.clone();
            }
            continue;
        }
        if !visible_pr(pr, linger_days, now_ms) {
            continue;
        }
        let mut slack_threads = Vec::new();
        merge_pr_slack_threads(&mut slack_threads, pr, &[]);
        entries.push(BoardPr {
            pr: pr.clone(),
            sessions: Vec::new(),
            slack_threads,
            stale: false,
        });
    }
    sort_entries(entries);
}

fn start_watched_fetch() -> mpsc::Receiver<Vec<crate::cloud::CloudWatchedPr>> {
    let (tx, rx) = mpsc::channel();
    tokio::spawn(async move {
        if let Ok(watched) = crate::cloud::fetch_watched_prs_standalone().await {
            let _ = tx.send(watched);
        }
    });
    rx
}

/// The sticky add-a-watch banner behind the `w` key: paste a PR URL or
/// `owner/repo#123` and Enter adds it to the watch list.
fn watch_banner(input: &str, error: Option<&str>, width: u16) -> String {
    let hint = match error {
        Some(error) => format!("{error} · edit and ⏎ to retry · Esc cancels"),
        None => "paste a PR URL or owner/repo#123 · ⏎ add · Esc cancels".to_string(),
    };
    let text = format!(" ◉ watch {input}▏ {hint} ");
    let padded = format!("{text:<width$}", width = width as usize);
    format!("{}{}{}{}", escape::bg(color::YELLOW), fg(16), padded, RESET)
}

/// Entry point for `crabigator prs`.
pub async fn run_prs_board(once: bool) -> Result<()> {
    if once {
        let overrides = crate::cloud::fetch_pr_overrides_standalone()
            .await
            .unwrap_or_default();
        let width = terminal_size().map(|(w, _)| w).unwrap_or(120);
        let mut activity_history = ActivityHistory::default();
        // One frame in the same view the interactive board last used.
        let preferences = crate::config::Config::load().unwrap_or_default().pr_board;
        let view = BoardView::new(
            preferences.detail,
            RecencyBucket::from_max_age_hours(preferences.oldest_visible_hours),
        )
        .with_mode(BoardMode::parse(&preferences.view));
        let linger_days = preferences.linger_days.min(MAX_LINGER_DAYS);
        let (mut entries, workspaces, _) =
            local_board(&mut activity_history, &overrides, linger_days)?;
        // Watched PRs render from their cloud-relayed stats; the one-frame
        // print doesn't run its own `gh` enrichment.
        let watched: HashMap<String, SessionPr> = crate::cloud::fetch_watched_prs_standalone()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(cloud_watch_entry)
            .collect();
        merge_watched_entries(&mut entries, &watched, &overrides, linger_days);
        let shaped = entries_for_mode(entries, view.mode);
        let rows: Vec<BoardRow> = shaped
            .prs
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let session_rows: Vec<SessionBlockRow> = shaped
            .sessions
            .iter()
            .map(|entry| SessionBlockRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let workspace_rows: Vec<WorkspaceRow> = workspaces
            .iter()
            .map(|entry| WorkspaceRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        for line in render(
            &rows,
            &session_rows,
            &workspace_rows,
            width,
            linger_days,
            false,
            view,
        )
        .lines
        {
            println!("{line}");
        }
        return Ok(());
    }

    let mut out = stdout();
    enable_raw_mode()?;
    let _terminal_guard = PrBoardTerminalGuard;
    execute!(out, EnterAlternateScreen, EnableBracketedPaste)?;
    // Auto-wrap stays off for the whole board: the quick look pane replays
    // another session's screen, whose lines can be wider than this terminal,
    // and clipping them at the margin beats corrupting the layout.
    write!(out, "{}{}", escape::CURSOR_HIDE, escape::WRAP_OFF)?;
    out.flush()?;
    let mut overrides = ScopedOverrides::default();
    let overrides_fetch = start_overrides_fetch();
    board_loop(&mut out, &mut overrides, overrides_fetch).await
}

fn start_overrides_fetch() -> mpsc::Receiver<ScopedOverrides> {
    let (tx, rx) = mpsc::channel();
    tokio::spawn(async move {
        if let Ok(overrides) = crate::cloud::fetch_pr_overrides_standalone().await {
            let _ = tx.send(overrides);
        }
    });
    rx
}

/// Identity of one rendered frame, so an unchanged board isn't repainted.
fn frame_hash(lines: &[String]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lines.hash(&mut hasher);
    hasher.finish()
}

/// Positions in `spans` the selection can land on: rows with a live screen.
fn selectable_positions(spans: &[RowSpan]) -> Vec<usize> {
    spans
        .iter()
        .enumerate()
        .filter_map(|(index, span)| (!span.peeks.is_empty()).then_some(index))
        .collect()
}

/// Every peekable (row, session) pair in board order, so ←→ can walk through
/// a PR row's sessions and continue into the neighboring rows.
fn peek_positions(spans: &[RowSpan], selectable: &[usize]) -> Vec<(usize, usize)> {
    selectable
        .iter()
        .flat_map(|&span_index| {
            (0..spans[span_index].peeks.len()).map(move |peek_index| (span_index, peek_index))
        })
        .collect()
}

/// The peekable session `step` away from the current one, walking within a
/// row's sessions before crossing into the next row, clamped at the ends.
fn step_peek_target<'a>(
    spans: &'a [RowSpan],
    selectable: &[usize],
    selected: Option<&str>,
    peek_index: usize,
    step: isize,
) -> Option<(&'a RowSpan, usize)> {
    let positions = peek_positions(spans, selectable);
    if positions.is_empty() {
        return None;
    }
    let current = selected.and_then(|key| {
        positions.iter().position(|&(span_index, index)| {
            let span = &spans[span_index];
            span.key == key && index == peek_index.min(span.peeks.len().saturating_sub(1))
        })
    });
    let next = match current {
        Some(position) => {
            (position as isize + step).clamp(0, positions.len() as isize - 1) as usize
        }
        None if step > 0 => 0,
        None => positions.len() - 1,
    };
    let (span_index, index) = positions[next];
    Some((&spans[span_index], index))
}

/// Where the current selection sits among `selectable`, when its row is still
/// on the board with a live screen.
fn selected_position(
    spans: &[RowSpan],
    selectable: &[usize],
    selected: Option<&str>,
) -> Option<usize> {
    let key = selected?;
    selectable.iter().position(|&index| spans[index].key == key)
}

/// The selectable row `step` away from the current selection, clamped at the
/// list's ends; entering the list from nowhere starts at the top. The caller
/// handles stepping up from the top row (it clears the selection) and up
/// with nothing selected (it scrolls), so only downward entry lands here.
fn step_selection<'a>(
    spans: &'a [RowSpan],
    selectable: &[usize],
    selected: Option<&str>,
    step: isize,
) -> Option<&'a RowSpan> {
    if selectable.is_empty() {
        return None;
    }
    let next = match selected_position(spans, selectable, selected) {
        Some(position) => {
            (position as isize + step).clamp(0, selectable.len() as isize - 1) as usize
        }
        None => 0,
    };
    Some(&spans[selectable[next]])
}

/// Repaint one already-styled frame line onto the selection band. Every full
/// SGR reset inside the line is followed by re-asserting the band background,
/// so a mid-line reset can't cut the band off partway; the tail is padded so
/// the band covers the full row width.
fn selection_band_line(line: &str, width: u16) -> String {
    let band = escape::bg(color::BG_SELECTED);
    let rebanded = line.replace(RESET, &format!("{RESET}{band}"));
    let pad = (width as usize).saturating_sub(crate::ui::utils::strip_ansi_len(line));
    format!("{band}{rebanded}{}{RESET}", " ".repeat(pad))
}

/// Scroll just enough to bring the span fully into the visible page; a block
/// taller than the page aligns its top edge.
fn scroll_to_reveal(scroll: usize, page: usize, span: &RowSpan) -> usize {
    if span.start < scroll {
        span.start
    } else if span.end > scroll + page {
        span.end.saturating_sub(page).min(span.start)
    } else {
        scroll
    }
}

/// Rows the quick look pane occupies: the bottom half of the terminal.
fn peek_pane_rows(height: u16) -> usize {
    (height as usize) / 2
}

/// The peeked session, resolved from the current selection. A `peek_index`
/// past the row's list (the row changed under the selection) falls back to
/// its first session.
fn selected_peek<'a>(
    spans: &'a [RowSpan],
    selected: Option<&str>,
    peek_index: usize,
) -> Option<&'a PeekTarget> {
    let span = selected.and_then(|key| spans.iter().find(|span| span.key == key))?;
    span.peeks.get(peek_index).or_else(|| span.peeks.first())
}

/// Step the pane's scrollback anchor by `delta` lines. `None` is the live
/// screen; scrolling up anchors a window into the transcript at a fixed top
/// line so new output can't shift what's being read. Scrolling down past the
/// transcript's tail returns to the live screen.
fn step_peek_scroll(
    current: Option<usize>,
    delta: isize,
    total_lines: usize,
    interior: usize,
) -> Option<usize> {
    let tail_top = total_lines.saturating_sub(interior);
    match current {
        None if delta < 0 && tail_top > 0 => Some(tail_top.saturating_sub(delta.unsigned_abs())),
        None => None,
        Some(top) if delta < 0 => Some(top.saturating_sub(delta.unsigned_abs())),
        Some(top) => {
            let next = top.saturating_add(delta.unsigned_abs());
            (next < tail_top).then_some(next)
        }
    }
}

/// The quick look pane: the previewed session boxed in a bright border on
/// all four sides, so it reads as a temporary read-only overlay rather than
/// more board content. At `scroll: None` the pane mirrors the live screen;
/// with an anchor it shows a window into the session's transcript starting
/// at that line. `rows` is the pane height including both border rows;
/// lines wider than the interior clip at the right border, which is pinned
/// by column addressing because auto-wrap is off.
#[allow(clippy::too_many_arguments)]
fn build_peek_pane(
    spans: &[RowSpan],
    selected: Option<&str>,
    peek_index: usize,
    width: u16,
    rows: usize,
    scroll: Option<usize>,
    transcripts: &mut TranscriptCache,
) -> Vec<String> {
    if rows == 0 {
        return Vec::new();
    }
    let target = selected_peek(spans, selected, peek_index);
    let interior = rows.saturating_sub(2);

    let (mut content, lines_back) = match (target, scroll) {
        (Some(target), Some(top)) => {
            let transcript = transcripts.lines(&target.session_dir).unwrap_or(&[]);
            let back = transcript.len().saturating_sub(top + interior);
            let window = transcript
                .iter()
                .skip(top)
                .take(interior)
                .cloned()
                .collect();
            (window, Some(back))
        }
        (Some(target), None) => {
            let screen = std::fs::read(target.session_dir.join("screen.txt")).ok();
            match screen {
                Some(bytes) => {
                    let content = String::from_utf8_lossy(&bytes);
                    let skip = content.lines().count().saturating_sub(interior);
                    (content.lines().skip(skip).map(String::from).collect(), None)
                }
                None => (vec![peek_placeholder()], None),
            }
        }
        (None, _) => (vec![peek_placeholder()], None),
    };

    let mut lines = vec![peek_top_border(target, width, lines_back)];
    if rows == 1 {
        return lines;
    }
    // Fill the box to the bottom so the frame doesn't jump with content.
    content.truncate(interior);
    content.resize(interior, String::new());
    lines.extend(
        content
            .into_iter()
            .map(|row| peek_interior_row(&row, width)),
    );
    lines.push(peek_bottom_border(width));
    lines
}

fn peek_placeholder() -> String {
    format!(
        "{} no live screen — the session ended or isn't captured{}",
        fg(color::GRAY),
        RESET_FG
    )
}

/// The box's top edge, carrying the pane title and its keys:
/// `┏━ ▶ title · dir ━━…━━ ←→ switch · ↑↓ scroll · ⏎ close ━┓`.
/// While scrolled into the transcript, the glyph flips to `≡` and the keys
/// give way to how far back the view is anchored.
fn peek_top_border(target: Option<&PeekTarget>, width: u16, lines_back: Option<usize>) -> String {
    let width = width as usize;
    let (title, dir) = match target {
        Some(target) if target.dir_name.is_empty() || target.title == target.dir_name => {
            (target.title.as_str(), String::new())
        }
        Some(target) => (target.title.as_str(), format!(" · {}", target.dir_name)),
        None => ("quick look", String::new()),
    };
    let (glyph, hint_text) = match lines_back {
        Some(back) => ("≡", format!("{back} back · ↓ live · ⏎ close")),
        None => ("▶", "←→ switch · ↑↓ scroll · ⏎ close".to_string()),
    };
    // `┏━ ` + label (+ dir) + ` ` + ━ fill (+ ` hint `) + `━┓`
    let label = crate::ui::pr_cells::truncate_to_width(
        &format!("{glyph} {title}"),
        width.saturating_sub(6),
    );
    let dir = if 6 + label.width() + dir.width() > width {
        String::new()
    } else {
        dir
    };
    let used = 3 + label.width() + dir.width() + 1 + 2;
    let hint = if used + hint_text.width() + 2 <= width {
        format!(" {hint_text} ")
    } else {
        String::new()
    };
    let fill = "━".repeat(width.saturating_sub(used + hint.width()));
    format!(
        "{border}┏━ {RESET_FG}{label}{dim}{dir}{border} {fill}{dim}{hint}{border}━┓{RESET_FG}",
        border = fg(color::CYAN),
        dim = fg(color::DARK_GRAY),
    )
}

/// One boxed screen row. The right edge is drawn at the last column with
/// cursor addressing, on top of whatever the (possibly overlong, arbitrarily
/// styled) screen line left there.
fn peek_interior_row(content: &str, width: u16) -> String {
    format!(
        "{}┃{RESET} {content}{RESET}{}{}┃{RESET}",
        fg(color::CYAN),
        escape::cursor_col(width.max(1)),
        fg(color::CYAN),
    )
}

fn peek_bottom_border(width: u16) -> String {
    format!(
        "{}┗{}┛{}",
        fg(color::CYAN),
        "━".repeat((width as usize).saturating_sub(2)),
        RESET_FG
    )
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

fn draw_changed_lines<W: Write>(
    out: &mut W,
    previous: &mut Vec<String>,
    current: &[String],
) -> std::io::Result<bool> {
    let changed: Vec<usize> = (0..previous.len().max(current.len()))
        .filter(|&index| previous.get(index) != current.get(index))
        .collect();
    if changed.is_empty() {
        return Ok(false);
    }

    write!(out, "{}", escape::SYNC_BEGIN)?;
    for index in changed {
        write!(
            out,
            "{}{}{}",
            escape::cursor_to(index as u16 + 1, 1),
            RESET,
            escape::CLEAR_LINE,
        )?;
        if let Some(line) = current.get(index) {
            write!(out, "{line}")?;
        }
    }
    write!(out, "{}", escape::SYNC_END)?;
    *previous = current.to_vec();
    Ok(true)
}

fn save_board_preferences(include_ended: bool, linger_days: u64, view: BoardView) -> Result<()> {
    let mut config = crate::config::Config::load()?;
    config.pr_board.include_ended = include_ended;
    config.pr_board.detail = view.detail;
    config.pr_board.linger_days = linger_days;
    config.pr_board.oldest_visible_hours = view.oldest_visible_bucket.max_age_hours();
    config.pr_board.view = view.mode.label().to_string();
    config.save()
}

async fn board_loop(
    out: &mut std::io::Stdout,
    overrides: &mut ScopedOverrides,
    overrides_fetch: mpsc::Receiver<ScopedOverrides>,
) -> Result<()> {
    let mut drawn_lines: Vec<String> = Vec::new();
    let mut last_refresh = Instant::now();
    let mut entries: Vec<BoardPr> = Vec::new();
    let mut workspaces: Vec<WorkspaceEntry> = Vec::new();
    let mut durable_workspaces: Vec<WorkspaceEntry> = Vec::new();
    let mut durable_history_loaded = false;
    let mut matched = 0usize;
    let mut scroll: usize = 0;
    let mut search: Option<String> = None;
    // Session selection and the Enter-toggled quick look pane. `peek_scroll`
    // anchors the pane into the session's transcript; None mirrors the live
    // screen.
    let mut spans: Vec<RowSpan> = Vec::new();
    let mut selected: Option<String> = None;
    let mut peek_open = false;
    let mut peek_scroll: Option<usize> = None;
    // Which of the selected row's sessions the pane mirrors (a PR block can
    // hold several; a session block just its own).
    let mut peek_index: usize = 0;
    let mut pane_lines: Vec<String> = Vec::new();
    // Transcript search state: cached scrollbacks plus the Tab-toggled
    // context view for the inline previews.
    let mut transcripts = TranscriptCache::default();
    let mut activity_history = None;
    let mut initial_snapshots = Vec::new();
    let mut loading = true;
    let mut pending_overrides = Some(overrides_fetch);
    let mut overrides_fetched = Instant::now();
    let mut expanded = false;
    // The cloud-synced watch list plus this board's `gh` enrichment of it,
    // and the `w` add-a-watch input with its parse error while open.
    let mut watched_board = WatchedBoard::default();
    let mut pending_watched = Some(start_watched_fetch());
    let mut watched_fetched = Instant::now();
    let mut add_input: Option<String> = None;
    let mut add_error: Option<String> = None;
    let preferences = crate::config::Config::load().unwrap_or_default().pr_board;
    let mut view = BoardView::new(
        preferences.detail,
        RecencyBucket::from_max_age_hours(preferences.oldest_visible_hours),
    )
    .with_mode(BoardMode::parse(&preferences.view));
    let mut linger_days = preferences.linger_days.min(MAX_LINGER_DAYS);
    let mut include_ended = preferences.include_ended;
    // Cloud fetches are throttled well below the local tick; toggling the
    // source or changing the day window forces one.
    let mut cloud_fetch_due = false;
    let mut cloud_fetched: Option<Instant> = None;
    let mut dirty = false;
    let mut needs_render = false;
    let mut last_throbber_frame = crate::ui::throbber_frame_index();
    // Badges and status cells that changed recently glow and cool; the board
    // keeps repainting while any are still warm.
    let mut cooldowns = Cooldowns::default();

    let (initial_width, _) = terminal_size().unwrap_or((120, 40));
    let mut lines = render_at(
        &[],
        &[],
        &[],
        initial_width,
        linger_days,
        include_ended,
        view,
        RenderState {
            now_ms: (now_secs() * 1000.0) as u64,
            loading: true,
            cooldowns: &cooldowns,
        },
    )
    .lines;
    let mut last_frame_hash = frame_hash(&lines);
    draw_changed_lines(out, &mut drawn_lines, &lines)?;
    out.flush()?;
    let mut initial_load = Some(start_initial_local_load());

    loop {
        if let Some(rx) = pending_overrides.as_ref() {
            match rx.try_recv() {
                Ok(fresh) => {
                    *overrides = fresh;
                    pending_overrides = None;
                    if loading {
                        entries = aggregate(&initial_snapshots, overrides, linger_days);
                        workspaces = local_workspaces(&initial_snapshots, &entries);
                        needs_render = true;
                    } else {
                        last_refresh = Instant::now() - REFRESH_INTERVAL;
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => pending_overrides = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if pending_overrides.is_none() && overrides_fetched.elapsed() >= OVERRIDES_REFRESH {
            overrides_fetched = Instant::now();
            pending_overrides = Some(start_overrides_fetch());
        }

        // The watch list follows the overrides cadence; between fetches this
        // board enriches watched PRs itself and relays what it learned.
        if let Some(rx) = pending_watched.as_ref() {
            match rx.try_recv() {
                Ok(list) => {
                    pending_watched = None;
                    if watched_board.apply_cloud(list) {
                        needs_render = true;
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => pending_watched = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if pending_watched.is_none() && watched_fetched.elapsed() >= OVERRIDES_REFRESH {
            watched_fetched = Instant::now();
            pending_watched = Some(start_watched_fetch());
        }
        watched_board.spawn_due_refreshes();
        if watched_board.poll() {
            needs_render = true;
        }
        watched_board.maybe_relay();

        let mut initial_finished = false;
        if let Some(rx) = initial_load.as_ref() {
            for _ in 0..INITIAL_LOAD_BATCH {
                match rx.try_recv() {
                    Ok(InitialLoadUpdate::Snapshot(snapshot)) => {
                        initial_snapshots.push(*snapshot);
                        entries = aggregate(&initial_snapshots, overrides, linger_days);
                        workspaces = local_workspaces(&initial_snapshots, &entries);
                        needs_render = true;
                    }
                    Ok(InitialLoadUpdate::Complete(history)) => {
                        activity_history = Some(*history);
                        initial_finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        activity_history = Some(ActivityHistory::default());
                        initial_finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
        }
        if initial_finished {
            initial_load = None;
            loading = false;
            last_refresh = Instant::now();
            needs_render = true;
        }

        if !loading && last_refresh.elapsed() >= REFRESH_INTERVAL {
            last_refresh = Instant::now();
            if include_ended {
                let stale = cloud_fetched.is_none_or(|at| at.elapsed() >= CLOUD_BOARD_REFRESH);
                if cloud_fetch_due || stale {
                    cloud_fetch_due = false;
                    cloud_fetched = Some(Instant::now());
                    if let Ok(cloud) = crate::cloud::fetch_pr_board_standalone(linger_days).await {
                        (entries, durable_workspaces) = cloud_entries_to_board(cloud);
                        durable_history_loaded = true;
                    }
                }
                let (live_entries, live_workspaces, live_mirrors) =
                    local_board(activity_history.as_mut().unwrap(), overrides, linger_days)?;
                if !durable_history_loaded {
                    entries = live_entries;
                }
                workspaces = durable_workspaces.clone();
                merge_live_workspaces(&mut workspaces, live_workspaces, &entries);
                attach_live_mirrors(&mut entries, &mut workspaces, &live_mirrors);
            } else {
                (entries, workspaces, _) =
                    local_board(activity_history.as_mut().unwrap(), overrides, linger_days)?;
            }
            activity_history
                .as_mut()
                .unwrap()
                .enrich_slack_threads(&mut entries);
            needs_render = true;
        }

        let now_ms = (now_secs() * 1000.0) as u64;
        let animating =
            loading || board_has_thinking(&entries, &workspaces) || cooldowns.active(now_ms);
        let throbber_frame = crate::ui::throbber_frame_index();
        if animating && throbber_frame != last_throbber_frame {
            last_throbber_frame = throbber_frame;
            needs_render = true;
        }

        let (width, height) = terminal_size().unwrap_or((120, 40));
        if needs_render {
            needs_render = false;
            let query = search.as_deref().unwrap_or("");
            let now = now_ms / 1000;
            // The canonical entries stay merged (one per PR); the watch list
            // overlays them, then the active view shapes its own rows.
            let mut merged_entries = entries.clone();
            merge_watched_entries(
                &mut merged_entries,
                &watched_board.prs,
                overrides,
                linger_days,
            );
            observe_cooldowns(&mut cooldowns, &merged_entries, &workspaces, now_ms);
            let shaped = entries_for_mode(merged_entries, view.mode);
            // A row stays visible when its metadata matches, or when any of
            // its sessions' transcripts contain the query — with the matched
            // excerpt shown inline so the hit can be confirmed.
            let filtered: Vec<BoardRow> = shaped
                .prs
                .iter()
                .filter_map(|entry| {
                    if entry_bucket(entry, now) > view.oldest_visible_bucket {
                        return None;
                    }
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
            let filtered_sessions: Vec<SessionBlockRow> = shaped
                .sessions
                .iter()
                .filter_map(|entry| {
                    if session_entry_bucket(entry, now) > view.oldest_visible_bucket {
                        return None;
                    }
                    let preview_lines = build_session_previews(
                        std::slice::from_ref(&entry.session),
                        &mut transcripts,
                        query,
                        width,
                        expanded,
                    );
                    (session_entry_matches_search(entry, query) || !preview_lines.is_empty())
                        .then_some(SessionBlockRow {
                            entry,
                            preview_lines,
                        })
                })
                .collect();
            let filtered_workspaces: Vec<WorkspaceRow> = workspaces
                .iter()
                .filter_map(|entry| {
                    if activity_bucket(entry.sessions(), now) > view.oldest_visible_bucket {
                        return None;
                    }
                    let preview_lines = build_session_previews(
                        entry.sessions(),
                        &mut transcripts,
                        query,
                        width,
                        expanded,
                    );
                    (workspace_matches_search(entry, query) || !preview_lines.is_empty()).then_some(
                        WorkspaceRow {
                            entry,
                            preview_lines,
                        },
                    )
                })
                .collect();
            matched = filtered.len() + filtered_sessions.len() + filtered_workspaces.len();
            let fresh = render_at(
                &filtered,
                &filtered_sessions,
                &filtered_workspaces,
                width,
                linger_days,
                include_ended,
                view,
                RenderState {
                    now_ms,
                    loading,
                    cooldowns: &cooldowns,
                },
            );
            // Spans track the fresh frame even when the lines are unchanged:
            // a session's screen can appear or vanish without moving a row.
            spans = fresh.spans;
            let hash = frame_hash(&fresh.lines);
            if hash != last_frame_hash {
                last_frame_hash = hash;
                lines = fresh.lines;
                dirty = true;
            }
        }

        // The banner is pinned above the scrolled content while searching;
        // the quick look pane claims the bottom half of the terminal.
        let banner_rows = usize::from(search.is_some()) + usize::from(add_input.is_some());
        let pane_rows = if peek_open { peek_pane_rows(height) } else { 0 };
        let page = (height as usize)
            .saturating_sub(banner_rows)
            .saturating_sub(pane_rows)
            .max(1);
        let max_scroll = lines.len().saturating_sub(page);
        if scroll > max_scroll {
            scroll = max_scroll;
            dirty = true;
        }

        // Re-read the previewed screen every pass; the diff-drawing below
        // only repaints when its content actually changed.
        if peek_open {
            let fresh_pane = build_peek_pane(
                &spans,
                selected.as_deref(),
                peek_index,
                width,
                pane_rows,
                peek_scroll,
                &mut transcripts,
            );
            if fresh_pane != pane_lines {
                pane_lines = fresh_pane;
                dirty = true;
            }
        } else if !pane_lines.is_empty() {
            pane_lines.clear();
            dirty = true;
        }

        if dirty {
            dirty = false;
            let mut visible_lines = Vec::new();
            if let Some(query) = &search {
                visible_lines.push(search_banner(query, matched, width));
            }
            if let Some(input) = &add_input {
                visible_lines.push(watch_banner(input, add_error.as_deref(), width));
            }
            let band = selected
                .as_deref()
                .and_then(|key| spans.iter().find(|span| span.key == key));
            for (offset, line) in lines.iter().skip(scroll).take(page).enumerate() {
                let index = scroll + offset;
                let on_selected_row =
                    band.is_some_and(|span| (span.start..span.end).contains(&index));
                visible_lines.push(if on_selected_row {
                    selection_band_line(line, width)
                } else {
                    line.clone()
                });
            }
            if peek_open {
                // Pin the pane to the bottom half even when the board is short.
                visible_lines.resize(banner_rows + page, String::new());
                visible_lines.extend(pane_lines.iter().cloned());
            }
            if draw_changed_lines(out, &mut drawn_lines, &visible_lines)? {
                out.flush()?;
            }
        }

        let poll_interval = if animating || peek_open {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(250)
        };
        if poll(poll_interval)? {
            match read()? {
                Event::Paste(text) => {
                    // Bracketed paste arrives as one event. A paste outside
                    // an input is inert instead of firing every matching
                    // shortcut in the pasted text.
                    if let Some(input) = &mut add_input {
                        input.push_str(text.trim());
                        add_error = None;
                        dirty = true;
                    } else if let Some(query) = &mut search {
                        query.push_str(&text);
                        scroll = 0;
                        needs_render = true;
                        dirty = true;
                    }
                }
                Event::Key(key) => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        save_board_preferences(include_ended, linger_days, view)?;
                        return Ok(());
                    }
                    // While the add-a-watch input is open, printable keys edit
                    // it; Enter parses and adds, Esc closes. The flag keeps
                    // the submit's Enter from also toggling the peek pane.
                    let add_input_consumed = add_input.is_some();
                    if let Some(input) = &mut add_input {
                        match key.code {
                            KeyCode::Esc => {
                                add_input = None;
                                add_error = None;
                                dirty = true;
                            }
                            KeyCode::Enter => match crate::pr::parse_watch_target(input) {
                                Some(add) => {
                                    watched_board.add(&add);
                                    tokio::spawn(async move {
                                        let _ = crate::cloud::add_watched_pr_standalone(&add).await;
                                    });
                                    add_input = None;
                                    add_error = None;
                                    needs_render = true;
                                    dirty = true;
                                }
                                None => {
                                    add_error = Some("not a PR URL or owner/repo#123".to_string());
                                    dirty = true;
                                }
                            },
                            KeyCode::Backspace => {
                                input.pop();
                                add_error = None;
                                dirty = true;
                            }
                            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                input.push(c);
                                add_error = None;
                                dirty = true;
                            }
                            _ => {}
                        }
                    }
                    // While a filter is active, printable keys edit the query
                    // and Esc clears it; outside one, they're the shortcuts.
                    else if let Some(query) = &mut search {
                        match key.code {
                            // Esc peels back one layer: the pane first, then
                            // the filter.
                            KeyCode::Esc if peek_open => {
                                peek_open = false;
                                peek_scroll = None;
                                dirty = true;
                            }
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
                            KeyCode::Esc if peek_open => {
                                peek_open = false;
                                peek_scroll = None;
                                dirty = true;
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                save_board_preferences(include_ended, linger_days, view)?;
                                return Ok(());
                            }
                            KeyCode::Char('/') => {
                                search = Some(String::new());
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            // Watch a PR that no session is working on.
                            KeyCode::Char('w') => {
                                add_input = Some(String::new());
                                add_error = None;
                                dirty = true;
                            }
                            // Widen or narrow how long finished PRs linger.
                            // The filter runs during aggregation (server-side
                            // for the cloud view), so force a fresh pass
                            // rather than waiting out the tick.
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                linger_days = (linger_days + 1).min(MAX_LINGER_DAYS);
                                save_board_preferences(include_ended, linger_days, view)?;
                                cloud_fetch_due = true;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                linger_days = linger_days.saturating_sub(1);
                                save_board_preferences(include_ended, linger_days, view)?;
                                cloud_fetch_due = true;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            // Flip between one block per primary PR and one
                            // block per session. Selection keys differ
                            // between the views, so it starts fresh.
                            KeyCode::Char('p') => {
                                view.toggle_mode();
                                save_board_preferences(include_ended, linger_days, view)?;
                                selected = None;
                                peek_open = false;
                                peek_scroll = None;
                                peek_index = 0;
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            // Recap visibility and age filtering are independent.
                            KeyCode::Char('r') => {
                                view.toggle_recap();
                                save_board_preferences(include_ended, linger_days, view)?;
                                needs_render = true;
                                dirty = true;
                            }
                            KeyCode::Char('a') => {
                                view.cycle_age();
                                save_board_preferences(include_ended, linger_days, view)?;
                                scroll = 0;
                                needs_render = true;
                                dirty = true;
                            }
                            // Flip between live mirrors and the durable cloud
                            // record, which includes ended sessions.
                            KeyCode::Char('s') => {
                                include_ended = !include_ended;
                                save_board_preferences(include_ended, linger_days, view)?;
                                cloud_fetch_due = true;
                                scroll = 0;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            _ => {}
                        }
                    }
                    // Enter toggles the quick look pane on the selected
                    // session (selecting the first live one if none is yet) —
                    // unless the add-a-watch input just consumed it.
                    let selectable = selectable_positions(&spans);
                    if !add_input_consumed && key.code == KeyCode::Enter {
                        if peek_open {
                            peek_open = false;
                            peek_scroll = None;
                            dirty = true;
                        } else if !selectable.is_empty() {
                            // A selection that has gone away falls back to the
                            // first live row.
                            let position =
                                selected_position(&spans, &selectable, selected.as_deref())
                                    .unwrap_or(0);
                            let span = &spans[selectable[position]];
                            selected = Some(span.key.clone());
                            peek_open = true;
                            peek_scroll = None;
                            peek_index = 0;
                            // The pane shrinks the page; keep the selection
                            // in view within the new top half.
                            let page = (height as usize)
                                .saturating_sub(banner_rows)
                                .saturating_sub(peek_pane_rows(height))
                                .max(1);
                            scroll = scroll_to_reveal(scroll, page, span);
                            dirty = true;
                        }
                    }

                    if peek_open {
                        // While the pane is open, ↑↓ scroll the peeked
                        // session's transcript and ←→ switch which session
                        // the pane mirrors.
                        let interior = peek_pane_rows(height).saturating_sub(2);
                        let page_step = interior.saturating_sub(1).max(1) as isize;
                        let delta: Option<isize> = match key.code {
                            KeyCode::Up => Some(-1),
                            KeyCode::Down => Some(1),
                            KeyCode::PageUp => Some(-page_step),
                            KeyCode::PageDown => Some(page_step),
                            KeyCode::Char('k') if search.is_none() => Some(-1),
                            KeyCode::Char('j') if search.is_none() => Some(1),
                            _ => None,
                        };
                        if delta.is_some() || key.code == KeyCode::Home || key.code == KeyCode::End
                        {
                            let total = selected_peek(&spans, selected.as_deref(), peek_index)
                                .and_then(|target| transcripts.lines(&target.session_dir))
                                .map_or(0, <[String]>::len);
                            let next = match (key.code, delta) {
                                (KeyCode::Home, _) => (total > interior).then_some(0),
                                (KeyCode::End, _) => None,
                                (_, Some(delta)) => {
                                    step_peek_scroll(peek_scroll, delta, total, interior)
                                }
                                _ => peek_scroll,
                            };
                            if next != peek_scroll {
                                peek_scroll = next;
                                dirty = true;
                            }
                        }
                        let switch: Option<isize> = match key.code {
                            KeyCode::Left => Some(-1),
                            KeyCode::Right => Some(1),
                            _ => None,
                        };
                        if let Some(step) = switch {
                            // ←→ walk peekable sessions: within a PR-view
                            // row's sub-rows first, then into the neighbors.
                            if let Some((span, next_index)) = step_peek_target(
                                &spans,
                                &selectable,
                                selected.as_deref(),
                                peek_index,
                                step,
                            ) {
                                if selected.as_deref() != Some(span.key.as_str())
                                    || next_index != peek_index
                                {
                                    selected = Some(span.key.clone());
                                    peek_index = next_index;
                                    peek_scroll = None;
                                    scroll = scroll_to_reveal(scroll, page, span).min(max_scroll);
                                    dirty = true;
                                }
                            }
                        }
                    } else {
                        // ↑↓ (plus j/k outside search) step the selection
                        // through live sessions and the view follows; with
                        // nothing selectable they fall back to line scrolling.
                        let step: Option<isize> = match key.code {
                            KeyCode::Up => Some(-1),
                            KeyCode::Down => Some(1),
                            KeyCode::Char('k') if search.is_none() => Some(-1),
                            KeyCode::Char('j') if search.is_none() => Some(1),
                            _ => None,
                        };
                        if let Some(step) = step {
                            let position =
                                selected_position(&spans, &selectable, selected.as_deref());
                            if step < 0 && position == Some(0) {
                                // Up from the top row clears the highlight
                                // instead of pinning it there; Down starts a
                                // fresh selection.
                                selected = None;
                                peek_index = 0;
                                dirty = true;
                            } else if step < 0 && position.is_none() {
                                // Up with nothing selected scrolls the board;
                                // only Down enters the selection.
                                let target = scroll.saturating_sub(1);
                                if target != scroll {
                                    scroll = target;
                                    dirty = true;
                                }
                            } else {
                                match step_selection(&spans, &selectable, selected.as_deref(), step)
                                {
                                    Some(span) => {
                                        let moved = selected.as_deref() != Some(span.key.as_str());
                                        let target =
                                            scroll_to_reveal(scroll, page, span).min(max_scroll);
                                        if moved || target != scroll {
                                            selected = Some(span.key.clone());
                                            peek_index = 0;
                                            scroll = target;
                                            dirty = true;
                                        }
                                    }
                                    None => {
                                        // Nothing selectable: Down falls back
                                        // to line scrolling (Up scrolled
                                        // above).
                                        let target = (scroll + 1).min(max_scroll);
                                        if target != scroll {
                                            scroll = target;
                                            dirty = true;
                                        }
                                    }
                                }
                            }
                        }

                        let target = match key.code {
                            KeyCode::PageUp => scroll.saturating_sub(page.saturating_sub(2)),
                            KeyCode::PageDown => scroll + page.saturating_sub(2),
                            KeyCode::Home if search.is_none() => 0,
                            KeyCode::End if search.is_none() => max_scroll,
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
    use crate::terminal::escape::bg;

    fn board_pr(number: u64, repo: &str) -> SessionPr {
        let mut pr = SessionPr::test_stub(number, "o", repo);
        pr.state = "OPEN".to_string();
        pr.refreshed_at = 1_000;
        pr
    }

    fn make_primary(pr: &mut SessionPr) {
        pr.primary = true;
        pr.primary_source = "auto".to_string();
        pr.created_here = true;
    }

    fn render_frame_at(entries: &[BoardPr], detail: u8) -> String {
        render_frame_with_oldest(entries, detail, DEFAULT_OLDEST_VISIBLE_BUCKET)
    }

    fn render_frame_with_oldest(
        entries: &[BoardPr],
        detail: u8,
        oldest_visible_bucket: RecencyBucket,
    ) -> String {
        let rows: Vec<BoardRow> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        render(
            &rows,
            &[],
            &[],
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::new(detail, oldest_visible_bucket),
        )
        .lines
        .join("\n")
    }

    fn render_frame(entries: &[BoardPr]) -> String {
        render_frame_at(entries, DEFAULT_DETAIL)
    }

    #[test]
    fn header_shows_the_saved_view_controls() {
        let styled = render_frame(&[]);
        for (shortcut, label) in [
            ("s", "live"),
            ("p", "prs"),
            ("r", "compact"),
            ("a", "all ages"),
            ("↑↓", "select"),
            ("⏎", "peek"),
            ("/", "search"),
            ("+/-", "days"),
            ("q", "quit"),
        ] {
            assert!(styled.contains(&format!("{UNDERLINE}{shortcut}{RESET_UNDERLINE} {label}")));
        }

        let frame = crate::parsers::strip_ansi_for_debug(&styled);
        assert!(frame.contains("s live · primary done ≤ 1d · p prs · r compact · a all ages"));
        assert!(!frame.contains("r recap · a age · s live/all"));
        assert!(!frame.contains("e/c recap/compact"));
        assert!(!frame.contains("[/] age"));

        let recap = render_frame_with_oldest(&[], MAX_DETAIL, RecencyBucket::LastSixHours);
        assert!(recap.contains(&format!("{UNDERLINE}r{RESET_UNDERLINE} recap")));
        assert!(recap.contains(&format!("{UNDERLINE}a{RESET_UNDERLINE} age ≤ 6h")));

        let all_sessions = render(
            &[],
            &[],
            &[],
            160,
            DEFAULT_LINGER_DAYS,
            true,
            BoardView::default(),
        )
        .lines
        .join("\n");
        assert!(all_sessions.contains(&format!("{UNDERLINE}s{RESET_UNDERLINE} all sessions")));
    }

    fn now_ms() -> u64 {
        (now_secs() * 1000.0) as u64
    }

    fn snapshot(dir: &str, prs: Vec<SessionPr>) -> SessionSnapshot {
        SessionSnapshot {
            session_id: dir.to_string(),
            platform: PlatformKind::Claude,
            cwd: format!("/tmp/{dir}"),
            pr_scope: format!("session:{dir}"),
            dir_name: dir.to_string(),
            repo_owner: "o".to_string(),
            repo_name: dir.to_string(),
            session_dir: PathBuf::new(),
            last_updated: now_secs(),
            branch: String::new(),
            additions: 0,
            deletions: 0,
            title: String::new(),
            title_set_at: 0,
            slack_threads: Vec::new(),
            recap: None,
            state: SessionState::Ready,
            prompted_at: 0,
            completed_at: 0,
            prs,
        }
    }

    fn render_prs_frame(entries: &[BoardPr], detail: u8) -> RenderedBoard {
        let rows: Vec<BoardRow> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        render(
            &rows,
            &[],
            &[],
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::new(detail, DEFAULT_OLDEST_VISIBLE_BUCKET).with_mode(BoardMode::Prs),
        )
    }

    fn render_session_view_frame(
        shaped: &ShapedEntries,
        workspaces: &[WorkspaceEntry],
        detail: u8,
    ) -> RenderedBoard {
        let rows: Vec<BoardRow> = shaped
            .prs
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let session_rows: Vec<SessionBlockRow> = shaped
            .sessions
            .iter()
            .map(|entry| SessionBlockRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let workspace_rows: Vec<WorkspaceRow> = workspaces
            .iter()
            .map(|entry| WorkspaceRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        render(
            &rows,
            &session_rows,
            &workspace_rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::new(detail, DEFAULT_OLDEST_VISIBLE_BUCKET).with_mode(BoardMode::Sessions),
        )
    }

    fn test_session_ref(name: &str, prompted_at: u64, ended: bool) -> SessionRef {
        SessionRef {
            session_id: name.to_string(),
            platform: PlatformKind::Claude,
            pr_scope: format!("session:{name}"),
            dir_name: name.to_string(),
            session_dir: None,
            title: String::new(),
            title_set_at: 0,
            branch: String::new(),
            recap: None,
            state: SessionState::Ready,
            prompted_at,
            completed_at: 0,
            ended,
        }
    }

    #[test]
    fn pr_view_groups_sessions_under_one_pr_block() {
        let mut pr = board_pr(5, "portal");
        make_primary(&mut pr);
        pr.title = "Ship the fallback".to_string();
        pr.branch = "fallback".to_string();
        let twin = pr.clone();

        let mut one = snapshot("one", vec![pr]);
        one.repo_name = "portal".to_string();
        one.branch = "sam/fallback-worktree".to_string();
        one.recap = Some(RecapBrief {
            headline: "Ported the fallback".to_string(),
            bullets: vec!["Moved the mirror logic".to_string()],
            next_prompt_notes: Vec::new(),
            artifacts: Vec::new(),
            generated_at: 0,
            line_delta: crate::recap::TurnLineDelta::default(),
        });
        let mut two = snapshot("two", vec![twin]);
        two.repo_name = "portal".to_string();

        let merged = aggregate(
            &[one, two],
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );
        let entries = entries_for_mode(merged, BoardMode::Prs).prs;
        assert_eq!(
            entries.len(),
            1,
            "one block per PR, not one row per session"
        );
        assert_eq!(entries[0].sessions.len(), 2);

        let rendered = render_prs_frame(&entries, DEFAULT_DETAIL);
        assert_eq!(rendered.spans.len(), 1, "the block is one selectable span");
        assert_eq!(rendered.spans[0].key, "o/portal#5");
        let frame = crate::parsers::strip_ansi_for_debug(&rendered.lines.join("\n"));
        assert!(frame.contains("#5: Ship the fallback"), "{frame}");
        assert!(frame.contains("◆"), "session sub-rows sit under the PR");
        assert!(frame.contains("one") && frame.contains("two"));
        assert!(
            frame.contains("⎇  sam/fallback-worktree"),
            "the session's branch rides the sub-row: {frame}"
        );
        assert!(
            !frame.contains("Ported the fallback"),
            "the recap headline waits for the recap detail toggle"
        );
        assert!(
            !frame.contains("Moved the mirror logic"),
            "bullets wait for the recap detail toggle"
        );

        let detailed = render_prs_frame(&entries, MAX_DETAIL);
        let frame = crate::parsers::strip_ansi_for_debug(&detailed.lines.join("\n"));
        assert!(frame.contains("Ported the fallback"), "{frame}");
        assert!(frame.contains("Moved the mirror logic"), "{frame}");
    }

    /// The header row carries everything about the PR: the full title, the
    /// sessions' state and ages, then the diff and file count beside the
    /// GitHub status. Sub-rows only name the session and its headline.
    #[test]
    fn pr_view_header_holds_the_title_activity_and_stats() {
        let mut pr = board_pr(7, "portal");
        make_primary(&mut pr);
        pr.title = "A title well past the forty-four cells the session view allows".to_string();
        pr.additions = 49;
        pr.deletions = 5;
        pr.changed_files = 3;
        pr.mergeable = "MERGEABLE".to_string();
        let mut session = snapshot("one", vec![pr]);
        session.repo_name = "portal".to_string();
        session.prompted_at = now_secs() as u64 - 30 * 60;
        session.completed_at = now_secs() as u64 - 5 * 60;

        let merged = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let entries = entries_for_mode(merged, BoardMode::Prs).prs;
        let rendered = render_prs_frame(&entries, DEFAULT_DETAIL);
        // Cells link to GitHub, so the hyperlink wrappers go too.
        let links = regex::Regex::new(r"\x1b\]8;;[^\x07]*\x07").unwrap();
        let lines: Vec<String> = rendered
            .lines
            .iter()
            .map(|line| {
                links
                    .replace_all(&crate::parsers::strip_ansi_for_debug(line), "")
                    .into_owned()
            })
            .collect();
        let header = lines
            .iter()
            .find(|line| line.contains("#7: "))
            .expect("the PR header row renders");
        assert!(
            header.contains("A title well past the forty-four cells the session view allows"),
            "the title is not cut at the session view's cap: {header}"
        );
        assert!(header.contains("⟩ 30m") && header.contains("⋖ 5m"));
        let activity_at = header.find("⟩ 30m").unwrap();
        let diff_at = header.find("+49 -5").unwrap();
        let state_at = header.find("open").unwrap();
        assert!(
            activity_at < diff_at && diff_at < state_at,
            "activity, then diff, then status: {header}"
        );
        assert!(
            header.contains("+49 -5  open") && !header.contains('☰'),
            "only the diff sits beside the status: {header}"
        );
        assert!(
            header.contains("⋖ 5m  +49"),
            "the stats sit apart from the activity: {header}"
        );

        let sub_row = lines
            .iter()
            .find(|line| line.trim_start().starts_with('◆'))
            .expect("the session sub-row renders");
        assert!(
            !sub_row.contains('⟩') && !sub_row.contains('⋖'),
            "sub-rows leave the activity to the header: {sub_row}"
        );
    }

    /// A session badge or status cell that changed since the last frame
    /// wears a cooldown tint; untouched cells and first sightings stay plain.
    #[test]
    fn changed_badges_and_status_cells_wear_a_cooldown_tint() {
        let mut pr = board_pr(8, "portal");
        make_primary(&mut pr);
        pr.mergeable = "MERGEABLE".to_string();
        let mut session = snapshot("one", vec![pr.clone()]);
        session.repo_name = "portal".to_string();
        session.state = SessionState::Thinking;
        let before = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);

        let now_ms = now_ms();
        let mut cooldowns = Cooldowns::default();
        observe_cooldowns(&mut cooldowns, &before, &[], now_ms);
        let frame = |entries: &[BoardPr], cooldowns: &Cooldowns, at: u64| {
            let rows: Vec<BoardRow> = entries
                .iter()
                .map(|entry| BoardRow {
                    entry,
                    preview_lines: Vec::new(),
                })
                .collect();
            render_at(
                &rows,
                &[],
                &[],
                160,
                DEFAULT_LINGER_DAYS,
                false,
                BoardView::new(DEFAULT_DETAIL, DEFAULT_OLDEST_VISIBLE_BUCKET),
                RenderState {
                    now_ms: at,
                    loading: false,
                    cooldowns,
                },
            )
            .lines
            .join("\n")
        };
        let hot = escape::bg_rgb(crate::ui::cooldown::tint_for_age(0).unwrap().bg);
        let first = frame(&before, &cooldowns, now_ms);
        assert!(!first.contains(&hot), "the first frame lights nothing up");
        let backgrounds = |frame: &str| frame.matches("\x1b[48;").count();
        let plain_backgrounds = backgrounds(&first);

        // The session finishes and the PR's merge state flips.
        pr.merge_state_status = "BEHIND".to_string();
        let mut session = snapshot("one", vec![pr]);
        session.repo_name = "portal".to_string();
        session.state = SessionState::Complete;
        let after = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        observe_cooldowns(&mut cooldowns, &after, &[], now_ms + 2_000);
        assert!(cooldowns.active(now_ms + 2_000));

        let styled = frame(&after, &cooldowns, now_ms + 2_000);
        assert!(
            styled.contains(&format!("{hot}{} ✓ ", fg(color::PURPLE))),
            "the badge glows on the hottest tint, keeping its own color: {styled:?}"
        );
        assert!(
            styled.contains(&format!("{hot}{}behind", fg(color::YELLOW))),
            "the merge cell glows too, keeping its own color: {styled:?}"
        );
        assert!(
            !styled.contains(&format!("{hot}{}open", fg(color::LIGHT_GREEN))),
            "the unchanged state cell stays plain"
        );

        let half = crate::ui::cooldown::COOLDOWN_MS / 2;
        let later = frame(&after, &cooldowns, now_ms + 2_000 + half);
        assert!(
            !later.contains(&hot),
            "halfway through, the tint has moved along"
        );
        let halfway = crate::ui::cooldown::tint_for_age(half).unwrap();
        assert!(later.contains(&escape::bg_rgb(halfway.bg)));
        let cold = frame(
            &after,
            &cooldowns,
            now_ms + 2_000 + crate::ui::cooldown::COOLDOWN_MS,
        );
        assert_eq!(
            backgrounds(&cold),
            plain_backgrounds,
            "once the cooldown runs out only the board's own backgrounds remain"
        );
    }

    #[test]
    fn watched_prs_join_the_board_without_sessions() {
        let mut tracked = board_pr(5, "portal");
        make_primary(&mut tracked);
        let mut session = snapshot("one", vec![tracked]);
        session.repo_name = "portal".to_string();
        let mut entries = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);

        let mut solo = SessionPr::watched_stub("o", "elsewhere", 9);
        solo.state = "OPEN".to_string();
        solo.refreshed_at = 1_000;
        solo.title = "Watched from afar".to_string();
        let watched = HashMap::from([
            (
                watch_key("o", "portal", 5),
                SessionPr::watched_stub("o", "portal", 5),
            ),
            (watch_key("o", "elsewhere", 9), solo),
        ]);
        merge_watched_entries(
            &mut entries,
            &watched,
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );

        assert_eq!(entries.len(), 2);
        let tracked = entries.iter().find(|e| e.pr.repo == "portal").unwrap();
        assert!(tracked.pr.watched, "session-tracked copies gain the flag");
        assert!(tracked.pr.primary, "the session's classification survives");
        let solo = entries.iter().find(|e| e.pr.repo == "elsewhere").unwrap();
        assert!(solo.pr.watched);
        assert!(solo.sessions.is_empty() && !solo.stale);

        // PR view keeps watched PRs even though nothing marks them primary,
        // and the watch glyph shows on the rendered block.
        let prs = entries_for_mode(entries, BoardMode::Prs).prs;
        assert_eq!(prs.len(), 2);
        let frame = render_prs_frame(&prs, DEFAULT_DETAIL).lines.join("\n");
        assert!(frame.contains('◉'), "the watch glyph renders: {frame}");
        assert!(
            crate::parsers::strip_ansi_for_debug(&frame).contains("#9: Watched from afar"),
            "{frame}"
        );
        assert!(
            frame.contains("disposition=unwatched"),
            "the glyph links to the unwatch action: {frame}"
        );

        // A dismissed disposition hides the watch everywhere.
        let mut entries = Vec::new();
        let overrides = ScopedOverrides::from(HashMap::from([(
            watch_key("o", "elsewhere", 9),
            PrDisposition::Dismissed,
        )]));
        merge_watched_entries(&mut entries, &watched, &overrides, DEFAULT_LINGER_DAYS);
        assert!(entries.iter().all(|e| e.pr.repo != "elsewhere"));
    }

    #[test]
    fn watched_prs_use_the_primary_linger_rules() {
        let now_ms = now_ms();
        let mut merged = SessionPr::watched_stub("o", "r", 7);
        merged.refreshed_at = 1_000;
        merged.state = "MERGED".to_string();
        merged.closed_at = now_ms - 3600 * 1000;
        assert!(
            visible_pr(&merged, 1, now_ms),
            "a freshly merged watch lingers"
        );
        merged.closed_at = now_ms - 3 * 24 * 3600 * 1000;
        merged.updated_at = merged.closed_at;
        assert!(
            !visible_pr(&merged, 1, now_ms),
            "an aged-out watch drops off"
        );
        assert!(
            visible_pr(&SessionPr::watched_stub("o", "r", 8), 1, now_ms),
            "a never-enriched watch stays while fetching"
        );
    }

    #[test]
    fn pr_view_lists_only_primary_prs() {
        let mut primary = board_pr(1, "portal");
        make_primary(&mut primary);
        let secondary = board_pr(2, "portal");
        let mut session = snapshot("one", vec![primary, secondary]);
        session.repo_name = "portal".to_string();

        let merged = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        assert_eq!(merged.len(), 2, "session view shows the open secondary");

        let prs = entries_for_mode(merged, BoardMode::Prs).prs;
        assert_eq!(prs.len(), 1, "PR view watches primaries only");
        assert_eq!(prs[0].pr.number, 1);
    }

    #[test]
    fn recency_follows_prompts_and_falls_back_to_pr_events() {
        let now_ms = now_ms();
        let now = now_ms / 1000;
        let mut pr = board_pr(9, "portal");
        make_primary(&mut pr);
        pr.state = "MERGED".to_string();
        pr.closed_at = now_ms - 60_000;
        pr.updated_at = now_ms - 60_000;
        let mut entry = BoardPr {
            pr,
            sessions: vec![test_session_ref("old", now - 30 * 3600, false)],
            slack_threads: Vec::new(),
            stale: false,
        };

        assert_eq!(
            entry_bucket(&entry, now),
            RecencyBucket::Older,
            "a stale prompt outranks a fresh merge: the board follows the user"
        );
        entry.sessions[0].completed_at = now - 60;
        assert_eq!(
            entry_bucket(&entry, now),
            RecencyBucket::Older,
            "a fresh completion never moves a row"
        );
        entry.sessions.clear();
        assert_eq!(
            entry_bucket(&entry, now),
            RecencyBucket::LastHour,
            "with no prompt to follow, the PR's own events place it"
        );
    }

    #[test]
    fn pr_view_sorts_ended_sessions_below_live_ones_and_dims_them() {
        let now = (now_secs()) as u64;
        let mut pr = board_pr(3, "portal");
        make_primary(&mut pr);
        let entry = BoardPr {
            pr,
            sessions: vec![
                test_session_ref("ended-fresh", now - 60, true),
                test_session_ref("live-old", now - 7200, false),
            ],
            slack_threads: Vec::new(),
            stale: false,
        };

        let shaped = entries_for_mode(vec![entry], BoardMode::Prs).prs;
        let names: Vec<&str> = shaped[0]
            .sessions
            .iter()
            .map(|session| session.dir_name.as_str())
            .collect();
        assert_eq!(
            names,
            ["live-old", "ended-fresh"],
            "live sessions come first even when older"
        );

        let widths = PrColumnWidths::from_pr_refs(&[&shaped[0].pr], 120);
        let ended_row = pr_view_session_row(&shaped[0].sessions[1], 120, 12, &widths);
        assert!(
            ended_row.trim_start().starts_with(&fg(color::DARK_GRAY)),
            "ended sub-rows dim: {ended_row:?}"
        );
    }

    #[test]
    fn peek_stepping_walks_a_rows_sessions_then_crosses_rows() {
        let target = |name: &str| PeekTarget {
            title: name.to_string(),
            dir_name: name.to_string(),
            session_dir: PathBuf::from(name),
        };
        let spans = vec![
            RowSpan {
                key: "a".to_string(),
                start: 0,
                end: 1,
                peeks: vec![target("a1"), target("a2")],
            },
            RowSpan {
                key: "b".to_string(),
                start: 1,
                end: 2,
                peeks: vec![target("b1")],
            },
        ];
        let selectable = selectable_positions(&spans);

        let step = |selected: Option<&str>, peek_index, step| {
            step_peek_target(&spans, &selectable, selected, peek_index, step)
                .map(|(span, index)| (span.key.as_str(), index))
        };
        assert_eq!(step(None, 0, 1), Some(("a", 0)), "enters at the top");
        assert_eq!(step(Some("a"), 0, 1), Some(("a", 1)), "walks within the PR");
        assert_eq!(step(Some("a"), 1, 1), Some(("b", 0)), "then crosses rows");
        assert_eq!(step(Some("b"), 0, -1), Some(("a", 1)), "and back");
        assert_eq!(step(Some("b"), 0, 1), Some(("b", 0)), "clamped at the end");
        assert_eq!(
            selected_peek(&spans, Some("a"), 5).map(|t| t.dir_name.as_str()),
            Some("a1"),
            "an out-of-range index falls back to the first session"
        );
    }

    #[test]
    fn session_view_groups_prs_under_each_session() {
        let mut a = board_pr(5, "portal");
        make_primary(&mut a);
        a.mentions = 10;
        a.user_mentions = 1;
        a.refreshed_at = 2_000;
        a.additions = 42;
        a.slack_comment_urls =
            vec!["https://t.slack.com/archives/C2/p1723500002000000".to_string()];
        let mut b = board_pr(5, "portal");
        make_primary(&mut b);
        b.mentions = 3;
        b.refreshed_at = 1_000;
        b.slack_origin_url = "https://t.slack.com/archives/C0/p1723500000000000".to_string();
        b.slack_comment_urls =
            vec!["https://t.slack.com/archives/C1/p1723500001000000".to_string()];

        let mut one = snapshot("one", vec![a]);
        one.repo_name = "portal".to_string();
        one.slack_threads =
            crate::slack::extract_threads("https://t.slack.com/archives/C2/p1723500002000000");
        let mut two = snapshot("two", vec![b]);
        two.repo_name = "portal".to_string();
        two.slack_threads = crate::slack::extract_threads(
            "https://t.slack.com/archives/C0/p1723500000000000 https://t.slack.com/archives/C1/p1723500001000000 https://t.slack.com/archives/C4/p1723500003000000",
        );
        let shaped = entries_for_mode(
            aggregate(
                &[two, one],
                &ScopedOverrides::default(),
                DEFAULT_LINGER_DAYS,
            ),
            BoardMode::Sessions,
        );
        assert!(shaped.prs.is_empty(), "every PR is under a session block");
        assert_eq!(shaped.sessions.len(), 2, "one block per session");
        let mut session_dirs: Vec<&str> = shaped
            .sessions
            .iter()
            .map(|block| block.session.dir_name.as_str())
            .collect();
        session_dirs.sort_unstable();
        assert_eq!(session_dirs, ["one", "two"]);
        for block in &shaped.sessions {
            assert_eq!(block.prs.len(), 1, "each block lists the session's PRs");
            let entry = &block.prs[0];
            assert_eq!(entry.pr.mentions, 13, "mentions sum across sessions");
            assert_eq!(entry.pr.additions, 42, "newest gh stats win");
            assert_eq!(
                entry.pr.slack_origin_url, "https://t.slack.com/archives/C0/p1723500000000000",
                "a newer snapshot cannot erase the original thread"
            );
            assert_eq!(entry.pr.slack_comment_urls.len(), 2);
            assert_eq!(
                entry
                    .slack_threads
                    .iter()
                    .map(|thread| thread.url.as_str())
                    .collect::<Vec<_>>(),
                [
                    "https://t.slack.com/archives/C0/p1723500000000000",
                    "https://t.slack.com/archives/C1/p1723500001000000",
                    "https://t.slack.com/archives/C2/p1723500002000000",
                    "https://t.slack.com/archives/C4/p1723500003000000",
                ],
                "the original thread stays first and session-only links remain"
            );
        }
    }

    #[test]
    fn selection_band_survives_mid_line_resets_and_fills_the_width() {
        let line = format!("{}text{RESET} tail", fg(color::GREEN));
        let banded = selection_band_line(&line, 20);
        let band = escape::bg(color::BG_SELECTED);
        assert!(banded.starts_with(&band));
        assert!(
            banded.contains(&format!("{RESET}{band}")),
            "the band background is re-asserted after a mid-line reset"
        );
        assert!(banded.ends_with(RESET));
        assert_eq!(
            crate::ui::utils::strip_ansi_len(&banded),
            20,
            "the band pads to the full width"
        );
    }

    #[test]
    fn scroll_reveals_the_selected_row_with_minimal_movement() {
        let span = |start, end| RowSpan {
            key: "k".to_string(),
            start,
            end,
            peeks: Vec::new(),
        };
        assert_eq!(scroll_to_reveal(10, 5, &span(3, 5)), 3, "scrolls up");
        assert_eq!(
            scroll_to_reveal(0, 5, &span(8, 10)),
            5,
            "scrolls down just enough"
        );
        assert_eq!(
            scroll_to_reveal(2, 5, &span(3, 5)),
            2,
            "visible rows stay put"
        );
        assert_eq!(
            scroll_to_reveal(0, 3, &span(4, 10)),
            4,
            "a tall block aligns its top"
        );
    }

    #[test]
    fn only_rows_with_a_live_screen_are_selectable() {
        let capture = tempfile::tempdir().unwrap();
        std::fs::write(capture.path().join("screen.txt"), "hello\n").unwrap();

        let mut live_pr = board_pr(7, "portal");
        make_primary(&mut live_pr);
        let mut live = snapshot("live", vec![live_pr]);
        live.repo_name = "portal".to_string();
        live.session_dir = capture.path().to_path_buf();

        let mut captureless_pr = board_pr(8, "portal");
        make_primary(&mut captureless_pr);
        let mut captureless = snapshot("captureless", vec![captureless_pr]);
        captureless.repo_name = "portal".to_string();
        captureless.session_dir = capture.path().join("missing");

        let entries = aggregate(
            &[live, captureless],
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );
        let rows: Vec<BoardRow> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let rendered = render(
            &rows,
            &[],
            &[],
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
        );
        assert_eq!(rendered.spans.len(), 2, "every row gets a span");

        let selectable = selectable_positions(&rendered.spans);
        assert_eq!(
            selectable.len(),
            1,
            "only the captured session is selectable"
        );
        let span = &rendered.spans[selectable[0]];
        assert_eq!(span.key, "o/portal#7");
        assert_eq!(span.peeks[0].session_dir, capture.path());
        let block =
            crate::parsers::strip_ansi_for_debug(&rendered.lines[span.start..span.end].join("\n"));
        assert!(
            block.contains("7:"),
            "the span brackets its own row: {block}"
        );
    }

    fn peek_spans(capture: &tempfile::TempDir) -> Vec<RowSpan> {
        vec![RowSpan {
            key: "k".to_string(),
            start: 0,
            end: 1,
            peeks: vec![PeekTarget {
                title: "Fix the flaky test".to_string(),
                dir_name: "portal".to_string(),
                session_dir: capture.path().to_path_buf(),
            }],
        }]
    }

    #[test]
    fn peek_pane_shows_the_screen_tail_under_a_titled_header() {
        let capture = tempfile::tempdir().unwrap();
        std::fs::write(capture.path().join("screen.txt"), "one\ntwo\nthree\nfour\n").unwrap();
        let spans = peek_spans(&capture);
        let mut transcripts = TranscriptCache::default();

        let pane = build_peek_pane(&spans, Some("k"), 0, 80, 4, None, &mut transcripts);
        assert_eq!(pane.len(), 4, "borders plus the screen rows that fit");
        assert!(pane[0].contains('┏') && pane[0].contains('┓'));
        assert_eq!(
            crate::ui::utils::strip_ansi_len(&pane[0]),
            80,
            "the top border spans the full width"
        );
        assert!(pane[0].contains("Fix the flaky test"));
        assert!(pane[0].contains("portal"));
        for row in &pane[1..3] {
            assert!(row.contains('┃'), "screen rows are boxed: {row}");
        }
        assert!(pane[1].contains("three"));
        assert!(pane[2].contains("four"));
        assert!(pane[3].contains('┗') && pane[3].contains('┛'));

        let missing = build_peek_pane(&spans, Some("unknown"), 0, 80, 4, None, &mut transcripts);
        assert!(
            missing[1].contains("no live screen"),
            "a vanished session reads as such instead of a stale screen"
        );
        assert_eq!(missing.len(), 4, "the box keeps its size without content");
    }

    #[test]
    fn scrolled_peek_pane_windows_the_transcript_and_says_how_far_back() {
        let capture = tempfile::tempdir().unwrap();
        std::fs::write(capture.path().join("screen.txt"), "live\n").unwrap();
        let transcript: String = (1..=10).map(|n| format!("line {n}\n")).collect();
        std::fs::write(capture.path().join("scrollback.log"), transcript).unwrap();
        let spans = peek_spans(&capture);
        let mut transcripts = TranscriptCache::default();

        let pane = build_peek_pane(&spans, Some("k"), 0, 80, 4, Some(3), &mut transcripts);
        assert!(pane[1].contains("line 4"), "window starts at the anchor");
        assert!(pane[2].contains("line 5"));
        assert!(
            pane[0].contains("5 back"),
            "the header says how far back the view is: {}",
            pane[0]
        );
        assert!(pane[0].contains('≡'));
    }

    #[test]
    fn peek_scroll_anchors_climbs_and_returns_to_live() {
        // 10 transcript lines, a 4-line interior: the tail anchors at 6.
        assert_eq!(
            step_peek_scroll(None, -1, 10, 4),
            Some(5),
            "first ↑ anchors and moves"
        );
        assert_eq!(step_peek_scroll(Some(5), -1, 10, 4), Some(4));
        assert_eq!(
            step_peek_scroll(Some(0), -1, 10, 4),
            Some(0),
            "clamped at the top"
        );
        assert_eq!(step_peek_scroll(Some(4), 1, 10, 4), Some(5));
        assert_eq!(
            step_peek_scroll(Some(5), 1, 10, 4),
            None,
            "reaching the tail returns to the live screen"
        );
        assert_eq!(
            step_peek_scroll(None, 1, 10, 4),
            None,
            "↓ while live stays live"
        );
        assert_eq!(
            step_peek_scroll(None, -1, 3, 4),
            None,
            "a transcript that fits the window has nothing to scroll into"
        );
    }

    /// The all-sessions view builds rows from the cloud record, which has no
    /// local paths; live sessions get their mirror re-attached so they stay
    /// selectable, and sessions with no live mirror stay unselectable.
    #[test]
    fn live_mirrors_reattach_to_cloud_rows() {
        let mut pr = board_pr(9, "portal");
        make_primary(&mut pr);
        let mut snapshot = snapshot("cloudid", vec![pr]);
        snapshot.repo_name = "portal".to_string();
        let mut entries = aggregate(
            &[snapshot],
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );
        entries[0].sessions[0].session_dir = None;

        let mirrors: HashMap<String, PathBuf> = [(
            "cloudid".to_string(),
            PathBuf::from("/tmp/crabigator-cloudid"),
        )]
        .into();
        attach_live_mirrors(&mut entries, &mut [], &mirrors);
        assert_eq!(
            entries[0].sessions[0].session_dir.as_deref(),
            Some(Path::new("/tmp/crabigator-cloudid"))
        );

        attach_live_mirrors(&mut entries, &mut [], &HashMap::new());
        assert_eq!(
            entries[0].sessions[0].session_dir, None,
            "a session with no live mirror is cleared, not left stale"
        );
    }

    #[test]
    fn loading_frame_is_truthful_before_sessions_arrive() {
        let loading = render_at(
            &[],
            &[],
            &[],
            120,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
            RenderState {
                now_ms: now_ms(),
                loading: true,
                cooldowns: &Cooldowns::default(),
            },
        )
        .lines
        .join("\n");
        let plain = crate::parsers::strip_ansi_for_debug(&loading);
        assert!(plain.contains("Loading live sessions… 0 found"));
        assert!(!plain.contains("No live sessions"));
    }

    #[test]
    fn active_sessions_without_visible_prs_stay_on_the_board() {
        let mut first = snapshot("crabigator", Vec::new());
        first.session_id = "one".to_string();
        first.repo_owner = "samuelclay".to_string();
        first.repo_name = "crabigator".to_string();
        first.branch = "sam/pr-board-session-rows".to_string();
        first.title = "First active session".to_string();
        first.additions = 10;
        first.deletions = 2;
        let mut second = snapshot("crabigator", Vec::new());
        second.session_id = "two".to_string();
        second.repo_owner = "SamuelClay".to_string();
        second.repo_name = "crabigator".to_string();
        second.branch = first.branch.clone();
        second.title = "Second active session".to_string();

        let snapshots = vec![first, second];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        assert_eq!(workspaces.len(), 2);

        let rows: Vec<WorkspaceRow<'_>> = workspaces
            .iter()
            .map(|entry| WorkspaceRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let frame = render(
            &[],
            &[],
            &rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
        )
        .lines
        .join("\n");
        let plain = crate::parsers::strip_ansi_for_debug(&frame);
        assert!(plain.lines().any(|line| line == "samuelclay/crabigator"));
        assert!(frame.contains(&format!("{}samuelclay/crabigator", fg(color::YELLOW))));
        assert!(frame.contains("sam/pr-board-session-rows"));
        assert!(frame.contains("2 sessions"));
        assert!(!frame.contains("no tracked PR"));
        assert!(frame.contains("First active session"));
        assert!(frame.contains("Second active session"));
        assert!(plain.contains("+10 -2"));
        assert!(frame.contains(&format!(
            "{}{}+10",
            crate::terminal::escape::BOLD,
            fg(color::GREEN)
        )));
        assert!(frame.contains(&format!("{}-2", fg(color::RED))));
        assert!(plain.contains("+10 -2"));
        assert!(!plain.contains('☰'));
    }

    #[test]
    fn all_mode_keeps_live_sessions_when_cloud_has_no_session_rows() {
        let mut session = snapshot("crabigator", Vec::new());
        session.session_id = "live-session".to_string();
        session.additions = 12;
        session.deletions = 3;
        let live = local_workspaces(&[session], &[]);
        let mut cloud = Vec::new();

        merge_live_workspaces(&mut cloud, live, &[]);

        assert_eq!(cloud.len(), 1);
        assert_eq!(cloud[0].session.session_id, "live-session");
        assert_eq!(cloud[0].additions, 12);
        assert_eq!(cloud[0].deletions, 3);
    }

    #[test]
    fn cloud_titles_do_not_borrow_recap_timestamps() {
        let cloud: crate::cloud::CloudBoard = serde_json::from_value(serde_json::json!({
            "sessions": [{
                "session_id": "cloud-session",
                "dir_name": "crabigator",
                "active": true,
                "title": "A durable title",
                "recap": {
                    "headline": "A later recap",
                    "next_prompt_notes": ["Continue from the durable session"],
                    "generated_at": 1_234_567
                }
            }]
        }))
        .unwrap();

        let (_, workspaces) = cloud_entries_to_board(cloud);
        assert_eq!(workspaces[0].session.title_set_at, 0);
        assert_eq!(
            workspaces[0]
                .session
                .recap
                .as_ref()
                .unwrap()
                .next_prompt_notes,
            ["Continue from the durable session"]
        );
    }

    #[test]
    fn all_mode_pr_represents_its_live_session_after_a_branch_change() {
        let mut pr = board_pr(7, "crabigator");
        make_primary(&mut pr);
        pr.branch = "old-branch".to_string();
        let mut durable = snapshot("crabigator", vec![pr]);
        durable.session_id = "cloud-session".to_string();
        let entries = aggregate(&[durable], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);

        let mut current = snapshot("crabigator", Vec::new());
        current.session_id = "cloud-session".to_string();
        current.branch = "new-branch".to_string();
        let live = local_workspaces(&[current], &[]);
        let mut workspaces = Vec::new();

        merge_live_workspaces(&mut workspaces, live, &entries);

        assert!(workspaces.is_empty());
    }

    #[test]
    fn repository_rows_share_columns_and_follow_prompt_activity() {
        let now = now_secs() as u64;
        let mut crab_pr = board_pr(7, "crabigator");
        make_primary(&mut crab_pr);
        crab_pr.owner = "samuelclay".to_string();
        crab_pr.branch = "with-pr".to_string();
        let mut pr_session = snapshot("crabigator", vec![crab_pr]);
        pr_session.platform = PlatformKind::Codex;
        pr_session.session_id = "with-pr".to_string();
        pr_session.repo_owner = "samuelclay".to_string();
        pr_session.repo_name = "crabigator".to_string();
        pr_session.branch = "with-pr".to_string();
        pr_session.prompted_at = now - 120;

        let mut no_pr_session = snapshot("crabigator", Vec::new());
        no_pr_session.session_id = "without-pr".to_string();
        no_pr_session.repo_owner = "samuelclay".to_string();
        no_pr_session.repo_name = "crabigator".to_string();
        no_pr_session.branch = "main".to_string();
        no_pr_session.prompted_at = now - 30;
        no_pr_session.title = "Newest completed session".to_string();

        let mut portal_pr = board_pr(9, "developer-portal");
        make_primary(&mut portal_pr);
        portal_pr.branch = "a-much-longer-branch-name".to_string();
        let mut portal_session = snapshot("developer-portal", vec![portal_pr]);
        portal_session.prompted_at = now - 300;

        let snapshots = vec![pr_session, no_pr_session, portal_session];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        let shaped = entries_for_mode(entries, BoardMode::Sessions);
        let frame = crate::parsers::strip_ansi_for_debug(
            &render_session_view_frame(&shaped, &workspaces, DEFAULT_DETAIL)
                .lines
                .join("\n"),
        );

        assert_eq!(frame.matches("samuelclay/crabigator").count(), 1);
        let no_pr_offset = frame.find("◇ ᛝ  Newest completed session").unwrap();
        let pr_offset = frame.find("#7:").unwrap();
        assert!(no_pr_offset < pr_offset, "newer prompt sorts first");

        let no_pr_row = frame
            .lines()
            .find(|line| line.contains("◇ ᛝ  Newest completed session"))
            .unwrap();
        let crab_header_row = frame
            .lines()
            .find(|line| line.contains("◆ ⟁  crabigator"))
            .expect("the PR session's block leads with the session");
        let portal_header_row = frame
            .lines()
            .find(|line| line.contains("◆ ᛝ  developer-portal"))
            .unwrap();
        let crab_pr_row = frame.lines().find(|line| line.contains("7:")).unwrap();
        let portal_pr_row = frame.lines().find(|line| line.contains("9:")).unwrap();
        let column = |line: &str, text: &str| {
            crate::ui::utils::strip_ansi_len(&line[..line.find(text).unwrap()])
        };
        assert_eq!(column(no_pr_row, "⟩"), column(crab_header_row, "⟩"));
        assert_eq!(column(crab_header_row, "⟩"), column(portal_header_row, "⟩"));
        assert_eq!(
            column(crab_pr_row, "⎇  with-pr"),
            column(portal_pr_row, "⎇  a-much-longer-branch-name")
        );
        assert!(!no_pr_row.contains("no tracked PR"));
        assert!(
            !crab_header_row.contains("#7:"),
            "the session leads its block; the PR sits beneath: {crab_header_row}"
        );
        assert!(crab_pr_row.contains("#7: crabigator"));
        assert!(crab_pr_row.contains("⎇  with-pr"));
        assert!(portal_pr_row.contains("#9: developer-portal"));
        assert!(portal_pr_row.contains("⎇  a-much-longer-branch-name"));
    }

    #[test]
    fn blank_lines_separate_repositories_in_compact_and_recap_views() {
        let now = now_secs() as u64;
        let mut pr = board_pr(2, "crabigator");
        make_primary(&mut pr);
        let mut pr_session = snapshot("crabigator", vec![pr]);
        pr_session.title = "PR session".to_string();
        pr_session.completed_at = now - 60;
        pr_session.recap = Some(RecapBrief {
            headline: "Finished the PR work".to_string(),
            bullets: Vec::new(),
            next_prompt_notes: Vec::new(),
            artifacts: Vec::new(),
            generated_at: now * 1000,
            line_delta: crate::recap::TurnLineDelta::default(),
        });

        let mut peer_session = snapshot("crabigator", Vec::new());
        peer_session.session_id = "peer-session".to_string();
        peer_session.title = "Peer session".to_string();
        peer_session.completed_at = now - 120;

        let mut other_repo = snapshot("portal", Vec::new());
        other_repo.title = "Portal session".to_string();
        other_repo.completed_at = now - 300;

        let snapshots = vec![pr_session, peer_session, other_repo];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        let shaped = entries_for_mode(entries, BoardMode::Sessions);

        for detail in [0, MAX_DETAIL] {
            let lines: Vec<String> = render_session_view_frame(&shaped, &workspaces, detail)
                .lines
                .into_iter()
                .map(|line| crate::parsers::strip_ansi_for_debug(&line))
                .collect();
            let crabigator = lines
                .iter()
                .position(|line| line.contains("o/crabigator"))
                .unwrap();
            let portal = lines
                .iter()
                .position(|line| line.contains("o/portal"))
                .unwrap();
            let header_row = lines
                .iter()
                .position(|line| line.contains("◆ ᛝ  PR session"))
                .unwrap();
            let pr_row = lines
                .iter()
                .position(|line| line.contains("2: crabigator"))
                .unwrap();
            let peer_row = lines
                .iter()
                .position(|line| line.contains("◇ ᛝ  Peer session"))
                .unwrap();
            let blank_lines: Vec<usize> = lines[crabigator..]
                .iter()
                .enumerate()
                .filter_map(|(offset, line)| line.is_empty().then_some(crabigator + offset))
                .collect();

            assert!(crabigator < header_row && header_row < pr_row);
            assert!(pr_row < peer_row && peer_row < portal);
            assert_eq!(blank_lines, vec![portal - 1]);
        }
    }

    #[test]
    fn recency_buckets_repeat_repositories_and_skip_empty_ranges() {
        let now = now_secs() as u64;
        let cases = [
            (1, "portal", 30 * 60),
            (2, "crabigator", 2 * 60 * 60),
            (3, "portal", 4 * 60 * 60),
            (4, "crabigator", 2 * 24 * 60 * 60),
        ];
        let snapshots: Vec<SessionSnapshot> = cases
            .into_iter()
            .map(|(number, repo, age)| {
                let mut pr = board_pr(number, repo);
                make_primary(&mut pr);
                let mut session = snapshot(repo, vec![pr]);
                session.prompted_at = now - age;
                session
            })
            .collect();
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let rendered = render_frame(&entries);
        for (label, bucket_color, text_color) in [
            ("● Last hour", RECENCY_1H, color::BLACK),
            ("● 1–3 hours", RECENCY_3H, color::BLACK),
            ("● 3–6 hours", RECENCY_6H, color::BLACK),
            ("● Older", color::DARK_GRAY, color::WHITE),
        ] {
            let header = rendered.lines().find(|line| line.contains(label)).unwrap();
            assert!(header.starts_with(&format!("{}{}", escape::bg(bucket_color), fg(text_color))));
            assert_eq!(crate::ui::utils::strip_ansi_len(header), 160);
        }
        let frame = crate::parsers::strip_ansi_for_debug(&rendered);

        let last_hour = frame.find("● Last hour").unwrap();
        let first_portal = frame[last_hour..].find("o/portal").unwrap() + last_hour;
        let three_hours = frame.find("● 1–3 hours").unwrap();
        let first_crabigator = frame[three_hours..].find("o/crabigator").unwrap() + three_hours;
        let six_hours = frame.find("● 3–6 hours").unwrap();
        let second_portal = frame[six_hours..].find("o/portal").unwrap() + six_hours;
        let older = frame.find("● Older").unwrap();
        let second_crabigator = frame[older..].find("o/crabigator").unwrap() + older;

        assert!(last_hour < first_portal && first_portal < three_hours);
        assert!(three_hours < first_crabigator && first_crabigator < six_hours);
        assert!(six_hours < second_portal && second_portal < older);
        assert!(older < second_crabigator);
        assert_eq!(frame.lines().filter(|line| *line == "o/portal").count(), 2);
        assert_eq!(
            frame.lines().filter(|line| *line == "o/crabigator").count(),
            2
        );
        assert!(!frame.contains("● 6–9 hours"));
        assert!(!frame.contains("● 9–12 hours"));
        assert!(!frame.contains("● 12–24 hours"));
    }

    #[test]
    fn recency_headers_choose_contrasting_text() {
        for (bucket, text_color) in [
            (RecencyBucket::LastHour, color::BLACK),
            (RecencyBucket::LastThreeHours, color::BLACK),
            (RecencyBucket::LastSixHours, color::BLACK),
            (RecencyBucket::LastNineHours, color::BLACK),
            (RecencyBucket::LastTwelveHours, color::WHITE),
            (RecencyBucket::LastDay, color::WHITE),
            (RecencyBucket::Older, color::WHITE),
        ] {
            assert_eq!(bucket.heading_text_color(), text_color);
        }
    }

    #[test]
    fn recap_and_age_controls_cycle_without_changing_each_other() {
        assert_eq!(
            BoardView::new(u8::MAX, DEFAULT_OLDEST_VISIBLE_BUCKET).detail,
            MAX_DETAIL,
            "invalid saved detail opens at the highest supported level"
        );
        let mut view = BoardView::new(MAX_DETAIL, RecencyBucket::LastNineHours);
        view.toggle_recap();
        assert_eq!(
            (view.detail, view.oldest_visible_bucket),
            (DEFAULT_DETAIL, RecencyBucket::LastNineHours)
        );
        view.toggle_recap();
        assert_eq!(
            (view.detail, view.oldest_visible_bucket),
            (MAX_DETAIL, RecencyBucket::LastNineHours)
        );

        for expected in [
            RecencyBucket::LastTwelveHours,
            RecencyBucket::LastDay,
            RecencyBucket::Older,
            RecencyBucket::LastHour,
            RecencyBucket::LastThreeHours,
            RecencyBucket::LastSixHours,
            RecencyBucket::LastNineHours,
        ] {
            view.cycle_age();
            assert_eq!(view.oldest_visible_bucket, expected);
            assert_eq!(view.detail, MAX_DETAIL);
        }
    }

    #[test]
    fn recency_preferences_round_trip_every_cutoff() {
        for (hours, bucket) in [
            (Some(1), RecencyBucket::LastHour),
            (Some(3), RecencyBucket::LastThreeHours),
            (Some(6), RecencyBucket::LastSixHours),
            (Some(9), RecencyBucket::LastNineHours),
            (Some(12), RecencyBucket::LastTwelveHours),
            (Some(24), RecencyBucket::LastDay),
            (None, RecencyBucket::Older),
        ] {
            assert_eq!(RecencyBucket::from_max_age_hours(hours), bucket);
            assert_eq!(bucket.max_age_hours(), hours);
        }
        assert_eq!(
            RecencyBucket::from_max_age_hours(Some(25)),
            RecencyBucket::Older,
            "invalid manual values safely show all ages"
        );
        assert_eq!(
            BoardView::new(DEFAULT_DETAIL, RecencyBucket::from_max_age_hours(Some(9))),
            BoardView::new(DEFAULT_DETAIL, RecencyBucket::LastNineHours),
            "a saved cutoff still reopens in compact view"
        );
    }

    #[test]
    fn recency_cutoff_hides_older_sections_and_updates_totals() {
        let now = now_secs() as u64;
        let snapshots: Vec<SessionSnapshot> = [
            (1, 30 * 60),
            (2, 2 * 60 * 60),
            (3, 4 * 60 * 60),
            (4, 7 * 60 * 60),
            (5, 10 * 60 * 60),
            (6, 18 * 60 * 60),
            (7, 2 * 24 * 60 * 60),
        ]
        .into_iter()
        .map(|(number, age)| {
            let mut pr = board_pr(number, "repo");
            make_primary(&mut pr);
            let mut session = snapshot("repo", vec![pr]);
            session.session_id = format!("session-{number}");
            session.prompted_at = now - age;
            session
        })
        .collect();
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);

        let six_hours = crate::parsers::strip_ansi_for_debug(&render_frame_with_oldest(
            &entries,
            DEFAULT_DETAIL,
            RecencyBucket::LastSixHours,
        ));
        assert!(six_hours
            .lines()
            .next()
            .unwrap()
            .contains("3 PRs · 3 sessions"));
        assert!(six_hours.contains("age ≤ 6h"));
        assert!(six_hours.contains("● Last hour"));
        assert!(six_hours.contains("● 1–3 hours"));
        assert!(six_hours.contains("● 3–6 hours"));
        assert!(!six_hours.contains("● 6–9 hours"));
        assert!(!six_hours.contains("#4: repo"));

        let last_hour = crate::parsers::strip_ansi_for_debug(&render_frame_with_oldest(
            &entries,
            DEFAULT_DETAIL,
            RecencyBucket::LastHour,
        ));
        assert!(last_hour
            .lines()
            .next()
            .unwrap()
            .contains("1 PRs · 1 sessions"));
        assert!(last_hour.contains("age ≤ 1h"));
        assert!(last_hour
            .lines()
            .any(|line| line.contains("repo") && line.contains("1:")));
        assert!(!last_hour.contains("#2: repo"));
    }

    #[test]
    fn sessions_with_visible_prs_do_not_get_duplicate_workspace_rows() {
        let mut pr = board_pr(1, "crabigator");
        make_primary(&mut pr);
        let session = snapshot("crabigator", vec![pr]);
        let snapshots = vec![session];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        assert_eq!(entries.len(), 1);
        assert!(local_workspaces(&snapshots, &entries).is_empty());
    }

    #[test]
    fn unowned_pr_mentions_keep_sessions_as_peer_rows_in_every_detail_mode() {
        let now = now_secs() as u64;
        let mut primary_pr = board_pr(2, "crabigator");
        make_primary(&mut primary_pr);

        let mut primary = snapshot("crabigator", vec![primary_pr.clone()]);
        primary.session_id = "primary-session".to_string();
        primary.branch = "main".to_string();
        primary.title = "PR owner session".to_string();
        primary.prompted_at = now - 180;
        primary.completed_at = now - 120;

        primary_pr.created_here = false;
        primary_pr.branch = "older-pr-branch".to_string();
        let mut secondary = snapshot("crabigator", vec![primary_pr]);
        secondary.session_id = "secondary-session".to_string();
        secondary.branch = "main".to_string();
        secondary.title = "Independent session".to_string();
        secondary.prompted_at = now - 60;
        secondary.completed_at = now - 30;

        let snapshots = vec![primary, secondary];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        assert_eq!(entries.len(), 1);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].session.session_id, "secondary-session");
        assert_eq!(entries[0].sessions.len(), 1);

        let shaped = entries_for_mode(entries, BoardMode::Sessions);
        for detail in 0..=MAX_DETAIL {
            let frame = crate::parsers::strip_ansi_for_debug(
                &render_session_view_frame(&shaped, &workspaces, detail)
                    .lines
                    .join("\n"),
            );
            assert!(frame.contains("2 sessions"));
            let header_row = frame
                .lines()
                .find(|line| line.contains("◆ ᛝ  PR owner session"))
                .expect("the owning session leads its block");
            assert!(header_row.contains(PROMPT_ICON));
            assert!(header_row.contains(COMPLETION_ICON));
            let pr_row = frame.lines().find(|line| line.contains("2:")).unwrap();
            assert!(
                !pr_row.contains(PROMPT_ICON),
                "the header carries the activity, not the PR sub-row: {pr_row}"
            );
            let session_row = frame
                .lines()
                .find(|line| line.contains("◇ ᛝ  Independent session"))
                .unwrap();
            assert!(session_row.contains(PROMPT_ICON));
            assert!(session_row.contains(COMPLETION_ICON));
            if detail == MAX_DETAIL {
                assert_eq!(
                    frame.matches("Independent session").count(),
                    1,
                    "the peer session must not also appear beneath the PR"
                );
            }
        }
    }

    #[test]
    fn cross_repo_primary_prs_identify_otherwise_unrepresented_sessions() {
        // A session working someone else's PR in another repository: the
        // strict ownership gate fails (repo and branch both differ), but no
        // session owns the PR, so its own primary classification stands.
        let mut review_pr = board_pr(1, "portal");
        make_primary(&mut review_pr);
        review_pr.created_here = false;
        review_pr.branch = "ryan/feature".to_string();
        let mut reviewer = snapshot("crabigator", vec![review_pr]);
        reviewer.session_id = "reviewer".to_string();
        reviewer.branch = "main".to_string();

        let snapshots = vec![reviewer];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sessions.len(), 1);
        assert_eq!(entries[0].sessions[0].session_id, "reviewer");
        assert!(
            workspaces.is_empty(),
            "the attached session needs no workspace row"
        );
    }

    #[test]
    fn cross_org_primaries_never_identify_sessions() {
        // A session in a personal repo that heavily discusses another org's
        // PR must keep its workspace row instead of migrating into that
        // organization's group.
        let mut discussed_pr = board_pr(1, "portal");
        make_primary(&mut discussed_pr);
        discussed_pr.created_here = false;
        discussed_pr.branch = "ryan/feature".to_string();
        let mut bystander = snapshot("crabigator", vec![discussed_pr]);
        bystander.session_id = "bystander".to_string();
        bystander.repo_owner = "someone-else".to_string();
        bystander.branch = "main".to_string();

        let snapshots = vec![bystander];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert_eq!(entries.len(), 1);
        assert!(entries[0].sessions.is_empty());
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].session.session_id, "bystander");
    }

    #[test]
    fn cross_repo_primaries_defer_to_a_session_that_owns_the_pr() {
        let mut review_pr = board_pr(1, "portal");
        make_primary(&mut review_pr);
        review_pr.created_here = false;
        review_pr.branch = "ryan/feature".to_string();
        let mut reviewer = snapshot("crabigator", vec![review_pr.clone()]);
        reviewer.session_id = "reviewer".to_string();
        reviewer.branch = "main".to_string();

        let mut owner = snapshot("portal", vec![review_pr]);
        owner.session_id = "owner".to_string();
        owner.branch = "ryan/feature".to_string();

        let snapshots = vec![reviewer, owner];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sessions.len(), 1);
        assert_eq!(entries[0].sessions[0].session_id, "owner");
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].session.session_id, "reviewer");
    }

    #[test]
    fn prs_a_session_opened_in_a_sibling_repo_stay_attached_to_it() {
        // One session fixes a bug across two repositories of the same org: the
        // checkout's own PR plus a PR it opened in a sibling repo. Both rows
        // carry the session, so both show its live activity.
        let mut portal_pr = board_pr(1260, "portal");
        make_primary(&mut portal_pr);
        portal_pr.branch = "sam/pal-force-refresh".to_string();
        let mut handler_pr = board_pr(2768, "handler");
        make_primary(&mut handler_pr);
        handler_pr.branch = "sam/hide-pal-publishing-state".to_string();
        let mut session = snapshot("portal", vec![portal_pr, handler_pr]);
        session.branch = "sam/pal-force-refresh".to_string();

        let snapshots = vec![session];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert_eq!(entries.len(), 2);
        for entry in &entries {
            assert_eq!(
                entry.sessions.len(),
                1,
                "{} lost its session",
                entry.pr.repo
            );
            assert_eq!(entry.sessions[0].session_id, "portal");
        }
        assert!(workspaces.is_empty());
    }

    #[test]
    fn paired_same_org_prs_share_session_title_state_and_activity() {
        let now = now_secs() as u64;
        let mut portal_pr = board_pr(1099, "developer-portal");
        make_primary(&mut portal_pr);
        portal_pr.created_here = false;
        portal_pr.branch = "sam/builder-document-intent".to_string();
        portal_pr.title = "Apply uploaded document instructions".to_string();
        let mut handler_pr = board_pr(2573, "request-handler");
        make_primary(&mut handler_pr);
        handler_pr.created_here = false;
        handler_pr.branch = "sam/builder-document-intent".to_string();
        handler_pr.title = "Classify uploaded document intent".to_string();

        let mut session = snapshot("developer-portal", vec![portal_pr, handler_pr]);
        session.branch = "sam/builder-document-intent".to_string();
        session.title = "Shared builder document work".to_string();
        session.state = SessionState::Thinking;
        session.prompted_at = now - 60;
        session.completed_at = now - 120;

        let snapshots = vec![session];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert_eq!(entries.len(), 2);
        for entry in &entries {
            assert_eq!(
                entry.sessions.len(),
                1,
                "{} lost its session",
                entry.pr.repo
            );
            let attached = &entry.sessions[0];
            assert_eq!(attached.title, "Shared builder document work");
            assert_eq!(attached.state, SessionState::Thinking);
            assert_eq!(attached.prompted_at, now - 60);
            assert_eq!(attached.completed_at, now - 120);
        }
        assert!(workspaces.is_empty());

        let frame = crate::parsers::strip_ansi_for_debug(
            &render_prs_frame(
                &entries_for_mode(entries, BoardMode::Prs).prs,
                DEFAULT_DETAIL,
            )
            .lines
            .join("\n"),
        );
        assert_eq!(
            frame.matches("Shared builder document work").count(),
            2,
            "each PR block shows the shared session title: {frame}"
        );
        for number in [1099, 2573] {
            let row = frame
                .lines()
                .find(|line| line.contains(&format!("#{number}:")))
                .unwrap_or_else(|| panic!("PR #{number} is missing from: {frame}"));
            assert!(
                row.contains(PROMPT_ICON),
                "PR #{number} lost its prompt time: {row}"
            );
            assert!(
                row.contains(COMPLETION_ICON),
                "PR #{number} lost its completion time: {row}"
            );
        }
    }

    #[test]
    fn prs_opened_in_another_org_still_need_a_matching_checkout() {
        let mut foreign_pr = board_pr(7, "portal");
        make_primary(&mut foreign_pr);
        foreign_pr.branch = "sam/fix".to_string();
        let mut session = snapshot("crabigator", vec![foreign_pr]);
        session.repo_owner = "someone-else".to_string();
        session.branch = "main".to_string();

        let snapshots = vec![session];
        let entries = aggregate(&snapshots, &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert!(entries[0].sessions.is_empty());
        assert_eq!(workspaces.len(), 1);
    }

    #[test]
    fn stale_session_mirrors_do_not_stay_on_the_board() {
        let mut session = snapshot("crabigator", Vec::new());
        session.last_updated = now_secs() - STALE_SESSION_SECS - 60.0;

        let workspaces = local_workspaces(&[session], &[]);

        assert!(workspaces.is_empty());
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
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );

        assert_eq!(entries.len(), 1);
        assert!(entries[0].pr.primary);
        assert_eq!(entries[0].pr.primary_source, "auto");
        assert_eq!(entries[0].pr.mentions, 7);
    }

    #[test]
    fn aggregation_promotes_a_worktree_directory_match_from_an_older_mirror() {
        let mut attached = board_pr(1206, "portal");
        attached.branch = "sam/keep-draft-values".to_string();
        attached.title = "Keep draft values during autosave".to_string();
        let mut old_mirror = snapshot("keep-draft-values", vec![attached]);
        old_mirror.repo_name = "portal".to_string();
        old_mirror.title = "⟁ Fix builder autosave".to_string();

        let entries = aggregate(
            &[old_mirror],
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );

        assert_eq!(entries.len(), 1);
        assert!(entries[0].pr.primary);
        assert_eq!(entries[0].pr.primary_source, "auto");
        assert_eq!(entries[0].sessions.len(), 1);
        assert_eq!(entries[0].pr.title, "Keep draft values during autosave");
        assert_eq!(entries[0].sessions[0].title, "⟁ Fix builder autosave");
    }

    #[test]
    fn temporary_worktree_visits_do_not_override_the_session_classification() {
        let mut source = board_pr(1437, "portal");
        source.branch = "sam/source-fix".into();
        source.worktree_visit = true;
        let mut session = snapshot("source-fix", vec![source.clone()]);
        session.repo_name = "portal".into();
        assert!(!effective_session_pr(&session, &source, None).primary);
        assert!(effective_session_pr(&session, &source, Some(PrDisposition::Primary)).primary);
    }

    #[test]
    fn secondary_override_beats_retroactive_worktree_ownership() {
        let mut attached = board_pr(1206, "portal");
        attached.branch = "sam/keep-draft-values".to_string();
        let mut old_mirror = snapshot("keep-draft-values", vec![attached]);
        old_mirror.repo_name = "portal".to_string();
        old_mirror.branch = "sam/keep-draft-values".to_string();
        let mut overrides = HashMap::new();
        overrides.insert("o/portal#1206".to_string(), PrDisposition::Secondary);

        let entries = aggregate(
            &[old_mirror],
            &ScopedOverrides::from(overrides),
            DEFAULT_LINGER_DAYS,
        );

        assert_eq!(entries.len(), 1);
        assert!(!entries[0].pr.primary);
        assert!(entries[0].sessions.is_empty());
    }

    #[test]
    fn overrides_reshape_the_board() {
        let primary_key = "o/portal#5".to_string();
        let dismissed_key = "o/portal#6".to_string();
        let mut overrides = ScopedOverrides::default();
        overrides.insert(primary_key, String::new(), PrDisposition::Primary);
        overrides.insert(dismissed_key, String::new(), PrDisposition::Dismissed);

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

    /// A dismissal scoped to one session hides the PR from that session's
    /// rows without touching a parallel session in the same directory — and a
    /// path-scoped one covers every session in the worktree.
    #[test]
    fn scoped_dismissals_only_reach_their_own_session() {
        let key = "o/portal#7".to_string();
        let mut overrides = ScopedOverrides::default();
        overrides.insert(
            key.clone(),
            "session:one".to_string(),
            PrDisposition::Dismissed,
        );

        let mut pr = board_pr(7, "portal");
        make_primary(&mut pr);
        let entries = aggregate(
            &[
                snapshot("one", vec![pr.clone()]),
                snapshot("two", vec![pr.clone()]),
            ],
            &overrides,
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1, "the PR survives for the other session");
        assert!(!entries[0].pr.dismissed);
        assert_eq!(
            entries[0].sessions.len(),
            1,
            "only the undismissed session claims the PR"
        );
        assert_eq!(entries[0].sessions[0].session_id, "two");

        // The same dismissal keyed to session one's path covers it too.
        let mut overrides = ScopedOverrides::default();
        overrides.insert(key, "path:/tmp/one".to_string(), PrDisposition::Dismissed);
        let entries = aggregate(
            &[snapshot("one", vec![pr.clone()]), snapshot("two", vec![pr])],
            &overrides,
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sessions.len(), 1);
        assert_eq!(entries[0].sessions[0].session_id, "two");
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
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );
        let numbers: Vec<u64> = entries.iter().map(|e| e.pr.number).collect();
        assert_eq!(numbers, vec![1, 3, 2], "failing → ready → merged");
    }

    #[test]
    fn titled_row_stays_compact_and_recap_view_carries_slack() {
        let mut pr = board_pr(9, "portal");
        make_primary(&mut pr);
        pr.title = "Fix the flow".to_string();
        pr.ai_note = "CI green, awaiting review".to_string();
        pr.ai_confidence = "medium".to_string();
        pr.mentions = 12;
        pr.user_mentions = 3;
        pr.slack_origin_url = "https://t.slack.com/archives/C1/p1723500000000000".to_string();
        pr.slack_comment_urls =
            vec!["https://t.slack.com/archives/C2/p1723500000000001".to_string()];

        let mut session = snapshot("one", vec![pr]);
        session.repo_name = "portal".to_string();
        session.title = "Builder Signals dashboard".to_string();
        session.title_set_at = now_ms() - 2 * 60 * 60 * 1000;
        session.recap = Some(RecapBrief {
            headline: "Finished the dashboard changes".to_string(),
            bullets: Vec::new(),
            next_prompt_notes: vec!["Wait for approval".to_string()],
            artifacts: Vec::new(),
            generated_at: now_ms() - 60_000,
            line_delta: crate::recap::TurnLineDelta::default(),
        });
        session.slack_threads = vec![
            SlackThread {
                url: "https://t.slack.com/archives/C1/p1723500000000000".to_string(),
                posted_at: 1_723_500_000,
                channel: Some("builder".to_string()),
                author: Some("Sam Clay".to_string()),
                text: None,
            },
            SlackThread {
                url: "https://t.slack.com/archives/C2/p1723500000000001".to_string(),
                posted_at: 1_723_500_001,
                channel: Some("pr-reviews".to_string()),
                author: Some("Mango".to_string()),
                text: None,
            },
            SlackThread {
                url: "https://t.slack.com/archives/C3/p1723500000000002".to_string(),
                posted_at: 1_723_500_002,
                channel: Some("builder-dev".to_string()),
                author: Some("Kapil".to_string()),
                text: None,
            },
            SlackThread {
                url: "https://t.slack.com/archives/C4/p1723500000000003".to_string(),
                posted_at: 1_723_500_003,
                channel: Some("deployments".to_string()),
                author: Some("Ivy".to_string()),
                text: None,
            },
        ];
        let entries = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);
        let compact = render_frame_at(&entries, 0);
        assert!(compact.contains("Builder Signals dashboard"));
        assert!(crate::parsers::strip_ansi_for_debug(&compact).contains("#9: Fix the flow"));
        assert!(!compact.contains("#builder"));
        assert!(!compact.contains("CI green, awaiting review"));
        assert!(!compact.contains("Wait for approval"));

        let recap = render_frame_at(&entries, 1);
        assert!(
            recap.contains("#builder"),
            "{}",
            crate::parsers::strip_ansi_for_debug(&recap)
        );
        assert!(recap.contains("https://t.slack.com/archives/C1/p1723500000000000"));
        assert!(recap.contains("https://t.slack.com/archives/C2/p1723500000000001"));
        assert!(recap.contains("https://t.slack.com/archives/C3/p1723500000000002"));
        assert!(recap.contains("https://t.slack.com/archives/C4/p1723500000000003"));
        assert!(
            recap.find("archives/C1").unwrap() < recap.find("archives/C2").unwrap(),
            "the original thread renders first"
        );
        assert_eq!(
            recap
                .lines()
                .filter(|line| line.contains("slack.com"))
                .count(),
            4,
            "each Slack target gets a clickable row"
        );
        for removed in [
            "slack origin",
            "mentions",
            "spoken",
            "uncommitted",
            "▓",
            "%",
        ] {
            assert!(!recap.contains(removed), "removed status text: {removed}");
        }

        assert!(recap.contains("CI green, awaiting review"));
        let judgment_line = recap
            .lines()
            .find(|line| line.contains("CI green, awaiting review"))
            .unwrap();
        assert!(
            judgment_line.contains("#builder"),
            "the first Slack link shares the judgment row"
        );
        let headline_line = recap
            .lines()
            .find(|line| line.contains("Finished the dashboard changes"))
            .unwrap();
        assert!(
            !headline_line.contains("Builder Signals dashboard"),
            "the headline gets its own detail row below the sub-row"
        );
        assert!(recap.contains("Next: Wait for approval"));
        assert_eq!(
            recap.lines().count(),
            compact.lines().count() + 6,
            "the judgment with four Slack links, the headline, and the next step add six rows"
        );
    }

    /// Slack links render only in recap view, so compact view spends their
    /// room on the title. In both views the diff stays beside the activity
    /// column, and a link lines up with the right edge like the status.
    #[test]
    fn title_takes_the_slack_link_room_and_the_diff_hugs_the_activity() {
        let title = "Skip URL-less documents in the component builder so knowledge sync \
                     stops failing on null links";
        let mut pr = board_pr(9, "portal");
        make_primary(&mut pr);
        pr.title = title.to_string();
        pr.additions = 34;
        pr.deletions = 6;
        pr.ai_note = "CI green, awaiting review".to_string();
        let mut session = snapshot("one", vec![pr]);
        session.prompted_at = now_secs() as u64 - 30 * 60;
        session.completed_at = now_secs() as u64 - 2 * 60 * 60;
        session.slack_threads = vec![SlackThread {
            url: "https://t.slack.com/archives/C1/p1723500000000000".to_string(),
            posted_at: 1_723_500_000,
            channel: Some("deployments-and-releases".to_string()),
            author: Some("Samuel Clay".to_string()),
            text: None,
        }];
        let entries = aggregate(&[session], &ScopedOverrides::default(), DEFAULT_LINGER_DAYS);

        // The styled frame line whose text holds `needle`.
        fn row_with<'a>(frame: &'a str, needle: &str) -> &'a str {
            frame
                .lines()
                .find(|line| crate::parsers::strip_ansi_for_debug(line).contains(needle))
                .unwrap_or_else(|| panic!("no line holds {needle:?}:\n{frame}"))
        }
        // The activity column ends with the completion age; only the column
        // gap and the diff's own leading space may separate them.
        let gap_before_diff = |row: &str| {
            let before_diff = &row[..row.find("+34 -6").unwrap()];
            before_diff.len() - before_diff.trim_end().len()
        };

        let compact_frame = render_frame_at(&entries, 0);
        let compact = crate::parsers::strip_ansi_for_debug(row_with(&compact_frame, "#9:"));
        assert!(
            compact.contains(title),
            "the title takes the room: {compact}"
        );
        assert!(gap_before_diff(&compact) <= 2, "{compact}");

        let recap = render_frame_at(&entries, 1);
        let recap_row = crate::parsers::strip_ansi_for_debug(row_with(&recap, "#9:"));
        assert!(gap_before_diff(&recap_row) <= 2, "{recap_row}");

        let link_row = row_with(&recap, "#deployments-and-releases");
        let link_text = crate::parsers::strip_ansi_for_debug(link_row);
        assert!(
            link_text.contains("CI green, awaiting review"),
            "the link shares the judgment row: {link_text}"
        );
        let label = crate::slack::thread_identity_label(&entries[0].slack_threads[0]);
        let label_at = link_row.find(&label).expect("the link renders whole");
        let right_edge = 160 - crate::ui::pr_cells::PR_RIGHT_PADDING;
        assert_eq!(
            crate::ui::utils::strip_ansi_len(&link_row[..label_at]) + label.width(),
            right_edge,
            "the link ends at the right edge: {link_text}"
        );
        assert_eq!(
            crate::ui::utils::strip_ansi_len(link_row),
            right_edge,
            "nothing follows the link: {link_text}"
        );
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
            &ScopedOverrides::default(),
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
            &ScopedOverrides::default(),
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

        assert!(aggregate(&snapshots, &ScopedOverrides::default(), 1).is_empty());
        assert_eq!(
            aggregate(&snapshots, &ScopedOverrides::default(), 3).len(),
            1
        );
        // Zero shows open PRs only, no matter how fresh the merge.
        let mut fresh = board_pr(2, "portal");
        fresh.state = "MERGED".to_string();
        fresh.closed_at = now_ms();
        make_primary(&mut fresh);
        assert!(aggregate(
            &[snapshot("one", vec![fresh])],
            &ScopedOverrides::default(),
            0
        )
        .is_empty());
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
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pr.number, 3);
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
                session_id: "portal".to_string(),
                platform: PlatformKind::Claude,
                pr_scope: String::new(),
                dir_name: "portal".to_string(),
                session_dir: Some(dir.path().to_path_buf()),
                title: String::new(),
                title_set_at: 0,
                branch: String::new(),
                recap: None,
                state: SessionState::Ready,
                prompted_at: 0,
                completed_at: 0,
                ended: false,
            }],
            slack_threads: Vec::new(),
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
        let mut pr = board_pr(9, "portal");
        make_primary(&mut pr);
        pr.additions = 49;
        pr.deletions = 5;
        pr.changed_files = 3;
        pr.mergeable = "MERGEABLE".to_string();
        pr.ai_note = "Open and ready for review".to_string();
        pr.title = "Make PR titles official".to_string();
        pr.branch = "sam/pr-title-layout".to_string();
        let mut with_title = snapshot("portal", vec![pr.clone()]);
        with_title.title = "Wiring the PR board detail levels".to_string();
        with_title.title_set_at = now_ms() - 2 * 60 * 60 * 1000;
        with_title.branch = "sam/detail-levels".to_string();
        with_title.prompted_at = now_secs() as u64 - 30 * 60;
        with_title.completed_at = now_secs() as u64 - 2 * 60 * 60;
        with_title.recap = Some(RecapBrief {
            headline: "Added r-toggled recaps to the PR board".to_string(),
            bullets: Vec::new(),
            next_prompt_notes: vec!["Next: Ready to merge once checks finish".to_string()],
            artifacts: Vec::new(),
            generated_at: now_ms() - 42 * 60 * 1000,
            line_delta: crate::recap::TurnLineDelta {
                additions: 120,
                deletions: 35,
            },
        });
        let mut bare = snapshot("other-dir", vec![pr]);
        bare.prompted_at = now_secs() as u64 - 4 * 60 * 60;
        bare.completed_at = now_secs() as u64 - 5 * 60 * 60;
        aggregate(
            &[with_title, bare],
            &ScopedOverrides::default(),
            DEFAULT_LINGER_DAYS,
        )
    }

    /// The title stays on the PR row; `r` adds the complete recap beneath it.
    #[test]
    fn compact_and_recap_views_keep_one_titled_pr_row() {
        let entries = titled_entries();

        let compact = render_frame(&entries);
        assert!(!compact.contains('▓'), "no progress bar at compact");
        assert!(compact.contains("Make PR titles official"));
        assert!(compact.contains("Wiring the PR board detail levels"));
        assert!(
            crate::parsers::strip_ansi_for_debug(&compact).contains("#9: Make PR titles official")
        );
        assert!(compact.contains("compact"), "header names the level");
        assert!(compact.contains("⟩ 30m"));
        assert!(compact.contains("⋖ 2h"));
        assert!(
            !compact.contains("Open and ready for review"),
            "judgments wait for recap view"
        );
        assert!(
            crate::parsers::strip_ansi_for_debug(&compact).contains("⎇  sam/detail-levels"),
            "the session's branch rides the sub-row"
        );
        assert!(
            !compact.contains("Added r-toggled"),
            "the recap headline waits for recap view"
        );
        assert!(
            !compact.contains("Next: Ready to merge"),
            "the rest of the recap waits for recap view"
        );

        let recaps = render_frame_at(&entries, 1);
        assert!(recaps.contains("Wiring the PR board"));
        assert!(recaps.contains("Open and ready for review"));
        assert!(recaps.contains("Added r-toggled"));
        assert!(recaps.contains("Next: Ready to merge once checks finish"));
        assert!(!recaps.contains("ago"), "expanded ages stay compact");
        assert!(
            crate::parsers::strip_ansi_for_debug(&recaps).contains(" · r recap"),
            "header uses the level name"
        );
        assert!(recaps.contains("⟩ 30m"));
        assert!(recaps.contains("⋖ 2h"));

        assert_eq!(
            recaps.lines().count(),
            compact.lines().count() + 3,
            "recap view adds the judgment, the headline, and the next step"
        );

        let compact_line = recaps
            .lines()
            .find(|line| {
                crate::parsers::strip_ansi_for_debug(line).contains("#9: Make PR titles official")
            })
            .unwrap();
        let judgment_line = recaps
            .lines()
            .find(|line| line.contains("Open and ready for review"))
            .unwrap();
        let next_line = recaps
            .lines()
            .find(|line| line.contains("Next: Ready to merge once checks finish"))
            .unwrap();
        let compact_plain = crate::parsers::strip_ansi_for_debug(compact_line);
        assert!(compact_plain.contains("Make PR titles official"));
        assert!(
            compact_plain.contains("#9: Make PR titles official"),
            "{compact_plain}"
        );
        let generated_line = recaps
            .lines()
            .find(|line| line.contains("ᛝ  Wiring the PR board detail levels"))
            .unwrap();
        let generated_plain = crate::parsers::strip_ansi_for_debug(generated_line);
        assert!(
            generated_plain.trim_start().starts_with("◆ ᛝ  "),
            "the session sub-row sits below the PR header: {generated_plain}"
        );
        assert!(
            compact_plain.contains("⎇  sam/pr-title-layout"),
            "the header carries the branch: {compact_plain}"
        );
        assert!(
            regex::Regex::new(r"\x1b\]8;;[^\x07]*\x07")
                .unwrap()
                .replace_all(&compact_plain, "")
                .contains("+49 -5"),
            "the board keeps the diff: {compact_plain}"
        );
        assert!(!compact_plain.contains('☰'), "the board omits file counts");
        assert!(
            !['↑', '↓']
                .iter()
                .any(|glyph| compact_plain.contains(*glyph)),
            "the PR board omits promote and demote actions: {compact_plain}"
        );
        assert!(
            compact_plain.contains('✕'),
            "the PR board keeps its dismiss action: {compact_plain}"
        );
        assert!(compact_line.contains(&escape::hyperlink(
            &entries[0].pr.url,
            &format!(
                "{}{}#9{}{}: Make PR titles official",
                escape::BOLD,
                escape::UNDERLINE,
                escape::RESET_UNDERLINE,
                escape::RESET_BOLD
            )
        )));
        assert!(
            crate::parsers::strip_ansi_for_debug(judgment_line).starts_with("   ✦ "),
            "the PR judgment remains a separate line"
        );
        assert!(
            crate::parsers::strip_ansi_for_debug(next_line).starts_with("     Next: "),
            "the next step remains a separate recap line"
        );

        let row = crate::parsers::strip_ansi_for_debug(&compact);
        assert!(
            row.find("⋖ 2h").unwrap() < row.find("open").unwrap(),
            "activity belongs before the GitHub status"
        );
        let main_row = compact
            .lines()
            .find(|line| {
                crate::parsers::strip_ansi_for_debug(line).contains("#9: Make PR titles official")
            })
            .unwrap();
        assert_eq!(
            crate::ui::utils::strip_ansi_len(main_row),
            159,
            "row keeps the right-edge padding"
        );
    }

    #[test]
    fn recap_detail_labels_are_added_once() {
        assert_eq!(
            labeled_recap_detail("Next:", "Continue from the last check"),
            "Next: Continue from the last check"
        );
        assert_eq!(
            labeled_recap_detail("Next:", "next: Continue from the last check"),
            "next: Continue from the last check"
        );
        assert_eq!(
            labeled_recap_detail("Artifact:", "Artifact: report.txt"),
            "Artifact: report.txt"
        );
    }

    #[test]
    fn changed_line_render_only_repaints_modified_and_removed_rows() {
        let mut previous = vec![
            "unchanged".to_string(),
            "old value".to_string(),
            "removed".to_string(),
        ];
        let current = vec!["unchanged".to_string(), "new value".to_string()];
        let mut output = Vec::new();

        assert!(draw_changed_lines(&mut output, &mut previous, &current).unwrap());
        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with(escape::SYNC_BEGIN));
        assert!(output.ends_with(escape::SYNC_END));
        assert!(!output.contains(escape::CLEAR_SCREEN_HOME));
        assert!(!output.contains(&escape::cursor_to(1, 1)));
        assert!(output.contains(&escape::cursor_to(2, 1)));
        assert!(output.contains(&escape::cursor_to(3, 1)));
        assert!(output.contains("new value"));
        assert_eq!(previous, current);

        let mut no_output = Vec::new();
        assert!(!draw_changed_lines(&mut no_output, &mut previous, &current).unwrap());
        assert!(no_output.is_empty());
    }

    #[test]
    fn activity_highlights_the_event_that_matches_session_state() {
        let now = 1_000_000;
        let mut sessions = vec![
            SessionRef {
                session_id: "older".to_string(),
                platform: PlatformKind::Claude,
                pr_scope: String::new(),
                dir_name: "older".to_string(),
                session_dir: None,
                title: String::new(),
                title_set_at: 0,
                branch: String::new(),
                recap: None,
                state: SessionState::Complete,
                prompted_at: now - 4 * 60 * 60,
                completed_at: now - 8 * 60 * 60,
                ended: false,
            },
            SessionRef {
                session_id: "newer".to_string(),
                platform: PlatformKind::Codex,
                pr_scope: String::new(),
                dir_name: "newer".to_string(),
                session_dir: None,
                title: String::new(),
                title_set_at: 0,
                branch: String::new(),
                recap: None,
                state: SessionState::Thinking,
                prompted_at: now - 30 * 60,
                completed_at: now - 2 * 60 * 60,
                ended: false,
            },
        ];
        let activity = activity_cell(&sessions, now * 1000, 0, &Cooldowns::default());
        assert!(activity.styled.contains("⟩ 30m"));
        assert!(activity.styled.contains("⋖ 2h"));
        assert!(
            crate::parsers::strip_ansi_for_debug(&activity.styled).starts_with(" ⠋   ⟩"),
            "thinking state sits centered in its badge slot before prompt activity"
        );
        assert!(activity
            .styled
            .contains(&format!("{}⟩ 30m", fg(ACTIVITY_BRIGHT_TEAL))));
        assert!(activity
            .styled
            .contains(&format!("{}⋖ 2h", fg(ACTIVITY_DARK_TEAL))));
        assert!(!activity.styled.contains("\x1b[48;5;"));

        sessions[1].state = SessionState::Complete;
        let settled = activity_cell(&sessions, now * 1000, 0, &Cooldowns::default());
        assert!(settled
            .styled
            .contains(&format!("{}⟩ 30m", fg(ACTIVITY_DARK_TEAL))));
        assert!(settled
            .styled
            .contains(&format!("{}⋖ 2h", fg(ACTIVITY_BRIGHT_TEAL))));

        assert_eq!(recency_color(0), RECENCY_1H);
        assert_eq!(recency_color(3_600), RECENCY_3H);
        assert_eq!(recency_color(10_800), RECENCY_6H);
        assert_eq!(recency_color(21_600), RECENCY_9H);
        assert_eq!(recency_color(32_400), RECENCY_12H);
        assert_eq!(recency_color(43_200), RECENCY_24H);
        assert_eq!(recency_color(86_400), RECENCY_24H);
        assert_eq!(recency_color(86_401), color::DARK_GRAY);

        let unknown = activity_cell(&[], now * 1000, 0, &Cooldowns::default());
        assert!(unknown.styled.contains("⟩ —"));
        assert!(unknown.styled.contains("⋖ —"));
        assert!(unknown
            .styled
            .contains(&format!("{}⟩ —", fg(color::DARK_GRAY))));
        assert!(unknown
            .styled
            .contains(&format!("{}⋖ —", fg(color::DARK_GRAY))));
        assert!(!unknown.styled.contains("\x1b[48;5;"));
    }

    /// Quiet states center a colored icon in the three-cell slot; the two
    /// states that wait on the user fill it with chevrons on a bright
    /// background so they stand out from a column of ✓ and ○.
    #[test]
    fn activity_icons_cover_every_session_state_and_thinking_animates() {
        let cases = [
            (SessionState::Ready, " ○ ", fg(color::GRAY)),
            (SessionState::Permission, "»!«", bg(color::YELLOW)),
            (SessionState::Question, "»?«", bg(color::DARK_ORANGE)),
            (SessionState::Complete, " ✓ ", fg(color::PURPLE)),
            (SessionState::Interrupted, " ⊘ ", fg(color::RED)),
        ];
        for (state, badge, styling) in cases {
            let session = SessionRef {
                session_id: "state".to_string(),
                platform: PlatformKind::Claude,
                pr_scope: String::new(),
                dir_name: "state".to_string(),
                session_dir: None,
                title: String::new(),
                title_set_at: 0,
                branch: String::new(),
                recap: None,
                state,
                prompted_at: 1,
                completed_at: 1,
                ended: false,
            };
            let activity = activity_cell(&[session], 1000, 0, &Cooldowns::default());
            let plain = crate::parsers::strip_ansi_for_debug(&activity.styled);
            assert!(plain.starts_with(&format!("{badge}  ⟩")), "{plain}");
            assert!(activity.styled.contains(&styling), "{state:?}");
            assert!(
                activity
                    .styled
                    .contains(&format!("{}⟩ 1m", fg(ACTIVITY_DARK_TEAL))),
                "{state:?}"
            );
            assert!(
                activity
                    .styled
                    .contains(&format!("{}⋖ 1m", fg(ACTIVITY_BRIGHT_TEAL))),
                "{state:?}"
            );
        }

        let thinking = SessionRef {
            session_id: "thinking".to_string(),
            platform: PlatformKind::Codex,
            pr_scope: String::new(),
            dir_name: "thinking".to_string(),
            session_dir: None,
            title: String::new(),
            title_set_at: 0,
            branch: String::new(),
            recap: None,
            state: SessionState::Thinking,
            prompted_at: 1,
            completed_at: 1,
            ended: false,
        };
        let first = activity_cell(
            std::slice::from_ref(&thinking),
            1000,
            0,
            &Cooldowns::default(),
        );
        let second = activity_cell(&[thinking], 1000, 1, &Cooldowns::default());
        assert_ne!(
            crate::parsers::strip_ansi_for_debug(&first.styled),
            crate::parsers::strip_ansi_for_debug(&second.styled)
        );
        assert!(first.styled.contains(&fg(color::GREEN)));
        assert!(first
            .styled
            .contains(&format!("{}⟩ 1m", fg(ACTIVITY_BRIGHT_TEAL))));
        assert!(first
            .styled
            .contains(&format!("{}⋖ 1m", fg(ACTIVITY_DARK_TEAL))));
    }

    #[test]
    fn recap_view_shows_standalone_session_recap_without_repeating_its_title() {
        let now = now_secs() as u64;
        let mut session = snapshot("crabigator", Vec::new());
        session.platform = PlatformKind::Codex;
        session.title = "⟁  Standalone work".to_string();
        session.recap = Some(RecapBrief {
            headline: "Kept standalone rows visible".to_string(),
            bullets: vec![
                "Preserved the first recap bullet".to_string(),
                "Preserved the second recap bullet".to_string(),
            ],
            next_prompt_notes: vec!["Verify the standalone recap".to_string()],
            artifacts: vec![
                "first-report.txt".to_string(),
                "second-report.txt".to_string(),
            ],
            generated_at: now * 1000,
            line_delta: crate::recap::TurnLineDelta {
                additions: 8,
                deletions: 2,
            },
        });
        let workspaces = local_workspaces(&[session], &[]);
        let rows: Vec<WorkspaceRow<'_>> = workspaces
            .iter()
            .map(|entry| WorkspaceRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();

        let styled = render(
            &[],
            &[],
            &rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::new(MAX_DETAIL, DEFAULT_OLDEST_VISIBLE_BUCKET),
        )
        .lines
        .join("\n");
        assert!(styled.contains(&format!("{}◇ ⟁  Standalone work", fg(color::PURPLE))));
        assert!(styled.contains(&format!("{}Kept standalone rows visible", fg(color::GRAY))));
        let frame = crate::parsers::strip_ansi_for_debug(&styled);
        assert_eq!(frame.matches("Standalone work").count(), 1);
        assert!(frame.contains("⟁  Standalone work"));
        assert!(frame.contains("↪ Kept standalone rows visible"));
        assert!(frame.contains("• Preserved the first recap bullet"));
        assert!(frame.contains("• Preserved the second recap bullet"));
        assert!(frame.contains("Next: Verify the standalone recap"));
        assert!(frame.contains("Artifact: first-report.txt"));
        assert!(frame.contains("Artifact: second-report.txt"));
        assert!(frame.contains("+8 -2"));
    }

    #[test]
    fn untitled_standalone_session_uses_primary_identity_color() {
        let workspaces = local_workspaces(&[snapshot("crabigator", Vec::new())], &[]);
        let rows = [WorkspaceRow {
            entry: &workspaces[0],
            preview_lines: Vec::new(),
        }];
        let styled = render(
            &[],
            &[],
            &rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
        )
        .lines
        .join("\n");
        assert!(styled.contains(&format!("{}◇ ᛝ  crabigator", fg(color::PURPLE))));
    }

    #[test]
    fn activity_sort_uses_the_newest_prompt_and_ignores_completions() {
        let now = 100_000;
        let mut session = SessionRef {
            session_id: "one".to_string(),
            platform: PlatformKind::Claude,
            pr_scope: String::new(),
            dir_name: "crabigator".to_string(),
            session_dir: None,
            title: String::new(),
            title_set_at: 0,
            branch: String::new(),
            recap: None,
            state: SessionState::Ready,
            prompted_at: now - 8 * 60,
            completed_at: now - 13 * 60 * 60,
            ended: false,
        };
        assert_eq!(
            activity_sort_time(std::slice::from_ref(&session)),
            session.prompted_at
        );
        assert_eq!(
            activity_bucket(std::slice::from_ref(&session), now),
            RecencyBucket::LastHour
        );

        session.completed_at = now - 30;
        assert_eq!(
            activity_sort_time(std::slice::from_ref(&session)),
            session.prompted_at
        );
    }

    #[test]
    fn live_mirror_activity_timestamps_are_read_from_stats() {
        let data = serde_json::json!({
            "widgets": {
                "stats": {
                    "data": {
                        "prompts_changed_at": 1_234_567.75,
                        "completions_changed_at": 1_234_890
                    }
                }
            }
        });
        assert_eq!(
            mirror_activity_timestamp(&data, "prompts_changed_at"),
            1_234_567
        );
        assert_eq!(
            mirror_activity_timestamp(&data, "completions_changed_at"),
            1_234_890
        );
        assert_eq!(mirror_activity_timestamp(&data, "missing"), 0);
        assert_eq!(activity_timestamp_secs(f64::NAN), 0);
    }

    #[test]
    fn live_mirror_state_includes_interrupted_sessions() {
        let interrupted = serde_json::json!({
            "widgets": { "stats": { "data": { "state": "interrupted" } } }
        });
        assert_eq!(
            mirror_session_state(&interrupted),
            SessionState::Interrupted
        );
        assert_eq!(
            mirror_session_state(&serde_json::json!({})),
            SessionState::Ready
        );
    }

    #[test]
    fn legacy_mirrors_recover_the_provider_without_a_platform_field() {
        assert_eq!(
            mirror_platform(&serde_json::json!({
                "transcript_path": "/Users/me/.codex/sessions/session.jsonl"
            })),
            PlatformKind::Codex
        );
        assert_eq!(
            mirror_platform(&serde_json::json!({
                "transcript_path": "/Users/me/.claude/projects/session.jsonl"
            })),
            PlatformKind::Claude
        );
        assert_eq!(
            mirror_platform(&serde_json::json!({ "terminal_title": "⟁  Existing title" })),
            PlatformKind::Codex
        );
    }

    #[test]
    fn legacy_hook_history_supplies_activity_timestamps() {
        let stats = serde_json::json!({
            "event_history": [
                {"ts": 100.25, "event": "UserPromptSubmit"},
                {"ts": 120.5, "event": "Stop"},
                {"ts": 200.75, "event": "UserPromptSubmit"},
                {"ts": 240.5, "event": "PostToolUse"}
            ]
        });
        let activity = hook_activity_from_stats(&stats);
        assert_eq!(activity.prompted_at, 200);
        assert_eq!(activity.completed_at, 120);
    }

    #[test]
    fn legacy_transcripts_supply_and_increment_activity() {
        use std::io::Write as _;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        let large_tool_line = format!(
            "{{\"timestamp\":\"2026-08-06T12:01:00Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"function_call_output\",\"output\":\"{}\"}}}}\n",
            "x".repeat(TRANSCRIPT_SCAN_CHUNK as usize * 2)
        );
        std::fs::write(
            &path,
            format!(
                "{{\"timestamp\":\"2026-08-06T12:00:00Z\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"user_message\"}}}}\n{large_tool_line}{{\"timestamp\":\"2026-08-06T12:02:00Z\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"agent_message\",\"phase\":\"commentary\"}}}}\n"
            ),
        )
        .unwrap();

        let expected_prompt = chrono::DateTime::parse_from_rfc3339("2026-08-06T12:00:00Z")
            .unwrap()
            .timestamp() as u64;
        let expected_completion = chrono::DateTime::parse_from_rfc3339("2026-08-06T12:02:00Z")
            .unwrap()
            .timestamp() as u64;
        let mut cache = ActivityHistory::default();
        let first = cache.transcript_activity(&path, ActivityTimes::default());
        assert_eq!(first.prompted_at, expected_prompt);
        assert_eq!(first.completed_at, expected_completion);

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(
            file,
            "{{\"timestamp\":\"2026-08-06T12:03:00Z\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"agent_message\",\"phase\":\"final\"}}}}"
        )
        .unwrap();
        let second = cache.transcript_activity(&path, ActivityTimes::default());
        assert_eq!(second.prompted_at, expected_prompt);
        assert_eq!(
            second.completed_at,
            chrono::DateTime::parse_from_rfc3339("2026-08-06T12:03:00Z")
                .unwrap()
                .timestamp() as u64
        );
    }

    #[test]
    fn slack_metadata_from_one_transcript_enriches_board_links() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        std::fs::write(
            &path,
            r#"{"type":"response_item","output":"MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n1786679090.037079,U0AEVB0102V,mango,Mango,C0AGPSMKQ3Z (#pr-reviews),,\"message\""}"#,
        )
        .unwrap();

        let mut pr = board_pr(1175, "developer-portal");
        pr.slack_comment_urls =
            vec!["https://tavus.slack.com/archives/C0AGPSMKQ3Z/p1786679090037079".into()];
        let data = serde_json::json!({
            "last_updated": now_secs(),
            "transcript_path": path,
            "cwd": dir.path(),
            "prs": [pr],
            "widgets": {
                "git": {
                    "data": {
                        "repo_owner": "o",
                        "repo_name": "developer-portal"
                    }
                }
            }
        });
        let mut history = ActivityHistory::default();
        let snapshot = snapshot_from_instance(dir.path().join("inspect.json"), data, &mut history)
            .expect("fresh session snapshot");

        let label = crate::slack::thread_identity_label(&snapshot.slack_threads[0]);
        // The trailing post time renders in the local timezone.
        assert!(
            label.starts_with("#pr-reviews · Mango · "),
            "unexpected label: {label}"
        );
    }

    /// Many permalinks into the same channel collapse to one row, labeled by
    /// whichever thread knows the most about the conversation.
    #[test]
    fn slack_rows_are_unique_per_channel_and_keep_the_best_metadata() {
        let thread = |message: &str, channel: Option<&str>, author: Option<&str>| SlackThread {
            url: format!("https://t.slack.com/archives/C1/p{message}"),
            posted_at: message[..10].parse().unwrap(),
            channel: channel.map(str::to_string),
            author: author.map(str::to_string),
            text: None,
        };
        let threads = vec![
            thread("1754404040123456", Some("C1"), None),
            thread("1754404050123456", Some("builder"), Some("Sam Clay")),
            thread("1754404060123456", Some("C1"), None),
            SlackThread {
                url: "https://t.slack.com/archives/C2/p1754404070123456".into(),
                posted_at: 1_754_404_070,
                channel: Some("C2".into()),
                author: None,
                text: None,
            },
        ];

        let unique = unique_channel_threads(&threads);
        assert_eq!(unique.len(), 2);
        assert_eq!(unique[0].channel.as_deref(), Some("builder"));
        assert_eq!(unique[0].author.as_deref(), Some("Sam Clay"));
        assert_eq!(unique[1].channel.as_deref(), Some("C2"));
    }

    #[test]
    fn claude_transcript_distinguishes_prompts_and_end_turns() {
        let mut activity = ActivityTimes::default();
        update_activity_from_jsonl(
            br#"{"timestamp":"2026-08-06T12:00:00Z","type":"user","message":{"content":"Ship it"}}"#,
            &mut activity,
        );
        update_activity_from_jsonl(
            br#"{"timestamp":"2026-08-06T12:01:00Z","type":"user","message":{"content":[{"type":"tool_result"}]}}"#,
            &mut activity,
        );
        update_activity_from_jsonl(
            br#"{"timestamp":"2026-08-06T12:02:00Z","type":"assistant","message":{"stop_reason":"tool_use"}}"#,
            &mut activity,
        );
        update_activity_from_jsonl(
            br#"{"timestamp":"2026-08-06T12:03:00Z","type":"assistant","message":{"stop_reason":"end_turn"}}"#,
            &mut activity,
        );
        assert_eq!(
            activity.prompted_at,
            chrono::DateTime::parse_from_rfc3339("2026-08-06T12:00:00Z")
                .unwrap()
                .timestamp() as u64
        );
        assert_eq!(
            activity.completed_at,
            chrono::DateTime::parse_from_rfc3339("2026-08-06T12:03:00Z")
                .unwrap()
                .timestamp() as u64
        );
    }

    #[test]
    fn pr_header_keeps_the_pr_title_and_each_session_gets_its_own_sub_row() {
        let mut entries = titled_entries();
        let mut newer = entries[0].sessions[0].clone();
        newer.session_id = "newer-session".to_string();
        newer.platform = PlatformKind::Codex;
        newer.dir_name = "other-worktree".to_string();
        newer.title = "⟁  A newer terminal title".to_string();
        newer.title_set_at = now_ms() - 30 * 60 * 1000;
        newer.recap = None;
        entries[0].sessions.push(newer);

        let frame = render_frame_at(&entries, 0);
        assert!(frame.contains("Make PR titles official"));
        assert!(frame.contains("A newer terminal title"));
        let plain = crate::parsers::strip_ansi_for_debug(&frame);
        let title_row = plain
            .lines()
            .find(|line| line.contains("Make PR titles official"))
            .unwrap();
        assert!(title_row.contains("#9: Make PR titles official"), "{plain}");
        let generated_row = plain
            .lines()
            .find(|line| line.contains("⟁  A newer terminal title"))
            .unwrap();
        assert!(!generated_row.contains("9:"), "{plain}");
        assert!(!frame.contains('⌾'));
        assert!(
            frame.contains("Wiring the PR board detail levels"),
            "every touching session keeps its own sub-row"
        );

        for session in &mut entries[0].sessions {
            session.title.clear();
        }
        entries[0].slack_threads.clear();
        let fallback = render_frame_at(&entries, 0);
        assert!(fallback.contains("Make PR titles official"));
        assert!(
            crate::parsers::strip_ansi_for_debug(&fallback).contains("#9: Make PR titles official")
        );
        assert!(!fallback.contains('⌾'));
        assert!(
            render_frame_at(&entries, 1).contains("Added r-toggled"),
            "recap remains available without a title: {}",
            crate::parsers::strip_ansi_for_debug(&render_frame_at(&entries, 1))
        );
    }
}
