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
}

fn default_platform() -> String {
    "claude".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_platform: default_platform(),
            ide: None,
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
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

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
}
