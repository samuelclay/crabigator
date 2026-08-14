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
//! session mirrors under /tmp; `a` flips to the durable cloud record, which
//! includes ended sessions' PRs (tagged for resurrection) at ~1min lag.

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

use crate::platforms::SessionState;
use crate::pr::SessionPr;
use crate::pr_rank::PrDisposition;
use crate::slack::{SlackDirectory, SlackThread};
use crate::terminal::escape::{self, color, fg, RESET, RESET_FG};
use crate::ui::pr_cells::{
    board_detail_row_text, pr_row_text_with_activity, session_row_text_with_activity,
    PrColumnWidths, PR_COLUMN_GAP,
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
/// Detail levels `e`/`c` expand and collapse through: 0 = compact (one line
/// per PR), 1 = + title and Slack link lines, 2 = + one recap line.
const MAX_DETAIL: u8 = 2;
const DEFAULT_DETAIL: u8 = 0;

/// Recency uses one cyan-blue hue at steadily lower intensities until old
/// activity becomes neutral gray after a day.
const RECENCY_1H: u8 = 51;
const RECENCY_3H: u8 = 45;
const RECENCY_6H: u8 = 39;
const RECENCY_9H: u8 = 33;
const RECENCY_12H: u8 = 27;
const RECENCY_24H: u8 = 25;

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

    fn narrower(self) -> Self {
        match self {
            Self::Older => Self::LastDay,
            Self::LastDay => Self::LastTwelveHours,
            Self::LastTwelveHours => Self::LastNineHours,
            Self::LastNineHours => Self::LastSixHours,
            Self::LastSixHours => Self::LastThreeHours,
            Self::LastThreeHours | Self::LastHour => Self::LastHour,
        }
    }

    fn wider(self) -> Self {
        match self {
            Self::LastHour => Self::LastThreeHours,
            Self::LastThreeHours => Self::LastSixHours,
            Self::LastSixHours => Self::LastNineHours,
            Self::LastNineHours => Self::LastTwelveHours,
            Self::LastTwelveHours => Self::LastDay,
            Self::LastDay | Self::Older => Self::Older,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BoardView {
    detail: u8,
    oldest_visible_bucket: RecencyBucket,
}

impl BoardView {
    const fn new(detail: u8, oldest_visible_bucket: RecencyBucket) -> Self {
        Self {
            detail,
            oldest_visible_bucket,
        }
    }

    fn expand(&mut self) {
        if self.detail < MAX_DETAIL {
            self.detail += 1;
        } else {
            self.oldest_visible_bucket = self.oldest_visible_bucket.wider();
        }
    }

    fn collapse(&mut self) {
        if self.detail > DEFAULT_DETAIL {
            self.detail -= 1;
        } else {
            self.oldest_visible_bucket = self.oldest_visible_bucket.narrower();
        }
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
        1 => "title",
        2 => "recap",
        _ => "compact",
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct GhosttyTabRef {
    id: String,
    name: String,
}

/// One live session's contribution to the board.
struct SessionSnapshot {
    session_id: String,
    dir_name: String,
    repo_owner: String,
    repo_name: String,
    /// The session's /tmp mirror directory, where scrollback.log lives.
    session_dir: PathBuf,
    last_updated: f64,
    branch: String,
    uncommitted_files: usize,
    additions: i64,
    deletions: i64,
    /// The session's current terminal title (generated or OSC-published).
    title: String,
    /// Unix ms when the current title was set; 0 for legacy mirrors.
    title_set_at: u64,
    /// The live Ghostty tab captured when Crabigator started.
    ghostty_tab: Option<GhosttyTabRef>,
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
    dir_name: String,
    /// Local mirror directory holding scrollback.log; None for cloud records,
    /// whose transcripts aren't reachable from this machine.
    session_dir: Option<PathBuf>,
    /// The session's current terminal title (detail level 1+).
    title: String,
    /// Unix ms when the current title was set; 0 when unknown.
    title_set_at: u64,
    /// The session's latest recap (detail level 2).
    recap: Option<RecapBrief>,
    /// The session's current effective state.
    state: SessionState,
    /// Unix seconds when the session last received a prompt.
    prompted_at: u64,
    /// Unix seconds when the session's completion count last changed.
    completed_at: u64,
}

/// Active sessions in a repository/branch that has no matching visible PR row.
#[derive(Clone)]
struct WorkspaceEntry {
    repo_owner: String,
    repo_name: String,
    branch: String,
    session: SessionRef,
    ghostty_tab: Option<GhosttyTabRef>,
    uncommitted: usize,
    additions: i64,
    deletions: i64,
}

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
    let title_set_at = data
        .get("terminal_title_changed_at")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let mut slack_threads: Vec<SlackThread> = data
        .get("slack_threads")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
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
    let ghostty_tab = data.get("ghostty").and_then(|ghostty| {
        let id = ghostty.get("tab_id")?.as_str()?;
        let name = ghostty
            .get("tab_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        Some(GhosttyTabRef {
            id: id.to_string(),
            name: name.to_string(),
        })
    });
    Some(SessionSnapshot {
        session_id: data
            .get("cloud_session_id")
            .and_then(|v| v.as_str())
            .or_else(|| data.get("session_id").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string(),
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
        uncommitted_files: files.map_or(0, Vec::len),
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
        ghostty_tab,
        slack_threads,
        recap: latest_recap.and_then(recap_brief_from),
        state: mirror_session_state(&data),
        prompted_at: activity.prompted_at,
        completed_at: activity.completed_at,
        prs,
    })
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
                    slack_threads: Vec::new(),
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
                && repository_matches(&session.repo_owner, &session.repo_name, &pr.owner, &pr.repo)
                && (pr.created_here || (!pr.branch.is_empty() && session.branch == pr.branch));
            if represents_session {
                for thread in &session.slack_threads {
                    merge_slack_thread(&mut entry.slack_threads, thread.clone());
                }
                entry.stale &= session_age > STALE_SESSION_SECS;
                entry.sessions.push(SessionRef {
                    session_id: session.session_id.clone(),
                    dir_name: session.dir_name.clone(),
                    session_dir: Some(session.session_dir.clone()),
                    title: session.title.clone(),
                    title_set_at: session.title_set_at,
                    recap: session.recap.clone(),
                    state: session.state,
                    prompted_at: session.prompted_at,
                    completed_at: session.completed_at,
                });
            }
        }
    }

    let now_ms = (now * 1000.0) as u64;
    let mut out: Vec<BoardPr> = order
        .into_iter()
        .filter_map(|key| {
            let mut entry = merged.remove(&key)?;
            order_pr_slack_threads(&mut entry);
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

/// Keep one row for every active session that has no visible PR in its current
/// repository. Sessions remain distinct even when they share a branch.
fn local_workspaces(snapshots: &[SessionSnapshot], entries: &[BoardPr]) -> Vec<WorkspaceEntry> {
    let now = now_secs();
    let mut workspaces: Vec<WorkspaceEntry> = Vec::new();
    for snapshot in snapshots {
        let age_secs = (now - snapshot.last_updated).max(0.0) as u64;
        if age_secs > STALE_SESSION_SECS as u64 || session_has_matching_pr(snapshot, entries) {
            continue;
        }
        let session = SessionRef {
            session_id: snapshot.session_id.clone(),
            dir_name: snapshot.dir_name.clone(),
            session_dir: Some(snapshot.session_dir.clone()),
            title: snapshot.title.clone(),
            title_set_at: snapshot.title_set_at,
            recap: snapshot.recap.clone(),
            state: snapshot.state,
            prompted_at: snapshot.prompted_at,
            completed_at: snapshot.completed_at,
        };
        workspaces.push(WorkspaceEntry {
            repo_owner: snapshot.repo_owner.clone(),
            repo_name: snapshot.repo_name.clone(),
            branch: snapshot.branch.clone(),
            session,
            ghostty_tab: snapshot.ghostty_tab.clone(),
            uncommitted: snapshot.uncommitted_files,
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
    entries.iter().any(|entry| {
        repository_matches(repo_owner, repo_name, &entry.pr.owner, &entry.pr.repo)
            && entry.sessions.iter().any(|session| {
                if session_id.is_empty() {
                    session.session_id.is_empty() && session.dir_name == dir_name
                } else {
                    session.session_id == session_id
                }
            })
    })
}

fn repository_matches(owner: &str, repo: &str, other_owner: &str, other_repo: &str) -> bool {
    !repo.is_empty()
        && owner.eq_ignore_ascii_case(other_owner)
        && repo.eq_ignore_ascii_case(other_repo)
}

fn local_board(
    history: &mut ActivityHistory,
    overrides: &HashMap<String, PrDisposition>,
    linger_days: u64,
) -> Result<(Vec<BoardPr>, Vec<WorkspaceEntry>)> {
    let snapshots = gather(history)?;
    let mut entries = aggregate(&snapshots, overrides, linger_days);
    history.enrich_slack_threads(&mut entries);
    let workspaces = local_workspaces(&snapshots, &entries);
    Ok((entries, workspaces))
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
    let mut out: Vec<BoardPr> = cloud
        .prs
        .into_iter()
        .map(|entry| {
            let any_active = entry.sessions.iter().any(|s| s.active);
            let pr = entry.pr;
            let mut slack_threads = Vec::new();
            merge_pr_slack_threads(&mut slack_threads, &pr, &[]);
            let sessions = entry
                .sessions
                .into_iter()
                .map(|s| {
                    represented.insert(s.session_id.clone());
                    let state = parse_session_state(&s.state).unwrap_or(if s.active {
                        SessionState::Ready
                    } else {
                        SessionState::Complete
                    });
                    let recap = s.recap.and_then(|r| {
                        (!r.headline.is_empty()).then_some(RecapBrief {
                            headline: r.headline,
                            generated_at: r.generated_at,
                            line_delta: crate::recap::TurnLineDelta {
                                additions: r.additions,
                                deletions: r.deletions,
                            },
                        })
                    });
                    SessionRef {
                        session_id: s.session_id,
                        dir_name: s.dir_name,
                        session_dir: None,
                        title: s.title,
                        title_set_at: 0,
                        recap,
                        state,
                        prompted_at: activity_timestamp_secs(s.prompts_changed_at),
                        completed_at: activity_timestamp_secs(s.completions_changed_at),
                    }
                })
                .collect();
            BoardPr {
                pr,
                sessions,
                slack_threads,
                stale: !any_active,
            }
        })
        .collect();
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
                generated_at: recap.generated_at,
                line_delta: crate::recap::TurnLineDelta {
                    additions: recap.additions,
                    deletions: recap.deletions,
                },
            })
        });
        let session_ref = SessionRef {
            state: parse_session_state(&session.state).unwrap_or(SessionState::Ready),
            session_id: session.session_id,
            dir_name: session.dir_name,
            session_dir: None,
            title: session.title,
            title_set_at: 0,
            recap,
            prompted_at: activity_timestamp_secs(session.prompts_changed_at),
            completed_at: activity_timestamp_secs(session.completions_changed_at),
        };
        workspaces.push(WorkspaceEntry {
            repo_owner: session.repo_owner,
            repo_name,
            branch: session.branch,
            session: session_ref,
            ghostty_tab: None,
            uncommitted: 0,
            additions: 0,
            deletions: 0,
        });
    }
    sort_workspaces(&mut workspaces);
    (out, workspaces)
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
/// Each age keeps its own color because the two events can be hours apart.
fn activity_cell(sessions: &[SessionRef], now: u64, throbber_frame: usize) -> ActivityCell {
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
    let prompt = activity_part(PROMPT_ICON, prompted_at, now);
    let completion = activity_part(COMPLETION_ICON, completed_at, now);
    let state = activity_state(sessions)
        .map(|state| crate::ui::session_state_icon(state, throbber_frame))
        .unwrap_or_else(|| " ".to_string());
    ActivityCell {
        styled: format!("{}  {}  {}", state, prompt.styled, completion.styled),
        visible: 1 + 2 + prompt.visible + 2 + completion.visible,
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

fn board_has_thinking(entries: &[BoardPr], workspaces: &[WorkspaceEntry]) -> bool {
    entries
        .iter()
        .flat_map(|entry| entry.sessions.iter())
        .chain(workspaces.iter().map(|entry| &entry.session))
        .any(|session| session.state == SessionState::Thinking)
}

/// Sort each row by its newest prompt or completion. New prompts rise
/// immediately, and their completions move the row again when the turn ends.
fn activity_sort_time(sessions: &[SessionRef]) -> u64 {
    sessions
        .iter()
        .map(|session| session.prompted_at.max(session.completed_at))
        .max()
        .unwrap_or(0)
}

fn activity_bucket(sessions: &[SessionRef], now: u64) -> RecencyBucket {
    RecencyBucket::from_age(now.saturating_sub(activity_sort_time(sessions)))
}

fn activity_part(label: &str, timestamp: u64, now: u64) -> ActivityCell {
    let (plain, part_color) = if timestamp == 0 {
        (format!("{label} —"), color::DARK_GRAY)
    } else {
        let age = now.saturating_sub(timestamp);
        (format!("{label} {}", format_age(age)), recency_color(age))
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

fn ghostty_tab_text(tab: &GhosttyTabRef) -> String {
    let suffix = tab
        .id
        .chars()
        .rev()
        .take(5)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    if tab.name.is_empty() {
        format!("tab {suffix}")
    } else {
        format!("tab {} [{suffix}]", tab.name)
    }
}

fn latest_session_title(sessions: &[SessionRef]) -> Option<(&str, u64)> {
    sessions
        .iter()
        .filter_map(|session| {
            let plain = crate::title::strip_generated_title_marker(&session.title);
            (!plain.is_empty() && plain != session.dir_name)
                .then_some((session.title.as_str(), session.title_set_at))
        })
        .max_by_key(|(_, set_at)| *set_at)
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

fn latest_recap(sessions: &[SessionRef]) -> Option<&RecapBrief> {
    sessions
        .iter()
        .filter_map(|session| session.recap.as_ref())
        .max_by_key(|recap| recap.generated_at)
}

fn slack_link_cell(thread: &SlackThread, width: usize) -> ActivityCell {
    let full_label = crate::slack::thread_identity_label(thread);
    let label = crate::ui::pr_cells::truncate_to_width(&full_label, width);
    let visible = label.width();
    ActivityCell {
        styled: if label.is_empty() {
            String::new()
        } else {
            format!(
                "{}{}{}",
                fg(color::CYAN),
                escape::hyperlink(&thread.url, &label),
                RESET_FG
            )
        },
        visible,
    }
}

fn slack_links_width(threads: &[SlackThread]) -> usize {
    let mut seen = HashSet::new();
    threads
        .iter()
        .filter(|thread| seen.insert(thread.url.as_str()))
        .map(crate::slack::thread_identity_label)
        .map(|label| label.width())
        .max()
        .unwrap_or(0)
}

fn title_detail_lines(
    entry: &BoardPr,
    width: u16,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
) -> Vec<String> {
    let title = latest_session_title(&entry.sessions);
    let mut seen = HashSet::new();
    let mut links = entry
        .slack_threads
        .iter()
        .filter(|thread| seen.insert(thread.url.as_str()))
        .map(|thread| slack_link_cell(thread, widths.right_width()));
    let first_link = links.next();
    if title.is_none() && first_link.is_none() {
        return Vec::new();
    }

    let (left_styled, left_visible, timestamp) = title.map_or_else(
        || (String::new(), 0, None),
        |(title, set_at)| {
            let plain = crate::ui::pr_cells::truncate_to_width(
                &format!("  ⌾ {title}"),
                widths.board_left_width(),
            );
            (
                format!("{}{}{}", fg(color::CYAN), plain, RESET_FG),
                plain.width(),
                Some(set_at),
            )
        },
    );
    let age = timestamp.map_or_else(
        || empty_cell(activity_width),
        |set_at| timestamp_cell(set_at, now_ms, activity_width),
    );
    let first_link = first_link.unwrap_or_else(|| empty_cell(0));
    let mut lines = vec![format!(
        "{}{}",
        board_detail_row_text(
            width,
            widths,
            left_styled,
            left_visible,
            age.styled,
            age.visible,
            activity_width,
            first_link.styled,
            first_link.visible,
        ),
        RESET
    )];
    lines.extend(links.map(|link| {
        format!(
            "{}{}",
            board_detail_row_text(
                width,
                widths,
                String::new(),
                0,
                String::new(),
                0,
                activity_width,
                link.styled,
                link.visible,
            ),
            RESET
        )
    }));
    lines
}

#[derive(Clone, Copy)]
enum RecapLineKind {
    Judgment,
    Recap,
}

impl RecapLineKind {
    fn style(self) -> (&'static str, u8) {
        match self {
            Self::Judgment => ("✦", color::YELLOW),
            Self::Recap => ("↪", color::DARK_GRAY),
        }
    }
}

fn recap_line(
    kind: RecapLineKind,
    headline: &str,
    recap: Option<&RecapBrief>,
    width: u16,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
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
    if prefix_width + delta_width >= widths.board_left_width() {
        delta.clear();
        delta_width = 0;
    }
    let headline = crate::ui::pr_cells::truncate_to_width(
        headline,
        widths
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
    let age = recap.map_or_else(
        || empty_cell(activity_width),
        |recap| timestamp_cell(recap.generated_at, now_ms, activity_width),
    );
    format!(
        "{}{}",
        board_detail_row_text(
            width,
            widths,
            left_styled,
            left_visible,
            age.styled,
            age.visible,
            activity_width,
            String::new(),
            0,
        ),
        RESET
    )
}

fn pr_recap_detail_line(
    entry: &BoardPr,
    width: u16,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
) -> Option<String> {
    let recap = latest_recap(&entry.sessions);
    if !entry.pr.ai_note.is_empty() && entry.pr.state == "OPEN" {
        return Some(recap_line(
            RecapLineKind::Judgment,
            &entry.pr.ai_note,
            recap,
            width,
            now_ms,
            activity_width,
            widths,
        ));
    }
    recap.map(|recap| {
        recap_line(
            RecapLineKind::Recap,
            &recap.headline,
            Some(recap),
            width,
            now_ms,
            activity_width,
            widths,
        )
    })
}

fn recap_detail_line(
    recap: &RecapBrief,
    width: u16,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
) -> String {
    recap_line(
        RecapLineKind::Recap,
        &recap.headline,
        Some(recap),
        width,
        now_ms,
        activity_width,
        widths,
    )
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

fn render_pr_board_row(
    board_row: &BoardRow<'_>,
    width: u16,
    detail: u8,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
    throbber_frame: usize,
) -> Vec<String> {
    let entry = board_row.entry;
    let activity = activity_cell(&entry.sessions, now_ms / 1000, throbber_frame);
    let mut row = pr_row_text_with_activity(
        width,
        &entry.pr,
        widths,
        activity.styled,
        activity.visible,
        activity_width,
    );
    if entry.stale {
        row = format!("{}{row}", fg(color::DARK_GRAY));
    }

    let mut lines = vec![format!("{row}{RESET}")];
    if detail == 0 {
        lines.extend(board_row.preview_lines.iter().cloned());
        return lines;
    }

    lines.extend(title_detail_lines(
        entry,
        width,
        now_ms,
        activity_width,
        widths,
    ));
    if detail >= 2 {
        if let Some(recap) = pr_recap_detail_line(entry, width, now_ms, activity_width, widths) {
            lines.push(recap);
        }
    }
    lines.extend(board_row.preview_lines.iter().cloned());
    lines
}

fn workspace_title(entry: &WorkspaceEntry) -> (String, u8) {
    let session = &entry.session;
    let plain = crate::title::strip_generated_title_marker(&session.title);
    let (title, title_color) = if plain.is_empty() {
        (session.dir_name.clone(), color::PURPLE)
    } else {
        (session.title.clone(), color::PURPLE)
    };
    let text = if let Some(tab) = &entry.ghostty_tab {
        format!("{title} · {}", ghostty_tab_text(tab))
    } else {
        title
    };
    (text, title_color)
}

fn workspace_diff_text(entry: &WorkspaceEntry) -> String {
    if entry.additions == 0 && entry.deletions == 0 {
        String::new()
    } else {
        format!("+{} -{}", entry.additions, entry.deletions)
    }
}

fn workspace_files_text(entry: &WorkspaceEntry) -> String {
    match entry.uncommitted {
        0 => String::new(),
        1 => "1 file".to_string(),
        count => format!("{count} files"),
    }
}

fn workspace_branch_text(entry: &WorkspaceEntry) -> String {
    let branch = if entry.branch.is_empty() {
        "(no branch)"
    } else {
        entry.branch.as_str()
    };
    format!("⎇ {branch}")
}

fn render_workspace_board_row(
    workspace_row: &WorkspaceRow<'_>,
    width: u16,
    detail: u8,
    now_ms: u64,
    activity_width: usize,
    widths: &PrColumnWidths,
    throbber_frame: usize,
) -> Vec<String> {
    let entry = workspace_row.entry;
    let (title, title_color) = workspace_title(entry);
    let branch = workspace_branch_text(entry);
    let files = workspace_files_text(entry);
    let activity = activity_cell(entry.sessions(), now_ms / 1000, throbber_frame);
    let row = session_row_text_with_activity(
        width,
        &title,
        title_color,
        entry.additions,
        entry.deletions,
        &files,
        &branch,
        widths,
        activity.styled,
        activity.visible,
        activity_width,
    );
    let mut lines = vec![format!("{row}{RESET}")];
    if detail >= 2 {
        if let Some(recap) = entry.session.recap.as_ref() {
            lines.push(recap_detail_line(
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
    workspace_rows: &[WorkspaceRow],
    width: u16,
    linger_days: u64,
    include_ended: bool,
    view: BoardView,
) -> Vec<String> {
    render_at(
        rows,
        workspace_rows,
        width,
        linger_days,
        include_ended,
        view,
        RenderState {
            now_ms: (now_secs() * 1000.0) as u64,
            loading: false,
        },
    )
}

#[derive(Clone, Copy)]
struct RenderState {
    now_ms: u64,
    loading: bool,
}

fn render_at(
    rows: &[BoardRow],
    workspace_rows: &[WorkspaceRow],
    width: u16,
    linger_days: u64,
    include_ended: bool,
    view: BoardView,
    state: RenderState,
) -> Vec<String> {
    let RenderState { now_ms, loading } = state;
    let BoardView {
        detail,
        oldest_visible_bucket,
    } = view;
    let now = now_ms / 1000;
    let throbber_frame = crate::ui::throbber_frame_index();
    let visible_row_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            (activity_bucket(&row.entry.sessions, now) <= oldest_visible_bucket).then_some(index)
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
        format!("{}all sessions{}", fg(color::YELLOW), fg(color::DARK_GRAY))
    } else {
        "live".to_string()
    };
    let detail_label = if detail == DEFAULT_DETAIL {
        format!(" · {}", detail_name(detail))
    } else {
        format!(
            " · {}{}{}",
            fg(color::YELLOW),
            detail_name(detail),
            fg(color::DARK_GRAY)
        )
    };
    let age_label = if oldest_visible_bucket == DEFAULT_OLDEST_VISIBLE_BUCKET {
        String::new()
    } else {
        format!(
            " · {}age ≤ {}{}",
            fg(color::YELLOW),
            oldest_visible_bucket.max_age_label(),
            fg(color::DARK_GRAY),
        )
    };

    let mut lines = vec![
        format!(
            "{}⑆ Crabigator PR board{}  {}{} PRs · {} sessions · {} · {}{}{} · ↑↓ scroll · / search · +/- days · e/c detail/age · a all · q quit{}",
            fg(color::PURPLE),
            RESET_FG,
            fg(color::DARK_GRAY),
            visible_row_indices.len(),
            session_count,
            source,
            window,
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
    if visible_row_indices.is_empty() && visible_workspace_indices.is_empty() {
        if loading {
            return lines;
        }
        lines.push(format!(
            "{}No live sessions or tracked PRs.{}",
            fg(color::GRAY),
            RESET_FG
        ));
        return lines;
    }

    let activity_width = visible_row_indices
        .iter()
        .map(|&index| activity_cell(&rows[index].entry.sessions, now, throbber_frame).visible)
        .chain(visible_workspace_indices.iter().map(|&index| {
            activity_cell(workspace_rows[index].entry.sessions(), now, throbber_frame).visible
        }))
        .max()
        .unwrap_or(0);
    let shared_width = (width as usize)
        .saturating_sub(activity_width)
        .saturating_sub(usize::from(activity_width > 0) * PR_COLUMN_GAP)
        .min(u16::MAX as usize) as u16;
    let pr_refs: Vec<&SessionPr> = visible_row_indices
        .iter()
        .map(|&index| &rows[index].entry.pr)
        .collect();
    let mut widths = PrColumnWidths::from_pr_refs(&pr_refs, shared_width as usize);
    for &index in &visible_workspace_indices {
        let entry = workspace_rows[index].entry;
        let (title, _) = workspace_title(entry);
        widths.include_board_row(
            &format!("◇ {title}"),
            &workspace_diff_text(entry),
            &workspace_files_text(entry),
            &workspace_branch_text(entry),
            shared_width as usize,
        );
    }
    for &index in &visible_row_indices {
        widths.include_board_detail_right(
            slack_links_width(&rows[index].entry.slack_threads),
            shared_width as usize,
        );
    }

    #[derive(Clone, Copy)]
    enum SectionRow {
        Pr(usize),
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
                activity_sort_time(&row.entry.sessions),
                SectionRow::Pr(index),
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
            repository.rows.sort_by_key(|row| std::cmp::Reverse(row.0));
        }
        section
            .repositories
            .sort_by(|a, b| b.rows[0].0.cmp(&a.rows[0].0));
    }
    sections.sort_by_key(|section| section.bucket);

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
                match row {
                    SectionRow::Pr(index) => lines.extend(render_pr_board_row(
                        &rows[index],
                        width,
                        detail,
                        now_ms,
                        activity_width,
                        &widths,
                        throbber_frame,
                    )),
                    SectionRow::Workspace(index) => lines.extend(render_workspace_board_row(
                        &workspace_rows[index],
                        width,
                        detail,
                        now_ms,
                        activity_width,
                        &widths,
                        throbber_frame,
                    )),
                }
            }
        }
    }
    lines
}

struct PrBoardTerminalGuard;

impl Drop for PrBoardTerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = execute!(out, DisableBracketedPaste, LeaveAlternateScreen);
        let _ = write!(out, "{}", escape::CURSOR_SHOW);
        let _ = out.flush();
        let _ = disable_raw_mode();
    }
}

/// Entry point for `crabigator prs`.
pub async fn run_prs_board(once: bool) -> Result<()> {
    if once {
        let overrides = crate::cloud::fetch_pr_overrides_standalone()
            .await
            .unwrap_or_default();
        let width = terminal_size().map(|(w, _)| w).unwrap_or(120);
        let mut activity_history = ActivityHistory::default();
        let (entries, workspaces) =
            local_board(&mut activity_history, &overrides, DEFAULT_LINGER_DAYS)?;
        let rows: Vec<BoardRow> = entries
            .iter()
            .map(|entry| BoardRow {
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
            &workspace_rows,
            width,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
        ) {
            println!("{line}");
        }
        return Ok(());
    }

    let mut out = stdout();
    enable_raw_mode()?;
    let _terminal_guard = PrBoardTerminalGuard;
    execute!(out, EnterAlternateScreen, EnableBracketedPaste)?;
    write!(out, "{}", escape::CURSOR_HIDE)?;
    out.flush()?;
    let mut overrides = HashMap::new();
    let overrides_fetch = start_overrides_fetch();
    board_loop(&mut out, &mut overrides, overrides_fetch).await
}

fn start_overrides_fetch() -> mpsc::Receiver<HashMap<String, PrDisposition>> {
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

fn save_board_preferences(
    include_ended: bool,
    linger_days: u64,
    oldest_visible_bucket: RecencyBucket,
) {
    let Ok(mut config) = crate::config::Config::load() else {
        return;
    };
    config.pr_board.include_ended = include_ended;
    config.pr_board.linger_days = linger_days;
    config.pr_board.oldest_visible_hours = oldest_visible_bucket.max_age_hours();
    let _ = config.save();
}

async fn board_loop(
    out: &mut std::io::Stdout,
    overrides: &mut HashMap<String, PrDisposition>,
    overrides_fetch: mpsc::Receiver<HashMap<String, PrDisposition>>,
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
    // Transcript search state: cached scrollbacks plus the Tab-toggled
    // context view for the inline previews.
    let mut transcripts = TranscriptCache::default();
    let mut activity_history = None;
    let mut initial_snapshots = Vec::new();
    let mut loading = true;
    let mut pending_overrides = Some(overrides_fetch);
    let mut overrides_fetched = Instant::now();
    let mut expanded = false;
    let preferences = crate::config::Config::load().unwrap_or_default().pr_board;
    let mut view = BoardView::new(
        DEFAULT_DETAIL,
        RecencyBucket::from_max_age_hours(preferences.oldest_visible_hours),
    );
    let mut linger_days = preferences.linger_days.min(MAX_LINGER_DAYS);
    let mut include_ended = preferences.include_ended;
    // Cloud fetches are throttled well below the local tick; toggling the
    // source or changing the day window forces one.
    let mut cloud_fetch_due = false;
    let mut cloud_fetched: Option<Instant> = None;
    let mut dirty = false;
    let mut needs_render = false;
    let mut last_throbber_frame = crate::ui::throbber_frame_index();

    let (initial_width, _) = terminal_size().unwrap_or((120, 40));
    let mut lines = render_at(
        &[],
        &[],
        initial_width,
        linger_days,
        include_ended,
        view,
        RenderState {
            now_ms: (now_secs() * 1000.0) as u64,
            loading: true,
        },
    );
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
                let (live_entries, live_workspaces) =
                    local_board(activity_history.as_mut().unwrap(), overrides, linger_days)?;
                if !durable_history_loaded {
                    entries = live_entries;
                }
                workspaces = durable_workspaces.clone();
                merge_live_workspaces(&mut workspaces, live_workspaces, &entries);
            } else {
                (entries, workspaces) =
                    local_board(activity_history.as_mut().unwrap(), overrides, linger_days)?;
            }
            activity_history
                .as_mut()
                .unwrap()
                .enrich_slack_threads(&mut entries);
            needs_render = true;
        }

        let animating = loading || board_has_thinking(&entries, &workspaces);
        let throbber_frame = crate::ui::throbber_frame_index();
        if animating && throbber_frame != last_throbber_frame {
            last_throbber_frame = throbber_frame;
            needs_render = true;
        }

        let (width, height) = terminal_size().unwrap_or((120, 40));
        if needs_render {
            needs_render = false;
            let query = search.as_deref().unwrap_or("");
            let now_ms = (now_secs() * 1000.0) as u64;
            let now = now_ms / 1000;
            // A PR stays visible when its metadata matches, or when any of
            // its sessions' transcripts contain the query — with the matched
            // excerpt shown inline so the hit can be confirmed.
            let filtered: Vec<BoardRow> = entries
                .iter()
                .filter_map(|entry| {
                    if activity_bucket(&entry.sessions, now) > view.oldest_visible_bucket {
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
            matched = filtered.len() + filtered_workspaces.len();
            let fresh = render_at(
                &filtered,
                &filtered_workspaces,
                width,
                linger_days,
                include_ended,
                view,
                RenderState { now_ms, loading },
            );
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
            let mut visible_lines = Vec::new();
            if let Some(query) = &search {
                visible_lines.push(search_banner(query, matched, width));
            }
            visible_lines.extend(lines.iter().skip(scroll).take(page).cloned());
            if draw_changed_lines(out, &mut drawn_lines, &visible_lines)? {
                out.flush()?;
            }
        }

        let poll_interval = if animating {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(250)
        };
        if poll(poll_interval)? {
            match read()? {
                Event::Paste(text) => {
                    // Bracketed paste arrives as one event. A paste outside
                    // search is inert instead of firing every matching
                    // shortcut in the pasted text.
                    if let Some(query) = &mut search {
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
                                save_board_preferences(
                                    include_ended,
                                    linger_days,
                                    view.oldest_visible_bucket,
                                );
                                cloud_fetch_due = true;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                linger_days = linger_days.saturating_sub(1);
                                save_board_preferences(
                                    include_ended,
                                    linger_days,
                                    view.oldest_visible_bucket,
                                );
                                cloud_fetch_due = true;
                                last_refresh = Instant::now() - REFRESH_INTERVAL;
                                last_frame_hash = 0;
                            }
                            // Detail changes first. At full detail, expanding
                            // restores one older recency bucket; at compact,
                            // collapsing hides the oldest visible bucket.
                            KeyCode::Char('e') => {
                                let previous_bucket = view.oldest_visible_bucket;
                                view.expand();
                                if view.oldest_visible_bucket != previous_bucket {
                                    save_board_preferences(
                                        include_ended,
                                        linger_days,
                                        view.oldest_visible_bucket,
                                    );
                                }
                                needs_render = true;
                                dirty = true;
                            }
                            KeyCode::Char('c') => {
                                let previous_bucket = view.oldest_visible_bucket;
                                view.collapse();
                                if view.oldest_visible_bucket != previous_bucket {
                                    save_board_preferences(
                                        include_ended,
                                        linger_days,
                                        view.oldest_visible_bucket,
                                    );
                                }
                                needs_render = true;
                                dirty = true;
                            }
                            // Flip between live mirrors and the durable cloud
                            // record, which includes ended sessions.
                            KeyCode::Char('a') => {
                                include_ended = !include_ended;
                                save_board_preferences(
                                    include_ended,
                                    linger_days,
                                    view.oldest_visible_bucket,
                                );
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
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::new(detail, oldest_visible_bucket),
        )
        .join("\n")
    }

    fn render_frame(entries: &[BoardPr]) -> String {
        render_frame_at(entries, DEFAULT_DETAIL)
    }

    fn now_ms() -> u64 {
        (now_secs() * 1000.0) as u64
    }

    fn snapshot(dir: &str, prs: Vec<SessionPr>) -> SessionSnapshot {
        SessionSnapshot {
            session_id: dir.to_string(),
            dir_name: dir.to_string(),
            repo_owner: "o".to_string(),
            repo_name: dir.to_string(),
            session_dir: PathBuf::new(),
            last_updated: now_secs(),
            branch: String::new(),
            uncommitted_files: 0,
            additions: 0,
            deletions: 0,
            title: String::new(),
            title_set_at: 0,
            ghostty_tab: None,
            slack_threads: Vec::new(),
            recap: None,
            state: SessionState::Ready,
            prompted_at: 0,
            completed_at: 0,
            prs,
        }
    }

    #[test]
    fn aggregation_merges_the_same_pr_across_sessions() {
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
        let entries = aggregate(&[two, one], &HashMap::new(), DEFAULT_LINGER_DAYS);
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.pr.mentions, 13, "mentions sum across sessions");
        assert_eq!(entry.pr.additions, 42, "newest gh stats win");
        assert_eq!(entry.sessions.len(), 2);
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

    #[test]
    fn loading_frame_is_truthful_before_sessions_arrive() {
        let loading = render_at(
            &[],
            &[],
            120,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
            RenderState {
                now_ms: now_ms(),
                loading: true,
            },
        )
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
        first.uncommitted_files = 3;
        first.additions = 10;
        first.deletions = 2;
        let mut second = snapshot("crabigator", Vec::new());
        second.session_id = "two".to_string();
        second.repo_owner = "SamuelClay".to_string();
        second.repo_name = "crabigator".to_string();
        second.branch = first.branch.clone();
        second.title = "Second active session".to_string();

        let snapshots = vec![first, second];
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
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
            &rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
        )
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
        assert!(frame.contains("3 files"));
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
                    "generated_at": 1_234_567
                }
            }]
        }))
        .unwrap();

        let (_, workspaces) = cloud_entries_to_board(cloud);
        assert_eq!(workspaces[0].session.title_set_at, 0);
    }

    #[test]
    fn all_mode_pr_represents_its_live_session_after_a_branch_change() {
        let mut pr = board_pr(7, "crabigator");
        make_primary(&mut pr);
        pr.branch = "old-branch".to_string();
        let mut durable = snapshot("crabigator", vec![pr]);
        durable.session_id = "cloud-session".to_string();
        let entries = aggregate(&[durable], &HashMap::new(), DEFAULT_LINGER_DAYS);

        let mut current = snapshot("crabigator", Vec::new());
        current.session_id = "cloud-session".to_string();
        current.branch = "new-branch".to_string();
        let live = local_workspaces(&[current], &[]);
        let mut workspaces = Vec::new();

        merge_live_workspaces(&mut workspaces, live, &entries);

        assert!(workspaces.is_empty());
    }

    #[test]
    fn repository_rows_share_columns_and_follow_completion_activity() {
        let now = now_secs() as u64;
        let mut crab_pr = board_pr(7, "crabigator");
        make_primary(&mut crab_pr);
        crab_pr.owner = "samuelclay".to_string();
        crab_pr.branch = "with-pr".to_string();
        let mut pr_session = snapshot("crabigator", vec![crab_pr]);
        pr_session.session_id = "with-pr".to_string();
        pr_session.repo_owner = "samuelclay".to_string();
        pr_session.repo_name = "crabigator".to_string();
        pr_session.branch = "with-pr".to_string();
        pr_session.completed_at = now - 120;

        let mut no_pr_session = snapshot("crabigator", Vec::new());
        no_pr_session.session_id = "without-pr".to_string();
        no_pr_session.repo_owner = "samuelclay".to_string();
        no_pr_session.repo_name = "crabigator".to_string();
        no_pr_session.branch = "main".to_string();
        no_pr_session.completed_at = now - 30;
        no_pr_session.title = "Newest completed session".to_string();

        let mut portal_pr = board_pr(9, "developer-portal");
        make_primary(&mut portal_pr);
        portal_pr.branch = "a-much-longer-branch-name".to_string();
        let mut portal_session = snapshot("developer-portal", vec![portal_pr]);
        portal_session.completed_at = now - 300;

        let snapshots = vec![pr_session, no_pr_session, portal_session];
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        let rows: Vec<BoardRow<'_>> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let workspace_rows: Vec<WorkspaceRow<'_>> = workspaces
            .iter()
            .map(|entry| WorkspaceRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let frame = crate::parsers::strip_ansi_for_debug(
            &render(
                &rows,
                &workspace_rows,
                160,
                DEFAULT_LINGER_DAYS,
                false,
                BoardView::default(),
            )
            .join("\n"),
        );

        assert_eq!(frame.matches("samuelclay/crabigator").count(), 1);
        let no_pr_offset = frame.find("◇ Newest completed session").unwrap();
        let pr_offset = frame.find("crabigator #7").unwrap();
        assert!(no_pr_offset < pr_offset, "newer completion sorts first");

        let no_pr_row = frame
            .lines()
            .find(|line| line.contains("◇ Newest completed session"))
            .unwrap();
        let crab_pr_row = frame
            .lines()
            .find(|line| line.contains("crabigator #7"))
            .unwrap();
        let portal_pr_row = frame
            .lines()
            .find(|line| line.contains("developer-portal #9"))
            .unwrap();
        let column = |line: &str, text: &str| {
            crate::ui::utils::strip_ansi_len(&line[..line.find(text).unwrap()])
        };
        assert_eq!(column(no_pr_row, "⟩"), column(crab_pr_row, "⟩"));
        assert_eq!(column(crab_pr_row, "⟩"), column(portal_pr_row, "⟩"));
        assert_eq!(column(no_pr_row, "main"), column(crab_pr_row, "with-pr"));
        assert_eq!(
            column(crab_pr_row, "with-pr"),
            column(portal_pr_row, "a-much-longer-branch-name")
        );
        assert!(!no_pr_row.contains("no tracked PR"));
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
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        let rows: Vec<BoardRow<'_>> = entries
            .iter()
            .map(|entry| BoardRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();
        let workspace_rows: Vec<WorkspaceRow<'_>> = workspaces
            .iter()
            .map(|entry| WorkspaceRow {
                entry,
                preview_lines: Vec::new(),
            })
            .collect();

        for detail in [0, MAX_DETAIL] {
            let lines: Vec<String> = render(
                &rows,
                &workspace_rows,
                160,
                DEFAULT_LINGER_DAYS,
                false,
                BoardView::new(detail, DEFAULT_OLDEST_VISIBLE_BUCKET),
            )
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
            let pr_row = lines
                .iter()
                .position(|line| line.contains("crabigator #2"))
                .unwrap();
            let peer_row = lines
                .iter()
                .position(|line| line.contains("◇ Peer session"))
                .unwrap();
            let blank_lines: Vec<usize> = lines[crabigator..]
                .iter()
                .enumerate()
                .filter_map(|(offset, line)| line.is_empty().then_some(crabigator + offset))
                .collect();

            assert!(crabigator < pr_row && pr_row < peer_row && peer_row < portal);
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
                session.completed_at = now - age;
                session
            })
            .collect();
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
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
    fn collapse_and_expand_move_from_detail_to_recency() {
        let mut view = BoardView::new(MAX_DETAIL, DEFAULT_OLDEST_VISIBLE_BUCKET);

        for expected in [
            (1, RecencyBucket::Older),
            (0, RecencyBucket::Older),
            (0, RecencyBucket::LastDay),
            (0, RecencyBucket::LastTwelveHours),
            (0, RecencyBucket::LastNineHours),
            (0, RecencyBucket::LastSixHours),
            (0, RecencyBucket::LastThreeHours),
            (0, RecencyBucket::LastHour),
        ] {
            view.collapse();
            assert_eq!((view.detail, view.oldest_visible_bucket), expected);
        }
        view.collapse();
        assert_eq!(
            (view.detail, view.oldest_visible_bucket),
            (0, RecencyBucket::LastHour)
        );

        for expected in [
            (1, RecencyBucket::LastHour),
            (2, RecencyBucket::LastHour),
            (2, RecencyBucket::LastThreeHours),
            (2, RecencyBucket::LastSixHours),
            (2, RecencyBucket::LastNineHours),
            (2, RecencyBucket::LastTwelveHours),
            (2, RecencyBucket::LastDay),
            (2, RecencyBucket::Older),
        ] {
            view.expand();
            assert_eq!((view.detail, view.oldest_visible_bucket), expected);
        }
        view.expand();
        assert_eq!(
            (view.detail, view.oldest_visible_bucket),
            (2, RecencyBucket::Older)
        );
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
            session.completed_at = now - age;
            session
        })
        .collect();
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);

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
        assert!(!six_hours.contains("repo #4"));

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
        assert!(last_hour.contains("repo #1"));
        assert!(!last_hour.contains("repo #2"));
    }

    #[test]
    fn sessions_with_visible_prs_do_not_get_duplicate_workspace_rows() {
        let mut pr = board_pr(1, "crabigator");
        make_primary(&mut pr);
        let session = snapshot("crabigator", vec![pr]);
        let snapshots = vec![session];
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
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
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);
        assert_eq!(entries.len(), 1);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].session.session_id, "secondary-session");
        assert_eq!(entries[0].sessions.len(), 1);

        let rows = [BoardRow {
            entry: &entries[0],
            preview_lines: Vec::new(),
        }];
        let workspace_rows = [WorkspaceRow {
            entry: &workspaces[0],
            preview_lines: Vec::new(),
        }];
        for detail in 0..=MAX_DETAIL {
            let frame = crate::parsers::strip_ansi_for_debug(
                &render(
                    &rows,
                    &workspace_rows,
                    160,
                    DEFAULT_LINGER_DAYS,
                    false,
                    BoardView::new(detail, DEFAULT_OLDEST_VISIBLE_BUCKET),
                )
                .join("\n"),
            );
            assert!(frame.contains("2 sessions"));
            let pr_row = frame.lines().find(|line| line.contains("#2")).unwrap();
            assert!(pr_row.contains(PROMPT_ICON));
            assert!(pr_row.contains(COMPLETION_ICON));
            let session_row = frame
                .lines()
                .find(|line| line.contains("◇ Independent session"))
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
    fn unrelated_visible_prs_do_not_hide_the_current_workspace() {
        let mut cross_repo_pr = board_pr(1, "portal");
        make_primary(&mut cross_repo_pr);
        cross_repo_pr.branch = "feature".to_string();
        let mut cross_repo = snapshot("crabigator", vec![cross_repo_pr]);
        cross_repo.session_id = "cross-repo".to_string();
        cross_repo.repo_name = "crabigator".to_string();
        cross_repo.branch = "feature".to_string();

        let mut other_branch_pr = board_pr(2, "crabigator");
        make_primary(&mut other_branch_pr);
        other_branch_pr.branch = "other".to_string();
        let mut other_branch = snapshot("crabigator", vec![other_branch_pr]);
        other_branch.session_id = "other-branch".to_string();
        other_branch.branch = "feature".to_string();

        let snapshots = vec![cross_repo, other_branch];
        let entries = aggregate(&snapshots, &HashMap::new(), DEFAULT_LINGER_DAYS);
        let workspaces = local_workspaces(&snapshots, &entries);

        assert_eq!(entries.len(), 2);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].session.session_id, "cross-repo");
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
    fn title_carries_slack_and_recap_skips_the_removed_status_line() {
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
        session.uncommitted_files = 4;
        session.slack_threads = vec![
            SlackThread {
                url: "https://t.slack.com/archives/C1/p1723500000000000".to_string(),
                posted_at: 1_723_500_000,
                channel: Some("builder".to_string()),
                author: Some("Sam Clay".to_string()),
            },
            SlackThread {
                url: "https://t.slack.com/archives/C2/p1723500000000001".to_string(),
                posted_at: 1_723_500_001,
                channel: Some("pr-reviews".to_string()),
                author: Some("Mango".to_string()),
            },
            SlackThread {
                url: "https://t.slack.com/archives/C3/p1723500000000002".to_string(),
                posted_at: 1_723_500_002,
                channel: Some("builder-dev".to_string()),
                author: Some("Kapil".to_string()),
            },
        ];
        let entries = aggregate(&[session], &HashMap::new(), DEFAULT_LINGER_DAYS);
        let title = render_frame_at(&entries, 1);
        assert!(title.contains("Builder Signals"));
        assert!(
            title.contains("#builder"),
            "{}",
            crate::parsers::strip_ansi_for_debug(&title)
        );
        assert!(title.contains("https://t.slack.com/archives/C1/p1723500000000000"));
        assert!(title.contains("https://t.slack.com/archives/C2/p1723500000000001"));
        assert!(title.contains("https://t.slack.com/archives/C3/p1723500000000002"));
        assert!(
            title.find("archives/C1").unwrap() < title.find("archives/C2").unwrap(),
            "the original thread renders first"
        );
        assert_eq!(
            title
                .lines()
                .filter(|line| line.contains("slack.com"))
                .count(),
            3,
            "each Slack target gets a clickable row"
        );
        assert!(title.contains("2h"));
        assert!(!title.contains("CI green, awaiting review"));
        for removed in [
            "slack origin",
            "mentions",
            "spoken",
            "uncommitted",
            "▓",
            "%",
        ] {
            assert!(!title.contains(removed), "removed status text: {removed}");
        }

        let recap = render_frame_at(&entries, 2);
        assert!(recap.contains("CI green, awaiting review"));
        let judgment_line = recap
            .lines()
            .find(|line| line.contains("CI green, awaiting review"))
            .unwrap();
        assert!(!judgment_line.contains("#builder"));
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
                dir_name: "portal".to_string(),
                session_dir: Some(dir.path().to_path_buf()),
                title: String::new(),
                title_set_at: 0,
                recap: None,
                state: SessionState::Ready,
                prompted_at: 0,
                completed_at: 0,
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
        pr.mergeable = "MERGEABLE".to_string();
        let mut with_title = snapshot("portal", vec![pr.clone()]);
        with_title.title = "Wiring the PR board detail levels".to_string();
        with_title.title_set_at = now_ms() - 2 * 60 * 60 * 1000;
        with_title.prompted_at = now_secs() as u64 - 30 * 60;
        with_title.completed_at = now_secs() as u64 - 2 * 60 * 60;
        with_title.recap = Some(RecapBrief {
            headline: "Added e-cycled detail to the prs board".to_string(),
            generated_at: now_ms() - 42 * 60 * 1000,
            line_delta: crate::recap::TurnLineDelta {
                additions: 120,
                deletions: 35,
            },
        });
        let mut bare = snapshot("other-dir", vec![pr]);
        bare.prompted_at = now_secs() as u64 - 4 * 60 * 60;
        bare.completed_at = now_secs() as u64 - 5 * 60 * 60;
        aggregate(&[with_title, bare], &HashMap::new(), DEFAULT_LINGER_DAYS)
    }

    /// `e` steps through compact, title, and recap. Each step adds at most one
    /// line per PR, and expanded timestamps stay in the activity column.
    #[test]
    fn detail_levels_reveal_titles_then_recaps() {
        let entries = titled_entries();

        let compact = render_frame(&entries);
        assert!(!compact.contains('▓'), "no progress bar at compact");
        assert!(!compact.contains("Wiring the PR board"), "no titles");
        assert!(compact.contains("#9"), "identity row stays");
        assert!(compact.contains("compact"), "header names the level");
        assert!(compact.contains("⟩ 30m"));
        assert!(compact.contains("⋖ 2h"));

        let title = render_frame_at(&entries, 1);
        assert!(title.contains("Wiring the PR board"));
        assert!(
            crate::parsers::strip_ansi_for_debug(&title).contains(" · title"),
            "header uses the level name"
        );
        assert!(title.contains("⟩ 30m"));
        assert!(title.contains("⋖ 2h"));
        assert!(
            !title.contains("Added e-cycled detail"),
            "recaps wait for level 2"
        );

        let recaps = render_frame_at(&entries, 2);
        assert!(recaps.contains("Wiring the PR board"));
        assert!(recaps.contains("Added e-cycled"));
        // The recap line carries the turn's diff and compact age.
        assert!(recaps.contains("+120"), "recap shows the line delta");
        assert!(recaps.contains("-35"));
        assert!(recaps.contains("42m"), "recap shows its age");
        assert!(!recaps.contains("ago"), "expanded ages stay compact");
        assert!(
            crate::parsers::strip_ansi_for_debug(&recaps).contains(" · recap"),
            "header uses the level name"
        );
        assert!(recaps.contains("⟩ 30m"));
        assert!(recaps.contains("⋖ 2h"));

        let counts = [
            compact.lines().count(),
            title.lines().count(),
            recaps.lines().count(),
        ];
        assert_eq!(counts[1], counts[0] + 1, "title adds one line");
        assert_eq!(counts[2], counts[1] + 1, "recap adds one line");

        let compact_line = recaps.lines().find(|line| line.contains("#9")).unwrap();
        let title_line = recaps
            .lines()
            .find(|line| line.contains("Wiring the PR board"))
            .unwrap();
        let recap_line = recaps
            .lines()
            .find(|line| line.contains("Added e-cycled"))
            .unwrap();
        assert!(
            crate::parsers::strip_ansi_for_debug(title_line).starts_with("   ⌾ "),
            "title starts three spaces in"
        );
        assert!(
            crate::parsers::strip_ansi_for_debug(recap_line).starts_with("   ↪ "),
            "recap shares the title indentation"
        );
        let visible_end = |line: &str, needle: &str| {
            let byte = line.rfind(needle).unwrap();
            crate::ui::utils::strip_ansi_len(&line[..byte]) + needle.width()
        };
        assert_eq!(
            visible_end(compact_line, "2h"),
            visible_end(title_line, "2h"),
            "title age ends in the compact activity column"
        );
        assert_eq!(
            visible_end(compact_line, "2h"),
            visible_end(recap_line, "42m"),
            "recap age ends in the compact activity column"
        );

        let row = crate::parsers::strip_ansi_for_debug(&compact);
        assert!(
            row.find("⋖ 2h").unwrap() < row.find("open").unwrap(),
            "activity belongs before the GitHub status"
        );
        let main_row = compact.lines().find(|line| line.contains("#9")).unwrap();
        assert_eq!(
            crate::ui::utils::strip_ansi_len(main_row),
            159,
            "row keeps the right-edge padding"
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
    fn activity_uses_independent_recency_bands_and_latest_session_times() {
        let now = 1_000_000;
        let sessions = vec![
            SessionRef {
                session_id: "older".to_string(),
                dir_name: "older".to_string(),
                session_dir: None,
                title: String::new(),
                title_set_at: 0,
                recap: None,
                state: SessionState::Complete,
                prompted_at: now - 4 * 60 * 60,
                completed_at: now - 8 * 60 * 60,
            },
            SessionRef {
                session_id: "newer".to_string(),
                dir_name: "newer".to_string(),
                session_dir: None,
                title: String::new(),
                title_set_at: 0,
                recap: None,
                state: SessionState::Thinking,
                prompted_at: now - 30 * 60,
                completed_at: now - 2 * 60 * 60,
            },
        ];
        let activity = activity_cell(&sessions, now, 0);
        assert!(activity.styled.contains("⟩ 30m"));
        assert!(activity.styled.contains("⋖ 2h"));
        assert!(
            crate::parsers::strip_ansi_for_debug(&activity.styled).starts_with("⠋  ⟩"),
            "thinking state sits directly before prompt activity"
        );
        assert!(activity.styled.contains(&fg(RECENCY_1H)));
        assert!(activity.styled.contains(&fg(RECENCY_3H)));
        assert!(!activity.styled.contains("\x1b[48;5;"));

        assert_eq!(recency_color(0), RECENCY_1H);
        assert_eq!(recency_color(3_600), RECENCY_3H);
        assert_eq!(recency_color(10_800), RECENCY_6H);
        assert_eq!(recency_color(21_600), RECENCY_9H);
        assert_eq!(recency_color(32_400), RECENCY_12H);
        assert_eq!(recency_color(43_200), RECENCY_24H);
        assert_eq!(recency_color(86_400), RECENCY_24H);
        assert_eq!(recency_color(86_401), color::DARK_GRAY);

        let unknown = activity_cell(&[], now, 0);
        assert!(unknown.styled.contains("⟩ —"));
        assert!(unknown.styled.contains("⋖ —"));
        assert!(!unknown.styled.contains("\x1b[48;5;"));
    }

    #[test]
    fn activity_icons_cover_every_session_state_and_thinking_animates() {
        let cases = [
            (SessionState::Ready, "○", color::GRAY),
            (SessionState::Permission, "!", color::YELLOW),
            (SessionState::Question, "?", color::ORANGE),
            (SessionState::Complete, "✓", color::PURPLE),
            (SessionState::Interrupted, "⊘", color::RED),
        ];
        for (state, icon, icon_color) in cases {
            let session = SessionRef {
                session_id: "state".to_string(),
                dir_name: "state".to_string(),
                session_dir: None,
                title: String::new(),
                title_set_at: 0,
                recap: None,
                state,
                prompted_at: 1,
                completed_at: 1,
            };
            let activity = activity_cell(&[session], 1, 0);
            assert!(crate::parsers::strip_ansi_for_debug(&activity.styled)
                .starts_with(&format!("{icon}  ⟩")));
            assert!(activity.styled.contains(&fg(icon_color)));
        }

        let thinking = SessionRef {
            session_id: "thinking".to_string(),
            dir_name: "thinking".to_string(),
            session_dir: None,
            title: String::new(),
            title_set_at: 0,
            recap: None,
            state: SessionState::Thinking,
            prompted_at: 1,
            completed_at: 0,
        };
        let first = activity_cell(std::slice::from_ref(&thinking), 1, 0);
        let second = activity_cell(&[thinking], 1, 1);
        assert_ne!(
            crate::parsers::strip_ansi_for_debug(&first.styled),
            crate::parsers::strip_ansi_for_debug(&second.styled)
        );
        assert!(first.styled.contains(&fg(color::GREEN)));
    }

    #[test]
    fn recap_view_shows_standalone_session_recap_without_repeating_its_title() {
        let now = now_secs() as u64;
        let mut session = snapshot("crabigator", Vec::new());
        session.title = "⟁  Standalone work".to_string();
        session.recap = Some(RecapBrief {
            headline: "Kept standalone rows visible".to_string(),
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
            &rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::new(MAX_DETAIL, DEFAULT_OLDEST_VISIBLE_BUCKET),
        )
        .join("\n");
        assert!(styled.contains(&format!("{}◇ ⟁  Standalone work", fg(color::PURPLE))));
        assert!(styled.contains(&format!("{}Kept standalone rows visible", fg(color::GRAY))));
        let frame = crate::parsers::strip_ansi_for_debug(&styled);
        assert_eq!(frame.matches("Standalone work").count(), 1);
        assert!(frame.contains("⟁  Standalone work"));
        assert!(frame.contains("↪ Kept standalone rows visible"));
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
            &rows,
            160,
            DEFAULT_LINGER_DAYS,
            false,
            BoardView::default(),
        )
        .join("\n");
        assert!(styled.contains(&format!("{}◇ crabigator", fg(color::PURPLE))));
    }

    #[test]
    fn activity_sort_uses_the_newest_prompt_or_completion() {
        let now = 100_000;
        let mut session = SessionRef {
            session_id: "one".to_string(),
            dir_name: "crabigator".to_string(),
            session_dir: None,
            title: String::new(),
            title_set_at: 0,
            recap: None,
            state: SessionState::Ready,
            prompted_at: now - 8 * 60,
            completed_at: now - 13 * 60 * 60,
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
            session.completed_at
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

        assert_eq!(
            crate::slack::thread_identity_label(&snapshot.slack_threads[0]),
            "#pr-reviews · Mango"
        );
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
    fn title_expansion_uses_only_the_newest_title() {
        let mut entries = titled_entries();
        let mut newer = entries[0].sessions[0].clone();
        newer.session_id = "newer-session".to_string();
        newer.dir_name = "other-worktree".to_string();
        newer.title = "⟁  A newer terminal title".to_string();
        newer.title_set_at = now_ms() - 30 * 60 * 1000;
        newer.recap = None;
        entries[0].sessions.push(newer);

        let frame = render_frame_at(&entries, 1);
        assert!(frame.contains("A newer terminal title"));
        assert!(frame.contains("⌾ ⟁  A newer terminal title"));
        assert!(frame.contains(&format!("{}  ⌾ ⟁  A newer", fg(color::CYAN))));
        assert!(!frame.contains("Wiring the PR board detail levels"));
        assert!(!frame.contains("other-worktree"));

        for session in &mut entries[0].sessions {
            session.title.clear();
        }
        entries[0].slack_threads.clear();
        assert_eq!(
            render_frame_at(&entries, 1).lines().count(),
            render_frame_at(&entries, 0).lines().count(),
            "an empty title adds no blank row"
        );
        assert!(
            render_frame_at(&entries, 2).contains("Added e-cycled"),
            "recap remains available without a title: {}",
            crate::parsers::strip_ansi_for_debug(&render_frame_at(&entries, 2))
        );
    }
}
