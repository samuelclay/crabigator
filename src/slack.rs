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
    /// Human-readable channel name when known, otherwise the channel ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Poster name when the recap transcript identifies it clearly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// A Slack message permalink: `https://<workspace>.slack.com/archives/<channel>/p<ts>`.
fn slack_permalink_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"https://[A-Za-z0-9_-]+\.slack\.com/archives/([A-Z0-9]+)/p(\d{10})\d*[^\s"'<>)\]]*"#,
        )
        .expect("valid Slack permalink regex")
    })
}

/// Extract every unique Slack permalink in appearance order. The timestamp is
/// encoded in the permalink, so it does not depend on model output.
pub fn extract_threads(text: &str) -> Vec<SlackThread> {
    let mut threads = Vec::new();
    for captures in slack_permalink_re().captures_iter(text) {
        let url = captures
            .get(0)
            .expect("permalink capture")
            .as_str()
            .trim_end_matches(['.', ',', ';', ':'])
            .to_string();
        if threads.iter().any(|thread: &SlackThread| thread.url == url) {
            continue;
        }
        let posted_at = captures
            .get(2)
            .and_then(|seconds| seconds.as_str().parse().ok())
            .unwrap_or_default();
        let channel = captures.get(1).map(|channel| channel.as_str().to_string());
        threads.push(SlackThread {
            url,
            posted_at,
            channel,
            author: None,
        });
    }
    threads
}

/// Full terminal label. The dashboard formats the same timestamp in the
/// viewer's local timezone.
pub fn display_label(thread: &SlackThread) -> String {
    let date = format_date(thread.posted_at, "%b %-d, %-I:%M %p");
    label_parts(thread, date).join(" · ")
}

/// Keep every Slack metadata field visible when the Changes column is narrow.
pub fn compact_display_label(thread: &SlackThread, max_width: usize) -> String {
    let full = display_label(thread);
    if full.chars().count() <= max_width {
        return full;
    }

    let date = format_date(thread.posted_at, "%-m/%-d %-I%P");
    let mut parts = label_parts(thread, date);
    let separator_width = parts.len().saturating_sub(1);
    let fixed_width = parts[0].chars().count() + separator_width;
    let field_count = parts.len().saturating_sub(1);
    let available = max_width.saturating_sub(fixed_width);

    if field_count > 0 && available >= field_count * 2 {
        let mut budgets = parts[1..]
            .iter()
            .map(|part| part.chars().count())
            .collect::<Vec<_>>();
        while budgets.iter().sum::<usize>() > available {
            let Some((index, _)) = budgets
                .iter()
                .enumerate()
                .filter(|(_, budget)| **budget > 2)
                .max_by_key(|(_, budget)| **budget)
            else {
                break;
            };
            budgets[index] -= 1;
        }
        for (part, budget) in parts[1..].iter_mut().zip(budgets) {
            *part = truncate_field(part, budget);
        }
    }

    let compact = parts.join(" ");
    truncate_field(&compact, max_width)
}

fn format_date(posted_at: u64, format: &str) -> String {
    Local
        .timestamp_opt(posted_at as i64, 0)
        .single()
        .map(|timestamp| timestamp.format(format).to_string())
        .unwrap_or_else(|| "Unknown time".to_string())
}

fn label_parts(thread: &SlackThread, date: String) -> Vec<String> {
    let mut parts = vec!["Slack".to_string(), date];
    if let Some(channel) = thread
        .channel
        .as_deref()
        .filter(|channel| !channel.is_empty())
    {
        parts.push(format!("#{}", channel.trim_start_matches('#')));
    }
    if let Some(author) = thread.author.as_deref().filter(|author| !author.is_empty()) {
        parts.push(author.to_string());
    }
    parts
}

fn truncate_field(text: &str, max_width: usize) -> String {
    if text.chars().count() <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".chars().take(max_width).collect();
    }
    let prefix = text
        .chars()
        .take(max_width - 1)
        .collect::<String>()
        .trim_end()
        .to_string();
    format!("{prefix}…")
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
        assert_eq!(threads[0].channel.as_deref(), Some("C123"));
        assert!(threads[0].author.is_none());
        assert!(display_label(&threads[0]).contains(" · #C123"));

        let mut named = threads[0].clone();
        named.channel = Some("builder".to_string());
        named.author = Some("Sam Clay".to_string());
        let label = display_label(&named);
        assert!(label.contains(" · #builder · "));
        assert!(label.ends_with("Sam Clay"));

        let compact = compact_display_label(&named, 25);
        assert!(compact.starts_with("Slack "));
        let date_index = compact.find('/').unwrap();
        let channel_index = compact.find("#b").unwrap();
        let author_index = compact.find("Sam").unwrap();
        assert!(date_index < channel_index);
        assert!(channel_index < author_index);
        assert!(compact.ends_with('…'));
        assert!(compact.chars().count() <= 25);
    }
}
