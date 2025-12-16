mod app;
mod git;
mod hooks;
mod parsers;
mod platforms;
mod terminal;
mod ui;

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size},
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
    let mut stdout = stdout();
    let (cols, rows) = terminal_size()?;

    // Push existing content up into scrollback by printing newlines
    // This preserves the command that launched us in the scrollback buffer
    for _ in 0..rows {
        writeln!(stdout)?;
    }
    stdout.flush()?;

    enable_raw_mode()?;

    // Move cursor to top of screen and enable bracketed paste
    // Primary screen buffer (no alternate screen) - allows native scrollback
    // Disable mouse capture to allow native text selection
    write!(stdout, "{}", terminal::escape::CURSOR_HOME)?;
    execute!(
        stdout,
        EnableBracketedPaste
    )?;

    Ok((cols, rows))
}

fn restore_terminal(total_rows: u16) -> Result<()> {
    let mut stdout = stdout();
    // Reset scroll region to full screen
    write!(stdout, "{}", terminal::escape::SCROLL_REGION_RESET)?;
    // Move cursor to the bottom of the screen, then down one more line
    // This ensures we're below all content (Claude output + status widgets)
    write!(stdout, "{}", terminal::escape::cursor_to(total_rows, 1))?;
    stdout.flush()?;

    disable_raw_mode()?;
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
        print!("{}", terminal::escape::SCROLL_REGION_RESET);
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

    // Install/update Claude Code hooks before anything else
    // This ensures our stats hooks are in place when Claude Code starts
    let platform = platforms::current_platform();
    if let Err(e) = platforms::Platform::ensure_hooks_installed(&platform) {
        eprintln!("Warning: Failed to install hooks: {}", e);
    }

    setup_panic_handler();

    let (cols, rows) = setup_terminal()?;
    let mut app = App::new(cols, rows, args.claude_args).await?;

    let result = app.run().await;

    // Capture stats and layout before restoring terminal
    let stats = app.claude_stats.clone();
    let total_rows = app.total_rows;

    restore_terminal(total_rows)?;

    // Print session summary after exit
    println!();
    println!(
        "Session: {} messages, {} tool calls",
        stats.platform_stats.messages,
        stats.platform_stats.total_tool_calls()
    );

    result
}
