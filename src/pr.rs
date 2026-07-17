//! GitHub PR tracking for the recap.
//!
//! Screen-scrapes the session's turn transcript for signs that *this* session
//! created or updated a pull request (`gh pr create`, `git push`,
//! `gh pr ready|edit|merge`), then enriches each PR with live details from the
//! GitHub CLI (`gh pr view`) on a background thread. The resulting list is
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

/// A pull request created or updated during this session.
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
#[derive(Clone, Debug)]
struct PrLocation {
    owner: String,
    repo: String,
    number: u64,
    url: String,
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
    /// a create until the next prompt ends the turn, which stops PR URLs merely
    /// referenced in a later prompt from being adopted.
    pub fn on_new_prompt(&mut self) {
        self.expect_created_since = None;
    }

    /// Scan a chunk of transcript/activity text (one turn's worth).
    ///
    /// Detects PR-mutating commands and PR URLs, kicks off `gh` enrichment for
    /// anything newly tracked, and returns true when the visible PR list changed
    /// (a placeholder was added). Enrichment results land later via [`poll`].
    pub fn scan_text(&mut self, text: &str, cwd: &Path) -> bool {
        if text.is_empty() {
            return false;
        }
        let mut changed = false;

        for line in text.lines() {
            // Command signals. `gh pr create` claims the next PR URL as created here;
            // pushes / PR edits trigger a branch-based lookup for an existing PR.
            if line.contains("gh pr create") {
                self.expect_created_since = Some(Instant::now());
            }
            if is_pr_update_command(line) {
                self.resolve_branch_pr(cwd);
            }

            for caps in pr_url_re().captures_iter(line) {
                let loc = PrLocation {
                    owner: caps[1].to_string(),
                    repo: caps[2].to_string(),
                    number: caps[3].parse().unwrap_or(0),
                    url: caps[0].to_string(),
                };
                changed |= self.observe_url(&loc);
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

        // New URL. Under the "created/updated by us" scope we only adopt it while a
        // `gh pr create` claim window is open (see `on_new_prompt`). Bare references
        // outside that window are ignored. The window stays open for the rest of the
        // turn so every PR from a batch of creates is captured.
        let claiming = self
            .expect_created_since
            .map(|t| t.elapsed() < CREATE_CLAIM_WINDOW)
            .unwrap_or(false);
        if !claiming {
            return false;
        }

        self.prs.push(SessionPr::placeholder(loc, true));
        self.spawn_fetch(loc.url.clone(), true);
        true
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

    #[test]
    fn ignores_bare_pr_references() {
        let mut tracker = PrTracker::new();
        // A referenced PR URL with no create/push command must not be tracked.
        let changed = tracker.scan_text(
            "see https://github.com/Tavus-Engineering/request-handler/pull/2371 for context",
            Path::new("/tmp"),
        );
        assert!(!changed);
        assert!(tracker.prs().is_empty());
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
    fn new_prompt_closes_claim_window() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("gh pr create", Path::new("/tmp"));
        // Next turn starts: a referenced PR URL must not be adopted.
        tracker.on_new_prompt();
        let changed = tracker.scan_text(
            "as discussed in https://github.com/o/r/pull/9\n",
            Path::new("/tmp"),
        );
        assert!(!changed);
        assert!(tracker.prs().is_empty());
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
