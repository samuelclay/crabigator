mod app;
mod events;
mod git;
mod hooks;
mod input;
mod parsers;
mod pty;
mod utils;
mod widgets;

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::env;
use std::io::{stdout, Write};
use std::panic;

use crate::app::App;

#[derive(Default)]
struct Args {
    claude_args: Vec<String>,
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut iter = env::args().skip(1); // Skip the binary name

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-r" | "--resume" => {
                args.claude_args.push("--resume".to_string());
            }
            "-c" | "--continue" => {
                args.claude_args.push("--continue".to_string());
            }
            _ => {
                // Pass through any other arguments to claude
                args.claude_args.push(arg);
            }
        }
    }

    args
}

fn setup_terminal() -> Result<(u16, u16)> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    // Primary screen buffer (no alternate screen) - allows native scrollback
    // Disable mouse capture to allow native text selection
    // Enable bracketed paste for efficient paste handling
    execute!(
        stdout,
        Clear(ClearType::All),
        EnableBracketedPaste
    )?;
    let size = terminal::size()?;
    Ok(size)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    let mut stdout = stdout();
    // Reset scroll region to full screen
    write!(stdout, "\x1b[r")?;
    execute!(
        stdout,
        DisableBracketedPaste,
        Show
    )?;
    stdout.flush()?;
    Ok(())
}

fn setup_panic_handler() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        // Reset scroll region
        print!("\x1b[r");
        let _ = execute!(
            stdout(),
            DisableBracketedPaste
        );
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    setup_panic_handler();

    let (cols, rows) = setup_terminal()?;
    let mut app = App::new(cols, rows, args.claude_args).await?;

    let result = app.run().await;

    // Capture stats before restoring terminal
    let stats = app.claude_stats.clone();

    restore_terminal()?;

    // Print session stats after exit
    println!("\n--- Crabigator Session Stats ---");
    println!("Session: {}", stats.format_work());
    println!("Tokens: {}", stats.tokens_used);
    println!("Messages: {}", stats.messages_count);
    println!("--------------------------------\n");

    result
}
