//! GitHub PR tracking for the recap.
//!
//! Screen-scrapes the session's turn transcript for pull requests the agent
//! mentions, creates, or updates, then enriches each PR with live details from
//! the GitHub CLI (`gh pr view`) on a background thread. The resulting list is
//! session-scoped and deduplicated by PR URL, so a single session working across
//! several PRs (e.g. an RQH PR and a dev portal PR) shows all of them.
//!
//! Detection is platform-agnostic: the caller feeds in latest-turn transcript
//! text (via `collect_latest_turn_text`, which handles both Claude and Codex), so
//! it works the same for both. `gh pr view <url>` is invoked with the full PR URL,
//! which encodes owner/repo/number, so refreshes are independent of the current
//! working directory (handy across worktrees).

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Minimum time between `gh pr view` refreshes for a single PR.
const REFRESH_THROTTLE: Duration = Duration::from_secs(30);
/// Safety cap on how long a `gh pr create` keeps claiming PR URLs, in case no
/// new prompt arrives to close the window. Normally closed by `on_new_prompt`.
const CREATE_CLAIM_WINDOW: Duration = Duration::from_secs(600);

fn pr_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/pull/(\d+)")
            .expect("valid PR url regex")
    })
}

fn pr_number_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"#(\d+)\b").expect("valid PR number regex"))
}

/// Uppercase words that precede a `#123` in prose without naming a repository.
/// Without this, `SEV #2` / `TODO #3` would be mistaken for repo shorthand.
const NON_REPO_ACRONYMS: &[&str] = &[
    "TODO", "FIXME", "FIX", "XXX", "HACK", "NOTE", "BUG", "WIP", "TBD", "SEV", "RFC", "ADR", "NB",
    "ETA", "EOD", "ID", "STEP", "ITEM", "Q", "CVE", "SLA", "P", "TASK", "ISSUE", "TICKET",
];

fn gh_pr_url_target_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?is)\bgh\s+pr\s+(?:view|checks|edit|ready|merge|reopen|close|comment|diff)\s+(?:--repo\s+\S+\s+)?https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/pull/(\d+)",
        )
        .expect("valid gh pr url target regex")
    })
}

fn repo_qualified_gh_pr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?is)\bgh\s+pr\s+(?:view|checks|edit|ready|merge|reopen|close)\s+#?(\d+)\b.{0,500}?(?:--repo|-R)\s+([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)",
        )
        .expect("valid repo-qualified gh pr regex")
    })
}

/// A pull request associated with this session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPr {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub url: String,
    /// Head branch name (`headRefName`). Empty until the first `gh` enrichment lands.
    pub branch: String,
    pub title: String,
    /// One of OPEN / MERGED / CLOSED (uppercase, as `gh` reports it).
    pub state: String,
    pub is_draft: bool,
    pub additions: i64,
    pub deletions: i64,
    pub changed_files: i64,
    /// GitHub mergeability: MERGEABLE / CONFLICTING / UNKNOWN.
    pub mergeable: String,
    /// GitHub merge state: CLEAN / DIRTY / BEHIND / BLOCKED / UNSTABLE / … .
    pub merge_state_status: String,
    /// CI check rollup counts derived from `statusCheckRollup`.
    pub checks_passed: i64,
    pub checks_failed: i64,
    pub checks_pending: i64,
    pub checks_total: i64,
    /// True when we saw `gh pr create` produce it; false when it was updated
    /// (pushed to / edited) but created elsewhere or in a prior session.
    pub created_here: bool,
    /// Unix ms of the last successful `gh` refresh (0 = never enriched yet).
    pub refreshed_at: u64,
}

impl SessionPr {
    fn placeholder(loc: &PrLocation, created_here: bool) -> Self {
        Self {
            number: loc.number,
            owner: loc.owner.clone(),
            repo: loc.repo.clone(),
            url: loc.url.clone(),
            branch: String::new(),
            title: String::new(),
            state: String::new(),
            is_draft: false,
            additions: 0,
            deletions: 0,
            changed_files: 0,
            mergeable: String::new(),
            merge_state_status: String::new(),
            checks_passed: 0,
            checks_failed: 0,
            checks_pending: 0,
            checks_total: 0,
            created_here,
            refreshed_at: 0,
        }
    }
}

/// Parsed owner/repo/number identity for a PR URL.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PrLocation {
    owner: String,
    repo: String,
    number: u64,
    url: String,
}

impl PrLocation {
    fn new(owner: &str, repo: &str, number: u64) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            number,
            url: format!("https://github.com/{owner}/{repo}/pull/{number}"),
        }
    }
}

/// Which part of a turn transcript a chunk of text came from.
///
/// Detection trusts what the user and the agent *say*, plus the PR commands that
/// actually ran. It does not trust tool output: a single `gh pr list` or `git log`
/// prints every recent PR in the repo, and adopting those made unrelated PRs look
/// like session work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Channel {
    /// User prompt or assistant prose.
    Prose,
    /// A tool invocation — the command line itself.
    Tool,
    /// Tool output.
    ToolResult,
}

/// What marked a `#123` in prose as a pull request.
#[derive(Clone, PartialEq, Eq, Debug)]
enum PrMarker {
    /// `PR #123`, `pull request #123`, `RQH #2469`, `developer-portal#123` — the
    /// repository isn't pinned, so the session's own repo is tried.
    Unqualified,
    /// `owner/repo#123` — the repository is explicit, so no guessing is needed.
    Repo(String, String),
}

/// JSON shape returned by `gh pr view --json ...`.
#[derive(Debug, Deserialize)]
struct GhPrJson {
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default, rename = "headRefName")]
    head_ref_name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    state: String,
    #[serde(default, rename = "isDraft")]
    is_draft: bool,
    #[serde(default)]
    additions: i64,
    #[serde(default)]
    deletions: i64,
    #[serde(default, rename = "changedFiles")]
    changed_files: i64,
    #[serde(default)]
    mergeable: String,
    #[serde(default, rename = "mergeStateStatus")]
    merge_state_status: String,
    #[serde(default, rename = "statusCheckRollup")]
    status_check_rollup: Vec<CheckEntry>,
}

/// One entry in `statusCheckRollup`: either a CheckRun (uses `status`/`conclusion`)
/// or a StatusContext (uses `state`).
#[derive(Debug, Deserialize)]
struct CheckEntry {
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum CheckClass {
    Pass,
    Fail,
    Pending,
}

impl CheckEntry {
    fn classify(&self) -> CheckClass {
        // Status contexts report `state`.
        if let Some(state) = &self.state {
            return match state.as_str() {
                "SUCCESS" => CheckClass::Pass,
                "FAILURE" | "ERROR" => CheckClass::Fail,
                _ => CheckClass::Pending, // PENDING / EXPECTED
            };
        }
        // Check runs report `status` (+ `conclusion` once COMPLETED).
        if self.status.as_deref() != Some("COMPLETED") {
            return CheckClass::Pending; // QUEUED / IN_PROGRESS / WAITING / …
        }
        match self.conclusion.as_deref() {
            Some("SUCCESS") | Some("NEUTRAL") | Some("SKIPPED") => CheckClass::Pass,
            Some("FAILURE") | Some("TIMED_OUT") | Some("CANCELLED") | Some("ACTION_REQUIRED")
            | Some("STARTUP_FAILURE") | Some("STALE") => CheckClass::Fail,
            _ => CheckClass::Pending,
        }
    }
}

/// Result of a background `gh` job.
struct FetchResult {
    /// Location we asked about (for placeholder identity / created_here carry-over).
    requested_url: Option<String>,
    created_here: bool,
    data: Result<GhPrJson, String>,
}

/// Owns PR detection, background enrichment, and the session-scoped list.
pub struct PrTracker {
    prs: Vec<SessionPr>,
    /// In-flight `gh` jobs keyed by the URL (or synthetic branch key) being fetched.
    pending: HashMap<String, mpsc::Receiver<FetchResult>>,
    /// Set when a `gh pr create` was seen; the next fresh PR URL is attributed to it.
    expect_created_since: Option<Instant>,
    /// Last time a `git push` / PR-edit triggered a current-branch PR lookup.
    /// Throttles branch resolution since the same turn text is re-scanned each tick.
    last_branch_resolve: Option<Instant>,
    /// Last attempt to resolve a bare `#123` mention in a working directory.
    /// The same latest-turn transcript is re-scanned on every hook tick.
    mention_resolved_at: HashMap<String, Instant>,
}

impl Default for PrTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl PrTracker {
    pub fn new() -> Self {
        Self {
            prs: Vec::new(),
            pending: HashMap::new(),
            expect_created_since: None,
            last_branch_resolve: None,
            mention_resolved_at: HashMap::new(),
        }
    }

    pub fn prs(&self) -> &[SessionPr] {
        &self.prs
    }

    /// Resolve the PR attached to the current branch in `cwd` and track it.
    ///
    /// Called on CLI startup and whenever the working directory / worktree changes,
    /// so the PR you're already working on shows up immediately — the same thing
    /// Claude Code surfaces as "PR #123" in its status line (via `gh pr view` on the
    /// current branch). Does nothing if the branch has no PR. Bypasses the
    /// push-scan throttle so a cwd switch resolves the new branch right away.
    pub fn resolve_current_branch(&mut self, cwd: &Path) {
        self.last_branch_resolve = None;
        self.resolve_branch_pr(cwd);
    }

    /// Close the "created here" claim window at the start of a new user turn.
    ///
    /// `gh pr create`'s own URL output is collapsed in the transcript, so the PR
    /// URLs only surface later in the assistant's summary text — and one visible
    /// create can produce several PRs. We therefore claim *every* new PR URL after
    /// a create until the next prompt ends the turn, so later references remain
    /// associated without being mislabeled as created by this session.
    pub fn on_new_prompt(&mut self) {
        self.expect_created_since = None;
    }

    /// Scan a chunk of transcript/activity text (one turn's worth).
    ///
    /// What counts as an association, by channel:
    /// - **prose** (user prompt, assistant text): PR URLs, and `#123` that carries
    ///   a PR marker (see [`pr_marker_before`]).
    /// - **tool commands**: the PR a `gh pr` subcommand targets — `gh pr view 2469
    ///   --repo owner/repo` or `gh pr checks <url>`. URLs merely quoted inside a
    ///   command (a `gh pr create --body` that cites related PRs) are not targets.
    /// - **tool output**: nothing, except the URL `gh pr create` prints for the PR
    ///   it just opened. Listings (`gh pr list`, `git log`, JSON dumps) name PRs
    ///   the session never touched.
    ///
    /// Bare numbers are validated against the current repository before being
    /// added. Enrichment results land later via [`poll`].
    pub fn scan_text(&mut self, text: &str, cwd: &Path) -> bool {
        if text.is_empty() {
            return false;
        }
        let mut changed = false;
        for event in scan_events(text) {
            match event {
                ScanEvent::Created => self.expect_created_since = Some(Instant::now()),
                ScanEvent::Updated => self.resolve_branch_pr(cwd),
                ScanEvent::Located(loc) => changed |= self.observe_url(&loc),
                // `gh pr view` rejects issue numbers and nonexistent PRs, so only
                // numbers that are really PRs in this repo reach the visible list.
                ScanEvent::Mentioned(number) => self.resolve_mentioned_pr(number, cwd),
            }
        }
        changed
    }

    /// Handle a PR URL seen in the scrollback.
    fn observe_url(&mut self, loc: &PrLocation) -> bool {
        // Already tracked → refresh its stats (it likely appeared because it was
        // touched again), but don't add a duplicate.
        if self.prs.iter().any(|p| p.url == loc.url) {
            self.refresh_url(&loc.url, false);
            return false;
        }

        // References are associated with the session whether they were created
        // here or merely discussed. The claim window only controls attribution.
        let created_here = self
            .expect_created_since
            .map(|t| t.elapsed() < CREATE_CLAIM_WINDOW)
            .unwrap_or(false);

        self.prs.push(SessionPr::placeholder(loc, created_here));
        self.spawn_fetch(loc.url.clone(), created_here);
        true
    }

    fn resolve_mentioned_pr(&mut self, number: u64, cwd: &Path) {
        if number == 0 {
            return;
        }
        // Already tracked from a source that named its repository. Prose nicknames
        // ("RQH #2499") don't map to a repo, so guessing this number against the
        // session's own repo could only find a different PR that happens to share it.
        if self.prs.iter().any(|pr| pr.number == number) {
            return;
        }
        let key = format!("mention:{}#{number}", cwd.display());
        if self.pending.contains_key(&key)
            || self
                .mention_resolved_at
                .get(&key)
                .map(|t| t.elapsed() < REFRESH_THROTTLE)
                .unwrap_or(false)
        {
            return;
        }
        self.mention_resolved_at.insert(key.clone(), Instant::now());
        let cwd = cwd.to_path_buf();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(FetchResult {
                requested_url: None,
                created_here: false,
                data: fetch_pr_number(&cwd, number),
            });
        });
        self.pending.insert(key, rx);
    }

    /// Refresh any tracked PRs whose stats are older than the throttle window.
    /// Called on turn completion. Returns true if a fetch was started (no visible
    /// change yet — results arrive via [`poll`]).
    pub fn refresh_stale(&mut self) -> bool {
        let now = now_unix_ms();
        let stale_urls: Vec<String> = self
            .prs
            .iter()
            .filter(|p| now.saturating_sub(p.refreshed_at) >= REFRESH_THROTTLE.as_millis() as u64)
            .map(|p| p.url.clone())
            .collect();
        let mut started = false;
        for url in stale_urls {
            started |= self.refresh_url(&url, false);
        }
        started
    }

    /// Start a `gh pr view` for a tracked URL unless one is already in flight or
    /// it was refreshed within the throttle window (bypassed when `force`).
    fn refresh_url(&mut self, url: &str, force: bool) -> bool {
        if self.pending.contains_key(url) {
            return false;
        }
        if !force {
            let now = now_unix_ms();
            if let Some(pr) = self.prs.iter().find(|p| p.url == url) {
                if pr.refreshed_at != 0
                    && now.saturating_sub(pr.refreshed_at) < REFRESH_THROTTLE.as_millis() as u64
                {
                    return false;
                }
            }
        }
        self.spawn_fetch(url.to_string(), false);
        false
    }

    /// Resolve the PR attached to the current branch in `cwd` (after a push/edit).
    /// Adds it as an "updated here" PR once `gh` reports back.
    fn resolve_branch_pr(&mut self, cwd: &Path) {
        // Throttle: the same turn text is re-scanned every hook tick, so a turn
        // containing a `git push` would otherwise spawn a lookup on every tick.
        if self
            .last_branch_resolve
            .map(|t| t.elapsed() < REFRESH_THROTTLE)
            .unwrap_or(false)
        {
            return;
        }
        let key = format!("branch:{}", cwd.display());
        if self.pending.contains_key(&key) {
            return;
        }
        self.last_branch_resolve = Some(Instant::now());
        let cwd = cwd.to_path_buf();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(FetchResult {
                requested_url: None,
                created_here: false,
                data: fetch_pr_for_branch(&cwd),
            });
        });
        self.pending.insert(key, rx);
    }

    /// Spawn a `gh pr view <url>` background job.
    fn spawn_fetch(&mut self, url: String, created_here: bool) {
        if self.pending.contains_key(&url) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let url_for_job = url.clone();
        std::thread::spawn(move || {
            let _ = tx.send(FetchResult {
                requested_url: Some(url_for_job.clone()),
                created_here,
                data: fetch_pr(&url_for_job),
            });
        });
        self.pending.insert(url, rx);
    }

    /// Collect any finished background jobs. Returns true if the visible list changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut done_keys = Vec::new();

        for (key, rx) in &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    done_keys.push((key.clone(), Some(result)));
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => done_keys.push((key.clone(), None)),
            }
        }

        for (key, result) in done_keys {
            self.pending.remove(&key);
            let Some(result) = result else { continue };
            if let Ok(json) = result.data {
                changed |= self.apply_fetch(json, result.requested_url, result.created_here);
            }
            // Errors are silent: a PR we can't view (auth, deleted, private) simply
            // doesn't gain live stats. If it was only a placeholder from a claim we
            // still keep it so the user sees the URL they just created.
        }

        changed
    }

    /// Merge a fetched PR into the list (update existing by URL, else insert).
    fn apply_fetch(
        &mut self,
        json: GhPrJson,
        requested_url: Option<String>,
        created_here: bool,
    ) -> bool {
        let url = if json.url.is_empty() {
            requested_url.unwrap_or_default()
        } else {
            json.url.clone()
        };
        if url.is_empty() {
            return false;
        }
        let (owner, repo) = split_owner_repo(&url).unwrap_or_default();
        let now = now_unix_ms();
        let (passed, failed, pending) = count_checks(&json.status_check_rollup);
        let total = passed + failed + pending;

        if let Some(existing) = self.prs.iter_mut().find(|p| p.url == url) {
            let before = existing.clone();
            existing.number = json.number;
            existing.branch = json.head_ref_name;
            existing.title = json.title;
            existing.state = json.state;
            existing.is_draft = json.is_draft;
            existing.additions = json.additions;
            existing.deletions = json.deletions;
            existing.changed_files = json.changed_files;
            existing.mergeable = json.mergeable;
            existing.merge_state_status = json.merge_state_status;
            existing.checks_passed = passed;
            existing.checks_failed = failed;
            existing.checks_pending = pending;
            existing.checks_total = total;
            existing.refreshed_at = now;
            return *existing != before;
        }

        self.prs.push(SessionPr {
            number: json.number,
            owner,
            repo,
            url,
            branch: json.head_ref_name,
            title: json.title,
            state: json.state,
            is_draft: json.is_draft,
            additions: json.additions,
            deletions: json.deletions,
            changed_files: json.changed_files,
            mergeable: json.mergeable,
            merge_state_status: json.merge_state_status,
            checks_passed: passed,
            checks_failed: failed,
            checks_pending: pending,
            checks_total: total,
            created_here,
            refreshed_at: now,
        });
        true
    }
}

/// Tally a `statusCheckRollup` into (passed, failed, pending) counts.
fn count_checks(rollup: &[CheckEntry]) -> (i64, i64, i64) {
    let (mut passed, mut failed, mut pending) = (0i64, 0i64, 0i64);
    for entry in rollup {
        match entry.classify() {
            CheckClass::Pass => passed += 1,
            CheckClass::Fail => failed += 1,
            CheckClass::Pending => pending += 1,
        }
    }
    (passed, failed, pending)
}

/// Whether a scrollback line looks like a command that updates an existing PR.
fn is_pr_update_command(line: &str) -> bool {
    // A push to a branch, or an explicit PR state change via gh.
    (line.contains("git push") && !line.contains("--delete"))
        || line.contains("gh pr ready")
        || line.contains("gh pr edit")
        || line.contains("gh pr merge")
        || line.contains("gh pr reopen")
}

/// Something a transcript scan found, in the order it was written.
#[derive(Clone, PartialEq, Eq, Debug)]
enum ScanEvent {
    /// A PR identified by repository and number.
    Located(PrLocation),
    /// A marked `#123` with no repository — resolved against the session's repo.
    Mentioned(u64),
    /// `gh pr create` ran, so PRs seen after it were created by this session.
    Created,
    /// A push or PR edit ran, so the current branch's PR is worth resolving.
    Updated,
}

/// Find every PR association in a chunk of transcript text.
///
/// Pure, so the channel rules can be tested (and replayed over real transcripts)
/// without running `gh`. See [`PrTracker::scan_text`] for the rules themselves.
fn scan_events(text: &str) -> Vec<ScanEvent> {
    let mut events = Vec::new();
    // Set while the section that just ran was a `gh pr create`, so its output
    // (and only its output) is read for the new PR's URL.
    let mut expect_create_output = false;

    for (channel, body) in split_channels(text) {
        match channel {
            Channel::Prose | Channel::Tool => {
                if channel == Channel::Tool {
                    expect_create_output = body.contains("gh pr create");
                }
                for line in body.lines() {
                    if line.contains("gh pr create") {
                        events.push(ScanEvent::Created);
                    }
                    if is_pr_update_command(line) {
                        events.push(ScanEvent::Updated);
                    }
                    if channel == Channel::Prose {
                        push_located(&mut events, pr_urls(line));
                        push_prose_mentions(&mut events, line);
                    }
                }
                // Raw scrollback arrives unmarked, so commands land in prose too.
                push_located(&mut events, gh_pr_targets(&body));
            }
            Channel::ToolResult => {
                if expect_create_output {
                    for line in body.lines() {
                        push_located(&mut events, pr_urls(line));
                    }
                }
            }
        }
    }
    events
}

fn push_located(events: &mut Vec<ScanEvent>, locations: Vec<PrLocation>) {
    for loc in locations {
        let event = ScanEvent::Located(loc);
        if !events.contains(&event) {
            events.push(event);
        }
    }
}

/// Record the marked `#123` mentions in one line of prose.
fn push_prose_mentions(events: &mut Vec<ScanEvent>, line: &str) {
    // A `#123` inside a PR link is part of the URL (or a review-thread anchor),
    // and the URL scan already covers it.
    if line.contains("/pull/") {
        return;
    }
    let mut mentions = Vec::new();
    line_pr_mentions(line, &mut mentions);
    for (marker, number) in mentions {
        let event = match marker {
            // A repository-pinned mention needs no guessing.
            PrMarker::Repo(owner, repo) => {
                ScanEvent::Located(PrLocation::new(&owner, &repo, number))
            }
            PrMarker::Unqualified => ScanEvent::Mentioned(number),
        };
        if !events.contains(&event) {
            events.push(event);
        }
    }
}

/// Every PR URL in a line of prose (or `gh pr create` output).
fn pr_urls(line: &str) -> Vec<PrLocation> {
    pr_url_re()
        .captures_iter(line)
        .map(|caps| PrLocation {
            owner: caps[1].to_string(),
            repo: caps[2].to_string(),
            number: caps[3].parse().unwrap_or(0),
            url: caps[0].to_string(),
        })
        .collect()
}

/// The PRs that `gh pr` subcommands in a section act on.
///
/// A Codex tool call often carries the identity only as `gh pr view 2469 --repo
/// owner/repo`, without printing a PR URL. URLs merely quoted elsewhere in the
/// command — a `gh pr create --body` citing related PRs — are not targets.
fn gh_pr_targets(section: &str) -> Vec<PrLocation> {
    let numbered = repo_qualified_gh_pr_re()
        .captures_iter(section)
        .map(|caps| PrLocation::new(&caps[2], &caps[3], caps[1].parse().unwrap_or(0)));
    let by_url = gh_pr_url_target_re()
        .captures_iter(section)
        .map(|caps| PrLocation::new(&caps[1], &caps[2], caps[3].parse().unwrap_or(0)));
    numbered.chain(by_url).collect()
}

/// Split transcript text into `(channel, body)` chunks on the `[assistant]` /
/// `[tool]` / `[tool_result]` markers that `collect_latest_turn_text` emits.
///
/// Text before the first marker — raw scrollback, a user prompt, or a plain chunk
/// from a unit test — is prose.
fn split_channels(text: &str) -> Vec<(Channel, String)> {
    let mut chunks: Vec<(Channel, String)> = Vec::new();
    let mut channel = Channel::Prose;
    let mut body = String::new();

    for line in text.lines() {
        let next = match line.trim() {
            "[assistant]" | "[user]" | "[prompt]" => Channel::Prose,
            "[tool]" => Channel::Tool,
            "[tool_result]" => Channel::ToolResult,
            _ => {
                body.push_str(line);
                body.push('\n');
                continue;
            }
        };
        if !body.is_empty() {
            chunks.push((channel, std::mem::take(&mut body)));
        }
        channel = next;
    }
    if !body.is_empty() {
        chunks.push((channel, body));
    }
    chunks
}

/// Collect marked PR mentions from one line of prose.
///
/// A match either carries its own marker (`PR #972`) or continues a comma/`and`
/// run started by one (`PRs #100, #101 and #102`). Anything else — including the
/// numbers in `Skipped #2, #3, and #5` — is skipped, and skipping also breaks the
/// run so a trailing list can't attach to an earlier marker.
fn line_pr_mentions(line: &str, out: &mut Vec<(PrMarker, u64)>) {
    // Byte offset just past the previously accepted `#N`, while a run is open.
    let mut run: Option<(usize, PrMarker)> = None;

    for caps in pr_number_re().captures_iter(line) {
        let matched = caps.get(0).expect("group 0 always present");
        let Ok(number) = caps[1].parse::<u64>() else {
            continue;
        };
        let marker = match pr_marker_before(&line[..matched.start()]) {
            Some(marker) => Some(marker),
            // `PRs #100, #101` — inherit the run's marker.
            None => run
                .as_ref()
                .filter(|(end, _)| is_number_list_gap(&line[*end..matched.start()]))
                .map(|(_, marker)| marker.clone()),
        };
        let Some(marker) = marker else {
            run = None;
            continue;
        };
        run = Some((matched.end(), marker.clone()));
        let mention = (marker, number);
        if !out.contains(&mention) {
            out.push(mention);
        }
    }
}

/// Whether the text right before a `#123` marks it as a pull request, and whether
/// that marker pins the repository.
///
/// Accepts `PR #123` / `PRs #123` / `pull request #123`, GitHub shorthand
/// (`owner/repo#123`, `developer-portal#123`), and short uppercase repo
/// nicknames (`RQH #2469`) that aren't conventional prose markers.
///
/// A bare `#123` is deliberately *not* enough: agents number their own findings
/// (`#1 Chat-mode hot mic`, `Skipped #2, #3, and #5`), and those numbers resolve
/// against any repository large enough to have them, so every list item would
/// otherwise become a tracked PR.
fn pr_marker_before(before: &str) -> Option<PrMarker> {
    // GitHub shorthand binds tightly: no space between the repo and the `#`.
    if !before.ends_with(char::is_whitespace) {
        if let Some(marker) = trailing_repo_ref(before) {
            return Some(marker);
        }
    }

    let head = before.trim_end_matches(|c: char| c.is_whitespace() || c == ':');
    let word = trailing_word(head);
    if word.is_empty() {
        return None;
    }
    if word.eq_ignore_ascii_case("pr") || word.eq_ignore_ascii_case("prs") {
        return Some(PrMarker::Unqualified);
    }
    if word.eq_ignore_ascii_case("request") || word.eq_ignore_ascii_case("requests") {
        let preceding = &head[..head.len() - word.len()];
        return trailing_word(preceding.trim_end())
            .eq_ignore_ascii_case("pull")
            .then_some(PrMarker::Unqualified);
    }
    // A repo nickname such as `RQH #2469`.
    let is_nickname = (2..=6).contains(&word.len())
        && word.chars().all(|c| c.is_ascii_uppercase())
        && !NON_REPO_ACRONYMS.contains(&word);
    is_nickname.then_some(PrMarker::Unqualified)
}

/// The trailing run of alphabetic characters in `text` (empty if it ends otherwise).
fn trailing_word(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphabetic())
        .last()
        .map(|(i, _)| i);
    start.map_or("", |i| &text[i..])
}

/// Read a repository reference off the end of `text` — `owner/repo` (which pins
/// the repository) or a single hyphenated/dotted name such as `developer-portal`
/// (which doesn't, since the owner is unknown).
fn trailing_repo_ref(text: &str) -> Option<PrMarker> {
    let start = text
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
        .last()
        .map(|(i, _)| i)?;
    let token = &text[start..];
    if let Some((owner, repo)) = token.rsplit_once('/') {
        if owner.is_empty() || repo.is_empty() || owner.contains('/') {
            return None;
        }
        return Some(PrMarker::Repo(owner.to_string(), repo.to_string()));
    }
    (token.len() >= 3 && token.contains(['-', '_', '.'])).then_some(PrMarker::Unqualified)
}

/// Whether the text between two `#N` matches is only list punctuation, so the
/// second number continues the first one's run (`#100, #101 and #102`).
fn is_number_list_gap(gap: &str) -> bool {
    gap.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty())
        .all(|token| matches!(token.to_ascii_lowercase().as_str(), "and" | "or" | "&" | "+"))
}

const GH_JSON_FIELDS: &str = "number,title,headRefName,url,state,isDraft,additions,deletions,\
    changedFiles,mergeable,mergeStateStatus,statusCheckRollup";

/// GitHub computes `mergeable` lazily, so the first read often returns UNKNOWN.
/// Re-query up to this many times (with a short sleep) to get a resolved value.
const MERGEABLE_RETRIES: usize = 3;
const MERGEABLE_RETRY_DELAY: Duration = Duration::from_millis(1800);

/// `gh pr view <url> --json ...` — cwd-independent (URL carries owner/repo).
fn fetch_pr(url: &str) -> Result<GhPrJson, String> {
    run_gh_pr_view(&["pr", "view", url, "--json", GH_JSON_FIELDS], None)
}

/// `gh pr view --json ...` run in `cwd` — resolves the current branch's PR.
fn fetch_pr_for_branch(cwd: &Path) -> Result<GhPrJson, String> {
    run_gh_pr_view(&["pr", "view", "--json", GH_JSON_FIELDS], Some(cwd))
}

/// `gh pr view <number>` in `cwd` — validates a natural-language `#123`
/// reference against the current repository.
fn fetch_pr_number(cwd: &Path, number: u64) -> Result<GhPrJson, String> {
    let number = number.to_string();
    run_gh_pr_view(
        &["pr", "view", &number, "--json", GH_JSON_FIELDS],
        Some(cwd),
    )
}

/// Run `gh pr view`, retrying while `mergeable` is UNKNOWN (GitHub is still
/// computing it). Runs on a background thread, so the sleeps don't block the UI.
fn run_gh_pr_view(args: &[&str], cwd: Option<&Path>) -> Result<GhPrJson, String> {
    let mut last: Option<GhPrJson> = None;
    for attempt in 0..=MERGEABLE_RETRIES {
        let mut cmd = Command::new("gh");
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let output = cmd.output().map_err(|e| format!("gh not runnable: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let json: GhPrJson =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("gh json parse: {e}"))?;
        let resolved = json.mergeable != "UNKNOWN";
        last = Some(json);
        if resolved || attempt == MERGEABLE_RETRIES {
            break;
        }
        std::thread::sleep(MERGEABLE_RETRY_DELAY);
    }
    last.ok_or_else(|| "gh returned no data".to_string())
}

fn split_owner_repo(url: &str) -> Option<(String, String)> {
    let caps = pr_url_re().captures(url)?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PR numbers a chunk of text mentions in prose, in order.
    fn prose_pr_numbers(text: &str) -> Vec<u64> {
        prose_pr_mentions(text).into_iter().map(|(_, n)| n).collect()
    }

    fn prose_pr_mentions(text: &str) -> Vec<(PrMarker, u64)> {
        let mut mentions = Vec::new();
        for (channel, body) in split_channels(text) {
            if channel != Channel::Prose {
                continue;
            }
            for line in body.lines().filter(|l| !l.contains("/pull/")) {
                line_pr_mentions(line, &mut mentions);
            }
        }
        mentions
    }

    #[test]
    fn adopts_pr_url_references_without_marking_them_created_here() {
        let mut tracker = PrTracker::new();
        let changed = tracker.scan_text(
            "see https://github.com/Tavus-Engineering/request-handler/pull/2371 for context",
            Path::new("/tmp"),
        );
        assert!(changed);
        assert_eq!(tracker.prs().len(), 1);
        assert!(!tracker.prs()[0].created_here);
    }

    #[test]
    fn adopts_pr_after_create_command() {
        let mut tracker = PrTracker::new();
        let scrollback = "● Bash(gh pr create --draft --title \"feat: thing\")\n\
             https://github.com/Tavus-Engineering/developer-portal/pull/955\n";
        let changed = tracker.scan_text(scrollback, Path::new("/tmp"));
        assert!(changed);
        assert_eq!(tracker.prs().len(), 1);
        let pr = &tracker.prs()[0];
        assert_eq!(pr.number, 955);
        assert_eq!(pr.owner, "Tavus-Engineering");
        assert_eq!(pr.repo, "developer-portal");
        assert!(pr.created_here);
    }

    #[test]
    fn create_claims_all_urls_in_the_turn() {
        // One visible `gh pr create` can produce several PRs whose URLs only show
        // up later in the assistant's summary — all of them should be adopted.
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "gh pr create\nsummary: https://github.com/o/r/pull/1 and https://github.com/o/r/pull/2\n",
            Path::new("/tmp"),
        );
        assert_eq!(tracker.prs().len(), 2);
        assert_eq!(tracker.prs()[0].number, 1);
        assert_eq!(tracker.prs()[1].number, 2);
    }

    #[test]
    fn new_prompt_stops_marking_references_created_here() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("gh pr create", Path::new("/tmp"));
        // Next turn starts: a referenced PR is still associated, but not claimed.
        tracker.on_new_prompt();
        let changed = tracker.scan_text(
            "as discussed in https://github.com/o/r/pull/9\n",
            Path::new("/tmp"),
        );
        assert!(changed);
        assert_eq!(tracker.prs().len(), 1);
        assert!(!tracker.prs()[0].created_here);
    }

    #[test]
    fn adopts_repo_qualified_gh_pr_references() {
        let mut tracker = PrTracker::new();
        let changed = tracker.scan_text(
            "gh pr view 2469 --repo Tavus-Engineering/request-handler --json url",
            Path::new("/tmp"),
        );
        assert!(changed);
        let pr = &tracker.prs()[0];
        assert_eq!(pr.number, 2469);
        assert_eq!(pr.repo, "request-handler");
        assert!(!pr.created_here);
    }

    #[test]
    fn extracts_numbers_from_assistant_prose_only() {
        let transcript = "[assistant]\nI’ll update RQH #2469 and PR #988.\n\
                          [tool_result]\nHistorical PR #111\n\
                          [assistant]\nSee https://github.com/o/r/pull/7 as well.";
        assert_eq!(prose_pr_numbers(transcript), vec![2469, 988]);
    }

    /// The regression from the fan-out session: an agent's own numbered findings
    /// (`#1`…`#6`) each resolved to a real merged PR in the repo.
    #[test]
    fn requires_a_pr_marker_on_bare_numbers() {
        let prose = "#1 Chat-mode hot mic — fixed in three layers:\n\
                     - BuilderCall.tsx — startAudioOff now includes !mediaActive\n\
                     #4 First-mention artifact explanations: the fan-out closeout nudge\n\
                     #6 End-call offer: the Platform-capabilities rule now allows one offer\n\
                     Skipped #2, #3, and #5 per your call, and I've noted those.\n\
                     Two loose ends from the original thread if you want them chased.";
        assert_eq!(prose_pr_numbers(prose), Vec::<u64>::new());
        // Unmarked numbers are dropped even in prose that discusses the code.
        assert_eq!(prose_pr_numbers("preserve #988 as-is"), Vec::<u64>::new());
    }

    #[test]
    fn accepts_marked_numbers() {
        assert_eq!(prose_pr_numbers("PR #972 is green"), vec![972]);
        assert_eq!(prose_pr_numbers("pull request #972 landed"), vec![972]);
        assert_eq!(prose_pr_numbers("PR: #972 needs a rebase"), vec![972]);
        assert_eq!(prose_pr_numbers("(see PR #972)"), vec![972]);
        assert_eq!(prose_pr_numbers("developer-portal#972 conflicts"), vec![972]);
        assert_eq!(prose_pr_numbers("RQH #2469 is next"), vec![2469]);
        // Conventional prose markers are not repo nicknames.
        assert_eq!(prose_pr_numbers("SEV #2 postmortem"), Vec::<u64>::new());
        assert_eq!(prose_pr_numbers("TODO #3 later"), Vec::<u64>::new());
        assert_eq!(prose_pr_numbers("PROD-1234 #5 is unrelated"), Vec::<u64>::new());
    }

    /// `owner/repo#123` pins the repository, so it never has to be guessed
    /// against the session's cwd.
    #[test]
    fn owner_repo_shorthand_pins_the_repository() {
        assert_eq!(
            prose_pr_mentions("Tavus-Engineering/developer-portal#972 conflicts"),
            vec![(
                PrMarker::Repo("Tavus-Engineering".into(), "developer-portal".into()),
                972
            )]
        );
        // A bare repo name has no owner, so it still resolves against the cwd.
        assert_eq!(
            prose_pr_mentions("developer-portal#972"),
            vec![(PrMarker::Unqualified, 972)]
        );
    }

    #[test]
    fn follows_marked_number_runs_until_prose_breaks_them() {
        assert_eq!(
            prose_pr_numbers("PRs #100, #101 and #102 are stacked"),
            vec![100, 101, 102]
        );
        // The run ends at the first non-list word, so the later list is ignored.
        assert_eq!(
            prose_pr_numbers("PR #100, #101 but skipped #2, #3"),
            vec![100, 101]
        );
    }

    /// The regression from the four-repo preview session: a `gh pr list` table and
    /// a `git log` naming five merged RQH PRs turned all of them into session PRs.
    #[test]
    fn ignores_prs_that_only_appear_in_tool_output() {
        let mut tracker = PrTracker::new();
        let transcript = "\n[assistant]\nChecking recent request-handler history.\n\
            \n[tool]\nexec_command {\"cmd\":\"gh pr list --repo Tavus-Engineering/request-handler --limit 5\"}\n\
            \n[tool_result]\n\
            2494\tci: Add :dev: reaction\troey/dev-reaction\tMERGED\thttps://github.com/Tavus-Engineering/request-handler/pull/2494\n\
            2492\tfeat: allow raven-1.5\tyonatan/allow-raven-1-5\tMERGED\thttps://github.com/Tavus-Engineering/request-handler/pull/2492\n\
            2481\tfeat: Expose sleep_phrase\troey/sleep-phrase\tMERGED\thttps://github.com/Tavus-Engineering/request-handler/pull/2481\n\
            69a790ca ci: Add :dev: reaction, ECS only on source file changes (#2494)\n";
        assert!(!tracker.scan_text(transcript, Path::new("/tmp")));
        assert!(tracker.prs().is_empty());
    }

    /// A `gh pr create --body` that cites related PRs is describing context, not
    /// touching those PRs — only the created PR's own URL counts.
    #[test]
    fn create_adopts_its_own_output_but_not_cited_prs() {
        let mut tracker = PrTracker::new();
        let transcript = "\n[tool]\nexec_command {\"cmd\":\"gh pr create --title docs --body \
            'This aligns with [RQH #2475](https://github.com/Tavus-Engineering/request-handler/pull/2475).'\"}\n\
            \n[tool_result]\nhttps://github.com/Tavus-Engineering/developer-portal/pull/1011\n";
        assert!(tracker.scan_text(transcript, Path::new("/tmp")));
        assert_eq!(tracker.prs().len(), 1);
        assert_eq!(tracker.prs()[0].number, 1011);
        assert!(tracker.prs()[0].created_here);
    }

    #[test]
    fn adopts_prs_a_gh_command_targets() {
        let mut tracker = PrTracker::new();
        let transcript = "\n[tool]\nexec_command {\"cmd\":\"gh pr ready 1011 --repo Tavus-Engineering/developer-portal\"}\n\
            \n[tool]\nexec_command {\"cmd\":\"gh pr checks https://github.com/Tavus-Engineering/tavus-api/pull/1073\"}\n";
        assert!(tracker.scan_text(transcript, Path::new("/tmp")));
        let numbers: Vec<u64> = tracker.prs().iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![1011, 1073]);
    }

    /// `RQH #2499` names a repo the tracker can't resolve, so once #2499 is known
    /// from a URL it must not also be looked up in the session's own repo, where
    /// that number could belong to something unrelated.
    #[test]
    fn skips_cwd_lookup_for_numbers_already_tracked() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "opened https://github.com/Tavus-Engineering/request-handler/pull/2499",
            Path::new("/tmp"),
        );
        tracker.scan_text("RQH #2499 still shows as open", Path::new("/tmp"));
        assert_eq!(tracker.prs().len(), 1);
        assert_eq!(tracker.prs()[0].repo, "request-handler");
        assert!(!tracker.pending.keys().any(|key| key.contains("mention:")));
    }

    /// A PR the user pastes into their prompt is session work, wherever it lives.
    #[test]
    fn adopts_pr_urls_from_a_user_prompt() {
        let mut tracker = PrTracker::new();
        let prompt = "Also, while we're here, look at \
            https://github.com/Tavus-Engineering/tavus-operator/pull/1509 and \
            https://github.com/Tavus-Engineering/tavus-api/pull/1073";
        assert!(tracker.scan_text(prompt, Path::new("/tmp")));
        let numbers: Vec<u64> = tracker.prs().iter().map(|p| p.number).collect();
        assert_eq!(numbers, vec![1509, 1073]);
    }

    /// Replay a real transcript through the scanner and print every PR it would
    /// associate, turn by turn — the audit that catches over-eager detection
    /// against a session whose widget you've actually looked at. Needs a local
    /// transcript, so it's ignored by default:
    ///   CRABIGATOR_TRANSCRIPT=~/.codex/sessions/2026/07/24/rollout-….jsonl \
    ///   CRABIGATOR_PLATFORM=codex \
    ///   cargo test pr::tests::replay_transcript -- --ignored --nocapture
    #[test]
    #[ignore]
    fn replay_transcript() {
        let Ok(path) = std::env::var("CRABIGATOR_TRANSCRIPT") else {
            panic!("set CRABIGATOR_TRANSCRIPT=<transcript.jsonl> (and CRABIGATOR_PLATFORM)");
        };
        let platform = match std::env::var("CRABIGATOR_PLATFORM").as_deref() {
            Ok("codex") => crate::platforms::PlatformKind::Codex,
            _ => crate::platforms::PlatformKind::Claude,
        };
        let content = std::fs::read_to_string(&path).expect("transcript is readable");
        let lines: Vec<&str> = content.lines().collect();

        // Each user message starts a turn; replaying the prefix that ends at the
        // next one reproduces what the app scanned while that turn was current.
        let mut boundaries: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| {
                line.contains(r#""role":"user""#) || line.contains(r#""type":"user""#)
            })
            .map(|(i, _)| i)
            .collect();
        boundaries.push(lines.len());

        let replay_path = std::env::temp_dir().join("crabigator-pr-replay.jsonl");
        let mut located: Vec<PrLocation> = Vec::new();
        let mut mentioned: Vec<u64> = Vec::new();
        let mut scanned_chars = 0usize;

        for (turn, end) in boundaries.iter().skip(1).enumerate() {
            std::fs::write(&replay_path, lines[..*end].join("\n")).expect("replay file written");
            let Ok(text) = crate::recap::collect_latest_turn_text(platform, Some(&replay_path))
            else {
                continue;
            };
            scanned_chars += text.activity.len() + text.user_prompt.as_deref().unwrap_or("").len();
            let mut events = scan_events(text.user_prompt.as_deref().unwrap_or_default());
            events.extend(scan_events(&text.activity));
            for event in events {
                match event {
                    ScanEvent::Located(loc) if !located.contains(&loc) => {
                        eprintln!("turn {turn}: located {}/{} #{}", loc.owner, loc.repo, loc.number);
                        located.push(loc);
                    }
                    // Mirrors `resolve_mentioned_pr`: a number already located with
                    // its own repository is never guessed against the cwd.
                    ScanEvent::Mentioned(number)
                        if !mentioned.contains(&number)
                            && !located.iter().any(|l| l.number == number) =>
                    {
                        eprintln!("turn {turn}: mentioned #{number} (resolved against cwd)");
                        mentioned.push(number);
                    }
                    _ => {}
                }
            }
        }
        let _ = std::fs::remove_file(&replay_path);

        let mut numbers: Vec<u64> = located.iter().map(|l| l.number).collect();
        numbers.sort_unstable();
        eprintln!(
            "\n{} turns, {scanned_chars} chars scanned\nlocated: {numbers:?}\nmentioned in cwd: {mentioned:?}",
            boundaries.len() - 1
        );
    }

    /// End-to-end against the live GitHub CLI: detect a created PR from scrollback,
    /// then confirm the background `gh pr view` enrichment fills in real branch/diff.
    /// Network + `gh` auth required, so it's ignored by default:
    ///   cargo test pr::tests::end_to_end_enriches_via_gh -- --ignored --nocapture
    #[test]
    #[ignore]
    fn end_to_end_enriches_via_gh() {
        let mut tracker = PrTracker::new();
        let scrollback = "● Bash(gh pr create --draft --title \"x\")\n\
            Opened https://github.com/Tavus-Engineering/request-handler/pull/2371\n";
        assert!(tracker.scan_text(scrollback, Path::new(".")));
        assert_eq!(tracker.prs().len(), 1);

        // Poll until the background gh job lands (up to ~10s).
        let mut enriched = false;
        for _ in 0..100 {
            tracker.poll();
            if !tracker.prs()[0].branch.is_empty() {
                enriched = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let pr = &tracker.prs()[0];
        eprintln!(
            "enriched PR: #{} branch={} state={} +{}-{} ({} files)",
            pr.number, pr.branch, pr.state, pr.additions, pr.deletions, pr.changed_files
        );
        assert!(enriched, "gh enrichment did not complete");
        assert_eq!(pr.branch, "ryan/builder-52-workers");
        assert!(pr.additions > 0);
    }

    #[test]
    fn apply_fetch_updates_existing() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation {
            owner: "o".into(),
            repo: "r".into(),
            number: 5,
            url: "https://github.com/o/r/pull/5".into(),
        };
        tracker.prs.push(SessionPr::placeholder(&loc, true));

        let json = GhPrJson {
            number: 5,
            title: "T".into(),
            head_ref_name: "feature/x".into(),
            url: loc.url.clone(),
            state: "OPEN".into(),
            is_draft: true,
            additions: 10,
            deletions: 3,
            changed_files: 2,
            mergeable: "MERGEABLE".into(),
            merge_state_status: "CLEAN".into(),
            status_check_rollup: vec![
                CheckEntry {
                    conclusion: Some("SUCCESS".into()),
                    status: Some("COMPLETED".into()),
                    state: None,
                },
                CheckEntry {
                    conclusion: Some("FAILURE".into()),
                    status: Some("COMPLETED".into()),
                    state: None,
                },
            ],
        };
        let changed = tracker.apply_fetch(json, Some(loc.url.clone()), true);
        assert!(changed);
        assert_eq!(tracker.prs().len(), 1);
        assert_eq!(tracker.prs()[0].mergeable, "MERGEABLE");
        assert_eq!(tracker.prs()[0].checks_passed, 1);
        assert_eq!(tracker.prs()[0].checks_failed, 1);
        assert_eq!(tracker.prs()[0].branch, "feature/x");
        assert_eq!(tracker.prs()[0].additions, 10);
    }
}
