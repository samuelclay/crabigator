//! GitHub PR tracking for the recap.
//!
//! Screen-scrapes the session's turn transcript for pull requests the agent
//! mentions, creates, or updates, then enriches each PR with live details from
//! the GitHub CLI on a background thread. The resulting list is session-scoped
//! and deduplicated by PR URL, so a single session working across several PRs
//! (e.g. an RQH PR and a dev portal PR) shows all of them.
//!
//! Detection is platform-agnostic: the caller feeds in latest-turn transcript
//! text (via `collect_latest_turn_text`, which handles both Claude and Codex), so
//! it works the same for both. Refreshes use the full PR URL, which encodes
//! owner/repo/number, so they are independent of the current working directory
//! (handy across worktrees).
//!
//! Background reads share one hourly budget across every session and PR board
//! on this machine (`pr::budget`). A session that has mentioned dozens of open
//! PRs keeps refreshing the handful it touched most recently, not every one.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod budget;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::pr_rank::PrDisposition;
use crate::slack::{extract_threads, has_only_channel_id, SlackDirectory, SlackThread};

/// Minimum time between `gh pr view` refreshes for a single PR, and between
/// repeated lookups of the current branch. Also the first step of an open
/// PR's backoff.
const REFRESH_THROTTLE: Duration = Duration::from_secs(30);
/// An open PR waits as long as it had been quiet at its last read, so reads
/// back off 30 s → 1 m → 2 m → 4 m → 8 m and settle at this cap. A change on
/// GitHub, a mention, or a push starts the backoff over (see [`refresh_due`]).
const OPEN_REFRESH_CAP: Duration = Duration::from_secs(10 * 60);
/// While checks run, their results are minutes away. A PR that moved within
/// `CI_FRESH_WINDOW` is read every `CI_FRESH_REFRESH`, and one that moved
/// within `CI_WINDOW` every 30 seconds. Past that, a check that never reports
/// would poll forever, so the PR falls back to the open backoff.
const CI_FRESH_WINDOW: Duration = Duration::from_secs(5 * 60);
const CI_FRESH_REFRESH: Duration = Duration::from_secs(15);
const CI_WINDOW: Duration = Duration::from_secs(60 * 60);
/// A merged PR can't change and a closed one rarely reopens.
const FINISHED_REFRESH: Duration = Duration::from_secs(60 * 60);
/// Never-loaded PRs past this many, counting from the most recently
/// mentioned, wait for recent work to go quiet before their first read.
const BACKGROUND_PR_LIMIT: usize = 8;
/// How many of the most recent open PRs count the session's own prompts and
/// completions as activity.
const HOT_SESSION_PR_LIMIT: usize = 2;
/// Reads one session will start in an hour, on top of the machine-wide budget.
const SESSION_READS_PER_HOUR: u32 = budget::SESSION_LIMIT;
/// Bare `PR #N` lookups that couldn't start immediately (a read was already
/// in flight, or the transcript was being replayed). Drained one at a time.
const DEFERRED_LOOKUP_LIMIT: usize = 8;
/// How many pasted-prompt URLs to keep for branch matching.
const PROMPT_URLS_KEPT: usize = 32;
/// How long a pasted Slack permalink keeps claiming new PRs as their origin.
const SLACK_ORIGIN_CLAIM_WINDOW: Duration = Duration::from_secs(600);
/// An untracked bare mention (`PR #7`, `RQH #12`) below this number never
/// adopts a new PR: small numbers false-match Docker build steps
/// (`#1 [internal] …`), docs anchors (`llm#1-model`), and numbered findings,
/// and any repository old enough has a PR to collide with. Repo-qualified
/// mentions (`owner/repo#12`), full URLs, `gh pr` commands, and refreshes of
/// already-tracked PRs are unaffected.
const MIN_BARE_PR_NUMBER: u64 = 100;

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

fn any_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"https?://[^\s)\]>'"]+"#).expect("valid url regex"))
}

/// `PR #123 … is (the) primary` / `#123 … secondary`.
fn decl_number_first_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:\bpr\s*#?|#)(\d+)\b[^\n.!?]{0,40}?\bis\s+(?:the\s+)?(primary|secondary)\b",
        )
        .expect("valid declaration regex")
    })
}

/// `the primary (PR) is #123`, `make … primary … #123`.
fn decl_keyword_first_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(primary|secondary)\b[^\n.!?]{0,40}?(?:\bpr\s*#?|#)(\d+)\b")
            .expect("valid declaration regex")
    })
}

/// `track PR <url>` / `watch <url>` — explicit watch-list adds by URL.
fn decl_watch_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:track|watch)\s+(?:pr\s+)?(https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/\d+)",
        )
        .expect("valid watch url regex")
    })
}

/// `owner/repo#123` on its own, as typed into a watch input.
fn watch_shorthand_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)#(\d+)$")
            .expect("valid watch shorthand regex")
    })
}

/// The same statement as `track owner/repo#123`.
fn decl_watch_repo_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:track|watch)\s+(?:pr\s+)?([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)#(\d+)\b")
            .expect("valid watch repo regex")
    })
}

/// `dismiss PR #123` — the verb must target the PR directly; a wider window
/// would turn "remove the flag from PR #123" into a dismissal.
fn decl_dismiss_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:dismiss|forget|untrack)\s+(?:pr\s*#?|#)(\d+)\b")
            .expect("valid dismissal regex")
    })
}

/// The same statement made with a full PR URL.
fn decl_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/\d+)[^\n.!?]{0,40}?\bis\s+(?:the\s+)?(primary|secondary)\b",
        )
        .expect("valid declaration regex")
    })
}

/// The disposition word the declaration regexes capture — only ever
/// `primary` or `secondary`, in any casing.
fn parse_disposition(word: &str) -> PrDisposition {
    if word.eq_ignore_ascii_case("primary") {
        PrDisposition::Primary
    } else {
        PrDisposition::Secondary
    }
}

/// Uppercase words that precede a `#123` in prose without naming a repository.
/// Without this, `SEV #2` / `TODO #3` would be mistaken for repo shorthand.
const NON_REPO_ACRONYMS: &[&str] = &[
    "TODO", "FIXME", "FIX", "XXX", "HACK", "NOTE", "BUG", "WIP", "TBD", "SEV", "RFC", "ADR", "NB",
    "ETA", "EOD", "ID", "STEP", "ITEM", "Q", "CVE", "SLA", "P", "TASK", "ISSUE", "TICKET",
];

fn gh_pr_command_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?is)(?:^|&&|\|\||[;\n|]|"(?:command|cmd|input)"\s*:\s*"|exec_command\s+)\s*gh\s+pr\s+(view|checks|edit|ready|merge|reopen|close|comment|diff)\b"#,
        )
        .expect("valid gh pr command regex")
    })
}

/// A pull request associated with this session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPr {
    pub number: u64,
    pub owner: String,
    pub repo: String,
    pub url: String,
    /// GitHub login that opened the PR. Empty for mirrors written before
    /// author tracking shipped or when GitHub no longer has an author.
    #[serde(default)]
    pub author_login: String,
    /// Whether `author_login` matches the GitHub account authenticated in `gh`.
    /// None means the comparison was unavailable, so legacy rows stay visible.
    #[serde(default)]
    pub authored_by_viewer: Option<bool>,
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
    /// Where the CI column points: the first failing job's own page when a check
    /// failed, else the PR's Checks tab. Empty when the PR has no checks yet.
    #[serde(default)]
    pub ci_url: String,
    /// Unresolved review threads — GitHub's "N unresolved conversations".
    /// Only queried while the PR is open, and cleared once it isn't.
    #[serde(default)]
    pub unresolved_comments: i64,
    /// Anchor of the first unresolved thread's comment, so the badge lands on
    /// the conversation itself rather than the top of the PR.
    #[serde(default)]
    pub comments_url: String,
    /// Unix ms of the last successful review-thread query (0 = never).
    #[serde(default)]
    pub comments_refreshed_at: u64,
    /// True when we saw `gh pr create` produce it; false when it was updated
    /// (pushed to / edited) but created elsewhere or in a prior session.
    pub created_here: bool,
    /// True when this session ran a `gh pr` command that changes the PR
    /// (`merge`, `ready`, `edit`, `close`, `reopen`). Viewing or diffing
    /// does not set this. It is ownership: the PR stays in the section even
    /// when another PR is mentioned more often.
    #[serde(default)]
    pub updated_here: bool,
    /// Counted mentions of this PR this session. Bulk listings — one line
    /// naming several tracked PRs, like `gh pr list` output quoted into
    /// prose — are excluded so a dump doesn't read as engagement.
    #[serde(default)]
    pub mentions: u64,
    /// Mentions that appeared in user-authored prompt text. The strongest
    /// primacy signal: a PR the human typed is almost always the work.
    #[serde(default)]
    pub user_mentions: u64,
    /// Unix ms of the first counted mention (0 = never mentioned).
    #[serde(default)]
    pub first_mentioned_at: u64,
    /// Unix ms of the most recent counted mention (0 = never mentioned).
    #[serde(default)]
    pub last_mentioned_at: u64,
    /// The session's prompt counter at the most recent counted mention.
    #[serde(default)]
    pub last_mention_prompt: u32,
    /// The PR's head branch matched the session's branch, its worktree
    /// directory name, or a URL pasted into a user prompt. Sticky once set.
    #[serde(default)]
    pub branch_matched: bool,
    /// GitHub review decision: APPROVED / CHANGES_REQUESTED / REVIEW_REQUIRED,
    /// or empty when no review is required or the value is unknown.
    #[serde(default)]
    pub review_decision: String,
    /// A review on the open PR was dismissed — usually an approval invalidated
    /// by a new push — and that reviewer has not re-reviewed since.
    #[serde(default)]
    pub review_dismissed: bool,
    /// Unix ms of the merge/close (0 while open or unknown).
    #[serde(default)]
    pub closed_at: u64,
    /// Unix ms of GitHub's `updatedAt` — the newest activity of any kind on
    /// the PR (push, comment, review, label). 0 on rows fetched before this
    /// field shipped.
    #[serde(default)]
    pub updated_at: u64,
    /// The PR this session is actually working on, as opposed to one that
    /// merely came up. Computed by [`crate::pr_rank::classify`].
    #[serde(default)]
    pub primary: bool,
    /// What decided `primary`: "auto", "session" (an in-session statement),
    /// or "override" (the cloud-stored dashboard disposition).
    #[serde(default)]
    pub primary_source: String,
    /// This secondary was classified while following a command's directory.
    /// Boards must not turn that temporary visit into worktree ownership.
    #[serde(default)]
    pub worktree_visit: bool,
    /// Hidden from rendered lists. User dismissals stay sticky; automatic
    /// stale-secondary dismissal clears if the PR is mentioned again.
    #[serde(default)]
    pub dismissed: bool,
    /// Explicitly added to the boards' watch list ("track PR <url>", the
    /// board's w key, or the dashboard). Watched PRs render like primaries
    /// and stay visible without any session working them.
    #[serde(default)]
    pub watched: bool,
    /// The Slack permalink the user pasted into the prompt that led to this
    /// PR — the conversation the work came from. Empty when none was seen.
    #[serde(default)]
    pub slack_origin_url: String,
    /// Every Slack permalink found in the PR's GitHub comments (notification
    /// bots and humans alike), deduped, in comment order. Never cleared once
    /// captured, even after the PR closes.
    #[serde(default)]
    pub slack_comment_urls: Vec<String>,
    /// The latest recap's one-line read on this PR's progress.
    #[serde(default)]
    pub ai_note: String,
    /// high / medium / low — the recap's confidence that the PR is finished.
    #[serde(default)]
    pub ai_confidence: String,
    /// Why the most recent `gh pr view` failed (first line of stderr), empty
    /// after a success. A never-enriched PR with this set renders a fetch
    /// error instead of looking silently bare; retries clear it on success.
    #[serde(default)]
    pub fetch_error: String,
    /// Initial details are waiting for a background-read quota to reset.
    #[serde(default)]
    pub fetch_limited: bool,
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
            author_login: String::new(),
            authored_by_viewer: None,
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
            ci_url: String::new(),
            unresolved_comments: 0,
            comments_url: String::new(),
            comments_refreshed_at: 0,
            created_here,
            updated_here: false,
            mentions: 0,
            user_mentions: 0,
            first_mentioned_at: 0,
            last_mentioned_at: 0,
            last_mention_prompt: 0,
            branch_matched: false,
            review_decision: String::new(),
            review_dismissed: false,
            closed_at: 0,
            updated_at: 0,
            primary: false,
            primary_source: String::new(),
            worktree_visit: false,
            dismissed: false,
            watched: false,
            slack_origin_url: String::new(),
            slack_comment_urls: Vec::new(),
            ai_note: String::new(),
            ai_confidence: String::new(),
            fetch_error: String::new(),
            fetch_limited: false,
            refreshed_at: 0,
        }
    }

    /// Take what GitHub reported about the PR from an earlier copy of its row.
    /// What the session did with it (mentions, primacy, dismissal) stays.
    fn adopt_github_fields(&mut self, known: &SessionPr) {
        self.author_login = known.author_login.clone();
        self.authored_by_viewer = known.authored_by_viewer;
        self.branch = known.branch.clone();
        self.title = known.title.clone();
        self.state = known.state.clone();
        self.is_draft = known.is_draft;
        self.additions = known.additions;
        self.deletions = known.deletions;
        self.changed_files = known.changed_files;
        self.mergeable = known.mergeable.clone();
        self.merge_state_status = known.merge_state_status.clone();
        self.checks_passed = known.checks_passed;
        self.checks_failed = known.checks_failed;
        self.checks_pending = known.checks_pending;
        self.checks_total = known.checks_total;
        self.ci_url = known.ci_url.clone();
        self.unresolved_comments = known.unresolved_comments;
        self.comments_url = known.comments_url.clone();
        self.comments_refreshed_at = known.comments_refreshed_at;
        self.review_decision = known.review_decision.clone();
        self.review_dismissed = known.review_dismissed;
        self.closed_at = known.closed_at;
        self.updated_at = known.updated_at;
        self.slack_comment_urls = known.slack_comment_urls.clone();
        self.refreshed_at = known.refreshed_at;
    }

    /// Bare tracked PR for classifier tests.
    #[cfg(test)]
    pub fn test_stub(number: u64, owner: &str, repo: &str) -> Self {
        Self::placeholder(&PrLocation::new(owner, repo, number), false)
    }

    /// A bare watched-PR entry for the boards, before `gh` enrichment lands.
    pub fn watched_stub(owner: &str, repo: &str, number: u64) -> Self {
        let mut pr = Self::placeholder(&PrLocation::new(owner, repo, number), false);
        pr.watched = true;
        pr
    }
}

/// Parse a user-entered watch target: a full PR URL or `owner/repo#123`.
pub fn parse_watch_target(input: &str) -> Option<WatchAdd> {
    let input = input.trim().trim_end_matches(['.', ',', ')', '>', ';']);
    if let Some(loc) = location_from_url(input) {
        return Some(loc.watch_add());
    }
    let caps = watch_shorthand_re().captures(input)?;
    let number: u64 = caps[3].parse().ok()?;
    if number == 0 {
        return None;
    }
    Some(PrLocation::new(&caps[1], &caps[2], number).watch_add())
}

/// One explicit watch request from an in-session "track PR <url>" statement,
/// waiting to be posted to the cloud watch list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchAdd {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub url: String,
}

/// Parsed owner/repo/number identity for a PR URL.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PrLocation {
    owner: String,
    repo: String,
    number: u64,
    url: String,
}

/// The location a full PR URL names, when it parses as one.
fn location_from_url(url: &str) -> Option<PrLocation> {
    let caps = pr_url_re().captures(url)?;
    Some(PrLocation::new(&caps[1], &caps[2], caps[3].parse().ok()?))
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

    /// This location as a watch-list add.
    fn watch_add(self) -> WatchAdd {
        WatchAdd {
            owner: self.owner,
            repo: self.repo,
            number: self.number,
            url: self.url,
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
    author: Option<GhAuthor>,
    /// Filled locally after deserialization; `gh pr view` does not expose the
    /// authenticated viewer in its JSON fields.
    #[serde(skip)]
    viewer_login: String,
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
    #[serde(default, rename = "reviewDecision")]
    review_decision: String,
    /// Each reviewer's most recent review; a DISMISSED entry means their
    /// approval (or change request) was invalidated and never redone.
    #[serde(default, rename = "latestReviews")]
    latest_reviews: Vec<GhReview>,
    /// ISO 8601 once the PR is merged or closed; GitHub returns null while open.
    #[serde(default, rename = "closedAt")]
    closed_at: Option<String>,
    /// ISO 8601 of the newest activity of any kind on the PR.
    #[serde(default, rename = "updatedAt")]
    updated_at: Option<String>,
    #[serde(default, rename = "statusCheckRollup")]
    status_check_rollup: Vec<CheckEntry>,
    /// Passed, failed, pending counts from the cheap background read.
    /// `None` means this payload didn't ask about checks, so the previous
    /// counts should stay.
    #[serde(skip)]
    check_counts: Option<(i64, i64, i64)>,
}

#[derive(Debug, Default, Deserialize)]
struct GhAuthor {
    #[serde(default)]
    login: String,
}

#[derive(Debug, Default, Deserialize)]
struct GhReview {
    #[serde(default)]
    state: String,
}

/// One entry in `statusCheckRollup`: either a CheckRun (uses `status`/`conclusion`
/// and links via `detailsUrl`) or a StatusContext (uses `state` and `targetUrl`).
#[derive(Debug, Deserialize)]
struct CheckEntry {
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default, rename = "detailsUrl")]
    details_url: Option<String>,
    #[serde(default, rename = "targetUrl")]
    target_url: Option<String>,
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
            Some("FAILURE")
            | Some("TIMED_OUT")
            | Some("CANCELLED")
            | Some("ACTION_REQUIRED")
            | Some("STARTUP_FAILURE")
            | Some("STALE") => CheckClass::Fail,
            _ => CheckClass::Pending,
        }
    }

    /// The page for this check — a workflow job's logs, or a status context's target.
    fn url(&self) -> Option<&str> {
        self.details_url
            .as_deref()
            .or(self.target_url.as_deref())
            .filter(|url| !url.is_empty())
    }
}

/// Result of a background PR read.
struct FetchResult {
    /// Location we asked about (for placeholder identity / created_here carry-over).
    requested_url: Option<String>,
    created_here: bool,
    /// This fetch was caused by a push/PR creation, so the PR should remain on
    /// the active status/review cadence for a short window.
    pr_active: bool,
    data: Result<GhPrJson, String>,
    /// Review threads included in the same read. `None` when this was only a
    /// branch or number lookup and the thread count should be left alone.
    threads: Option<ReviewThreads>,
}

/// What a finished background job carries back.
enum JobResult {
    /// A PR enrichment.
    Pr(Box<FetchResult>),
}

/// Unresolved review threads on one PR, plus any Slack permalinks its
/// issue comments carry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReviewThreads {
    unresolved: i64,
    /// The first unresolved thread's comment anchor (empty when none).
    first_url: String,
    /// Every Slack permalink in the PR's comments, deduped, in order.
    slack_urls: Vec<String>,
}

/// GraphQL response shape for the review-thread query used by tests.
#[cfg(test)]
#[derive(Debug, Deserialize)]
struct ThreadsResponse {
    data: ThreadsData,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
struct ThreadsData {
    repository: ThreadsRepository,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
struct ThreadsRepository {
    #[serde(rename = "pullRequest")]
    pull_request: ThreadsPullRequest,
}

#[cfg(test)]
#[derive(Debug, Deserialize)]
struct ThreadsPullRequest {
    #[serde(rename = "reviewThreads")]
    review_threads: ThreadNodes,
    #[serde(default)]
    comments: IssueComments,
}

#[derive(Debug, Default, Deserialize)]
struct IssueComments {
    #[serde(default)]
    nodes: Vec<IssueComment>,
}

#[derive(Debug, Deserialize)]
struct IssueComment {
    #[serde(default)]
    body: String,
}

#[derive(Debug, Default, Deserialize)]
struct ThreadNodes {
    #[serde(default)]
    nodes: Vec<ReviewThreadNode>,
    #[serde(default, rename = "pageInfo")]
    page_info: Option<ThreadPageInfo>,
}

#[derive(Debug, Deserialize)]
struct ThreadPageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
}

#[derive(Debug, Deserialize)]
struct ReviewThreadNode {
    #[serde(default, rename = "isResolved")]
    is_resolved: bool,
    #[serde(default)]
    comments: ThreadComments,
}

#[derive(Debug, Default, Deserialize)]
struct ThreadComments {
    #[serde(default)]
    nodes: Vec<ThreadComment>,
}

#[derive(Debug, Deserialize)]
struct ThreadComment {
    #[serde(default)]
    url: String,
}

/// Owns PR detection, background enrichment, and the session-scoped list.
pub struct PrTracker {
    prs: Vec<SessionPr>,
    /// In-flight `gh` jobs keyed by the URL (or synthetic branch key) being fetched.
    pending: HashMap<String, mpsc::Receiver<JobResult>>,
    /// Bare-number mentions awaiting a repository identity from GitHub.
    pending_mentions: HashMap<String, Vec<(bool, u32, u64)>>,
    /// Number lookups whose `gh pr` verb changes the PR (`merge`, `ready`,
    /// `edit`, `close`, `reopen`). Applied when that lookup returns.
    pending_updates: HashSet<String>,
    /// Last time a `git push` / PR-edit triggered a current-branch PR lookup.
    /// Throttles branch resolution since the same turn text is re-scanned each tick.
    last_branch_resolve: Option<Instant>,
    /// Mention occurrences already handled in the current turn. Latest-turn
    /// transcripts are re-scanned every hook tick, so this prevents one mention
    /// from perpetually resetting the active window or forcing API calls.
    mention_events_seen: HashMap<String, usize>,
    /// User prompt that owns the per-turn command and mention deduplication.
    /// This is also a fallback turn boundary when the prompt counter arrives late.
    scan_prompt_owner: Option<String>,
    /// Last background read attempt per PR URL. Failed requests back off too,
    /// rather than retrying on every two-second idle hook tick.
    refresh_attempted_at: HashMap<String, Instant>,
    /// Consecutive read failures per PR URL, cleared on success.
    /// Never-enriched PRs retry on this count's backoff schedule.
    fetch_failures: HashMap<String, u32>,
    /// Last observed mention, push, or creation per PR URL. Recent activity
    /// restarts the PR's refresh backoff.
    pr_active_at: HashMap<String, Instant>,
    /// When a read last found the PR different on GitHub. A PR that just
    /// moved is likely to move again, so this restarts the backoff too.
    status_changed_at: HashMap<String, Instant>,
    /// `PR #N` lookups waiting for a free GitHub slot.
    deferred_lookups: Vec<(PathBuf, u64)>,
    /// URLs a real mention asked to refresh while another read was running.
    force_refresh: HashSet<String>,
    /// When the per-session hourly read count started.
    read_window_started: Option<Instant>,
    /// Background reads this session has started in the current hour.
    reads_this_window: u32,
    /// Latest prompt or completion time for this session (unix seconds).
    session_activity_at: Option<f64>,
    /// Update commands already handled in the current turn, counted by their
    /// command text so rescanning the growing transcript does not make one push
    /// look perpetually recent.
    update_commands_seen: HashMap<String, usize>,
    /// A push can arrive while the startup/current-branch lookup is still in
    /// flight. Carry that activity onto the existing job instead of dropping it.
    pending_pr_active: HashMap<String, bool>,
    /// The platform's prompt counter, stamped onto mentions so recency can be
    /// judged in turns rather than wall-clock time.
    prompt_count: u32,
    /// A command's workdir is a temporary visit, weaker than creating a PR.
    command_workdir: bool,
    replaying_history: bool,
    mention_time: Option<u64>,
    /// URLs pasted into user prompts, kept for branch/preview-URL matching.
    prompt_urls: Vec<String>,
    /// "PR #123 is the primary" statements from this session, by number.
    declared_numbers: HashMap<u64, PrDisposition>,
    /// The same statements when made with a full PR URL.
    declared_urls: HashMap<String, PrDisposition>,
    /// Cloud-stored dispositions keyed `owner/repo#number`.
    overrides: HashMap<String, PrDisposition>,
    /// Watch requests typed this session ("track PR <url>") that still need
    /// posting to the cloud watch list.
    pending_watch_adds: Vec<WatchAdd>,
    /// The most recent Slack permalink pasted into a user prompt, and when.
    /// A PR that appears shortly after inherits it as its origin.
    latest_prompt_slack: Option<(String, Instant)>,
    /// Every Slack permalink pasted this session, oldest first.
    slack_threads: Vec<SlackThread>,
    /// Enriched metadata for every Slack permalink attached to a tracked PR
    /// (its origin and GitHub comment links). A lookup directory for the PR
    /// boards; the status bar keeps showing only `slack_threads`.
    pr_slack_threads: Vec<SlackThread>,
    /// Readable Slack channel and user names from the local Slack MCP cache.
    slack_directory: SlackDirectory,
    /// What GitHub last said about PRs dropped when the conversation reset,
    /// by URL, so a PR that comes back shows those values at once rather than
    /// a blank row waiting on a read.
    last_known: HashMap<String, SessionPr>,
}

impl Default for PrTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl PrTracker {
    pub fn new() -> Self {
        Self::with_slack_directory(load_slack_directory())
    }

    fn with_slack_directory(slack_directory: SlackDirectory) -> Self {
        Self {
            prs: Vec::new(),
            pending: HashMap::new(),
            pending_mentions: HashMap::new(),
            pending_updates: HashSet::new(),
            last_branch_resolve: None,
            mention_events_seen: HashMap::new(),
            scan_prompt_owner: None,
            refresh_attempted_at: HashMap::new(),
            fetch_failures: HashMap::new(),
            pr_active_at: HashMap::new(),
            status_changed_at: HashMap::new(),
            deferred_lookups: Vec::new(),
            force_refresh: HashSet::new(),
            read_window_started: None,
            reads_this_window: 0,
            session_activity_at: None,
            update_commands_seen: HashMap::new(),
            pending_pr_active: HashMap::new(),
            prompt_count: 0,
            command_workdir: false,
            replaying_history: false,
            mention_time: None,
            prompt_urls: Vec::new(),
            declared_numbers: HashMap::new(),
            declared_urls: HashMap::new(),
            overrides: HashMap::new(),
            pending_watch_adds: Vec::new(),
            latest_prompt_slack: None,
            slack_threads: Vec::new(),
            pr_slack_threads: Vec::new(),
            slack_directory,
            last_known: HashMap::new(),
        }
    }

    /// Forget everything learned from the conversation so far. Only state
    /// that belongs to the pane rather than the conversation survives: cloud
    /// dispositions, watch requests still waiting to post, and the Slack
    /// directory. What GitHub last said about each PR is kept too, for a PR
    /// that comes back. Called when the pane moves to a different conversation
    /// (Codex `/new` or `/resume`, Claude Code `/clear`) so the previous
    /// conversation's PRs and Slack threads stop showing under the new one.
    pub fn reset_conversation(&mut self) {
        let mut fresh = Self::with_slack_directory(std::mem::take(&mut self.slack_directory));
        fresh.last_known = std::mem::take(&mut self.last_known);
        fresh.last_known.extend(
            self.prs
                .drain(..)
                .filter(|pr| pr.refreshed_at != 0)
                .map(|pr| (pr.url.clone(), pr)),
        );
        fresh.overrides = std::mem::take(&mut self.overrides);
        fresh.pending_watch_adds = std::mem::take(&mut self.pending_watch_adds);
        fresh.command_workdir = self.command_workdir;
        *self = fresh;
    }

    pub fn prs(&self) -> &[SessionPr] {
        &self.prs
    }

    /// Record the platform's prompt counter for mention recency stamps.
    pub fn set_prompt_count(&mut self, prompts: u32) {
        self.prompt_count = prompts;
    }

    pub fn set_command_workdir(&mut self, command_workdir: bool) {
        self.command_workdir = command_workdir;
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
        self.resolve_branch_pr(cwd, false);
    }

    /// Reset per-turn transcript deduplication after the platform confirms that
    /// a new prompt was recorded, rather than during the gap after Enter.
    pub fn on_prompt_observed(&mut self) {
        self.reset_scan_deduplication();
    }

    fn reset_scan_deduplication(&mut self) {
        self.mention_events_seen.clear();
        self.update_commands_seen.clear();
        self.scan_prompt_owner = None;
    }

    /// Scan a chunk of transcript/activity text (one turn's worth).
    ///
    /// What counts as an association, by channel:
    /// - **prose** (user prompt, assistant text): PR URLs, and `#123` that carries
    ///   a PR marker (see [`pr_marker_before`]).
    /// - **tool commands**: the PR a `gh pr` subcommand targets. `gh pr view 49`
    ///   and `gh pr merge 49` use the session checkout, including numbers too
    ///   small for a prose mention. `gh pr view 2469 --repo owner/repo` and
    ///   `gh pr checks <url>` name the repository themselves. A `merge`,
    ///   `ready`, `edit`, `close`, or `reopen` marks the PR as work this
    ///   session did. URLs merely quoted inside a command (a `gh pr create
    ///   --body` that cites related PRs) are not targets.
    /// - **tool output**: nothing, except the URL `gh pr create` prints for the PR
    ///   it just opened. Listings (`gh pr list`, `git log`, JSON dumps) name PRs
    ///   the session never touched.
    ///
    /// Bare numbers are validated against the current repository before being
    /// added. Enrichment results land later via [`poll`].
    pub fn scan_text(&mut self, text: &str, cwd: &Path) -> bool {
        self.scan_slack_metadata(text) | self.scan_text_inner(text, cwd, true, "activity")
    }

    /// Restore detections in chronological turn order. This replays evidence,
    /// never shell commands. Historical pushes must not query today's checkout
    /// as though that were the branch the old command updated.
    pub fn scan_transcript_turn(
        &mut self,
        turn: &crate::recap::TrackingTurn,
        cwd: &Path,
        historical: bool,
    ) -> bool {
        if self.prompt_count != turn.number {
            self.on_prompt_observed();
            self.set_prompt_count(turn.number);
        }
        self.replaying_history = historical;
        self.mention_time = (historical && turn.timestamp != 0).then_some(turn.timestamp);
        let cwd = turn
            .cwd
            .as_deref()
            .filter(|path| path.is_dir())
            .unwrap_or(cwd);
        let mut changed = false;
        if let Some(prompt) = turn.transcript.user_prompt.as_deref() {
            changed |= self.scan_prompt(prompt, cwd);
        }
        changed |= self.scan_text(&turn.transcript.activity, cwd);
        self.replaying_history = false;
        self.mention_time = None;
        changed
    }

    /// Enrich restored PRs once after the historical scan, without refreshing
    /// on every old mention while the transcript is being replayed. One read
    /// starts here; the rest follow on later polls, under the same budget.
    pub fn refresh_restored_prs(&mut self) {
        self.start_due_refresh();
    }

    /// Scan a user prompt for PR references without treating text such as
    /// "please git push" as evidence that a push already happened.
    pub fn scan_prompt(&mut self, text: &str, cwd: &Path) -> bool {
        if self.scan_prompt_owner.as_deref() != Some(text) {
            self.reset_scan_deduplication();
            self.scan_prompt_owner = Some(text.to_string());
            self.note_prompt_urls(text);
            self.scan_declarations(text);
        }
        self.scan_text_inner(text, cwd, false, "prompt")
    }

    /// Remember URLs the user pasted — preview deployments embed branch names,
    /// which is sometimes the only tie between a session and its PR. Slack
    /// permalinks are also noted as the likely origin of upcoming work.
    fn note_prompt_urls(&mut self, text: &str) {
        let mut prompt_origin = None;
        for mut thread in extract_threads(text) {
            self.slack_directory.enrich_thread(&mut thread);
            prompt_origin.get_or_insert_with(|| thread.url.clone());
            if !self
                .slack_threads
                .iter()
                .any(|existing| existing.url == thread.url)
            {
                self.slack_threads.push(thread);
            }
        }
        if let Some(origin) = prompt_origin {
            self.latest_prompt_slack = Some((origin, Instant::now()));
        }
        for found in any_url_re().find_iter(text) {
            let url = found
                .as_str()
                .trim_end_matches(['.', ',', ')', ']', '>', ';'])
                .to_string();
            if !self.prompt_urls.contains(&url) {
                self.prompt_urls.push(url);
            }
        }
        let excess = self.prompt_urls.len().saturating_sub(PROMPT_URLS_KEPT);
        if excess > 0 {
            self.prompt_urls.drain(..excess);
        }
    }

    /// The pasted Slack permalink a brand-new PR should claim as its origin,
    /// while it is still fresh enough to plausibly be the source conversation.
    fn current_origin_slack(&self) -> String {
        self.latest_prompt_slack
            .as_ref()
            .filter(|(_, at)| at.elapsed() < SLACK_ORIGIN_CLAIM_WINDOW)
            .map(|(url, _)| url.clone())
            .unwrap_or_default()
    }

    /// The most recent Slack permalink the user pasted, for session-level use.
    pub fn session_slack_origin(&self) -> Option<&str> {
        self.latest_prompt_slack
            .as_ref()
            .map(|(url, _)| url.as_str())
    }

    /// Every Slack message permalink pasted into a user prompt this session.
    pub fn slack_threads(&self) -> &[SlackThread] {
        &self.slack_threads
    }

    /// Enriched metadata for the Slack permalinks attached to tracked PRs, so
    /// the PR boards can show channel and author names instead of raw IDs.
    pub fn pr_slack_threads(&self) -> &[SlackThread] {
        &self.pr_slack_threads
    }

    /// Give every PR-attached Slack permalink a metadata entry. Pasted
    /// permalinks donate what the session already learned about them; new
    /// URLs resolve against the local Slack directory.
    fn sync_pr_slack_threads(&mut self) -> bool {
        let urls: Vec<String> = self
            .prs
            .iter()
            .flat_map(|pr| {
                std::iter::once(pr.slack_origin_url.as_str())
                    .chain(pr.slack_comment_urls.iter().map(String::as_str))
            })
            .filter(|url| !url.is_empty())
            .map(str::to_string)
            .collect();
        let mut changed = false;
        for url in urls {
            if self.pr_slack_threads.iter().any(|thread| thread.url == url) {
                continue;
            }
            let Some(mut thread) = self
                .slack_threads
                .iter()
                .find(|thread| thread.url == url)
                .cloned()
                .or_else(|| extract_threads(&url).into_iter().next())
            else {
                continue;
            };
            self.slack_directory.enrich_thread(&mut thread);
            self.pr_slack_threads.push(thread);
            changed = true;
        }
        changed
    }

    /// Add recap-derived names only when exact Slack metadata is still absent.
    /// Unknown or invented URLs never enter session state.
    pub fn apply_slack_metadata(&mut self, threads: &[SlackThread]) -> bool {
        let mut changed = false;
        for thread in threads {
            for existing in self
                .slack_threads
                .iter_mut()
                .chain(self.pr_slack_threads.iter_mut())
                .filter(|existing| existing.url == thread.url)
            {
                if let Some(channel) = thread.channel.as_ref().filter(|channel| {
                    !channel.is_empty()
                        && existing.channel.as_ref() != Some(*channel)
                        && has_only_channel_id(existing)
                }) {
                    existing.channel = Some(channel.clone());
                    changed = true;
                }
                if existing.author.is_none() && thread.author.is_some() {
                    existing.author = thread.author.clone();
                    changed = true;
                }
            }
        }
        changed
    }

    fn scan_slack_metadata(&mut self, text: &str) -> bool {
        let metadata = self.slack_directory.message_metadata(text);
        let mut changed = false;
        for metadata in metadata {
            for thread in self
                .slack_threads
                .iter_mut()
                .chain(self.pr_slack_threads.iter_mut())
                .filter(|thread| metadata.matches(thread))
            {
                changed |= metadata.apply_to(thread);
            }
        }
        changed
    }

    /// Pick up explicit dispositions the user types: "PR #123 is the primary",
    /// "the secondary one is #99", "dismiss PR #4546". User statements outrank
    /// automatic scoring and only a dashboard override outranks them.
    fn scan_declarations(&mut self, text: &str) {
        for caps in decl_number_first_re().captures_iter(text) {
            if let (Ok(number), Some(word)) = (caps[1].parse::<u64>(), caps.get(2)) {
                self.declared_numbers
                    .insert(number, parse_disposition(word.as_str()));
            }
        }
        for caps in decl_keyword_first_re().captures_iter(text) {
            if let (Some(word), Ok(number)) = (caps.get(1), caps[2].parse::<u64>()) {
                self.declared_numbers
                    .insert(number, parse_disposition(word.as_str()));
            }
        }
        for caps in decl_dismiss_re().captures_iter(text) {
            if let Ok(number) = caps[1].parse::<u64>() {
                self.declared_numbers
                    .insert(number, PrDisposition::Dismissed);
            }
        }
        for caps in decl_url_re().captures_iter(text) {
            if let (Some(url), Some(word)) = (caps.get(1), caps.get(2)) {
                self.declared_urls
                    .insert(url.as_str().to_string(), parse_disposition(word.as_str()));
            }
        }
        for caps in decl_watch_url_re().captures_iter(text) {
            if let Some(loc) = location_from_url(&caps[1]) {
                self.note_watch(loc);
            }
        }
        for caps in decl_watch_repo_re().captures_iter(text) {
            if let Ok(number) = caps[3].parse::<u64>() {
                self.note_watch(PrLocation::new(&caps[1], &caps[2], number));
            }
        }
    }

    /// Record an explicit watch: track the PR in this session, flag it
    /// watched, and queue the cloud watch-list add.
    fn note_watch(&mut self, loc: PrLocation) {
        self.observe_url(&loc, true);
        if let Some(pr) = self.prs.iter_mut().find(|p| p.url == loc.url) {
            pr.watched = true;
        }
        let add = loc.watch_add();
        if !self.replaying_history && !self.pending_watch_adds.contains(&add) {
            self.pending_watch_adds.push(add);
        }
    }

    /// Watch requests typed this session that still need posting to the cloud.
    pub fn take_watch_adds(&mut self) -> Vec<WatchAdd> {
        std::mem::take(&mut self.pending_watch_adds)
    }

    /// Replace the cloud-stored dispositions (dashboard toggles / action links).
    pub fn set_overrides(&mut self, overrides: HashMap<String, PrDisposition>) {
        self.overrides = overrides;
    }

    /// Copy a fresh recap's per-PR judgments onto the tracked PRs, so the
    /// notes travel with the PR to the mirror, the cloud, and the boards.
    pub fn apply_recap_notes(&mut self, notes: &[crate::recap::PrRecapNote]) -> bool {
        let mut changed = false;
        for note in notes {
            if let Some(pr) = self.prs.iter_mut().find(|p| p.url == note.url) {
                if pr.ai_note != note.note || pr.ai_confidence != note.confidence {
                    pr.ai_note = note.note.clone();
                    pr.ai_confidence = note.confidence.clone();
                    changed = true;
                }
            }
        }
        changed
    }

    /// Re-run primary/secondary classification against the session's current
    /// branch and working directory. Returns true when anything visible moved.
    pub fn reclassify(&mut self, current_branch: &str, cwd: &Path) -> bool {
        if self.prs.is_empty() {
            return false;
        }
        let ctx = crate::pr_rank::RankContext {
            command_workdir: self.command_workdir,
            current_branch: current_branch.to_string(),
            worktree_dir: cwd
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            prompt_count: self.prompt_count,
            prompt_urls: self.prompt_urls.clone(),
            declared_numbers: self.declared_numbers.clone(),
            declared_urls: self.declared_urls.clone(),
            overrides: self.overrides.clone(),
        };
        crate::pr_rank::classify(&mut self.prs, &ctx)
    }

    fn scan_text_inner(
        &mut self,
        text: &str,
        cwd: &Path,
        handle_updates: bool,
        mention_scope: &str,
    ) -> bool {
        if text.is_empty() {
            return false;
        }
        let mut changed = false;
        let user_authored = mention_scope == "prompt";
        let mut update_occurrences = HashMap::<String, usize>::new();
        let mut mention_occurrences = HashMap::<String, usize>::new();
        let known_pr_numbers: HashSet<u64> = self.prs.iter().map(|pr| pr.number).collect();
        for event in scan_events_with_known_prs(text, &known_pr_numbers) {
            match event {
                ScanEvent::Created(loc) if handle_updates => {
                    if self
                        .prs
                        .iter()
                        .any(|pr| pr.url == loc.url && pr.created_here)
                    {
                        continue;
                    }
                    changed |= self.observe_url(&loc, true);
                    if let Some(pr) = self.prs.iter_mut().find(|pr| pr.url == loc.url) {
                        if !pr.created_here {
                            pr.created_here = true;
                            changed = true;
                        }
                    }
                }
                ScanEvent::Created(_) => {}
                ScanEvent::Updated(command) if handle_updates => {
                    let occurrence = update_occurrences.entry(command.clone()).or_default();
                    *occurrence += 1;
                    let seen = self.update_commands_seen.entry(command).or_default();
                    if *occurrence > *seen {
                        *seen = *occurrence;
                        if !self.replaying_history {
                            self.resolve_branch_pr(cwd, true);
                        }
                    }
                }
                ScanEvent::Updated(_) => {}
                ScanEvent::Located { loc, bulk } => {
                    if self.is_new_mention(mention_scope, &loc.url, &mut mention_occurrences) {
                        // A line that lists many PRs tracks them, but it is not a
                        // reason to ask GitHub about each one.
                        changed |= self.observe_url(&loc, !bulk);
                        if !bulk {
                            changed |= self.record_mention_for_url(&loc.url, user_authored);
                        }
                    }
                }
                ScanEvent::CommandLocated { loc, updates } => {
                    if self.is_new_mention(mention_scope, &loc.url, &mut mention_occurrences) {
                        changed |= self.observe_url(&loc, true);
                        changed |= self.record_mention_for_url(&loc.url, user_authored);
                        if updates {
                            changed |= self.mark_updated_here(&loc.url);
                        }
                    }
                }
                ScanEvent::CommandNumber { number, updates } => {
                    let identity = format!("gh#{number}");
                    if self.is_new_mention(mention_scope, &identity, &mut mention_occurrences) {
                        changed |= self.resolve_command_number(number, cwd, updates);
                        let recorded = self.record_mention_for_number(number, user_authored);
                        changed |= recorded;
                        if !recorded {
                            self.remember_lookup_mention(
                                &format!("mention:{}#{number}", cwd.display()),
                                user_authored,
                            );
                        }
                    }
                }
                // `gh pr view` rejects issue numbers and nonexistent PRs, so only
                // numbers that are really PRs in this repo reach the visible list.
                ScanEvent::Mentioned { number, bulk } => {
                    if self.is_new_mention(
                        mention_scope,
                        &format!("#{number}"),
                        &mut mention_occurrences,
                    ) {
                        self.resolve_mentioned_pr(number, cwd);
                        if !bulk {
                            let recorded = self.record_mention_for_number(number, user_authored);
                            changed |= recorded;
                            let key = format!("mention:{}#{number}", cwd.display());
                            if !recorded && self.pending.contains_key(&key) {
                                self.pending_mentions.entry(key).or_default().push((
                                    user_authored,
                                    self.prompt_count,
                                    self.mention_time.unwrap_or_else(now_unix_ms),
                                ));
                            }
                        }
                    }
                }
            }
        }
        changed
    }

    /// Queue a mention for a number lookup that has not resolved to a PR yet.
    fn remember_lookup_mention(&mut self, key: &str, user_authored: bool) {
        let waiting = self.pending.contains_key(key)
            || mention_lookup_key(key)
                .is_some_and(|lookup| self.deferred_lookups.contains(&lookup));
        if !waiting {
            return;
        }
        self.pending_mentions
            .entry(key.to_string())
            .or_default()
            .push((
                user_authored,
                self.prompt_count,
                self.mention_time.unwrap_or_else(now_unix_ms),
            ));
    }

    /// Count one real mention against the tracked PR at `url`.
    fn record_mention_for_url(&mut self, url: &str, user_authored: bool) -> bool {
        let prompt_count = self.prompt_count;
        match self.prs.iter_mut().find(|p| p.url == url) {
            Some(pr) => bump_mention(pr, user_authored, prompt_count, self.mention_time),
            None => false,
        }
    }

    /// Count one real mention against every tracked PR sharing `number` —
    /// a bare mention doesn't name a repository, so all candidates gain it.
    fn record_mention_for_number(&mut self, number: u64, user_authored: bool) -> bool {
        let prompt_count = self.prompt_count;
        let mut changed = false;
        for pr in self.prs.iter_mut().filter(|p| p.number == number) {
            changed |= bump_mention(pr, user_authored, prompt_count, self.mention_time);
        }
        changed
    }

    /// The session ran `gh pr merge` (or ready, edit, close, reopen) on this PR.
    fn mark_updated_here(&mut self, url: &str) -> bool {
        let Some(pr) = self.prs.iter_mut().find(|pr| pr.url == url) else {
            return false;
        };
        if pr.updated_here {
            return false;
        }
        pr.updated_here = true;
        true
    }

    fn is_new_mention(
        &mut self,
        scope: &str,
        identity: &str,
        occurrences: &mut HashMap<String, usize>,
    ) -> bool {
        let occurrence = occurrences.entry(identity.to_string()).or_default();
        *occurrence += 1;
        let seen = self
            .mention_events_seen
            .entry(format!("{scope}:{identity}"))
            .or_default();
        if *occurrence <= *seen {
            return false;
        }
        *seen = *occurrence;
        true
    }

    /// Handle a PR URL seen in the scrollback.
    ///
    /// `engage` is false for a bulk listing. Those PRs are remembered, but they
    /// do not start a GitHub read or restart the fast refresh window.
    fn observe_url(&mut self, loc: &PrLocation, engage: bool) -> bool {
        // Every real mention refreshes immediately and restarts the active
        // window. Transcript rescans are filtered by `is_new_mention`, so
        // `force` still means once per actual mention rather than once per
        // hook tick. A second mention while a read is already in flight waits
        // for that read to finish instead of starting another.
        if self.prs.iter().any(|p| p.url == loc.url) {
            if self.replaying_history || !engage {
                return false;
            }
            self.note_pr_active(&loc.url);
            self.refresh_url(&loc.url, true);
            return false;
        }

        let mut pr = SessionPr::placeholder(loc, false);
        pr.slack_origin_url = self.current_origin_slack();
        if let Some(known) = self.last_known.get(&loc.url) {
            pr.adopt_github_fields(known);
        }
        self.prs.push(pr);
        if !self.replaying_history && engage {
            self.note_pr_active(&loc.url);
            self.spawn_fetch(loc.url.clone(), false);
        }
        true
    }

    fn resolve_mentioned_pr(&mut self, number: u64, cwd: &Path) {
        if number == 0 {
            return;
        }
        // Already tracked from a source that named its repository. Prose nicknames
        // ("RQH #2499") don't map to a repo, so guessing this number against the
        // session's own repo could only find a different PR that happens to share it.
        let tracked_urls: Vec<String> = self
            .prs
            .iter()
            .filter(|pr| pr.number == number)
            .map(|pr| pr.url.clone())
            .collect();
        if !tracked_urls.is_empty() {
            for url in tracked_urls {
                if self.replaying_history {
                    continue;
                }
                self.note_pr_active(&url);
                self.refresh_url(&url, true);
            }
            return;
        }
        // Adopting a new PR from a bare number is only safe when the number is
        // large enough that a coincidental match is unlikely.
        if number < MIN_BARE_PR_NUMBER {
            return;
        }
        let key = format!("mention:{}#{number}", cwd.display());
        if self.pending.contains_key(&key) {
            if !self.replaying_history {
                self.pending_pr_active.insert(key, true);
            }
            return;
        }
        let cwd_buf = cwd.to_path_buf();
        let pr_active = !self.replaying_history;
        if !self.spawn_pr_job(key, None, false, pr_active, move || {
            fetch_pr_number(&cwd_buf, number, budget::Reader::Session)
        }) {
            self.defer_lookup(cwd, number);
        }
    }

    /// `gh pr view 49` with no `--repo`: the number is this checkout's PR.
    /// Unlike a prose `#49`, the command is unambiguous, so small numbers
    /// are looked up too. A verb that changes the PR is remembered and
    /// applied when the lookup returns.
    fn resolve_command_number(&mut self, number: u64, cwd: &Path, updates: bool) -> bool {
        if number == 0 {
            return false;
        }
        let tracked_urls: Vec<String> = self
            .prs
            .iter()
            .filter(|pr| pr.number == number)
            .map(|pr| pr.url.clone())
            .collect();
        if !tracked_urls.is_empty() {
            let mut changed = false;
            for url in &tracked_urls {
                if updates {
                    changed |= self.mark_updated_here(url);
                }
                if self.replaying_history {
                    continue;
                }
                self.note_pr_active(url);
                self.refresh_url(url, true);
            }
            return changed;
        }
        let key = format!("mention:{}#{number}", cwd.display());
        if updates {
            self.pending_updates.insert(key.clone());
        }
        if self.pending.contains_key(&key) {
            if !self.replaying_history {
                self.pending_pr_active.insert(key, true);
            }
            return false;
        }
        let cwd_buf = cwd.to_path_buf();
        let pr_active = !self.replaying_history;
        if !self.spawn_pr_job(key, None, false, pr_active, move || {
            fetch_pr_number(&cwd_buf, number, budget::Reader::Session)
        }) {
            self.defer_lookup(cwd, number);
        }
        false
    }

    /// Remember a bare-number lookup that could not start yet.
    fn defer_lookup(&mut self, cwd: &Path, number: u64) {
        let lookup = (cwd.to_path_buf(), number);
        if self.deferred_lookups.contains(&lookup)
            || self.deferred_lookups.len() >= DEFERRED_LOOKUP_LIMIT
        {
            return;
        }
        self.deferred_lookups.push(lookup);
    }

    /// Refresh the one tracked PR most in need of fresh stats.
    ///
    /// Called on turn completion. A turn that names dozens of PRs must not
    /// start dozens of GitHub reads. Returns true if a fetch was started (no
    /// visible change yet — results arrive via [`poll`]).
    pub fn refresh_stale(&mut self) -> bool {
        self.start_due_refresh()
    }

    /// Start a background read for a tracked URL unless one is already in
    /// flight or it was refreshed within the throttle window (bypassed when
    /// `force`). At most one read runs at a time.
    fn refresh_url(&mut self, url: &str, force: bool) -> bool {
        if self.pending.contains_key(url) {
            return false;
        }
        if !force {
            if self
                .refresh_attempted_at
                .get(url)
                .map(|t| t.elapsed() < REFRESH_THROTTLE)
                .unwrap_or(false)
            {
                return false;
            }
            let now = now_unix_ms();
            if let Some(pr) = self.prs.iter().find(|p| p.url == url) {
                if pr.refreshed_at != 0
                    && now.saturating_sub(pr.refreshed_at) < REFRESH_THROTTLE.as_millis() as u64
                {
                    return false;
                }
            }
        }
        let url_for_job = url.to_string();
        let started = self.spawn_pr_job(
            url.to_string(),
            Some(url.to_string()),
            false,
            false,
            move || fetch_pr(&url_for_job, budget::Reader::Session),
        );
        if force && !started {
            self.force_refresh.insert(url.to_string());
        }
        started
    }

    /// Resolve the PR attached to the current branch in `cwd` (after a push/edit).
    /// Adds it as an "updated here" PR once `gh` reports back.
    fn resolve_branch_pr(&mut self, cwd: &Path, pr_active: bool) {
        let key = format!("branch:{}", cwd.display());
        if pr_active {
            self.pending_pr_active.insert(key.clone(), true);
        }
        // Throttle: the same turn text is re-scanned every hook tick, so a turn
        // containing a `git push` would otherwise spawn a lookup on every tick.
        if !pr_active
            && self
                .last_branch_resolve
                .map(|t| t.elapsed() < REFRESH_THROTTLE)
                .unwrap_or(false)
        {
            return;
        }
        if self.pending.contains_key(&key) {
            return;
        }
        let cwd_buf = cwd.to_path_buf();
        if self.spawn_pr_job(key, None, false, pr_active, move || {
            fetch_pr_for_branch(&cwd_buf, budget::Reader::Session)
        }) && !pr_active
        {
            self.last_branch_resolve = Some(Instant::now());
        }
    }

    /// Spawn a background read of one PR URL. Review threads come back in the
    /// same response, following additional pages only for review threads.
    fn spawn_fetch(&mut self, url: String, created_here: bool) {
        let url_for_job = url.clone();
        self.spawn_pr_job(
            url,
            Some(url_for_job.clone()),
            created_here,
            created_here,
            move || fetch_pr(&url_for_job, budget::Reader::Session),
        );
    }

    /// Start one `gh` job. Returns false when a read is already running, this
    /// session has used its hourly allowance, or the machine-wide budget says
    /// to wait. Nothing is marked as attempted in that case, so a later poll
    /// tries again.
    fn spawn_pr_job<F>(
        &mut self,
        key: String,
        requested_url: Option<String>,
        created_here: bool,
        pr_active: bool,
        job: F,
    ) -> bool
    where
        F: FnOnce() -> Result<(GhPrJson, Option<ReviewThreads>), String> + Send + 'static,
    {
        if self.pending.contains_key(&key) || !self.pending.is_empty() {
            return false;
        }
        let cost = if requested_url.is_some() {
            budget::READ_POINTS
        } else {
            budget::UNREPORTED_READ_POINTS
        };
        if !self.session_read_allowed()
            || !budget::can_start_with_estimate(budget::Reader::Session, cost)
        {
            return false;
        }
        self.note_session_read();
        if let Some(url) = &requested_url {
            self.refresh_attempted_at
                .insert(url.clone(), Instant::now());
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (data, threads) = match job() {
                Ok((pr, threads)) => (Ok(pr), threads),
                Err(error) => (Err(error), None),
            };
            let _ = tx.send(JobResult::Pr(Box::new(FetchResult {
                requested_url,
                created_here,
                pr_active,
                data,
                threads,
            })));
        });
        self.pending.insert(key, rx);
        true
    }

    /// One background read per poll: a queued number lookup, a PR that has
    /// never loaded, or the most recently touched open PR whose cadence is due.
    fn start_due_refresh(&mut self) -> bool {
        if !self.pending.is_empty() {
            return false;
        }
        if let Some(url) = self.force_refresh.iter().next().cloned() {
            self.spawn_fetch(url, false);
            return !self.pending.is_empty();
        }
        if let Some((cwd, number)) = self.deferred_lookups.first().cloned() {
            self.deferred_lookups.remove(0);
            let key = format!("mention:{}#{number}", cwd.display());
            let cwd_for_job = cwd.clone();
            if self.spawn_pr_job(key, None, false, false, move || {
                fetch_pr_number(&cwd_for_job, number, budget::Reader::Session)
            }) {
                return true;
            }
            self.defer_lookup(&cwd, number);
            return false;
        }
        if let Some(url) = self.next_budgeted_url() {
            self.spawn_fetch(url, false);
            return !self.pending.is_empty();
        }
        false
    }

    /// The one PR a free slot should read. Recent work wins. An older PR that
    /// has never loaded waits until that recent work is quiet.
    fn next_budgeted_url(&self) -> Option<String> {
        if let Some(url) = self.force_refresh.iter().next() {
            return Some(url.clone());
        }
        if let Some(url) = self.next_unenriched_url() {
            if self.is_recent_pr(&url) || self.next_refresh_url().is_none() {
                return Some(url);
            }
        }
        self.next_refresh_url()
    }

    fn session_read_allowed(&mut self) -> bool {
        let started = self.read_window_started.get_or_insert_with(Instant::now);
        if started.elapsed() >= Duration::from_secs(60 * 60) {
            self.read_window_started = Some(Instant::now());
            self.reads_this_window = 0;
        }
        self.reads_this_window < SESSION_READS_PER_HOUR
    }

    fn note_session_read(&mut self) {
        self.reads_this_window = self.reads_this_window.saturating_add(1);
    }

    /// Whether the session worked with this PR rather than only listing it.
    fn engaged(&self, pr: &SessionPr) -> bool {
        pr.last_mentioned_at > 0
            || pr.created_here
            || pr.branch_matched
            || self.pr_active_at.contains_key(&pr.url)
    }

    /// Whether `url` is one of the few PRs this session touched most recently.
    fn is_recent_pr(&self, url: &str) -> bool {
        let mut engaged: Vec<&SessionPr> = self
            .prs
            .iter()
            .filter(|pr| !pr.dismissed && !pr.url.is_empty() && self.engaged(pr))
            .collect();
        engaged.sort_by(|a, b| {
            b.last_mentioned_at
                .cmp(&a.last_mentioned_at)
                .then(a.url.cmp(&b.url))
        });
        engaged
            .into_iter()
            .take(BACKGROUND_PR_LIMIT)
            .any(|pr| pr.url == url)
    }

    /// A tracked PR that has never loaded and that this session actually
    /// engaged with (not a bulk listing).
    fn next_unenriched_url(&self) -> Option<String> {
        self.prs
            .iter()
            .filter(|pr| {
                pr.refreshed_at == 0 && !pr.url.is_empty() && !pr.dismissed && self.engaged(pr)
            })
            .filter(|pr| {
                let failures = self.fetch_failures.get(&pr.url).copied().unwrap_or(0);
                self.refresh_attempted_at
                    .get(&pr.url)
                    .map(|t| t.elapsed() >= unenriched_retry_delay(failures))
                    .unwrap_or(true)
            })
            .max_by_key(|pr| pr.last_mentioned_at)
            .map(|pr| pr.url.clone())
    }

    /// The loaded PR that should get the one background read this poll is
    /// allowed to start. Every PR the session engaged with is eligible, open
    /// ones first; each waits out its own backoff.
    fn next_refresh_url(&self) -> Option<String> {
        let now = now_unix_ms();
        let mut tracked: Vec<&SessionPr> = self
            .prs
            .iter()
            .filter(|pr| {
                pr.refreshed_at != 0 && !pr.url.is_empty() && !pr.dismissed && self.engaged(pr)
            })
            .collect();
        tracked.sort_by(|a, b| {
            (b.state == "OPEN")
                .cmp(&(a.state == "OPEN"))
                .then(b.last_mentioned_at.cmp(&a.last_mentioned_at))
                .then(b.updated_at.cmp(&a.updated_at))
                .then(a.url.cmp(&b.url))
        });
        tracked
            .into_iter()
            .enumerate()
            .find(|(rank, pr)| {
                refresh_due(
                    pr,
                    self.refresh_attempted_at.get(&pr.url).map(Instant::elapsed),
                    Some(Duration::from_millis(now.saturating_sub(pr.refreshed_at))),
                    self.activity_age(pr, *rank < HOT_SESSION_PR_LIMIT, now),
                )
            })
            .map(|(_, pr)| pr.url.clone())
    }

    /// How long ago the PR last moved: GitHub last showed it changing, the
    /// session last mentioned or pushed to it, or, for the newest few PRs, the
    /// session's last prompt or completion.
    fn activity_age(&self, pr: &SessionPr, near_session: bool, now_ms: u64) -> Option<Duration> {
        let since =
            |at_ms: u64| (at_ms > 0).then(|| Duration::from_millis(now_ms.saturating_sub(at_ms)));
        let session_ms = self
            .session_activity_at
            .filter(|_| near_session)
            .map(|secs| (secs * 1000.0) as u64);
        [
            self.pr_active_at.get(&pr.url).map(Instant::elapsed),
            self.status_changed_at.get(&pr.url).map(Instant::elapsed),
            since(pr.last_mentioned_at),
            since(pr.updated_at),
            session_ms.and_then(since),
        ]
        .into_iter()
        .flatten()
        .min()
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
            let mentions = self.pending_mentions.remove(&key).unwrap_or_default();
            let pending_pr_active = self.pending_pr_active.remove(&key).unwrap_or(false);
            let mark_updated = self.pending_updates.remove(&key);
            // Most errors are silent: a PR we can't view right now (auth,
            // network, private) simply doesn't gain live stats and keeps its
            // placeholder. But when GitHub says the PR flat-out doesn't exist,
            // the placeholder was a scanning artifact — a doc example like
            // `o/r#500` or a line-wrapped `owner/repo#N` shorthand — and it
            // must not linger on the boards.
            match result {
                Some(JobResult::Pr(result)) => match result.data {
                    Ok(json) => {
                        let resolved_url = json.url.clone();
                        let threads = result.threads;
                        if let Some(url) = &result.requested_url {
                            self.force_refresh.remove(url);
                        }
                        let pr_active = if (result.pr_active || pending_pr_active)
                            && json.state == "OPEN"
                            && !json.url.is_empty()
                        {
                            Some(json.url.clone())
                        } else {
                            None
                        };
                        let before = self.prs.iter().find(|pr| pr.url == resolved_url).cloned();
                        changed |=
                            self.apply_fetch(json, result.requested_url, result.created_here);
                        if let Some(threads) = threads {
                            if self
                                .prs
                                .iter()
                                .any(|pr| pr.url == resolved_url && pr.state == "OPEN")
                            {
                                changed |= self.apply_threads(&resolved_url, threads);
                            }
                        }
                        if let Some(pr) = self.prs.iter_mut().find(|pr| pr.url == resolved_url) {
                            if before.is_some_and(|before| status_changed(&before, pr)) {
                                self.status_changed_at
                                    .insert(resolved_url.clone(), Instant::now());
                            }
                            for (user_authored, prompt_count, timestamp) in mentions {
                                changed |=
                                    bump_mention(pr, user_authored, prompt_count, Some(timestamp));
                            }
                            if mark_updated && !pr.updated_here {
                                pr.updated_here = true;
                                changed = true;
                            }
                        }
                        if let Some(url) = pr_active {
                            self.note_pr_active(&url);
                        }
                    }
                    Err(error) if budget::is_deferral(&error) => {
                        // The slot was gone by the time the read started. Give
                        // the allowance back and try on a later poll.
                        self.reads_this_window = self.reads_this_window.saturating_sub(1);
                        if mark_updated {
                            self.pending_updates.insert(key.clone());
                        }
                        if !mentions.is_empty() {
                            self.pending_mentions.insert(key.clone(), mentions);
                        }
                        if pending_pr_active {
                            self.pending_pr_active.insert(key.clone(), true);
                        }
                        if let Some(url) = &result.requested_url {
                            self.refresh_attempted_at.remove(url);
                            self.force_refresh.insert(url.clone());
                        } else if let Some((cwd, number)) = mention_lookup_key(&key) {
                            self.defer_lookup(&cwd, number);
                        }
                    }
                    Err(error) => {
                        if let Some(url) = &result.requested_url {
                            self.force_refresh.remove(url);
                            if pr_does_not_exist(&error) {
                                let before = self.prs.len();
                                // Never drop a PR that once enriched — a repo
                                // deleted later keeps its last known stats.
                                self.prs.retain(|pr| pr.url != *url || pr.refreshed_at > 0);
                                changed |= self.prs.len() != before;
                            } else {
                                // Real failure (auth, network, rate limit):
                                // remember it so the row can say so instead of
                                // sitting silently bare, and so retries back off.
                                *self.fetch_failures.entry(url.clone()).or_insert(0) += 1;
                                let brief = brief_error(&error);
                                if let Some(pr) = self.prs.iter_mut().find(|p| p.url == *url) {
                                    if pr.fetch_error != brief {
                                        pr.fetch_error = brief;
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                },
                None => {}
            }
        }

        // One due read per poll. Its result lands on a later poll.
        self.start_due_refresh();
        changed |= self.sync_fetch_limits();
        changed |= self.sync_pr_slack_threads();

        changed
    }

    fn sync_fetch_limits(&mut self) -> bool {
        if !self.prs.iter().any(|pr| pr.refreshed_at == 0) {
            return false;
        }
        let next_cost = if self.force_refresh.is_empty() && !self.deferred_lookups.is_empty() {
            budget::UNREPORTED_READ_POINTS
        } else {
            budget::READ_POINTS
        };
        let limited = !self.session_read_allowed()
            || budget::limit_hit_with_estimate(budget::Reader::Session, next_cost);
        let mut changed = false;
        for pr in &mut self.prs {
            let waiting = limited
                && pr.refreshed_at == 0
                && !pr.dismissed
                && !self.pending.contains_key(&pr.url);
            if pr.fetch_limited != waiting {
                pr.fetch_limited = waiting;
                changed = true;
            }
        }
        changed
    }

    /// Restart the fast refresh window for one PR the session just touched.
    fn note_pr_active(&mut self, url: &str) {
        self.pr_active_at.insert(url.to_string(), Instant::now());
    }

    /// The session's latest prompt or completion. Only the few most recently
    /// mentioned open PRs count it as activity that restarts their backoff.
    pub fn note_session_activity(&mut self, unix_secs: Option<f64>) {
        self.session_activity_at = unix_secs;
    }

    /// Merge a review-thread count into the PR it belongs to.
    fn apply_threads(&mut self, url: &str, threads: ReviewThreads) -> bool {
        let now = now_unix_ms();
        let Some(pr) = self.prs.iter_mut().find(|p| p.url == url) else {
            return false;
        };
        let mut changed =
            pr.unresolved_comments != threads.unresolved || pr.comments_url != threads.first_url;
        pr.unresolved_comments = threads.unresolved;
        pr.comments_url = threads.first_url;
        pr.comments_refreshed_at = now;
        // Slack links persist once seen: thread queries stop when a PR closes,
        // and the notification comment doesn't stop mattering when it does.
        if !threads.slack_urls.is_empty() && pr.slack_comment_urls != threads.slack_urls {
            pr.slack_comment_urls = threads.slack_urls;
            changed = true;
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
        self.fetch_failures.remove(&url);
        let origin_slack = self.current_origin_slack();
        let loc = PrLocation {
            owner,
            repo,
            number: json.number,
            url: url.clone(),
        };
        let has_checks = json.check_counts.is_some() || !json.status_check_rollup.is_empty();
        let has_reviews = json.check_counts.is_some() || !json.latest_reviews.is_empty();
        let fetched = session_pr_from_fetch(&loc, json, created_here);

        if let Some(existing) = self.prs.iter_mut().find(|p| p.url == url) {
            let before = existing.clone();
            existing.number = fetched.number;
            existing.branch = fetched.branch;
            existing.title = fetched.title;
            existing.state = fetched.state;
            existing.is_draft = fetched.is_draft;
            existing.additions = fetched.additions;
            existing.deletions = fetched.deletions;
            existing.changed_files = fetched.changed_files;
            existing.mergeable = fetched.mergeable;
            existing.merge_state_status = fetched.merge_state_status;
            existing.review_decision = fetched.review_decision;
            // A branch lookup doesn't ask about reviews. Don't clear a
            // dismissed-review flag the last full read already found.
            if has_reviews {
                existing.review_dismissed = fetched.review_dismissed;
            }
            existing.closed_at = fetched.closed_at;
            existing.updated_at = fetched.updated_at;
            if !fetched.author_login.is_empty() {
                existing.author_login = fetched.author_login;
            }
            if fetched.authored_by_viewer.is_some() {
                existing.authored_by_viewer = fetched.authored_by_viewer;
            }
            // A lookup that didn't ask about checks must not wipe the counts
            // already on the row.
            if has_checks {
                existing.checks_passed = fetched.checks_passed;
                existing.checks_failed = fetched.checks_failed;
                existing.checks_pending = fetched.checks_pending;
                existing.checks_total = fetched.checks_total;
                existing.ci_url = fetched.ci_url;
            }
            // Unresolved threads are only tracked while a PR is open; once it
            // merges or closes the count would freeze at a stale value, so drop
            // it rather than leave a badge that no longer refreshes.
            if existing.state != "OPEN" {
                existing.unresolved_comments = 0;
                existing.comments_url = String::new();
                existing.comments_refreshed_at = 0;
            }
            existing.fetch_error.clear();
            existing.fetch_limited = false;
            existing.refreshed_at = fetched.refreshed_at;
            let changed = *existing != before;
            if existing.state != "OPEN" {
                self.pr_active_at.remove(&url);
                self.force_refresh.remove(&url);
            }
            return changed;
        }

        self.prs.push(SessionPr {
            slack_origin_url: origin_slack,
            ..fetched
        });
        true
    }
}

/// Build a fresh SessionPr from one `gh pr view` payload. Everything the
/// fetch didn't answer for — mention counters, review threads, and the
/// classification — comes from the placeholder, so a new field only needs a
/// default in one place. Shared by the tracker's insert path and the board's
/// watched-PR enrichment.
fn session_pr_from_fetch(loc: &PrLocation, json: GhPrJson, created_here: bool) -> SessionPr {
    let (passed, failed, pending) = json
        .check_counts
        .unwrap_or_else(|| count_checks(&json.status_check_rollup));
    let ci_url = if json.check_counts.is_some() {
        // The background read asks for counts, not every job URL. A checks
        // tab link stays cheap; asking for each job is what exhausted the
        // hourly GraphQL budget.
        if passed + failed + pending > 0 && !loc.url.is_empty() {
            format!("{}/checks", loc.url)
        } else {
            String::new()
        }
    } else {
        ci_link(&json.status_check_rollup, &loc.url)
    };
    let closed_at = json.closed_at.as_deref().map_or(0, parse_iso_ms);
    let updated_at = json.updated_at.as_deref().map_or(0, parse_iso_ms);
    // Only meaningful while open: after a merge or close the dismissal
    // history no longer needs attention.
    let review_dismissed = json.state == "OPEN"
        && json
            .latest_reviews
            .iter()
            .any(|review| review.state == "DISMISSED");
    let author_login = json.author.map(|author| author.login).unwrap_or_default();
    let authored_by_viewer = match (author_login.is_empty(), json.viewer_login.is_empty()) {
        (false, false) => Some(author_login == json.viewer_login),
        _ => None,
    };
    SessionPr {
        branch: json.head_ref_name,
        title: json.title,
        state: json.state,
        is_draft: json.is_draft,
        additions: json.additions,
        deletions: json.deletions,
        changed_files: json.changed_files,
        mergeable: json.mergeable,
        merge_state_status: json.merge_state_status,
        review_decision: json.review_decision,
        review_dismissed,
        closed_at,
        updated_at,
        checks_passed: passed,
        checks_failed: failed,
        checks_pending: pending,
        checks_total: passed + failed + pending,
        ci_url,
        author_login,
        authored_by_viewer,
        refreshed_at: now_unix_ms(),
        ..SessionPr::placeholder(loc, created_here)
    }
}

/// One-shot enrichment for a board-watched PR: `gh pr view` plus the
/// review-thread count while it is open, combined into a standalone
/// SessionPr flagged `watched`. Runs `gh`, so call from a background thread.
/// Whether an open PR board may start a background GitHub read.
pub(crate) fn board_reads_allowed() -> bool {
    budget::can_start(budget::Reader::Board)
}

pub(crate) fn board_read_limit_hit() -> bool {
    budget::limit_hit(budget::Reader::Board)
}

/// A read that did not call GitHub because the shared budget said to wait.
pub(crate) fn github_read_deferred(error: &str) -> bool {
    budget::is_deferral(error)
}

pub fn fetch_watched_session_pr(url: &str) -> Result<SessionPr, String> {
    let (json, threads) = fetch_pr(url, budget::Reader::Board)?;
    let (owner, repo) = split_owner_repo(url).unwrap_or_default();
    let loc = PrLocation {
        owner,
        repo,
        number: json.number,
        url: url.to_string(),
    };
    let mut pr = session_pr_from_fetch(&loc, json, false);
    pr.watched = true;
    if pr.state == "OPEN" {
        if let Some(threads) = threads {
            pr.unresolved_comments = threads.unresolved;
            pr.comments_url = threads.first_url;
            pr.comments_refreshed_at = now_unix_ms();
            for slack_url in threads.slack_urls {
                if !pr.slack_comment_urls.contains(&slack_url) {
                    pr.slack_comment_urls.push(slack_url);
                }
            }
        }
    }
    Ok(pr)
}

fn load_slack_directory() -> SlackDirectory {
    #[cfg(test)]
    {
        SlackDirectory::default()
    }
    #[cfg(not(test))]
    {
        SlackDirectory::load()
    }
}

/// Delay before re-attempting a never-enriched PR: 30s doubling to a
/// four-minute ceiling, so a transient failure recovers quickly without
/// hammering `gh` when something is durably wrong (auth, private repo).
fn unenriched_retry_delay(failures: u32) -> Duration {
    REFRESH_THROTTLE * 2u32.pow(failures.min(3))
}

/// First meaningful line of a `gh` error, capped so a stack of stderr noise
/// doesn't travel through the mirror and cloud streams.
fn brief_error(error: &str) -> String {
    let line = error
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("gh pr view failed");
    let mut brief: String = line.chars().take(120).collect();
    if brief.len() < line.len() {
        brief.push('…');
    }
    brief
}

/// Where the CI column should link for this PR.
///
/// A red rollup goes straight to the first failing job, so clicking the `✗N CI`
/// cell lands on the logs that explain it. Anything else goes to the PR's Checks
/// tab, which lists every run in PR context.
fn ci_link(rollup: &[CheckEntry], pr_url: &str) -> String {
    if rollup.is_empty() || pr_url.is_empty() {
        return String::new();
    }
    rollup
        .iter()
        .find(|entry| entry.classify() == CheckClass::Fail)
        .and_then(CheckEntry::url)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{pr_url}/checks"))
}

/// Stamp one counted mention onto a PR. Always a change: the counters and the
/// recency stamps feed classification, the mirror, and the cloud stream.
fn bump_mention(
    pr: &mut SessionPr,
    user_authored: bool,
    prompt_count: u32,
    timestamp: Option<u64>,
) -> bool {
    let now = timestamp.unwrap_or_else(now_unix_ms);
    pr.mentions += 1;
    if user_authored {
        pr.user_mentions += 1;
    }
    if pr.first_mentioned_at == 0 || now < pr.first_mentioned_at {
        pr.first_mentioned_at = now;
    }
    pr.last_mentioned_at = pr.last_mentioned_at.max(now);
    pr.last_mention_prompt = pr.last_mention_prompt.max(prompt_count);
    true
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
    /// A PR identified by repository and number. `bulk` marks a reference from
    /// a line that enumerated several PRs at once — a quoted `gh pr list` or
    /// ticket dump — which still tracks the PR but doesn't count as a mention.
    Located { loc: PrLocation, bulk: bool },
    /// A marked `#123` with no repository — resolved against the session's repo.
    Mentioned { number: u64, bulk: bool },
    /// `gh pr <verb> <number>` with no repository. Resolved against the checkout
    /// even when `number` is too small for a prose mention. `updates` means the
    /// verb changes the PR.
    CommandNumber { number: u64, updates: bool },
    /// The same command when `--repo` or a pull request URL names the repository.
    CommandLocated { loc: PrLocation, updates: bool },
    /// The URL printed by a completed `gh pr create`, including a delayed result.
    Created(PrLocation),
    /// A push or PR edit ran, so the current branch's PR is worth resolving.
    Updated(String),
}

/// A prose line naming this many distinct PRs is a listing, not engagement.
const BULK_MENTION_LINE_THRESHOLD: usize = 4;

/// Find every PR association in a chunk of transcript text.
///
/// Pure, so the channel rules can be tested (and replayed over real transcripts)
/// without running `gh`. See [`PrTracker::scan_text`] for the rules themselves.
#[cfg(test)]
fn scan_events(text: &str) -> Vec<ScanEvent> {
    scan_events_with_known_prs(text, &HashSet::new())
}

fn scan_events_with_known_prs(text: &str, known_pr_numbers: &HashSet<u64>) -> Vec<ScanEvent> {
    let mut events = Vec::new();
    let mut creates = Vec::<PendingCreation>::new();
    let mut result_creates = Vec::new();
    let mut tool_creates = HashMap::new();
    let mut raw_creates = 0;

    for (channel, body) in split_channels(text) {
        match channel {
            Channel::Prose => {
                for line in body.lines() {
                    // Legacy unmarked scrollback can contain the command and
                    // its stdout together. Ordinary prose links prove no creation.
                    if line.starts_with("gh pr create") || line.contains("Bash(gh pr create") {
                        raw_creates += 1;
                    } else if raw_creates > 0 {
                        if let Some(loc) = standalone_pr_url(line) {
                            events.push(ScanEvent::Created(loc));
                            raw_creates -= 1;
                        }
                    }
                    if is_pr_update_command(line) {
                        events.push(ScanEvent::Updated(line.trim().to_string()));
                    }
                    push_prose_line(&mut events, line, known_pr_numbers);
                }
                events.extend(gh_pr_command_events(&body));
            }
            Channel::Tool => {
                result_creates.clear();
                let count = body.matches("gh pr create").count();
                if count > 0 {
                    result_creates.push(creates.len());
                    creates.push(PendingCreation {
                        remaining: count,
                        sessions: HashSet::new(),
                    });
                } else if body.contains("write_stdin") || body.contains("wait") {
                    let sessions = command_session_ids(&body);
                    result_creates.extend(creates.iter().enumerate().filter_map(|(i, create)| {
                        (create.remaining > 0 && !create.sessions.is_disjoint(&sessions))
                            .then_some(i)
                    }));
                }
                if let Some(call_id) = tool_call_id(&body) {
                    tool_creates.insert(call_id.to_string(), result_creates.clone());
                }
                for line in body.lines().filter(|line| is_pr_update_command(line)) {
                    events.push(ScanEvent::Updated(line.trim().to_string()));
                }
                events.extend(gh_pr_command_events(&body));
            }
            Channel::ToolResult => {
                let matching_creates = match tool_call_id(&body) {
                    Some(id) => tool_creates.get(id).cloned().unwrap_or_default(),
                    None => result_creates.clone(),
                };
                if matching_creates.is_empty() {
                    continue;
                }
                let output = creation_output(&body);
                let mut urls = output.urls.into_iter();
                for &index in &matching_creates {
                    let create = &mut creates[index];
                    for loc in urls.by_ref().take(create.remaining) {
                        create.remaining -= 1;
                        events.push(ScanEvent::Created(loc.clone()));
                        push_located(&mut events, vec![loc]);
                    }
                    // An async command or exec cell can finish in a later poll.
                    // Only polls of this execution may supply its creation URL.
                    create.sessions.extend(output.sessions.iter().cloned());
                    if create.sessions.is_empty() {
                        create.remaining = 0;
                    }
                }
            }
        }
    }
    events
}

fn tool_call_id(body: &str) -> Option<&str> {
    body.lines().next()?.strip_prefix("call_id: ")
}

#[derive(Default)]
struct PendingCreation {
    remaining: usize,
    sessions: HashSet<String>,
}

#[derive(Default)]
struct CreationOutput {
    urls: Vec<PrLocation>,
    sessions: HashSet<String>,
}

fn command_session_ids(text: &str) -> HashSet<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:session_id|cell_id)["']?\s*:\s*["']?([A-Za-z0-9_-]+)"#)
            .expect("valid command session regex")
    })
    .captures_iter(text)
    .map(|caps| caps[1].to_string())
    .collect()
}

/// gh prints the created PR on its own stdout line. JSON listings, cited
/// links, and "already exists" errors do not establish creation.
fn standalone_pr_url(line: &str) -> Option<PrLocation> {
    let line = line.trim();
    pr_urls(line).into_iter().find(|loc| loc.url == line)
}

/// Unwrap Codex execution results and MCP text blocks without scanning the
/// arbitrary fields of a JSON listing as if they were command stdout.
fn creation_output(text: &str) -> CreationOutput {
    let text = if tool_call_id(text).is_some() {
        text.split_once('\n').map_or("", |(_, body)| body)
    } else {
        text
    };
    fn read_value(value: &serde_json::Value, result: &mut CreationOutput, depth: usize) {
        match value {
            serde_json::Value::String(text) => read_text(text, result, depth + 1),
            serde_json::Value::Array(parts) => {
                for part in parts {
                    read_value(part, result, depth + 1);
                }
            }
            serde_json::Value::Object(object) => {
                if object
                    .get("exit_code")
                    .and_then(|v| v.as_i64())
                    .is_some_and(|code| code != 0)
                {
                    return;
                }
                for key in ["session_id", "cell_id"] {
                    if let Some(id) = object.get(key).filter(|v| v.is_string() || v.is_number()) {
                        result.sessions.insert(
                            id.as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| id.to_string()),
                        );
                    }
                }
                for key in ["output", "text", "content"] {
                    if let Some(value) = object.get(key) {
                        read_value(value, result, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }
    fn read_text(text: &str, result: &mut CreationOutput, depth: usize) {
        if depth > 12 {
            return;
        }
        if text.lines().any(|line| {
            line.strip_prefix("Process exited with code ")
                .or_else(|| line.strip_prefix("Exit code: "))
                .and_then(|code| code.trim().parse::<i32>().ok())
                .is_some_and(|code| code != 0)
        }) || text.lines().any(|line| {
            line.trim_start().starts_with("a pull request for branch")
                && line.contains("already exists")
        }) {
            return;
        }
        if let Ok(value) = serde_json::from_str(text) {
            read_value(&value, result, depth);
            return;
        }
        for line in text.lines() {
            if let Some(loc) = standalone_pr_url(line) {
                if !result.urls.contains(&loc) {
                    result.urls.push(loc);
                }
            } else if let Ok(value) = serde_json::from_str(line) {
                read_value(&value, result, depth + 1);
            } else {
                for marker in [
                    "Process running with session ID ",
                    "Script running with cell ID ",
                ] {
                    if let Some((_, tail)) = line.split_once(marker) {
                        if let Some(id) = tail.split_whitespace().next() {
                            result.sessions.insert(id.to_string());
                        }
                    }
                }
            }
        }
    }
    let mut result = CreationOutput::default();
    read_text(text, &mut result, 0);
    result
}

/// Whether a `gh` error means the PR definitively doesn't exist, as opposed
/// to a transient failure (network, auth, rate limit) that spares the row.
fn pr_does_not_exist(error: &str) -> bool {
    error.contains("Could not resolve to")
        || error.contains("HTTP 404")
        || error.contains("no pull requests found")
}

/// How long to wait between reads of one PR, given how long it had been
/// quiet at its last read. An open PR waits that long, between 30 seconds and
/// ten minutes, so a quiet PR backs off by doubling. Running CI on a PR that
/// moved recently is read every 15 or 30 seconds. A merged or closed PR is
/// read hourly.
fn refresh_interval(pr: &SessionPr, quiet: Option<Duration>) -> Duration {
    if matches!(pr.state.as_str(), "MERGED" | "CLOSED") {
        return FINISHED_REFRESH;
    }
    match quiet {
        Some(quiet) if pr.checks_pending > 0 && quiet < CI_FRESH_WINDOW => CI_FRESH_REFRESH,
        Some(quiet) if pr.checks_pending > 0 && quiet < CI_WINDOW => REFRESH_THROTTLE,
        quiet => quiet
            .unwrap_or(OPEN_REFRESH_CAP)
            .clamp(REFRESH_THROTTLE, OPEN_REFRESH_CAP),
    }
}

/// Whether a PR is due for another read. `attempt_ago` and `read_ago` are
/// the last try and the last success; `activity_age` is how long ago the PR
/// last moved ([`PrTracker::activity_age`]). Activity after the last read
/// makes the PR due again after 30 seconds.
pub fn refresh_due(
    pr: &SessionPr,
    attempt_ago: Option<Duration>,
    read_ago: Option<Duration>,
    activity_age: Option<Duration>,
) -> bool {
    let Some(last) = attempt_ago.into_iter().chain(read_ago).min() else {
        return true;
    };
    let quiet_then = activity_age.map(|age| age.saturating_sub(last));
    last >= refresh_interval(pr, quiet_then)
}

/// Whether two copies of a PR differ in what GitHub reported, apart from
/// when each was read. A first read isn't a change.
pub fn status_changed(before: &SessionPr, after: &SessionPr) -> bool {
    if before.refreshed_at == 0 {
        return false;
    }
    let mut was = after.clone();
    was.adopt_github_fields(before);
    was.refreshed_at = after.refreshed_at;
    was.comments_refreshed_at = after.comments_refreshed_at;
    was != *after
}

/// Command-targeted references — never part of a listing line.
fn push_located(events: &mut Vec<ScanEvent>, locations: Vec<PrLocation>) {
    for loc in locations {
        events.push(ScanEvent::Located { loc, bulk: false });
    }
}

/// Record every PR reference in one line of prose — URLs plus marked `#123`
/// mentions — tagging them all `bulk` when the line enumerates enough distinct
/// PRs to be a listing rather than engagement.
fn push_prose_line(events: &mut Vec<ScanEvent>, line: &str, known_pr_numbers: &HashSet<u64>) {
    let urls = pr_urls(line);
    let mut mentions = Vec::new();
    // A `#123` inside a PR link is part of the URL (or a review-thread anchor),
    // and the URL scan already covers it.
    if !line.contains("/pull/") {
        line_pr_mentions(line, known_pr_numbers, &mut mentions);
    }
    let mut distinct: Vec<String> = urls.iter().map(|loc| loc.url.clone()).collect();
    for (marker, number) in &mentions {
        let identity = match marker {
            PrMarker::Repo(owner, repo) => format!("{owner}/{repo}#{number}"),
            PrMarker::Unqualified => format!("#{number}"),
        };
        if !distinct.contains(&identity) {
            distinct.push(identity);
        }
    }
    let bulk = distinct.len() >= BULK_MENTION_LINE_THRESHOLD;
    for loc in urls {
        events.push(ScanEvent::Located { loc, bulk });
    }
    for (marker, number) in mentions {
        let event = match marker {
            // A repository-pinned mention needs no guessing.
            PrMarker::Repo(owner, repo) => ScanEvent::Located {
                loc: PrLocation::new(&owner, &repo, number),
                bulk,
            },
            PrMarker::Unqualified => ScanEvent::Mentioned { number, bulk },
        };
        events.push(event);
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
/// A command is only a target when `gh pr <verb>` starts it (`gh pr view 49`,
/// `gh pr merge 49 --repo owner/repo`, `gh pr checks <url>`). The same words
/// quoted inside another command — a `gh pr create --body` that cites a
/// related PR — are not targets. A number with no repository is resolved
/// against the session checkout. `--repo` or a pull request URL names one.
fn gh_pr_command_events(section: &str) -> Vec<ScanEvent> {
    let mut events = Vec::new();
    for caps in gh_pr_command_re().captures_iter(section) {
        let Some(verb) = caps.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let tail_start = caps.get(0).map(|m| m.end()).unwrap_or(section.len());
        let tail = command_tail(&section[tail_start..]);
        if let Some(event) = gh_pr_command_event(verb, tail) {
            events.push(event);
        }
    }
    events
}

/// Arguments of one `gh pr` invocation, stopping at the next command.
///
/// A raw quote ends the tail too. In a tool call the command is a JSON
/// string, so the closing quote is the end of the command and shell quotes
/// inside it are written as `\"`. Stopping there keeps the description that
/// follows (`"description":"Review PR 49"`) from looking like another number.
fn command_tail(text: &str) -> &str {
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '\n' | ';' | '|' | '&' | '"' | '\'') {
            return &text[..index];
        }
    }
    text
}

fn gh_pr_command_event(verb: &str, tail: &str) -> Option<ScanEvent> {
    let updates = matches!(
        verb.to_ascii_lowercase().as_str(),
        "merge" | "edit" | "ready" | "close" | "reopen"
    );
    let mut repo: Option<(String, String)> = None;
    let mut number: Option<u64> = None;
    let mut url: Option<PrLocation> = None;
    let mut tokens = tail.split_whitespace();
    while let Some(raw) = tokens.next() {
        let token = trim_shell_token(raw);
        if token.is_empty() {
            continue;
        }
        if let Some(value) = token
            .strip_prefix("--repo=")
            .or_else(|| token.strip_prefix("-R="))
        {
            repo = owner_repo_token(value);
            continue;
        }
        if token == "--repo" || token == "-R" {
            if let Some(value) = tokens.next() {
                repo = owner_repo_token(trim_shell_token(value));
            }
            continue;
        }
        if token.starts_with('-') {
            // `--json=fields` already carries its value. `--json fields` does not.
            if !token.contains('=') && flag_takes_value(token) {
                tokens.next();
            }
            continue;
        }
        if url.is_none() {
            if let Some(loc) = location_from_url(token) {
                url = Some(loc);
                continue;
            }
        }
        if number.is_none() {
            if let Some(found) = pure_pr_number(token) {
                number = Some(found);
            }
        }
    }
    if let Some(loc) = url {
        return Some(ScanEvent::CommandLocated { loc, updates });
    }
    if let Some((owner, repo_name)) = repo {
        let number = number?;
        return Some(ScanEvent::CommandLocated {
            loc: PrLocation::new(&owner, &repo_name, number),
            updates,
        });
    }
    Some(ScanEvent::CommandNumber {
        number: number?,
        updates,
    })
}

fn trim_shell_token(token: &str) -> &str {
    token.trim_matches(|c: char| matches!(c, '"' | '\'' | '\\' | ',' | ';' | '.'))
}

fn pure_pr_number(token: &str) -> Option<u64> {
    let digits = token.trim_start_matches('#');
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number = digits.parse().ok()?;
    (number > 0).then_some(number)
}

fn owner_repo_token(value: &str) -> Option<(String, String)> {
    let value = trim_shell_token(value);
    let (owner, repo) = value.split_once('/')?;
    let name = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    if !name(owner) || !name(repo) {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

fn flag_takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--json"
            | "--jq"
            | "--template"
            | "--subject"
            | "--body"
            | "--body-file"
            | "--title"
            | "--milestone"
            | "--label"
            | "--assignee"
            | "--reviewer"
            | "--project"
            | "--match"
            | "--author"
            | "--base"
            | "--head"
            | "--message"
            | "--comment"
            | "-b"
            | "-F"
            | "-t"
            | "-m"
            | "-f"
    )
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
/// A match either carries its own marker (`PR #972`), continues a comma/`and`
/// run started by one (`PRs #100, #101 and #102`), or names a PR the session
/// already tracks. Anything else — including the numbers in `Skipped #2, #3,
/// and #5` — is skipped, and skipping also breaks the run so a trailing list
/// can't attach to an earlier marker.
fn line_pr_mentions(line: &str, known_pr_numbers: &HashSet<u64>, out: &mut Vec<(PrMarker, u64)>) {
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
            if known_pr_numbers.contains(&number) {
                let mention = (PrMarker::Unqualified, number);
                if !out.contains(&mention) {
                    out.push(mention);
                }
            }
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

    let head = before.trim_end_matches(|c: char| c.is_whitespace() || c == ':' || c == '(');
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
        .all(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "and" | "or" | "&" | "+"
            )
        })
}

/// Scalar fields only. `latestReviews` and `statusCheckRollup` make `gh pr view`
/// request hundreds of GraphQL nodes; the background read asks for those
/// counts itself, with a fixed small page size.
const GH_JSON_FIELDS: &str =
    "number,title,headRefName,url,author,state,isDraft,additions,deletions,\
    changedFiles,mergeable,mergeStateStatus,reviewDecision,closedAt,updatedAt";

/// PR status, aggregate CI counts, and every review thread. Only threads are
/// paginated; check counts avoid enumerating expensive check-run connections.
// A raw string on purpose. A `\` line continuation in a regular string
// deletes the break and the indent, which glued `isDraft` to `additions`.
const PR_READ_QUERY: &str = r#"query($owner:String!,$repo:String!,$number:Int!,$endCursor:String){
     rateLimit{limit remaining resetAt cost}
     repository(owner:$owner,name:$repo){
       pullRequest(number:$number){
         number title url headRefName state isDraft
         additions deletions changedFiles
         mergeable mergeStateStatus reviewDecision
         closedAt updatedAt
         author{login}
         latestReviews(first:6){nodes{state}}
         commits(last:1){nodes{commit{statusCheckRollup{contexts{
           checkRunCount checkRunCountsByState{state count}
           statusContextCount statusContextCountsByState{state count}
         }}}}}
         reviewThreads(first:100,after:$endCursor){
           nodes{isResolved comments(first:1){nodes{url}}}
           pageInfo{hasNextPage endCursor}
         }
         comments(first:6){nodes{body}}
       }
     }
   }"#;

/// Full read of one PR URL, following every page of review threads.
fn fetch_pr(
    url: &str,
    reader: budget::Reader,
) -> Result<(GhPrJson, Option<ReviewThreads>), String> {
    let (pr, threads) = fetch_pr_graphql(url, reader)?;
    Ok((pr, Some(threads)))
}

/// `gh pr view` in `cwd` — resolves the current branch's PR. Checks and
/// threads are filled by a later URL read so this stays one cheap call.
fn fetch_pr_for_branch(
    cwd: &Path,
    reader: budget::Reader,
) -> Result<(GhPrJson, Option<ReviewThreads>), String> {
    Ok((
        fetch_pr_view(&["pr", "view", "--json", GH_JSON_FIELDS], Some(cwd), reader)?,
        None,
    ))
}

/// `gh pr view <number>` in `cwd` — validates a natural-language `#123`
/// reference against the current repository.
fn fetch_pr_number(
    cwd: &Path,
    number: u64,
    reader: budget::Reader,
) -> Result<(GhPrJson, Option<ReviewThreads>), String> {
    let number = number.to_string();
    Ok((
        fetch_pr_view(
            &["pr", "view", &number, "--json", GH_JSON_FIELDS],
            Some(cwd),
            reader,
        )?,
        None,
    ))
}

/// One `gh pr view`. `mergeable` is left as GitHub reported it; repeating the
/// call while it is UNKNOWN multiplies the cost of every read.
fn fetch_pr_view(
    args: &[&str],
    cwd: Option<&Path>,
    reader: budget::Reader,
) -> Result<GhPrJson, String> {
    let output = run_gh(reader, args, cwd)?;
    let mut json: GhPrJson =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("gh json parse: {e}"))?;
    json.viewer_login = gh_viewer_login().unwrap_or_default().to_string();
    Ok(json)
}

fn fetch_pr_graphql(
    url: &str,
    reader: budget::Reader,
) -> Result<(GhPrJson, ReviewThreads), String> {
    let caps = pr_url_re()
        .captures(url)
        .ok_or_else(|| format!("not a PR url: {url}"))?;
    let output = run_gh(
        reader,
        &[
            "api",
            "graphql",
            "--paginate",
            "--slurp",
            "-f",
            &format!("query={PR_READ_QUERY}"),
            "-f",
            &format!("owner={}", &caps[1]),
            "-f",
            &format!("repo={}", &caps[2]),
            "-F",
            &format!("number={}", &caps[3]),
        ],
        None,
    )?;
    let mut parsed = parse_pr_read(&output.stdout)?;
    parsed.pr.viewer_login = gh_viewer_login().unwrap_or_default().to_string();
    Ok((parsed.pr, parsed.threads))
}

/// Run `gh` once, counting it against the shared background budget.
fn run_gh(
    reader: budget::Reader,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<std::process::Output, String> {
    let mut permit = if args.starts_with(&["pr", "view"]) {
        budget::acquire_with_estimate(reader, budget::UNREPORTED_READ_POINTS)
    } else {
        budget::acquire(reader)
    }
    .ok_or_else(budget::deferral_reason)?;
    let mut cmd = Command::new("gh");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("gh not runnable: {e}"))?;
    // Record usage even if GitHub returned partial data with an error.
    if let Some(rate) = parse_rate_limit(&output.stdout) {
        permit.record_usage(
            rate.cost,
            rate.limit,
            rate.remaining,
            parse_iso_ms(&rate.reset_at),
        );
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let error = if stderr.is_empty() { stdout } else { stderr };
        if budget::looks_limited(&error) {
            budget::note_limited(&error);
        }
        return Err(error);
    }
    Ok(output)
}

/// Resolve the account authenticated in `gh` once per Crabigator process.
/// PR fetches already run on background threads, so this never blocks the UI.
fn gh_viewer_login() -> Option<&'static str> {
    static VIEWER_LOGIN: OnceLock<Option<String>> = OnceLock::new();
    VIEWER_LOGIN
        .get_or_init(|| {
            let output = Command::new("gh")
                .args(["api", "user", "--jq", ".login"])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let login = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (!login.is_empty()).then_some(login)
        })
        .as_deref()
}

struct ParsedPrRead {
    pr: GhPrJson,
    threads: ReviewThreads,
}

#[derive(Deserialize)]
struct PrReadResponse {
    #[serde(default)]
    data: Option<PrReadData>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PrReadOutput {
    Pages(Vec<PrReadResponse>),
    Single(Box<PrReadResponse>),
}

#[derive(Deserialize)]
struct GraphqlError {
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct PrReadData {
    #[serde(default)]
    repository: Option<PrReadRepository>,
}

#[derive(Deserialize)]
struct RateLimitInfo {
    #[serde(default)]
    limit: u32,
    cost: u32,
    remaining: u32,
    #[serde(default, rename = "resetAt")]
    reset_at: String,
}

fn parse_rate_limit(json: &[u8]) -> Option<RateLimitInfo> {
    let response: serde_json::Value = serde_json::from_slice(json).ok()?;
    let pages = match &response {
        serde_json::Value::Array(pages) => pages.as_slice(),
        page => std::slice::from_ref(page),
    };
    let mut total_cost: u32 = 0;
    let mut latest = None;
    for page in pages {
        if let Some(rate) = page.get("data").and_then(|data| data.get("rateLimit")) {
            if let Ok(mut rate) = serde_json::from_value::<RateLimitInfo>(rate.clone()) {
                total_cost = total_cost.saturating_add(rate.cost);
                rate.cost = total_cost;
                latest = Some(rate);
            }
        }
    }
    latest
}

#[derive(Deserialize)]
struct PrReadRepository {
    #[serde(default, rename = "pullRequest")]
    pull_request: Option<PrReadPull>,
}

#[derive(Deserialize)]
struct PrReadPull {
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default, rename = "headRefName")]
    head_ref_name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    author: Option<GhAuthor>,
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
    #[serde(default, rename = "reviewDecision")]
    review_decision: String,
    #[serde(default, rename = "latestReviews")]
    latest_reviews: ReviewNodes,
    #[serde(default, rename = "closedAt")]
    closed_at: Option<String>,
    #[serde(default, rename = "updatedAt")]
    updated_at: Option<String>,
    #[serde(default)]
    commits: CommitNodes,
    #[serde(default, rename = "reviewThreads")]
    review_threads: ThreadNodes,
    #[serde(default)]
    comments: IssueComments,
}

#[derive(Default, Deserialize)]
struct ReviewNodes {
    #[serde(default)]
    nodes: Vec<GhReview>,
}

#[derive(Default, Deserialize)]
struct CommitNodes {
    #[serde(default)]
    nodes: Vec<CommitNode>,
}

#[derive(Default, Deserialize)]
struct CommitNode {
    #[serde(default)]
    commit: CommitBody,
}

#[derive(Default, Deserialize)]
struct CommitBody {
    #[serde(default, rename = "statusCheckRollup")]
    status_check_rollup: Option<StatusRollup>,
}

#[derive(Default, Deserialize)]
struct StatusRollup {
    #[serde(default)]
    contexts: StatusContexts,
}

#[derive(Default, Deserialize)]
struct StatusContexts {
    #[serde(default, rename = "checkRunCountsByState")]
    check_runs: Vec<StateCount>,
    #[serde(default, rename = "statusContextCountsByState")]
    status_contexts: Vec<StateCount>,
}

#[derive(Default, Deserialize)]
struct StateCount {
    #[serde(default)]
    state: String,
    #[serde(default)]
    count: i64,
}

/// Turn one cheap PR read into the same stats `gh pr view` used to provide,
/// plus the review-thread badge.
fn parse_pr_read(json: &[u8]) -> Result<ParsedPrRead, String> {
    let output: PrReadOutput =
        serde_json::from_slice(json).map_err(|e| format!("gh json parse: {e}"))?;
    let pages = match output {
        PrReadOutput::Single(page) => vec![*page],
        PrReadOutput::Pages(pages) => pages,
    };
    let mut pulls = pages.into_iter().map(pr_read_pull);
    let mut pull = pulls
        .next()
        .ok_or_else(|| "gh json parse: missing PR pages".to_string())??;
    for next in pulls {
        let next = next?;
        pull.review_threads.nodes.extend(next.review_threads.nodes);
        pull.review_threads.page_info = next.review_threads.page_info;
    }
    if pull
        .review_threads
        .page_info
        .as_ref()
        .is_some_and(|page| page.has_next_page)
    {
        return Err("gh json parse: incomplete review threads".to_string());
    }
    Ok(pr_read_from_pull(pull))
}

fn pr_read_pull(response: PrReadResponse) -> Result<PrReadPull, String> {
    if let Some(error) = response
        .errors
        .into_iter()
        .find(|error| !error.message.is_empty())
    {
        if budget::looks_limited(&error.message) {
            budget::note_limited(&error.message);
        }
        return Err(error.message);
    }
    let data = response
        .data
        .ok_or_else(|| "gh json parse: missing data".to_string())?;
    data.repository
        .and_then(|repository| repository.pull_request)
        .ok_or_else(|| "Could not resolve to a PullRequest".to_string())
}

fn pr_read_from_pull(pull: PrReadPull) -> ParsedPrRead {
    let (passed, failed, pending) = tally_check_states(&pull.commits);
    let threads = review_threads_from(&pull.review_threads, &pull.comments);
    let pr = GhPrJson {
        number: pull.number,
        title: pull.title,
        head_ref_name: pull.head_ref_name,
        url: pull.url,
        author: pull.author,
        viewer_login: String::new(),
        state: pull.state,
        is_draft: pull.is_draft,
        additions: pull.additions,
        deletions: pull.deletions,
        changed_files: pull.changed_files,
        mergeable: pull.mergeable,
        merge_state_status: pull.merge_state_status,
        review_decision: pull.review_decision,
        latest_reviews: pull.latest_reviews.nodes,
        closed_at: pull.closed_at,
        updated_at: pull.updated_at,
        status_check_rollup: Vec::new(),
        check_counts: Some((passed, failed, pending)),
    };
    ParsedPrRead { pr, threads }
}

fn tally_check_states(commits: &CommitNodes) -> (i64, i64, i64) {
    let mut passed = 0;
    let mut failed = 0;
    let mut pending = 0;
    for node in &commits.nodes {
        let Some(rollup) = &node.commit.status_check_rollup else {
            continue;
        };
        for row in rollup
            .contexts
            .check_runs
            .iter()
            .chain(&rollup.contexts.status_contexts)
        {
            let count = row.count.max(0);
            match classify_rollup_state(&row.state) {
                CheckClass::Pass => passed += count,
                CheckClass::Fail => failed += count,
                CheckClass::Pending => pending += count,
            }
        }
    }
    (passed, failed, pending)
}

fn classify_rollup_state(state: &str) -> CheckClass {
    match state {
        "SUCCESS" | "NEUTRAL" | "SKIPPED" => CheckClass::Pass,
        "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" | "CANCELED" | "ACTION_REQUIRED"
        | "STARTUP_FAILURE" | "STALE" => CheckClass::Fail,
        _ => CheckClass::Pending,
    }
}

fn review_threads_from(threads: &ThreadNodes, comments: &IssueComments) -> ReviewThreads {
    let mut tally = ReviewThreads::default();
    for thread in threads.nodes.iter().filter(|thread| !thread.is_resolved) {
        tally.unresolved += 1;
        if tally.first_url.is_empty() {
            if let Some(comment) = thread.comments.nodes.first() {
                tally.first_url = comment.url.clone();
            }
        }
    }
    for comment in &comments.nodes {
        for thread in extract_threads(&comment.body) {
            let url = thread.url;
            if !tally.slack_urls.contains(&url) {
                tally.slack_urls.push(url);
            }
        }
    }
    tally
}

/// Tally the unresolved threads in a review-thread query response.
#[cfg(test)]
fn parse_review_threads(json: &[u8]) -> Result<ReviewThreads, String> {
    let response: ThreadsResponse =
        serde_json::from_slice(json).map_err(|e| format!("gh json parse: {e}"))?;
    Ok(review_threads_from(
        &response.data.repository.pull_request.review_threads,
        &response.data.repository.pull_request.comments,
    ))
}

/// `mention:/path#123` back into the lookup that should be retried.
fn mention_lookup_key(key: &str) -> Option<(PathBuf, u64)> {
    let rest = key.strip_prefix("mention:")?;
    let (cwd, number) = rest.rsplit_once('#')?;
    let number = number.parse().ok()?;
    Some((PathBuf::from(cwd), number))
}

fn split_owner_repo(url: &str) -> Option<(String, String)> {
    let caps = pr_url_re().captures(url)?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

/// Unix ms from an ISO 8601 timestamp (0 for empty or unparsable input).
fn parse_iso_ms(iso: &str) -> u64 {
    if iso.is_empty() {
        return 0;
    }
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.timestamp_millis().max(0) as u64)
        .unwrap_or(0)
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
    fn resume_restores_early_prs_for_claude_and_codex_without_repeating_old_pushes() {
        use crate::platforms::PlatformKind;
        use serde_json::json;
        for platform in [PlatformKind::Claude, PlatformKind::Codex] {
            let file = tempfile::NamedTempFile::new().unwrap();
            let cwd = file.path().parent().unwrap();
            let user = |text: &str| match platform {
                PlatformKind::Claude => {
                    json!({"type":"user","cwd":cwd,"timestamp":"2026-08-01T12:00:00Z","message":{"content":text}})
                }
                _ => {
                    json!({"type":"event_msg","timestamp":"2026-08-01T12:00:00Z","payload":{"type":"user_message","message":text}})
                }
            };
            let assistant = |text: &str| match platform {
                PlatformKind::Claude => {
                    json!({"type":"assistant","message":{"content":[{"type":"text","text":text}]}})
                }
                _ => json!({"type":"event_msg","payload":{"type":"agent_message","message":text}}),
            };
            let tool = |id: &str, command: &str| match platform {
                PlatformKind::Claude => {
                    json!({"type":"assistant","message":{"content":[{"type":"tool_use","id":id,"name":"Bash","input":{"command":command}}]}})
                }
                _ => {
                    json!({"type":"response_item","payload":{"type":"function_call","call_id":id,"name":"exec_command","arguments":json!({"cmd":command}).to_string()}})
                }
            };
            let output = |id: &str, text: &str| match platform {
                PlatformKind::Claude => {
                    json!({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":id,"content":text}]}})
                }
                _ => {
                    json!({"type":"response_item","payload":{"type":"function_call_output","call_id":id,"output":text}})
                }
            };
            let lines = [
                user("Combine the source changes into two integration PRs."),
                assistant("Sources: https://github.com/o/portal/pull/1437 and https://github.com/o/rqh/pull/2889."),
                tool("portal", "gh pr create"),
                output("portal", "https://github.com/o/portal/pull/1438"),
                tool("rqh", "gh pr create"),
                output("rqh", "https://github.com/o/rqh/pull/2890"),
                user("Review PR #1438."),
                assistant("Review complete."),
                user("Run checks."),
                tool("push", "git push"),
            ];
            std::fs::write(
                file.path(),
                lines
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n",
            )
            .unwrap();
            let update =
                crate::recap::collect_tracking_turns_incremental(platform, file.path(), &mut None)
                    .unwrap()
                    .unwrap();
            assert_eq!(update.turns.len(), 3);
            let mut tracker = PrTracker::new();
            for turn in &update.turns {
                tracker.scan_transcript_turn(turn, cwd, true);
            }
            assert!(
                tracker.pending.is_empty(),
                "historical pushes never resolve today's branch"
            );
            assert!(
                tracker.pr_active_at.is_empty(),
                "resume is not new PR activity"
            );
            for pr in &mut tracker.prs {
                pr.state = "OPEN".to_string();
            }
            tracker.reclassify("main", cwd);
            let primary: Vec<_> = tracker
                .prs
                .iter()
                .filter(|pr| pr.primary)
                .map(|pr| pr.number)
                .collect();
            assert_eq!(primary, vec![1438, 2890]);
            let portal = tracker.prs.iter().find(|pr| pr.number == 1438).unwrap();
            assert_eq!(
                (
                    portal.mentions,
                    portal.user_mentions,
                    portal.last_mention_prompt
                ),
                (2, 1, 2)
            );
            assert_eq!(
                portal.first_mentioned_at,
                parse_iso_ms("2026-08-01T12:00:00Z")
            );
            let before = tracker.prs.clone();
            tracker.scan_transcript_turn(update.turns.last().unwrap(), cwd, false);
            assert_eq!(tracker.prs, before);
            assert!(tracker.pending.is_empty());

            // Resume uses the same aging rules as an uninterrupted session.
            tracker.set_prompt_count(4);
            tracker.reclassify("main", cwd);
            assert!(tracker
                .prs
                .iter()
                .filter(|pr| [1437, 2889].contains(&pr.number))
                .all(|pr| pr.dismissed));
        }
    }

    #[test]
    fn restoring_a_watch_does_not_repost_it_to_the_cloud() {
        let mut tracker = PrTracker::new();
        let mut turn = crate::recap::TrackingTurn {
            transcript: crate::recap::TurnTranscript {
                user_prompt: Some("track PR https://github.com/o/portal/pull/1438".to_string()),
                activity: String::new(),
                turn_start: 0,
            },
            number: 1,
            timestamp: 1000,
            cwd: None,
        };
        tracker.scan_transcript_turn(&turn, Path::new("/tmp"), true);
        assert!(tracker.prs[0].watched);
        assert!(tracker.take_watch_adds().is_empty());
        // A later, explicit request is a new watch action.
        tracker
            .pending
            .insert(tracker.prs[0].url.clone(), mpsc::channel().1);
        turn.number = 2;
        tracker.scan_transcript_turn(&turn, Path::new("/tmp"), false);
        assert_eq!(tracker.take_watch_adds().len(), 1);
    }

    #[test]
    fn delayed_bare_pr_lookup_preserves_historical_mentions() {
        let mut tracker = PrTracker::new();
        let cwd = Path::new("/tmp");
        let key = "mention:/tmp#1438".to_string();
        let (sender, receiver) = mpsc::channel();
        tracker.pending.insert(key, receiver);
        for number in 1..=2 {
            tracker.scan_transcript_turn(
                &crate::recap::TrackingTurn {
                    transcript: crate::recap::TurnTranscript {
                        user_prompt: Some("Review PR #1438".to_string()),
                        activity: String::new(),
                        turn_start: 0,
                    },
                    number,
                    timestamp: number as u64 * 1000,
                    cwd: None,
                },
                cwd,
                true,
            );
        }
        sender.send(JobResult::Pr(Box::new(FetchResult {
            requested_url: None,
            created_here: false,
            pr_active: false,
            // Closed avoids unrelated periodic status/review jobs in this test.
            data: Ok(serde_json::from_value(serde_json::json!({"number":1438,"url":"https://github.com/o/portal/pull/1438","state":"CLOSED"})).unwrap()),
            threads: None,
        }))).unwrap();
        tracker.poll();
        let pr = &tracker.prs[0];
        assert_eq!(
            (pr.mentions, pr.user_mentions, pr.last_mention_prompt),
            (2, 2, 2)
        );
        assert_eq!((pr.first_mentioned_at, pr.last_mentioned_at), (1000, 2000));
        assert!(tracker.pr_active_at.is_empty());
        assert!(tracker.pending_mentions.is_empty());
    }

    #[test]
    fn a_pr_that_returns_after_a_reset_keeps_what_github_last_said() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation {
            owner: "o".to_string(),
            repo: "r".to_string(),
            number: 3223,
            url: "https://github.com/o/r/pull/3223".to_string(),
        };
        let mut fetched = SessionPr::placeholder(&loc, false);
        fetched.state = "OPEN".to_string();
        fetched.merge_state_status = "CLEAN".to_string();
        fetched.checks_passed = 31;
        fetched.checks_total = 31;
        fetched.mentions = 9;
        fetched.refreshed_at = 1_000;
        tracker.prs.push(fetched);

        tracker.reset_conversation();
        assert!(tracker.prs().is_empty());
        // Mentioned again (a replayed listing, so no read starts): last values.
        tracker.observe_url(&loc, false);
        let back = &tracker.prs()[0];
        assert_eq!(back.state, "OPEN");
        assert_eq!((back.checks_passed, back.checks_total), (31, 31));
        assert_eq!(back.refreshed_at, 1_000);
        assert_eq!(
            back.mentions, 0,
            "the new conversation counts its own mentions"
        );
        tracker.sync_fetch_limits();
        assert!(!tracker.prs()[0].fetch_limited);
    }

    /// Switching the pane to another conversation drops that conversation's
    /// PRs and Slack threads, while cloud dispositions and queued watch adds
    /// stay with the pane. The next prompt is then scanned fresh.
    #[test]
    fn reset_conversation_forgets_the_conversation_but_keeps_pane_state() {
        let mut tracker = PrTracker::new();
        tracker.scan_prompt(
            "Investigate https://t.slack.com/archives/C0OLD/p1723500000000000 and track PR https://github.com/o/r/pull/5",
            Path::new("/tmp"),
        );
        tracker.set_overrides(HashMap::from([(
            "o/r#9".to_string(),
            PrDisposition::Primary,
        )]));
        assert_eq!(tracker.prs().len(), 1);
        assert_eq!(tracker.slack_threads().len(), 1);
        assert!(tracker.session_slack_origin().is_some());

        tracker.reset_conversation();

        assert!(tracker.prs().is_empty());
        assert!(tracker.slack_threads().is_empty());
        assert!(tracker.pr_slack_threads().is_empty());
        assert!(tracker.session_slack_origin().is_none());
        assert_eq!(
            tracker.overrides.get("o/r#9"),
            Some(&PrDisposition::Primary),
            "cloud dispositions belong to the pane"
        );
        assert_eq!(
            tracker.take_watch_adds().len(),
            1,
            "queued watch adds still post"
        );

        tracker.scan_prompt(
            "Look at https://t.slack.com/archives/C0NEW/p1723600000000000 next",
            Path::new("/tmp"),
        );
        assert_eq!(tracker.slack_threads().len(), 1);
        assert!(tracker
            .session_slack_origin()
            .is_some_and(|url| url.contains("C0NEW")));
    }

    #[test]
    fn unenriched_retry_delay_doubles_then_caps() {
        assert_eq!(unenriched_retry_delay(0), Duration::from_secs(30));
        assert_eq!(unenriched_retry_delay(1), Duration::from_secs(60));
        assert_eq!(unenriched_retry_delay(2), Duration::from_secs(120));
        assert_eq!(unenriched_retry_delay(3), Duration::from_secs(240));
        assert_eq!(unenriched_retry_delay(12), Duration::from_secs(240));
    }

    #[test]
    fn brief_error_keeps_the_first_line_and_caps_length() {
        assert_eq!(brief_error("HTTP 403\nfull stack trace"), "HTTP 403");
        assert_eq!(brief_error("\n  spaced  \nrest"), "spaced");
        assert_eq!(brief_error(""), "gh pr view failed");
        let long = "x".repeat(300);
        assert_eq!(
            brief_error(&long).chars().count(),
            121,
            "120 chars + ellipsis"
        );
    }

    /// The PR numbers a chunk of text mentions in prose, in order.
    fn prose_pr_numbers(text: &str) -> Vec<u64> {
        prose_pr_mentions(text)
            .into_iter()
            .map(|(_, n)| n)
            .collect()
    }

    fn prose_pr_mentions(text: &str) -> Vec<(PrMarker, u64)> {
        let mut mentions = Vec::new();
        for (channel, body) in split_channels(text) {
            if channel != Channel::Prose {
                continue;
            }
            for line in body.lines().filter(|l| !l.contains("/pull/")) {
                line_pr_mentions(line, &HashSet::new(), &mut mentions);
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
    fn create_does_not_claim_unrelated_summary_links() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "gh pr create\nsummary: https://github.com/o/r/pull/1 and https://github.com/o/r/pull/2\n",
            Path::new("/tmp"),
        );
        assert_eq!(tracker.prs().len(), 2);
        assert_eq!(tracker.prs()[0].number, 1);
        assert_eq!(tracker.prs()[1].number, 2);
        assert!(tracker.prs().iter().all(|pr| !pr.created_here));
    }

    #[test]
    fn new_prompt_stops_marking_references_created_here() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("gh pr create", Path::new("/tmp"));
        // Next turn starts: a referenced PR is still associated, but not claimed.
        tracker.on_prompt_observed();
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
        assert!(!pr.updated_here);
    }

    /// tavus-mcp session 761bb011 merged PR 49 with `gh pr view 49` and
    /// `gh pr merge 49`. Neither command names the repository, and 49 is below
    /// the prose cutoff, so the pull request never reached the section.
    #[test]
    fn gh_commands_in_the_checkout_adopt_low_numbered_prs() {
        let mut tracker = PrTracker::new();
        let transcript = "\
[tool]
call_id: view
Bash {\"command\":\"gh pr view 49 --json title,body && gh pr diff 49\",\"description\":\"Review PR 49\"}
[tool]
call_id: merge
Bash {\"command\":\"gh repo view --json squashMergeAllowed && gh pr merge 49 --squash --subject \\\"fix (#49)\\\" && gh pr view 49 --json state\",\"description\":\"Squash-merge PR 49\"}
[tool]
call_id: release
Bash {\"command\":\"gh pr view 50 --json files && gh pr merge 50 --squash\",\"description\":\"Merge the release PR\"}
[tool]
call_id: previous
Bash {\"command\":\"gh pr view 48 --json statusCheckRollup; gh pr checks 50\",\"description\":\"Compare checks\"}
[assistant]
I merged Andy's PR #49. Merging #49 deployed staging. I then merged the release PR (#50, which only bumps version numbers).
";
        tracker.scan_text(transcript, Path::new("/tmp/tavus-mcp"));

        let queued = |number: u64| {
            let suffix = format!("#{number}");
            tracker.pending.keys().any(|key| key.ends_with(&suffix))
                || tracker
                    .deferred_lookups
                    .iter()
                    .any(|(_, queued)| *queued == number)
        };
        assert!(queued(49), "gh pr view 49 is this checkout's pull request");
        assert!(queued(50), "gh pr merge 50 is this checkout's pull request");
        assert!(
            queued(48),
            "gh pr view 48 is recorded, but only as a lookup"
        );
        assert!(
            tracker
                .pending_updates
                .iter()
                .any(|key| key.ends_with("#49")),
            "merging 49 marks it as work this session did"
        );
        assert!(
            tracker
                .pending_updates
                .iter()
                .any(|key| key.ends_with("#50")),
            "merging 50 marks it as work this session did"
        );
        assert!(
            !tracker
                .pending_updates
                .iter()
                .any(|key| key.ends_with("#48")),
            "viewing 48 does not mark it as work this session did"
        );

        let mut prose = PrTracker::new();
        prose.scan_text("I merged Andy's PR #49.", Path::new("/tmp/tavus-mcp"));
        assert!(prose.prs().is_empty());
        assert!(
            prose.pending.is_empty() && prose.deferred_lookups.is_empty(),
            "prose PR #49 stays below the cutoff"
        );
    }

    #[test]
    fn gh_pr_cited_inside_another_command_is_not_a_target() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "[tool]\nBash {\"command\":\"gh pr create --body 'see gh pr view 49 and https://github.com/o/r/pull/49'\"}\n",
            Path::new("/tmp"),
        );
        assert!(tracker.prs().is_empty());
        assert!(!tracker.pending.keys().any(|key| key.contains("#49")));
        assert!(!tracker
            .deferred_lookups
            .iter()
            .any(|(_, number)| *number == 49));
    }

    #[test]
    fn repo_qualified_merge_marks_updated_here_without_a_checkout_lookup() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "[tool]\nBash {\"command\":\"gh pr merge 49 --repo Tavus-Engineering/tavus-mcp --squash\"}\n",
            Path::new("/tmp/other"),
        );
        assert_eq!(tracker.prs().len(), 1);
        assert_eq!(tracker.prs()[0].number, 49);
        assert_eq!(tracker.prs()[0].repo, "tavus-mcp");
        assert!(tracker.prs()[0].updated_here);
        assert!(
            !tracker.pending.keys().any(|key| key.contains("mention:")),
            "a --repo command must not also guess the checkout"
        );
    }

    #[test]
    fn merge_command_marks_an_already_tracked_pr_updated_here() {
        let mut tracker = PrTracker::new();
        tracker
            .prs
            .push(SessionPr::test_stub(49, "Tavus-Engineering", "tavus-mcp"));
        tracker.scan_text(
            "[tool]\nBash {\"command\":\"gh pr merge 49 --squash\"}\n",
            Path::new("/tmp/tavus-mcp"),
        );
        assert!(tracker.prs()[0].updated_here);
        assert!(tracker.prs()[0].mentions >= 1);
        tracker.reclassify("main", Path::new("/tmp/tavus-mcp"));
        assert!(tracker.prs()[0].primary);
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

    /// The regression from the developer-portal audit: a marked-but-bare `#1`
    /// (Docker build steps, docs anchors, numbered findings) resolved against
    /// the session repo and adopted an unrelated PR by another author.
    #[test]
    fn low_numbered_bare_mentions_never_adopt() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("Let's revisit PR #7 from last week", Path::new("/tmp"));
        assert!(tracker.prs().is_empty());
        assert!(tracker.pending.is_empty(), "no gh lookup may be spawned");
    }

    #[test]
    fn low_numbered_bare_mentions_still_refresh_tracked_prs() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("see https://github.com/o/r/pull/7", Path::new("/tmp"));
        assert_eq!(tracker.prs().len(), 1);
        tracker.on_prompt_observed();
        tracker.pr_active_at.clear();
        tracker.scan_text("PR #7 is ready to merge", Path::new("/tmp"));
        assert!(
            tracker
                .pr_active_at
                .contains_key("https://github.com/o/r/pull/7"),
            "a bare mention of a tracked PR must restart its active window"
        );
    }

    #[test]
    fn titlecase_labels_keep_tracked_prs_recent() {
        let mut portal = SessionPr::test_stub(1139, "o", "developer-portal");
        portal.state = "OPEN".to_string();
        portal.created_here = true;
        portal.mentions = 1;
        portal.last_mention_prompt = 30;
        let mut rqh = SessionPr::test_stub(2642, "o", "request-handler");
        rqh.state = "OPEN".to_string();
        rqh.created_here = true;
        rqh.mentions = 1;
        rqh.last_mention_prompt = 30;
        let mut tracker = PrTracker::new();
        tracker.prs = vec![portal, rqh];
        tracker.set_prompt_count(38);

        tracker.scan_text(
            "[assistant]\nPortal #1139 and RQH #2642 are both active. Finding #9876 is not a PR.",
            Path::new("/tmp"),
        );
        tracker.reclassify("", Path::new("/tmp"));

        assert!(tracker.prs.iter().all(|pr| pr.primary));
        assert!(tracker.prs.iter().all(|pr| pr.last_mention_prompt == 38));
        assert!(
            !tracker.pending.contains_key("mention:/tmp#9876"),
            "untracked findings stay ignored"
        );
    }

    #[test]
    fn counts_mentions_once_per_occurrence_per_turn() {
        let mut tracker = PrTracker::new();
        let text = "working on https://github.com/o/r/pull/500 now";
        tracker.scan_text(text, Path::new("/tmp"));
        // The growing transcript is re-scanned every tick; the same occurrence
        // must not inflate the counters.
        tracker.scan_text(text, Path::new("/tmp"));
        let pr = &tracker.prs()[0];
        assert_eq!(pr.mentions, 1);
        assert_eq!(pr.user_mentions, 0);
        assert!(pr.first_mentioned_at > 0);
        assert_eq!(pr.first_mentioned_at, pr.last_mentioned_at);

        // A new turn mentioning it again counts a second time.
        tracker.on_prompt_observed();
        tracker.scan_text(text, Path::new("/tmp"));
        assert_eq!(tracker.prs()[0].mentions, 2);
    }

    #[test]
    fn user_prompt_mentions_count_separately() {
        let mut tracker = PrTracker::new();
        tracker.set_prompt_count(7);
        tracker.scan_prompt(
            "please rebase https://github.com/o/r/pull/500",
            Path::new("/tmp"),
        );
        let pr = &tracker.prs()[0];
        assert_eq!(pr.mentions, 1);
        assert_eq!(pr.user_mentions, 1);
        assert_eq!(pr.last_mention_prompt, 7);
    }

    /// A `gh pr list` dump quoted into prose names every PR the session never
    /// touched; those references track the PRs but must not read as engagement.
    #[test]
    fn bulk_listing_lines_track_without_counting() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "open: https://github.com/o/r/pull/501 https://github.com/o/r/pull/502 \
             https://github.com/o/r/pull/503 https://github.com/o/r/pull/504",
            Path::new("/tmp"),
        );
        assert_eq!(tracker.prs().len(), 4);
        assert!(tracker.prs().iter().all(|pr| pr.mentions == 0));
        assert!(tracker.prs().iter().all(|pr| pr.last_mentioned_at == 0));
    }

    #[test]
    fn short_pr_runs_still_count() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "merged https://github.com/o/r/pull/501 and https://github.com/o/r/pull/502",
            Path::new("/tmp"),
        );
        assert_eq!(tracker.prs().len(), 2);
        assert!(tracker.prs().iter().all(|pr| pr.mentions == 1));
    }

    #[test]
    fn prompt_declarations_classify_prs() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("see https://github.com/o/r/pull/500", Path::new("/tmp"));
        tracker.scan_prompt("I think PR #500 is the primary here", Path::new("/tmp"));
        tracker.reclassify("", Path::new("/tmp"));
        let pr = &tracker.prs()[0];
        assert!(pr.primary);
        assert_eq!(pr.primary_source, "session");
    }

    #[test]
    fn dismissal_statements_hide_prs() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("see https://github.com/o/r/pull/500", Path::new("/tmp"));
        tracker.scan_prompt("dismiss PR #500, it landed last week", Path::new("/tmp"));
        tracker.reclassify("", Path::new("/tmp"));
        assert!(tracker.prs()[0].dismissed);
        // But "remove the flag from PR #500" must NOT dismiss.
        let mut tracker = PrTracker::new();
        tracker.scan_text("see https://github.com/o/r/pull/500", Path::new("/tmp"));
        tracker.scan_prompt("remove the flag from PR #500", Path::new("/tmp"));
        tracker.reclassify("", Path::new("/tmp"));
        assert!(!tracker.prs()[0].dismissed);
    }

    #[test]
    fn watch_targets_parse_urls_and_repo_shorthand() {
        let add = parse_watch_target(" https://github.com/o/r/pull/77 ").unwrap();
        assert_eq!(
            (add.owner.as_str(), add.repo.as_str(), add.number),
            ("o", "r", 77)
        );
        let add = parse_watch_target("octo/hello-world#123").unwrap();
        assert_eq!(add.url, "https://github.com/octo/hello-world/pull/123");
        assert!(
            parse_watch_target("#123").is_none(),
            "a bare number has no repo"
        );
        assert!(parse_watch_target("not a pr").is_none());
        assert!(parse_watch_target("o/r#0").is_none());
    }

    /// "track PR <url>" and "watch owner/repo#N" track the PR, flag it
    /// watched, and queue exactly one cloud watch-list add each.
    #[test]
    fn track_statements_watch_the_pr_and_queue_a_cloud_add() {
        let mut tracker = PrTracker::new();
        tracker.scan_prompt(
            "track PR https://github.com/o/r/pull/812 for me",
            Path::new("/tmp"),
        );
        assert!(
            tracker.prs()[0].watched,
            "the tracked PR is flagged watched"
        );

        tracker.scan_prompt("also watch other/repo#4102 please", Path::new("/tmp"));
        let adds = tracker.take_watch_adds();
        assert_eq!(
            adds,
            vec![
                WatchAdd {
                    owner: "o".to_string(),
                    repo: "r".to_string(),
                    number: 812,
                    url: "https://github.com/o/r/pull/812".to_string(),
                },
                WatchAdd {
                    owner: "other".to_string(),
                    repo: "repo".to_string(),
                    number: 4102,
                    url: "https://github.com/other/repo/pull/4102".to_string(),
                },
            ],
        );
        assert!(tracker.take_watch_adds().is_empty(), "adds drain once");

        // Prose that merely contains the words never queues a watch.
        let mut tracker = PrTracker::new();
        tracker.scan_prompt("watch the tests and track progress", Path::new("/tmp"));
        assert!(tracker.take_watch_adds().is_empty());
    }

    #[test]
    fn bare_mentions_at_the_threshold_still_adopt() {
        let mut tracker = PrTracker::new();
        tracker.scan_text("PR #100 tracks the rollout", Path::new("/tmp"));
        assert!(
            tracker.pending.keys().any(|k| k.starts_with("mention:")),
            "a bare #100 should still be looked up against the session repo"
        );
    }

    #[test]
    fn accepts_marked_numbers() {
        assert_eq!(prose_pr_numbers("PR #972 is green"), vec![972]);
        assert_eq!(prose_pr_numbers("pull request #972 landed"), vec![972]);
        assert_eq!(prose_pr_numbers("PR: #972 needs a rebase"), vec![972]);
        assert_eq!(prose_pr_numbers("(see PR #972)"), vec![972]);
        assert_eq!(
            prose_pr_numbers("the release PR (#50, which only bumps versions)"),
            vec![50]
        );
        assert_eq!(
            prose_pr_numbers("developer-portal#972 conflicts"),
            vec![972]
        );
        assert_eq!(prose_pr_numbers("RQH #2469 is next"), vec![2469]);
        // Conventional prose markers are not repo nicknames.
        assert_eq!(prose_pr_numbers("SEV #2 postmortem"), Vec::<u64>::new());
        assert_eq!(prose_pr_numbers("TODO #3 later"), Vec::<u64>::new());
        assert_eq!(
            prose_pr_numbers("PROD-1234 #5 is unrelated"),
            Vec::<u64>::new()
        );
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
    fn create_results_follow_their_background_command() {
        let text = r#"[tool]
exec_command {"cmd":"gh pr create --body-file /tmp/body.md"}
[tool_result]
{"session_id":42,"output":""}
[tool]
exec_command {"cmd":"gh pr view 90 --json url"}
[tool_result]
https://github.com/o/r/pull/90
[tool]
write_stdin {"session_id":99}
[tool_result]
https://github.com/o/r/pull/99
[tool]
write_stdin {"session_id":42}
[tool_result]
{"exit_code":0,"output":"https://github.com/o/r/pull/100\n"}
[assistant]
Created https://github.com/o/r/pull/100; depends on https://github.com/o/r/pull/90.
"#;
        let created: Vec<_> = scan_events(text)
            .into_iter()
            .filter_map(|event| match event {
                ScanEvent::Created(loc) => Some(loc.number),
                _ => None,
            })
            .collect();
        assert_eq!(created, vec![100]);
    }

    #[test]
    fn failed_create_does_not_claim_an_existing_pr_or_later_output() {
        let text = r#"[tool]
exec_command {"cmd":"gh pr create"}
[tool_result]
{"exit_code":1,"output":"already exists:\nhttps://github.com/o/r/pull/90\n"}
[tool]
exec_command {"cmd":"gh pr list --json url"}
[tool_result]
[{"url":"https://github.com/o/r/pull/91"}]
[assistant]
See https://github.com/o/r/pull/92 for context.
"#;
        assert!(!scan_events(text)
            .iter()
            .any(|event| matches!(event, ScanEvent::Created(_))));
    }

    #[test]
    fn plain_failed_create_does_not_claim_an_existing_pr() {
        let text = "[tool]\nexec_command gh pr create\n[tool_result]\nProcess exited with code 1\nFinal output:\na pull request for branch work already exists:\nhttps://github.com/o/r/pull/90\n";
        assert!(!scan_events(text)
            .iter()
            .any(|event| matches!(event, ScanEvent::Created(_))));
    }

    #[test]
    fn interleaved_tool_results_keep_their_creation_owner() {
        let text = "[tool]\ncall_id: create\nexec_command gh pr create\n[tool]\ncall_id: inspect\nexec_command gh pr view 90\n[tool_result]\ncall_id: inspect\nhttps://github.com/o/r/pull/90\n[tool_result]\ncall_id: create\nhttps://github.com/o/r/pull/100\n";
        let created: Vec<_> = scan_events(text)
            .into_iter()
            .filter_map(|event| match event {
                ScanEvent::Created(loc) => Some(loc.number),
                _ => None,
            })
            .collect();
        assert_eq!(created, vec![100]);
    }

    #[test]
    fn creation_output_reads_pretty_json_after_call_metadata() {
        let output = creation_output("call_id: create\n{\n  \"output\": \"https://github.com/o/r/pull/100\\n\",\n  \"exit_code\": 0\n}");
        assert_eq!(output.urls, vec![PrLocation::new("o", "r", 100)]);
    }

    #[test]
    fn long_codex_turn_keeps_only_created_integration_prs_primary() {
        use serde_json::json;
        let portal = "https://github.com/o/portal/pull/1438";
        let rqh = "https://github.com/o/rqh/pull/2890";
        let mut lines = Vec::new();
        let mut record =
            |payload| lines.push(json!({"type":"response_item", "payload":payload}).to_string());
        record(
            json!({"type":"message","role":"user","content":[{"text":"Combine the source changes into linked integration PRs."}]}),
        );
        record(
            json!({"type":"message","role":"assistant","content":[{"text":"Sources: https://github.com/o/portal/pull/1437 and https://github.com/o/rqh/pull/2889."}]}),
        );
        // The create sits in the middle of a long functions.exec input, with
        // its output delivered by a later write_stdin call.
        let padding = "body text ".repeat(200);
        record(
            json!({"type":"custom_tool_call","call_id":"create-portal","name":"exec","input":format!("{padding}\ntext(await tools.exec_command({{cmd:\"gh pr create\"}}));\n{padding}")}),
        );
        record(
            json!({"type":"custom_tool_call_output","call_id":"create-portal","output":[{"type":"input_text","text":"{\"session_id\":42,\"output\":\"\"}"}]}),
        );
        record(
            json!({"type":"function_call","call_id":"checks","name":"exec_command","arguments":"{\"cmd\":\"gh pr checks 66 --repo o/evals\"}"}),
        );
        record(
            json!({"type":"function_call_output","call_id":"checks","output":"https://github.com/o/evals/pull/66"}),
        );
        record(
            json!({"type":"custom_tool_call","call_id":"poll","name":"exec","input":"text(await tools.write_stdin({session_id:42}));"}),
        );
        record(
            json!({"type":"custom_tool_call_output","call_id":"poll","output":[{"type":"input_text","text":json!({"exit_code":0,"output":format!("{padding}\n{portal}\n")}).to_string()}]}),
        );
        // A conventional JSON tool call has its create beyond the old preview
        // limit too. Its result can arrive after another tool was started.
        record(
            json!({"type":"function_call","call_id":"create-rqh","name":"exec_command","arguments":json!({"cmd":format!("{padding}\ngh pr create")}).to_string()}),
        );
        record(
            json!({"type":"function_call","call_id":"inspect","name":"exec_command","arguments":"{\"cmd\":\"gh pr view 66 --repo o/evals\"}"}),
        );
        record(
            json!({"type":"function_call_output","call_id":"inspect","output":"https://github.com/o/evals/pull/66"}),
        );
        record(json!({"type":"function_call_output","call_id":"create-rqh","output":rqh}));
        record(
            json!({"type":"message","role":"assistant","content":[{"text":format!("Opened {portal} and {rqh}. Source PR https://github.com/o/evals/pull/66 remains separate.")}]}),
        );
        record(
            json!({"type":"message","role":"assistant","content":[{"text":"Running the integration checks. ".repeat(2_000)}]}),
        );
        let file = tempfile::NamedTempFile::new().unwrap();
        // JSONL records end with a newline. Without one, the last line is
        // still being written and the reader leaves it for the next pass.
        std::fs::write(file.path(), format!("{}\n", lines.join("\n"))).unwrap();
        let platform = crate::platforms::PlatformKind::Codex;
        let turn =
            crate::recap::collect_latest_turn_text_incremental(platform, file.path(), &mut None)
                .unwrap()
                .unwrap();
        assert!(turn.activity.len() > 28_000);
        let summary = crate::recap::collect_latest_turn_text(platform, Some(file.path())).unwrap();
        assert!(summary.activity.chars().count() < 28_100);

        let cwd = Path::new("/tmp/crabigator-pr-test");
        let mut tracker = PrTracker::new();
        for (number, repo) in [
            (1437, "portal"),
            (2889, "rqh"),
            (66, "evals"),
            (1438, "portal"),
            (2890, "rqh"),
        ] {
            let mut pr = SessionPr::test_stub(number, "o", repo);
            pr.state = "OPEN".to_string();
            tracker.pending.insert(pr.url.clone(), mpsc::channel().1);
            tracker.prs.push(pr);
        }
        tracker.set_prompt_count(1);
        tracker.scan_prompt(turn.user_prompt.as_deref().unwrap(), cwd);
        tracker.scan_text(&turn.activity, cwd);
        tracker.reclassify("main", cwd);
        let primaries: Vec<_> = tracker
            .prs()
            .iter()
            .filter(|pr| pr.primary)
            .map(|pr| pr.url.as_str())
            .collect();
        assert_eq!(primaries, vec![portal, rqh]);
        assert!(tracker.prs()[..3].iter().all(|pr| !pr.created_here));
        let before = tracker.prs.clone();
        let activity_before = tracker.pr_active_at.clone();
        assert!(!tracker.scan_text(&turn.activity, cwd));
        assert_eq!(
            tracker.prs, before,
            "rescans must not inflate mention counts"
        );
        assert_eq!(
            tracker.pr_active_at, activity_before,
            "rescans must not restart polling"
        );
    }

    #[test]
    fn create_results_follow_a_yielded_exec_cell() {
        let text = r#"[tool]
exec text(await tools.exec_command({cmd:"gh pr create"}));
[tool_result]
Script running with cell ID 17
[tool]
functions.wait {"cell_id":"17"}
[tool_result]
{"content":[{"type":"text","text":"{\"exit_code\":0,\"output\":\"https://github.com/o/r/pull/100\"}"}]}
"#;
        assert!(scan_events(text)
            .iter()
            .any(|event| matches!(event, ScanEvent::Created(loc) if loc.number == 100)));
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
            Ok("grok") => crate::platforms::PlatformKind::Grok,
            Ok("opencode") => crate::platforms::PlatformKind::Opencode,
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
            let Ok(Some(text)) = crate::recap::collect_latest_turn_text_incremental(
                platform,
                &replay_path,
                &mut None,
            ) else {
                continue;
            };
            scanned_chars += text.activity.len() + text.user_prompt.as_deref().unwrap_or("").len();
            let mut events = scan_events(text.user_prompt.as_deref().unwrap_or_default());
            events.extend(scan_events(&text.activity));
            for event in events {
                match event {
                    ScanEvent::Located { loc, .. } if !located.contains(&loc) => {
                        eprintln!(
                            "turn {turn}: located {}/{} #{}",
                            loc.owner, loc.repo, loc.number
                        );
                        located.push(loc);
                    }
                    // Mirrors `resolve_mentioned_pr`: a number already located with
                    // its own repository is never guessed against the cwd.
                    ScanEvent::Mentioned { number, .. }
                        if !mentioned.contains(&number)
                            && !located.iter().any(|l| l.number == number) =>
                    {
                        eprintln!("turn {turn}: mentioned #{number} (resolved against cwd)");
                        mentioned.push(number);
                    }
                    ScanEvent::CommandNumber { number, .. }
                        if !mentioned.contains(&number)
                            && !located.iter().any(|l| l.number == number) =>
                    {
                        eprintln!("turn {turn}: gh command #{number} (resolved against cwd)");
                        mentioned.push(number);
                    }
                    ScanEvent::CommandLocated { loc, .. } if !located.contains(&loc) => {
                        eprintln!(
                            "turn {turn}: gh command {}/{} #{}",
                            loc.owner, loc.repo, loc.number
                        );
                        located.push(loc);
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

    /// Read-only audit of a live session. Rebuild its full conversation ownership
    /// from the transcript, using mirrored GitHub facts without network calls.
    #[test]
    #[ignore]
    fn replay_pr_classification_from_mirror() {
        let path =
            std::env::var("CRABIGATOR_MIRROR").expect("set CRABIGATOR_MIRROR=<inspect.json>");
        let mirror: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let transcript = Path::new(mirror["transcript_path"].as_str().unwrap());
        let mut update = crate::recap::collect_tracking_turns_incremental(
            crate::platforms::PlatformKind::Codex,
            transcript,
            &mut None,
        )
        .unwrap()
        .unwrap();
        let mut tracker = PrTracker::new();
        tracker.prs = serde_json::from_value(mirror["prs"].clone()).unwrap();
        for pr in &mut tracker.prs {
            pr.created_here = false;
            pr.branch_matched = false;
            pr.mentions = 0;
            pr.user_mentions = 0;
            pr.first_mentioned_at = 0;
            pr.last_mentioned_at = 0;
            pr.last_mention_prompt = 0;
            tracker.pending.insert(pr.url.clone(), mpsc::channel().1);
        }
        let cwd = Path::new("/tmp/crabigator-pr-audit");
        tracker
            .pending
            .insert(format!("branch:{}", cwd.display()), mpsc::channel().1);
        for turn in &mut update.turns {
            // Unknown bare numbers remain unresolved in this offline audit.
            turn.cwd = None;
            let text = format!(
                "{}\n{}",
                turn.transcript.user_prompt.as_deref().unwrap_or_default(),
                turn.transcript.activity
            );
            for event in scan_events(&text) {
                let number = match event {
                    ScanEvent::Mentioned { number, .. }
                    | ScanEvent::CommandNumber { number, .. } => number,
                    _ => continue,
                };
                tracker.pending.insert(
                    format!("mention:{}#{number}", cwd.display()),
                    mpsc::channel().1,
                );
            }
            tracker.scan_transcript_turn(turn, cwd, true);
        }
        tracker.reclassify("main", cwd);
        let mut primary: Vec<_> = tracker
            .prs()
            .iter()
            .filter(|pr| pr.primary)
            .map(|pr| format!("{}/{}#{}", pr.owner, pr.repo, pr.number))
            .collect();
        primary.sort();
        eprintln!(
            "Replayed {} turns, {} characters. Primary PRs: {primary:?}",
            update.turns.len(),
            update
                .turns
                .iter()
                .map(|turn| turn.transcript.activity.len())
                .sum::<usize>()
        );
        if let Ok(expected) = std::env::var("CRABIGATOR_EXPECT_PRIMARY") {
            let mut expected: Vec<_> = expected.split(',').map(str::to_string).collect();
            expected.sort();
            assert_eq!(primary, expected);
        }
    }

    /// End-to-end against the live GitHub CLI: run the review-thread query and
    /// parse what comes back, which is the part unit tests can't check (arg
    /// typing, query text, auth). Thread counts change as people review, so it
    /// asserts the shape rather than a number. Network + `gh` auth required:
    ///   cargo test pr::tests::end_to_end_counts_review_threads -- --ignored --nocapture
    #[test]
    #[ignore]
    fn end_to_end_counts_review_threads() {
        let url = std::env::var("CRABIGATOR_PR_URL")
            .unwrap_or_else(|_| "https://github.com/rust-lang/rust/pull/135000".to_string());
        let (pr, threads) = fetch_pr_graphql(&url, budget::Reader::Session)
            .expect("production paginated PR query runs");
        eprintln!("CI counts (passed, failed, pending): {:?}", pr.check_counts);
        eprintln!(
            "{url}\n  unresolved={} first={}",
            threads.unresolved,
            if threads.first_url.is_empty() {
                "(none)"
            } else {
                &threads.first_url
            }
        );
        // A count and a link must agree: either both are present, or neither is.
        assert!(threads.first_url.is_empty() ^ (threads.unresolved > 0));
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

    /// A completed check run in the given class, optionally with its own page.
    fn check(class: CheckClass, url: Option<&str>) -> CheckEntry {
        CheckEntry {
            conclusion: match class {
                CheckClass::Pass => Some("SUCCESS".into()),
                CheckClass::Fail => Some("FAILURE".into()),
                CheckClass::Pending => None,
            },
            status: Some(match class {
                CheckClass::Pending => "IN_PROGRESS".into(),
                _ => "COMPLETED".to_string(),
            }),
            state: None,
            details_url: url.map(str::to_string),
            target_url: None,
        }
    }

    /// Verbatim `gh api graphql` output for a PR with two unresolved threads,
    /// so the field renames are checked against the shape GitHub really sends.
    const LIVE_THREADS_JSON: &[u8] = br#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[
        {"isResolved":false,"comments":{"nodes":[{"url":"https://github.com/Tavus-Engineering/request-handler/pull/2501#discussion_r3648377388"}]}},
        {"isResolved":true,"comments":{"nodes":[{"url":"https://github.com/Tavus-Engineering/request-handler/pull/2501#discussion_r3648300000"}]}},
        {"isResolved":false,"comments":{"nodes":[{"url":"https://github.com/Tavus-Engineering/request-handler/pull/2501#discussion_r3648377828"}]}}
    ]}}}}}"#;

    #[test]
    fn review_threads_count_the_unresolved_and_link_the_first() {
        let threads = parse_review_threads(LIVE_THREADS_JSON).expect("parses gh output");
        assert_eq!(threads.unresolved, 2);
        assert_eq!(
            threads.first_url,
            "https://github.com/Tavus-Engineering/request-handler/pull/2501#discussion_r3648377388"
        );

        // A settled conversation leaves nothing to show or click.
        let settled = br#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[
            {"isResolved":true,"comments":{"nodes":[{"url":"https://example.com/x"}]}}]}}}}}"#;
        assert_eq!(
            parse_review_threads(settled).expect("parses"),
            ReviewThreads::default()
        );
        // A thread whose comment GitHub withheld still counts.
        let no_comment = br#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[
            {"isResolved":false,"comments":{"nodes":[]}}]}}}}}"#;
        let bare = parse_review_threads(no_comment).expect("parses");
        assert_eq!((bare.unresolved, bare.first_url.as_str()), (1, ""));
    }

    /// PR comments carry Slack notification permalinks (posted by bots) and
    /// human-pasted thread links; all are collected, deduped, in order.
    #[test]
    fn review_threads_collect_slack_permalinks_from_comments() {
        let with_comments = br#"{"data":{"repository":{"pullRequest":{
            "reviewThreads":{"nodes":[]},
            "comments":{"nodes":[
                {"body":"PR notification sent: https://tavus.slack.com/archives/C012AB3CD/p1722800000000100"},
                {"body":"no links here"},
                {"body":"discussed in https://tavus.slack.com/archives/C012AB3CD/p1722800000000100 and https://tavus.slack.com/archives/C09XYZ111/p1722899999000200?thread_ts=1"}
            ]}}}}}"#;
        let threads = parse_review_threads(with_comments).expect("parses");
        assert_eq!(
            threads.slack_urls,
            vec![
                "https://tavus.slack.com/archives/C012AB3CD/p1722800000000100",
                "https://tavus.slack.com/archives/C09XYZ111/p1722899999000200?thread_ts=1",
            ]
        );
        // The old shape (no comments field) still parses.
        assert!(parse_review_threads(LIVE_THREADS_JSON)
            .expect("parses")
            .slack_urls
            .is_empty());
    }

    /// A PR that appears right after the user pastes a Slack link inherits it
    /// as its origin; PRs adopted with no fresh link get none.
    #[test]
    fn new_prs_inherit_the_prompt_slack_origin() {
        let mut tracker = PrTracker::new();
        tracker.scan_prompt(
            "Investigate https://tavus.slack.com/archives/C012AB3CD/p1722800000000100 please",
            Path::new("/tmp"),
        );
        tracker.scan_text("opened https://github.com/o/r/pull/900", Path::new("/tmp"));
        assert_eq!(
            tracker.prs()[0].slack_origin_url,
            "https://tavus.slack.com/archives/C012AB3CD/p1722800000000100"
        );

        let mut plain = PrTracker::new();
        plain.scan_text("opened https://github.com/o/r/pull/901", Path::new("/tmp"));
        assert!(plain.prs()[0].slack_origin_url.is_empty());
    }

    #[test]
    fn prompt_tracking_keeps_every_slack_link_and_applies_known_authors() {
        let first = "https://tavus.slack.com/archives/C012AB3CD/p1722800000000100";
        let second = "https://tavus.slack.com/archives/C09XYZ111/p1722899999000200";
        let mut tracker = PrTracker::new();
        tracker.scan_prompt(
            &format!("Compare {first} with {second} and {first}"),
            Path::new("/tmp"),
        );

        assert_eq!(tracker.slack_threads().len(), 2);
        assert_eq!(tracker.slack_threads()[0].url, first);
        assert_eq!(tracker.slack_threads()[1].url, second);
        assert_eq!(tracker.session_slack_origin(), Some(first));

        let mut metadata = extract_threads(first);
        metadata[0].channel = Some("builder".to_string());
        metadata[0].author = Some("Sam Clay".to_string());
        assert!(tracker.apply_slack_metadata(&metadata));
        assert_eq!(
            tracker.slack_threads()[0].channel.as_deref(),
            Some("builder")
        );
        assert_eq!(
            tracker.slack_threads()[0].author.as_deref(),
            Some("Sam Clay")
        );
        assert!(!tracker.apply_slack_metadata(&metadata));
    }

    #[test]
    fn slack_tool_results_enrich_links_without_waiting_for_a_recap() {
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let mut tracker = PrTracker::new();
        tracker.scan_prompt(&format!("Investigate {url}"), Path::new("/tmp"));

        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,\"message\"";
        assert!(tracker.scan_text(result, Path::new("/tmp")));

        let thread = &tracker.slack_threads()[0];
        assert_eq!(thread.url, url);
        assert_eq!(thread.channel.as_deref(), Some("dogfood"));
        assert_eq!(thread.author.as_deref(), Some("Jared Vishno"));
        assert_eq!(thread.text.as_deref(), Some("message"));
    }

    #[test]
    fn exact_slack_result_replaces_recap_guesses() {
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let mut tracker = PrTracker::new();
        tracker.scan_prompt(&format!("Investigate {url}"), Path::new("/tmp"));

        let mut guessed = extract_threads(url);
        guessed[0].channel = Some("guessed-channel".to_string());
        guessed[0].author = Some("Guessed Author".to_string());
        assert!(tracker.apply_slack_metadata(&guessed));

        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,\"message\"";
        assert!(tracker.scan_text(result, Path::new("/tmp")));

        let thread = &tracker.slack_threads()[0];
        assert_eq!(thread.channel.as_deref(), Some("dogfood"));
        assert_eq!(thread.author.as_deref(), Some("Jared Vishno"));
    }

    #[test]
    fn a_quiet_open_pr_backs_off_by_doubling_to_ten_minutes() {
        let mut pr = SessionPr::test_stub(1, "o", "r");
        pr.state = "OPEN".into();
        // Clean means the merge button is green: the merge itself is next.
        pr.merge_state_status = "CLEAN".into();
        // The PR last moved at t=0, when it was also read. Step through the
        // seconds after that and note each read the schedule asks for.
        let reads = |pr: &SessionPr, until: u64| {
            let mut reads = Vec::new();
            let mut last_read = 0;
            for t in 1..=until {
                let ago = |at: u64| Some(Duration::from_secs(t - at));
                if refresh_due(pr, None, ago(last_read), ago(0)) {
                    reads.push(t);
                    last_read = t;
                }
            }
            reads
        };
        assert_eq!(reads(&pr, 2200), [30, 60, 120, 240, 480, 960, 1560, 2160]);
        // Running CI: every 15 s for five quiet minutes, every 30 s for the
        // rest of the hour, then the open backoff for a check that never ends.
        pr.checks_pending = 3;
        let fresh = (1..=20).map(|n| n * 15);
        let warm = (11..=120).map(|n| n * 30);
        let stuck = [4200, 4800];
        let expected: Vec<u64> = fresh.chain(warm).chain(stuck).collect();
        assert_eq!(reads(&pr, 4800), expected);
    }

    #[test]
    fn merged_and_closed_prs_are_read_hourly() {
        let mut pr = SessionPr::test_stub(1, "o", "r");
        let hour = Duration::from_secs(60 * 60);
        let just_moved = Some(Duration::ZERO);
        for state in ["MERGED", "CLOSED"] {
            pr.state = state.into();
            assert!(!refresh_due(
                &pr,
                None,
                Some(hour - Duration::from_secs(1)),
                just_moved
            ));
            assert!(refresh_due(&pr, None, Some(hour), None));
        }
    }

    #[test]
    fn only_a_real_change_on_github_restarts_the_backoff() {
        let mut tracker = PrTracker::new();
        let url = "https://github.com/o/r/pull/7";
        let deliver = |tracker: &mut PrTracker, json: GhPrJson| {
            let (sender, receiver) = mpsc::channel();
            tracker.pending.insert(url.to_string(), receiver);
            sender
                .send(JobResult::Pr(Box::new(FetchResult {
                    requested_url: Some(url.to_string()),
                    created_here: false,
                    pr_active: false,
                    data: Ok(json),
                    threads: None,
                })))
                .unwrap();
            tracker.poll();
        };
        deliver(&mut tracker, gh_json(url, "OPEN"));
        assert!(tracker.status_changed_at.is_empty(), "a first read");
        deliver(&mut tracker, gh_json(url, "OPEN"));
        assert!(
            tracker.status_changed_at.is_empty(),
            "the same status again"
        );
        let mut moved = gh_json(url, "OPEN");
        moved.additions = 5;
        deliver(&mut tracker, moved);
        assert!(tracker.status_changed_at.contains_key(url));
    }

    #[test]
    fn every_engaged_pr_keeps_polling() {
        let mut tracker = PrTracker::new();
        let now = now_unix_ms();
        for number in 1..=10 {
            let loc = PrLocation::new("o", "r", number);
            let mut pr = SessionPr::placeholder(&loc, false);
            pr.state = "OPEN".into();
            pr.refreshed_at = now - 11 * 60 * 1000;
            // Mentioned days ago, except PR 10, seen only in a bulk listing.
            if number < 10 {
                pr.last_mentioned_at = now - 3 * 24 * 60 * 60 * 1000;
            }
            tracker.prs.push(pr);
        }
        let mut read = Vec::new();
        while let Some(url) = tracker.next_refresh_url() {
            tracker
                .refresh_attempted_at
                .insert(url.clone(), Instant::now());
            read.push(url);
        }
        assert_eq!(read.len(), 9);
        assert!(!read.iter().any(|url| url.ends_with("/pull/10")));
    }

    #[test]
    fn a_recent_prompt_or_completion_refreshes_ci_without_another_pr_mention() {
        let mut tracker = PrTracker::new();
        let now = now_unix_ms();
        for number in 1..=3 {
            let loc = PrLocation::new("o", "r", number);
            let mut pr = SessionPr::placeholder(&loc, false);
            pr.state = "OPEN".into();
            pr.last_mentioned_at = now - number * 60_000;
            pr.refreshed_at = now - 31_000;
            tracker.prs.push(pr);
        }
        tracker.note_session_activity(Some(now as f64 / 1000.0));
        for number in 1..=2 {
            let url = tracker.next_refresh_url().expect("active PR is due");
            assert!(url.ends_with(&format!("/pull/{number}")));
            tracker.refresh_attempted_at.insert(url, Instant::now());
        }
        assert!(
            tracker.next_refresh_url().is_none(),
            "only two PRs poll quickly"
        );

        tracker.refresh_attempted_at.clear();
        for pr in &mut tracker.prs {
            pr.last_mentioned_at = now - 60 * 60_000;
        }
        tracker.note_session_activity(Some((now - 60 * 60_000) as f64 / 1000.0));
        assert!(
            tracker.next_refresh_url().is_none(),
            "idle sessions slow down"
        );
        tracker.note_session_activity(Some(now as f64 / 1000.0));
        assert!(
            tracker.next_refresh_url().is_some(),
            "a new completion wakes refreshes"
        );
    }

    #[test]
    fn an_old_unloaded_pr_does_not_jump_ahead_of_recent_work() {
        let mut tracker = PrTracker::new();
        let now = now_unix_ms();
        for number in 1..=8 {
            let loc = PrLocation::new("o", "r", number);
            let mut pr = SessionPr::placeholder(&loc, false);
            pr.state = "OPEN".into();
            pr.last_mentioned_at = now - number as u64 * 60_000;
            pr.refreshed_at = now - 2 * 60 * 60 * 1000;
            tracker.prs.push(pr);
        }
        let loc = PrLocation::new("o", "r", 99);
        let mut old = SessionPr::placeholder(&loc, false);
        old.state = "OPEN".into();
        old.last_mentioned_at = now - 24 * 60 * 60 * 1000;
        tracker.prs.push(old);

        let next = tracker.next_budgeted_url().expect("a recent PR is due");
        assert!(
            next.ends_with("/pull/1"),
            "recent work stays ahead of the backlog"
        );
        assert!(!tracker.is_recent_pr(&loc.url));
    }

    #[test]
    fn a_bulk_listing_does_not_ask_github_about_each_pr() {
        let mut tracker = PrTracker::new();
        tracker.scan_text(
            "open: https://github.com/o/r/pull/501 https://github.com/o/r/pull/502 \
             https://github.com/o/r/pull/503 https://github.com/o/r/pull/504",
            Path::new("/tmp"),
        );
        assert_eq!(tracker.prs().len(), 4);
        assert!(tracker.pending.is_empty());
        assert!(tracker.next_unenriched_url().is_none());
    }

    #[test]
    fn only_one_background_read_starts_at_a_time() {
        let mut tracker = PrTracker::new();
        let (_tx, rx) = mpsc::channel();
        tracker.pending.insert("busy".into(), rx);
        let now = now_unix_ms();
        for number in 1..=3 {
            let loc = PrLocation::new("o", "r", number);
            let mut pr = SessionPr::placeholder(&loc, false);
            pr.state = "OPEN".into();
            pr.last_mentioned_at = now;
            pr.refreshed_at = 1;
            tracker.prs.push(pr);
        }
        assert!(!tracker.start_due_refresh());
        assert_eq!(tracker.pending.len(), 1);
    }

    #[test]
    fn unloaded_prs_show_limits_until_the_allowance_resets() {
        let mut tracker = PrTracker::new();
        for number in 1..=3 {
            tracker.prs.push(SessionPr::test_stub(number, "o", "r"));
        }
        tracker.prs[1].state = "OPEN".into();
        tracker.prs[1].refreshed_at = 1;
        let (_tx, rx) = mpsc::channel();
        tracker.pending.insert(tracker.prs[2].url.clone(), rx);
        tracker.reads_this_window = SESSION_READS_PER_HOUR;
        tracker.read_window_started = Some(Instant::now());

        assert!(tracker.sync_fetch_limits());
        assert!(tracker.prs[0].fetch_limited);
        assert!(!tracker.prs[1].fetch_limited, "loaded PRs keep their state");
        assert!(
            !tracker.prs[2].fetch_limited,
            "an active read is still fetching"
        );
        assert!(
            !tracker.sync_fetch_limits(),
            "unchanged limits do not emit events"
        );
        let encoded = serde_json::to_value(&tracker.prs[0]).unwrap();
        assert_eq!(
            encoded["fetch_limited"], true,
            "mirrors and cloud carry the limit"
        );

        tracker.read_window_started = Some(Instant::now() - Duration::from_secs(3601));
        assert!(tracker.sync_fetch_limits());
        assert!(!tracker.prs[0].fetch_limited);
        assert!(tracker.session_read_allowed());
    }

    #[test]
    fn a_mention_during_an_in_flight_read_is_retried_later() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation::new("o", "r", 7);
        let mut pr = SessionPr::placeholder(&loc, false);
        pr.state = "OPEN".into();
        pr.refreshed_at = now_unix_ms();
        tracker.prs.push(pr);
        let (_tx, rx) = mpsc::channel();
        tracker.pending.insert("other".into(), rx);

        tracker.scan_text("PR #7 needs another look", Path::new("/tmp"));

        assert!(tracker.force_refresh.contains(&loc.url));
        assert!(!tracker.pending.contains_key(&loc.url));
    }

    #[test]
    fn pr_read_query_keeps_graphql_fields_apart() {
        for glued in [
            "isDraftadditions",
            "changedFilesmergeable",
            "reviewDecisionclosedAt",
            "updatedAtauthor",
            "}commits",
            "}statusContextCount",
            "}reviewThreads",
            "}comments",
        ] {
            assert!(
                !PR_READ_QUERY.contains(glued),
                "query glues fields together: {glued}"
            );
        }
        assert!(PR_READ_QUERY.contains("isDraft"));
        assert!(PR_READ_QUERY.contains("additions"));
        assert!(PR_READ_QUERY.contains("reviewThreads"));
    }

    #[test]
    fn a_cheap_pr_read_counts_checks_threads_and_slack_links() {
        let body = br#"{"data":{
            "rateLimit":{"limit":5000,"remaining":4200,"resetAt":"2026-09-28T18:00:00Z","cost":40},
            "repository":{"pullRequest":{
                "number":7,"title":"T","url":"https://github.com/o/r/pull/7",
                "headRefName":"feature","state":"OPEN","isDraft":false,
                "additions":3,"deletions":1,"changedFiles":2,
                "mergeable":"MERGEABLE","mergeStateStatus":"CLEAN","reviewDecision":"APPROVED",
                "author":{"login":"octocat"},
                "latestReviews":{"nodes":[{"state":"DISMISSED"}]},
                "commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{
                    "checkRunCount":6,
                    "checkRunCountsByState":[
                        {"state":"SUCCESS","count":3},
                        {"state":"FAILURE","count":1},
                        {"state":"IN_PROGRESS","count":2}
                    ],
                    "statusContextCount":2,
                    "statusContextCountsByState":[
                        {"state":"SUCCESS","count":1},
                        {"state":"PENDING","count":1}
                    ]
                }}}}]},
                "reviewThreads":{"nodes":[
                    {"isResolved":false,"comments":{"nodes":[{"url":"https://github.com/o/r/pull/7#discussion_r1"}]}},
                    {"isResolved":true,"comments":{"nodes":[{"url":"https://github.com/o/r/pull/7#discussion_r2"}]}}
                ]},
                "comments":{"nodes":[{"body":"see https://tavus.slack.com/archives/C0123ABCD/p1786640707322729"}]}
            }}
        }}"#;
        let parsed = parse_pr_read(body).expect("parses");
        assert_eq!(parsed.pr.check_counts, Some((4, 1, 3)));
        assert_eq!(parsed.pr.latest_reviews.len(), 1);
        assert_eq!(parsed.threads.unresolved, 1);
        assert_eq!(
            parsed.threads.first_url,
            "https://github.com/o/r/pull/7#discussion_r1"
        );
        assert_eq!(parsed.threads.slack_urls.len(), 1);
        let rate = parse_rate_limit(body).expect("GitHub reports its query cost");
        assert_eq!((rate.limit, rate.cost, rate.remaining), (5000, 40, 4200));

        let loc = PrLocation::new("o", "r", 7);
        let session = session_pr_from_fetch(&loc, parsed.pr, false);
        assert_eq!(
            session.ci_url, "https://github.com/o/r/pull/7/checks",
            "check counts link to the checks tab instead of every job"
        );
        assert!(session.review_dismissed);
        assert_eq!(
            (
                session.checks_passed,
                session.checks_failed,
                session.checks_pending
            ),
            (4, 1, 3)
        );
    }

    #[test]
    fn paginated_pr_reads_find_late_threads_and_charge_every_page() {
        let page = |nodes: Vec<serde_json::Value>, more, remaining| {
            serde_json::json!({
                "data": {
                    "rateLimit": {"limit":5000,"remaining":remaining,"cost":2,"resetAt":"2026-10-06T22:00:00Z"},
                    "repository": {"pullRequest": {
                        "number":1774,
                        "reviewThreads": {"nodes":nodes,"pageInfo":{"hasNextPage":more}},
                    }},
                },
            })
        };
        let first = page(
            vec![serde_json::json!({"isResolved":true}); 100],
            true,
            4000,
        );
        let second = page(
            vec![serde_json::json!({
                "isResolved":false,"comments":{"nodes":[{"url":"https://github.com/o/r/pull/1774#discussion_r101"}]}
            })],
            false,
            3998,
        );
        let bytes = serde_json::to_vec(&vec![&first, &second]).unwrap();
        let single_page = serde_json::to_vec(&vec![&second]).unwrap();
        assert_eq!(parse_pr_read(&single_page).unwrap().threads.unresolved, 1);
        let parsed = parse_pr_read(&bytes).unwrap();
        assert_eq!(parsed.threads.unresolved, 1);
        assert!(parsed.threads.first_url.ends_with("#discussion_r101"));
        let rate = parse_rate_limit(&bytes).unwrap();
        assert_eq!(rate.cost, 4);
        assert_eq!(rate.remaining, 3998);

        let incomplete = serde_json::to_vec(&vec![&first]).unwrap();
        assert!(
            parse_pr_read(&incomplete).is_err(),
            "partial threads must not look clean"
        );
        let failed = serde_json::to_vec(&vec![
            first,
            serde_json::json!({
                "errors":[{"message":"Could not load the remaining review threads"}]
            }),
        ])
        .unwrap();
        assert!(parse_pr_read(&failed).is_err());
    }

    #[test]
    fn tracked_number_mentions_refresh_once_per_occurrence() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation::new("o", "r", 7);
        let mut pr = SessionPr::placeholder(&loc, false);
        pr.state = "OPEN".into();
        pr.refreshed_at = now_unix_ms();
        tracker.prs.push(pr);

        tracker.scan_text("PR #7 is still running", Path::new("/tmp"));
        assert!(tracker.pending.contains_key(&loc.url));
        assert!(tracker.pr_active_at.contains_key(&loc.url));

        let old_activity = Instant::now() - Duration::from_secs(10 * 60);
        tracker.pr_active_at.insert(loc.url.clone(), old_activity);
        tracker.scan_text("PR #7 is still running", Path::new("/tmp"));
        assert_eq!(tracker.pr_active_at[&loc.url], old_activity);

        tracker.scan_text(
            "PR #7 is still running\nFinal update: PR #7 passed",
            Path::new("/tmp"),
        );
        assert!(tracker.pr_active_at[&loc.url].elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_new_prompt_reactivates_an_identical_pr_mention() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation::new("o", "r", 7);
        let mut pr = SessionPr::placeholder(&loc, false);
        pr.state = "OPEN".into();
        tracker.prs.push(pr);

        tracker.scan_prompt("Check PR #7", Path::new("/tmp"));
        let old_activity = Instant::now() - Duration::from_secs(10 * 60);
        tracker.pr_active_at.insert(loc.url.clone(), old_activity);

        tracker.on_prompt_observed();
        tracker.scan_prompt("Check PR #7", Path::new("/tmp"));

        assert!(tracker.pr_active_at[&loc.url].elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn user_request_to_push_is_not_mistaken_for_completed_activity() {
        let mut tracker = PrTracker::new();
        tracker.scan_prompt("Please git push this branch", Path::new("/tmp"));

        assert!(tracker.update_commands_seen.is_empty());
        assert!(tracker.pending_pr_active.is_empty());
    }

    #[test]
    fn update_command_deduplication_is_scoped_to_one_prompt() {
        let mut tracker = PrTracker::new();
        tracker.scan_prompt("First turn", Path::new("/tmp"));
        tracker.update_commands_seen.insert("git push".into(), 1);

        tracker.scan_prompt("Second turn", Path::new("/tmp"));

        assert!(tracker.update_commands_seen.is_empty());
    }

    /// A PR that leaves the open state stops being queried, so the count it had
    /// is dropped rather than frozen at whatever it was mid-review.
    #[test]
    fn merging_a_pr_clears_its_unresolved_count() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation::new("o", "r", 7);
        tracker.prs.push(SessionPr::placeholder(&loc, false));
        tracker.apply_threads(
            &loc.url,
            ReviewThreads {
                unresolved: 3,
                first_url: "https://github.com/o/r/pull/7#discussion_r1".into(),
                slack_urls: vec!["https://t.slack.com/archives/C1/p100".into()],
            },
        );
        assert_eq!(tracker.prs()[0].unresolved_comments, 3);

        tracker.apply_fetch(gh_json(&loc.url, "MERGED"), None, false);
        assert_eq!(tracker.prs()[0].unresolved_comments, 0);
        assert!(tracker.prs()[0].comments_url.is_empty());
        // Slack links survive the close-clearing: the notification comment
        // doesn't stop mattering when the PR merges.
        assert_eq!(tracker.prs()[0].slack_comment_urls.len(), 1);

        // ...and a still-open PR keeps it across a stats refresh.
        tracker.apply_threads(
            &loc.url,
            ReviewThreads {
                unresolved: 2,
                first_url: "https://github.com/o/r/pull/7#discussion_r2".into(),
                slack_urls: Vec::new(),
            },
        );
        tracker.apply_fetch(gh_json(&loc.url, "OPEN"), None, false);
        assert_eq!(tracker.prs()[0].unresolved_comments, 2);
    }

    /// Every Slack link attached to a PR gains a metadata entry the boards can
    /// label: pasted permalinks donate what the session already learned, and
    /// later Slack tool results keep enriching the rest.
    #[test]
    fn pr_slack_links_build_an_enriched_lookup() {
        let mut tracker = PrTracker::new();
        let loc = PrLocation::new("o", "r", 7);
        let origin = "https://t.slack.com/archives/C1/p1754404040123456";
        tracker.scan_prompt(&format!("see {origin}"), Path::new("/tmp"));
        tracker.scan_text(
            "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
             1754404040.123456,U1,sam,Sam Clay,C1 (#builder),,\"m\"",
            Path::new("/tmp"),
        );

        tracker.prs.push(SessionPr::placeholder(&loc, false));
        tracker.prs[0].slack_origin_url = origin.into();
        tracker.apply_threads(
            &loc.url,
            ReviewThreads {
                unresolved: 0,
                first_url: String::new(),
                slack_urls: vec!["https://t.slack.com/archives/C2/p1754404050123456".into()],
            },
        );
        assert!(tracker.sync_pr_slack_threads());

        let threads = tracker.pr_slack_threads();
        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].channel.as_deref(), Some("builder"));
        assert_eq!(threads[0].author.as_deref(), Some("Sam Clay"));
        assert_eq!(threads[1].channel.as_deref(), Some("C2"));

        // The comment link resolves when a Slack tool result names it.
        tracker.scan_text(
            "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
             1754404050.123456,U2,geoff,Geoff Barnes,C2 (#dogfood),,\"m\"",
            Path::new("/tmp"),
        );
        assert_eq!(
            tracker.pr_slack_threads()[1].channel.as_deref(),
            Some("dogfood")
        );
        assert_eq!(
            tracker.pr_slack_threads()[1].author.as_deref(),
            Some("Geoff Barnes")
        );
    }

    #[test]
    fn open_pr_with_null_closed_at_enriches() {
        let url = "https://github.com/o/r/pull/7";
        let json: GhPrJson = serde_json::from_str(&format!(
            r#"{{"number":7,"title":"Open PR","url":"{url}","state":"OPEN","closedAt":null}}"#
        ))
        .expect("open PR response should parse");
        let mut tracker = PrTracker::new();

        assert!(tracker.apply_fetch(json, None, false));
        assert_eq!(tracker.prs()[0].title, "Open PR");
        assert_eq!(tracker.prs()[0].closed_at, 0);
    }

    /// Minimal `gh pr view` payload for a PR in the given state.
    fn gh_json(url: &str, state: &str) -> GhPrJson {
        GhPrJson {
            number: 7,
            title: "T".into(),
            head_ref_name: "b".into(),
            url: url.to_string(),
            author: None,
            viewer_login: String::new(),
            state: state.to_string(),
            is_draft: false,
            additions: 1,
            deletions: 1,
            changed_files: 1,
            mergeable: "MERGEABLE".into(),
            merge_state_status: "CLEAN".into(),
            review_decision: String::new(),
            latest_reviews: Vec::new(),
            closed_at: None,
            updated_at: None,
            status_check_rollup: Vec::new(),
            check_counts: None,
        }
    }

    #[test]
    fn ci_link_targets_the_failing_job_then_falls_back_to_the_checks_tab() {
        let pr_url = "https://github.com/o/r/pull/5";
        let job = "https://github.com/o/r/actions/runs/1/job/2";

        // Red: straight to the first failing job's logs.
        let red = [
            check(CheckClass::Pass, Some("https://github.com/o/r/actions/9")),
            check(CheckClass::Fail, Some(job)),
            check(CheckClass::Fail, Some("https://github.com/o/r/actions/3")),
        ];
        assert_eq!(ci_link(&red, pr_url), job);

        // Green and pending both list every run in PR context instead.
        let checks_tab = format!("{pr_url}/checks");
        assert_eq!(
            ci_link(&[check(CheckClass::Pass, Some(job))], pr_url),
            checks_tab
        );
        assert_eq!(
            ci_link(&[check(CheckClass::Pending, Some(job))], pr_url),
            checks_tab
        );
        // A failure GitHub gave no page for still leads somewhere useful.
        assert_eq!(
            ci_link(&[check(CheckClass::Fail, None)], pr_url),
            checks_tab
        );
        // No checks at all → no link (the cell is blank anyway).
        assert_eq!(ci_link(&[], pr_url), "");
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
            author: Some(GhAuthor {
                login: "octocat".into(),
            }),
            viewer_login: "octocat".into(),
            state: "OPEN".into(),
            is_draft: true,
            additions: 10,
            deletions: 3,
            changed_files: 2,
            mergeable: "MERGEABLE".into(),
            merge_state_status: "CLEAN".into(),
            review_decision: String::new(),
            latest_reviews: vec![GhReview {
                state: "DISMISSED".into(),
            }],
            closed_at: None,
            updated_at: Some("2026-08-24T10:00:00Z".into()),
            status_check_rollup: vec![
                check(CheckClass::Pass, Some("https://github.com/o/r/actions/1")),
                check(CheckClass::Fail, Some("https://github.com/o/r/actions/2")),
            ],
            check_counts: None,
        };
        let changed = tracker.apply_fetch(json, Some(loc.url.clone()), true);
        assert!(changed);
        assert_eq!(tracker.prs().len(), 1);
        assert_eq!(tracker.prs()[0].mergeable, "MERGEABLE");
        assert_eq!(tracker.prs()[0].checks_passed, 1);
        assert_eq!(tracker.prs()[0].checks_failed, 1);
        // The failing job wins the CI link, so the cell lands on its logs.
        assert_eq!(tracker.prs()[0].ci_url, "https://github.com/o/r/actions/2");
        assert_eq!(tracker.prs()[0].branch, "feature/x");
        assert_eq!(tracker.prs()[0].additions, 10);
        assert_eq!(tracker.prs()[0].author_login, "octocat");
        assert_eq!(tracker.prs()[0].authored_by_viewer, Some(true));
        // The dismissed latest review marks the approval as invalidated.
        assert!(tracker.prs()[0].review_dismissed);
    }
}
