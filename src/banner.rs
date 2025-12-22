//! Session banners
//!
//! Prints styled session start/end banners with version, platform, and date.

use crate::platforms::PlatformKind;
use crate::terminal::escape::{BOLD, FG_BLUE, FG_CYAN, FG_GRAY, FG_ORANGE, FG_PURPLE, RESET};
#[cfg(debug_assertions)]
use crate::terminal::escape::DIM;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print session info banner with file paths
#[allow(unused_variables)]
pub fn print_session_banner(session_id: &str, platform: PlatformKind, cols: u16) {
    use chrono::Local;

    println!();

    // Format date: "Saturday, December 21st, 2025 5:58 PM"
    let now = Local::now();
    let day = now.format("%A").to_string();
    let month = now.format("%B").to_string();
    let date_num = now.format("%e").to_string().trim().to_string();
    let suffix = date_suffix(&date_num);
    let year = now.format("%Y").to_string();
    let time = now.format("%l:%M %p").to_string().trim().to_string();
    let date_str = format!("{}, {} {}{}, {} {}", day, month, date_num, suffix, year, time);

    // Header line: 🦀 Crabigator v0.1.0 ⛵ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ Platform · Date
    let platform_name = platform.display_name();
    let version_str = format!("{FG_GRAY}v{VERSION}{RESET}");
    let title = format!(
        "{FG_ORANGE}🦀{RESET} {BOLD}{FG_CYAN}Crabigator{RESET} {version_str} {FG_ORANGE}⛵{RESET}"
    );
    let right_side = format!("{FG_PURPLE}{platform_name}{RESET} {FG_BLUE}·{RESET} {FG_BLUE}{date_str}{RESET}");

    // Plain lengths
    let version_plain_len = 1 + VERSION.len(); // "v" + version
    let title_plain_len = 2 + 1 + 10 + 1 + version_plain_len + 1 + 2;
    let right_plain_len = platform_name.len() + 3 + date_str.len(); // "Platform · Date"
    // +4 accounts for: leading space, 2 spaces around rule, trailing space
    let rule_len = (cols as usize).saturating_sub(title_plain_len + right_plain_len + 4);
    let rule = format!("{FG_BLUE}{}{RESET}", "━".repeat(rule_len));
    println!(" {title} {rule} {right_side} ");

    // Only show session directory in debug builds
    #[cfg(debug_assertions)]
    {
        let session_dir = format!("/tmp/crabigator-{}/", session_id);
        println!("    {FG_PURPLE}Session{RESET}  {DIM}{session_dir}{RESET}");
    }

    println!();
}

/// Print session end line matching banner style with date
pub fn print_session_end_line(platform: PlatformKind, cols: u16) {
    use chrono::Local;

    let width = cols as usize;

    // Format date: "Saturday, December 21st, 2025 5:58 PM"
    let now = Local::now();
    let day = now.format("%A").to_string(); // Saturday
    let month = now.format("%B").to_string(); // December
    let date_num = now.format("%e").to_string().trim().to_string(); // 21
    let suffix = date_suffix(&date_num);
    let year = now.format("%Y").to_string(); // 2025
    let time = now.format("%l:%M %p").to_string().trim().to_string(); // 5:58 PM

    let date_str = format!("{}, {} {}{}, {} {}", day, month, date_num, suffix, year, time);

    // Left side: 🦀 Crabigator v0.1.0 ⛵
    let version_str = format!("{FG_GRAY}v{VERSION}{RESET}");
    let title = format!(
        "{FG_ORANGE}🦀{RESET} {BOLD}{FG_CYAN}Crabigator{RESET} {version_str} {FG_ORANGE}⛵{RESET}"
    );

    // Right side: Platform · Date
    let platform_name = platform.display_name();
    let right_side = format!("{FG_PURPLE}{platform_name}{RESET} {FG_BLUE}·{RESET} {FG_BLUE}{date_str}{RESET}");

    // Calculate plain lengths
    let version_plain_len = 1 + VERSION.len(); // "v" + version
    let title_plain_len = 2 + 1 + 10 + 1 + version_plain_len + 1 + 2; // 🦀 Crabigator vX.X.X ⛵
    let right_plain_len = platform_name.len() + 3 + date_str.len(); // "Platform · Date"

    // +4 accounts for: leading space, 2 spaces around rule, trailing space
    let rule_len = width.saturating_sub(title_plain_len + right_plain_len + 4);
    let rule = format!("{FG_BLUE}{}{RESET}", "━".repeat(rule_len));

    println!(" {title} {rule} {right_side} ");
}

/// Get ordinal suffix for a date number
fn date_suffix(date_num: &str) -> &'static str {
    match date_num {
        "1" | "21" | "31" => "st",
        "2" | "22" => "nd",
        "3" | "23" => "rd",
        _ => "th",
    }
}
