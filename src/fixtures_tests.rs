#[cfg(test)]
mod fixtures {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Once;

    use anyhow::{bail, Context, Result};
    use serde::Deserialize;
    use serde_json::Value;
    use tempfile::TempDir;
    use tokio::process::Command;

    use crate::git::GitState;
    use crate::hooks::SessionStats;
    use crate::mirror::MirrorPublisher;
    use crate::parsers::DiffSummary;
    use crate::platforms::{PlatformStats, SessionState};

    static INIT_ENV: Once = Once::new();

    #[derive(Debug, Deserialize)]
    struct FixtureConfig {
        stats: FixtureStats,
        #[serde(default)]
        staged_paths: Vec<String>,
        #[serde(default)]
        remove_paths: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureStats {
        state: String,
        messages: u32,
        #[serde(default)]
        tools: HashMap<String, u32>,
        compressions: u32,
        work_seconds: u64,
    }

    #[tokio::test]
    async fn fixture_snapshots() -> Result<()> {
        INIT_ENV.call_once(|| {
            std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
            std::env::set_var("GIT_CONFIG_SYSTEM", "/dev/null");
            std::env::set_var("GIT_ATTR_NOSYSTEM", "1");
        });

        let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut fixture_dirs = Vec::new();

        for entry in fs::read_dir(&fixtures_root).context("read fixtures directory")? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                fixture_dirs.push(entry.path());
            }
        }

        fixture_dirs.sort();

        for fixture_dir in fixture_dirs {
            run_fixture(&fixture_dir).await.with_context(|| {
                format!(
                    "fixture {}",
                    fixture_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                )
            })?;
        }

        Ok(())
    }

    async fn run_fixture(fixture_dir: &Path) -> Result<()> {
        let config_path = fixture_dir.join("fixture.json");
        let config_contents = fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?;
        let config: FixtureConfig = serde_json::from_str(&config_contents)
            .with_context(|| format!("parse {}", config_path.display()))?;

        let base_dir = fixture_dir.join("base");
        let worktree_dir = fixture_dir.join("worktree");
        let fixture_name = fixture_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("fixture");

        let temp_dir = TempDir::new().context("create temp dir")?;
        let repo_dir = temp_dir.path();

        copy_dir(&base_dir, repo_dir)?;
        init_repo(repo_dir).await?;

        copy_dir(&worktree_dir, repo_dir)?;
        remove_paths(repo_dir, &config.remove_paths)?;

        stage_paths(repo_dir, &config.staged_paths).await?;

        let git_state = GitState::new().refresh_in_dir(repo_dir).await?;
        let diff_summary = DiffSummary::new().refresh_in_dir(repo_dir).await?;

        let stats = build_stats(&config.stats);
        let mut publisher = MirrorPublisher::new(
            true,
            format!("fixture-{}", fixture_name),
            fixture_name.to_string(),
            false,
        );

        let published = publisher.maybe_publish(&stats, &git_state, &diff_summary)?;
        if !published {
            bail!("mirror publish throttled or unchanged");
        }

        let mirror_path = publisher.mirror_path();
        let mirror_contents = fs::read_to_string(&mirror_path)
            .with_context(|| format!("read {}", mirror_path.display()))?;
        let mut actual_json: Value = serde_json::from_str(&mirror_contents)
            .with_context(|| format!("parse {}", mirror_path.display()))?;
        normalize_mirror(&mut actual_json);
        publisher.cleanup();

        let expected_path = fixture_dir.join("expected.json");
        if should_update_fixtures() {
            let pretty = serde_json::to_string_pretty(&actual_json)?;
            fs::write(&expected_path, format!("{}\n", pretty))
                .with_context(|| format!("write {}", expected_path.display()))?;
            return Ok(());
        }

        let expected_contents = fs::read_to_string(&expected_path)
            .with_context(|| format!("read {}", expected_path.display()))?;
        let expected_json: Value = serde_json::from_str(&expected_contents)
            .with_context(|| format!("parse {}", expected_path.display()))?;

        if expected_json != actual_json {
            bail!("fixture snapshot mismatch: {}", expected_path.display());
        }

        Ok(())
    }

    fn build_stats(stats: &FixtureStats) -> SessionStats {
        let mut result = SessionStats::new();
        result.work_seconds = stats.work_seconds;
        result.platform_stats = PlatformStats {
            messages: stats.messages,
            tools: stats.tools.clone(),
            compressions: stats.compressions,
            state: parse_state(&stats.state),
            ..PlatformStats::default()
        };
        result
    }

    fn parse_state(state: &str) -> SessionState {
        match state {
            "ready" => SessionState::Ready,
            "thinking" | "planning" => SessionState::Thinking,
            "question" => SessionState::Question,
            "complete" => SessionState::Complete,
            other => panic!("unknown session state: {}", other),
        }
    }

    async fn init_repo(dir: &Path) -> Result<()> {
        run_git(dir, &["init", "-b", "main"]).await?;
        run_git(dir, &["config", "user.name", "Crabigator Tests"]).await?;
        run_git(dir, &["config", "user.email", "tests@example.com"]).await?;
        run_git(dir, &["config", "commit.gpgsign", "false"]).await?;
        run_git(dir, &["add", "."]).await?;
        run_git(dir, &["commit", "-m", "base"])
            .await
            .context("commit base")?;
        Ok(())
    }

    async fn stage_paths(dir: &Path, paths: &[String]) -> Result<()> {
        for path in paths {
            run_git(dir, &["add", path]).await?;
        }
        Ok(())
    }

    async fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .await
            .with_context(|| format!("run git {:?}", args))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git {:?} failed: {}", args, stderr.trim());
        }

        Ok(())
    }

    fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
        if !src.exists() {
            return Ok(());
        }

        fs::create_dir_all(dest)
            .with_context(|| format!("create dir {}", dest.display()))?;

        for entry in fs::read_dir(src).with_context(|| format!("read dir {}", src.display()))? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if file_type.is_dir() {
                copy_dir(&src_path, &dest_path)?;
            } else if file_type.is_file() {
                fs::copy(&src_path, &dest_path).with_context(|| {
                    format!("copy {} to {}", src_path.display(), dest_path.display())
                })?;
            }
        }

        Ok(())
    }

    fn remove_paths(root: &Path, paths: &[String]) -> Result<()> {
        for path in paths {
            let full_path = root.join(path);
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path)
                    .with_context(|| format!("remove dir {}", full_path.display()))?;
            } else if full_path.exists() {
                fs::remove_file(&full_path)
                    .with_context(|| format!("remove file {}", full_path.display()))?;
            }
        }
        Ok(())
    }

    fn normalize_mirror(value: &mut Value) {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("last_updated".to_string(), Value::from(0.0));
        }
    }

    fn should_update_fixtures() -> bool {
        matches!(std::env::var("CRABIGATOR_UPDATE_FIXTURES"), Ok(v) if v == "1" || v == "true")
    }
}
