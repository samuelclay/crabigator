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

use super::events::{CloudEvent, CloudToDesktopMessage};

/// WebSocket connection handle
pub struct CloudWebSocket {
    /// Sender for outgoing events
    event_tx: mpsc::Sender<CloudEvent>,
    /// Receiver for incoming answers
    answer_rx: mpsc::Receiver<String>,
    /// Receiver for incoming key commands
    key_rx: mpsc::Receiver<String>,
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
            .header("Host", ws_url.trim_start_matches("wss://").trim_start_matches("ws://").split('/').next().unwrap_or(""))
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
            shutdown_rx,
        })
    }
}

/// Non-async WebSocket handle for use in the main loop
pub struct WebSocketHandle {
    event_tx: mpsc::Sender<CloudEvent>,
    answer_rx: mpsc::Receiver<String>,
    key_rx: mpsc::Receiver<String>,
}

impl CloudWebSocket {
    /// Split into a handle for the main thread and a shutdown receiver for the runtime thread
    pub fn into_parts(self) -> (WebSocketHandle, mpsc::Receiver<()>) {
        let handle = WebSocketHandle {
            event_tx: self.event_tx,
            answer_rx: self.answer_rx,
            key_rx: self.key_rx,
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

    /// Check if the connection is still alive
    pub fn is_alive(&self) -> bool {
        !self.event_tx.is_closed()
    }
}
