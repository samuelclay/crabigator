//! Auto-update system for Crabigator
//!
//! Checks for new versions via GitHub Releases API and prompts users to update.
//! Supports multiple installation methods (npm, cargo, homebrew) with appropriate
//! update commands for each.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cloud::DeviceIdentity;
use crate::config::Config;

/// Current version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for API calls
const GITHUB_REPO: &str = "samuelclay/crabigator";

/// Cache duration before checking again (20 hours)
const CACHE_DURATION_SECS: u64 = 20 * 60 * 60;

/// Version cache stored at ~/.crabigator/version.json
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VersionCache {
    /// Unix timestamp of last check
    #[serde(default)]
    pub last_checked: u64,
    /// Latest version found (e.g., "0.4.0")
    #[serde(default)]
    pub latest_version: Option<String>,
    /// Version user said "no" to (won't prompt again for this version)
    #[serde(default)]
    pub dismissed_version: Option<String>,
    /// URL to the release page
    #[serde(default)]
    pub release_url: Option<String>,
}

impl VersionCache {
    /// Path to version cache file
    fn cache_path() -> PathBuf {
        Config::config_dir().join("version.json")
    }

    /// Load cache from disk, or return default if not found
    pub fn load() -> Self {
        let path = Self::cache_path();
        if !path.exists() {
            return Self::default();
        }

        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<()> {
        let dir = Config::config_dir();
        fs::create_dir_all(&dir)?;

        let path = Self::cache_path();
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Check if cache is stale (older than CACHE_DURATION_SECS)
    pub fn is_stale(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.last_checked) > CACHE_DURATION_SECS
    }

    /// Update the cache with new version info
    pub fn update(&mut self, latest: String, release_url: Option<String>) {
        self.last_checked = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.latest_version = Some(latest);
        self.release_url = release_url;
    }
}

/// Result of checking for updates
#[derive(Clone, Debug)]
pub struct UpdateCheckResult {
    /// Whether an update is available
    pub update_available: bool,
    /// New version string (e.g., "0.4.0")
    pub new_version: Option<String>,
    /// Current installed version
    #[allow(dead_code)]
    pub current_version: String,
    /// Whether user previously dismissed this version
    pub was_dismissed: bool,
    /// URL to the release page (for Unknown install method)
    #[allow(dead_code)]
    pub release_url: Option<String>,
}

/// How Crabigator was installed
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstallMethod {
    /// Installed via npm (npm install -g crabigator)
    Npm,
    /// Installed via cargo (cargo install --git ...)
    Cargo,
    /// Installed via Homebrew (brew install crabigator)
    Homebrew,
    /// Unknown installation method
    #[default]
    Unknown,
}

impl InstallMethod {
    /// Get the shell command to run for updating
    pub fn update_command(&self) -> &'static str {
        match self {
            InstallMethod::Npm => "npm install -g crabigator@latest",
            InstallMethod::Cargo => "cargo install --git https://github.com/samuelclay/crabigator",
            InstallMethod::Homebrew => "brew upgrade crabigator",
            InstallMethod::Unknown => "https://github.com/samuelclay/crabigator/releases",
        }
    }

    /// Get a short description for the banner
    pub fn banner_command(&self) -> &'static str {
        match self {
            InstallMethod::Npm => "npm install -g crabigator@latest",
            InstallMethod::Cargo => "cargo install --git github.com/samuelclay/crabigator",
            InstallMethod::Homebrew => "brew upgrade crabigator",
            InstallMethod::Unknown => "https://github.com/samuelclay/crabigator/releases",
        }
    }
}

/// Detect how Crabigator was installed
pub fn detect_install_method() -> InstallMethod {
    // Check environment variable set by npm wrapper
    if std::env::var("CRABIGATOR_INSTALLED_VIA").as_deref() == Ok("npm") {
        return InstallMethod::Npm;
    }

    // Check if binary is in a Homebrew prefix
    if let Ok(exe) = std::env::current_exe() {
        let path_str = exe.to_string_lossy();
        if path_str.contains("/homebrew/") || path_str.contains("/Cellar/") {
            return InstallMethod::Homebrew;
        }
        // Check for npm global install paths
        if path_str.contains("/node_modules/") || path_str.contains("npm") {
            return InstallMethod::Npm;
        }
        // Check for cargo install path
        if path_str.contains("/.cargo/bin/") {
            return InstallMethod::Cargo;
        }
    }

    InstallMethod::Unknown
}

/// GitHub release response (simplified)
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// Cloud API URL for update checks
const CLOUD_API_URL: &str = "https://drinkcrabigator.com/api/update-check";

/// Telemetry data sent with update checks
#[derive(Debug, Serialize)]
struct TelemetryRequest {
    device_id: String,
    machine_name: Option<String>,
    os: &'static str,
    os_version: Option<String>,
    timezone_offset: i32,
    app_version: &'static str,
    cli_version: Option<String>,
}

/// Response from cloud update check endpoint
#[derive(Debug, Deserialize)]
struct CloudUpdateResponse {
    tag_name: String,
    html_url: String,
}

/// Get current OS name for telemetry
fn get_os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    }
}

/// Get OS version string (e.g., "Darwin 25.2.0", "Linux 6.1.0", "Windows 10.0.19045")
fn get_os_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        // Use sw_vers to get macOS version
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| format!("macOS {}", s.trim()))
                } else {
                    None
                }
            })
    }

    #[cfg(target_os = "linux")]
    {
        // Try /etc/os-release first for distro info
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            let mut name = None;
            let mut version = None;
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                    return Some(val.trim_matches('"').to_string());
                }
                if let Some(val) = line.strip_prefix("NAME=") {
                    name = Some(val.trim_matches('"').to_string());
                }
                if let Some(val) = line.strip_prefix("VERSION_ID=") {
                    version = Some(val.trim_matches('"').to_string());
                }
            }
            if let (Some(n), Some(v)) = (name, version) {
                return Some(format!("{} {}", n, v));
            }
        }
        // Fallback to uname
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| format!("Linux {}", s.trim()))
                } else {
                    None
                }
            })
    }

    #[cfg(target_os = "windows")]
    {
        // Use ver command or read from registry
        std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Get local timezone offset in minutes (e.g., -480 for PST)
fn get_timezone_offset() -> i32 {
    chrono::Local::now().offset().local_minus_utc() / 60
}

/// Get CLI version by running `<command> --version`
/// Returns parsed version string (e.g., "2.1.21" for Claude, "0.91.0" for Codex)
pub fn get_cli_version(command: &str) -> Option<String> {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Check for updates via cloud endpoint (sends telemetry)
async fn check_via_cloud(
    client: &reqwest::Client,
    cli_version: Option<String>,
) -> Result<(String, String)> {
    // Load device identity (creates one if doesn't exist)
    let identity = DeviceIdentity::load_or_create()?;

    // Get machine name
    let machine_name = hostname::get().ok().and_then(|h| h.into_string().ok());

    let telemetry = TelemetryRequest {
        device_id: identity.device_id,
        machine_name,
        os: get_os_name(),
        os_version: get_os_version(),
        timezone_offset: get_timezone_offset(),
        app_version: CURRENT_VERSION,
        cli_version,
    };

    let response = client
        .post(CLOUD_API_URL)
        .json(&telemetry)
        .send()
        .await
        .context("Failed to reach cloud API")?;

    if !response.status().is_success() {
        anyhow::bail!("Cloud API returned status {}", response.status());
    }

    let cloud_response: CloudUpdateResponse = response
        .json()
        .await
        .context("Failed to parse cloud response")?;

    Ok((cloud_response.tag_name, cloud_response.html_url))
}

/// Check for updates via direct GitHub API (fallback, no telemetry)
async fn check_via_github(client: &reqwest::Client) -> Result<(String, String)> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch latest release")?;

    if !response.status().is_success() {
        anyhow::bail!("GitHub API returned status {}", response.status());
    }

    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse release response")?;

    Ok((release.tag_name, release.html_url))
}

/// Check for updates by querying cloud API (with telemetry) or falling back to GitHub
pub async fn check_for_update(cli_version: Option<String>) -> Result<UpdateCheckResult> {
    let cache = VersionCache::load();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("crabigator")
        .build()?;

    // Always try to send telemetry on startup (even if we have cached version info)
    // This ensures every session is recorded for analytics
    let cloud_result = check_via_cloud(&client, cli_version).await;

    // If cache is fresh and cloud check failed, use cached version info
    // (but we still attempted to send telemetry above)
    if !cache.is_stale() && cloud_result.is_err() {
        if let Some(latest) = cache.latest_version.as_ref() {
            let update_available = is_newer_version(latest, CURRENT_VERSION);
            let was_dismissed = cache.dismissed_version.as_ref() == Some(latest);

            return Ok(UpdateCheckResult {
                update_available,
                new_version: Some(latest.clone()),
                current_version: CURRENT_VERSION.to_string(),
                was_dismissed,
                release_url: cache.release_url.clone(),
            });
        }
    }

    // Use cloud result or fallback to GitHub
    let (tag_name, html_url) = match cloud_result {
        Ok(result) => result,
        Err(_) => {
            // Fallback to direct GitHub API if cloud fails
            check_via_github(&client).await?
        }
    };

    // Strip 'v' prefix if present (e.g., "v0.4.0" -> "0.4.0")
    let latest_version = tag_name.trim_start_matches('v').to_string();

    // Update cache
    let mut cache = cache;
    cache.update(latest_version.clone(), Some(html_url.clone()));
    let _ = cache.save(); // Best-effort save

    let update_available = is_newer_version(&latest_version, CURRENT_VERSION);
    let was_dismissed = cache.dismissed_version.as_ref() == Some(&latest_version);

    Ok(UpdateCheckResult {
        update_available,
        new_version: Some(latest_version),
        current_version: CURRENT_VERSION.to_string(),
        was_dismissed,
        release_url: Some(html_url),
    })
}

/// Dismiss a version so user won't be prompted again
pub fn dismiss_version(version: &str) -> Result<()> {
    let mut cache = VersionCache::load();
    cache.dismissed_version = Some(version.to_string());
    cache.save()
}

/// Compare two semver versions, returns true if `new` is newer than `current`
fn is_newer_version(new: &str, current: &str) -> bool {
    let parse_version = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };

    let new_v = parse_version(new);
    let current_v = parse_version(current);

    new_v > current_v
}

/// State passed to the app for banner display
#[derive(Clone, Debug, Default)]
pub struct UpdateState {
    /// Whether an update is available
    pub update_available: bool,
    /// New version string
    pub new_version: Option<String>,
    /// Whether user dismissed the modal prompt (show banner instead)
    pub prompt_dismissed: bool,
    /// Detected installation method
    pub install_method: InstallMethod,
}

impl UpdateState {
    /// Create from an update check result
    pub fn from_check(result: &UpdateCheckResult, dismissed_modal: bool) -> Self {
        Self {
            update_available: result.update_available,
            new_version: result.new_version.clone(),
            prompt_dismissed: dismissed_modal,
            install_method: detect_install_method(),
        }
    }

    /// Check if we should show the update banner
    pub fn should_show_banner(&self) -> bool {
        self.update_available && self.prompt_dismissed
    }

    /// Get the number of rows needed for the update banner
    pub fn banner_rows(&self) -> u16 {
        if self.should_show_banner() {
            1 // Single line banner
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.4.0", "0.3.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.3.1", "0.3.0"));
        assert!(!is_newer_version("0.3.0", "0.3.0"));
        assert!(!is_newer_version("0.2.0", "0.3.0"));
        assert!(is_newer_version("v0.4.0", "0.3.0"));
    }

    #[test]
    fn test_cache_staleness() {
        let mut cache = VersionCache::default();
        assert!(cache.is_stale()); // Fresh cache with 0 timestamp is stale

        cache.last_checked = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!cache.is_stale()); // Just updated, not stale
    }

    #[test]
    fn test_install_method_commands() {
        assert_eq!(
            InstallMethod::Npm.update_command(),
            "npm install -g crabigator@latest"
        );
        assert_eq!(
            InstallMethod::Cargo.update_command(),
            "cargo install --git https://github.com/samuelclay/crabigator"
        );
    }
}
