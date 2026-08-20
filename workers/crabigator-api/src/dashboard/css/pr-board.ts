// Dashboard CSS - cross-session PR board
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
    flex: 1;
    min-width: 0;
    padding: 16px 20px 40px;
    overflow-y: auto;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    display: grid;
    grid-template-columns: 18px minmax(18ch, 1fr) auto auto minmax(8ch, 24ch) 15ch auto;
    column-gap: 8px;
    align-content: start;
}
.pr-board-header {
    grid-column: 1 / -1;
    font-size: 15px;
    font-weight: 700;
    color: #c4a7f7;
    margin-bottom: 14px;
}
.pr-board-count {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-dim, #6e7681);
    margin-left: 6px;
}
.pr-board-repo {
    grid-column: 1 / -1;
    font-size: 12px;
    font-weight: 700;
    color: #58d6ff;
    margin: 16px 0 6px;
    border-bottom: 1px solid var(--border, #30363d);
    padding-bottom: 3px;
}
.pr-board-empty {
    grid-column: 1 / -1;
    color: var(--text-dim, #6e7681);
    font-size: 13px;
    padding: 24px 0;
}
.pr-board-row {
    display: grid;
    grid-template-columns: subgrid;
    grid-column: 1 / -1;
    padding: 7px 8px;
    border-radius: 6px;
    margin-bottom: 2px;
}
.pr-board-row:hover { background: var(--bg-tertiary, #21262d); }
/* Secondary PRs recede so the primary being worked on stands out. */
.pr-board-row.pr-secondary { opacity: 0.7; }
.pr-board-row.pr-secondary:hover { opacity: 1; }
.pr-board-row .pr-row-top {
    display: grid;
    grid-template-columns: subgrid;
    grid-column: 1 / -1;
    align-items: center;
}
.pr-board-row .pr-primary-toggle { font-size: 12px; cursor: pointer; user-select: none; }
.pr-board-row .pr-session-glyph { color: #6e7681; font-size: 12px; user-select: none; }
.pr-board-row .pr-primary-toggle.primary { color: #c4a7f7; }
.pr-board-row .pr-primary-toggle.secondary { color: #6e7681; }
.pr-board-row .pr-primary-toggle:hover { color: #d6bffb; }
.pr-board-row .pr-dismiss { font-size: 10px; color: #6e7681; opacity: 0.55; cursor: pointer; user-select: none; }
.pr-board-row .pr-dismiss:hover { opacity: 1; color: #f85149; }
.pr-board-row .pr-link { font-size: 12px; font-weight: 600; color: #c4a7f7; text-decoration: none; white-space: nowrap; }
.pr-board-row .pr-link:hover { text-decoration: underline; }
.pr-board-identity { min-width: 0; display: flex; align-items: baseline; gap: 8px; overflow: hidden; }
.pr-board-title {
    font-size: 12px;
    color: #bc8cff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 34ch;
}
.pr-board-generated-title {
    grid-column: 2 / -1;
    color: #58a6ff;
    font-size: 11px;
    margin-top: 3px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.pr-board-row .pr-diff { font-size: 10px; white-space: nowrap; text-decoration: none; }
.pr-board-row .pr-diff .rd-add { color: var(--accent-green, #3fb950); }
.pr-board-row .pr-diff .rd-del { color: var(--accent-red, #f85149); }
.pr-files, .pr-branch { color: var(--text-dim, #6e7681); font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.pr-activity { color: var(--text-dim, #6e7681); font-size: 11px; white-space: nowrap; }
.pr-board-row .pr-status { display: inline-flex; align-items: center; gap: 8px; white-space: nowrap; }
.pr-board-meta { grid-column: 1 / -1; font-size: 11px; margin-top: 3px; color: var(--text-secondary, #8b949e); }
.pr-board-session { grid-column: 1 / -1; color: var(--text-secondary, #8b949e); font-size: 11px; margin: 3px 0 0 26px; }
.pb-checklist { letter-spacing: 2px; margin-right: 4px; }
.pb-dot.good { color: #3fb950; }
.pb-dot.bad { color: #f85149; }
.pb-dot.warn { color: #d29922; }
.pb-dot.dim { color: #484f58; }
.pb-dot.merged { color: #a371f7; }
.pb-stage.good { color: #3fb950; }
.pb-stage.bad { color: #f85149; }
.pb-stage.warn { color: #d29922; }
.pb-stage.dim { color: #6e7681; }
.pb-stage.merged { color: #a371f7; }
.pb-dim { color: var(--text-dim, #6e7681); }
.pr-board-extras { grid-column: 1 / -1; font-size: 11px; margin-top: 3px; display: flex; gap: 12px; flex-wrap: wrap; }
.pb-ai { color: #d29922; }
.pb-slack { color: #58d6ff; text-decoration: none; }
.pb-slack:hover { text-decoration: underline; }
`;
