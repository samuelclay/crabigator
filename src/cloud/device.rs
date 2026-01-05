//! Device identity management for cloud authentication
//!
//! Each desktop running crabigator gets a unique device identity stored in
//! ~/.crabigator/device.json. This identity is used to authenticate with
//! the cloud API using HMAC-SHA256 signatures.

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Device identity stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    /// Unique device identifier (UUID v4)
    pub device_id: String,
    /// Secret for signing requests (base64-encoded 32 bytes)
    pub device_secret: String,
    /// Optional device name for display
    #[serde(default)]
    pub name: Option<String>,
}

impl DeviceIdentity {
    /// Load device identity from ~/.crabigator/device.json or create new
    pub fn load_or_create() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read device identity from {:?}", path))?;
            let identity: DeviceIdentity = serde_json::from_str(&content)
                .with_context(|| "Failed to parse device identity JSON")?;
            Ok(identity)
        } else {
            let identity = Self::generate()?;
            identity.save()?;
            Ok(identity)
        }
    }

    /// Generate a new device identity
    fn generate() -> Result<Self> {
        use rand::RngCore;

        let device_id = Uuid::new_v4().to_string();

        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let device_secret = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            secret_bytes,
        );

        // Get hostname as default device name
        let name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok());

        Ok(Self {
            device_id,
            device_secret,
            name,
        })
    }

    /// Save device identity to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write device identity to {:?}", path))?;

        Ok(())
    }

    /// Get the config file path
    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".crabigator").join("device.json"))
    }

    /// Compute SHA-256 hash of the device secret (for registration)
    pub fn secret_hash(&self) -> String {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(self.device_secret.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Sign a message with the secret hash (for API requests)
    ///
    /// The server stores secret_hash, so both client and server use
    /// secret_hash as the HMAC key for consistency.
    pub fn sign(&self, message: &str) -> Result<String> {
        let key = self.secret_hash();
        let mut mac = HmacSha256::new_from_slice(key.as_bytes())
            .map_err(|e| anyhow::anyhow!("Invalid HMAC key: {}", e))?;
        mac.update(message.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    /// Create authentication headers for an API request
    pub fn auth_headers(&self, method: &str, path: &str) -> Result<Vec<(String, String)>> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let message = format!("{}:{}:{}", method, path, timestamp);
        let signature = self.sign(&message)?;

        Ok(vec![
            ("X-Device-Id".to_string(), self.device_id.clone()),
            ("X-Timestamp".to_string(), timestamp.to_string()),
            ("X-Signature".to_string(), signature),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let identity = DeviceIdentity::generate().unwrap();
        assert!(!identity.device_id.is_empty());
        assert!(!identity.device_secret.is_empty());
        // Verify UUID format
        assert!(Uuid::parse_str(&identity.device_id).is_ok());
    }

    #[test]
    fn test_secret_hash() {
        let identity = DeviceIdentity::generate().unwrap();
        let hash = identity.secret_hash();
        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_sign() {
        let identity = DeviceIdentity::generate().unwrap();
        let signature = identity.sign("test message").unwrap();
        // HMAC-SHA256 produces 64 hex characters
        assert_eq!(signature.len(), 64);
    }
}
