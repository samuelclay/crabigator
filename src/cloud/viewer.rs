//! Watch one session's cloud screen and type into it.
//!
//! The owning desktop already streams its screen to the account and already
//! writes key-sequence text into the PTY. This is the viewer side the PR
//! board uses for a session running on another computer: the events
//! websocket, the viewer heartbeat that keeps that stream live, and ordered
//! key posts. Dropping the watch stops the heartbeat.

use std::time::Duration;

use anyhow::{Context, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as HttpClient;
use serde_json::Value;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{http::Request, Message},
};

use super::device::DeviceIdentity;
use super::endpoints::CloudEndpoints;

const HEARTBEAT: Duration = Duration::from_secs(10);
const RECONNECT: Duration = Duration::from_secs(1);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
/// The server keeps about this much scrollback. Trim to the same bound so a
/// long session cannot grow without limit on the viewing computer.
const SCROLLBACK_CAP: usize = 500 * 1024;

/// One update from the session's viewer stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerUpdate {
    Screen(String),
    /// Replace the transcript. Sent once when the viewer connects.
    ScrollbackReset(String),
    /// Append to the transcript.
    ScrollbackDiff(String),
    DesktopOffline,
    DesktopOnline,
    Failed(String),
}

/// A live view of one cloud session. Drop it to stop the heartbeat and the
/// key posts.
pub struct SessionWatch {
    session_id: String,
    updates: mpsc::Receiver<ViewerUpdate>,
    keys: mpsc::UnboundedSender<Vec<u8>>,
    cancel: watch::Sender<bool>,
}

impl SessionWatch {
    /// Start watching `session_id`. The first screen can take a moment: the
    /// owning desktop only streams quickly after it sees the heartbeat.
    pub fn start(session_id: String) -> Self {
        let (update_tx, updates) = mpsc::channel(32);
        let (keys, key_rx) = mpsc::unbounded_channel();
        let (cancel, cancel_rx) = watch::channel(false);
        let id = session_id.clone();
        tokio::spawn(async move {
            run_watch(id, update_tx, key_rx, cancel_rx).await;
        });
        Self {
            session_id,
            updates,
            keys,
            cancel,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn try_recv(&mut self) -> Option<ViewerUpdate> {
        self.updates.try_recv().ok()
    }

    /// Queue terminal bytes for the owning desktop. Bytes that arrive while
    /// a post is in flight stay in order and go out as one step.
    pub fn send_keys(&self, bytes: Vec<u8>) -> bool {
        self.keys.send(bytes).is_ok()
    }
}

impl Drop for SessionWatch {
    fn drop(&mut self) {
        let _ = self.cancel.send(true);
    }
}

/// The viewer events this board paints. Anything else on the socket is ignored.
pub fn apply_viewer_message(value: &Value) -> Option<ViewerUpdate> {
    let kind = value.get("type").and_then(Value::as_str)?;
    match kind {
        "screen" => text_field(value, "content").map(ViewerUpdate::Screen),
        "scrollback_history" => text_field(value, "content").map(ViewerUpdate::ScrollbackReset),
        "scrollback" => text_field(value, "diff").map(ViewerUpdate::ScrollbackDiff),
        "desktop_status" => {
            let connected = value
                .get("connected")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some(if connected {
                ViewerUpdate::DesktopOnline
            } else {
                ViewerUpdate::DesktopOffline
            })
        }
        _ => None,
    }
}

fn text_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}

/// Keep the tail of a scrollback that has grown past the cap, cut on a
/// newline when one is near the trim point.
pub fn trim_scrollback(text: &mut String) {
    if text.len() <= SCROLLBACK_CAP {
        return;
    }
    let start = text.len() - SCROLLBACK_CAP;
    let mut cut = text[start..]
        .find('\n')
        .map(|index| start + index + 1)
        .unwrap_or(start);
    while cut < text.len() && !text.is_char_boundary(cut) {
        cut += 1;
    }
    text.drain(..cut);
}

/// `https://host` becomes `wss://host/api/sessions/{id}/events`. Loopback
/// HTTP becomes `ws://`.
pub fn viewer_events_url(origin: &str, session_id: &str) -> String {
    let ws_origin = if let Some(rest) = origin.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = origin.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        origin.to_string()
    };
    format!("{ws_origin}/api/sessions/{session_id}/events")
}

fn session_id_ok(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[derive(Clone)]
struct WatchIo {
    http: HttpClient,
    device: DeviceIdentity,
    origin: String,
}

impl WatchIo {
    fn load() -> Result<Self> {
        let endpoints = CloudEndpoints::load()?;
        let device = DeviceIdentity::load_or_create()?;
        let http = HttpClient::builder().timeout(REQUEST_TIMEOUT).build()?;
        Ok(Self {
            http,
            device,
            origin: endpoints.origin().to_string(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }

    async fn post_json(&self, path: &str, body: Option<&Value>) -> Result<(), String> {
        let headers = self
            .device
            .auth_headers("POST", path)
            .map_err(|err| format!("couldn't sign the request: {err}"))?;
        let mut request = self.http.post(self.url(path));
        for (key, value) in headers {
            request = request.header(key, value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|err| format!("couldn't reach the session: {err}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(status_message(response.status().as_u16()))
        }
    }
}

fn status_message(code: u16) -> String {
    match code {
        404 => "couldn't reach that session: it is gone".to_string(),
        503 => "that computer is offline".to_string(),
        _ => format!("couldn't reach that session (HTTP {code})"),
    }
}

fn connect_message(err: &anyhow::Error) -> String {
    // The status code usually sits on the source, under a short context.
    let mut text = String::new();
    for (index, cause) in err.chain().enumerate() {
        if index > 0 {
            text.push_str(": ");
        }
        text.push_str(&cause.to_string());
    }
    let text = text.replace('\n', " ");
    let lower = text.to_ascii_lowercase();
    if lower.contains("401") || lower.contains("403") {
        return "couldn't open that session (HTTP 401)".to_string();
    }
    shorten(&format!("couldn't open that session: {text}"))
}

fn shorten(text: &str) -> String {
    let mut chars = text.chars();
    let short: String = chars.by_ref().take(160).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

async fn report(last: &mut String, updates: &mpsc::Sender<ViewerUpdate>, message: String) {
    if message == *last {
        return;
    }
    *last = message.clone();
    let _ = updates.send(ViewerUpdate::Failed(message)).await;
}

async fn run_watch(
    session_id: String,
    updates: mpsc::Sender<ViewerUpdate>,
    keys: mpsc::UnboundedReceiver<Vec<u8>>,
    mut cancel: watch::Receiver<bool>,
) {
    // The initial `false` is not a cancellation.
    cancel.borrow_and_update();
    if *cancel.borrow() {
        return;
    }
    if !session_id_ok(&session_id) {
        let _ = updates
            .send(ViewerUpdate::Failed(
                "that session has no cloud id".to_string(),
            ))
            .await;
        return;
    }
    let io = match WatchIo::load() {
        Ok(io) => io,
        Err(err) => {
            let _ = updates
                .send(ViewerUpdate::Failed(shorten(&format!(
                    "couldn't open that session: {err}"
                ))))
                .await;
            return;
        }
    };

    let mut heart_cancel = cancel.clone();
    heart_cancel.borrow_and_update();
    let heart = tokio::spawn(heartbeat_loop(
        session_id.clone(),
        io.clone(),
        updates.clone(),
        heart_cancel,
    ));
    let mut key_cancel = cancel.clone();
    key_cancel.borrow_and_update();
    let keys_task = tokio::spawn(key_loop(
        session_id.clone(),
        io.clone(),
        keys,
        updates.clone(),
        key_cancel,
    ));

    let mut last_error = String::new();
    loop {
        if *cancel.borrow() {
            break;
        }
        match connect_events(&io, &session_id).await {
            Ok(socket) => drive_events(socket, &updates, &mut cancel).await,
            Err(err) => report(&mut last_error, &updates, connect_message(&err)).await,
        }
        if !pause(&mut cancel, RECONNECT).await {
            break;
        }
    }
    heart.abort();
    keys_task.abort();
}

async fn pause(cancel: &mut watch::Receiver<bool>, delay: Duration) -> bool {
    tokio::select! {
        result = cancel.changed() => {
            result.is_ok() && !*cancel.borrow()
        }
        _ = tokio::time::sleep(delay) => !*cancel.borrow(),
    }
}

async fn connect_events(
    io: &WatchIo,
    session_id: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
> {
    let path = format!("/api/sessions/{session_id}/events");
    let headers = io.device.auth_headers("GET", &path)?;
    let url = viewer_events_url(&io.origin, session_id);
    let host = url
        .trim_start_matches("wss://")
        .trim_start_matches("ws://")
        .split('/')
        .next()
        .unwrap_or("");
    let ws_key = base64::engine::general_purpose::STANDARD.encode(rand::random::<[u8; 16]>());
    let mut builder = Request::builder()
        .uri(&url)
        .header("Host", host)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", ws_key);
    for (key, value) in headers {
        builder = builder.header(key, value);
    }
    let request = builder
        .body(())
        .context("Failed to build the viewer websocket request")?;
    let connect = tokio::time::timeout(REQUEST_TIMEOUT, connect_async(request))
        .await
        .context("Timed out opening the session")?
        .context("WebSocket connection error")?;
    Ok(connect.0)
}

async fn drive_events(
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    updates: &mpsc::Sender<ViewerUpdate>,
    cancel: &mut watch::Receiver<bool>,
) {
    let (mut write, mut read) = socket.split();
    let mut keepalive = tokio::time::interval(Duration::from_secs(20));
    keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick would fire immediately and race the greeting frames.
    keepalive.tick().await;
    loop {
        tokio::select! {
            result = cancel.changed() => {
                if result.is_err() || *cancel.borrow() {
                    break;
                }
            }
            message = read.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(value) = serde_json::from_str::<Value>(&text) else {
                            continue;
                        };
                        if let Some(update) = apply_viewer_message(&value) {
                            if updates.send(update).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if write.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
            _ = keepalive.tick() => {
                if write.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn heartbeat_loop(
    session_id: String,
    io: WatchIo,
    updates: mpsc::Sender<ViewerUpdate>,
    mut cancel: watch::Receiver<bool>,
) {
    let path = format!("/api/sessions/{session_id}/viewer-active");
    let mut ticks = tokio::time::interval(HEARTBEAT);
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_error = String::new();
    loop {
        tokio::select! {
            result = cancel.changed() => {
                if result.is_err() || *cancel.borrow() {
                    return;
                }
            }
            _ = ticks.tick() => {
                match io.post_json(&path, None).await {
                    Ok(()) => last_error.clear(),
                    Err(message) => {
                        if message == "that computer is offline" {
                            let _ = updates.send(ViewerUpdate::DesktopOffline).await;
                        }
                        report(&mut last_error, &updates, message).await;
                    }
                }
            }
        }
    }
}

async fn key_loop(
    session_id: String,
    io: WatchIo,
    mut keys: mpsc::UnboundedReceiver<Vec<u8>>,
    updates: mpsc::Sender<ViewerUpdate>,
    mut cancel: watch::Receiver<bool>,
) {
    let path = format!("/api/sessions/{session_id}/key-sequence");
    let mut last_error = String::new();
    loop {
        let first = tokio::select! {
            result = cancel.changed() => {
                if result.is_err() || *cancel.borrow() {
                    return;
                }
                continue;
            }
            message = keys.recv() => message,
        };
        let Some(mut bytes) = first else {
            return;
        };
        while let Ok(more) = keys.try_recv() {
            bytes.extend(more);
        }
        if bytes.is_empty() {
            continue;
        }
        // Control bytes and escape sequences are valid UTF-8, and the owning
        // desktop writes this text straight to the PTY.
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let body = serde_json::json!({
            "steps": [{ "type": "text", "text": text }]
        });
        match io.post_json(&path, Some(&body)).await {
            Ok(()) => last_error.clear(),
            Err(message) => {
                if message == "that computer is offline" {
                    let _ = updates.send(ViewerUpdate::DesktopOffline).await;
                }
                report(&mut last_error, &updates, message).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_messages_keep_the_screen_the_transcript_and_presence() {
        let screen = serde_json::json!({"type": "screen", "content": "hello"});
        assert_eq!(
            apply_viewer_message(&screen),
            Some(ViewerUpdate::Screen("hello".to_string()))
        );
        let history = serde_json::json!({"type": "scrollback_history", "content": "old\n"});
        assert_eq!(
            apply_viewer_message(&history),
            Some(ViewerUpdate::ScrollbackReset("old\n".to_string()))
        );
        let diff = serde_json::json!({"type": "scrollback", "diff": "new\n"});
        assert_eq!(
            apply_viewer_message(&diff),
            Some(ViewerUpdate::ScrollbackDiff("new\n".to_string()))
        );
        let offline = serde_json::json!({"type": "desktop_status", "connected": false});
        assert_eq!(
            apply_viewer_message(&offline),
            Some(ViewerUpdate::DesktopOffline)
        );
        let online = serde_json::json!({"type": "desktop_status", "connected": true});
        assert_eq!(
            apply_viewer_message(&online),
            Some(ViewerUpdate::DesktopOnline)
        );
        let state = serde_json::json!({"type": "state", "state": "ready"});
        assert!(apply_viewer_message(&state).is_none());
    }

    #[test]
    fn a_rejected_socket_says_the_session_could_not_be_opened() {
        let err =
            anyhow::anyhow!("HTTP error: 401 Unauthorized").context("WebSocket connection error");
        assert_eq!(
            connect_message(&err),
            "couldn't open that session (HTTP 401)"
        );
    }

    #[test]
    fn viewer_url_follows_the_origin_scheme() {
        assert_eq!(
            viewer_events_url("https://drinkcrabigator.com", "abc"),
            "wss://drinkcrabigator.com/api/sessions/abc/events"
        );
        assert_eq!(
            viewer_events_url("http://127.0.0.1:8787", "abc"),
            "ws://127.0.0.1:8787/api/sessions/abc/events"
        );
    }

    #[test]
    fn trim_scrollback_keeps_the_tail() {
        let mut text = format!("{}\nkeep this tail", "x".repeat(SCROLLBACK_CAP));
        trim_scrollback(&mut text);
        assert_eq!(text, "keep this tail");
    }
}
