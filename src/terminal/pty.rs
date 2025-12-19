use anyhow::Result;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::env;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct CliPty {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    parser: vt100::Parser,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    #[allow(dead_code)]
    scroll_offset: usize,
}

impl CliPty {
    pub async fn new(
        output_tx: mpsc::Sender<Vec<u8>>,
        cols: u16,
        rows: u16,
        cli_name: &str,
        extra_args: Vec<String>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(cli_name);

        // Add any extra arguments (e.g., --resume, --continue)
        for arg in extra_args {
            cmd.arg(arg);
        }

        // Inherit current working directory
        if let Ok(cwd) = env::current_dir() {
            cmd.cwd(cwd);
        }

        // Inherit all environment variables from parent process
        for (key, value) in env::vars() {
            cmd.env(key, value);
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

    async fn read_loop(mut reader: Box<dyn Read + Send>, tx: mpsc::Sender<Vec<u8>>) {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }

    pub fn process_output(&mut self, data: &[u8]) {
        // Wrap in catch_unwind to prevent vt100 parser panics from crashing the app
        // The parser can panic on certain edge cases (e.g., cursor position out of bounds)
        let parser_ptr = &mut self.parser as *mut vt100::Parser;
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // SAFETY: We're only accessing the parser within this closure
            unsafe { (*parser_ptr).process(data) };
        }));
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
        self.parser.set_size(rows, cols);
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
