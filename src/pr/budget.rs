//! Shared limit for Crabigator's own background GitHub reads.
//!
//! Every session and every open PR board on this machine uses the same `gh`
//! login, and GitHub bills GraphQL by the hour. One busy session used to
//! re-read dozens of pull requests every minute. These caps keep background
//! refreshes small enough that the user, and the assistant's own `gh`
//! commands, still have quota left.
//!
//! The file lives in `~/.crabigator/gh-budget.json`. A lock is held only while
//! the numbers are updated, never during the network call.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Points reserved for one background PR read. The query asks for a fixed
/// handful of nodes; this is a cushion above that cost, not a measurement.
pub const READ_POINTS: u32 = 80;
/// Points all sessions together may spend in an hour.
const SESSION_POINTS: u32 = 1200;
/// Points all open PR boards together may spend in an hour.
const BOARD_POINTS: u32 = 240;
/// Stop background reads while GitHub still has this many points left, so a
/// wrong cost estimate cannot empty the account.
const LOW_REMAINING: u32 = 1500;
/// Shortest gap between background reads from different jobs.
const MIN_GAP: Duration = Duration::from_secs(20);
/// A read that never checked back in stops blocking the next one after this.
const INFLIGHT_TIMEOUT: Duration = Duration::from_secs(90);
/// Pause this long when GitHub says the limit is spent but not when it resets.
const RATE_LIMIT_FALLBACK: Duration = Duration::from_secs(15 * 60);
const HOUR: Duration = Duration::from_secs(60 * 60);

/// Who is spending the points. Sessions and PR boards have separate allowances
/// so a wall of watched PRs cannot crowd out the session you are working in.
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
}

/// Held until the `gh` process exits. Releasing it lets the next read start.
pub struct Permit {
    pid: u32,
    armed: bool,
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
    if !limits_apply() {
        return true;
    }
    with_budget(|state| {
        let now = now_ms();
        roll_window(state, now);
        allow(state, now, std::process::id(), reader, READ_POINTS)
    })
    .unwrap_or(false)
}

/// Reserve points and the single in-flight slot. `None` means skip the call.
pub fn acquire(reader: Reader) -> Option<Permit> {
    if !limits_apply() {
        return Some(Permit {
            pid: 0,
            armed: false,
        });
    }
    let pid = std::process::id();
    let reserved = with_budget(|state| {
        let now = now_ms();
        roll_window(state, now);
        if !allow(state, now, pid, reader, READ_POINTS) {
            return false;
        }
        add_points(state, reader, READ_POINTS);
        state.last_start_ms = now;
        state.inflight_pid = pid;
        state.inflight_since_ms = now;
        true
    })?;
    reserved.then_some(Permit { pid, armed: true })
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

/// A successful read reported how many points are left. Leave the tail of the
/// hour for the user.
pub fn note_remaining(remaining: u32, reset_ms: u64) {
    if !limits_apply() || remaining >= LOW_REMAINING {
        return;
    }
    let now = now_ms();
    let until = if reset_ms > now {
        reset_ms
    } else {
        now.saturating_add(RATE_LIMIT_FALLBACK.as_millis() as u64)
    };
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
    if now < state.paused_until_ms {
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
    points_used(state, reader).saturating_add(points) <= allowance(reader)
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

fn allowance(reader: Reader) -> u32 {
    match reader {
        Reader::Session => SESSION_POINTS,
        Reader::Board => BOARD_POINTS,
    }
}

fn points_used(state: &BudgetState, reader: Reader) -> u32 {
    match reader {
        Reader::Session => state.session_points,
        Reader::Board => state.board_points,
    }
}

fn add_points(state: &mut BudgetState, reader: Reader, points: u32) {
    match reader {
        Reader::Session => state.session_points = state.session_points.saturating_add(points),
        Reader::Board => state.board_points = state.board_points.saturating_add(points),
    }
}

fn roll_window(state: &mut BudgetState, now: u64) {
    if state.window_start_ms == 0
        || now.saturating_sub(state.window_start_ms) >= HOUR.as_millis() as u64
    {
        state.window_start_ms = now;
        state.session_points = 0;
        state.board_points = 0;
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
    Some(dirs::home_dir()?.join(".crabigator").join("gh-budget.json"))
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
            acquire(Reader::Session).is_none(),
            "the next read waits for the gap"
        );

        let mut stale: BudgetState = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("gh-budget.json")).unwrap(),
        )
        .unwrap();
        stale.window_start_ms = now_ms().saturating_sub(HOUR.as_millis() as u64 + 1);
        stale.session_points = SESSION_POINTS;
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

        set_test_dir(None);
    }

    #[test]
    fn rate_limit_and_low_remaining_pause_both_pools() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));

        note_limited("GraphQL: API rate limit already exceeded [rate reset in 5m]");
        assert!(!can_start(Reader::Session));
        assert!(!can_start(Reader::Board));

        // Clear the pause by rolling it into the past, then trip the headroom stop.
        with_budget(|state| state.paused_until_ms = 0);
        note_remaining(LOW_REMAINING - 1, now_ms().saturating_add(60_000));
        assert!(!can_start(Reader::Session));

        set_test_dir(None);
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
    fn session_reads_do_not_spend_the_board_allowance() {
        let dir = tempfile::tempdir().unwrap();
        set_test_dir(Some(dir.path().join("gh-budget.json")));
        with_budget(|state| {
            state.window_start_ms = now_ms();
            state.session_points = SESSION_POINTS;
        });
        // Gap and inflight would also refuse; clear them by writing a quiet state.
        with_budget(|state| {
            state.last_start_ms = 0;
            state.inflight_pid = 0;
        });
        assert!(!can_start(Reader::Session));
        assert!(can_start(Reader::Board));
        set_test_dir(None);
    }
}
