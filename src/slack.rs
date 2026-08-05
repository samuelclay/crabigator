//! Slack permalink extraction and display metadata.

use std::sync::OnceLock;

use chrono::{Local, TimeZone};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// One Slack message permalink pasted by the user during this session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct SlackThread {
    pub url: String,
    /// Unix seconds decoded from the permalink's `p<timestamp>` segment.
    pub posted_at: u64,
    /// Poster name when the recap transcript identifies it clearly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// A Slack message permalink: `https://<workspace>.slack.com/archives/<channel>/p<ts>`.
fn slack_permalink_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"https://[A-Za-z0-9_-]+\.slack\.com/archives/[A-Z0-9]+/p\d+[^\s"'<>)\]]*"#)
            .expect("valid Slack permalink regex")
    })
}

fn slack_timestamp_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/p(\d{10})\d*").expect("valid Slack timestamp regex"))
}

/// Extract every unique Slack permalink in appearance order. The timestamp is
/// encoded in the permalink, so it does not depend on model output.
pub fn extract_threads(text: &str) -> Vec<SlackThread> {
    let mut threads = Vec::new();
    for found in slack_permalink_re().find_iter(text) {
        let url = found
            .as_str()
            .trim_end_matches(['.', ',', ';', ':'])
            .to_string();
        if threads.iter().any(|thread: &SlackThread| thread.url == url) {
            continue;
        }
        let posted_at = slack_timestamp_re()
            .captures(&url)
            .and_then(|captures| captures.get(1))
            .and_then(|seconds| seconds.as_str().parse().ok())
            .unwrap_or_default();
        threads.push(SlackThread {
            url,
            posted_at,
            author: None,
        });
    }
    threads
}

/// Compact label shared by the terminal link. The dashboard formats the same
/// timestamp in the viewer's local timezone.
pub fn display_label(thread: &SlackThread) -> String {
    let date = Local
        .timestamp_opt(thread.posted_at as i64, 0)
        .single()
        .map(|timestamp| timestamp.format("%b %-d, %-I:%M %p").to_string())
        .unwrap_or_else(|| "Unknown time".to_string());
    match thread.author.as_deref() {
        Some(author) if !author.is_empty() => format!("Slack · {date} · {author}"),
        _ => format!("Slack · {date}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_deduplicates_permalink_timestamps() {
        let url = "https://tavus.slack.com/archives/C123/p1754404040123456?thread_ts=1754400000.000000&cid=C123";
        let threads = extract_threads(&format!("{url}\n{url}."));

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].url, url);
        assert_eq!(threads[0].posted_at, 1_754_404_040);
        assert!(threads[0].author.is_none());
        assert!(display_label(&threads[0]).starts_with("Slack · "));

        let mut named = threads[0].clone();
        named.author = Some("Sam Clay".to_string());
        assert!(display_label(&named).ends_with("· Sam Clay"));
    }
}
