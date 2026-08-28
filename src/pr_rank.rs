//! Primary/secondary classification for session PRs.
//!
//! Distilled from an audit of 17 live sessions (78 PR-instances, 22
//! hand-labeled primaries): a PR is primary when it was mentioned RECENTLY
//! and the session OWNS it — created it here, matches its branch or worktree,
//! or the user typed it into a prompt — or when it dominates the session's
//! discussion outright. Raw mention counts alone are noise: abandoned PRs can
//! out-mention the real primary several times over.

use std::collections::HashMap;

use crate::pr::SessionPr;

/// A user-made disposition for one PR, from the web dashboard, a TUI action
/// link, or an in-session statement ("PR #123 is the primary").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrDisposition {
    Primary,
    Secondary,
    /// Drop the PR from the lists its scope covers — it stays tracked so a
    /// re-mention can't resurrect it, but nothing renders it there.
    Dismissed,
}

/// The group's cloud-stored dispositions, keyed `owner/repo#number` and then
/// by the scope each was set under: `""` covers the whole group,
/// `session:<id>` one session, and `path:<cwd>` every session in one worktree
/// directory — sticky for future sessions started there.
#[derive(Clone, Debug, Default)]
pub struct ScopedOverrides {
    rows: HashMap<String, Vec<(String, PrDisposition)>>,
}

impl ScopedOverrides {
    pub fn insert(&mut self, key: String, scope_key: String, disposition: PrDisposition) {
        let scopes = self.rows.entry(key).or_default();
        match scopes.iter_mut().find(|(scope, _)| *scope == scope_key) {
            Some(entry) => entry.1 = disposition,
            None => scopes.push((scope_key, disposition)),
        }
    }

    /// The disposition one session sees for a PR: its own session scope beats
    /// its worktree-path scope beats the group-wide row.
    pub fn for_session(
        &self,
        key: &str,
        session_id: &str,
        cwd: &str,
    ) -> Option<PrDisposition> {
        let scopes = self.rows.get(key)?;
        let find = |wanted: String| {
            scopes
                .iter()
                .find(|(scope, _)| *scope == wanted)
                .map(|(_, disposition)| *disposition)
        };
        (!session_id.is_empty())
            .then(|| find(format!("session:{session_id}")))
            .flatten()
            .or_else(|| (!cwd.is_empty()).then(|| find(format!("path:{cwd}"))).flatten())
            .or_else(|| find(String::new()))
    }

    /// Only the group-wide row — for surfaces that stand for the PR itself
    /// rather than one session's view of it.
    pub fn group(&self, key: &str) -> Option<PrDisposition> {
        self.rows
            .get(key)?
            .iter()
            .find(|(scope, _)| scope.is_empty())
            .map(|(_, disposition)| *disposition)
    }

    /// Flatten to the single map one session's classifier consumes.
    pub fn session_map(&self, session_id: &str, cwd: &str) -> HashMap<String, PrDisposition> {
        self.rows
            .keys()
            .filter_map(|key| {
                self.for_session(key, session_id, cwd)
                    .map(|disposition| (key.clone(), disposition))
            })
            .collect()
    }
}

/// Group-wide rows only, the shape older callers and tests build directly.
impl From<HashMap<String, PrDisposition>> for ScopedOverrides {
    fn from(map: HashMap<String, PrDisposition>) -> Self {
        let mut overrides = Self::default();
        for (key, disposition) in map {
            overrides.insert(key, String::new(), disposition);
        }
        overrides
    }
}

/// Session facts the classifier needs beyond the PRs themselves.
#[derive(Clone, Debug, Default)]
pub struct RankContext {
    /// The session's current git branch (empty when unknown).
    pub current_branch: String,
    /// Basename of the session's working directory — worktrees are commonly
    /// named after the branch they host.
    pub worktree_dir: String,
    /// The platform's prompt counter; mention recency is judged in turns.
    pub prompt_count: u32,
    /// URLs the user pasted into prompts. Preview URLs embed branch names,
    /// which is sometimes the only tie between a session and its PR.
    pub prompt_urls: Vec<String>,
    /// In-session statements keyed by bare PR number.
    pub declared_numbers: HashMap<u64, PrDisposition>,
    /// In-session statements keyed by full PR URL.
    pub declared_urls: HashMap<String, PrDisposition>,
    /// Cloud-stored dispositions keyed `owner/repo#number`. Highest precedence.
    pub overrides: HashMap<String, PrDisposition>,
}

/// How many trailing turns count as "recent". A fifth of the session, but
/// never fewer than three turns so young sessions aren't all-or-nothing.
fn recent_window(prompt_count: u32) -> u32 {
    (prompt_count / 5).max(3)
}

fn is_recent(last_mention_prompt: u32, prompt_count: u32) -> bool {
    last_mention_prompt + recent_window(prompt_count) >= prompt_count
}

/// Hide an automatically secondary PR after this many prompts pass without
/// another mention. User dispositions and watch-list entries stay sticky.
const SECONDARY_DISMISS_AFTER_PROMPTS: u32 = 3;

fn auto_secondary_is_stale(pr: &SessionPr, ctx: &RankContext) -> bool {
    !pr.watched
        && ctx.prompt_count.saturating_sub(pr.last_mention_prompt)
            >= SECONDARY_DISMISS_AFTER_PROMPTS
}

/// Classify every PR in place. Returns true when any visible field changed.
pub fn classify(prs: &mut [SessionPr], ctx: &RankContext) -> bool {
    let mut changed = false;

    // Branch matching is sticky: a branch can stop matching when the session
    // hops worktrees, but the ownership it proved doesn't expire.
    for pr in prs.iter_mut() {
        if !pr.branch_matched && branch_matches(pr, ctx) {
            pr.branch_matched = true;
            changed = true;
        }
    }

    let total_mentions: u64 = prs.iter().map(|p| p.mentions).sum();
    // Created-here PRs, for the supersession veto: an eval/retry chain keeps
    // only its newest link as primary.
    let created: Vec<(String, u64, u64)> = prs
        .iter()
        .filter(|p| p.created_here)
        .map(|p| (repo_key(p), p.number, p.first_mentioned_at))
        .collect();

    let decisions: Vec<_> = prs
        .iter()
        .map(|pr| decide(pr, ctx, total_mentions, &created))
        .collect();
    for (pr, (primary, source, dismissed)) in prs.iter_mut().zip(decisions) {
        if pr.primary != primary || pr.primary_source != source || pr.dismissed != dismissed {
            pr.primary = primary;
            pr.primary_source = source.to_string();
            pr.dismissed = dismissed;
            changed = true;
        }
    }
    changed
}

fn repo_key(pr: &SessionPr) -> String {
    format!("{}/{}", pr.owner, pr.repo)
}

/// (primary, source, dismissed) for one PR, honoring precedence:
/// cloud override > in-session statement > automatic scoring.
fn decide(
    pr: &SessionPr,
    ctx: &RankContext,
    total_mentions: u64,
    created: &[(String, u64, u64)],
) -> (bool, &'static str, bool) {
    let key = format!("{}/{}#{}", pr.owner, pr.repo, pr.number);
    if let Some(disposition) = ctx.overrides.get(&key) {
        return apply_disposition(*disposition, "override");
    }
    if let Some(disposition) = ctx
        .declared_urls
        .get(&pr.url)
        .or_else(|| ctx.declared_numbers.get(&pr.number))
    {
        return apply_disposition(*disposition, "session");
    }
    let primary = auto_primary(pr, ctx, total_mentions, created);
    (
        primary,
        "auto",
        !primary && auto_secondary_is_stale(pr, ctx),
    )
}

fn apply_disposition(
    disposition: PrDisposition,
    source: &'static str,
) -> (bool, &'static str, bool) {
    match disposition {
        PrDisposition::Primary => (true, source, false),
        PrDisposition::Secondary => (false, source, false),
        PrDisposition::Dismissed => (false, source, true),
    }
}

/// A current-worktree PR stays primary unless it is closed. Other PRs use the
/// audited conjunction (F1 0.83): recent AND (ownership OR dominance), with a
/// supersession veto.
fn auto_primary(
    pr: &SessionPr,
    ctx: &RankContext,
    total_mentions: u64,
    created: &[(String, u64, u64)],
) -> bool {
    // Closed-not-merged means the approach was abandoned — the audit found a
    // closed PR with 2.7× the primary's mentions that the user had rejected.
    if pr.state == "CLOSED" {
        return false;
    }
    if attached_to_worktree(pr, &ctx.current_branch, &ctx.worktree_dir) {
        return true;
    }
    if is_superseded(pr, created) {
        return false;
    }
    if !is_recent(pr.last_mention_prompt, ctx.prompt_count) {
        return false;
    }
    let ownership = pr.user_mentions > 0 || pr.created_here || pr.branch_matched;
    // A strict majority of the session's discussion carries a PR nobody
    // "owns" — the sole-subject investigation session.
    let dominant = total_mentions > 0 && pr.mentions * 2 > total_mentions;
    ownership || dominant
}

/// A PR whose mentions stopped before a created-here sibling in the same repo
/// even existed has been replaced by it (eval rerun chains, rebased retries).
fn is_superseded(pr: &SessionPr, created: &[(String, u64, u64)]) -> bool {
    if pr.last_mentioned_at == 0 {
        return false;
    }
    let repo = repo_key(pr);
    created
        .iter()
        .any(|(sibling_repo, number, first_mentioned)| {
            *sibling_repo == repo && *number != pr.number && *first_mentioned > pr.last_mentioned_at
        })
}

/// Whether the PR's head branch ties it to this session: the checked-out
/// branch, the worktree directory named after it, or a URL the user pasted
/// that embeds it (preview deployments).
fn branch_matches(pr: &SessionPr, ctx: &RankContext) -> bool {
    if attached_to_worktree(pr, &ctx.current_branch, &ctx.worktree_dir) {
        return true;
    }
    if pr.branch.is_empty() {
        return false;
    }
    let branch = pr.branch.as_str();
    // `sam/pal-test-video-receive-cap` inside
    // `https://pal-test-video-receive-cap.tavus-preview.io/…`.
    let segment = branch.rsplit('/').next().unwrap_or(branch);
    [branch, segment]
        .iter()
        .filter(|candidate| candidate.len() >= 8)
        .any(|candidate| ctx.prompt_urls.iter().any(|url| url.contains(*candidate)))
}

/// Whether this PR is attached to the branch checked out in a session's
/// worktree. This is public within the crate so readers of older session
/// mirrors can derive the same automatic-primary result retroactively.
pub(crate) fn attached_to_worktree(
    pr: &SessionPr,
    current_branch: &str,
    worktree_dir: &str,
) -> bool {
    if pr.branch.is_empty() || is_default_branch(&pr.branch) {
        return false;
    }
    if pr.branch == current_branch {
        return true;
    }
    !worktree_dir.is_empty()
        && (pr.branch == worktree_dir || pr.branch.ends_with(&format!("/{worktree_dir}")))
}

fn is_default_branch(branch: &str) -> bool {
    matches!(branch, "main" | "master" | "develop")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pr::SessionPr;

    fn pr(number: u64, repo: &str) -> SessionPr {
        let mut pr = SessionPr::test_stub(number, "o", repo);
        pr.state = "OPEN".to_string();
        pr
    }

    fn ctx(prompt_count: u32) -> RankContext {
        RankContext {
            prompt_count,
            ..RankContext::default()
        }
    }

    /// A session sees its own scope first, then its worktree path, then the
    /// group row — and other sessions never see scoped rows at all.
    #[test]
    fn scoped_overrides_resolve_most_specific_first() {
        let key = "o/portal#9";
        let mut overrides = ScopedOverrides::default();
        overrides.insert(key.to_string(), String::new(), PrDisposition::Primary);
        overrides.insert(
            key.to_string(),
            "path:/w/tree".to_string(),
            PrDisposition::Secondary,
        );
        overrides.insert(
            key.to_string(),
            "session:abc".to_string(),
            PrDisposition::Dismissed,
        );

        assert_eq!(
            overrides.for_session(key, "abc", "/w/tree"),
            Some(PrDisposition::Dismissed),
            "own session beats path and group"
        );
        assert_eq!(
            overrides.for_session(key, "other", "/w/tree"),
            Some(PrDisposition::Secondary),
            "worktree path beats group"
        );
        assert_eq!(
            overrides.for_session(key, "other", "/elsewhere"),
            Some(PrDisposition::Primary),
            "unrelated sessions get only the group row"
        );
        assert_eq!(overrides.group(key), Some(PrDisposition::Primary));
        assert_eq!(overrides.for_session("o/portal#10", "abc", "/w/tree"), None);

        let map = overrides.session_map("other", "/elsewhere");
        assert_eq!(map.get(key), Some(&PrDisposition::Primary));
    }

    #[test]
    fn recent_ownership_is_primary() {
        let mut prs = vec![pr(500, "portal")];
        prs[0].created_here = true;
        prs[0].mentions = 5;
        prs[0].last_mentioned_at = 100;
        prs[0].last_mention_prompt = 10;
        assert!(classify(&mut prs, &ctx(10)));
        assert!(prs[0].primary);
        assert_eq!(prs[0].primary_source, "auto");
    }

    /// Audit sessions 3/5/6/11: investigations and bookkeeping have no primary.
    #[test]
    fn zero_primary_sessions_stay_zero() {
        let mut prs = vec![pr(500, "portal"), pr(501, "portal"), pr(502, "portal")];
        for (i, p) in prs.iter_mut().enumerate() {
            p.mentions = 5; // evidence citations, evenly spread
            p.last_mention_prompt = 2;
            p.last_mentioned_at = 100 + i as u64;
        }
        classify(&mut prs, &ctx(2));
        assert!(prs.iter().all(|p| !p.primary));
    }

    /// Audit session 14 (#4546): created here, most-mentioned, and abandoned —
    /// CLOSED must veto everything.
    #[test]
    fn closed_prs_are_never_auto_primary() {
        let mut prs = vec![pr(4546, "cvi"), pr(1079, "portal")];
        prs[0].created_here = true;
        prs[0].mentions = 76;
        prs[0].user_mentions = 1;
        prs[0].state = "CLOSED".to_string();
        prs[0].last_mention_prompt = 3;
        prs[0].last_mentioned_at = 300;
        prs[1].created_here = true;
        prs[1].mentions = 28;
        prs[1].last_mention_prompt = 3;
        prs[1].last_mentioned_at = 400;
        classify(&mut prs, &ctx(3));
        assert!(!prs[0].primary);
        assert!(prs[1].primary);
    }

    /// Audit session 15 (evals #31→#37): each rerun obsoletes the last; only
    /// the link still mentioned after the newest was created stays primary.
    #[test]
    fn superseded_created_here_chains_keep_only_the_newest() {
        let mut prs = vec![pr(31, "evals"), pr(37, "evals")];
        prs[0].created_here = true;
        prs[0].mentions = 52;
        prs[0].last_mentioned_at = 1_000;
        prs[0].last_mention_prompt = 48;
        prs[1].created_here = true;
        prs[1].mentions = 12;
        prs[1].first_mentioned_at = 2_000;
        prs[1].last_mentioned_at = 2_500;
        prs[1].last_mention_prompt = 50;
        classify(&mut prs, &ctx(52));
        assert!(!prs[0].primary, "obsoleted rerun must demote");
        assert!(prs[1].primary);
    }

    /// Audit session 1: a pivot demotes the early phase's PR once its mentions
    /// age out of the recent window.
    #[test]
    fn stale_mentions_demote_on_pivot() {
        let mut prs = vec![pr(2557, "rqh")];
        prs[0].created_here = true;
        prs[0].mentions = 441;
        prs[0].user_mentions = 2;
        prs[0].last_mentioned_at = 100;
        prs[0].last_mention_prompt = 4;
        let context = ctx(12);
        classify(&mut prs, &context);
        assert!(!prs[0].primary, "last mention at prompt 4 of 12 is stale");
    }

    /// Audit session 7: 100% of the discussion, but no ownership signal —
    /// dominance alone carries it.
    #[test]
    fn dominant_discussion_is_primary_without_ownership() {
        let mut prs = vec![pr(2569, "rqh"), pr(2570, "rqh")];
        prs[0].mentions = 48;
        prs[0].last_mentioned_at = 900;
        prs[0].last_mention_prompt = 2;
        prs[1].mentions = 2;
        prs[1].last_mentioned_at = 900;
        prs[1].last_mention_prompt = 2;
        classify(&mut prs, &ctx(2));
        assert!(prs[0].primary);
        assert!(!prs[1].primary);
    }

    /// Audit session 12 (#1080): the only tie is the branch name inside a
    /// pasted preview URL.
    #[test]
    fn branch_inside_pasted_url_counts_as_ownership() {
        let mut prs = vec![pr(1080, "portal")];
        prs[0].branch = "sam/pal-test-video-receive-cap".to_string();
        prs[0].mentions = 36;
        prs[0].last_mentioned_at = 500;
        prs[0].last_mention_prompt = 8;
        let mut context = ctx(8);
        context.prompt_urls =
            vec!["https://pal-test-video-receive-cap.tavus-preview.io/builder".to_string()];
        classify(&mut prs, &context);
        assert!(prs[0].branch_matched);
        assert!(prs[0].primary);
    }

    #[test]
    fn worktree_dir_matches_branch_suffix() {
        let mut prs = vec![pr(1078, "portal")];
        prs[0].branch = "sam/pal-maker-sol-fast".to_string();
        prs[0].mentions = 22;
        prs[0].last_mentioned_at = 500;
        prs[0].last_mention_prompt = 3;
        let mut context = ctx(3);
        context.worktree_dir = "pal-maker-sol-fast".to_string();
        classify(&mut prs, &context);
        assert!(prs[0].primary);
    }

    #[test]
    fn attached_worktree_pr_stays_primary_after_mentions_age_out() {
        let mut prs = vec![pr(1078, "portal")];
        prs[0].branch = "sam/pal-maker-sol-fast".to_string();
        prs[0].mentions = 1;
        prs[0].last_mention_prompt = 1;
        let mut context = ctx(20);
        context.current_branch = prs[0].branch.clone();

        classify(&mut prs, &context);

        assert!(prs[0].primary);
        assert_eq!(prs[0].primary_source, "auto");
    }

    #[test]
    fn auto_secondary_is_dismissed_on_third_unmentioned_prompt() {
        let mut prs = vec![pr(1078, "portal"), pr(1079, "portal")];
        prs[0].mentions = 1;
        prs[0].last_mention_prompt = 5;
        prs[1].mentions = 2;
        prs[1].last_mention_prompt = 7;

        classify(&mut prs, &ctx(7));
        assert!(!prs[0].primary);
        assert!(!prs[0].dismissed, "two missed prompts keep it visible");

        classify(&mut prs, &ctx(8));
        assert!(prs[0].dismissed, "the third missed prompt dismisses it");
    }

    #[test]
    fn a_new_mention_restores_an_auto_dismissed_secondary() {
        let mut prs = vec![pr(1078, "portal"), pr(1079, "portal")];
        prs[0].mentions = 1;
        prs[0].last_mention_prompt = 5;
        prs[1].mentions = 2;
        prs[1].last_mention_prompt = 8;

        classify(&mut prs, &ctx(8));
        assert!(prs[0].dismissed);

        prs[0].last_mention_prompt = 9;
        classify(&mut prs, &ctx(9));
        assert!(!prs[0].dismissed);
    }

    #[test]
    fn watched_and_explicit_secondary_prs_do_not_decay() {
        let mut watched = pr(1078, "portal");
        watched.watched = true;
        watched.last_mention_prompt = 1;

        let mut explicit = pr(1079, "portal");
        explicit.last_mention_prompt = 1;
        let mut context = ctx(20);
        context
            .declared_numbers
            .insert(explicit.number, PrDisposition::Secondary);

        let mut prs = vec![watched, explicit];
        classify(&mut prs, &context);

        assert!(!prs[0].dismissed, "watched PR stays visible");
        assert!(!prs[1].dismissed, "explicit secondary stays visible");
    }

    #[test]
    fn explicit_secondary_beats_attached_worktree() {
        let mut prs = vec![pr(1078, "portal")];
        prs[0].branch = "sam/pal-maker-sol-fast".to_string();
        let mut context = ctx(20);
        context.current_branch = prs[0].branch.clone();
        context
            .declared_numbers
            .insert(1078, PrDisposition::Secondary);

        classify(&mut prs, &context);

        assert!(!prs[0].primary);
        assert_eq!(prs[0].primary_source, "session");
    }

    #[test]
    fn session_statement_beats_auto_and_override_beats_statement() {
        let mut prs = vec![pr(2567, "rqh")];
        prs[0].mentions = 4; // too little for auto
        prs[0].last_mention_prompt = 3;
        prs[0].last_mentioned_at = 100;
        let mut context = ctx(3);
        context
            .declared_numbers
            .insert(2567, PrDisposition::Primary);
        classify(&mut prs, &context);
        assert!(prs[0].primary);
        assert_eq!(prs[0].primary_source, "session");

        context
            .overrides
            .insert("o/rqh#2567".to_string(), PrDisposition::Dismissed);
        classify(&mut prs, &context);
        assert!(!prs[0].primary);
        assert!(prs[0].dismissed);
        assert_eq!(prs[0].primary_source, "override");
    }

    #[test]
    fn matching_the_default_branch_is_not_ownership() {
        let mut prs = vec![pr(9, "portal"), pr(10, "portal")];
        for p in prs.iter_mut() {
            p.branch = "main".to_string();
            p.mentions = 1;
            p.last_mention_prompt = 1;
            p.last_mentioned_at = 50;
        }
        let mut context = ctx(1);
        context.current_branch = "main".to_string();
        classify(&mut prs, &context);
        assert!(prs.iter().all(|p| !p.branch_matched));
        assert!(prs.iter().all(|p| !p.primary));
    }
}
