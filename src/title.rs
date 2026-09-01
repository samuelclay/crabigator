//! Session title helpers.
//!
//! Native OSC titles, recap-generated titles, and provider markers for
//! Claude, Codex, opencode, and Grok.

use crate::platforms::PlatformKind;
use crate::pr::SessionPr;

/// Marker prefixed to Codex session titles.
pub const CODEX_TITLE_MARKER: &str = "⟁  ";
/// Marker prefixed to Claude session titles.
pub const CLAUDE_TITLE_MARKER: &str = "ᛝ  ";
/// Marker prefixed to opencode session titles.
pub const OPENCODE_TITLE_MARKER: &str = "▣  ";
/// Marker prefixed to Grok session titles.
pub const GROK_TITLE_MARKER: &str = "↯  ";

pub(crate) const fn provider_title_marker(platform: PlatformKind) -> &'static str {
    match platform {
        PlatformKind::Claude => CLAUDE_TITLE_MARKER,
        PlatformKind::Codex => CODEX_TITLE_MARKER,
        PlatformKind::Opencode => OPENCODE_TITLE_MARKER,
        PlatformKind::Grok => GROK_TITLE_MARKER,
    }
}

/// Word budget for a generated title.
const MAX_TITLE_WORDS: usize = 6;

/// Normalize the recap model's title into a clean, bounded single line.
pub(crate) fn clean_generated_title(text: &str) -> String {
    for raw in text.lines() {
        let line = raw
            .trim()
            .trim_matches(|c: char| c == '"' || c == '\'' || c == '`')
            .trim_end_matches(['.', '!', ',', ':', ';'])
            .trim();
        if line.is_empty() {
            continue;
        }
        return line
            .split_whitespace()
            .take(MAX_TITLE_WORDS)
            .collect::<Vec<_>>()
            .join(" ");
    }
    String::new()
}

/// Remove a provider marker while accepting one- and two-space variants.
pub(crate) fn strip_provider_title_marker(title: &str) -> &str {
    for marker in [
        CODEX_TITLE_MARKER,
        CLAUDE_TITLE_MARKER,
        OPENCODE_TITLE_MARKER,
        GROK_TITLE_MARKER,
        "⟁ ",
        "ᛝ ",
        "▣ ",
        "↯ ",
        "✦  ",
        "✦ ",
    ] {
        if let Some(title) = title.strip_prefix(marker) {
            return title;
        }
    }
    title
}

/// Identify the provider encoded by a current or legacy title marker.
pub(crate) fn marked_title_platform(title: &str) -> Option<PlatformKind> {
    [
        PlatformKind::Codex,
        PlatformKind::Claude,
        PlatformKind::Grok,
        PlatformKind::Opencode,
    ]
    .into_iter()
    .find(|platform| title.starts_with(provider_title_marker(*platform).trim_end()))
}

/// Build a marker from the platforms that actually appear, in a stable
/// order: Claude, Codex, Grok, opencode. One platform returns its usual
/// marker; several concatenate the symbols and a trailing double space.
pub(crate) fn combined_provider_title_marker(
    platforms: impl IntoIterator<Item = PlatformKind>,
) -> String {
    let order = [
        PlatformKind::Claude,
        PlatformKind::Codex,
        PlatformKind::Grok,
        PlatformKind::Opencode,
    ];
    let mut seen = [false; 4];
    for platform in platforms {
        if let Some(index) = order.iter().position(|item| *item == platform) {
            seen[index] = true;
        }
    }
    let mut symbols = String::new();
    for (index, platform) in order.iter().enumerate() {
        if seen[index] {
            symbols.push_str(provider_title_marker(*platform).trim());
        }
    }
    if symbols.is_empty() {
        String::new()
    } else {
        format!("{symbols}  ")
    }
}

/// Prefix one normalized title with its provider marker.
pub(crate) fn mark_provider_title(platform: PlatformKind, title: &str) -> String {
    format!(
        "{}{}",
        provider_title_marker(platform),
        strip_provider_title_marker(title)
    )
}

/// Choose the title shown by Crabigator without allowing generated recap
/// titles to replace Claude Code's native OSC title.
pub(crate) fn display_title(
    platform: PlatformKind,
    native_title: Option<&str>,
    recap_title: Option<&str>,
) -> Option<String> {
    let native_title = native_title
        .map(strip_provider_title_marker)
        .filter(|title| !title.is_empty());
    let recap_title = recap_title
        .map(strip_provider_title_marker)
        .filter(|title| !title.is_empty());
    let title = match platform {
        PlatformKind::Claude => native_title,
        PlatformKind::Codex => recap_title.or(native_title),
        // opencode and Grok title their sessions natively, with the recap
        // title as fallback if the user turns terminal titles off.
        PlatformKind::Opencode | PlatformKind::Grok => native_title.or(recap_title),
    };
    title.map(|title| mark_provider_title(platform, title))
}

/// The two title levels shown for one session. A primary PR title becomes the
/// official title; the assistant's own title remains visible underneath it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SessionTitleHierarchy<'a> {
    pub(crate) pr_title: Option<&'a str>,
    pub(crate) generated_title: Option<&'a str>,
}

impl SessionTitleHierarchy<'_> {
    pub(crate) fn row_count(self) -> u16 {
        u16::from(self.pr_title.is_some()) + u16::from(self.generated_title.is_some())
    }
}

/// Choose the most recently mentioned enriched primary PR. The refresh time
/// and PR number make the result deterministic when mention times tie.
pub(crate) fn official_pr_title(prs: &[SessionPr]) -> Option<&str> {
    prs.iter()
        .filter(|pr| pr.primary && !pr.dismissed && !pr.title.trim().is_empty())
        .max_by_key(|pr| (pr.last_mentioned_at, pr.refreshed_at, pr.number))
        .map(|pr| pr.title.trim())
}

/// Build the display hierarchy while avoiding a repeated subtitle when the
/// generated title already matches the PR title.
pub(crate) fn session_title_hierarchy<'a>(
    prs: &'a [SessionPr],
    generated_title: Option<&'a str>,
) -> SessionTitleHierarchy<'a> {
    let pr_title = official_pr_title(prs);
    let generated_title = generated_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .filter(|title| {
            pr_title.is_none_or(|pr_title| strip_provider_title_marker(title).trim() != pr_title)
        });
    SessionTitleHierarchy {
        pr_title,
        generated_title,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_title_bounds_words_and_strips_decoration() {
        assert_eq!(
            clean_generated_title("\"Refactor The Title Rendering Path Now Please\"."),
            "Refactor The Title Rendering Path Now"
        );
        assert_eq!(
            clean_generated_title("```\nFix Branch Truncation\n```"),
            "Fix Branch Truncation"
        );
        assert_eq!(
            clean_generated_title("Add Dashboard Auth Flow"),
            "Add Dashboard Auth Flow"
        );
    }

    #[test]
    fn provider_markers_distinguish_codex_and_claude_titles() {
        assert_eq!(
            display_title(
                PlatformKind::Codex,
                Some("crabigator"),
                Some("Track Slack Threads")
            )
            .as_deref(),
            Some("⟁  Track Slack Threads")
        );
        assert_eq!(
            display_title(PlatformKind::Codex, Some("Codex CLI"), None).as_deref(),
            Some("⟁  Codex CLI")
        );
        assert_eq!(
            display_title(PlatformKind::Codex, Some("Codex CLI"), Some("⟁  ")).as_deref(),
            Some("⟁  Codex CLI")
        );
        assert_eq!(
            display_title(
                PlatformKind::Claude,
                Some("Native Claude Title"),
                Some("Ignored Recap Title")
            )
            .as_deref(),
            Some("ᛝ  Native Claude Title")
        );
    }

    #[test]
    fn provider_marker_stripping_accepts_both_providers_and_spacing() {
        assert_eq!(strip_provider_title_marker("⟁ Old Title"), "Old Title");
        assert_eq!(strip_provider_title_marker("⟁  New Title"), "New Title");
        assert_eq!(strip_provider_title_marker("ᛝ Old Title"), "Old Title");
        assert_eq!(strip_provider_title_marker("ᛝ  New Title"), "New Title");
        assert_eq!(
            marked_title_platform("⟁ Existing Title"),
            Some(PlatformKind::Codex)
        );
        assert_eq!(
            marked_title_platform("ᛝ  Existing Title"),
            Some(PlatformKind::Claude)
        );
        assert_eq!(
            strip_provider_title_marker("⟁Native Title"),
            "⟁Native Title"
        );
        assert_eq!(strip_provider_title_marker("Native Title"), "Native Title");
        assert_eq!(strip_provider_title_marker("↯  Grok Title"), "Grok Title");
        assert_eq!(strip_provider_title_marker("✦  Grok Title"), "Grok Title");
        assert_eq!(
            marked_title_platform("↯ Existing Title"),
            Some(PlatformKind::Grok)
        );
    }

    #[test]
    fn grok_prefers_native_title() {
        assert_eq!(
            display_title(
                PlatformKind::Grok,
                Some("Add Grok support"),
                Some("Ignored Recap")
            )
            .as_deref(),
            Some("↯  Add Grok support")
        );
        assert_eq!(
            display_title(PlatformKind::Grok, None, Some("Recap Title")).as_deref(),
            Some("↯  Recap Title")
        );
    }

    #[test]
    fn combined_markers_follow_stable_platform_order() {
        assert_eq!(
            combined_provider_title_marker([PlatformKind::Codex, PlatformKind::Claude]),
            "ᛝ⟁  "
        );
        assert_eq!(combined_provider_title_marker([PlatformKind::Grok]), "↯  ");
        assert_eq!(
            combined_provider_title_marker([
                PlatformKind::Opencode,
                PlatformKind::Grok,
                PlatformKind::Claude
            ]),
            "ᛝ↯▣  "
        );
    }

    #[test]
    fn latest_primary_pr_becomes_the_official_title() {
        let mut older = SessionPr::test_stub(8, "o", "repo");
        older.primary = true;
        older.title = "Older official title".to_string();
        older.last_mentioned_at = 100;

        let mut newer = SessionPr::test_stub(9, "o", "repo");
        newer.primary = true;
        newer.title = "Newer official title".to_string();
        newer.last_mentioned_at = 200;

        let mut dismissed = SessionPr::test_stub(10, "o", "repo");
        dismissed.primary = true;
        dismissed.dismissed = true;
        dismissed.title = "Dismissed title".to_string();
        dismissed.last_mentioned_at = 300;

        let prs = [older, newer, dismissed];
        let titles = session_title_hierarchy(&prs, Some("⟁  Automatic title"));
        assert_eq!(titles.pr_title, Some("Newer official title"));
        assert_eq!(titles.generated_title, Some("⟁  Automatic title"));
        assert_eq!(titles.row_count(), 2);
    }

    #[test]
    fn generated_title_is_the_fallback_and_is_not_repeated() {
        let no_prs = session_title_hierarchy(&[], Some("ᛝ  Automatic title"));
        assert_eq!(no_prs.pr_title, None);
        assert_eq!(no_prs.generated_title, Some("ᛝ  Automatic title"));

        let mut pr = SessionPr::test_stub(9, "o", "repo");
        pr.primary = true;
        pr.title = "Same title".to_string();
        let prs = [pr];
        let same = session_title_hierarchy(&prs, Some("⟁  Same title"));
        assert_eq!(same.pr_title, Some("Same title"));
        assert_eq!(same.generated_title, None);
        assert_eq!(same.row_count(), 1);
    }
}
