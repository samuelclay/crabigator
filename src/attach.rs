//! Local attach socket.
//!
//! The session that owns the agent listens on `attach.sock` in its session
//! directory. The PR board's fullscreen view connects and sends key and paste
//! frames. The owner applies them on the same path as a key typed locally.
//! Closing the connection only detaches; the agent keeps running.

use std::io;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub const SOCKET_NAME: &str = "attach.sock";

/// Shown when the session is on this computer but its process is not
/// listening. That happens when it was started before attach existed.
pub const NOT_RUNNING: &str =
    "This session is running, but it was started before attach. Restart it, then try again.";

pub fn socket_path(session_dir: &Path) -> PathBuf {
    session_dir.join(SOCKET_NAME)
}

/// One line on the socket. `mods` uses crossterm's modifier bits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AttachFrame {
    Key {
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ch: Option<char>,
        #[serde(default, skip_serializing_if = "is_zero_mod")]
        mods: u8,
    },
    Paste {
        text: String,
    },
}

fn is_zero_mod(mods: &u8) -> bool {
    *mods == 0
}

/// What the attach view should do with a key.
#[derive(Debug, PartialEq, Eq)]
pub enum AttachKeyAction {
    /// Leave fullscreen. The key is not sent.
    Detach,
    Forward(AttachFrame),
    Ignore,
}

/// A message from the socket task to the session that owns the agent.
#[derive(Debug, PartialEq, Eq)]
pub enum AttachEvent {
    Connected,
    Disconnected,
    Input(AttachFrame),
}

/// `Some` when a client connected or left. `None` for key and paste traffic.
pub fn next_client_count(count: usize, event: &AttachEvent) -> Option<usize> {
    match event {
        AttachEvent::Connected => Some(count.saturating_add(1)),
        AttachEvent::Disconnected => Some(count.saturating_sub(1)),
        AttachEvent::Input(_) => None,
    }
}

/// Ctrl-] detaches. Crossterm delivers that byte (`0x1d`) as Ctrl-5, and
/// some terminals send the raw character instead. Ctrl-C does not detach.
pub fn is_detach_key(key: KeyEvent) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }
    let ctrl_only = key.modifiers.contains(KeyModifiers::CONTROL)
        && !key.modifiers.contains(KeyModifiers::ALT)
        && !key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Char('\u{1d}') => true,
        KeyCode::Char(']') | KeyCode::Char('5') if ctrl_only => true,
        _ => false,
    }
}

pub fn attach_key_action(key: KeyEvent) -> AttachKeyAction {
    if is_detach_key(key) {
        AttachKeyAction::Detach
    } else if let Some(frame) = frame_from_key(key) {
        AttachKeyAction::Forward(frame)
    } else {
        AttachKeyAction::Ignore
    }
}

pub fn frame_from_key(key: KeyEvent) -> Option<AttachFrame> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let mods = key.modifiers.bits();
    let (code, ch) = match key.code {
        KeyCode::Char(ch) => ("char".to_string(), Some(ch)),
        KeyCode::Enter => ("enter".to_string(), None),
        KeyCode::Backspace => ("backspace".to_string(), None),
        KeyCode::Tab => ("tab".to_string(), None),
        KeyCode::BackTab => ("backtab".to_string(), None),
        KeyCode::Esc => ("esc".to_string(), None),
        KeyCode::Up => ("up".to_string(), None),
        KeyCode::Down => ("down".to_string(), None),
        KeyCode::Left => ("left".to_string(), None),
        KeyCode::Right => ("right".to_string(), None),
        KeyCode::Home => ("home".to_string(), None),
        KeyCode::End => ("end".to_string(), None),
        KeyCode::PageUp => ("pageup".to_string(), None),
        KeyCode::PageDown => ("pagedown".to_string(), None),
        KeyCode::Delete => ("delete".to_string(), None),
        KeyCode::Insert => ("insert".to_string(), None),
        KeyCode::F(n) => (format!("f{n}"), None),
        KeyCode::Null => ("null".to_string(), None),
        _ => return None,
    };
    Some(AttachFrame::Key { code, ch, mods })
}

pub fn key_event_from_frame(frame: &AttachFrame) -> Option<KeyEvent> {
    let AttachFrame::Key { code, ch, mods } = frame else {
        return None;
    };
    let modifiers = KeyModifiers::from_bits_truncate(*mods);
    let key_code = match code.as_str() {
        "char" => KeyCode::Char((*ch)?),
        "enter" => KeyCode::Enter,
        "backspace" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" => KeyCode::Esc,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "null" => KeyCode::Null,
        other => {
            let n = other.strip_prefix('f')?.parse().ok()?;
            KeyCode::F(n)
        }
    };
    Some(KeyEvent::new(key_code, modifiers))
}

/// Removes the socket file when dropped, including after a crash-free exit.
#[cfg(unix)]
pub struct SocketGuard(PathBuf);

#[cfg(unix)]
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(not(unix))]
pub struct SocketGuard;

/// Bind `attach.sock` and forward client frames. Returns `None` when the
/// session directory is missing, the platform has no unix sockets, or the
/// bind fails. A session still starts if attach cannot listen.
pub fn serve(session_dir: &Path) -> Option<(mpsc::Receiver<AttachEvent>, SocketGuard)> {
    #[cfg(unix)]
    {
        if session_dir.as_os_str().is_empty() || !session_dir.is_dir() {
            return None;
        }
        bind_socket(session_dir).ok()
    }
    #[cfg(not(unix))]
    {
        let _ = session_dir;
        None
    }
}

#[cfg(unix)]
fn bind_socket(session_dir: &Path) -> io::Result<(mpsc::Receiver<AttachEvent>, SocketGuard)> {
    use tokio::net::UnixListener;

    let path = socket_path(session_dir);
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    let listener = std::os::unix::net::UnixListener::bind(&path)?;
    set_owner_only(&path)?;
    listener.set_nonblocking(true)?;
    let listener = UnixListener::from_std(listener)?;
    let (tx, rx) = mpsc::channel(256);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let tx = tx.clone();
            tokio::spawn(read_client(stream, tx));
        }
    });
    Ok((rx, SocketGuard(path)))
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(unix)]
async fn read_client(stream: tokio::net::UnixStream, tx: mpsc::Sender<AttachEvent>) {
    use tokio::io::{AsyncBufReadExt, BufReader};

    if tx.send(AttachEvent::Connected).await.is_err() {
        return;
    }
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(frame) = serde_json::from_str::<AttachFrame>(&line) else {
            continue;
        };
        if tx.send(AttachEvent::Input(frame)).await.is_err() {
            return;
        }
    }
    let _ = tx.send(AttachEvent::Disconnected).await;
}

/// Writes frames to a session's attach socket.
#[cfg(unix)]
pub struct AttachClient {
    stream: std::os::unix::net::UnixStream,
}

#[cfg(unix)]
impl AttachClient {
    pub fn connect(session_dir: &Path) -> io::Result<Self> {
        let stream = std::os::unix::net::UnixStream::connect(socket_path(session_dir))?;
        Ok(Self { stream })
    }

    pub fn send(&mut self, frame: &AttachFrame) -> io::Result<()> {
        use std::io::Write;
        let mut line = serde_json::to_vec(frame)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        line.push(b'\n');
        self.stream.write_all(&line)?;
        self.stream.flush()
    }
}

#[cfg(not(unix))]
pub struct AttachClient;

#[cfg(not(unix))]
impl AttachClient {
    pub fn connect(_session_dir: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "attach needs a unix socket",
        ))
    }

    pub fn send(&mut self, _frame: &AttachFrame) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "attach needs a unix socket",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn ctrl_bracket_detaches_and_ctrl_c_is_forwarded() {
        let detach = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL);
        assert_eq!(attach_key_action(detach), AttachKeyAction::Detach);
        // The byte 0x1d arrives from crossterm as Ctrl-5.
        let as_ctrl_five = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::CONTROL);
        assert_eq!(attach_key_action(as_ctrl_five), AttachKeyAction::Detach);

        let interrupt = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        match attach_key_action(interrupt) {
            AttachKeyAction::Forward(AttachFrame::Key { code, ch, mods }) => {
                assert_eq!(code, "char");
                assert_eq!(ch, Some('c'));
                assert_eq!(mods, KeyModifiers::CONTROL.bits());
            }
            other => panic!("ctrl-c should be forwarded, got {other:?}"),
        }
    }

    #[test]
    fn key_frame_round_trips() {
        let samples = [
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
            KeyEvent::new(KeyCode::Up, KeyModifiers::ALT),
            KeyEvent::new(KeyCode::F(2), KeyModifiers::empty()),
        ];
        for key in samples {
            let frame = frame_from_key(key).expect("key encodes");
            let back = key_event_from_frame(&frame).expect("frame decodes");
            assert_eq!(back.code, key.code);
            assert_eq!(back.modifiers, key.modifiers);
        }
    }

    #[test]
    fn client_count_tracks_connect_and_disconnect() {
        let key = AttachEvent::Input(AttachFrame::Key {
            code: "enter".to_string(),
            ch: None,
            mods: 0,
        });
        assert_eq!(next_client_count(0, &AttachEvent::Connected), Some(1));
        assert_eq!(next_client_count(1, &AttachEvent::Disconnected), Some(0));
        assert_eq!(next_client_count(0, &AttachEvent::Disconnected), Some(0));
        assert_eq!(next_client_count(2, &key), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn client_frames_arrive_and_the_socket_is_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let (mut events, _guard) = serve(dir.path()).expect("socket binds");
        let path = socket_path(dir.path());
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777
        };
        assert_eq!(mode, 0o600);

        let mut client = AttachClient::connect(dir.path()).unwrap();
        let connected = tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .expect("connected in time")
            .expect("connected event");
        assert_eq!(connected, AttachEvent::Connected);

        {
            use std::io::Write;
            let mut raw = std::os::unix::net::UnixStream::connect(&path).unwrap();
            raw.write_all(b"not json\n").unwrap();
            raw.write_all(b"{\"type\":\"key\",\"code\":\"char\",\"ch\":\"z\"}\n")
                .unwrap();
            // Dropping `raw` is the disconnect for this second client.
        }

        client
            .send(&AttachFrame::Paste {
                text: "paste-me".to_string(),
            })
            .unwrap();
        drop(client);

        let mut saw_key = false;
        let mut saw_paste = false;
        let mut disconnects = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline && disconnects < 2 {
            let Ok(Some(event)) =
                tokio::time::timeout(Duration::from_millis(200), events.recv()).await
            else {
                continue;
            };
            match event {
                AttachEvent::Input(AttachFrame::Key { ch: Some('z'), .. }) => saw_key = true,
                AttachEvent::Input(AttachFrame::Paste { text }) if text == "paste-me" => {
                    saw_paste = true
                }
                AttachEvent::Disconnected => disconnects += 1,
                AttachEvent::Connected => {}
                other => panic!("unexpected {other:?}"),
            }
        }
        assert!(saw_key, "a key frame arrived");
        assert!(saw_paste, "a paste frame arrived");
        assert_eq!(disconnects, 2, "both clients disconnect");
    }
}
