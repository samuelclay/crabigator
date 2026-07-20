// Header, filter indicator, container styles
export const headerCss = `
/* Header */
.header {
    background: rgba(10, 15, 26, 0.8);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border-dim);
    padding: 16px 24px;
    display: flex;
    align-items: center;
    gap: 16px;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;
    max-width: 100%;
}
/* Disable expensive backdrop-filter on mobile - use solid background instead */
@media (hover: none) {
    .header {
        backdrop-filter: none;
        -webkit-backdrop-filter: none;
        background: var(--bg-deep);
    }
}
.header h1 {
    font-family: 'JetBrains Mono', monospace;
    font-size: 18px;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 10px;
    letter-spacing: -0.5px;
    color: var(--text-bright);
}
.header h1::before {
    content: '>';
    color: var(--accent-cyan);
    animation: blink 1s step-end infinite;
}
@keyframes blink {
    50% { opacity: 0; }
}
.thinking-spinner {
    display: inline-block;
}
.header .status {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    margin-left: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
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
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--accent-yellow);
    animation: filter-pulse 2s ease-in-out infinite;
}
.filter-indicator.visible {
    display: flex;
}
@keyframes filter-pulse {
    0%, 100% { box-shadow: 0 0 0 rgba(251, 191, 36, 0); }
    50% { box-shadow: 0 0 12px rgba(251, 191, 36, 0.3); }
}
.filter-text {
    text-transform: uppercase;
    letter-spacing: 0.5px;
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

@media (max-width: 768px) {
    .header {
        flex-wrap: nowrap;
        gap: 6px;
        padding: 8px;
    }
    .header h1 {
        flex: 0 0 24px;
        width: 24px;
        min-width: 24px;
        overflow: hidden;
        white-space: nowrap;
        font-size: 16px;
        gap: 0;
        letter-spacing: 0;
    }
    .header h1::before {
        display: none;
    }
    .header .refresh-btn,
    .header .style-btn,
    .header .settings-btn {
        width: 30px;
        height: 30px;
        padding: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        flex: 0 0 30px;
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
    .header .sessions-container,
    .header .style-container,
    .header .settings-container {
        flex: 0 0 auto;
    }
    .header .sessions-btn {
        height: 30px;
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
        height: 30px;
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

/* Container */
.container {
    padding: 16px;
    column-gap: 16px;
    max-width: 100%;
    overflow-x: hidden;
}
`;
