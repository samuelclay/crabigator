//! Cloud origin validation and endpoint construction.

use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::Config;

pub const OFFICIAL_CLOUD_ORIGIN: &str = "https://drinkcrabigator.com";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudEndpoints {
    origin: String,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    service: Option<String>,
    api_version: Option<String>,
    status: Option<String>,
}

impl Default for CloudEndpoints {
    fn default() -> Self {
        Self {
            origin: OFFICIAL_CLOUD_ORIGIN.to_string(),
        }
    }
}

impl CloudEndpoints {
    pub fn load() -> Result<Self> {
        Self::from_config(&Config::load()?)
    }

    pub fn from_config(config: &Config) -> Result<Self> {
        match config.cloud.url.as_deref() {
            Some(origin) => Self::parse(origin),
            None => Ok(Self::default()),
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        let mut url = Url::parse(input.trim()).context("Cloud URL must be an absolute URL")?;
        if url.host_str().is_none() {
            bail!("Cloud URL must include a host");
        }
        if !url.username().is_empty() || url.password().is_some() {
            bail!("Cloud URL cannot include a username or password");
        }
        if url.query().is_some() || url.fragment().is_some() {
            bail!("Cloud URL cannot include a query or fragment");
        }
        if url.path() != "/" && !url.path().is_empty() {
            bail!("Cloud URL must be an origin without a path");
        }

        let loopback = url.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .parse::<IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        });
        match url.scheme() {
            "https" => {}
            "http" if loopback => {}
            _ => bail!("Cloud URL must use HTTPS (HTTP is allowed only for loopback hosts)"),
        }

        url.set_path("");
        Ok(Self {
            origin: url.as_str().trim_end_matches('/').to_string(),
        })
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn api_url(&self) -> String {
        format!("{}/api", self.origin)
    }

    pub fn dashboard_url(&self) -> String {
        format!("{}/dashboard", self.origin)
    }

    pub fn dashboard_setup_url(&self, code: &str) -> String {
        format!("{}/dashboard?setup={}", self.origin, code)
    }

    pub fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.origin, path)
    }

    pub fn state_dir(&self) -> PathBuf {
        self.state_dir_under(&Config::config_dir())
    }

    fn state_dir_under(&self, config_dir: &Path) -> PathBuf {
        if self.origin == OFFICIAL_CLOUD_ORIGIN {
            return config_dir.to_path_buf();
        }
        let digest = Sha256::digest(self.origin.as_bytes());
        config_dir.join("cloud").join(hex::encode(&digest[..8]))
    }

    pub async fn verify(&self) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()?;
        let response = client
            .get(self.endpoint("/api/health"))
            .send()
            .await
            .context("Could not reach the Crabigator health endpoint")?;
        if !response.status().is_success() {
            bail!("Health endpoint returned {}", response.status());
        }
        let health: HealthResponse = response
            .json()
            .await
            .context("Health endpoint did not return Crabigator JSON")?;
        if health.service.as_deref() != Some("crabigator-api")
            || health.api_version.as_deref() != Some("v1")
        {
            bail!("Health endpoint is not a compatible Crabigator v1 service");
        }
        Ok(health.status.unwrap_or_else(|| "unknown".to_string()))
    }
}

pub async fn set_cloud(origin: &str, force: bool) -> Result<()> {
    let endpoints = CloudEndpoints::parse(origin)?;
    if !force {
        let status = endpoints.verify().await?;
        if status != "ok" && status != "degraded" {
            bail!("Cloud service reported status {status}");
        }
    }

    let mut config = Config::load()?;
    config.cloud.url =
        (endpoints.origin() != OFFICIAL_CLOUD_ORIGIN).then(|| endpoints.origin().to_string());
    config.save()?;

    println!("Cloud: {}", endpoints.origin());
    if force {
        println!("Compatibility check skipped.");
    } else {
        println!("Compatible Crabigator v1 service found.");
    }
    Ok(())
}

pub fn print_cloud_status() -> Result<()> {
    let endpoints = CloudEndpoints::load()?;
    println!("Cloud: {}", endpoints.origin());
    println!("API: {}", endpoints.api_url());
    println!("Dashboard: {}", endpoints.dashboard_url());
    println!("Local state: {}", endpoints.state_dir().display());
    Ok(())
}

pub fn reset_cloud() -> Result<()> {
    let mut config = Config::load()?;
    config.cloud.url = None;
    config.save()?;
    println!("Cloud reset to {}.", OFFICIAL_CLOUD_ORIGIN);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_origins_and_removes_the_trailing_slash() {
        let endpoints = CloudEndpoints::parse("https://example.com/").unwrap();
        assert_eq!(endpoints.origin(), "https://example.com");
        assert_eq!(endpoints.api_url(), "https://example.com/api");
    }

    #[test]
    fn accepts_loopback_http_only() {
        assert!(CloudEndpoints::parse("http://localhost:8787").is_ok());
        assert!(CloudEndpoints::parse("http://127.0.0.1:8787").is_ok());
        assert!(CloudEndpoints::parse("http://example.com").is_err());
    }

    #[test]
    fn rejects_paths_credentials_queries_and_fragments() {
        assert!(CloudEndpoints::parse("https://example.com/api").is_err());
        assert!(CloudEndpoints::parse("https://user@example.com").is_err());
        assert!(CloudEndpoints::parse("https://example.com?x=1").is_err());
        assert!(CloudEndpoints::parse("https://example.com/#x").is_err());
    }

    #[test]
    fn scopes_custom_state_but_keeps_official_paths_compatible() {
        let base = Path::new("/tmp/config");
        assert_eq!(CloudEndpoints::default().state_dir_under(base), base);

        let custom = CloudEndpoints::parse("https://example.com").unwrap();
        let state = custom.state_dir_under(base);
        assert!(state.starts_with(base.join("cloud")));
        assert_ne!(state, base);
    }
}
