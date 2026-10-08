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

use crate::cloud::{CloudEndpoints, DeviceIdentity};
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
    /// What's new in the latest version, in one line (from its release title)
    #[serde(default)]
    pub summary: Option<String>,
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
    pub fn update(&mut self, latest: &LatestRelease) {
        self.last_checked = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.latest_version = Some(latest.version.clone());
        self.release_url = Some(latest.html_url.clone());
        self.summary = latest.summary.clone();
    }

    /// The cached version as a check result.
    fn check_result(&self) -> Option<UpdateCheckResult> {
        let latest = self.latest_version.clone()?;
        Some(UpdateCheckResult {
            update_available: is_newer_version(&latest, CURRENT_VERSION),
            was_dismissed: self.dismissed_version.as_deref() == Some(latest.as_str()),
            new_version: Some(latest),
            current_version: CURRENT_VERSION.to_string(),
            release_url: self.release_url.clone(),
            summary: self.summary.clone(),
        })
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
    /// What's new in the new version, in one line
    pub summary: Option<String>,
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

/// The latest release, as the GitHub API or the cloud's proxy of it gives it
/// (simplified). Its title is the tag and a one-line summary of what's new.
#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    name: Option<String>,
}

/// The latest published version.
#[derive(Clone, Debug)]
pub struct LatestRelease {
    /// e.g. "0.4.0"
    pub version: String,
    pub html_url: String,
    /// What's new, in one line
    pub summary: Option<String>,
}

impl From<ReleaseResponse> for LatestRelease {
    fn from(release: ReleaseResponse) -> Self {
        let summary = release
            .name
            .as_deref()
            .and_then(|name| release_summary(name, &release.tag_name));
        Self {
            version: release.tag_name.trim_start_matches('v').to_string(),
            html_url: release.html_url,
            summary,
        }
    }
}

/// The one-liner in a release title like "v0.16.1 · Show what's new": the
/// title without its tag. `None` when the title is only the tag.
fn release_summary(name: &str, tag: &str) -> Option<String> {
    let version = tag.trim_start_matches('v');
    let rest = name.trim();
    let rest = rest
        .strip_prefix(tag)
        .or_else(|| rest.strip_prefix(version))
        .unwrap_or(rest);
    let summary = rest.trim_start_matches(|c: char| c.is_whitespace() || "·:—–-|".contains(c));
    let summary = summary.trim();
    (!summary.is_empty()).then(|| summary.to_string())
}

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
        // Never let the probe touch the terminal the session is running in.
        .stdin(std::process::Stdio::null())
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
) -> Result<LatestRelease> {
    let endpoints = CloudEndpoints::load()?;
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
        .post(endpoints.endpoint("/api/update-check"))
        .json(&telemetry)
        .send()
        .await
        .context("Failed to reach cloud API")?;

    if !response.status().is_success() {
        anyhow::bail!("Cloud API returned status {}", response.status());
    }

    let release: ReleaseResponse = response
        .json()
        .await
        .context("Failed to parse cloud response")?;

    Ok(release.into())
}

/// Check for updates via direct GitHub API (fallback, no telemetry)
async fn check_via_github(client: &reqwest::Client) -> Result<LatestRelease> {
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

    let release: ReleaseResponse = response
        .json()
        .await
        .context("Failed to parse release response")?;

    Ok(release.into())
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
        if let Some(result) = cache.check_result() {
            return Ok(result);
        }
    }

    // Use cloud result or fallback to GitHub
    let latest = match cloud_result {
        Ok(result) => result,
        Err(_) => {
            // Fallback to direct GitHub API if cloud fails
            check_via_github(&client).await?
        }
    };

    // Update cache
    let mut cache = cache;
    cache.update(&latest);
    let _ = cache.save(); // Best-effort save

    Ok(cache
        .check_result()
        .expect("the cache was just given a version"))
}

/// Dismiss a version so user won't be prompted again
pub fn dismiss_version(version: &str) -> Result<()> {
    let mut cache = VersionCache::load();
    cache.dismissed_version = Some(version.to_string());
    cache.save()
}

/// Build an update-check result from the local version cache alone.
///
/// Startup uses this to decide about the update prompt without waiting on the
/// network; the live check runs in the background and refreshes the cache for
/// the next launch. Returns `None` when no version has been cached yet.
pub fn cached_update_check() -> Option<UpdateCheckResult> {
    VersionCache::load().check_result()
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
    /// What's new in the new version, in one line
    pub summary: Option<String>,
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
            summary: result.summary.clone(),
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
    fn a_release_title_gives_its_one_liner_without_the_tag() {
        let summary = |name| release_summary(name, "v0.16.1");
        assert_eq!(
            summary("v0.16.1 · Turn the flow animations off with ctrl+]").as_deref(),
            Some("Turn the flow animations off with ctrl+]")
        );
        assert_eq!(
            summary("v0.16.1: Faster startup").as_deref(),
            Some("Faster startup")
        );
        assert_eq!(
            summary("0.16.1 — Faster startup").as_deref(),
            Some("Faster startup")
        );
        // A title that is only the tag has no one-liner.
        assert_eq!(summary("v0.16.1"), None);
        assert_eq!(summary(" v0.16.1 "), None);
        // A title without the tag is all one-liner.
        assert_eq!(summary("Faster startup").as_deref(), Some("Faster startup"));
    }

    #[test]
    fn the_one_liner_comes_through_the_cloud_or_github_and_is_cached() {
        let release: ReleaseResponse = serde_json::from_str(
            r#"{"tag_name":"v99.0.0","html_url":"https://x","name":"v99.0.0 · Faster startup"}"#,
        )
        .unwrap();
        let latest = LatestRelease::from(release);
        assert_eq!(latest.version, "99.0.0");
        assert_eq!(latest.summary.as_deref(), Some("Faster startup"));
        // An older cloud proxy sends no title.
        let release: ReleaseResponse =
            serde_json::from_str(r#"{"tag_name":"v99.0.0","html_url":"https://x"}"#).unwrap();
        assert_eq!(LatestRelease::from(release).summary, None);

        let mut cache = VersionCache::default();
        cache.update(&latest);
        let result = cache.check_result().unwrap();
        assert!(result.update_available);
        assert_eq!(result.summary.as_deref(), Some("Faster startup"));
        // A cache from before summaries reads without one.
        let old: VersionCache =
            serde_json::from_str(r#"{"last_checked":1,"latest_version":"99.0.0"}"#).unwrap();
        assert_eq!(old.check_result().unwrap().summary, None);
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
