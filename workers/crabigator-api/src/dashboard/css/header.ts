// Header, filter indicator, container styles
export const headerCss = `
/* Header */
.header {
    background: var(--bg-abyss);
    border-bottom: 1px solid rgba(255,255,255,.05);
    padding: 14px 24px;
    display: flex;
    align-items: center;
    gap: 10px;
    position: fixed;
    inset: 0 0 auto;
    z-index: 210;
    max-width: 100%;
}
.header h1 { display: flex; align-items: center; gap: 20px; margin-right: auto; flex-shrink: 0; }
.dashboard-brand { display: inline-flex; align-items: center; text-decoration: none; }
.dashboard-brand .brand-logo { display: block; width: 216px; height: 27.2px; }
.dashboard-brand .brand-crab { display: none; }
.dashboard-label { font: 400 14px var(--font-ui); color: var(--text-dim); border-left: 1px solid var(--border-dim); padding-left: 20px; }
.thinking-spinner {
    display: inline-block;
}
.header .status {
    font-family: var(--font-ui);
    font-size: 11px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    text-transform: none;
    letter-spacing: 0;
}

/* Filter indicator */
.filter-indicator {
    display: none;
    align-items: center;
    gap: 10px;
    padding: 6px 12px;
    background: rgba(251, 191, 36, 0.15);
    border: 1px solid var(--accent-yellow);
    border-radius: 6px;
    font-family: var(--font-ui);
    font-size: 11px;
    color: var(--accent-yellow);
}
.filter-indicator.visible {
    display: flex;
}

.filter-text {
    text-transform: none;
    letter-spacing: 0;
    white-space: nowrap;
}
.filter-clear {
    background: transparent;
    border: none;
    color: var(--accent-yellow);
    cursor: pointer;
    padding: 2px 6px;
    font-size: 14px;
    line-height: 1;
    border-radius: 4px;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
}
.filter-clear:hover {
    background: rgba(251, 191, 36, 0.2);
    color: var(--text-bright);
}

/* One compact control language for navigation and utility actions. */
.header .refresh-btn, .header .style-btn, .header .settings-btn,
.header .sessions-btn, .header .pr-board-btn {
    font-family: var(--font-ui);
    min-height: 34px;
    padding: 7px 11px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--text-mid);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    white-space: nowrap;
    box-shadow: none;
}
.header .sessions-btn.active, .header .pr-board-btn.active,
.header .style-btn.active, .header .settings-btn.active {
    color: var(--accent-peach);
    border-color: transparent;
    background: #2b302f;
}
.header .sessions-count { color: inherit; }
.header .pr-board-btn-icon { color: inherit; }
.header .refresh-btn:hover, .header .style-btn:hover, .header .settings-btn:hover,
.header .sessions-btn:hover, .header .pr-board-btn:hover { background: var(--bg-surface); color: var(--text-bright); }
.header .status { flex: 0 1 auto; max-width: 170px; font: 11px var(--font-ui); }

@media (max-width: 768px) {
    .header-control-label { display: none; }
    .header {
        flex-wrap: nowrap;
        gap: 6px;
        padding: 10px 12px;
    }
    .header h1 { flex: 0 0 32px; margin-right: auto; }
    .dashboard-brand .brand-logo, .dashboard-label { display: none; }
    .dashboard-brand .brand-crab { display: block; width: 32px; height: 32px; }
    .header .refresh-btn,
    .header .style-btn,
    .header .settings-btn {
        width: 32px;
        height: 32px;
        padding: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        flex: 0 0 32px;
        gap: 0;
        font-size: 0;
        letter-spacing: 0;
    }
    .header .refresh-btn::before {
        content: '↻';
        font-size: 13px;
        line-height: 1;
    }
    .header .style-btn svg,
    .header .settings-btn svg {
        width: 14px;
        height: 14px;
    }
    /* Icon-only PR board button, matching its style/settings neighbors */
    .header .pr-board-btn {
        width: 32px;
        height: 32px;
        padding: 0;
        justify-content: center;
        flex: 0 0 32px;
        gap: 0;
        font-size: 0;
    }
    .header .pr-board-btn .pr-board-btn-icon {
        font-size: 13px;
        line-height: 1;
    }
    .header .sessions-container,
    .header .style-container,
    .header .settings-container {
        flex: 0 0 auto;
    }
    .header .sessions-btn {
        height: 32px;
        min-width: 34px;
        padding: 0 10px;
        flex: 0 0 auto;
        gap: 6px;
        letter-spacing: 0;
    }
    .header .sessions-label {
        display: inline;
    }
    .filter-indicator {
        flex: 1 1 78px;
        min-width: 0;
        max-width: 92px;
        height: 32px;
        padding: 0 6px;
        gap: 4px;
        font-size: 9px;
        letter-spacing: 0;
    }
    .filter-text {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        letter-spacing: 0;
    }
    .filter-clear {
        width: 16px;
        height: 20px;
        padding: 0;
        flex: 0 0 16px;
        font-size: 11px;
    }
}

@media (min-width: 769px) and (max-width: 1050px) { .dashboard-label { display: none; } }
@media (max-width: 380px) {
    .header { gap: 4px; padding: 8px; }
    .header .sessions-btn { padding: 0 6px; }
    .header .status { display: none; }
}
/* Container */
.container {
    padding: 16px 20px;
    column-gap: 16px;
    max-width: 100%;
    overflow-x: hidden;
}
`;
