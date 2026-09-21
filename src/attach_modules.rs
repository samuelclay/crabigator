//! The status modules under an attached session.
//!
//! The original session publishes `inspect.json`. The attach view redraws
//! that snapshot at its own width: recap, PRs, then stats, git, and changes.

use std::io::{self, Write};
use std::path::Path;

use serde_json::Value;

use crate::git::{FileStatus, GitState};
use crate::hooks::SessionStats;
use crate::ide::IdeKind;
use crate::parsers::{ChangeNode, ChangeType, DiffSummary, FileChanges, NodeKind};
use crate::platforms::{ClaudeMode, PlatformStats, SessionState};
use crate::pr::SessionPr;
use crate::recap::RecapState;
use crate::session_mark::SessionMark;
use crate::slack::SlackThread;
use crate::ui::cooldown::Cooldowns;
use crate::ui::{compute_dynamic_status_rows, draw_status_bar, handoff_rows, Layout, PairingState};
use crate::update::UpdateState;

/// Lines for the modules under the live screen. Empty when the session has
/// no mirror yet.
pub fn session_modules(session_dir: &Path, width: u16, terminal_rows: u16) -> Vec<String> {
    if width == 0 || terminal_rows == 0 {
        return Vec::new();
    }
    let Some(snapshot) = load_snapshot(session_dir) else {
        return Vec::new();
    };
    // Leave the agent screen and the Secondary bar some room.
    let budget = terminal_rows.saturating_sub(6).max(1);
    let mut lines = render_snapshot(&snapshot, width, terminal_rows);
    if lines.len() > budget as usize {
        lines.truncate(budget as usize);
    }
    lines
}

struct Snapshot {
    stats: SessionStats,
    git: GitState,
    diff: DiffSummary,
    recap: RecapState,
    prs: Vec<SessionPr>,
    slack: Vec<SlackThread>,
    title: Option<String>,
    cwd: String,
    pr_scope: String,
    mark: SessionMark,
}

fn load_snapshot(session_dir: &Path) -> Option<Snapshot> {
    let bytes = std::fs::read(session_dir.join("inspect.json")).ok()?;
    let data: Value = serde_json::from_slice(&bytes).ok()?;
    Some(snapshot_from_json(&data))
}

fn snapshot_from_json(data: &Value) -> Snapshot {
    let session_id = data
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let cwd = data
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let pr_scope = data
        .get("pr_scope")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("session:{session_id}"));
    let title = data
        .get("terminal_title")
        .and_then(|v| v.as_str())
        .filter(|title| !title.is_empty())
        .map(str::to_string);
    let prs = data
        .get("prs")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let slack = data
        .get("slack_threads")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let recap = data
        .get("recap")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    Snapshot {
        stats: stats_from_json(data.get("widgets").and_then(|w| w.get("stats"))),
        git: git_from_json(data.get("widgets").and_then(|w| w.get("git"))),
        diff: diff_from_json(data.get("widgets").and_then(|w| w.get("changes"))),
        recap,
        prs,
        slack,
        title,
        cwd,
        pr_scope,
        mark: SessionMark::from_mirror(data.get("session_mark"), session_id),
    }
}

fn stats_from_json(stats: Option<&Value>) -> SessionStats {
    let data = stats.and_then(|stats| stats.get("data"));
    let number = |key: &str| data.and_then(|d| d.get(key)).and_then(|v| v.as_u64());
    let state = data
        .and_then(|d| d.get("state"))
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_value::<SessionState>(Value::String(s.to_string())).ok())
        .unwrap_or_default();
    let mode = match data.and_then(|d| d.get("mode")).and_then(|v| v.as_str()) {
        Some("auto_accept" | "autoaccept") => ClaudeMode::AutoAccept,
        Some("plan") => ClaudeMode::Plan,
        _ => ClaudeMode::Normal,
    };
    let tool_count = number("tools").unwrap_or(0) as u32;
    let mut tools = std::collections::HashMap::new();
    if tool_count > 0 {
        tools.insert("calls".to_string(), tool_count);
    }
    let platform = PlatformStats {
        prompts: number("prompts").unwrap_or(0) as u32,
        completions: number("completions").unwrap_or(0) as u32,
        compressions: number("compressions").unwrap_or(0) as u32,
        tools,
        tool_timestamps: data
            .and_then(|d| d.get("tool_timestamps"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        state,
        mode,
        active_prompt: data
            .and_then(|d| d.get("active_prompt"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        ..PlatformStats::default()
    };
    let stamp = |key: &str| data.and_then(|d| d.get(key)).and_then(|v| v.as_f64());
    SessionStats::from_published(
        number("work_seconds").unwrap_or(0),
        number("thinking_seconds").unwrap_or(0),
        platform,
        stamp("prompts_changed_at"),
        stamp("completions_changed_at"),
        stamp("compressions_changed_at"),
    )
}

fn git_from_json(git: Option<&Value>) -> GitState {
    let data = git.and_then(|git| git.get("data"));
    let files = data
        .and_then(|d| d.get("files"))
        .and_then(|v| v.as_array())
        .map(|files| {
            files
                .iter()
                .filter_map(|file| {
                    Some(FileStatus {
                        status: file.get("status")?.as_str()?.to_string(),
                        path: file.get("path")?.as_str()?.to_string(),
                        additions: file.get("additions").and_then(|v| v.as_u64()).unwrap_or(0)
                            as usize,
                        deletions: file.get("deletions").and_then(|v| v.as_u64()).unwrap_or(0)
                            as usize,
                        is_folder: false,
                        file_count: 0,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    GitState {
        files,
        branch: data
            .and_then(|d| d.get("branch"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        repo_owner: data
            .and_then(|d| d.get("repo_owner"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        repo_name: data
            .and_then(|d| d.get("repo_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        is_repo: data
            .and_then(|d| d.get("is_repo"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        loading: false,
        ..GitState::default()
    }
}

fn diff_from_json(changes: Option<&Value>) -> DiffSummary {
    let groups = changes
        .and_then(|changes| changes.get("data"))
        .and_then(|d| d.get("by_language"))
        .and_then(|v| v.as_array());
    let Some(groups) = groups else {
        return DiffSummary::default();
    };
    let files = groups
        .iter()
        .filter_map(|group| {
            let language = group.get("language")?.as_str()?.to_string();
            let changes = group
                .get("changes")?
                .as_array()?
                .iter()
                .filter_map(|change| {
                    Some(ChangeNode {
                        kind: node_kind(change.get("kind")?.as_str()?),
                        name: change.get("name")?.as_str()?.to_string(),
                        scope: Vec::new(),
                        change_type: change_type(
                            change
                                .get("change_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or(""),
                        ),
                        additions: change
                            .get("additions")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize,
                        deletions: change
                            .get("deletions")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as usize,
                        file_path: change
                            .get("file_path")
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        line_number: change
                            .get("line_number")
                            .and_then(|v| v.as_u64())
                            .map(|n| n as usize),
                        children: Vec::new(),
                    })
                });
            Some(FileChanges {
                path: language.clone(),
                language,
                changes: changes.collect(),
            })
        })
        .collect();
    DiffSummary {
        files,
        loading: false,
    }
}

fn node_kind(name: &str) -> NodeKind {
    match name {
        "class" => NodeKind::Class,
        "function" => NodeKind::Function,
        "method" => NodeKind::Method,
        "struct" => NodeKind::Struct,
        "enum" => NodeKind::Enum,
        "trait" => NodeKind::Trait,
        "impl" => NodeKind::Impl,
        "module" => NodeKind::Module,
        "const" => NodeKind::Const,
        _ => NodeKind::Other,
    }
}

fn change_type(name: &str) -> ChangeType {
    match name {
        "added" => ChangeType::Added,
        "deleted" => ChangeType::Deleted,
        _ => ChangeType::Modified,
    }
}

fn render_snapshot(snapshot: &Snapshot, width: u16, terminal_rows: u16) -> Vec<String> {
    let pairing = PairingState::default();
    let update = UpdateState::default();
    let handoff = handoff_rows(
        width,
        &pairing,
        &update,
        &snapshot.recap,
        false,
        &snapshot.prs,
    );
    let cwd = Path::new(&snapshot.cwd);
    let status_rows = compute_dynamic_status_rows(
        terminal_rows,
        width,
        &snapshot.stats,
        &snapshot.git,
        &snapshot.diff,
        snapshot.title.as_deref(),
        &snapshot.prs,
        &snapshot.slack,
        handoff,
    );
    let layout = Layout {
        pty_rows: 0,
        total_cols: width,
        status_rows,
        handoff_rows: handoff,
    };
    let mut screen = AnsiScreen::new(width as usize, (handoff + status_rows) as usize);
    let _ = draw_status_bar(
        &mut screen,
        &layout,
        &snapshot.stats,
        &snapshot.git,
        &snapshot.diff,
        snapshot.title.as_deref(),
        &snapshot.slack,
        IdeKind::None,
        cwd,
        None,
        &pairing,
        &update,
        &snapshot.recap,
        false,
        &snapshot.prs,
        &snapshot.pr_scope,
        None,
        &Cooldowns::default(),
        0,
        snapshot.mark,
        true,
    );
    screen.lines()
}

/// A tiny screen that understands the cursor moves the widgets emit.
struct AnsiScreen {
    width: usize,
    cells: Vec<Vec<Cell>>,
    row: usize,
    col: usize,
    style: Vec<String>,
    markup: Vec<(usize, usize, String)>,
    pending: Vec<u8>,
}

#[derive(Clone)]
struct Cell {
    ch: char,
    skip: bool,
    style: Vec<String>,
}

impl Cell {
    fn blank() -> Self {
        Self {
            ch: ' ',
            skip: false,
            style: Vec::new(),
        }
    }
}

impl AnsiScreen {
    fn new(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let row = vec![Cell::blank(); width];
        Self {
            width,
            cells: vec![row; height.max(1)],
            row: 0,
            col: 0,
            style: Vec::new(),
            markup: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for (index, row) in self.cells.iter().enumerate() {
            lines.push(render_row(row, &self.markup, index));
        }
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        lines
    }

    fn ensure_row(&mut self, row: usize) {
        while self.cells.len() <= row {
            self.cells.push(vec![Cell::blank(); self.width]);
        }
    }

    fn put(&mut self, ch: char) {
        let width = unicode_width::UnicodeWidthChar::width(ch)
            .unwrap_or(1)
            .max(1);
        if self.col + width > self.width {
            return;
        }
        self.ensure_row(self.row);
        self.cells[self.row][self.col] = Cell {
            ch,
            skip: false,
            style: self.style.clone(),
        };
        for offset in 1..width {
            self.cells[self.row][self.col + offset] = Cell {
                ch: '\0',
                skip: true,
                style: Vec::new(),
            };
        }
        self.col += width;
    }

    fn apply_sgr(&mut self, params: &str) {
        if params.is_empty() || params.split(';').any(|part| part == "0") {
            self.style.clear();
            return;
        }
        self.style.push(params.to_string());
    }

    fn move_to(&mut self, row: usize, col: usize) {
        self.row = row.saturating_sub(1);
        self.col = col.saturating_sub(1).min(self.width);
    }

    fn clear_line(&mut self) {
        self.ensure_row(self.row);
        self.cells[self.row] = vec![Cell::blank(); self.width];
        self.markup.retain(|(row, _, _)| *row != self.row);
    }

    fn clear_down(&mut self) {
        self.clear_line();
        if self.row + 1 < self.cells.len() {
            self.cells.truncate(self.row + 1);
        }
    }

    fn pump(&mut self) {
        loop {
            if self.pending.is_empty() {
                return;
            }
            if self.pending[0] != 0x1b {
                let width = utf8_width(self.pending[0]);
                if self.pending.len() < width {
                    return;
                }
                let bytes: Vec<u8> = self.pending.drain(..width).collect();
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    if let Some(ch) = text.chars().next() {
                        self.put(ch);
                    }
                }
                continue;
            }
            if self.pending.len() < 2 {
                return;
            }
            match self.pending[1] {
                b'[' => {
                    let Some(end) = self.pending[2..]
                        .iter()
                        .position(|byte| (0x40..=0x7e).contains(byte))
                    else {
                        return;
                    };
                    let final_at = 2 + end;
                    let params = String::from_utf8_lossy(&self.pending[2..final_at]).into_owned();
                    let kind = self.pending[final_at];
                    self.pending.drain(..=final_at);
                    if params.starts_with('?') {
                        continue;
                    }
                    match kind {
                        b'H' | b'f' => {
                            let mut parts = params.split(';');
                            let row = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
                            let col = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
                            self.move_to(row, col);
                        }
                        b'G' => {
                            let col: usize = params.parse().unwrap_or(1);
                            self.col = col.saturating_sub(1).min(self.width);
                        }
                        b'm' => self.apply_sgr(&params),
                        b'K' => self.clear_line(),
                        b'J' => self.clear_down(),
                        _ => {}
                    }
                }
                b']' => {
                    let Some(end) = osc_end(&self.pending) else {
                        return;
                    };
                    let raw = String::from_utf8_lossy(&self.pending[..=end]).into_owned();
                    self.pending.drain(..=end);
                    self.markup.push((self.row, self.col, raw));
                }
                _ => {
                    self.pending.drain(..2);
                }
            }
        }
    }
}

fn utf8_width(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xf7 => 3,
        0xf8..=0xfb => 4,
        _ => 1,
    }
}

fn osc_end(bytes: &[u8]) -> Option<usize> {
    let mut i = 2;
    while i < bytes.len() {
        if bytes[i] == 0x07 {
            return Some(i);
        }
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'\\') {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn render_row(row: &[Cell], markup: &[(usize, usize, String)], row_index: usize) -> String {
    if row
        .iter()
        .all(|cell| cell.skip || cell.ch == ' ' || cell.ch == '\0')
        && !markup.iter().any(|(r, _, _)| *r == row_index)
    {
        return String::new();
    }
    let mut out = String::new();
    let mut current = String::new();
    for (col, cell) in row.iter().enumerate() {
        for (mark_row, mark_col, raw) in markup {
            if *mark_row == row_index && *mark_col == col {
                out.push_str(raw);
            }
        }
        if cell.skip {
            continue;
        }
        let style = cell.style.join(";");
        if style != current {
            if style.is_empty() {
                out.push_str("\x1b[0m");
            } else {
                out.push_str(&format!("\x1b[{style}m"));
            }
            current = style;
        }
        out.push(cell.ch);
    }
    if !current.is_empty() {
        out.push_str("\x1b[0m");
    }
    out.trim_end().to_string()
}

impl Write for AnsiScreen {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        self.pump();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::escape;

    #[test]
    fn screen_places_text_at_the_cursor() {
        let mut screen = AnsiScreen::new(20, 3);
        write!(
            screen,
            "{}{}Hi{}",
            escape::cursor_to(2, 3),
            escape::fg(39),
            escape::RESET
        )
        .unwrap();
        let lines = screen.lines();
        assert!(lines[0].is_empty());
        assert!(lines[1].contains("Hi"));
        assert!(lines[1].contains("38;5;39"));
    }

    #[test]
    fn modules_include_the_recap_pr_git_and_changes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("inspect.json"),
            r#"{
                "session_id": "abc",
                "cwd": "/tmp/work",
                "terminal_title": "Fix the parser",
                "recap": {
                    "enabled": true,
                    "status": "Ready",
                    "latest": {
                        "prompt_count": 1,
                        "generated_at": 1,
                        "variant": "brief",
                        "headline": "Recap headline here",
                        "bullets": [],
                        "next_prompt_notes": [],
                        "artifacts": [],
                        "line_delta": {"additions": 1, "deletions": 0}
                    },
                    "line_delta": null,
                    "model": "test"
                },
                "prs": [{
                    "number": 42,
                    "owner": "acme",
                    "repo": "widgets",
                    "url": "https://github.com/acme/widgets/pull/42",
                    "branch": "sam/modules",
                    "title": "Show the modules",
                    "state": "OPEN",
                    "is_draft": false,
                    "additions": 10,
                    "deletions": 2,
                    "changed_files": 1,
                    "mergeable": "MERGEABLE",
                    "merge_state_status": "CLEAN",
                    "checks_passed": 1,
                    "checks_failed": 0,
                    "checks_pending": 0,
                    "checks_total": 1,
                    "created_here": true,
                    "primary": true,
                    "refreshed_at": 1
                }],
                "widgets": {
                    "stats": {"data": {
                        "work_seconds": 120,
                        "thinking_seconds": 30,
                        "state": "ready",
                        "mode": "normal",
                        "prompts": 3,
                        "completions": 2,
                        "tools": 4,
                        "compressions": 0,
                        "tool_timestamps": [],
                        "session_start": 0
                    }},
                    "git": {"data": {
                        "repo_owner": "acme",
                        "repo_name": "widgets",
                        "branch": "sam/modules",
                        "is_repo": true,
                        "files": [{"path": "src/app.rs", "status": "M", "additions": 4, "deletions": 1}]
                    }},
                    "changes": {"data": {
                        "by_language": [{
                            "language": "Rust",
                            "changes": [{
                                "kind": "function",
                                "name": "session_modules",
                                "change_type": "modified",
                                "additions": 4,
                                "deletions": 1
                            }]
                        }],
                        "total": 1
                    }}
                }
            }"#,
        )
        .unwrap();
        let lines = session_modules(dir.path(), 120, 40);
        let text = lines.join("\n");
        assert!(text.contains("Recap headline here"), "{text}");
        assert!(text.contains("Show the modules"), "{text}");
        assert!(text.contains("42") || text.contains("#42"), "{text}");
        assert!(text.contains("app.rs"), "{text}");
        assert!(text.contains("session_modules"), "{text}");
        assert!(text.contains("Session"), "{text}");
        assert!(text.contains("secondary"), "{text}");
    }
}
