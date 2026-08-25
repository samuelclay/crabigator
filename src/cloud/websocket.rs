//! WebSocket connection to the cloud for bidirectional communication
//!
//! - Sends events from desktop to cloud
//! - Receives answers from mobile devices

use anyhow::Result;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{http::Request, Message},
};

use super::events::{CloudEvent, CloudToDesktopMessage, KeyStep};

/// Spawn request received from cloud
pub struct SpawnRequest {
    pub cwd: String,
    pub platform: Option<String>,
}

/// WebSocket connection handle
pub struct CloudWebSocket {
    /// Sender for outgoing events
    event_tx: mpsc::Sender<CloudEvent>,
    /// Receiver for incoming answers
    answer_rx: mpsc::Receiver<String>,
    /// Receiver for incoming key commands
    key_rx: mpsc::Receiver<String>,
    /// Receiver for incoming key sequences (multi-step)
    key_sequence_rx: mpsc::Receiver<Vec<KeyStep>>,
    /// Receiver for viewer status changes
    viewer_status_rx: mpsc::Receiver<bool>,
    /// Receiver for spawn requests
    spawn_rx: mpsc::Receiver<SpawnRequest>,
    /// Receiver for "PR dispositions changed" nudges
    pr_overrides_changed_rx: mpsc::Receiver<()>,
    /// Receiver that completes when the connection closes
    shutdown_rx: mpsc::Receiver<()>,
}

impl CloudWebSocket {
    /// Connect to the cloud WebSocket
    pub async fn connect(
        ws_url: &str,
        device_id: &str,
        signature: &str,
        timestamp: &str,
    ) -> Result<Self> {
        // Generate WebSocket key
        let ws_key = base64::engine::general_purpose::STANDARD.encode(rand::random::<[u8; 16]>());

        let request = Request::builder()
            .uri(ws_url)
            .header(
                "Host",
                ws_url
                    .trim_start_matches("wss://")
                    .trim_start_matches("ws://")
                    .split('/')
                    .next()
                    .unwrap_or(""),
            )
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", &ws_key)
            .header("X-Device-Id", device_id)
            .header("X-Signature", signature)
            .header("X-Timestamp", timestamp)
            .body(())
            .map_err(|e| anyhow::anyhow!("Failed to build WebSocket request: {}", e))?;

        let (ws_stream, _response) = connect_async(request)
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket connection error: {:?}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // Channel for outgoing events (desktop -> cloud)
        let (event_tx, mut event_rx) = mpsc::channel::<CloudEvent>(100);

        // Channel for incoming answers (cloud -> desktop)
        let (answer_tx, answer_rx) = mpsc::channel::<String>(16);

        // Channel for incoming key commands (cloud -> desktop)
        let (key_tx, key_rx) = mpsc::channel::<String>(16);

        // Channel for incoming key sequences (cloud -> desktop)
        let (key_sequence_tx, key_sequence_rx) = mpsc::channel::<Vec<KeyStep>>(16);

        // Channel for viewer status changes (cloud -> desktop)
        let (viewer_status_tx, viewer_status_rx) = mpsc::channel::<bool>(4);

        // Channel for spawn requests (cloud -> desktop)
        let (spawn_tx, spawn_rx) = mpsc::channel::<SpawnRequest>(4);

        // Channel for PR disposition change nudges (cloud -> desktop)
        let (pr_overrides_changed_tx, pr_overrides_changed_rx) = mpsc::channel::<()>(4);

        // Channel to signal when connection closes (read task will signal this)
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        // Spawn task to handle outgoing events
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let json = match serde_json::to_string(&event) {
                    Ok(j) => j,
                    Err(_) => continue,
                };

                if write.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to handle incoming messages
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                let msg = match msg_result {
                    Ok(m) => m,
                    Err(_) => break,
                };

                if let Message::Text(text) = msg {
                    match serde_json::from_str::<CloudToDesktopMessage>(&text) {
                        Ok(CloudToDesktopMessage::Answer { text }) => {
                            let _ = answer_tx.send(text).await;
                        }
                        Ok(CloudToDesktopMessage::Key { key }) => {
                            let _ = key_tx.send(key).await;
                        }
                        Ok(CloudToDesktopMessage::KeySequence { steps }) => {
                            let _ = key_sequence_tx.send(steps).await;
                        }
                        Ok(CloudToDesktopMessage::ViewerStatus { active }) => {
                            let _ = viewer_status_tx.send(active).await;
                        }
                        Ok(CloudToDesktopMessage::Spawn { cwd, platform }) => {
                            let _ = spawn_tx.send(SpawnRequest { cwd, platform }).await;
                        }
                        Ok(CloudToDesktopMessage::PrOverridesChanged) => {
                            // A full channel already holds a pending nudge.
                            let _ = pr_overrides_changed_tx.try_send(());
                        }
                        Ok(CloudToDesktopMessage::Ping) | Err(_) => {}
                    }
                }
            }
            // Signal that connection has closed
            let _ = shutdown_tx.send(()).await;
        });

        Ok(Self {
            event_tx,
            answer_rx,
            key_rx,
            key_sequence_rx,
            viewer_status_rx,
            spawn_rx,
            pr_overrides_changed_rx,
            shutdown_rx,
        })
    }
}

/// Non-async WebSocket handle for use in the main loop
pub struct WebSocketHandle {
    event_tx: mpsc::Sender<CloudEvent>,
    answer_rx: mpsc::Receiver<String>,
    key_rx: mpsc::Receiver<String>,
    key_sequence_rx: mpsc::Receiver<Vec<KeyStep>>,
    viewer_status_rx: mpsc::Receiver<bool>,
    spawn_rx: mpsc::Receiver<SpawnRequest>,
    pr_overrides_changed_rx: mpsc::Receiver<()>,
}

impl CloudWebSocket {
    /// Split into a handle for the main thread and a shutdown receiver for the runtime thread
    pub fn into_parts(self) -> (WebSocketHandle, mpsc::Receiver<()>) {
        let handle = WebSocketHandle {
            event_tx: self.event_tx,
            answer_rx: self.answer_rx,
            key_rx: self.key_rx,
            key_sequence_rx: self.key_sequence_rx,
            viewer_status_rx: self.viewer_status_rx,
            spawn_rx: self.spawn_rx,
            pr_overrides_changed_rx: self.pr_overrides_changed_rx,
        };
        (handle, self.shutdown_rx)
    }
}

impl WebSocketHandle {
    /// Try to send an event (non-blocking)
    pub fn try_send(&self, event: CloudEvent) -> bool {
        self.event_tx.try_send(event).is_ok()
    }

    /// Try to receive an answer (non-blocking)
    pub fn try_recv_answer(&mut self) -> Option<String> {
        self.answer_rx.try_recv().ok()
    }

    /// Try to receive a key command (non-blocking)
    pub fn try_recv_key(&mut self) -> Option<String> {
        self.key_rx.try_recv().ok()
    }

    /// Whether the cloud nudged us that PR dispositions changed since the
    /// last check (non-blocking; drains every queued nudge).
    pub fn take_pr_overrides_changed(&mut self) -> bool {
        let mut changed = false;
        while self.pr_overrides_changed_rx.try_recv().is_ok() {
            changed = true;
        }
        changed
    }

    /// Try to receive a key sequence (non-blocking)
    pub fn try_recv_key_sequence(&mut self) -> Option<Vec<KeyStep>> {
        self.key_sequence_rx.try_recv().ok()
    }

    /// Try to receive a viewer status change (non-blocking)
    pub fn try_recv_viewer_status(&mut self) -> Option<bool> {
        self.viewer_status_rx.try_recv().ok()
    }

    /// Try to receive a spawn request (non-blocking)
    pub fn try_recv_spawn(&mut self) -> Option<SpawnRequest> {
        self.spawn_rx.try_recv().ok()
    }

    /// Check if the connection is still alive
    pub fn is_alive(&self) -> bool {
        !self.event_tx.is_closed()
    }
}
