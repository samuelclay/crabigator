mod app;
mod app_v2;
mod events;
mod git;
mod hooks;
mod parsers;
mod pty;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, stdout};
use std::panic;

use crate::app_v2::AppV2;

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    // Primary screen buffer (no alternate screen) - allows native scrollback
    // Disable mouse capture to allow native text selection
    // Enable bracketed paste for efficient paste handling
    execute!(
        stdout,
        Clear(ClearType::All),
        // No mouse capture - let terminal handle selection
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    // Reset scroll region to full screen
    print!("\x1b[r");
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
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
    setup_panic_handler();

    let mut terminal = setup_terminal()?;
    let size = terminal.size()?;
    let mut app = AppV2::new(size.width, size.height).await?;

    let result = app.run().await;

    // Capture stats before restoring terminal
    let stats = app.claude_stats.clone();

    restore_terminal(&mut terminal)?;

    // Print session stats after exit
    println!("\n--- Crabigator Session Stats ---");
    println!("Idle time: {}s", stats.idle_seconds);
    println!("Work time: {}s", stats.work_seconds);
    println!("Tokens used: {}", stats.tokens_used);
    println!("Messages: {}", stats.messages_count);
    println!("--------------------------------\n");

    result
}
