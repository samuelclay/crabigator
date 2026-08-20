// Session cards, actions, state badges, info popover, pin, collapse, summary
export const sessionCardCss = `
/* Session cards */
.session-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    overflow: visible;
    position: relative;
    break-inside: avoid;
    margin-bottom: 16px;
    max-width: 100%;
    transition: border-color 0.3s ease, box-shadow 0.3s ease;
}
.session-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--accent-cyan), transparent);
    opacity: 0;
    transition: opacity 0.3s;
}
.session-card:hover {
    border-color: var(--border-glow);
    box-shadow: 0 0 30px rgba(34, 211, 238, 0.1);
}
.session-card:hover::before {
    opacity: 1;
}

.session-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-dim);
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-card);
    border-radius: 11px 11px 0 0;
}
.session-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.session-header .title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent-cyan);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.session-header .title.official {
    color: #bc8cff;
}
.session-header .generated-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: #58a6ff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.session-header .generated-title:empty { display: none; }
.session-header .cwd {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Session actions */
.session-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
}

/* State badges */
.session-header .state {
    font-family: 'JetBrains Mono', monospace;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
}
.state.ready {
    background: rgba(74, 222, 128, 0.2);
    color: var(--accent-green);
    border: 1px solid var(--accent-green);
}
.state.thinking {
    background: rgba(59, 130, 246, 0.2);
    color: var(--accent-blue);
    border: 1px solid var(--accent-blue);
}
.state.permission {
    background: rgba(251, 146, 60, 0.2);
    color: var(--accent-orange);
    border: 1px solid var(--accent-orange);
}
.state.question {
    background: rgba(167, 139, 250, 0.2);
    color: var(--accent-purple);
    border: 1px solid var(--accent-purple);
}
.state.complete {
    background: rgba(100, 116, 139, 0.2);
    color: var(--text-mid);
    border: 1px solid var(--text-dim);
}
.state.interrupted {
    background: rgba(248, 113, 113, 0.2);
    color: var(--accent-red);
    border: 1px solid var(--accent-red);
}

/* Info button and popover */
.info-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid transparent;
    padding: 4px 8px;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-dim);
    border-radius: 4px;
    transition: all 0.2s;
}
.info-btn:hover {
    background: var(--bg-surface);
    color: var(--accent-cyan);
    border-color: var(--border-dim);
}
.info-popover {
    display: none;
    position: absolute;
    right: 8px;
    left: 8px;
    top: 52px;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 14px 16px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-bright);
    z-index: 100;
    box-shadow: 0 16px 48px rgba(0,0,0,0.5), 0 0 30px rgba(34, 211, 238, 0.1);
    user-select: text;
}
.info-popover.visible { display: block; }
.info-popover-row {
    display: flex;
    justify-content: space-between;
    padding: 6px 0;
    gap: 16px;
}
.info-popover-row:not(:last-child) {
    border-bottom: 1px solid var(--border-dim);
}
.info-popover-label {
    color: var(--text-dim);
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-size: 10px;
}
.info-popover-value {
    color: var(--text-bright);
    text-align: right;
    word-break: break-all;
}
.info-popover-value.copyable {
    cursor: pointer;
    transition: color 0.2s;
}
.info-popover-value.copyable:hover {
    color: var(--accent-cyan);
}
.info-popover-actions {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border-dim);
    display: flex;
    gap: 10px;
}
.focus-session-btn {
    font-family: 'JetBrains Mono', monospace;
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 14px;
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.15) 0%, rgba(59, 130, 246, 0.15) 100%);
    border: 1px solid var(--accent-cyan);
    border-radius: 6px;
    color: var(--accent-cyan);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
}
.focus-session-btn:hover {
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.25) 0%, rgba(59, 130, 246, 0.25) 100%);
    box-shadow: 0 0 20px var(--glow-cyan);
    transform: translateY(-1px);
}
.focus-session-btn:active {
    transform: translateY(0);
}
.focus-session-btn svg {
    flex-shrink: 0;
}

/* Pin indicator and button */
.pin-indicator {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.pin-indicator.pinned {
    color: var(--accent-green);
}
.pin-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px dashed var(--accent-yellow);
    padding: 4px 10px;
    cursor: pointer;
    font-size: 10px;
    border-radius: 4px;
    transition: all 0.2s;
    color: var(--accent-yellow);
    display: flex;
    align-items: center;
    gap: 6px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.pin-btn:hover {
    background: rgba(251, 191, 36, 0.1);
    border-style: solid;
    box-shadow: 0 0 15px rgba(251, 191, 36, 0.2);
}

/* Collapse button */
.collapse-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: none;
    padding: 4px 8px;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-dim);
    border-radius: 4px;
    transition: all 0.2s;
}
.collapse-btn:hover {
    background: var(--bg-surface);
    color: var(--text-mid);
}
.collapse-btn.collapsed { transform: rotate(-90deg); }

/* Session body - animated collapse container */
.session-body {
    display: grid;
    grid-template-rows: 1fr;
    transition: grid-template-rows 0.35s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
}
.session-body-inner {
    min-height: 0;
    overflow: hidden;
}

/* Collapsed state */
.session-card.collapsed .session-body {
    grid-template-rows: 0fr;
}
.session-card.collapsed .session-header { border-bottom: none; }
.session-card.collapsed .info-popover { display: none; }
.session-card.widgets-collapsed .widgets-content {
    max-height: 0;
    overflow: hidden;
}
.session-card.widgets-collapsed .widgets-header .collapse-btn { transform: rotate(-90deg); }

/* Session summary (shown when collapsed) */
.session-summary {
    display: none;
    padding: 10px 16px;
    background: var(--bg-deep);
    border-top: 1px solid var(--border-dim);
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-mid);
    gap: 16px;
    flex-wrap: wrap;
}
.session-card.collapsed .session-summary { display: flex; }
.summary-item {
    display: flex;
    align-items: center;
    gap: 6px;
}
.summary-item .label { color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
.summary-item .value { color: var(--text-bright); }
.summary-item .green { color: var(--accent-green); }
.summary-item .red { color: var(--accent-red); }
.summary-item .purple { color: var(--accent-purple); }
.summary-item .blue { color: var(--accent-cyan); }
`;
