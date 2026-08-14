//! Slack permalink extraction and display metadata.

use std::collections::HashMap;
#[cfg(not(test))]
use std::fs;
#[cfg(not(test))]
use std::path::{Path, PathBuf};
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
    /// Poster display name when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// A Slack message permalink: `https://<workspace>.slack.com/archives/<channel>/p<ts>`.
fn slack_permalink_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"https://[A-Za-z0-9_-]+\.slack\.com/archives/([A-Z0-9]+)/p(\d{10,})[^\s"'<>)\]]*"#,
        )
        .expect("valid Slack permalink regex")
    })
}

/// Prefix of a Slack MCP message CSV row, through the channel field. Message
/// text may contain commas and newlines, so only parse the stable leading
/// fields needed to identify the linked message and its author.
fn slack_message_row_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^(\d{10})\.(\d+),([A-Z0-9]+),([^,\r\n]*),([^,\r\n]*),([A-Z0-9]+)(?: \(#([^)]+)\))?,"#,
        )
        .expect("valid Slack MCP message row regex")
    })
}

/// Local Slack directory learned by the installed Slack MCP server. Its cache
/// contains only workspace metadata, so Crabigator can resolve names without
/// reading or copying the server's credentials.
#[derive(Default)]
pub(crate) struct SlackDirectory {
    channels: HashMap<String, String>,
    users: HashMap<String, String>,
    messages: HashMap<(String, String), SlackMessageMetadata>,
}

#[derive(Deserialize)]
struct CachedChannel {
    id: String,
    name: String,
}

#[derive(Default, Deserialize)]
struct CachedUserProfile {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    real_name: String,
}

#[derive(Deserialize)]
struct CachedUser {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    real_name: String,
    #[serde(default)]
    profile: CachedUserProfile,
}

#[derive(Clone)]
pub(crate) struct SlackMessageMetadata {
    message_id: String,
    channel_id: String,
    channel: Option<String>,
    author: Option<String>,
}

struct SlackMessageRow {
    message_id: String,
    user_id: String,
    username: String,
    real_name: String,
    channel_id: String,
    channel: Option<String>,
}

impl SlackDirectory {
    pub fn load() -> Self {
        #[cfg(not(test))]
        {
            let mut directory = Self::default();
            directory.reload();
            directory
        }
        #[cfg(test)]
        {
            Self::default()
        }
    }

    #[cfg(not(test))]
    fn reload(&mut self) {
        for path in slack_cache_paths("SLACK_MCP_CHANNELS_CACHE", "channels_cache_v2.json") {
            let Ok(text) = fs::read_to_string(path) else {
                continue;
            };
            self.add_channels(&text);
        }
        for path in slack_cache_paths("SLACK_MCP_USERS_CACHE", "users_cache.json") {
            let Ok(text) = fs::read_to_string(path) else {
                continue;
            };
            self.add_users(&text);
        }
    }

    /// Replace the permalink's fallback channel ID when the local Slack
    /// directory already knows its readable name.
    pub fn enrich_thread(&mut self, thread: &mut SlackThread) {
        let Some(channel_id) = slack_channel_id(&thread.url) else {
            return;
        };
        if !self.channels.contains_key(channel_id) {
            // The Slack MCP server may create its cache after Crabigator starts.
            // Retry when a newly pasted channel is not in the startup snapshot.
            #[cfg(not(test))]
            self.reload();
        }
        if let Some(channel) = self.channels.get(channel_id) {
            thread.channel = Some(channel.clone());
        }
        let Some(message_id) = slack_message_id(&thread.url) else {
            return;
        };
        if let Some(metadata) = self
            .messages
            .get(&(channel_id.to_string(), message_id.to_string()))
        {
            metadata.apply_to(thread);
        }
    }

    /// Read exact channel and poster metadata from Slack MCP message results.
    /// This runs while the turn is active, so an interrupted turn still keeps
    /// the metadata already returned by Slack.
    pub fn message_metadata(&mut self, text: &str) -> Vec<SlackMessageMetadata> {
        let mut rows = parse_slack_message_rows(text);
        if text.contains("\\n") {
            let decoded_newlines = text.replace("\\r\\n", "\n").replace("\\n", "\n");
            rows.extend(parse_slack_message_rows(&decoded_newlines));
        }

        for row in &rows {
            if let Some(channel) = &row.channel {
                self.channels
                    .insert(row.channel_id.clone(), channel.clone());
            }
        }

        let metadata = rows
            .into_iter()
            .map(|row| {
                let channel = self.channels.get(&row.channel_id).cloned().or(row.channel);
                let author = self
                    .users
                    .get(&row.user_id)
                    .cloned()
                    .or_else(|| nonempty(row.real_name))
                    .or_else(|| nonempty(row.username));
                SlackMessageMetadata {
                    message_id: row.message_id,
                    channel_id: row.channel_id,
                    channel,
                    author,
                }
            })
            .collect::<Vec<_>>();
        for item in &metadata {
            let key = (item.channel_id.clone(), item.message_id.clone());
            if let Some(existing) = self.messages.get_mut(&key) {
                if item.channel.is_some() {
                    existing.channel = item.channel.clone();
                }
                if item.author.is_some() {
                    existing.author = item.author.clone();
                }
            } else {
                self.messages.insert(key, item.clone());
            }
        }
        metadata
    }

    fn add_channels(&mut self, text: &str) {
        let Ok(channels) = serde_json::from_str::<Vec<CachedChannel>>(text) else {
            return;
        };
        for channel in channels {
            let name = clean_channel_name(&channel.name);
            if !channel.id.is_empty() && !name.is_empty() {
                self.channels.insert(channel.id, name);
            }
        }
    }

    fn add_users(&mut self, text: &str) {
        let Ok(users) = serde_json::from_str::<Vec<CachedUser>>(text) else {
            return;
        };
        for user in users {
            let name = nonempty(user.profile.display_name)
                .or_else(|| nonempty(user.profile.real_name))
                .or_else(|| nonempty(user.real_name))
                .or_else(|| nonempty(user.name));
            if let Some(name) = name.filter(|_| !user.id.is_empty()) {
                self.users.insert(user.id, name);
            }
        }
    }
}

fn parse_slack_message_rows(text: &str) -> Vec<SlackMessageRow> {
    slack_message_row_re()
        .captures_iter(text)
        .filter_map(|captures| {
            let seconds = captures.get(1)?.as_str();
            let fraction = captures.get(2)?.as_str();
            let channel = captures
                .get(7)
                .map(|name| clean_channel_name(name.as_str()))
                .filter(|name| !name.is_empty());
            Some(SlackMessageRow {
                message_id: format!("{seconds}{fraction}"),
                user_id: captures.get(3)?.as_str().to_string(),
                username: captures.get(4)?.as_str().trim().to_string(),
                real_name: captures.get(5)?.as_str().trim().to_string(),
                channel_id: captures.get(6)?.as_str().to_string(),
                channel,
            })
        })
        .collect()
}

impl SlackMessageMetadata {
    pub(crate) fn matches(&self, thread: &SlackThread) -> bool {
        slack_message_id(&thread.url) == Some(self.message_id.as_str())
            && slack_channel_id(&thread.url) == Some(self.channel_id.as_str())
    }

    pub(crate) fn apply_to(&self, thread: &mut SlackThread) -> bool {
        let mut changed = false;
        if let Some(channel) = &self.channel {
            if thread.channel.as_ref() != Some(channel) {
                thread.channel = Some(channel.clone());
                changed = true;
            }
        }
        if let Some(author) = &self.author {
            if thread.author.as_ref() != Some(author) {
                thread.author = Some(author.clone());
                changed = true;
            }
        }
        changed
    }
}

#[cfg(not(test))]
fn slack_cache_paths(env_name: &str, suffix: &str) -> Vec<PathBuf> {
    if let Some(path) = std::env::var_os(env_name).filter(|path| !path.is_empty()) {
        return vec![PathBuf::from(path)];
    }
    let Some(cache_dir) = dirs::cache_dir() else {
        return Vec::new();
    };
    let directory = cache_dir.join("slack-mcp-server");
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| cache_name_ends_with(path, suffix))
        .collect()
}

#[cfg(not(test))]
fn cache_name_ends_with(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(suffix))
}

fn nonempty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn clean_channel_name(name: &str) -> String {
    name.trim().trim_start_matches('#').to_string()
}

fn slack_channel_id(url: &str) -> Option<&str> {
    slack_permalink_re()
        .captures(url)
        .and_then(|captures| captures.get(1).map(|channel| channel.as_str()))
}

pub(crate) fn has_only_channel_id(thread: &SlackThread) -> bool {
    thread
        .channel
        .as_deref()
        .is_none_or(|channel| slack_channel_id(&thread.url) == Some(channel))
}

fn slack_message_id(url: &str) -> Option<&str> {
    slack_permalink_re()
        .captures(url)
        .and_then(|captures| captures.get(2).map(|message| message.as_str()))
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
            .and_then(|timestamp| timestamp.as_str().get(..10))
            .and_then(|seconds| seconds.parse().ok())
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

/// PR-board label: the thread identity without repeating the word "Slack" or
/// its timestamp. The permalink always supplies at least a channel ID.
pub(crate) fn thread_identity_label(thread: &SlackThread) -> String {
    let channel = thread
        .channel
        .as_deref()
        .filter(|channel| !channel.is_empty())
        .or_else(|| slack_channel_id(&thread.url))
        .map(|channel| format!("#{}", channel.trim_start_matches('#')))
        .unwrap_or_else(|| "#thread".to_string());
    match thread.author.as_deref().filter(|author| !author.is_empty()) {
        Some(author) => format!("{channel} · {author}"),
        None => channel,
    }
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

    const CHANNEL_CACHE: &str = r##"[{"id":"C0BANBTAKLL","name":"#blackbird-bug-reports"}]"##;
    const USER_CACHE: &str = r#"[{
        "id":"U09062D9XJP",
        "name":"ashish",
        "real_name":"Ashish Kumar",
        "profile":{"display_name":"Ashish","real_name":"Ashish Kumar"}
    }]"#;

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

    #[test]
    fn thread_identity_uses_channel_and_author_without_slack_prose() {
        let named = SlackThread {
            url: "https://t.slack.com/archives/C123/p1234567890000000".to_string(),
            posted_at: 0,
            channel: Some("#builder".to_string()),
            author: Some("Sam Clay".to_string()),
        };
        assert_eq!(thread_identity_label(&named), "#builder · Sam Clay");

        let channel_only = SlackThread {
            author: None,
            ..named
        };
        assert_eq!(thread_identity_label(&channel_only), "#builder");

        let permalink_channel = SlackThread {
            channel: None,
            ..channel_only
        };
        assert_eq!(thread_identity_label(&permalink_channel), "#C123");
    }

    #[test]
    fn resolves_full_label_before_an_interrupted_turn_finishes() {
        let url = "https://tavus.slack.com/archives/C0BANBTAKLL/p1786584195616319";
        let mut directory = SlackDirectory::default();
        directory.add_channels(CHANNEL_CACHE);
        directory.add_users(USER_CACHE);

        let mut thread = extract_threads(url).remove(0);
        directory.enrich_thread(&mut thread);
        assert_eq!(thread.channel.as_deref(), Some("blackbird-bug-reports"));
        assert!(thread.author.is_none());

        // This is the leading portion of the conversations_replies result from
        // d559d27d. It has an ID instead of a readable channel and no permalink.
        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
            1786584195.616319,U09062D9XJP,ashish,Ashish,C0BANBTAKLL,,\"message\"";
        let metadata = directory.message_metadata(result);
        let matching = metadata
            .iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(thread.author.as_deref(), Some("Ashish"));
        assert!(display_label(&thread).ends_with("#blackbird-bug-reports · Ashish"));
    }

    #[test]
    fn learns_channel_and_author_from_search_results_without_a_cache() {
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let mut thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,\"message\"";

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(thread.channel.as_deref(), Some("dogfood"));
        assert_eq!(thread.author.as_deref(), Some("Jared Vishno"));
    }

    #[test]
    fn metadata_seen_before_a_permalink_enriches_it_later() {
        let url = "https://tavus.slack.com/archives/C0AGPSMKQ3Z/p1786679090037079";
        let mut directory = SlackDirectory::default();
        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n\
            1786679090.037079,U0AEVB0102V,mango,Mango,C0AGPSMKQ3Z (#pr-reviews),,\"message\"";

        directory.message_metadata(result);
        let mut thread = extract_threads(url).remove(0);
        directory.enrich_thread(&mut thread);

        assert_eq!(thread.channel.as_deref(), Some("pr-reviews"));
        assert_eq!(thread.author.as_deref(), Some("Mango"));
    }

    #[test]
    fn reads_slack_rows_wrapped_in_codex_json_output() {
        let url = "https://tavus.slack.com/archives/C0B0CJXV0NL/p1786632730606179";
        let mut thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        let result = r#"{"slack":{"content":[{"type":"text","text":"MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text\n1786632730.606179,U096KSH9HEY,geoff,Geoff Barnes,C0B0CJXV0NL (#project-blackbird),,\"message\""}]}}"#;

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(thread.channel.as_deref(), Some("project-blackbird"));
        assert_eq!(thread.author.as_deref(), Some("Geoff Barnes"));
    }
}
