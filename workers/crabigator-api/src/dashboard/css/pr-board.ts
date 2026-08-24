// Dashboard CSS - cross-session PR board
// Mirrors the CLI board's terminal look: xterm palette hexes, aligned
// activity/status columns, recency section bars. Never scrolls horizontally —
// the title column shrinks and narrow screens collapse into wrapped lines.
export const prBoardCss = `
/* Header toggle button, styled like its sessions-btn neighbor */
.pr-board-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: var(--bg-tertiary, #21262d);
    border: 1px solid var(--border, #30363d);
    border-radius: 6px;
    color: var(--text-secondary, #8b949e);
    font-size: 12px;
    font-weight: 600;
    padding: 5px 10px;
    cursor: pointer;
}
.pr-board-btn:hover { color: var(--text-primary, #c9d1d9); border-color: #8b949e; }
.pr-board-btn.active { color: #c4a7f7; border-color: #a371f7; }
.pr-board-btn-icon { color: #a371f7; }

/* The board pane replaces the session grid while open */
.pr-board {
    /* The board owns the viewport below the fixed header so its rows scroll
       inside it and the quick look pane can hold the bottom half. */
    height: calc(100vh - var(--header-height, 67px));
    height: calc(100dvh - var(--header-height, 67px));
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 14px 18px 14px;
    overflow: hidden;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    font-size: 11px;
    line-height: 1.6;
    color: #8a8a8a;
}
.pr-board[hidden] { display: none !important; }

/* Header: title, counts, and the CLI's s/r/a/+- controls as chips */
.prb-head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-bottom: 14px;
}
.prb-hdr { color: #af87ff; font-weight: 700; font-size: 13px; white-space: nowrap; }
.prb-counts { color: #585858; white-space: nowrap; }
.prb-ctl { color: #585858; cursor: pointer; user-select: none; white-space: nowrap; }
.prb-ctl:hover { color: #8a8a8a; }
.prb-ctl.active { color: #ffd700; }
.prb-ctl u { text-underline-offset: 2px; }
.prb-step { padding: 0 2px; }
.prb-step:hover { color: #ffd700; }
.prb-keys { color: #585858; white-space: nowrap; }
.prb-keys u { text-underline-offset: 2px; }
.prb-searchwrap { margin-left: auto; display: flex; align-items: center; gap: 8px; }
#prb-search {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border, #30363d);
    border-radius: 4px;
    color: var(--text-bright, #c9d1d9);
    font: inherit;
    padding: 2px 8px;
    width: 170px;
}
#prb-search:focus { outline: none; border-color: #ffd700; }
#prb-matches { color: #ffd700; white-space: nowrap; }

/* Shared columns: title stretches and truncates, activity and GitHub status
   stay compact and right-aligned across every row. */
.prb-body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content max-content;
    column-gap: 16px;
    align-content: start;
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-bottom: 34px;
}
.prb-bucket {
    grid-column: 1 / -1;
    margin: 14px 0 4px;
    padding: 1px 8px;
    font-weight: 700;
    border-radius: 3px;
}
.prb-bucket:first-child { margin-top: 0; }
.prb-repo { grid-column: 1 / -1; color: #ffd700; font-weight: 700; margin: 6px 0 2px; }
.prb-empty { grid-column: 1 / -1; color: #8a8a8a; padding: 16px 0; }

.prb-row {
    display: grid;
    grid-template-columns: subgrid;
    grid-column: 1 / -1;
    padding: 2px 4px;
    border-radius: 4px;
}
.prb-row:hover { background: rgba(255, 255, 255, 0.035); }
/* Secondary PRs recede so the primary being worked on stands out. */
.prb-row.prb-secondary { opacity: 0.75; }
.prb-row.prb-secondary:hover { opacity: 1; }
/* Rows whose sessions have all ended dim like the CLI's stale rows. */
.prb-row.prb-stale { opacity: 0.55; }
.prb-row.prb-stale:hover { opacity: 0.8; }

/* Every line inside a row rides the same three columns */
.prb-l1, .prb-l2, .prb-dl, .prb-sub {
    display: grid;
    grid-template-columns: subgrid;
    grid-column: 1 / -1;
    align-items: baseline;
}
.prb-l1-left { display: flex; align-items: baseline; gap: 7px; min-width: 0; }
.prb-star { cursor: pointer; flex-shrink: 0; }
.prb-star.primary { color: #af87ff; }
.prb-star.secondary { color: #6c6c6c; }
.prb-star:hover { color: #d6bffb; }
.prb-diamond { color: #af87ff; flex-shrink: 0; }
.prb-ident {
    color: #af87ff;
    font-weight: 600;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
a.prb-ident:hover { text-decoration: underline; }
.prb-secondary .prb-ident { color: #6c6c6c; }
.prb-branch {
    color: #585858;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.prb-activity { white-space: nowrap; justify-self: end; }
/* The activity cell of a live session opens the quick look pane. */
.prb-peek-open { cursor: pointer; border-radius: 3px; }
.prb-peek-open:hover { background: rgba(0, 215, 255, 0.12); }
/* The selection band: the ↑↓ cursor, matching the CLI's highlighted row. */
.prb-row.prb-sel { background: rgba(0, 215, 255, 0.14); }
.prb-row.prb-sel:hover { background: rgba(0, 215, 255, 0.18); }
/* A session row's uncommitted diff and file count, in the PR columns. */
.prb-wsdiff, .prb-wsfiles { white-space: nowrap; flex-shrink: 0; }
.prb-wsfiles { color: #585858; }
.prb-status {
    white-space: nowrap;
    justify-self: end;
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
}
.prb-status a, .prb-a { text-decoration: none; }
.prb-status a:hover, .prb-a:hover { text-decoration: underline; }
.prb-x { color: #585858; opacity: 0.55; cursor: pointer; }
.prb-x:hover { opacity: 1; color: #ff5f5f; }

/* Metadata row: generated session title + branch, diff + files right */
.prb-l2-left { display: flex; align-items: baseline; gap: 10px; min-width: 0; padding-left: 19px; }
.prb-gen { color: #5fafff; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.prb-l2-right { grid-column: 2 / -1; justify-self: end; white-space: nowrap; }
.prb-difffiles { color: #585858; text-decoration: none; }
a.prb-difffiles:hover { text-decoration: underline; }
.prb-add { color: #5fff5f; font-weight: 700; }
.prb-del { color: #ff5f5f; font-weight: 700; }

/* Recap detail rows (r toggle): ✦ note, ↪ headline, bullets, Slack links */
.prb-dl-left { display: flex; align-items: baseline; gap: 6px; min-width: 0; padding-left: 19px; }
.prb-dl-icon { flex-shrink: 0; }
.prb-dl-text { color: #8a8a8a; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.prb-dl-right { grid-column: 2 / -1; justify-self: end; white-space: nowrap; display: inline-flex; gap: 10px; }
.prb-slack { color: #00d7ff; text-decoration: none; }
.prb-slack:hover { text-decoration: underline; }

/* PR view (p): session sub-rows beneath each primary PR's header */
.prb-sub-left { display: flex; align-items: baseline; gap: 6px; min-width: 0; padding-left: 19px; }
.prb-sub-bullet { color: #585858; flex-shrink: 0; }
.prb-sub-title { color: #5fafff; white-space: nowrap; flex-shrink: 0; }
.prb-sub-headline { color: #8a8a8a; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
/* Ended sessions dim in whole so the live ones stand out. */
.prb-sub.prb-ended .prb-sub-title, .prb-sub.prb-ended .prb-sub-headline { color: #585858; }

/* Search previews: recap excerpts confirming why a row matched */
.prb-pv {
    grid-column: 1 / -1;
    padding-left: 19px;
    color: #585858;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.prb-pv-dir { color: #00d7ff; }
.prb-pv-text { color: #8a8a8a; }
.prb-pv-ctx { padding-left: 31px; }
.prb-pv-dim { color: #585858; }
.prb-pv-gap { color: #585858; }
.prb-mark { background: #ffd700; color: #000; border-radius: 2px; padding: 0 1px; }

/* Quick look pane: the selected session's live screen (or a window into its
   transcript) boxed in the bottom half, so it reads as a temporary read-only
   overlay rather than more board content. */
.prb-peek {
    flex: 0 0 50%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    margin-top: 8px;
    border: 1px solid #00d7ff;
    border-radius: 4px;
    overflow: hidden;
}
.prb-peek[hidden] { display: none !important; }
.prb-peek-top {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 2px 8px;
    border-bottom: 1px solid rgba(0, 215, 255, 0.35);
    color: #c9d1d9;
    white-space: nowrap;
}
.prb-peek-glyph { color: #00d7ff; }
.prb-peek-title { overflow: hidden; text-overflow: ellipsis; min-width: 0; }
.prb-peek-dir { color: #585858; }
.prb-peek-keys { margin-left: auto; color: #585858; }
.prb-peek-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: 2px 8px;
    white-space: pre;
    color: #c9d1d9;
}
.prb-peek-line { overflow: hidden; text-overflow: clip; }
.prb-peek-empty { color: #8a8a8a; }

/* Narrow screens: aligned columns collapse into wrapped lines so the board
   never scrolls horizontally. */
@media (max-width: 760px) {
    .pr-board { padding: 10px 10px 40px; }
    .prb-searchwrap { margin-left: 0; width: 100%; }
    #prb-search { flex: 1; width: auto; }
    .prb-body { display: block; }
    .prb-row { display: block; }
    .prb-l1, .prb-l2, .prb-dl, .prb-sub { display: flex; flex-wrap: wrap; align-items: baseline; gap: 2px 12px; }
    .prb-sub-left { flex: 1 1 auto; }
    .prb-l1-left { flex: 1 1 100%; }
    .prb-activity { margin-left: 19px; }
    .prb-status { margin-left: auto; }
    .prb-l2-right, .prb-dl-right { margin-left: auto; }
    .prb-keys { display: none; }
    .prb-peek { flex-basis: 45%; }
}
`;
