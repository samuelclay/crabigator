//! Shared limit for Crabigator's own background GitHub reads.
//!
//! Every session and every open PR board on this machine uses the same `gh`
//! login, and GitHub bills GraphQL by the hour. One busy session used to
//! re-read dozens of pull requests every minute. These caps keep background
//! refreshes small enough that the user, and the assistant's own `gh`
//! commands, still have quota left.
//!
//! The file lives in `~/.crabigator/gh-budget-v2.json`. Older processes use
//! estimated costs in a separate file so they cannot erase the per-session
//! counters. A lock is held only while updating numbers, never over the network.
//!
//! The counts are partly estimates. When only they stand in the way of a
//! read, GitHub is asked what the hour has really cost, and the counts are
//! corrected, so a read is refused only when GitHub is actually running low.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The bounded PR query costs one point. Reconcile with GitHub's reported
/// cost before releasing the read slot if the query ever costs more.
pub const READ_POINTS: u32 = 1;
/// `gh pr view` doesn't report its cost. It makes one small GraphQL query
/// (about a point, measured); charge two to stay on the safe side.
pub const UNREPORTED_READ_POINTS: u32 = 2;
const DEFAULT_GITHUB_LIMIT: u32 = 5_000;
const BUDGET_PERCENT: u32 = 80;
/// A single session may spend only a small share of the account allowance.
pub const SESSION_LIMIT: u32 = 500;
/// Watches may use at most this much of the shared allowance.
const BOARD_POINTS: u32 = 400;
/// Shortest gap between background reads from different jobs.
const MIN_GAP: Duration = Duration::from_millis(500);
/// A read that never checked back in stops blocking the next one after this.
const INFLIGHT_TIMEOUT: Duration = Duration::from_secs(90);
/// Pause this long when GitHub says the limit is spent but not when it resets.
const RATE_LIMIT_FALLBACK: Duration = Duration::from_secs(15 * 60);
/// How often any process on this machine may ask GitHub what the hour cost.
const RECONCILE_GAP: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(60 * 60);

/// Both readers spend the shared allowance; watches also have a smaller cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reader {
    Session,
    Board,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct BudgetState {
    window_start_ms: u64,
    session_points: u32,
    board_points: u32,
    paused_until_ms: u64,
    last_start_ms: u64,
    inflight_pid: u32,
    inflight_since_ms: u64,
    #[serde(default)]
    github_limit: Option<u32>,
    #[serde(default)]
    session_points_by_pid: HashMap<u32, u32>,
    /// When a process last asked GitHub what the hour cost.
    #[serde(default)]
    reconciled_at_ms: u64,
}

/// Held until the `gh` process exits. Releasing it lets the next read start.
pub struct Permit {
    pid: u32,
    armed: bool,
    reader: Reader,
    window_start_ms: u64,
    reserved_points: u32,
}

impl Permit {
    /// Account for the completed query and retain 20% of the account quota,
    /// including when other programs or computers have spent points.
    pub fn record_usage(&mut self, cost: u32, limit: u32, remaining: u32, reset_ms: u64) {
        if !self.armed {
            return;
        }
        let _ = with_budget(|state| {
            if state.window_start_ms == self.window_start_ms {
                replace_points(
                    state,
                    self.reader,
                    self.pid,
                    self.reserved_points,
                    cost.max(1),
                );
            }
            if limit > 0 {
                state.github_limit = Some(limit);
            }
            let limit = state.github_limit.unwrap_or(DEFAULT_GITHUB_LIMIT);
            let reserve = limit.saturating_sub(shared_allowance(state));
            if remaining <= reserve {
                let now = now_ms();
                let until = if reset_ms > now {
                    reset_ms
                } else {
                    now.saturating_add(RATE_LIMIT_FALLBACK.as_millis() as u64)
                };
                state.paused_until_ms = state.paused_until_ms.max(until);
            }
        });
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if self.armed {
            release(self.pid);
        }
    }
}

/// Whether a background read is allowed to start right now.
pub fn can_start(reader: Reader) -> bool {
    can_start_with_estimate(reader, READ_POINTS)
}

pub fn can_start_with_estimate(reader: Reader, points: u32) -> bool {
    if !limits_apply() {
        return true;
    }
    decide(reader, points, |state, now, pid| {
        allow(state, now, pid, reader, points)
    })
    .unwrap_or(false)
}

/// A quota or rate-limit pause, excluding the short gap and in-flight reads.
pub fn limit_hit(reader: Reader) -> bool {
    limit_hit_with_estimate(reader, READ_POINTS)
}

pub fn limit_hit_with_estimate(reader: Reader, points: u32) -> bool {
    if !limits_apply() {
        return false;
    }
    decide(reader, points, |state, now, pid| {
        quota_blocks(state, now, pid, reader, points)
    })
    .unwrap_or(false)
}

/// Reserve points and the single in-flight slot. `None` means skip the call.
pub fn acquire(reader: Reader) -> Option<Permit> {
    acquire_with_estimate(reader, READ_POINTS)
}

pub fn acquire_with_estimate(reader: Reader, points: u32) -> Option<Permit> {
    if !limits_apply() {
        return Some(Permit {
            pid: 0,
            armed: false,
            reader,
            window_start_ms: 0,
            reserved_points: points,
        });
    }
    let pid = std::process::id();
    let reserved = decide(reader, points, |state, now, pid| {
        if !allow(state, now, pid, reader, points) {
            return None;
        }
        add_points(state, reader, pid, points);
        state.last_start_ms = now;
        state.inflight_pid = pid;
        state.inflight_since_ms = now;
        Some(state.window_start_ms)
    })??;
    Some(Permit {
        pid,
        armed: true,
        reader,
        window_start_ms: reserved,
        reserved_points: points,
    })
}

/// GitHub refused the call because the hourly limit is already spent.
pub fn note_limited(message: &str) {
    if !limits_apply() {
        return;
    }
    let until = now_ms().saturating_add(pause_after(message).as_millis() as u64);
    let _ = with_budget(|state| {
        if until > state.paused_until_ms {
            state.paused_until_ms = until;
        }
    });
}

pub fn looks_limited(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("rate limit") || lower.contains("rate_limit")
}

pub fn is_deferral(error: &str) -> bool {
    error.starts_with("github budget:")
}

pub fn deferral_reason() -> String {
    "github budget: background GitHub reads are paused".to_string()
}

/// How long to stay quiet after a rate-limit error.
fn pause_after(text: &str) -> Duration {
    let lower = text.to_ascii_lowercase();
    for marker in ["rate reset in ", "try again in ", "retry after "] {
        if let Some(index) = lower.find(marker) {
            if let Some(delay) = parse_hms(&text[index + marker.len()..]) {
                return delay.clamp(Duration::from_secs(30), HOUR);
            }
        }
    }
    RATE_LIMIT_FALLBACK
}

/// `12m34s`, `1h2m`, `45s`, or a bare number of seconds.
fn parse_hms(text: &str) -> Option<Duration> {
    let token = text
        .trim()
        .split_whitespace()
        .next()?
        .trim_matches(|c: char| matches!(c, ',' | '.' | ']' | ')'));
    if token.is_empty() {
        return None;
    }
    let mut total = 0u64;
    let mut number = String::new();
    let mut saw_unit = false;
    for character in token.chars() {
        if character.is_ascii_digit() {
            number.push(character);
            continue;
        }
        let value: u64 = number.parse().ok()?;
        number.clear();
        let unit = match character {
            'h' | 'H' => 3600,
            'm' | 'M' => 60,
            's' | 'S' => 1,
            _ => return None,
        };
        total = total.saturating_add(value.saturating_mul(unit));
        saw_unit = true;
    }
    if !number.is_empty() {
        let value: u64 = number.parse().ok()?;
        // A bare number, or digits with no trailing unit, counts as seconds.
        total = total.saturating_add(value);
        saw_unit = true;
    }
    saw_unit.then(|| Duration::from_secs(total))
}

fn allow(state: &BudgetState, now: u64, pid: u32, reader: Reader, points: u32) -> bool {
    if quota_blocks(state, now, pid, reader, points) {
        return false;
    }
    let same_job = state.inflight_pid == pid && pid != 0;
    if !same_job {
        if inflight_busy(state, now) {
            return false;
        }
        if now.saturating_sub(state.last_start_ms) < MIN_GAP.as_millis() as u64 {
            return false;
        }
    }
    true
}

/// Look at the budget for one read of `points`. When only the counted points
/// stand in the way (no pause), ask GitHub what the hour really cost, on a
/// thread of its own: callers sit on the UI path.
fn decide<T>(
    reader: Reader,
    points: u32,
    look: impl FnOnce(&mut BudgetState, u64, u32) -> T,
) -> Option<T> {
    let pid = std::process::id();
    let (result, reconcile) = with_budget(|state| {
        let now = now_ms();
        roll_window(state, now);
        let reconcile = claim_reconcile(state, now, pid, reader, points);
        (look(state, now, pid), reconcile)
    })?;
    if reconcile {
        spawn_reconcile();
    }
    Some(result)
}

/// Whether this caller should ask GitHub what the hour cost: the counted
/// points refuse the read, nothing is paused, and no process on the machine
/// asked in the last minute. Claims the turn when it answers yes.
fn claim_reconcile(
    state: &mut BudgetState,
    now: u64,
    pid: u32,
    reader: Reader,
    points: u32,
) -> bool {
    let due = now >= state.paused_until_ms
        && points_block(state, pid, reader, points)
        && now.saturating_sub(state.reconciled_at_ms) >= RECONCILE_GAP.as_millis() as u64;
    if due {
        state.reconciled_at_ms = now;
    }
    due
}

/// GitHub's GraphQL allowance for the hour, as `/rate_limit` reports it.
#[derive(Debug, Deserialize)]
struct GithubRate {
    limit: u32,
    used: u32,
    remaining: u32,
    /// Unix seconds when the hour resets.
    reset: u64,
}

fn spawn_reconcile() {
    if cfg!(test) {
        return;
    }
    std::thread::spawn(|| {
        if let Some(rate) = github_rate() {
            let _ = with_budget(|state| reconcile(state, &rate, now_ms()));
        }
    });
}

/// `gh api rate_limit` costs nothing against the allowance it reports.
fn github_rate() -> Option<GithubRate> {
    let output = std::process::Command::new("gh")
        .args(["api", "rate_limit", "--jq", ".resources.graphql"])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

/// Bring the counts in line with GitHub. GitHub's figure includes every
/// program on the account, so counts above it can only be overestimates:
/// scale each down to it. Follow GitHub's hour from now on, and pause until
/// it resets when only the spare fifth is left.
fn reconcile(state: &mut BudgetState, rate: &GithubRate, now: u64) {
    if rate.limit > 0 {
        state.github_limit = Some(rate.limit);
    }
    let reset_ms = rate.reset.saturating_mul(1000);
    if reset_ms > now {
        state.window_start_ms = reset_ms.saturating_sub(HOUR.as_millis() as u64);
    }
    let spent = state.session_points.saturating_add(state.board_points);
    if spent > rate.used {
        let scale =
            |points: u32| (u64::from(points) * u64::from(rate.used) / u64::from(spent)) as u32;
        state.session_points = scale(state.session_points);
        state.board_points = scale(state.board_points);
        for points in state.session_points_by_pid.values_mut() {
            *points = scale(*points);
        }
    }
    let limit = state.github_limit.unwrap_or(DEFAULT_GITHUB_LIMIT);
    if rate.remaining <= limit.saturating_sub(shared_allowance(state)) {
        let until = if reset_ms > now {
            reset_ms
        } else {
            now.saturating_add(RATE_LIMIT_FALLBACK.as_millis() as u64)
        };
        state.paused_until_ms = state.paused_until_ms.max(until);
    }
}

fn quota_blocks(state: &BudgetState, now: u64, pid: u32, reader: Reader, points: u32) -> bool {
    now < state.paused_until_ms || points_block(state, pid, reader, points)
}

/// The counted points leave no room for `points` more.
fn points_block(state: &BudgetState, pid: u32, reader: Reader, points: u32) -> bool {
    state
        .session_points
        .saturating_add(state.board_points)
        .saturating_add(points)
        > shared_allowance(state)
        || (reader == Reader::Board && state.board_points.saturating_add(points) > BOARD_POINTS)
        || (reader == Reader::Session
            && state
                .session_points_by_pid
                .get(&pid)
                .copied()
                .unwrap_or(0)
                .saturating_add(points)
                > SESSION_LIMIT)
}

fn inflight_busy(state: &BudgetState, now: u64) -> bool {
    if state.inflight_pid == 0 {
        return false;
    }
    if now.saturating_sub(state.inflight_since_ms) > INFLIGHT_TIMEOUT.as_millis() as u64 {
        return false;
    }
    process_alive(state.inflight_pid)
}

fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        let result = unsafe { kill(pid as i32, 0) };
        if result == 0 {
            return true;
        }
        // EPERM: the process exists, but it isn't ours to signal.
        std::io::Error::last_os_error().raw_os_error() == Some(1)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn shared_allowance(state: &BudgetState) -> u32 {
    state.github_limit.unwrap_or(DEFAULT_GITHUB_LIMIT) / 100 * BUDGET_PERCENT
}

fn add_points(state: &mut BudgetState, reader: Reader, pid: u32, points: u32) {
    replace_points(state, reader, pid, 0, points);
}

fn replace_points(state: &mut BudgetState, reader: Reader, pid: u32, reserved: u32, actual: u32) {
    let replace = |spent: u32| spent.saturating_sub(reserved).saturating_add(actual);
    match reader {
        Reader::Session => {
            state.session_points = replace(state.session_points);
            let spent = state.session_points_by_pid.entry(pid).or_default();
            *spent = replace(*spent);
        }
        Reader::Board => state.board_points = replace(state.board_points),
    }
}

fn roll_window(state: &mut BudgetState, now: u64) {
    if state.window_start_ms == 0
        || now.saturating_sub(state.window_start_ms) >= HOUR.as_millis() as u64
    {
        state.window_start_ms = now;
        state.session_points = 0;
        state.board_points = 0;
        state.session_points_by_pid.clear();
    }
}

fn release(pid: u32) {
    if !limits_apply() {
        return;
    }
    let _ = with_budget(|state| {
        if state.inflight_pid == pid {
            state.inflight_pid = 0;
            state.inflight_since_ms = 0;
        }
    });
}

fn with_budget<T>(update: impl FnOnce(&mut BudgetState) -> T) -> Option<T> {
    let path = budget_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .ok()?;
    file.lock().ok()?;
    let mut state = read_state(&mut file);
    let result = update(&mut state);
    write_state(&mut file, &state);
    let _ = file.unlock();
    Some(result)
}

fn read_state(file: &mut File) -> BudgetState {
    let _ = file.seek(SeekFrom::Start(0));
    let mut text = String::new();
    if file.read_to_string(&mut text).is_err() || text.trim().is_empty() {
        return BudgetState::default();
    }
    serde_json::from_str(&text).unwrap_or_default()
}

fn write_state(file: &mut File, state: &BudgetState) {
    let Ok(body) = serde_json::to_vec(state) else {
        return;
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return;
    }
    let _ = file.set_len(0);
    let _ = file.write_all(&body);
    let _ = file.flush();
}

fn budget_path() -> Option<PathBuf> {
    if let Some(path) = test_budget_path() {
        return Some(path);
    }
    if !limits_apply() {
        return None;
    }
    Some(
        dirs::home_dir()?
            .join(".crabigator")
            .join("gh-budget-v2.json"),
    )
}

/// Limits are off in unit tests unless a test points them at its own directory.
/// Spawned `gh` calls inside those tests must not touch the user's real budget.
fn limits_apply() -> bool {
    if cfg!(test) {
        test_budget_path().is_some()
    } else {
        true
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn test_budget_path() -> Option<PathBuf> {
    #[cfg(test)]
    {
        TEST_DIR.with(|slot| slot.borrow().clone())
    }
    #[cfg(not(test))]
    {
        None
    }
}

#[cfg(test)]
thread_local! {
    static TEST_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn set_test_dir(path: Option<PathBuf>) {
    TEST_DIR.with(|slot| *slot.borrow_mut() = path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_phrases_become_a_pause() {
        assert_eq!(
            pause_after("API rate limit exceeded [rate reset in 12m34s]"),
            Duration::from_secs(12 * 60 + 34)
        );
        assert_eq!(
            pause_after("please try again in 2m3s."),
            Duration::from_secs(120 + 3)
        );
        assert_eq!(
            pause_after("rate reset in 2h"),
            HOUR,
            "a reset further out than an hour still waits out this hour"
        );
        assert_eq!(pause_after("secondary rate limit hit"), RATE_LIMIT_FALLBACK);
    }

    #[test]
    fn a_second_read_waits_out_the_gap_and_a_later_hour_starts_fresh() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));

        assert!(acquire(Reader::Session).is_some());
        assert!(
            !limit_hit(Reader::Session),
            "a short gap is not a quota limit"
        );
        assert!(
            acquire(Reader::Session).is_none(),
            "the next read waits for the gap"
        );

        let mut stale: BudgetState = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("gh-budget.json")).unwrap(),
        )
        .unwrap();
        stale.window_start_ms = now_ms().saturating_sub(HOUR.as_millis() as u64 + 1);
        stale.session_points = shared_allowance(&stale);
        stale.last_start_ms = 0;
        stale.inflight_pid = 0;
        std::fs::write(
            dir.path().join("gh-budget.json"),
            serde_json::to_vec(&stale).unwrap(),
        )
        .unwrap();
        assert!(
            acquire(Reader::Session).is_some(),
            "a new hour clears the spent allowance"
        );
        assert!(!limit_hit(Reader::Session));

        set_test_dir(None);
    }

    #[test]
    fn rate_limit_and_low_remaining_pause_both_pools() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));

        note_limited("GraphQL: API rate limit already exceeded [rate reset in 5m]");
        assert!(!can_start(Reader::Session));
        assert!(!can_start(Reader::Board));
        assert!(limit_hit(Reader::Session));
        assert!(limit_hit(Reader::Board));

        // Clear the pause by rolling it into the past, then trip the headroom stop.
        with_budget(|state| state.paused_until_ms = 0);
        let mut permit = acquire(Reader::Session).unwrap();
        permit.record_usage(1, 5000, 1000, now_ms().saturating_add(60_000));
        drop(permit);
        assert!(!can_start(Reader::Session));
        assert!(limit_hit(Reader::Session));

        with_budget(|state| state.paused_until_ms = now_ms().saturating_sub(1));
        assert!(
            !limit_hit(Reader::Session),
            "the label clears after the pause"
        );

        set_test_dir(None);
    }

    #[test]
    fn github_corrects_overcounted_points_and_its_hour_takes_over() {
        let now = now_ms();
        let mut state = BudgetState {
            window_start_ms: now - 60_000,
            session_points: 1300,
            session_points_by_pid: HashMap::from([(1, 480), (2, 480), (3, 340)]),
            ..BudgetState::default()
        };
        assert!(points_block(&state, 1, Reader::Session, 80));
        let reset = now / 1000 + 9 * 60;
        let rate = GithubRate {
            limit: 5000,
            used: 130,
            remaining: 4870,
            reset,
        };
        reconcile(&mut state, &rate, now);
        // Scaled down to what GitHub counted, in the same proportions.
        assert_eq!(state.session_points, 130);
        assert_eq!(state.session_points_by_pid[&1], 48);
        assert_eq!(state.session_points_by_pid[&3], 34);
        assert!(!points_block(&state, 1, Reader::Session, 80));
        // The hour now ends when GitHub's does.
        assert_eq!(
            state.window_start_ms,
            reset * 1000 - HOUR.as_millis() as u64
        );
        assert_eq!(state.paused_until_ms, 0);

        // Counts under GitHub's figure stand: other programs spent the rest.
        let mut quiet = BudgetState {
            session_points: 40,
            ..BudgetState::default()
        };
        reconcile(&mut quiet, &rate, now);
        assert_eq!(quiet.session_points, 40);

        // Only the spare fifth left: pause until GitHub's hour resets.
        let low = GithubRate {
            remaining: 900,
            used: 4100,
            ..rate
        };
        reconcile(&mut quiet, &low, now);
        assert_eq!(quiet.paused_until_ms, reset * 1000);
    }

    #[test]
    fn only_counted_points_ask_github_and_at_most_once_a_minute() {
        let now = now_ms();
        let mut state = BudgetState::default();
        assert!(
            !claim_reconcile(&mut state, now, 1, Reader::Session, 1),
            "nothing blocks: no need to ask"
        );
        state.session_points_by_pid.insert(1, SESSION_LIMIT);
        assert!(claim_reconcile(&mut state, now, 1, Reader::Session, 1));
        assert!(
            !claim_reconcile(&mut state, now + 1_000, 1, Reader::Session, 1),
            "another asked a moment ago"
        );
        assert!(claim_reconcile(
            &mut state,
            now + RECONCILE_GAP.as_millis() as u64,
            1,
            Reader::Session,
            1
        ));
        state.paused_until_ms = now + 10 * 60_000;
        assert!(
            !claim_reconcile(&mut state, now + 3 * 60_000, 1, Reader::Session, 1),
            "a pause GitHub asked for stands"
        );
    }

    #[test]
    fn a_live_holder_blocks_other_processes_until_it_goes_stale() {
        let now = now_ms();
        let state = BudgetState {
            inflight_pid: std::process::id(),
            inflight_since_ms: now,
            ..BudgetState::default()
        };
        assert!(
            allow(
                &state,
                now,
                std::process::id(),
                Reader::Session,
                READ_POINTS
            ),
            "the job that already holds the slot may make a follow-up call"
        );
        assert!(
            !allow(
                &state,
                now,
                std::process::id().wrapping_add(1),
                Reader::Session,
                READ_POINTS
            ),
            "another process waits"
        );
        assert!(
            allow(
                &state,
                now.saturating_add(INFLIGHT_TIMEOUT.as_millis() as u64 + 1),
                std::process::id().wrapping_add(1),
                Reader::Session,
                READ_POINTS
            ),
            "a holder that never checked back in stops blocking"
        );
    }

    #[test]
    fn a_spent_board_allowance_does_not_block_sessions() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));
        with_budget(|state| {
            state.window_start_ms = now_ms();
            state.board_points = BOARD_POINTS;
        });
        // Gap and inflight would also refuse; clear them by writing a quiet state.
        with_budget(|state| {
            state.last_start_ms = 0;
            state.inflight_pid = 0;
        });
        assert!(can_start(Reader::Session));
        assert!(!can_start(Reader::Board));
        assert!(!limit_hit(Reader::Session));
        assert!(limit_hit(Reader::Board));
        set_test_dir(None);
    }

    #[test]
    fn one_session_spends_its_own_quota_without_blocking_a_neighbor() {
        let now = now_ms();
        let mut state = BudgetState::default();
        for _ in 0..SESSION_LIMIT {
            assert!(allow(&state, now, 11, Reader::Session, 1));
            add_points(&mut state, Reader::Session, 11, 1);
        }
        assert!(!allow(&state, now, 11, Reader::Session, 1));
        assert!(allow(&state, now, 12, Reader::Session, 1));
        assert!(allow(&state, now, 13, Reader::Board, 1));

        state.window_start_ms = now.saturating_sub(HOUR.as_millis() as u64);
        roll_window(&mut state, now);
        assert!(allow(&state, now, 11, Reader::Session, 1));
    }

    #[test]
    fn sessions_and_boards_share_eighty_percent_of_the_reported_limit() {
        let now = now_ms();
        let mut state = BudgetState {
            session_points: 3600,
            board_points: 399,
            ..BudgetState::default()
        };
        assert!(allow(&state, now, 1, Reader::Board, 1));
        add_points(&mut state, Reader::Board, 1, 1);
        assert!(!allow(&state, now, 2, Reader::Session, 1));
        assert!(!allow(&state, now, 1, Reader::Board, 1));
        state.github_limit = Some(10_000);
        assert_eq!(shared_allowance(&state), 8000);
        assert!(allow(&state, now, 2, Reader::Session, 1));
    }

    #[test]
    fn reported_cost_is_charged_before_the_next_session_can_read() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));
        let mut permit = acquire(Reader::Session).unwrap();
        permit.record_usage(SESSION_LIMIT, 5000, 4500, now_ms() + 60_000);
        drop(permit);
        let state = with_budget(|state| state.clone()).unwrap();
        assert_eq!(state.session_points, SESSION_LIMIT);
        assert!(limit_hit(Reader::Session));
        assert!(!quota_blocks(
            &state,
            now_ms(),
            std::process::id().wrapping_add(1),
            Reader::Session,
            1
        ));
        assert_eq!(
            state.paused_until_ms, 0,
            "a session cap never pauses the account"
        );
        set_test_dir(None);
    }

    #[test]
    fn an_unaffordable_number_lookup_stays_queued_and_shows_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));
        with_budget(|state| {
            state.window_start_ms = now_ms();
            state
                .session_points_by_pid
                .insert(std::process::id(), SESSION_LIMIT - 1);
        });
        let mut tracker = crate::pr::PrTracker::new();
        tracker
            .prs
            .push(crate::pr::SessionPr::test_stub(123, "o", "r"));
        tracker.deferred_lookups.push((PathBuf::from("/tmp"), 123));
        for _ in 0..3 {
            tracker.poll();
            assert!(tracker.pending.is_empty(), "do not spawn a doomed worker");
            assert_eq!(tracker.deferred_lookups.len(), 1);
            assert!(tracker.prs[0].fetch_limited);
            assert_eq!(tracker.reads_this_window, 0);
        }
        set_test_dir(None);
    }

    #[test]
    fn unreported_reads_reserve_their_cost_and_reported_cost_replaces_it() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));
        let mut permit = acquire_with_estimate(Reader::Session, UNREPORTED_READ_POINTS).unwrap();
        assert_eq!(
            with_budget(|state| state.session_points),
            Some(UNREPORTED_READ_POINTS)
        );
        permit.record_usage(1, 5000, 4999, now_ms() + 60_000);
        drop(permit);
        assert_eq!(with_budget(|state| state.session_points), Some(1));

        with_budget(|state| {
            state
                .session_points_by_pid
                .insert(std::process::id(), SESSION_LIMIT - 1);
        });
        assert!(limit_hit_with_estimate(
            Reader::Session,
            UNREPORTED_READ_POINTS
        ));
        assert!(
            !limit_hit(Reader::Session),
            "a cheap URL read can still run"
        );
        set_test_dir(None);
    }
}
