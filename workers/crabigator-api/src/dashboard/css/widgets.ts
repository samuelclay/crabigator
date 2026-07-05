// Widgets panel, tooltips, grid, git files, changes list
export const widgetsCss = `
/* Widgets panel */
.widgets-panel {
    border-top: 1px solid var(--border-dim);
}
.widgets-header {
    display: flex;
    flex-direction: column;
    padding: 10px 14px;
    background: var(--bg-card);
    cursor: pointer;
    user-select: none;
    gap: 6px;
    border-bottom: 1px solid var(--border-dim);
    transition: background 0.2s;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    overflow: visible;
}
.widgets-header:hover { background: var(--bg-surface); }
.widgets-header .collapse-btn {
    padding: 2px 6px;
    font-size: 10px;
    flex-shrink: 0;
}
.widgets-header-row1 {
    display: flex;
    align-items: center;
    gap: 10px;
    overflow: visible;
}
.widgets-title {
    color: var(--accent-cyan);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.widgets-state {
    color: var(--text-mid);
    flex-shrink: 0;
}
.widgets-header-spacer {
    flex: 1;
}
.widgets-header-row2 {
    display: flex;
    align-items: center;
    gap: 8px 12px;
    flex-wrap: wrap;
    justify-content: flex-start;
    overflow: visible;
}
.wh-stat {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: visible;
    color: var(--text-mid);
}
.wh-icon {
    font-size: 10px;
}
.wh-value {
    color: var(--text-bright);
}
.wh-spacer {
    flex: 1;
}
.wh-git {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
    margin-left: auto;
}
.wh-git .green { color: var(--accent-green); }
.wh-git .red { color: var(--accent-red); }
.wh-git .files { color: var(--text-mid); }
.wh-elapsed {
    color: var(--text-dim);
    font-size: 9px;
    margin-left: 2px;
}
.wh-elapsed:not(:empty)::before {
    content: ' ';
}

/* Custom CSS tooltips */
[data-tooltip] {
    position: relative;
}
[data-tooltip]::after {
    content: attr(data-tooltip);
    position: absolute;
    bottom: calc(100% + 4px);
    left: 50%;
    transform: translateX(-50%);
    padding: 4px 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 4px;
    font-size: 10px;
    color: var(--text-bright);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s ease;
    transition-delay: 0s;
    z-index: 9999;
    box-shadow: 0 2px 8px rgba(0,0,0,0.3);
}
[data-tooltip]:hover::after {
    opacity: 1;
    transition-delay: 0.5s;
}

/* Widgets content grid - now only Git and Changes */
.widgets-content {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1px;
    background: var(--border-dim);
    max-height: 500px;
    overflow: hidden;
    transition: max-height 0.25s ease-out;
}
.widget {
    background: var(--bg-deep);
    padding: 14px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    min-width: 0;
    overflow: hidden;
}
.widget-title {
    color: var(--accent-cyan);
    font-weight: 600;
    margin-bottom: 10px;
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 1px;
    display: flex;
    align-items: center;
    gap: 8px;
}
.widget-title::before {
    content: '';
    width: 6px;
    height: 6px;
    background: var(--accent-cyan);
    border-radius: 50%;
    box-shadow: 0 0 8px var(--glow-cyan);
}
.widget-row {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
    min-width: 0;
    gap: 10px;
}
.widget-label {
    color: var(--text-dim);
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.widget-value {
    color: var(--text-bright);
    flex-shrink: 0;
    font-weight: 500;
}
.widget-value.green { color: var(--accent-green); }
.widget-value.red { color: var(--accent-red); }
.widget-value.cyan { color: var(--accent-cyan); }
.widget-value.purple { color: var(--accent-purple); }
.widget-value.yellow { color: var(--accent-yellow); }

/* Hide empty changes widget */
.widget.hidden-changes { display: none; }

/* Single column when changes hidden (Git spans full width) */
.widgets-content.no-changes { grid-template-columns: 1fr; }

/* Title history widget - spans full width */
.title-history-widget {
    grid-column: 1 / -1;
}

/* Adjust grid when title history is hidden */
.widgets-content.no-titles .title-history-widget { display: none; }
.titles-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.titles-list .title-entry {
    color: var(--text-mid);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 10px;
}

/* Stack widgets for multi-column layouts */
.container[data-layout="2"] .widgets-content,
.container[data-layout="3"] .widgets-content,
.container[data-layout="4"] .widgets-content,
.container[data-layout="fit"] .widgets-content {
    grid-template-columns: 1fr;
}

/* Mobile responsive */
@media (max-width: 768px) {
    .widgets-content { grid-template-columns: 1fr !important; }
    .style-popover {
        right: -8px;
        min-width: 200px;
    }
    .container {
        padding: 10px;
    }
    .session-card {
        min-width: 0;
        max-width: 100%;
        border-radius: 8px;
        margin-bottom: 12px;
    }
    .session-card:last-child {
        margin-bottom: 0;
    }
    .session-header {
        padding: 12px 14px;
        gap: 10px;
        border-radius: 7px 7px 0 0;
    }
    .input-area {
        border-radius: 0 0 7px 7px;
    }
    .terminal {
        font-size: 10px;
        line-height: 1;
        padding: 10px;
    }
    .widget {
        padding: 12px;
    }
    .input-area {
        padding: 12px;
    }
    .input-area input {
        font-size: 16px;  /* Prevent iOS zoom */
    }
    .permission-bar {
        padding: 12px 14px;
    }
    .perm-btn {
        padding: 10px 14px;
        font-size: 11px;
    }
    .perm-hint {
        display: none;
    }
}

/* Git files list */
.git-file {
    display: flex;
    gap: 8px;
    padding: 3px 0;
    align-items: center;
    min-width: 0;
}
.git-file .path {
    color: var(--text-bright);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.git-file .diff {
    flex-shrink: 0;
    white-space: nowrap;
    display: flex;
    gap: 6px;
    align-items: center;
}

/* Changes list */
.change-item {
    display: flex;
    gap: 6px;
    padding: 3px 0;
    align-items: center;
    min-width: 0;
}
.change-item .name {
    color: var(--text-bright);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
`;
