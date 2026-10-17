// Recap card + history widget styling. The recap card sits inside the
// session body, directly under the terminal screen, so it reads as a
// "what just happened" handoff for the live PTY view above it.
export const recapCss = `
/* Section visibility toggles (Style popover → Visible Sections).
   !important overrides the inline display the widgets set from live data. */
body.hide-section-recap .session-recap,
body.hide-section-recap .session-recap-history { display: none !important; }
body.hide-section-prs .session-prs { display: none !important; }

.session-recap {
    display: grid;
    grid-template-columns: auto 1fr auto;
    grid-template-rows: auto auto;
    grid-template-areas:
        "status headline meta"
        ".      bullets  bullets";
    column-gap: 10px;
    row-gap: 4px;
    padding: 10px 16px 12px;
    background: var(--bg-card);
    cursor: pointer;
    position: relative;
    transition: background 0.15s ease;
    max-height: 92px;
    overflow: hidden;
}
@media (hover: hover) and (pointer: fine) {
.session-recap:hover {
    background: var(--bg-surface);
}
}
.session-recap.expanded {
    max-height: none;
    overflow: visible;
}
.session-recap.empty,
.session-recap.disabled,
.session-recap.waiting:not(.has-content) {
    display: none;
}

.session-recap-status {
    grid-area: status;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-ui);
    font-size: 10px;
    text-transform: none;
    letter-spacing: 0;
    white-space: nowrap;
    padding-top: 1px;
}
.session-recap-status .rs-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    box-shadow: none;
    flex: 0 0 auto;
}
.session-recap-status .rs-label {
    font-weight: 600;
}

.session-recap-headline {
    grid-area: headline;
    color: var(--text-bright);
    font-size: 13px;
    line-height: 1.4;
    font-weight: 500;
    /* The headline can wrap to two lines before the card itself caps height. */
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
}
.session-recap.expanded .session-recap-headline {
    display: block;
    -webkit-line-clamp: unset;
}

.session-recap-bullets {
    grid-area: bullets;
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--text-mid);
    font-size: 11.5px;
    line-height: 1.4;
    overflow: hidden;
}
.session-recap-bullets .rb-line {
    position: relative;
    padding-left: 12px;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
}
.session-recap-bullets .rb-line::before {
    content: '·';
    position: absolute;
    left: 2px;
    color: var(--accent-cyan);
    opacity: 0.65;
}
.session-recap-bullets .rb-hint {
    color: var(--text-dim);
    font-style: italic;
}
.session-recap.expanded .session-recap-bullets .rb-line {
    display: block;
    -webkit-line-clamp: unset;
}

.session-recap-meta {
    grid-area: meta;
    align-self: start;
    font-family: var(--font-ui);
    font-size: 10px;
    color: var(--text-dim);
    white-space: nowrap;
}
.session-recap-meta .rd-age { color: var(--text-dim); }
.session-recap-meta .rd-delta { letter-spacing: 0.3px; }
.session-recap-meta .rd-add { color: var(--accent-green); }
.session-recap-meta .rd-del { color: var(--accent-red); }

/* Recap history sits directly under the recap card, outside Git/Changes collapse. */
.session-recap-history {
    padding: 12px 16px 14px;
    background: var(--bg-card);
    font-family: var(--font-ui);
    font-size: 11px;
}
.recap-history-widget .widget-title { display: flex; justify-content: space-between; align-items: baseline; }
.recap-history-widget .recap-history-count {
    color: var(--text-dim);
    font-size: 11px;
    font-family: var(--font-ui);
}
.recap-history-widget .recaps-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 320px;
    overflow-y: auto;
    padding-right: 4px;
}
.recap-history-widget .recap-entry {
    padding: 8px 10px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-left: 2px solid rgba(34, 211, 238, 0.55);
    border-radius: 4px;
}
.recap-history-widget .rh-headline {
    color: var(--text-bright);
    font-size: 12px;
    line-height: 1.4;
    margin-bottom: 4px;
    word-break: break-word;
}
.recap-history-widget .rh-bullet {
    color: var(--text-mid);
    font-size: 11px;
    line-height: 1.4;
    padding-left: 10px;
    position: relative;
    word-break: break-word;
}
.recap-history-widget .rh-bullet::before {
    content: '·';
    position: absolute;
    left: 0;
    color: var(--accent-cyan);
    opacity: 0.55;
}
.recap-history-widget .rh-delta {
    margin-top: 4px;
    font-family: var(--font-ui);
    font-size: 10px;
    color: var(--text-dim);
}
.recap-history-widget .rh-delta .rd-add { color: var(--accent-green); }
.recap-history-widget .rh-delta .rd-del { color: var(--accent-red); }

/* Narrow cards: the status label ("Latest recap" / "Previous recap") sits
   above the headline instead of beside it — there isn't enough width on a
   phone for a side-by-side layout. Meta shares the status row, right-aligned. */
@media (max-width: 720px) {
    .session-recap {
        grid-template-columns: 1fr auto;
        grid-template-areas:
            "status   meta"
            "headline headline"
            "bullets  bullets";
        max-height: 128px;
    }
    .session-recap.expanded {
        max-height: none;
    }
    .session-recap-meta { justify-self: end; padding-top: 2px; }
}

/* PR list sits directly under the recap card: every PR created or updated in
   this session, linked to GitHub, with branch + live diff stats. */
.session-prs {
    padding: 10px 16px 12px;
    background: var(--bg-card);
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
}
/* Header is its own full-width tap target. The padding/negative-margin pair
   grows the touch area into the surrounding whitespace without moving
   anything, so a tap near the title can't land on a PR row's controls. */
.session-prs .pr-list-title {
    font-family: var(--font-ui);
    font-size: 10px;
    text-transform: none;
    letter-spacing: 0;
    color: #c4a7f7;
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    padding: 6px 0;
    margin: -6px 0;
    -webkit-tap-highlight-color: transparent;
}
@media (hover: hover) and (pointer: fine) {
.session-prs .pr-list-title:hover { color: #d6bffb; }
}
.session-prs .pr-list-chevron {
    display: inline-block;
    width: 12px;
    color: #a371f7;
    font-size: 9px;
}
.session-prs .pr-list-count {
    color: var(--text-dim);
    margin-left: 6px;
    text-transform: none;
    letter-spacing: 0;
}
/* Hidden-secondaries hint next to the count while collapsed. */
.session-prs .pr-list-more {
    opacity: 0.75;
}
.session-prs .pr-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-left: 2px solid rgba(163, 113, 247, 0.55);
    border-radius: 4px;
}
/* Secondary PRs recede so the primary being worked on stands out. */
.session-prs .pr-row.pr-secondary {
    opacity: 0.7;
    border-left-color: rgba(110, 118, 129, 0.55);
}
@media (hover: hover) and (pointer: fine) {
.session-prs .pr-row.pr-secondary:hover { opacity: 1; }
}
.session-prs .pr-row-top {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}
.session-prs .pr-badge {
    font-family: var(--font-ui);
    font-size: 9px;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 700;
    padding: 1px 5px;
    border: 1px solid currentColor;
    border-radius: 3px;
    flex: 0 0 auto;
}
/* Primary/secondary toggle: ★ purple for the PR being worked on, ☆ gray for
   the rest. Click flips the disposition for the whole device group. */
.session-prs .pr-primary-toggle {
    font-size: 12px;
    cursor: pointer;
    flex: 0 0 auto;
    user-select: none;
}
.session-prs .pr-primary-toggle.primary { color: #c4a7f7; }
.session-prs .pr-primary-toggle.secondary { color: #6e7681; }
@media (hover: hover) and (pointer: fine) {
.session-prs .pr-primary-toggle:hover { color: #d6bffb; }
}
/* Promote/demote (↑/↓) flips primary and secondary; dismiss (✕) removes the
   PR from every list in the group. */
.session-prs .pr-flip, .session-prs .pr-dismiss {
    font-size: 10px;
    color: #6e7681;
    opacity: 0.55;
    cursor: pointer;
    user-select: none;
}
@media (hover: hover) and (pointer: fine) {
.session-prs .pr-flip:hover { opacity: 1; color: #d6bffb; }
}
@media (hover: hover) and (pointer: fine) {
.session-prs .pr-dismiss:hover { opacity: 1; color: #f85149; }
}
.session-prs .pr-link {
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    color: #c4a7f7;
    text-decoration: none;
    white-space: nowrap;
}
/* Secondary rows recede so primaries read first. */
.session-prs .pr-row:has(.pr-primary-toggle.secondary) .pr-link { color: #8b949e; }
@media (hover: hover) and (pointer: fine) {
.session-prs .pr-link:hover {
    text-decoration: underline;
    color: #d6bffb;
}
}
.session-prs .pr-diff {
    font-family: var(--font-ui);
    font-size: 10px;
    margin-left: auto;
    white-space: nowrap;
    text-decoration: none;
}
/* Diff and CI badges are links when GitHub gave us somewhere to go; the
   underline stays on hover so the row reads as a table at rest. */
@media (hover: hover) and (pointer: fine) {
a.pr-diff:hover,
a.pr-ci:hover,
a.pr-comments:hover { text-decoration: underline; }
}
.session-prs .pr-diff .rd-add { color: var(--accent-green); }
.session-prs .pr-diff .rd-del { color: var(--accent-red); }
.session-prs .pr-diff .pr-files { color: var(--text-dim); margin-left: 4px; }

/* Right-hand status cluster: state, CI, merge cleanliness. */
.session-prs .pr-status {
    margin-left: 12px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
    flex: 0 0 auto;
}
.session-prs .pr-ci,
.session-prs .pr-comments,
.session-prs .pr-review,
.session-prs .pr-merge {
    font-family: var(--font-ui);
    font-size: 10px;
    font-weight: 600;
    white-space: nowrap;
    text-decoration: none;
}
/* Unresolved review threads — orange, distinct from the yellow of a pending
   check or a behind branch, since this one is waiting on a person. */
.session-prs .pr-comments { color: #f0883e; }
.session-prs .pr-ci.pass { color: var(--accent-green); }
.session-prs .pr-ci.fail { color: var(--accent-red); }
.session-prs .pr-ci.pending { color: #d29922; }
.session-prs .pr-merge.clean { color: var(--accent-green); }
.session-prs .pr-merge.conflict { color: var(--accent-red); }
.session-prs .pr-merge.behind { color: #d29922; }
/* Review approval glyph: green approved, red changes requested, orange for
   an approval dismissed by new commits, dim while still awaiting review. */
.session-prs .pr-review.approved { color: var(--accent-green); }
.session-prs .pr-review.changes { color: var(--accent-red); }
.session-prs .pr-review.dismissed { color: #f0883e; }
.session-prs .pr-review.waiting { color: #6e7681; }
.session-prs .pr-title {
    color: var(--text-mid);
    font-size: 11.5px;
    line-height: 1.35;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
}
.session-prs .pr-row-bottom {
    display: flex;
    align-items: center;
    gap: 6px;
}
.session-prs .pr-branch {
    font-family: var(--font-ui);
    font-size: 10px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
}
`;
