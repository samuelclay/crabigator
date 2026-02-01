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
    position: sticky;
    top: 0;
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

/* Container */
.container {
    padding: 16px;
    column-gap: 16px;
    max-width: 100%;
    overflow-x: hidden;
}
`;
