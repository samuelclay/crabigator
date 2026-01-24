//! CloudClient - main interface for cloud integration
//!
//! Handles:
//! - Device registration
//! - Session registration
//! - Event streaming via WebSocket
//! - Offline queuing

use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use super::device::DeviceIdentity;
use super::events::CloudEvent;
use super::queue::OfflineQueue;
use super::websocket::{CloudWebSocket, WebSocketHandle};

/// Default API URL
const DEFAULT_API_URL: &str = "https://drinkcrabigator.com/api";


/// Response from POST /api/sessions
#[derive(Debug, Deserialize)]
struct CreateSessionResponse {
    id: String,
    ws_url: String,
}

#[derive(Debug, Serialize)]
struct UpdateSessionStats {
    prompts: u32,
    completions: u32,
    tool_calls: u32,
    thinking_seconds: u64,
}

#[derive(Debug, Serialize)]
struct UpdateSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    ended_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<UpdateSessionStats>,
}

/// Cloud connection status for display in the UI
#[derive(Clone, Debug)]
pub struct CloudStatus {
    /// Whether currently connected to cloud
    pub connected: bool,
    /// Number of reconnection attempts since last successful connection
    pub reconnect_attempts: u32,
    /// Current backoff in seconds before next retry
    pub _backoff_secs: u64,
    /// Number of queued events waiting to be sent
    pub _queue_len: usize,
}

/// Cloud client for session streaming
pub struct CloudClient {
    /// Device identity
    device: DeviceIdentity,
    /// Cloud session ID (assigned after registration)
    session_id: Option<String>,
    /// WebSocket URL for reconnection
    ws_url: Option<String>,
    /// WebSocket handle for bidirectional communication
    ws_handle: Option<WebSocketHandle>,
    /// Offline queue for when cloud is unreachable
    queue: OfflineQueue,
    /// HTTP client
    http: HttpClient,
    /// API base URL
    api_url: String,
    /// Whether device is registered with cloud
    device_registered: bool,
    /// Last reconnection attempt time
    last_reconnect_attempt: Option<std::time::Instant>,
    /// Reconnection backoff (starts at 1s, max 30s)
    reconnect_backoff_secs: u64,
    /// Number of reconnection attempts since last successful connection
    reconnect_attempts: u32,
    /// Pending reconnection attempt (receiver for async connection result)
    pending_reconnect: Option<std::sync::mpsc::Receiver<anyhow::Result<WebSocketHandle>>>,
    /// Flag set when a (re)connection just succeeded - cleared after reading
    just_connected: bool,
    /// Whether there are active viewers watching via dashboard/phone
    /// When false, desktop can reduce streaming frequency to save costs
    viewer_active: bool,
}

impl CloudClient {
    /// Create a new cloud client
    ///
    /// This loads or creates the device identity and initializes the offline queue.
    /// Call `register_device()` and `register_session()` to connect to the cloud.
    pub fn new() -> Result<Self> {
        let device = DeviceIdentity::load_or_create()?;
        let queue = OfflineQueue::new()?;
        let http = HttpClient::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            device,
            session_id: None,
            ws_url: None,
            ws_handle: None,
            queue,
            http,
            api_url: DEFAULT_API_URL.to_string(),
            device_registered: false,
            last_reconnect_attempt: None,
            reconnect_backoff_secs: 1,
            reconnect_attempts: 0,
            pending_reconnect: None,
            just_connected: false,
            viewer_active: false, // Assume no viewers initially, will be notified when one connects
        })
    }

    /// Set custom API URL (for testing)
    #[allow(dead_code)]
    pub fn with_api_url(mut self, url: &str) -> Self {
        self.api_url = url.to_string();
        self
    }

    /// Get the device ID (for future CLI commands)
    #[allow(dead_code)]
    pub fn device_id(&self) -> &str {
        &self.device.device_id
    }

    /// Get the cloud session ID (for future CLI commands)
    #[allow(dead_code)]
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Get the device identity (for pairing)
    pub fn device(&self) -> &DeviceIdentity {
        &self.device
    }

    /// Get the HTTP client (for pairing)
    pub fn http_client(&self) -> &HttpClient {
        &self.http
    }

    /// Get the API URL (for pairing)
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Check if connected to cloud
    pub fn is_connected(&self) -> bool {
        self.ws_handle.as_ref().map(|h| h.is_alive()).unwrap_or(false)
    }

    /// Check if we just (re)connected and need to send initial state.
    /// Returns true once after each successful connection, then resets.
    pub fn take_just_connected(&mut self) -> bool {
        let was_connected = self.just_connected;
        self.just_connected = false;
        was_connected
    }

    /// Get current cloud connection status for UI display
    pub fn status(&self) -> CloudStatus {
        CloudStatus {
            connected: self.is_connected(),
            reconnect_attempts: self.reconnect_attempts,
            _backoff_secs: self.reconnect_backoff_secs,
            _queue_len: self.queue.len(),
        }
    }

    /// Register device with cloud (idempotent)
    pub async fn register_device(&mut self) -> Result<()> {
        if self.device_registered {
            return Ok(());
        }

        #[derive(Serialize)]
        struct RegisterRequest {
            device_id: String,
            secret_hash: String,
            name: Option<String>,
        }

        let request = RegisterRequest {
            device_id: self.device.device_id.clone(),
            secret_hash: self.device.secret_hash(),
            name: self.device.name.clone(),
        };

        let url = format!("{}/devices", self.api_url);
        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .with_context(|| "Failed to register device with cloud")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Device registration failed: {} - {}", status, body);
        }

        // Success - we don't need to parse the response body
        self.device_registered = true;

        Ok(())
    }

    /// Register a session with the cloud
    pub async fn register_session(
        &mut self,
        client_session_id: &str,
        cwd: &str,
        platform: &str,
    ) -> Result<String> {
        // Ensure device is registered first
        self.register_device().await?;

        #[derive(Serialize)]
        struct CreateSessionRequest {
            client_session_id: String,
            cwd: String,
            platform: String,
        }

        let request = CreateSessionRequest {
            client_session_id: client_session_id.to_string(),
            cwd: cwd.to_string(),
            platform: platform.to_string(),
        };

        let url = format!("{}/sessions", self.api_url);
        let headers = self.device.auth_headers("POST", "/api/sessions")?;

        let mut req = self.http.post(&url).json(&request);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req
            .send()
            .await
            .with_context(|| "Failed to register session with cloud")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Session registration failed: {} - {}", status, body);
        }

        let data: CreateSessionResponse = response.json().await?;
        self.session_id = Some(data.id.clone());

        // Connect WebSocket
        self.connect_websocket(&data.ws_url).await?;

        // Drain any queued events
        self.drain_queue();

        Ok(data.id)
    }

    /// Update session state asynchronously (non-blocking)
    pub fn spawn_update_state(&self, state: &str) {
        self.spawn_session_update(UpdateSessionRequest {
            ended_at: None,
            state: Some(state.to_string()),
            stats: None,
        });
    }

    /// Update session stats asynchronously (non-blocking)
    pub fn spawn_update_stats(
        &self,
        prompts: u32,
        completions: u32,
        tool_calls: u32,
        thinking_seconds: u64,
    ) {
        self.spawn_session_update(UpdateSessionRequest {
            ended_at: None,
            state: None,
            stats: Some(UpdateSessionStats {
                prompts,
                completions,
                tool_calls,
                thinking_seconds,
            }),
        });
    }

    /// Connect WebSocket for bidirectional communication
    async fn connect_websocket(&mut self, ws_url: &str) -> Result<()> {
        // Store URL for reconnection
        self.ws_url = Some(ws_url.to_string());

        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
        let message = format!("GET:/api/sessions/{}/connect:{}", self.session_id.as_ref().unwrap(), timestamp);
        let signature = self.device.sign(&message)?;

        let ws = CloudWebSocket::connect(
            ws_url,
            &self.device.device_id,
            &signature,
            &timestamp,
        )
        .await?;

        // Split into handle and shutdown receiver
        // For initial connection, we're in the main runtime so tasks stay alive
        let (handle, _shutdown_rx) = ws.into_parts();
        self.ws_handle = Some(handle);
        // Reset backoff and attempts on successful connection
        self.reconnect_backoff_secs = 1;
        self.reconnect_attempts = 0;
        self.last_reconnect_attempt = None;
        // Mark that we just connected (for initial sync)
        self.just_connected = true;
        Ok(())
    }

    /// Try to reconnect WebSocket if disconnected
    ///
    /// Returns true if connected (already or after reconnect), false if reconnection is pending or failed.
    /// This function is non-blocking - it starts connection attempts asynchronously and checks
    /// for completion on subsequent calls.
    pub fn try_reconnect(&mut self) -> bool {
        // Already connected?
        if self.is_connected() {
            self.pending_reconnect = None;
            return true;
        }

        // Check if there's a pending reconnection attempt
        if let Some(ref rx) = self.pending_reconnect {
            match rx.try_recv() {
                Ok(Ok(handle)) => {
                    // Connection succeeded!
                    self.ws_handle = Some(handle);
                    self.reconnect_backoff_secs = 1;
                    self.reconnect_attempts = 0;
                    self.pending_reconnect = None;
                    self.just_connected = true;
                    self.drain_queue();
                    return true;
                }
                Ok(Err(_)) => {
                    // Connection failed - increase backoff and clear pending
                    self.reconnect_backoff_secs = (self.reconnect_backoff_secs * 2).min(30);
                    self.last_reconnect_attempt = Some(std::time::Instant::now());
                    self.pending_reconnect = None;
                    return false; // Wait for backoff before retrying
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still connecting - don't start another attempt
                    return false;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Thread died unexpectedly - clear and retry
                    self.pending_reconnect = None;
                }
            }
        }

        // No URL to reconnect to?
        let ws_url = match &self.ws_url {
            Some(url) => url.clone(),
            None => return false,
        };

        // Check backoff
        if let Some(last_attempt) = self.last_reconnect_attempt {
            if last_attempt.elapsed().as_secs() < self.reconnect_backoff_secs {
                return false; // Still in backoff period
            }
        }

        // Start new reconnection attempt
        self.last_reconnect_attempt = Some(std::time::Instant::now());
        self.reconnect_attempts += 1;

        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
        let session_id = match &self.session_id {
            Some(id) => id.clone(),
            None => return false,
        };
        let message = format!("GET:/api/sessions/{}/connect:{}", session_id, timestamp);
        let signature = match self.device.sign(&message) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        // Spawn async reconnection task
        // IMPORTANT: The runtime must stay alive as long as the WebSocket tasks need to run.
        // We split the WebSocket into a handle (sent to main thread) and shutdown receiver
        // (kept in this thread to block until connection closes).
        let device_id = self.device.device_id.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let result = rt.block_on(async {
                CloudWebSocket::connect(&ws_url, &device_id, &signature, &timestamp).await
            });

            match result {
                Ok(ws) => {
                    // Split into handle (for main thread) and shutdown receiver (for us)
                    let (handle, mut shutdown_rx) = ws.into_parts();
                    let _ = tx.send(Ok(handle));

                    // Keep runtime alive until connection closes
                    // The read task will signal shutdown when the WebSocket disconnects
                    rt.block_on(async {
                        let _ = shutdown_rx.recv().await;
                    });
                }
                Err(e) => {
                    // Connection failed - just send the error
                    let _ = tx.send(Err(e));
                }
            }
        });

        // Store the receiver to check on next call
        self.pending_reconnect = Some(rx);
        false // Connection in progress, not yet connected
    }

    /// Send an event to the cloud
    ///
    /// If not connected, attempts to reconnect. Events are queued if offline.
    pub fn send_event(&mut self, event: CloudEvent) {
        // Try to send if connected
        if let Some(ref handle) = self.ws_handle {
            if handle.is_alive() {
                if !handle.try_send(event.clone()) {
                    // Channel full, queue it
                    self.queue.enqueue(event);
                }
                return;
            }
        }

        // Not connected - try to reconnect
        if self.try_reconnect() {
            // Reconnected! Try to send
            if let Some(ref handle) = self.ws_handle {
                if handle.try_send(event.clone()) {
                    return;
                }
            }
        }

        // Still not connected, queue it
        self.queue.enqueue(event);
    }

    /// Try to receive an answer from mobile (non-blocking)
    pub fn try_recv_answer(&mut self) -> Option<String> {
        self.ws_handle.as_mut()?.try_recv_answer()
    }

    /// Try to receive a key command from cloud (non-blocking)
    pub fn try_recv_key(&mut self) -> Option<String> {
        self.ws_handle.as_mut()?.try_recv_key()
    }

    /// Poll for viewer status changes and update internal state
    /// Returns true if there are active viewers watching
    pub fn poll_viewer_status(&mut self) -> bool {
        // Check for any status updates from the cloud
        if let Some(handle) = self.ws_handle.as_mut() {
            while let Some(active) = handle.try_recv_viewer_status() {
                self.viewer_active = active;
            }
        }
        self.viewer_active
    }

    /// Drain queued events after reconnection
    fn drain_queue(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        let events = self.queue.drain();
        for queued in events {
            self.send_event(queued.event);
        }
    }

    /// Send a session update asynchronously
    fn spawn_session_update(&self, update: UpdateSessionRequest) {
        let Some(session_id) = self.session_id.clone() else {
            return;
        };
        let device = self.device.clone();
        let http = self.http.clone();
        let api_url = self.api_url.clone();

        tokio::spawn(async move {
            let _ = Self::send_session_update_with(device, http, api_url, session_id, update).await;
        });
    }

    /// Update session state in the cloud (blocking, for CLI commands)
    #[allow(dead_code)]
    pub async fn update_session_state(&self, state: &str) -> Result<()> {
        self.send_session_update(UpdateSessionRequest {
            ended_at: None,
            state: Some(state.to_string()),
            stats: None,
        })
        .await
    }

    /// Update session stats in the cloud (blocking, for CLI commands)
    #[allow(dead_code)]
    pub async fn update_session_stats(
        &self,
        prompts: u32,
        completions: u32,
        tool_calls: u32,
        thinking_seconds: u64,
    ) -> Result<()> {
        self.send_session_update(UpdateSessionRequest {
            ended_at: None,
            state: None,
            stats: Some(UpdateSessionStats {
                prompts,
                completions,
                tool_calls,
                thinking_seconds,
            }),
        })
        .await
    }

    /// Mark session as ended
    pub async fn end_session(
        &self,
        prompts: u32,
        completions: u32,
        tool_calls: u32,
        thinking_seconds: u64,
    ) -> Result<()> {
        self.send_session_update(UpdateSessionRequest {
            ended_at: Some(chrono::Utc::now().timestamp() as u64),
            state: None,
            stats: Some(UpdateSessionStats {
                prompts,
                completions,
                tool_calls,
                thinking_seconds,
            }),
        })
        .await
    }

    async fn send_session_update(&self, update: UpdateSessionRequest) -> Result<()> {
        let session_id = self
            .session_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No session registered"))?;
        Self::send_session_update_with(
            self.device.clone(),
            self.http.clone(),
            self.api_url.clone(),
            session_id.to_string(),
            update,
        )
        .await
    }

    async fn send_session_update_with(
        device: DeviceIdentity,
        http: HttpClient,
        api_url: String,
        session_id: String,
        update: UpdateSessionRequest,
    ) -> Result<()> {
        let url = format!("{}/sessions/{}", api_url, session_id);
        let headers = device.auth_headers("PATCH", &format!("/api/sessions/{}", session_id))?;

        let mut req = http.patch(&url).json(&update);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req.send().await?;
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update session: {}", body);
        }

        Ok(())
    }

    // ========================================
    // Pairing API Methods
    // ========================================

    /// Generate a pairing token for mobile device linking
    pub async fn generate_pairing_token(&self) -> Result<PairingTokenResponse> {
        let url = format!("{}/pairing/generate", self.api_url);
        let headers = self.device.auth_headers("POST", "/api/pairing/generate")?;

        let mut req = self.http.post(&url);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req.send().await
            .with_context(|| "Failed to generate pairing token")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to generate pairing token: {} - {}", status, body);
        }

        let data: PairingTokenResponse = response.json().await?;
        Ok(data)
    }

    /// Poll pairing status to check if mobile device has claimed the token
    #[allow(dead_code)]
    pub async fn poll_pairing_status(&self, token: &str) -> Result<PairingStatusResponse> {
        let url = format!("{}/pairing/{}/status", self.api_url, token);
        let headers = self.device.auth_headers("GET", &format!("/api/pairing/{}/status", token))?;

        let mut req = self.http.get(&url);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req.send().await
            .with_context(|| "Failed to poll pairing status")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to poll pairing status: {} - {}", status, body);
        }

        let data: PairingStatusResponse = response.json().await?;
        Ok(data)
    }

    /// Get list of linked mobile devices
    pub async fn get_linked_devices(&self) -> Result<LinkedDevicesResponse> {
        let url = format!("{}/devices/linked", self.api_url);
        let headers = self.device.auth_headers("GET", "/api/devices/linked")?;

        let mut req = self.http.get(&url);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req.send().await
            .with_context(|| "Failed to get linked devices")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get linked devices: {} - {}", status, body);
        }

        let data: LinkedDevicesResponse = response.json().await?;
        Ok(data)
    }

    /// Revoke a linked mobile device
    #[allow(dead_code)]
    pub async fn revoke_linked_device(&self, mobile_id: &str) -> Result<()> {
        let url = format!("{}/devices/linked/{}", self.api_url, mobile_id);
        let headers = self.device.auth_headers("DELETE", &format!("/api/devices/linked/{}", mobile_id))?;

        let mut req = self.http.delete(&url);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req.send().await
            .with_context(|| "Failed to revoke linked device")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to revoke linked device: {} - {}", status, body);
        }

        Ok(())
    }
}

// ========================================
// Pairing Response Types
// ========================================

/// Response from POST /api/pairing/generate
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PairingTokenResponse {
    pub token: String,
    pub expires_at: u64,
    pub qr_data: String,
    pub code: String,
}

/// Response from GET /api/pairing/{token}/status
#[derive(Debug, Deserialize)]
pub struct PairingStatusResponse {
    /// Whether pairing is complete
    pub paired: bool,
    /// Name of the mobile device (if paired)
    pub mobile_name: Option<String>,
    /// Whether the token has expired
    pub expired: bool,
}

/// Response from GET /api/devices/linked
#[derive(Debug, Deserialize)]
pub struct LinkedDevicesResponse {
    pub devices: Vec<LinkedDevice>,
}

/// A linked mobile device
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LinkedDevice {
    pub mobile_id: String,
    pub mobile_name: Option<String>,
    pub paired_at: u64,
}
