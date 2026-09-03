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
    /// The message body on one line, cleaned of Slack markup, once a Slack
    /// tool result in this session has carried it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
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

/// A message header in the Slack plugin's thread/channel output:
/// "From: Ari (U043KK8KBT2)" followed by a Time line and
/// "Message TS: 1787765223.159079". The channel is not in the result (it was
/// the tool call's input), so these rows match threads by message TS alone.
fn slack_plugin_message_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^From:\s*([^\r\n(]+?)\s*\((U[A-Z0-9]+)\)\s*\r?\nTime:[^\r\n]*\r?\nMessage TS:\s*(\d{10})\.(\d+)"#,
        )
        .expect("valid Slack plugin message regex")
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
    text: Option<String>,
}

struct SlackMessageRow {
    message_id: String,
    user_id: String,
    username: String,
    real_name: String,
    channel_id: String,
    channel: Option<String>,
    /// Raw message body as Slack returned it, still carrying Slack markup.
    text: Option<String>,
}

/// Longest message snippet kept per thread. The widgets truncate further to
/// their own width; this only bounds the mirror and cloud payloads.
const MESSAGE_SNIPPET_MAX_CHARS: usize = 400;

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
        // Plugin-format rows carry no channel, so they index under an empty
        // channel ID and match on the message TS alone.
        if let Some(metadata) = self
            .messages
            .get(&(channel_id.to_string(), message_id.to_string()))
            .or_else(|| self.messages.get(&(String::new(), message_id.to_string())))
        {
            metadata.apply_to(thread);
        }
    }

    /// Read exact channel and poster metadata from Slack MCP message results.
    /// This runs while the turn is active, so an interrupted turn still keeps
    /// the metadata already returned by Slack.
    pub fn message_metadata(&mut self, text: &str) -> Vec<SlackMessageMetadata> {
        let mut rows = parse_slack_message_rows(text);
        if text.contains("\\n") || text.contains("\\\"") {
            // Tool results sit inside JSON strings in the transcript, so
            // newlines and quotes arrive escaped. Rows parse either way, but
            // quoted CSV text only parses once the quotes are real.
            let decoded = text
                .replace("\\r\\n", "\n")
                .replace("\\n", "\n")
                .replace("\\\"", "\"");
            rows.extend(parse_slack_message_rows(&decoded));
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
                let text = row
                    .text
                    .as_deref()
                    .and_then(|text| clean_message_text(text, &self.users, &self.channels));
                SlackMessageMetadata {
                    message_id: row.message_id,
                    channel_id: row.channel_id,
                    channel,
                    author,
                    text,
                }
            })
            .collect::<Vec<_>>();
        for item in &metadata {
            let key = (item.channel_id.clone(), item.message_id.clone());
            if let Some(existing) = self.messages.get_mut(&key) {
                update_field(&mut existing.channel, &item.channel);
                update_field(&mut existing.author, &item.author);
                update_field(&mut existing.text, &item.text);
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
    let mut rows: Vec<SlackMessageRow> = slack_message_row_re()
        .captures_iter(text)
        .filter_map(|captures| {
            let seconds = captures.get(1)?.as_str();
            let fraction = captures.get(2)?.as_str();
            let channel = captures
                .get(7)
                .map(|name| clean_channel_name(name.as_str()))
                .filter(|name| !name.is_empty());
            let rest = &text[captures.get(0)?.end()..];
            Some(SlackMessageRow {
                message_id: format!("{seconds}{fraction}"),
                user_id: captures.get(3)?.as_str().to_string(),
                username: captures.get(4)?.as_str().trim().to_string(),
                real_name: captures.get(5)?.as_str().trim().to_string(),
                channel_id: captures.get(6)?.as_str().to_string(),
                channel,
                text: csv_message_text(rest),
            })
        })
        .collect();
    // The Slack plugin prints message headers instead of CSV. Its results
    // carry no channel (that was the call's input), so the row's channel ID
    // stays empty and threads match on the message TS alone.
    rows.extend(
        slack_plugin_message_re()
            .captures_iter(text)
            .filter_map(|captures| {
                let seconds = captures.get(3)?.as_str();
                let fraction = captures.get(4)?.as_str();
                let rest = &text[captures.get(0)?.end()..];
                Some(SlackMessageRow {
                    message_id: format!("{seconds}{fraction}"),
                    user_id: captures.get(2)?.as_str().to_string(),
                    username: String::new(),
                    real_name: captures.get(1)?.as_str().trim().to_string(),
                    channel_id: String::new(),
                    channel: None,
                    text: plugin_message_text(rest),
                })
            }),
    );
    rows
}

/// The `Text` column that follows a CSV row's channel field. `rest` starts at
/// the `ThreadTs` column: `<thread ts>,<text>,<time>,...`. Text with commas,
/// quotes, or newlines arrives CSV-quoted; a leading backslash means the
/// quotes are still JSON-escaped, so this pass leaves the text to the decoded
/// pass rather than cutting it at the first comma.
fn csv_message_text(rest: &str) -> Option<String> {
    let thread_ts_end = rest.find([',', '\r', '\n'])?;
    if !rest[thread_ts_end..].starts_with(',') {
        return None;
    }
    let field = &rest[thread_ts_end + 1..];
    if field.starts_with('\\') {
        return None;
    }
    let text = match field.strip_prefix('"') {
        Some(quoted) => quoted_csv_field(quoted)?,
        None => field
            .split([',', '\r', '\n'])
            .next()
            .unwrap_or(field)
            .to_string(),
    };
    nonempty(text)
}

/// A CSV field's contents, starting just past its opening quote, with each
/// doubled quote turned back into one. `None` when the field never closes,
/// which means the tool result was cut short and the text is not fully known.
fn quoted_csv_field(quoted: &str) -> Option<String> {
    let mut text = String::new();
    let mut chars = quoted.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            text.push(c);
            continue;
        }
        if chars.peek() != Some(&'"') {
            return Some(text);
        }
        chars.next();
        text.push('"');
    }
    None
}

/// The message body printed under a Slack plugin message header, up to the
/// first blank line, the next header, or the closing quote of the JSON string
/// that wraps a tool result in the transcript.
fn plugin_message_text(rest: &str) -> Option<String> {
    /// Line starts that end the message body: a separator rule, or the
    /// header of the next message.
    const BODY_ENDINGS: [&str; 4] = ["===", "---", "From:", "Message TS:"];

    let (_, body) = rest.split_once('\n')?;
    let mut text = body
        .lines()
        .map(str::trim_end)
        .take_while(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty()
                && !BODY_ENDINGS
                    .iter()
                    .any(|ending| trimmed.starts_with(ending))
                && !json_closer_line_re().is_match(trimmed)
        })
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(closer) = trailing_json_closer_re().find(&text) {
        text.truncate(closer.start());
    }
    nonempty(text)
}

/// A line that begins with the closing quote of the JSON string wrapping a
/// tool result, followed by the brackets that close its containers.
fn json_closer_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"^"[\]},]"#).expect("valid JSON closer line regex"))
}

/// The same closing quote and brackets when they end the last message line.
fn trailing_json_closer_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#""[\]}]+\s*$"#).expect("valid trailing JSON closer regex"))
}

/// `<@U…>`, `<#C…|name>`, `<!here>`, `<url|label>`, or `<url>`.
fn slack_markup_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"<([@#!]?)([^<>|]*)(?:\|([^<>]*))?>").expect("valid Slack markup regex")
    })
}

/// One display line of a Slack message: mentions and links rendered the way
/// Slack shows them, entities decoded, whitespace collapsed, length bounded.
fn clean_message_text(
    text: &str,
    users: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) -> Option<String> {
    let rendered = slack_markup_re().replace_all(text, |captures: &regex::Captures| {
        let kind = captures.get(1).map_or("", |m| m.as_str());
        let target = captures.get(2).map_or("", |m| m.as_str());
        let label = captures
            .get(3)
            .map(|m| m.as_str())
            .filter(|label| !label.is_empty());
        match kind {
            "@" => format!(
                "@{}",
                label
                    .or_else(|| users.get(target).map(String::as_str))
                    .unwrap_or(target)
            ),
            "#" => format!(
                "#{}",
                label
                    .or_else(|| channels.get(target).map(String::as_str))
                    .unwrap_or(target)
                    .trim_start_matches('#')
            ),
            // <!here>, <!channel>, or a user group like <!subteam^S123|@eng>.
            "!" => match label {
                Some(label) => format!("@{}", label.trim_start_matches('@')),
                None => format!(
                    "@{}",
                    target
                        .split_once('^')
                        .map_or(target, |(command, _)| command)
                ),
            },
            _ => label.unwrap_or(target).to_string(),
        }
    });
    let decoded = rendered
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    let collapsed = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    nonempty(collapsed.chars().take(MESSAGE_SNIPPET_MAX_CHARS).collect())
}

impl SlackMessageMetadata {
    pub(crate) fn matches(&self, thread: &SlackThread) -> bool {
        slack_message_id(&thread.url) == Some(self.message_id.as_str())
            && (self.channel_id.is_empty()
                || slack_channel_id(&thread.url) == Some(self.channel_id.as_str()))
    }

    pub(crate) fn apply_to(&self, thread: &mut SlackThread) -> bool {
        let channel = update_field(&mut thread.channel, &self.channel);
        let author = update_field(&mut thread.author, &self.author);
        let text = update_field(&mut thread.text, &self.text);
        channel || author || text
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

/// Replace `target` when Slack returned a value for the field and it differs
/// from what is already stored. Returns whether the field changed.
fn update_field(target: &mut Option<String>, value: &Option<String>) -> bool {
    match value {
        Some(value) if target.as_ref() != Some(value) => {
            *target = Some(value.clone());
            true
        }
        _ => false,
    }
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
            text: None,
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

/// PR-board label: channel, author, and post time, without repeating the word
/// "Slack". The permalink always supplies at least a channel ID.
pub(crate) fn thread_identity_label(thread: &SlackThread) -> String {
    let channel = thread
        .channel
        .as_deref()
        .filter(|channel| !channel.is_empty())
        .or_else(|| slack_channel_id(&thread.url))
        .map(|channel| format!("#{}", channel.trim_start_matches('#')))
        .unwrap_or_else(|| "#thread".to_string());
    let mut parts = vec![channel];
    if let Some(author) = thread.author.as_deref().filter(|author| !author.is_empty()) {
        parts.push(author.to_string());
    }
    if thread.posted_at > 0 {
        parts.push(format_date(thread.posted_at, "%b %-d, %-I:%M %p"));
    }
    parts.join(" · ")
}

/// The channel a permalink belongs to, for grouping threads by channel. The
/// URL's channel ID is stable even while the readable name is still unknown.
pub(crate) fn channel_key(thread: &SlackThread) -> String {
    slack_channel_id(&thread.url)
        .or(thread.channel.as_deref())
        .unwrap_or(&thread.url)
        .to_string()
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
            text: None,
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
        assert_eq!(thread.text.as_deref(), Some("message"));
        assert!(display_label(&thread).ends_with("#blackbird-bug-reports · Ashish"));
    }

    #[test]
    fn keeps_the_message_text_from_a_transcript_csv_row() {
        // Verbatim shape of a conversations_replies result inside a Claude
        // Code transcript: JSON-escaped, unquoted text with no commas, and
        // more columns after Text.
        let url = "https://tavus.slack.com/archives/C0BANBTAKLL/p1788460617071009";
        let mut thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        let result = r#"{"type":"text","text":"MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text,Time,Permalink,Reactions,BotName,FileCount,AttachmentIDs,HasMedia,Cursor
1788460617.071009,U0AK7QDF8MU,elisa,Elisa Ding,C0BANBTAKLL (#blackbird-bug-reports),,It looks like phx4.5 faces have their voices displayed as Haiyao - can we get them renamed to face default (Lucas - default voice),2026-09-03T18:36:57Z,https://tavus.slack.com/archives/C0BANBTAKLL/p1788460617071009,,,0,,false,
"}"#;

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(thread.author.as_deref(), Some("Elisa Ding"));
        assert_eq!(
            thread.text.as_deref(),
            Some("It looks like phx4.5 faces have their voices displayed as Haiyao - can we get them renamed to face default (Lucas - default voice)")
        );
    }

    #[test]
    fn reads_quoted_csv_text_with_commas_quotes_and_newlines() {
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let mut thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        // JSON-escaped, as the transcript stores it: " for quotes and

        // for the newline inside the quoted Text column.
        let result = r#"{"text":"MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text,Time
1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,"Fix the build, please ""now""
second line",2026-08-10T10:00:00Z
"}"#;

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .rfind(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(
            thread.text.as_deref(),
            Some("Fix the build, please \"now\" second line")
        );

        // The same row already decoded (Codex logs and plain output).
        let decoded = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text,Time
\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),1786640000.000001,\"Fix the build, please \"\"now\"\"\nsecond line\",2026-08-10T10:00:00Z
";
        let mut fresh = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        let matching = directory
            .message_metadata(decoded)
            .into_iter()
            .find(|metadata| metadata.matches(&fresh))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut fresh));
        assert_eq!(
            fresh.text.as_deref(),
            Some("Fix the build, please \"now\" second line")
        );
    }

    #[test]
    fn renders_slack_markup_the_way_slack_shows_it() {
        let mut directory = SlackDirectory::default();
        directory.add_channels(CHANNEL_CACHE);
        directory.add_users(USER_CACHE);
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let mut thread = extract_threads(url).remove(0);
        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text,Time
\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,\"<@U09062D9XJP> can you check <https://example.com/x|this link> and <https://example.com/y> in <#C0BANBTAKLL> &amp; <#C0OTHER|random>? <!here> <!subteam^S123|@eng>   thanks\",2026-08-10T10:00:00Z
";

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(
            thread.text.as_deref(),
            Some("@Ashish can you check this link and https://example.com/y in #blackbird-bug-reports & #random? @here @eng thanks")
        );
    }

    #[test]
    fn caps_the_stored_snippet_length() {
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let mut thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        let long = "word ".repeat(200);
        let result = format!(
            "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text,Time
\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,{long},2026-08-10T10:00:00Z
"
        );

        let matching = directory
            .message_metadata(&result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        let stored = thread.text.as_deref().unwrap().chars().count();
        assert!(stored <= MESSAGE_SNIPPET_MAX_CHARS, "stored {stored} chars");
        assert!(
            stored > MESSAGE_SNIPPET_MAX_CHARS - 5,
            "stored {stored} chars"
        );
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
    fn reads_message_headers_from_slack_plugin_output() {
        let url = "https://tavus.slack.com/archives/D0AQWNX3GDT/p1787765223159079";
        let mut thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        // Verbatim shape of a slack_read_thread result inside a transcript:
        // JSON-escaped text with \n newlines and no channel field.
        let result = r#"{"messages":"=== THREAD PARENT MESSAGE ===\nFrom: Ari (U043KK8KBT2)\nTime: 2026-08-26 10:27:03 PDT\nMessage TS: 1787765223.159079\nHey Sam — sharing the review\n"}"#;

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert!(matching.apply_to(&mut thread));
        assert_eq!(thread.author.as_deref(), Some("Ari"));
        assert_eq!(thread.text.as_deref(), Some("Hey Sam — sharing the review"));
        // No channel in the result: the permalink's ID stays as the fallback.
        assert_eq!(thread.channel.as_deref(), Some("D0AQWNX3GDT"));

        // A later permalink to the same message enriches through the
        // channel-less index too.
        let mut fresh = extract_threads(url).remove(0);
        directory.enrich_thread(&mut fresh);
        assert_eq!(fresh.author.as_deref(), Some("Ari"));
        assert_eq!(fresh.text.as_deref(), Some("Hey Sam — sharing the review"));

        // Codex wraps the same payload in its own JSON, so the closing
        // brackets can share the last message line; a second paragraph and
        // the next message header stay out of the snippet.
        let wrapped = r#"{"slack":{"content":[{"type":"text","text":"=== THREAD PARENT MESSAGE ===\nFrom: Ari (U043KK8KBT2)\nTime: 2026-08-26 10:27:03 PDT\nMessage TS: 1787765223.159079\nFirst line\nstill first paragraph\n\nSecond paragraph\nFrom: Sam (U1)\nTime: x\nMessage TS: 1787765300.000001\nReply text"}]}}"#;
        let metadata = SlackDirectory::default().message_metadata(wrapped);
        let texts: Vec<Option<&str>> = metadata.iter().map(|m| m.text.as_deref()).collect();
        assert!(texts.contains(&Some("First line still first paragraph")));
        assert!(texts.contains(&Some("Reply text")));
    }

    #[test]
    fn a_cut_off_quoted_message_yields_no_snippet() {
        let url = "https://tavus.slack.com/archives/C06J3D25T4H/p1786640707322729";
        let thread = extract_threads(url).remove(0);
        let mut directory = SlackDirectory::default();
        let result = "MsgID,UserID,UserName,RealName,Channel,ThreadTs,Text,Time
\
            1786640707.322729,U067BG5GHT5,jared,Jared Vishno,C06J3D25T4H (#dogfood),,\"The result was truncated here…";

        let matching = directory
            .message_metadata(result)
            .into_iter()
            .find(|metadata| metadata.matches(&thread))
            .expect("linked Slack row");
        assert_eq!(matching.author.as_deref(), Some("Jared Vishno"));
        assert!(matching.text.is_none());
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
