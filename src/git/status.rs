use anyhow::Result;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub struct FileStatus {
    pub status: String,
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub is_folder: bool,
    pub file_count: usize,
}

impl FileStatus {
    /// Total lines changed (additions + deletions)
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions
    }
}

#[derive(Clone, Debug, Default)]
pub struct GitState {
    pub files: Vec<FileStatus>,
    pub branch: String,
    pub is_repo: bool,
    pub loading: bool,
}

impl GitState {
    pub fn new() -> Self {
        Self {
            loading: true,
            ..Self::default()
        }
    }

    pub async fn refresh(&self) -> Result<Self> {
        let profile = std::env::var("CRABIGATOR_PROFILE").is_ok();
        let start = std::time::Instant::now();
        let mut state = GitState::default();

        // Check if we're in a git repo
        let status_output = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .await;

        match status_output {
            Ok(output) if output.status.success() => {
                state.is_repo = true;
            }
            _ => {
                return Ok(state);
            }
        }

        // Get current branch
        if let Ok(output) = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .await
        {
            if output.status.success() {
                state.branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }

        // Get file statuses using porcelain format
        if let Ok(output) = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .await
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.len() >= 3 {
                        let status = line[0..2].trim().to_string();
                        let path = line[3..].to_string();

                        // Detect if this is an untracked folder
                        let is_folder = status == "??" && path.ends_with('/');

                        state.files.push(FileStatus {
                            status,
                            path,
                            additions: 0,
                            deletions: 0,
                            is_folder,
                            file_count: 0,
                        });
                    }
                }
            }
        }

        // Count files in untracked folders
        // Note: This can be slow on large directories (node_modules, venv, etc.)
        let folder_start = std::time::Instant::now();
        for file in &mut state.files {
            if file.is_folder {
                file.file_count = Self::count_files_in_folder(&file.path).await;
            }
        }
        if std::env::var("CRABIGATOR_PROFILE").is_ok() && folder_start.elapsed().as_millis() > 100 {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/crabigator-profile.log") {
                use std::io::Write;
                let _ = writeln!(f, "[profile] count_files_in_folder took {:?}", folder_start.elapsed());
            }
        }

        // Get diff --numstat for line counts
        if let Ok(output) = Command::new("git")
            .args(["diff", "--numstat"])
            .output()
            .await
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Self::parse_numstat(&stdout, &mut state.files);
            }
        }

        // Also get staged diff stats
        if let Ok(output) = Command::new("git")
            .args(["diff", "--cached", "--numstat"])
            .output()
            .await
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Self::parse_numstat(&stdout, &mut state.files);
            }
        }

        // Sort files by total changes (descending)
        state.files.sort_by(|a, b| b.total_changes().cmp(&a.total_changes()));

        if profile && start.elapsed().as_millis() > 100 {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/crabigator-profile.log") {
                use std::io::Write;
                let _ = writeln!(f, "[profile] GitState::refresh took {:?}", start.elapsed());
            }
        }

        Ok(state)
    }

    fn parse_numstat(numstat: &str, files: &mut [FileStatus]) {
        for line in numstat.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let additions = parts[0].parse::<usize>().unwrap_or(0);
                let deletions = parts[1].parse::<usize>().unwrap_or(0);
                let path = parts[2];

                // Find matching file and update stats
                if let Some(file) = files.iter_mut().find(|f| f.path == path) {
                    file.additions += additions;
                    file.deletions += deletions;
                }
            }
        }
    }

    /// Count files inside an untracked folder using find
    async fn count_files_in_folder(path: &str) -> usize {
        // Skip known slow directories
        let slow_dirs = ["node_modules", "venv", ".venv", "__pycache__", "target", ".git", "vendor"];
        if slow_dirs.iter().any(|d| path.contains(d)) {
            return 0;
        }

        // Use timeout to prevent blocking on large directories
        let folder_path = path.trim_end_matches('/').to_string();
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            Command::new("find")
                .args([&folder_path, "-type", "f", "-maxdepth", "3"])
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().count()
            }
            _ => 0,
        }
    }
}
