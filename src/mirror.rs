//! Widget state mirroring for external inspection
//!
//! When --profile is enabled, periodically writes widget state to JSON.
//! Another crabigator instance can inspect this state via `crabigator inspect`.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::Serialize;

use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::parsers::{ChangeType, DiffSummary};

/// Minimum interval between publishes (1 second)
const PUBLISH_INTERVAL: Duration = Duration::from_secs(1);

/// Mirror state for a single widget
#[derive(Serialize)]
pub struct WidgetMirror<T: Serialize> {
    pub data: T,
    pub rendered: Vec<String>,
}

/// Complete mirrored state
#[derive(Serialize)]
pub struct MirrorState {
    pub session_id: String,
    pub cwd: String,
    pub terminal_title: Option<String>,
    pub last_updated: f64,
    pub capture: CaptureMirror,
    pub launch_timing: LaunchTimingMirror,
    pub widgets: MirrorWidgets,
}

/// Launch timing information
#[derive(Serialize, Clone, Default)]
pub struct LaunchTimingMirror {
    /// Time since app started (seconds)
    pub uptime_secs: u64,
    /// Time for initial git status refresh (ms), None if still loading
    pub git_time_ms: Option<u64>,
    /// Time for initial diff parsing (ms), None if still loading
    pub diff_time_ms: Option<u64>,
}

/// Capture file info
#[derive(Serialize, Clone)]
pub struct CaptureMirror {
    pub enabled: bool,
    pub directory: String,
    pub scrollback_path: String,
    pub screen_path: String,
}

#[derive(Serialize)]
pub struct MirrorWidgets {
    pub stats: WidgetMirror<StatsMirrorData>,
    pub git: WidgetMirror<GitMirrorData>,
    pub changes: WidgetMirror<ChangesMirrorData>,
}

/// Simplified stats data for JSON
#[derive(Serialize)]
pub struct StatsMirrorData {
    pub work_seconds: u64,
    pub thinking_seconds: u64,
    pub state: String,
    pub prompts: u32,
    pub completions: u32,
    pub tools: u32,
    pub compressions: u32,
}

/// Simplified git data for JSON
#[derive(Serialize)]
pub struct GitMirrorData {
    pub branch: String,
    pub is_repo: bool,
    pub files: Vec<GitFileMirror>,
}

#[derive(Serialize)]
pub struct GitFileMirror {
    pub path: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Simplified changes data for JSON - grouped by language
#[derive(Serialize)]
pub struct ChangesMirrorData {
    pub by_language: Vec<LanguageChangesMirror>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct LanguageChangesMirror {
    pub language: String,
    pub changes: Vec<ChangeMirror>,
}

#[derive(Serialize)]
pub struct ChangeMirror {
    pub kind: String,
    pub name: String,
    pub change_type: String, // "added", "modified", "deleted"
    pub additions: usize,
    pub deletions: usize,
}

/// Publisher that handles throttled state mirroring
pub struct MirrorPublisher {
    enabled: bool,
    session_id: String,
    cwd: String,
    capture: CaptureMirror,
    last_publish: Instant,
    last_hash: u64,
    app_start: Instant,
}

impl MirrorPublisher {
    pub fn new(enabled: bool, session_id: String, cwd: String, capture_enabled: bool) -> Self {
        let session_dir = format!("/tmp/crabigator-{}", session_id);
        let capture = CaptureMirror {
            enabled: capture_enabled,
            directory: session_dir.clone(),
            scrollback_path: format!("{}/scrollback.log", session_dir),
            screen_path: format!("{}/screen.txt", session_dir),
        };

        Self {
            enabled,
            session_id,
            cwd,
            capture,
            // Allow immediate first publish
            last_publish: Instant::now() - Duration::from_secs(10),
            last_hash: 0,
            app_start: Instant::now(),
        }
    }

    /// Get the session directory path
    pub fn session_dir(&self) -> PathBuf {
        PathBuf::from(format!("/tmp/crabigator-{}", self.session_id))
    }

    /// Get the mirror file path (inside session directory)
    pub fn mirror_path(&self) -> PathBuf {
        self.session_dir().join("inspect.json")
    }

    /// Attempt to publish if conditions are met (enabled, changed, throttle elapsed)
    /// Returns true if publish occurred
    pub fn maybe_publish(
        &mut self,
        stats: &SessionStats,
        git: &GitState,
        diff: &DiffSummary,
        terminal_title: Option<&str>,
        initial_git_time_ms: Option<u64>,
        initial_diff_time_ms: Option<u64>,
    ) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }

        // Check throttle
        if self.last_publish.elapsed() < PUBLISH_INTERVAL {
            return Ok(false);
        }

        // Compute hash for change detection
        let hash = self.compute_hash(stats, git, diff, terminal_title);
        if hash == self.last_hash {
            return Ok(false);
        }

        // Publish
        let launch_timing = LaunchTimingMirror {
            uptime_secs: self.app_start.elapsed().as_secs(),
            git_time_ms: initial_git_time_ms,
            diff_time_ms: initial_diff_time_ms,
        };
        let state = self.build_state(stats, git, diff, terminal_title, launch_timing);
        let json = serde_json::to_string_pretty(&state)?;

        // Ensure session directory exists
        let session_dir = self.session_dir();
        fs::create_dir_all(&session_dir)?;

        // Atomic write via temp file + rename
        let path = self.mirror_path();
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, &json)?;
        fs::rename(&tmp_path, &path)?;

        self.last_publish = Instant::now();
        self.last_hash = hash;
        Ok(true)
    }

    fn compute_hash(&self, stats: &SessionStats, git: &GitState, diff: &DiffSummary, terminal_title: Option<&str>) -> u64 {
        let mut hasher = DefaultHasher::new();

        // Hash terminal title
        terminal_title.hash(&mut hasher);

        // Hash key fields from stats
        stats.work_seconds.hash(&mut hasher);
        stats.thinking_seconds().hash(&mut hasher);
        stats.platform_stats.prompts.hash(&mut hasher);
        stats.platform_stats.completions.hash(&mut hasher);
        stats.platform_stats.total_tool_calls().hash(&mut hasher);
        stats.platform_stats.compressions.hash(&mut hasher);
        format!("{:?}", stats.platform_stats.state).hash(&mut hasher);

        // Hash key fields from git
        git.branch.hash(&mut hasher);
        git.files.len().hash(&mut hasher);
        for f in &git.files {
            f.path.hash(&mut hasher);
            f.status.hash(&mut hasher);
            f.additions.hash(&mut hasher);
            f.deletions.hash(&mut hasher);
        }

        // Hash key fields from diff
        diff.files.len().hash(&mut hasher);
        for f in &diff.files {
            f.changes.len().hash(&mut hasher);
            for c in &f.changes {
                c.name.hash(&mut hasher);
                c.additions.hash(&mut hasher);
                c.deletions.hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    fn build_state(
        &self,
        stats: &SessionStats,
        git: &GitState,
        diff: &DiffSummary,
        terminal_title: Option<&str>,
        launch_timing: LaunchTimingMirror,
    ) -> MirrorState {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        MirrorState {
            session_id: self.session_id.clone(),
            cwd: self.cwd.clone(),
            terminal_title: terminal_title.map(String::from),
            last_updated: timestamp,
            capture: self.capture.clone(),
            launch_timing,
            widgets: MirrorWidgets {
                stats: WidgetMirror {
                    data: StatsMirrorData {
                        work_seconds: stats.work_seconds,
                        thinking_seconds: stats.thinking_seconds(),
                        state: format!("{:?}", stats.platform_stats.state).to_lowercase(),
                        prompts: stats.platform_stats.prompts,
                        completions: stats.platform_stats.completions,
                        tools: stats.platform_stats.total_tool_calls(),
                        compressions: stats.platform_stats.compressions,
                    },
                    rendered: render_stats_preview(stats),
                },
                git: WidgetMirror {
                    data: GitMirrorData {
                        branch: git.branch.clone(),
                        is_repo: git.is_repo,
                        files: git
                            .files
                            .iter()
                            .map(|f| GitFileMirror {
                                path: f.path.clone(),
                                status: f.status.clone(),
                                additions: f.additions,
                                deletions: f.deletions,
                            })
                            .collect(),
                    },
                    rendered: render_git_preview(git),
                },
                changes: WidgetMirror {
                    data: ChangesMirrorData {
                        by_language: diff
                            .by_language()
                            .iter()
                            .map(|lc| LanguageChangesMirror {
                                language: lc.language.clone(),
                                changes: lc
                                    .changes
                                    .iter()
                                    .map(|c| ChangeMirror {
                                        kind: format!("{:?}", c.kind).to_lowercase(),
                                        name: c.name.clone(),
                                        change_type: format!("{:?}", c.change_type).to_lowercase(),
                                        additions: c.additions,
                                        deletions: c.deletions,
                                    })
                                    .collect(),
                            })
                            .collect(),
                        total: diff.total_changes(),
                    },
                    rendered: render_changes_preview(diff),
                },
            },
        }
    }

    /// Clean up mirror file on exit
    pub fn cleanup(&self) {
        if self.enabled {
            let _ = fs::remove_file(self.mirror_path());
        }
    }
}

// Preview rendering functions (ANSI-stripped text)

fn render_stats_preview(stats: &SessionStats) -> Vec<String> {
    let mut lines = vec![
        format!("Stats - {:?}", stats.platform_stats.state),
        format!("Session: {}", stats.format_work()),
    ];
    let thinking = stats.format_thinking().unwrap_or_else(|| "—".to_string());
    lines.push(format!("Thinking: {}", thinking));
    lines.extend([
        format!("Prompts: {}", stats.platform_stats.prompts),
        format!("Completions: {}", stats.platform_stats.completions),
        format!("Tools: {}", stats.platform_stats.total_tool_calls()),
    ]);
    if stats.platform_stats.compressions > 0 {
        lines.push(format!("Compressions: {}", stats.platform_stats.compressions));
    }
    // Show idle time if >= 60 seconds
    if let Some(idle_since) = stats.platform_stats.idle_since {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let idle_secs = (now - idle_since) as u64;
        if idle_secs >= 60 {
            let idle_str = if idle_secs >= 3600 {
                format!("{}h{}m", idle_secs / 3600, (idle_secs % 3600) / 60)
            } else {
                format!("{}m", idle_secs / 60)
            };
            lines.push(format!("Idle: {}", idle_str));
        }
    }
    lines
}

fn render_git_preview(git: &GitState) -> Vec<String> {
    let mut lines = vec![];
    if git.branch.is_empty() {
        lines.push("Git (no branch)".to_string());
    } else {
        lines.push(format!("{} - {} files", git.branch, git.files.len()));
    }
    for f in git.files.iter().take(5) {
        lines.push(format!(
            "  {} {} +{}-{}",
            f.status, f.path, f.additions, f.deletions
        ));
    }
    if git.files.len() > 5 {
        lines.push(format!("  ... and {} more", git.files.len() - 5));
    }
    lines
}

fn render_changes_preview(diff: &DiffSummary) -> Vec<String> {
    let by_language = diff.by_language();
    let mut lines = Vec::new();

    for lc in &by_language {
        let count = lc.changes.len();
        let label = if count == 1 { "change" } else { "changes" };
        lines.push(format!("{} - {} {}", lc.language, count, label));

        for c in lc.changes.iter().take(3) {
            let modifier = match c.change_type {
                ChangeType::Added => "+",
                ChangeType::Modified => "~",
                ChangeType::Deleted => "-",
            };
            let stats = if c.additions > 0 || c.deletions > 0 {
                format!(" +{}-{}", c.additions, c.deletions)
            } else {
                String::new()
            };
            lines.push(format!("  {}{:?} {}{}", modifier, c.kind, c.name, stats));
        }
        if lc.changes.len() > 3 {
            lines.push(format!("  ... and {} more", lc.changes.len() - 3));
        }
    }

    if lines.is_empty() {
        lines.push("No changes".to_string());
    }

    lines
}
