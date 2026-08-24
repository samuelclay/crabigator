//! Configuration management for Crabigator
//!
//! Handles loading and saving user preferences, including the default platform.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Crabigator configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Default platform: "claude" or "codex"
    #[serde(default = "default_platform")]
    pub default_platform: String,

    /// IDE for clickable hyperlinks: "vscode", "cursor", "idea", "zed", "sublime", or "none"
    /// If not set, auto-detects from environment
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ide: Option<String>,

    /// Whether to check for updates on startup
    #[serde(default = "default_true")]
    pub check_for_updates: bool,

    /// Terminal emulator override: "terminal" or "ghostty"
    /// If not set, auto-detects from $TERM_PROGRAM
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,

    /// Whether automatic turn recaps are enabled.
    #[serde(default = "default_true")]
    pub recap_enabled: bool,

    /// Anthropic model used for recap generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recap_model: Option<String>,

    /// Persistent view preferences for `crabigator prs`.
    #[serde(default)]
    pub pr_board: PrBoardPreferences,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrBoardPreferences {
    /// Whether the board opens with durable ended sessions included.
    #[serde(default)]
    pub include_ended: bool,
    /// Recap visibility: 0 = hidden, 1 = shown.
    #[serde(default)]
    pub detail: u8,
    /// Number of days to keep completed PRs on the board.
    #[serde(default = "default_pr_board_linger_days")]
    pub linger_days: u64,
    /// Oldest activity shown by default, in hours. None means all activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oldest_visible_hours: Option<u64>,
    /// Which grouping the board opens in: "sessions" (one row per session)
    /// or "prs" (one block per primary PR with its sessions beneath).
    #[serde(default = "default_pr_board_view")]
    pub view: String,
}

fn default_true() -> bool {
    true
}

fn default_platform() -> String {
    "claude".to_string()
}

fn default_pr_board_linger_days() -> u64 {
    1
}

fn default_pr_board_view() -> String {
    "sessions".to_string()
}

impl Default for PrBoardPreferences {
    fn default() -> Self {
        Self {
            include_ended: false,
            detail: 0,
            linger_days: default_pr_board_linger_days(),
            oldest_visible_hours: None,
            view: default_pr_board_view(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_platform: default_platform(),
            ide: None,
            check_for_updates: true,
            terminal: None,
            recap_enabled: true,
            recap_model: None,
            pr_board: PrBoardPreferences::default(),
        }
    }
}

impl Config {
    /// Get config directory path (~/.crabigator)
    pub fn config_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".crabigator")
    }

    /// Get config file path (~/.crabigator/config.toml)
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Load config from file, or return default if not found
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config from {}", path.display()))
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory {}", dir.display()))?;

        let path = Self::config_path();
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;

        // Atomic write: write to temp file then rename
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, &contents)
            .with_context(|| format!("Failed to write config to {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to rename config file to {}", path.display()))?;

        Ok(())
    }

    /// Update the default platform and save
    pub fn set_default_platform(&mut self, platform: &str) -> Result<()> {
        self.default_platform = platform.to_string();
        self.save()
    }

    /// Get the secret file path for the Anthropic API key used by recaps.
    pub fn recap_key_path() -> PathBuf {
        Self::config_dir().join("anthropic_api_key")
    }

    /// Read the stored recap API key, if present.
    pub fn read_recap_api_key() -> Result<Option<String>> {
        let path = Self::recap_key_path();
        if !path.exists() {
            return Ok(None);
        }
        let key = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?
            .trim()
            .to_string();
        if key.is_empty() {
            Ok(None)
        } else {
            Ok(Some(key))
        }
    }

    /// Store the recap API key in a separate file so normal config can be shared safely.
    pub fn write_recap_api_key(api_key: &str) -> Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory {}", dir.display()))?;

        let path = Self::recap_key_path();
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, api_key.trim())
            .with_context(|| format!("Failed to write {}", tmp_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&tmp_path, perms)
                .with_context(|| format!("Failed to chmod {}", tmp_path.display()))?;
        }

        fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to rename recap key file to {}", path.display()))?;
        Ok(())
    }

    /// Remove the stored recap API key, if present.
    pub fn remove_recap_api_key() -> Result<()> {
        let path = Self::recap_key_path();
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("Failed to remove {}", path.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_pr_board_preferences_keep_the_existing_defaults() {
        let config: Config = toml::from_str("default_platform = \"codex\"").unwrap();
        assert!(!config.pr_board.include_ended);
        assert_eq!(config.pr_board.detail, 0);
        assert_eq!(config.pr_board.linger_days, 1);
        assert_eq!(config.pr_board.oldest_visible_hours, None);
        assert_eq!(config.pr_board.view, "sessions");
    }

    #[test]
    fn pr_board_preferences_round_trip() {
        let mut config = Config::default();
        config.pr_board.include_ended = true;
        config.pr_board.detail = 1;
        config.pr_board.linger_days = 7;
        config.pr_board.oldest_visible_hours = Some(9);
        config.pr_board.view = "prs".to_string();

        let encoded = toml::to_string(&config).unwrap();
        let decoded: Config = toml::from_str(&encoded).unwrap();
        assert!(decoded.pr_board.include_ended);
        assert_eq!(decoded.pr_board.detail, 1);
        assert_eq!(decoded.pr_board.linger_days, 7);
        assert_eq!(decoded.pr_board.oldest_visible_hours, Some(9));
        assert_eq!(decoded.pr_board.view, "prs");
    }

    #[test]
    fn existing_pr_board_detail_is_loaded() {
        let config: Config =
            toml::from_str("[pr_board]\ninclude_ended = true\ndetail = 3\nlinger_days = 7\n")
                .unwrap();

        assert!(config.pr_board.include_ended);
        assert_eq!(config.pr_board.detail, 3);
        assert_eq!(config.pr_board.linger_days, 7);
        assert_eq!(config.pr_board.oldest_visible_hours, None);
    }
}
