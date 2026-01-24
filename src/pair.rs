//! Pairing code generation for Chrome MCP auto-login
//!
//! This module provides a simple command that outputs a pairing code
//! for use with the dashboard's `?setup=<code>` URL parameter.
//!
//! Pairing codes are cached in `~/.crabigator/.pairing-cache` to avoid
//! generating new codes on every call. Cached codes are reused until
//! they expire (with a 30-second buffer).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cloud::CloudClient;

/// Cached pairing code
#[derive(Serialize, Deserialize)]
struct PairingCache {
    code: String,
    expires_at: u64,
}

/// Get the cache file path (~/.crabigator/.pairing-cache)
fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".crabigator").join(".pairing-cache"))
}

/// Get current unix timestamp
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Try to load a valid cached pairing code
fn load_cached_code() -> Option<String> {
    let path = cache_path()?;
    let content = fs::read_to_string(&path).ok()?;
    let cache: PairingCache = serde_json::from_str(&content).ok()?;

    // Check if still valid (with 30-second buffer for network latency)
    if cache.expires_at > now_secs() + 30 {
        Some(cache.code)
    } else {
        // Expired - remove stale cache
        let _ = fs::remove_file(&path);
        None
    }
}

/// Save a pairing code to cache
fn save_cached_code(code: &str, expires_at: u64) {
    let Some(path) = cache_path() else { return };

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let cache = PairingCache {
        code: code.to_string(),
        expires_at,
    };

    if let Ok(content) = serde_json::to_string(&cache) {
        let _ = fs::write(&path, content);
    }
}

/// Generate and print a pairing code
///
/// This is designed for Chrome MCP auto-login:
/// 1. Run `crabigator pair` to get a code
/// 2. Navigate to `https://drinkcrabigator.com/dashboard?setup=<code>`
/// 3. Dashboard authenticates automatically
///
/// Codes are cached and reused until expiry or claimed by a browser.
pub async fn run_pair() -> Result<()> {
    let mut client = CloudClient::new()?;
    client.register_device().await?;

    // Check for valid cached code first
    if let Some(code) = load_cached_code() {
        // Verify the cached code hasn't been claimed yet
        match client.poll_pairing_status(&code).await {
            Ok(status) if !status.paired && !status.expired => {
                // Code is still valid and unclaimed
                println!("{}", code);
                return Ok(());
            }
            _ => {
                // Code was claimed or expired - clear cache and generate new
                if let Some(path) = cache_path() {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    // Generate new code
    let pairing = client.generate_pairing_token().await?;

    // Cache it for future calls
    save_cached_code(&pairing.code, pairing.expires_at);

    // Output just the code - easy to capture programmatically
    println!("{}", pairing.code);

    Ok(())
}
