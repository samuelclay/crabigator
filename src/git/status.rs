use anyhow::Result;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub struct FileStatus {
    pub status: String,
    pub path: String,
}

#[derive(Clone, Debug, Default)]
pub struct GitState {
    pub files: Vec<FileStatus>,
    pub branch: String,
    pub is_repo: bool,
}

impl GitState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn refresh(&self) -> Result<Self> {
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
                        state.files.push(FileStatus { status, path });
                    }
                }
            }
        }

        Ok(state)
    }
}
