use anyhow::{bail, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct PlatformPty {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    parser: vt100::Parser,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    #[allow(dead_code)]
    scroll_offset: usize,
}

impl PlatformPty {
    pub async fn new(
        output_tx: mpsc::Sender<Vec<u8>>,
        cols: u16,
        rows: u16,
        command: &str,
        extra_args: Vec<String>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let (resolved_command, path_prefix) = resolve_command(command)?;
        let mut cmd = CommandBuilder::new(&resolved_command);

        // Add any extra arguments (e.g., --resume, --continue)
        for arg in extra_args {
            cmd.arg(arg);
        }

        // Inherit current working directory
        if let Ok(cwd) = env::current_dir() {
            cmd.cwd(cwd);
        }

        // Inherit all environment variables from parent process
        // Ensure PATH includes the resolved command directory if needed.
        let mut saw_path = false;
        let path_override = build_path_override(path_prefix.as_ref());
        for (key, value) in env::vars() {
            if key == "PATH" {
                saw_path = true;
                if let Some(ref override_path) = path_override {
                    cmd.env("PATH", override_path);
                } else {
                    cmd.env(key, value);
                }
            } else {
                cmd.env(key, value);
            }
        }
        if !saw_path {
            if let Some(override_path) = path_override {
                cmd.env("PATH", override_path);
            }
        }

        // Override TERM for proper terminal support
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd)?;
        let child = Arc::new(Mutex::new(child));

        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let master = Arc::new(Mutex::new(pair.master));
        let writer = Arc::new(Mutex::new(writer));

        // Spawn reader task
        let output_tx_clone = output_tx.clone();
        tokio::spawn(async move {
            Self::read_loop(reader, output_tx_clone).await;
        });

        let parser = vt100::Parser::new(rows, cols, 1000);

        Ok(Self {
            master,
            parser,
            writer,
            child,
            scroll_offset: 0,
        })
    }

    pub fn is_running(&self) -> bool {
        let mut child = self.child.lock().unwrap();
        // try_wait returns Ok(Some(status)) if exited, Ok(None) if still running
        match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }

    #[allow(dead_code)]
    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = self.parser.screen().scrollback();
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    #[allow(dead_code)]
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    #[allow(dead_code)]
    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
    }

    async fn read_loop(reader: Box<dyn Read + Send>, tx: mpsc::Sender<Vec<u8>>) {
        // Wrap reader in Arc<Mutex> so it can be shared with spawn_blocking
        let reader = Arc::new(Mutex::new(reader));

        loop {
            let reader_clone = Arc::clone(&reader);

            // Use spawn_blocking for the blocking read to avoid blocking tokio runtime
            let read_result = tokio::task::spawn_blocking(move || {
                let mut buf = [0u8; 4096];
                let mut reader = reader_clone.lock().unwrap();
                match reader.read(&mut buf) {
                    Ok(0) => None,
                    Ok(n) => Some(buf[..n].to_vec()),
                    Err(_) => None,
                }
            })
            .await;

            match read_result {
                Ok(Some(data)) => {
                    if tx.send(data).await.is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    pub fn process_output(&mut self, data: &[u8]) {
        crate::guarded_process(&mut self.parser, data, 1000);
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let master = self.master.lock().unwrap();
        master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        crate::guarded_resize(&mut self.parser, cols, rows);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn parser(&self) -> &vt100::Parser {
        &self.parser
    }

    #[allow(dead_code)]
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
}

fn resolve_command(command: &str) -> Result<(String, Option<PathBuf>)> {
    if looks_like_path(command) {
        let path = PathBuf::from(command);
        if path.is_file() {
            return Ok((path.to_string_lossy().into_owned(), path.parent().map(Path::to_path_buf)));
        }
        bail!("Command path not found: {command}");
    }

    if let Some(found) = find_in_path(command) {
        return Ok((
            found.to_string_lossy().into_owned(),
            None,
        ));
    }

    if let Some(found) = find_in_fallbacks(command) {
        let prefix = found.parent().map(Path::to_path_buf);
        return Ok((found.to_string_lossy().into_owned(), prefix));
    }

    bail!("Command '{command}' not found in PATH. Install it or add it to PATH.");
}

fn looks_like_path(command: &str) -> bool {
    command.contains('/') || command.contains('\\')
}

fn find_in_path(command: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn find_in_fallbacks(command: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".npm-global/bin"));
        dirs.push(home.join(".yarn/bin"));
        dirs.push(home.join(".volta/bin"));
        dirs.push(home.join("bin"));
    }

    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/opt/homebrew/opt/node/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    dirs.push(PathBuf::from("/usr/bin"));
    dirs.push(PathBuf::from("/bin"));

    for dir in dirs {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn build_path_override(prefix: Option<&PathBuf>) -> Option<String> {
    let prefix = prefix?;
    let existing = env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<PathBuf> = env::split_paths(&existing).collect();
    if !paths.iter().any(|p| p == prefix) {
        paths.insert(0, prefix.to_path_buf());
    }
    env::join_paths(paths).ok().and_then(|p| p.into_string().ok())
}
