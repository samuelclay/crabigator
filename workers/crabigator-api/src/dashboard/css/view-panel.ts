// Filter bar, its menus, the View panel, the Account popover, and the
// optional lines a session row can show. Colours, radii and sizes follow the
// settings design: accent xterm 45, 44px targets, drawn tiles.
export const viewPanelCss = `
:root {
    --vp-bg: #1B2129;
    --vp-ink: #E6EDF3;
    --vp-soft: #C4CDD8;
    --vp-muted: #9AA3AD;
    --vp-line: #303A46;
    --vp-line-strong: #3A4450;
    --vp-well: #101419;
    --vp-tile: #151B22;
    --vp-accent: #00d7ff;
    --vp-accent-tint: #12262E;
    --vp-gold: #d7af5f;
    --vp-danger: #ff5f5f;
}

/* ── The lines under a session's title ─────────────────────────── */
.row-line {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 10px;
    min-width: 0;
    font-family: var(--font-ui);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    line-height: 1.35;
}
.row-recap {
    display: block;
    color: var(--text-mid);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.row-stat { display: inline-flex; align-items: baseline; gap: 4px; white-space: nowrap; }
.row-glyph { font-family: 'JetBrains Mono', ui-monospace, monospace; }
.row-muted { color: var(--vp-soft); }
/* Read aloud, not shown: the name of a stat that is drawn as a glyph. */
.row-sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

/* ── Filter bar ─────────────────────────────────────────────────── */
.filter-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 24px 12px;
}
.filter-bar[hidden] { display: none; }
/* Chips scroll sideways when the bar is narrow; View stays at the end. */
.filter-chips {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1 1 0;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
    /* Room for the focus ring, which the scroller would otherwise clip. */
    padding: 5px;
    margin: -5px;
}
.filter-chips::-webkit-scrollbar { display: none; }
.filter-chip, .view-btn {
    appearance: none;
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 600;
    color: var(--vp-ink);
    min-height: 44px;
    padding: 0 10px;
    border-radius: 22px;
    border: 1px solid var(--vp-line);
    background: var(--vp-bg);
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    white-space: nowrap;
}
.filter-chip .icon-chevron-down { color: var(--vp-muted); flex: none; }
.filter-chip[aria-expanded="true"] { border-color: var(--vp-accent); }
.chip-dot { display: block; width: 8px; height: 8px; border-radius: 4px; flex: none; }
.view-btn { font-weight: 700; padding: 0 12px; }
.view-btn .icon-sliders { flex: none; }
.view-btn .icon-sliders circle { fill: var(--vp-bg); }
/* Filled accent while its panel is open. */
.view-btn[aria-expanded="true"] { background: var(--vp-accent); border-color: var(--vp-accent); color: var(--vp-well); }
.view-btn[aria-expanded="true"] .icon-sliders circle { fill: var(--vp-accent); }
@media (hover: hover) and (pointer: fine) {
    .filter-chip:hover, .view-btn:not([aria-expanded="true"]):hover { border-color: #6B7684; }
}
/* An open session's own View button sits among its header actions. */
.session-view-btn { min-height: 32px; padding: 0 10px; font-size: 12px; align-self: center; }
/* A mouse needs less room than a finger: the bar is shorter on a desktop. */
@media (hover: hover) and (pointer: fine) and (min-width: 769px) {
    .filter-chip, .filter-bar .view-btn { min-height: 36px; }
}
@media (max-width: 768px) {
    .filter-bar { padding: 0 12px 10px; }
    .session-view-btn { min-height: 44px; }
}

.chip-menu {
    position: fixed;
    z-index: 400;
    min-width: 200px;
    max-width: calc(100vw - 16px);
    max-height: min(70vh, 460px);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 6px;
    background: var(--vp-bg);
    border: 1px solid var(--vp-line-strong);
    border-radius: 12px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
}
.chip-menu[hidden] { display: none; }
.chip-menu-item {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--vp-ink);
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 500;
    min-height: 44px;
    padding: 0 14px 0 8px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
}
.chip-menu-item:focus-visible { outline-offset: -2px; }
.chip-menu-item[aria-checked="true"] { background: var(--vp-accent-tint); font-weight: 600; }
@media (hover: hover) and (pointer: fine) {
    .chip-menu-item:hover { background: var(--vp-tile); }
    .chip-menu-item[aria-checked="true"]:hover { background: var(--vp-accent-tint); }
}
.chip-menu-check { flex: none; width: 16px; color: var(--vp-accent); font-weight: 700; }

/* Sessions the filters hide are counted under the List, never dropped silently. */
.filter-note { padding: 0 20px 24px; }
.filter-note[hidden] { display: none; }
.filter-note-box {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    padding: 10px 14px;
    border: 1px dashed var(--vp-line);
    border-radius: 10px;
    color: var(--vp-muted);
    font-family: var(--font-ui);
    font-size: 13px;
}
.filter-note-btn {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--vp-accent);
    font: inherit;
    font-weight: 600;
    min-height: 32px;
    padding: 0 4px;
    cursor: pointer;
}
/* Every session is filtered out: the note stands where the List would be. */
.filter-note-empty { padding: 28px 20px 8px; }
.filter-note-empty .filter-note-box {
    flex-direction: column;
    justify-content: center;
    gap: 14px;
    padding: 40px 20px;
    border-style: solid;
    border-color: var(--vp-line-strong);
    color: var(--vp-ink);
    font-size: 16px;
    text-align: center;
}
.filter-note-empty .filter-note-btn {
    min-height: 44px;
    padding: 0 18px;
    border-radius: 22px;
    background: var(--vp-accent);
    color: var(--vp-well);
    font-weight: 700;
}

/* ── View panel ─────────────────────────────────────────────────── */
/* A popover under its button on tablets and desktop; a sheet on phones. */
.view-panel {
    display: none;
    position: fixed;
    inset: auto;
    margin: 0;
    z-index: 420;
    width: min(460px, calc(100vw - 24px));
    max-height: calc(100vh - 24px);
    max-height: calc(100dvh - 24px);
    padding: 0;
    flex-direction: column;
    overflow: hidden;
    background: var(--vp-bg);
    color: var(--vp-ink);
    border: 1px solid var(--vp-line-strong);
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    font-family: var(--font-ui);
}
.view-panel.open { display: flex; }
.view-panel:focus { outline: none; }
.view-panel::backdrop { background: transparent; }
.vp-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex: none;
    min-height: 48px;
    padding: 6px 16px 0;
}
.vp-title { margin: 0; font-size: 17px; font-weight: 700; }
/* The scope word wears the brand's gold. */
.vp-title span { font-weight: 500; color: var(--vp-gold); }
.vp-done {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--vp-accent);
    font-family: var(--font-ui);
    font-size: 16px;
    font-weight: 600;
    min-height: 44px;
    padding: 0 4px;
    cursor: pointer;
}
.vp-body {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 8px 16px 24px;
    overflow-y: auto;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
}
.vp-section { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
.vp-section h3 { margin: 0; font-size: 15px; font-weight: 600; }
.vp-note { margin: 0; font-size: 12px; line-height: 1.4; color: var(--vp-muted); }
.vp-label { font-size: 13px; color: var(--vp-soft); }
.vp-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 44px; font-size: 15px; }

/* Time ruler: every bar up to the choice is lit in its band's colour. */
.vp-ruler { display: flex; gap: 4px; }
.vp-band {
    appearance: none;
    cursor: pointer;
    font-family: var(--font-ui);
    flex: 1 1 0;
    min-width: 0;
    min-height: 52px;
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 7px 2px;
    border-radius: 9px;
    border: 2px solid transparent;
    background: transparent;
    color: var(--vp-muted);
}
.vp-band.on { border-color: var(--vp-accent); color: var(--vp-ink); }
.vp-band-bar { display: block; height: 8px; border-radius: 4px; }
.vp-band-label { font-size: 12px; font-weight: 500; text-align: center; }
.vp-band.on .vp-band-label { font-weight: 700; }

/* Switch */
.vp-switch { appearance: none; border: 0; background: transparent; cursor: pointer; flex: none; width: 52px; height: 44px; padding: 7px 0; }
.vp-switch-track {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: 52px;
    height: 30px;
    padding: 3px;
    border-radius: 15px;
    background: var(--vp-line-strong);
}
.vp-switch[aria-checked="true"] .vp-switch-track { background: var(--vp-accent); justify-content: flex-end; }
.vp-switch-knob { display: block; width: 24px; height: 24px; border-radius: 12px; background: #FFFFFF; }

/* Segmented control: the chosen segment is filled accent with dark text. */
.vp-seg { display: flex; flex-wrap: wrap; gap: 3px; padding: 3px; border-radius: 10px; background: var(--vp-well); min-width: 0; }
.vp-seg button {
    appearance: none;
    border: 0;
    cursor: pointer;
    flex: 1 1 0;
    min-height: 44px;
    padding: 0 10px;
    border-radius: 7px;
    background: transparent;
    color: var(--vp-soft);
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
}
.vp-seg button[aria-pressed="true"] { background: var(--vp-accent); color: var(--vp-well) !important; font-weight: 700; }

/* Drawn tiles */
.vp-tiles { display: grid; gap: 6px; }
.vp-tiles-4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
.vp-tiles-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.vp-tile {
    appearance: none;
    cursor: pointer;
    font-family: var(--font-ui);
    color: var(--vp-ink);
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
    min-height: 44px;
    padding: 5px;
    border-radius: 10px;
    border: 2px solid var(--vp-line);
    background: var(--vp-tile);
    text-align: center;
}
/* Chosen: a 2px accent border and a 13% accent tint over the tile. */
.vp-tile.on { border-color: var(--vp-accent); background: var(--vp-accent-tint); }
.vp-thumb { display: block; width: 100%; height: auto; border-radius: 4px; background: #12171D; }
.vp-thumb-terminal { background: #0B0F14; }
.vp-tile-label { font-size: 12px; font-weight: 600; line-height: 1.25; }
.vp-tile-note { font-size: 12px; line-height: 1.3; color: var(--vp-muted); }
.vp-tiles-2 .vp-tile { gap: 6px; padding: 8px; border-radius: 12px; text-align: left; }
.vp-tiles-2 .vp-thumb { border-radius: 6px; }
.vp-tiles-2 .vp-tile-label { font-size: 14px; }

/* Chips: a 14px check box, then the label */
.vp-chips { display: flex; flex-wrap: wrap; gap: 8px; }
.vp-chip {
    appearance: none;
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 13px;
    min-height: 44px;
    padding: 0 11px;
    border-radius: 10px;
    border: 2px solid var(--vp-line);
    background: transparent;
    color: var(--vp-muted);
    display: flex;
    align-items: center;
    gap: 7px;
}
.vp-chip[aria-pressed="true"] { border-color: var(--vp-accent); background: var(--vp-accent-tint); color: var(--vp-ink); }
.vp-check { display: block; flex: none; width: 14px; height: 14px; border-radius: 4px; border: 2px solid #6B7684; }
.vp-chip[aria-pressed="true"] .vp-check { border-color: var(--vp-accent); background: var(--vp-accent); }
.vp-tone { display: block; flex: none; width: 8px; height: 8px; border-radius: 4px; }

/* Steppers */
.vp-stepper { display: flex; align-items: center; gap: 8px; flex: none; }
.vp-step {
    appearance: none;
    cursor: pointer;
    flex: none;
    width: 44px;
    height: 44px;
    border-radius: 10px;
    border: 1px solid var(--vp-line);
    background: var(--vp-tile);
    color: var(--vp-ink);
    font-family: var(--font-ui);
    font-size: 16px;
    font-weight: 600;
}
.vp-step:disabled { opacity: 0.35; cursor: default; }
.vp-text-size .vp-step-down { font-size: 13px; }
.vp-text-size .vp-step-up { font-size: 19px; }
.vp-step-value { min-width: 44px; text-align: center; font-size: 14px; font-weight: 600; color: var(--vp-accent); white-space: nowrap; }
.vp-size-row { display: flex; align-items: center; gap: 8px; }
.vp-size-row .vp-seg { flex: 1 1 0; }
.vp-wide-only { display: flex; flex-direction: column; gap: 8px; }
/* The session list is an overlay on a phone: its side and pin have no effect. */
@media (max-width: 768px) { .vp-wide-only { display: none; } }

/* Sample rows: the real row markup, at rest (nothing in them can be pressed) */
.vp-sample { background: var(--vp-well); border-radius: 12px; padding: 10px; min-width: 0; overflow: hidden; }
.vp-sample .container { padding: 0; }
.vp-sample .container .session-card.session-row { margin: 0; }
.vp-sample .pr-board { height: auto; padding: 0; display: block; overflow: visible; }
.vp-sample .prb-body { display: block; overflow: visible; padding: 0; }
/* The panel is narrow, so the sample block wraps as the board does on a phone. */
.vp-sample .prb-row { display: block; padding-inline: 0; }
.vp-sample .prb-l1, .vp-sample .prb-l2, .vp-sample .prb-dl, .vp-sample .prb-sub { display: flex; flex-wrap: wrap; align-items: baseline; gap: 2px 12px; }
.vp-sample .prb-l1-left { flex: 1 1 100%; }
.vp-sample .prb-sub-left { flex: 1 1 auto; }
.vp-sample .prb-activity { margin-left: 19px; }
.vp-sample .prb-status, .vp-sample .prb-dl-right { margin-left: auto; }
.vp-sample-strip { padding: 0 6px; }
.vp-sample-strip .session-card { margin: 0; background: transparent; }
.vp-sample-strip .widgets-header { border-top: 0; cursor: default; padding: 0; background: transparent; height: auto; }
/* The real strip scrolls sideways; the sample wraps so every stat can be seen. */
.vp-sample-strip .widgets-header-row2 { flex-wrap: wrap; height: auto; overflow: visible; white-space: normal; gap: 6px 10px; padding: 9px 0; }
.vp-terminal-sample {
    background: #0B0F14;
    border-radius: 10px;
    padding: 10px 12px;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    line-height: 1.45;
    color: var(--vp-soft);
    overflow: hidden;
    white-space: nowrap;
}

/* The small drawing of an open session */
.vp-height-row { display: flex; align-items: flex-start; gap: 14px; }
.vp-height-controls { display: flex; flex-direction: column; gap: 8px; flex: 1 1 0; min-width: 0; }
.vp-height .vp-step-value { flex: 1 1 0; }
.vp-drawing {
    flex: 0 0 104px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: #0B0F14;
    border: 1px solid var(--vp-line-strong);
    border-radius: 12px;
}
.vp-drawing-head { flex: 0 0 14px; background: var(--vp-well); }
.vp-drawing-terminal { display: flex; flex-direction: column; gap: 4px; padding: 6px 7px; overflow: hidden; }
.vp-drawing-terminal span { flex: 0 0 2px; }
.vp-drawing-drawer { display: flex; flex-direction: column; gap: 4px; padding: 5px 7px; background: var(--vp-tile); border-top: 1px solid var(--vp-line); }
.vp-drawing-drawer span { height: 3px; border-radius: 2px; }
.vp-drawing-strip { flex: 0 0 12px; background: var(--vp-bg); border-top: 1px solid var(--vp-line); }
.vp-drawing-input { flex: 0 0 16px; background: var(--vp-well); border-top: 1px solid var(--vp-line); }

@media (max-width: 600px) {
    .view-panel {
        left: 0 !important;
        right: 0;
        top: auto !important;
        bottom: 0;
        width: 100%;
        max-width: none;
        max-height: 86vh !important;
        max-height: 86dvh !important;
        border-width: 1px 0 0;
        border-radius: 22px 22px 0 0;
        padding-bottom: env(safe-area-inset-bottom);
    }
    .view-panel::backdrop { background: rgba(0, 0, 0, 0.5); }
}

/* ── Account ────────────────────────────────────────────────────── */
/* Account, device and agent items only: nothing here changes how a screen looks. */
.settings-popover {
    display: none;
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    z-index: 200;
    width: 380px;
    /* The base reset caps every box at its container, which here is the button. */
    max-width: calc(100vw - 24px);
    max-height: calc(100vh - var(--header-height, 67px) - 16px);
    max-height: calc(100dvh - var(--header-height, 67px) - 16px);
    overflow-y: auto;
    overscroll-behavior: contain;
    flex-direction: column;
    gap: 16px;
    padding: 0 16px 20px;
    background: var(--vp-well);
    color: var(--vp-ink);
    border: 1px solid var(--vp-line-strong);
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    font-family: var(--font-ui);
}
.settings-popover.visible { display: flex; }
.settings-popover:focus { outline: none; }
.account-head { display: flex; align-items: center; justify-content: space-between; min-height: 56px; margin-bottom: -8px; }
.account-head h2 { margin: 0; font-size: 20px; font-weight: 700; }
.account-group { display: flex; flex-direction: column; gap: 8px; }
.account-group[hidden] { display: none; }
.account-group h3 { margin: 0; font-size: 13px; font-weight: 600; color: var(--vp-muted); }
.account-card {
    display: flex;
    flex-direction: column;
    padding: 0 14px;
    background: var(--vp-bg);
    border: 1px solid var(--vp-line);
    border-radius: 14px;
}
.account-card > * + * { border-top: 1px solid var(--vp-line); }
.account-card > :empty, .account-card > .invite-result:not(.visible) { display: none; }
.account-row {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--vp-ink);
    font-family: var(--font-ui);
    font-size: 15px;
    text-align: left;
    text-decoration: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 0;
}
.account-card > * + .account-row { border-top: 1px solid var(--vp-line); }
a.account-row, .account-action, .account-block a { color: var(--vp-accent); }
.account-danger { color: var(--vp-danger); }
.account-row:disabled { opacity: 0.6; cursor: default; }
.account-block { display: flex; flex-direction: column; gap: 4px; padding: 12px 0; font-size: 15px; min-width: 0; }
.account-block code { font-family: 'JetBrains Mono', ui-monospace, monospace; font-size: 13px; color: var(--vp-soft); overflow-wrap: anywhere; }
.account-block a { display: flex; align-items: center; min-height: 44px; margin-bottom: -12px; font-size: 14px; text-decoration: none; }
.account-muted { font-size: 13px; color: var(--vp-muted); }
.account-text { margin: 0; padding: 12px 0; font-size: 13px; line-height: 1.5; color: var(--vp-soft); }
.account-card .account-identities { padding: 12px 0; margin: 0; font-size: 15px; color: var(--vp-ink); }
.account-card .account-connect-row { padding: 12px 0; margin: 0; }
.account-card .invite-result { margin: 0 0 12px; border-top: 1px solid var(--vp-line); }
.account-note { margin: 0; font-size: 12px; line-height: 1.4; color: var(--vp-muted); }
@media (max-width: 768px) {
    /* The button sits near the right edge; keep the popover inside the window. */
    .settings-popover { position: fixed; top: calc(var(--header-height, 46px) + 6px); right: 12px; }
}
`;
