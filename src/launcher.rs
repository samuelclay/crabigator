//! URL scheme handler installer for macOS
//!
//! Creates a minimal .app bundle at ~/.crabigator/CrabigatorLauncher.app
//! that handles crabigator:// URLs for spawning new terminal sessions.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// Version of the launcher - bump when the script changes
const LAUNCHER_VERSION: &str = "1";

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.crabigator.launcher</string>
    <key>CFBundleName</key>
    <string>CrabigatorLauncher</string>
    <key>CFBundleExecutable</key>
    <string>launcher</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key>
            <string>Crabigator URL</string>
            <key>CFBundleURLSchemes</key>
            <array>
                <string>crabigator</string>
            </array>
        </dict>
    </array>
    <key>LSBackgroundOnly</key>
    <true/>
</dict>
</plist>"#;

fn launcher_script() -> String {
    // Find the crabigator binary path to embed in the script
    let binary = std::env::current_exe()
        .ok()
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "crabigator".to_string());

    format!(
        r#"#!/bin/bash
# Crabigator URL scheme handler
# Handles crabigator://spawn?cwd=/path&platform=claude

URL="$1"
if [ -z "$URL" ]; then
    exit 0
fi

# Parse query parameters from URL
parse_param() {{
    echo "$URL" | sed -n "s/.*$1=\([^&]*\).*/\1/p" | python3 -c "import sys, urllib.parse; print(urllib.parse.unquote(sys.stdin.read().strip()))" 2>/dev/null
}}

CWD=$(parse_param "cwd")
PLATFORM=$(parse_param "platform")

# Defaults
CWD="${{CWD:-$HOME}}"
PLATFORM="${{PLATFORM:-claude}}"

# Find crabigator binary
CRABIGATOR="{binary}"
if [ ! -x "$CRABIGATOR" ]; then
    CRABIGATOR=$(which crabigator 2>/dev/null || echo "crabigator")
fi

# Read terminal preference from config
TERMINAL=""
if [ -f "$HOME/.crabigator/config.toml" ]; then
    TERMINAL=$(grep '^terminal' "$HOME/.crabigator/config.toml" 2>/dev/null | sed 's/.*= *"\(.*\)"/\1/')
fi

# Auto-detect terminal if not configured
if [ -z "$TERMINAL" ]; then
    # Check for running Ghostty
    if pgrep -q "Ghostty"; then
        TERMINAL="ghostty"
    else
        TERMINAL="terminal"
    fi
fi

case "$TERMINAL" in
    ghostty)
        osascript -e "tell application \"Ghostty\" to activate
delay 0.3
tell application \"System Events\"
    tell process \"Ghostty\"
        keystroke \"n\" using command down
        delay 0.3
        keystroke \"cd '$CWD' && '$CRABIGATOR' $PLATFORM\"
        delay 0.1
        keystroke return
    end tell
end tell"
        ;;
    *)
        osascript -e "tell application \"Terminal\"
            activate
            do script \"cd '$CWD' && '$CRABIGATOR' $PLATFORM\"
        end tell"
        ;;
esac
"#
    )
}

fn crabigator_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".crabigator")
}

fn app_dir() -> PathBuf {
    crabigator_dir().join("CrabigatorLauncher.app")
}

fn version_file() -> PathBuf {
    crabigator_dir().join(".launcher-version")
}

/// Check if the launcher is installed and up to date
pub fn is_current() -> bool {
    app_dir().exists()
        && fs::read_to_string(version_file())
            .map(|v| v.trim() == LAUNCHER_VERSION)
            .unwrap_or(false)
}

/// Install the URL scheme handler on macOS
pub fn install_launcher() -> Result<()> {
    let app = app_dir();
    let contents_dir = app.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    fs::create_dir_all(&macos_dir).context("Failed to create launcher app bundle directories")?;

    // Write Info.plist
    fs::write(contents_dir.join("Info.plist"), INFO_PLIST).context("Failed to write Info.plist")?;

    // Write launcher script
    let launcher_path = macos_dir.join("launcher");
    fs::write(&launcher_path, launcher_script()).context("Failed to write launcher script")?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&launcher_path, fs::Permissions::from_mode(0o755))
            .context("Failed to make launcher executable")?;
    }

    // Register with Launch Services
    let status = std::process::Command::new(
        "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
    )
    .arg("-R")
    .arg(&app)
    .status()
    .context("Failed to run lsregister")?;

    if !status.success() {
        anyhow::bail!("lsregister failed with exit code: {:?}", status.code());
    }

    // Write version file
    fs::write(version_file(), LAUNCHER_VERSION).context("Failed to write launcher version")?;

    Ok(())
}

/// Auto-install the launcher if not present or outdated (non-blocking)
pub fn ensure_installed() {
    if is_current() {
        return;
    }
    if let Err(e) = install_launcher() {
        // Silently fail - URL scheme just won't work
        eprintln!("Warning: Failed to install URL scheme handler: {}", e);
    }
}
