// Dashboard CSS - cross-session PR board
// Keeps the CLI board's aligned
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
    color: var(--text-mid);
}
.pr-board[hidden] { display: none !important; }

/* Header: title, counts, and the CLI's s/p/r/a controls as chips */
.prb-head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 14px;
    margin-bottom: 14px;
}
.prb-hdr { color: var(--text-bright); font-family: var(--font-ui); font-weight: 600; font-size: 16px; white-space: nowrap; }
.prb-ctl { color: var(--text-dim); cursor: pointer; user-select: none; white-space: nowrap; }
.prb-ctl:hover { color: var(--text-mid); }
.prb-ctl.active { color: var(--accent-yellow); }
.prb-ctl u { text-underline-offset: 2px; }
.prb-keys { color: var(--text-dim); white-space: nowrap; }
.prb-keys u { text-underline-offset: 2px; }
.prb-addwrap { margin-left: auto; }
.prb-searchwrap { display: flex; align-items: center; gap: 8px; }
#prb-search,
#prb-add {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border, #30363d);
    border-radius: 4px;
    color: var(--text-bright, #c9d1d9);
    font: inherit;
    padding: 2px 8px;
    width: 170px;
}
#prb-search:focus, #prb-add:focus { outline: none; border-color: var(--accent-yellow); }
/* A watch input that didn't parse or post flashes red briefly. */
#prb-add.prb-add-error { border-color: var(--accent-red); }
#prb-matches { color: var(--accent-yellow); white-space: nowrap; }

/* Shared columns: title stretches and truncates, then the activity, the
   diff, the file count, and the GitHub status stay compact and
   right-aligned, so the stats line up across PR rows, session blocks, and
   workspace rows, like the CLI board. */
.prb-body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content max-content max-content max-content;
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
    padding: 4px 8px;
    font-weight: 600;
    border-radius: 3px;
}
.prb-bucket:first-child { margin-top: 0; }
.prb-repo { grid-column: 1 / -1; color: var(--accent-yellow); font-weight: 700; margin: 6px 0 2px; }
.prb-dot { color: var(--text-dim); font-weight: 400; }
.prb-machine { color: var(--accent-blue); }
.prb-empty { grid-column: 1 / -1; color: var(--text-mid); padding: 16px 0; }

.prb-row {
    display: grid;
    grid-template-columns: subgrid;
    grid-column: 1 / -1;
    padding: 2px 4px;
    border-radius: 4px;
}
.prb-row:hover { background: rgba(255, 255, 255, 0.035); }
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
/* A watch no session claims renders full purple, like a primary. */
.prb-star.primary, .prb-star.watched { color: var(--accent-purple); }
.prb-star.secondary { color: var(--text-dim); }
.prb-star:hover { color: #d6bffb; }
.prb-diamond { color: var(--accent-purple); flex-shrink: 0; }
/* A session without a PR carries its title in the same blue as the session
   sub-rows beneath a PR, so every session title reads alike. */
.prb-session-title {
    color: var(--accent-blue);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.prb-ident {
    color: var(--accent-purple);
    font-weight: 600;
    text-decoration: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
a.prb-ident:hover { text-decoration: underline; }
/* The PR's handle is bold and underlined so it stands out and reads as the link it is. */
.prb-num { font-weight: 700; text-decoration: underline; }
.prb-branch {
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.prb-activity { grid-column: 2; white-space: nowrap; justify-self: end; }
/* PR view's stats columns; both open the PR's Files-changed tab. */
.prb-diff { grid-column: 3; white-space: nowrap; text-decoration: none; }
.prb-files { grid-column: 4; white-space: nowrap; color: var(--text-dim); text-decoration: none; }
a.prb-diff:hover, a.prb-files:hover { text-decoration: underline; }
/* States that wait on the user: chevrons on a filled background. */
.prb-badge { color: #000; font-weight: 700; border-radius: 3px; padding: 0 1px; }
.prb-ask { background: #ff8700; }
.prb-perm { background: #ffd700; }
/* A badge or status cell that changed recently: bright purple when fresh,
   fading to a faint purple-gray, plain again after two minutes. Only the
   background glows; the cell keeps its normal text color. */
.prb-cool { border-radius: 3px; padding: 0 3px; margin: 0 -3px; }
/* The activity cell of a live session (or its PR-view sub-row title) opens
   the quick look pane. */
.prb-peek-open { cursor: pointer; border-radius: 3px; }
.prb-peek-open:hover { background: rgba(0, 215, 255, 0.12); }
/* The selection band: the ↑↓ cursor, matching the CLI's highlighted row. */
.prb-row.prb-sel { background: rgba(0, 215, 255, 0.14); }
.prb-row.prb-sel:hover { background: rgba(0, 215, 255, 0.18); }
/* The GitHub status cluster, anchored against the right edge. */
.prb-status {
    grid-column: -2;
    white-space: nowrap;
    justify-self: end;
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
}
.prb-status a, .prb-a { text-decoration: none; }
.prb-status a:hover, .prb-a:hover { text-decoration: underline; }
/* Promote/demote (↑/↓) and dismiss (✕) actions at the right edge. */
.prb-flip, .prb-x { color: var(--text-dim); cursor: pointer; }
.prb-flip:hover { opacity: 1; color: #d6bffb; }
.prb-x:hover { opacity: 1; color: var(--accent-red); }

/* Metadata row: the PR header's branch, indented under its title */
.prb-l2-left { display: flex; align-items: baseline; gap: 10px; min-width: 0; padding-left: 19px; }
.prb-add { color: var(--accent-green); font-weight: 700; }
.prb-del { color: var(--accent-red); font-weight: 700; }

/* Recap detail rows (r toggle): ✦ note, ↪ headline, bullets, Slack links */
.prb-dl-left { display: flex; align-items: baseline; gap: 6px; min-width: 0; padding-left: 19px; }
.prb-dl-icon { flex-shrink: 0; }
.prb-dl-text { color: var(--text-mid); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.prb-dl-right { grid-column: 2 / -1; justify-self: end; white-space: nowrap; display: inline-flex; gap: 10px; }
.prb-slack { color: var(--accent-cyan); text-decoration: none; }
.prb-slack:hover { text-decoration: underline; }

/* PR view (p): session sub-rows beneath each primary PR's header */
.prb-sub-left { display: flex; align-items: baseline; gap: 6px; min-width: 0; padding-left: 19px; }
.prb-sub-bullet { color: var(--text-dim); flex-shrink: 0; }
.prb-sub-left .session-mark, .prb-l1-left .session-mark { margin-right: 2px; }
.prb-sub-title {
    color: var(--accent-blue);
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.prb-sub-headline { color: var(--text-mid); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
/* Ended sessions dim in whole so the live ones stand out. */
.prb-sub.prb-ended .prb-sub-title, .prb-sub.prb-ended .prb-sub-headline { color: var(--text-dim); }
/* A session block's PR sub-rows: the PR-view anatomy indented beneath the
   session header. */
.prb-pr-sub { padding-left: 19px; }

/* Search previews: recap excerpts confirming why a row matched */
.prb-pv {
    grid-column: 1 / -1;
    padding-left: 19px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.prb-pv-dir { color: var(--accent-cyan); }
.prb-pv-text { color: var(--text-mid); }
.prb-pv-ctx { padding-left: 31px; }
.prb-pv-dim { color: var(--text-dim); }
.prb-pv-gap { color: var(--text-dim); }
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
    border: 1px solid var(--accent-cyan);
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
.prb-peek-glyph { color: var(--accent-cyan); }
.prb-peek-title { overflow: hidden; text-overflow: ellipsis; min-width: 0; }
.prb-peek-dir { color: var(--text-dim); }
.prb-peek-keys { margin-left: auto; color: var(--text-dim); }
.prb-peek-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: 2px 8px;
    white-space: pre;
    color: #c9d1d9;
}
.prb-peek-line { overflow: hidden; text-overflow: clip; }
.prb-peek-empty { color: var(--text-mid); }

/* Narrow screens: aligned columns collapse into wrapped lines so the board
   never scrolls horizontally. */
@media (max-width: 760px) {
    .pr-board { padding: 10px 10px 40px; }
    .prb-addwrap, .prb-searchwrap { margin-left: 0; width: 100%; }
    #prb-search, #prb-add { flex: 1; width: 100%; }
    .prb-body { display: block; }
    .prb-row { display: block; }
    .prb-l1, .prb-l2, .prb-dl, .prb-sub { display: flex; flex-wrap: wrap; align-items: baseline; gap: 2px 12px; }
    .prb-sub-left { flex: 1 1 auto; }
    .prb-l1-left { flex: 1 1 100%; }
    .prb-activity { margin-left: 19px; }
    .prb-status { margin-left: auto; }
    .prb-dl-right { margin-left: auto; }
    .prb-keys { display: none; }
    .prb-peek { flex-basis: 45%; }
}
`;
