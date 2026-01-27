// Dashboard CSS styles - Ocean Depths theme (matching landing page)
export const dashboardCss = `
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Space+Grotesk:wght@400;500;600;700&display=swap');

* { box-sizing: border-box; margin: 0; padding: 0; max-width: 100%; }

:root {
    --font-scale: 1;
    /* Ocean Depths palette */
    --bg-abyss: #030712;
    --bg-deep: #0a0f1a;
    --bg-surface: #111827;
    --bg-card: #1a2332;
    --border-dim: #1e293b;
    --border-glow: #0ea5e9;
    --text-bright: #f1f5f9;
    --text-mid: #94a3b8;
    --text-dim: #64748b;
    --accent-cyan: #22d3ee;
    --accent-blue: #3b82f6;
    --accent-magenta: #e879f9;
    --accent-green: #4ade80;
    --accent-orange: #fb923c;
    --accent-red: #f87171;
    --accent-purple: #a78bfa;
    --accent-yellow: #fbbf24;
    --glow-cyan: rgba(34, 211, 238, 0.4);
    --glow-magenta: rgba(232, 121, 249, 0.3);
    --glow-green: rgba(74, 222, 128, 0.3);
}

html {
    overflow-x: hidden;
    width: 100%;
    scroll-behavior: smooth;
    touch-action: pan-x pan-y;
}

/* Prevent double-tap zoom on interactive elements */
button, input, select, .style-option, .style-popover {
    touch-action: manipulation;
}

/* Scanline overlay for retro terminal feel */
body::before {
    content: '';
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: repeating-linear-gradient(
        0deg,
        transparent,
        transparent 2px,
        rgba(0, 0, 0, 0.02) 2px,
        rgba(0, 0, 0, 0.02) 4px
    );
    pointer-events: none;
    z-index: 9999;
}

/* Gradient scrollbar */
* {
    scrollbar-width: thin;
    scrollbar-color: var(--accent-cyan) var(--bg-abyss);
}
::-webkit-scrollbar {
    width: 6px;
    height: 6px;
}
::-webkit-scrollbar-track {
    background: var(--bg-abyss);
}
::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    border-radius: 3px;
}
::-webkit-scrollbar-thumb:hover {
    background: linear-gradient(180deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
}

/* Font size scaling */
.container {
    zoom: var(--font-scale);
}
@supports not (zoom: 1) {
    .container {
        transform: scale(var(--font-scale));
        transform-origin: top left;
    }
}

/* Deploy overlay */
.deploy-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(3, 7, 18, 0.95);
    backdrop-filter: blur(8px);
    z-index: 1000;
    justify-content: center;
    align-items: center;
    flex-direction: column;
    gap: 24px;
}
.deploy-overlay.visible { display: flex; }
.deploy-spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--border-dim);
    border-top-color: var(--accent-cyan);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 20px var(--glow-cyan);
}
@keyframes spin { to { transform: rotate(360deg); } }
.deploy-text {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 18px;
    color: var(--text-bright);
    text-align: center;
}
.deploy-subtext {
    font-size: 13px;
    color: var(--text-mid);
    text-align: center;
}
.deploy-countdown {
    font-size: 12px;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
}

body {
    font-family: 'Space Grotesk', -apple-system, BlinkMacSystemFont, sans-serif;
    background: var(--bg-abyss);
    color: var(--text-bright);
    min-height: 100vh;
    overflow-x: hidden;
    width: 100%;
    max-width: 100%;
    position: relative;
    -webkit-font-smoothing: antialiased;
}

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

/* Session cards */
.session-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    overflow: hidden;
    position: relative;
    break-inside: avoid;
    margin-bottom: 16px;
    max-width: 100%;
    transition: all 0.3s ease;
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

/* Terminal */
.terminal {
    background: var(--bg-abyss);
    padding: 12px;
    overflow-y: auto;
    overflow-x: hidden;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    line-height: 1.25;
    transition: height 0.25s ease-out;
    width: 100%;
    min-width: 0;
}
.terminal .line {
    white-space: break-spaces;
    word-break: break-word;
    overflow-wrap: anywhere;
    min-width: 0;
    max-width: 100%;
    display: block;
    margin-bottom: -0.25em;
}
.terminal .line:last-child {
    margin-bottom: 0;
}
/* Scroll mode */
.terminal.scroll-mode {
    overflow-x: auto;
}
.terminal.scroll-mode .line {
    white-space: pre;
    max-width: none;
    word-break: normal;
}
.terminal-scrollback {
    display: none;
    text-align: left;
    min-width: 0;
    width: 100%;
}
.terminal-scrollback.has-content {
    display: block;
}
.terminal-separator {
    display: none;
    text-align: center;
    color: var(--text-dim);
    font-size: 9px;
    margin: 10px 0;
    user-select: none;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.terminal-separator.visible {
    display: block;
}
.terminal-screen {
    min-width: 0;
    width: 100%;
}
.terminal .line:empty::before {
    content: ' ';
    white-space: pre;
}
.terminal .split-line {
    display: flex;
    justify-content: space-between;
    gap: 1em;
}
.terminal .split-line > span:first-child {
    white-space: pre;
    min-width: 0;
}
.terminal .split-line > span:last-child {
    white-space: pre;
    flex-shrink: 0;
}
.terminal .rule-line {
    white-space: pre;
    overflow: hidden;
}
.terminal span { box-decoration-break: clone; -webkit-box-decoration-break: clone; }
.terminal .ansi-bright { font-weight: bold; }
.terminal .ansi-dim { opacity: 0.5; }
.terminal .ansi-italic { font-style: italic; }
.terminal .ansi-underline { text-decoration: underline; }

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
}
.wh-stat {
    display: flex;
    align-items: center;
    gap: 4px;
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
    bottom: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%) translateY(4px);
    padding: 4px 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 4px;
    font-size: 10px;
    color: var(--text-bright);
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s ease, transform 0.15s ease;
    transition-delay: 0s;
    z-index: 1000;
    box-shadow: 0 2px 8px rgba(0,0,0,0.3);
}
[data-tooltip]::before {
    content: '';
    position: absolute;
    bottom: calc(100% + 2px);
    left: 50%;
    transform: translateX(-50%);
    border: 4px solid transparent;
    border-top-color: var(--border-dim);
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s ease;
    transition-delay: 0s;
    z-index: 1001;
}
[data-tooltip]:hover::after {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
    transition-delay: 0.5s;
}
[data-tooltip]:hover::before {
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
.container[data-layout="fit"] .widgets-content {
    grid-template-columns: 1fr;
}

/* Mobile responsive */
@media (max-width: 768px) {
    .widgets-content { grid-template-columns: 1fr !important; }
    .header {
        flex-wrap: wrap;
        padding: 12px 14px;
        gap: 10px;
    }
    .header h1 {
        font-size: 15px;
    }
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
    }
    .terminal {
        font-size: 10px;
        line-height: 15px;  /* Match desktop ratio for descender room */
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
    .refresh-btn {
        padding: 6px 10px;
        font-size: 11px;
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

/* Input area */
.input-area {
    padding: 14px;
    border-top: 1px solid var(--border-dim);
    display: flex;
    gap: 10px;
    background: var(--bg-card);
}
.input-area input {
    flex: 1;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 10px 14px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    transition: all 0.2s;
}
.input-area input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.input-area input::placeholder {
    color: var(--text-dim);
}
.input-area button {
    font-family: 'JetBrains Mono', monospace;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 8px;
    padding: 10px 20px;
    color: var(--bg-abyss);
    font-weight: 700;
    font-size: 12px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
}
.input-area button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 24px var(--glow-cyan);
}
.input-area button:disabled {
    background: var(--bg-surface);
    color: var(--text-dim);
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}

/* No sessions message */
.no-sessions {
    text-align: center;
    padding: 64px 24px;
    color: var(--text-mid);
    font-family: 'JetBrains Mono', monospace;
}

/* Refresh button */
.refresh-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    padding: 8px 14px;
    color: var(--text-mid);
    cursor: pointer;
    font-size: 12px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.refresh-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}

/* Style popover button */
.style-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 8px 14px;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.style-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
.style-btn.active {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 15px var(--glow-cyan);
}
.style-btn svg { width: 14px; height: 14px; }

/* Style popover */
.style-popover {
    display: none;
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 18px;
    min-width: 240px;
    box-shadow: 0 20px 48px rgba(0,0,0,0.5), 0 0 40px rgba(34, 211, 238, 0.1);
    z-index: 200;
}
.style-popover.visible { display: block; }
.style-section {
    margin-bottom: 18px;
}
.style-section:last-child { margin-bottom: 0; }
.style-section-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    font-weight: 600;
    color: var(--accent-magenta);
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 10px;
}
.style-options {
    display: flex;
    gap: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    overflow: hidden;
}
.style-option {
    font-family: 'JetBrains Mono', monospace;
    flex: 1;
    background: transparent;
    border: none;
    padding: 10px 12px;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    border-right: 1px solid var(--border-dim);
    transition: all 0.2s;
}
.style-option:last-child { border-right: none; }
.style-option:hover {
    background: var(--bg-card);
    color: var(--text-mid);
}
.style-option.active {
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    color: var(--bg-abyss);
    font-weight: 700;
}
.font-size-control {
    display: flex;
    align-items: center;
    gap: 10px;
}
.font-size-btn {
    font-family: 'JetBrains Mono', monospace;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    width: 38px;
    height: 38px;
    border-radius: 6px;
    color: var(--text-mid);
    cursor: pointer;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
}
.font-size-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
.font-size-btn:active {
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    color: var(--bg-abyss);
    border-color: transparent;
}
.font-size-btn.decrease { font-size: 12px; }
.font-size-btn.increase { font-size: 18px; }
.font-size-value {
    flex: 1;
    text-align: center;
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-bright);
    background: var(--bg-abyss);
    padding: 10px;
    border-radius: 6px;
    border: 1px solid var(--border-dim);
}
.style-container {
    position: relative;
}

/* Settings button and popover */
.settings-container {
    position: relative;
}
.settings-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 8px 14px;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.settings-btn:hover {
    border-color: var(--accent-magenta);
    color: var(--accent-magenta);
    background: rgba(232, 121, 249, 0.05);
}
.settings-btn.active {
    border-color: var(--accent-magenta);
    color: var(--accent-magenta);
    background: rgba(232, 121, 249, 0.1);
    box-shadow: 0 0 15px var(--glow-magenta);
}
.settings-btn svg { width: 14px; height: 14px; }
.settings-popover {
    display: none;
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 20px;
    min-width: 300px;
    box-shadow: 0 20px 48px rgba(0,0,0,0.5), 0 0 40px rgba(232, 121, 249, 0.1);
    z-index: 200;
}
.settings-popover.visible { display: block; }
.settings-section {
    margin-bottom: 0;
}
.settings-section-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-bright);
    margin-bottom: 8px;
}
.settings-description {
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 14px;
    line-height: 1.5;
}
.settings-divider {
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--border-dim), transparent);
    margin: 20px 0;
}
.settings-action-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    padding: 12px 18px;
    color: var(--bg-abyss);
    cursor: pointer;
    font-size: 12px;
    font-weight: 700;
    border-radius: 8px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.settings-action-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 24px var(--glow-cyan);
}
.settings-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}
.settings-danger-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    background: transparent;
    border: 1px solid rgba(248, 113, 113, 0.4);
    padding: 10px 18px;
    color: var(--accent-red);
    cursor: pointer;
    font-size: 11px;
    border-radius: 6px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.settings-danger-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    border-color: var(--accent-red);
    box-shadow: 0 0 15px rgba(248, 113, 113, 0.2);
}

/* Sessions popover container */
.sessions-container {
    position: relative;
    margin-left: auto;
}
.sessions-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 8px 14px;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.sessions-btn:hover {
    border-color: var(--accent-green);
    color: var(--accent-green);
    background: rgba(74, 222, 128, 0.05);
}
.sessions-btn.active {
    border-color: var(--accent-green);
    color: var(--accent-green);
    background: rgba(74, 222, 128, 0.1);
    box-shadow: 0 0 15px var(--glow-green);
}
.sessions-btn svg {
    transition: transform 0.2s;
}
.sessions-btn.active svg {
    transform: rotate(180deg);
}
.sessions-count {
    font-weight: 700;
    color: var(--accent-green);
}
.sessions-popover {
    display: none;
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    min-width: 320px;
    max-width: 400px;
    max-height: 70vh;
    overflow-y: auto;
    box-shadow: 0 20px 48px rgba(0,0,0,0.5), 0 0 40px rgba(74, 222, 128, 0.1);
    z-index: 200;
}
.sessions-popover.visible {
    display: block;
}
.sessions-group {
    border-bottom: 1px solid var(--border-dim);
}
.sessions-group:last-child {
    border-bottom: none;
}
.sessions-group-header {
    padding: 12px 16px;
    background: var(--bg-surface);
    display: flex;
    align-items: center;
    gap: 10px;
}
.sessions-group-name {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-cyan);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
}
.sessions-group-path {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
}
.sessions-group-count {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-dim);
    background: var(--bg-abyss);
    padding: 2px 8px;
    border-radius: 10px;
    flex-shrink: 0;
}
.session-item {
    padding: 10px 16px;
    cursor: pointer;
    transition: background 0.15s;
    border-bottom: 1px solid var(--border-dim);
}
.session-item:last-child {
    border-bottom: none;
}
.session-item:hover {
    background: var(--bg-surface);
}
.session-item.focused {
    background: rgba(74, 222, 128, 0.08);
    border-left: 2px solid var(--accent-green);
    padding-left: 14px;
}
.session-item.focused:hover {
    background: rgba(74, 222, 128, 0.12);
}
.session-item-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
}
.session-item-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-bright);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
}
.session-item-state {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    flex-shrink: 0;
}
.session-item-state.ready { background: rgba(74, 222, 128, 0.2); color: var(--accent-green); }
.session-item-state.thinking { background: rgba(59, 130, 246, 0.2); color: var(--accent-blue); }
.session-item-state.permission { background: rgba(251, 146, 60, 0.2); color: var(--accent-orange); }
.session-item-state.question { background: rgba(167, 139, 250, 0.2); color: var(--accent-purple); }
.session-item-state.complete { background: rgba(100, 116, 139, 0.2); color: var(--text-mid); }
.session-item-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-dim);
}
.session-item-branch {
    color: var(--accent-green);
}
.session-item-duration {
    color: var(--text-dim);
}
.session-item-duration::before {
    content: '•';
    margin-right: 8px;
    color: var(--border-dim);
}
.session-item-stats {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
}
.si-stat {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-bright);
}
.si-elapsed {
    color: var(--text-dim);
    font-size: 9px;
}
.sessions-empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
}
@media (max-width: 768px) {
    .sessions-popover {
        position: fixed;
        top: 95px;
        left: 16px;
        right: 16px;
        min-width: auto;
        max-width: none;
    }
    .sessions-btn .sessions-label {
        display: none;
    }
}

/* Invite result */
.invite-result {
    margin-top: 14px;
    padding: 16px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    display: none;
}
.invite-result.visible { display: block; }
.invite-code {
    font-family: 'JetBrains Mono', monospace;
    font-size: 24px;
    font-weight: 700;
    color: var(--accent-yellow);
    text-align: center;
    letter-spacing: 0.15em;
    margin-bottom: 10px;
    text-shadow: 0 0 20px rgba(251, 191, 36, 0.3);
}
.invite-hint {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-dim);
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.invite-link {
    margin-top: 10px;
    text-align: center;
}
.invite-link a {
    font-size: 11px;
    color: var(--accent-cyan);
    text-decoration: none;
    transition: all 0.2s;
}
.invite-link a:hover {
    color: var(--accent-magenta);
    text-decoration: none;
}

/* Layout-based container styles */
.container[data-layout="1"] { column-count: 1; }
.container[data-layout="2"] { column-count: 2; }
.container[data-layout="3"] { column-count: 3; }

/* Adjust terminal heights */
.container[data-layout="2"] .terminal { height: 250px; }
.container[data-layout="3"] .terminal { height: 200px; }
.container[data-layout="fit"] .terminal { height: 150px; }

/* Stack widgets in narrow layouts */
.container[data-layout="2"] .widgets-content,
.container[data-layout="3"] .widgets-content,
.container[data-layout="fit"] .widgets-content {
    grid-template-columns: 1fr;
}

/* Prompt panel (questions, permissions) */
.prompt-panel {
    display: none;
    padding: 18px;
    background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-deep) 100%);
    border-bottom: 1px solid var(--border-dim);
}
.prompt-panel.visible { display: block; }
.prompt-header {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-orange);
    font-size: 11px;
    font-weight: 600;
    margin-bottom: 10px;
    text-transform: uppercase;
    letter-spacing: 1px;
    display: flex;
    align-items: center;
    gap: 8px;
}
.prompt-header::before {
    content: '';
    width: 6px;
    height: 6px;
    background: var(--accent-orange);
    border-radius: 50%;
    box-shadow: 0 0 8px rgba(251, 146, 60, 0.5);
    animation: pulse-dot 2s ease-in-out infinite;
}
@keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}
.prompt-question {
    color: var(--text-bright);
    font-size: 14px;
    margin-bottom: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
}
.prompt-options {
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.prompt-option {
    font-family: 'JetBrains Mono', monospace;
    padding: 12px 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
}
.prompt-option:hover {
    background: var(--bg-card);
    border-color: var(--accent-cyan);
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-option.selected {
    border-color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.prompt-option-number {
    color: var(--text-dim);
    font-size: 11px;
    margin-right: 10px;
}
.prompt-option-label {
    color: var(--text-bright);
    font-weight: 500;
    font-size: 12px;
}
.prompt-option-desc {
    color: var(--text-mid);
    font-size: 11px;
    margin-top: 6px;
    padding-left: 24px;
}
.prompt-other {
    margin-top: 14px;
    display: flex;
    gap: 10px;
}
.prompt-other input {
    flex: 1;
    padding: 10px 14px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    transition: all 0.2s;
}
.prompt-other input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-other button {
    font-family: 'JetBrains Mono', monospace;
    padding: 10px 18px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 8px;
    color: var(--bg-abyss);
    font-weight: 700;
    font-size: 11px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
}
.prompt-other button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 20px var(--glow-cyan);
}

/* Inline tab instruction inputs */
.prompt-option-row {
    display: flex;
    align-items: center;
    gap: 10px;
}
.prompt-option-row .prompt-option {
    flex: 1;
    min-width: 0;
}
.prompt-tab-wrapper {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 0 0 auto;
}
.prompt-tab-input {
    font-family: 'JetBrains Mono', monospace;
    width: 180px;
    padding: 10px 12px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-size: 11px;
    transition: all 0.2s;
}
.prompt-tab-input:focus {
    border-color: var(--accent-cyan);
    outline: none;
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-tab-input::placeholder {
    color: var(--text-dim);
    font-size: 10px;
}
.prompt-tab-send {
    font-family: 'JetBrains Mono', monospace;
    padding: 8px 12px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 6px;
    color: var(--bg-abyss);
    font-weight: 700;
    font-size: 10px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
    white-space: nowrap;
}
.prompt-tab-send:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px var(--glow-cyan);
}
@media (max-width: 768px) {
    .prompt-option-row {
        flex-direction: column;
        align-items: stretch;
    }
    .prompt-tab-wrapper {
        margin-top: 6px;
    }
    .prompt-tab-input {
        flex: 1;
        width: auto;
    }
}

/* Pairing gate styles */
.pairing-gate {
    width: 100%;
    max-width: 420px;
    margin: 0 auto;
    padding: 32px 16px;
}
.pairing-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 16px;
    padding: 40px;
    text-align: center;
    position: relative;
    overflow: hidden;
}
.pairing-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-magenta), var(--accent-cyan));
    background-size: 200% 100%;
    animation: gradient-shift 3s ease infinite;
}
@keyframes gradient-shift {
    0%, 100% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
}
.pairing-icon {
    width: 72px;
    height: 72px;
    margin: 0 auto 28px;
    color: var(--accent-cyan);
    filter: drop-shadow(0 0 20px var(--glow-cyan));
}
.pairing-icon svg {
    width: 100%;
    height: 100%;
}
.pairing-icon.spinning svg {
    animation: spin 1s linear infinite;
}
.pairing-card h2 {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 26px;
    font-weight: 700;
    margin-bottom: 14px;
    color: var(--text-bright);
}
.pairing-description {
    color: var(--text-mid);
    font-size: 14px;
    line-height: 1.6;
    margin-bottom: 28px;
}
.pairing-form {
    display: flex;
    gap: 12px;
    margin-bottom: 18px;
}
.pairing-form input {
    flex: 1;
    min-width: 0;
    padding: 14px 18px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 10px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 20px;
    text-align: center;
    letter-spacing: 3px;
    text-transform: uppercase;
    transition: all 0.2s;
}
.pairing-form input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 25px var(--glow-cyan);
}
.pairing-form input::placeholder {
    color: var(--text-dim);
    letter-spacing: 3px;
    font-size: 14px;
}
.pairing-form button {
    font-family: 'JetBrains Mono', monospace;
    flex-shrink: 0;
    padding: 14px 28px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 10px;
    color: var(--bg-abyss);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.pairing-form button:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 30px var(--glow-cyan);
}
.pairing-form button:disabled {
    background: var(--bg-surface);
    color: var(--text-dim);
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}
.pairing-error {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-red);
    font-size: 12px;
    min-height: 20px;
    margin-bottom: 16px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.pairing-help {
    color: var(--text-dim);
    font-size: 12px;
}
.pairing-help code {
    font-family: 'JetBrains Mono', monospace;
    background: var(--bg-surface);
    padding: 3px 8px;
    border-radius: 4px;
    color: var(--accent-cyan);
}

/* Install card (shown on pairing page) */
.install-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 24px;
    margin-top: 24px;
    text-align: center;
}
.install-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin-bottom: 16px;
    color: var(--text-mid);
    font-size: 13px;
}
.install-icon {
    width: 18px;
    height: 18px;
    min-width: 18px;
    min-height: 18px;
    max-width: 18px;
    max-height: 18px;
    color: var(--accent-purple);
    flex-shrink: 0;
}
.install-icon svg {
    width: 100%;
    height: 100%;
}
.install-command {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 12px 16px;
    margin-bottom: 12px;
}
.install-command code {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    color: var(--accent-cyan);
    letter-spacing: 0.5px;
}
.install-copy-btn {
    background: transparent;
    border: none;
    padding: 6px;
    cursor: pointer;
    color: var(--text-dim);
    border-radius: 4px;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
}
.install-copy-btn:hover {
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
}
.install-hint {
    color: var(--text-dim);
    font-size: 12px;
    margin-bottom: 16px;
}
.install-hint code {
    font-family: 'JetBrains Mono', monospace;
    background: var(--bg-card);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-mid);
}
.install-link {
    display: inline-block;
    color: var(--accent-magenta);
    font-size: 12px;
    text-decoration: none;
    transition: all 0.2s;
}
.install-link:hover {
    color: var(--accent-cyan);
    text-decoration: underline;
}

/* Project grouping */
.container[data-grouping="project"] {
    column-count: 1 !important;
}
.project-group {
    margin-bottom: 20px;
    break-inside: avoid;
}
.project-group:last-child {
    margin-bottom: 0;
}
/* Apply column layout within each project group */
.container[data-grouping="project"][data-layout="2"] .project-sessions-content {
    column-count: 2;
    column-gap: 16px;
}
.container[data-grouping="project"][data-layout="3"] .project-sessions-content {
    column-count: 3;
    column-gap: 16px;
}
.container[data-grouping="project"][data-layout="fit"] .project-sessions-content {
    column-count: var(--group-fit-columns, 1);
    column-gap: 16px;
}
/* Restore card styling in multi-column project mode */
.container[data-grouping="project"][data-layout="2"] .session-card,
.container[data-grouping="project"][data-layout="3"] .session-card,
.container[data-grouping="project"][data-layout="fit"] .session-card {
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    margin-bottom: 16px;
    break-inside: avoid;
}
.container[data-grouping="project"][data-layout="2"] .project-sessions-inner,
.container[data-grouping="project"][data-layout="3"] .project-sessions-inner,
.container[data-grouping="project"][data-layout="fit"] .project-sessions-inner {
    border-left: none;
    margin-left: 0;
}
.container[data-grouping="project"][data-layout="2"] .project-sessions-content,
.container[data-grouping="project"][data-layout="3"] .project-sessions-content,
.container[data-grouping="project"][data-layout="fit"] .project-sessions-content {
    padding: 16px;
    padding-top: 10px;
}

/* Project separator */
.project-separator {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 22px;
    background: linear-gradient(180deg, var(--bg-abyss) 0%, var(--bg-deep) 50%, var(--bg-abyss) 100%);
    cursor: pointer;
    user-select: none;
    transition: all 0.2s;
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    margin: 0;
    position: relative;
}
.project-separator::before,
.project-separator::after {
    content: '';
    flex: 1;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--border-dim) 20%, var(--border-dim) 80%, transparent);
}
.project-separator:hover {
    background: linear-gradient(180deg, var(--bg-deep) 0%, var(--bg-card) 50%, var(--bg-deep) 100%);
}
.project-separator:active {
    background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-surface) 50%, var(--bg-card) 100%);
}
.project-separator-content {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
}
.project-collapse-icon {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-dim);
    font-size: 10px;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    background: var(--bg-surface);
    border-radius: 4px;
    border: 1px solid var(--border-dim);
}
.project-group.collapsed .project-collapse-icon {
    transform: rotate(-90deg);
}
.project-name {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 600;
    color: var(--accent-cyan);
    font-size: 12px;
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.project-path {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
}
.project-count {
    font-family: 'JetBrains Mono', monospace;
    color: var(--text-dim);
    font-size: 10px;
    flex-shrink: 0;
    background: var(--bg-surface);
    padding: 3px 10px;
    border-radius: 10px;
    border: 1px solid var(--border-dim);
}

/* Animated sessions container */
.project-sessions {
    display: grid;
    grid-template-rows: 1fr;
    transition: grid-template-rows 0.35s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
}
.project-sessions-inner {
    min-height: 0;
    overflow: hidden;
}
.project-group.collapsed .project-sessions {
    grid-template-rows: 0fr;
}
.project-group.collapsed .project-sessions-inner {
    min-height: 0;
}

/* Hide cwd when grouped */
.container[data-grouping="project"] .session-card .cwd {
    display: none;
}

/* Cards inside group */
.container[data-grouping="project"] .session-card {
    margin-bottom: 0;
    border-radius: 0;
    border-left: none;
    border-right: none;
    border-bottom: none;
}
.container[data-grouping="project"] .session-card:first-child {
    border-top: none;
}
.container[data-grouping="project"] .session-card::before {
    display: none;
}

/* Single column mode spacing */
.container[data-grouping="project"][data-layout="1"] .project-sessions-content {
    background: var(--bg-abyss);
}

/* Mobile adjustments for project groups */
@media (max-width: 768px) {
    .project-separator {
        padding: 14px 16px;
        gap: 10px;
    }
    .project-name {
        font-size: 11px;
    }
    .project-path {
        display: none;
    }
    .project-count {
        font-size: 9px;
        padding: 2px 8px;
    }
    /* Add spacing between cards in project groups on mobile */
    .container[data-grouping="project"] .session-card {
        margin-bottom: 8px;
        border-radius: 8px;
        border: 1px solid var(--border-dim);
    }
    .container[data-grouping="project"] .session-card:last-child {
        margin-bottom: 0;
    }
}

/* Usage display in settings popover */
.usage-display {
    padding: 16px 0;
}
.usage-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
}
.usage-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-bright);
}
.usage-time {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-mid);
}
.usage-bar-container {
    height: 8px;
    background: var(--bg-abyss);
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--border-dim);
}
.usage-bar {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue));
    border-radius: 3px;
    transition: width 0.3s ease, background 0.3s ease;
}
.usage-bar.warning {
    background: linear-gradient(90deg, var(--accent-orange), var(--accent-yellow));
}
.usage-bar.critical {
    background: linear-gradient(90deg, var(--accent-red), var(--accent-orange));
}
/* Pro subscriber card */
.pro-status-card {
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.08) 0%, rgba(59, 130, 246, 0.08) 100%);
    border: 1px solid rgba(34, 211, 238, 0.3);
    border-radius: 8px;
    padding: 14px 16px;
    position: relative;
    overflow: hidden;
}
.pro-status-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue), var(--accent-purple));
}
.pro-status-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}
.pro-status-icon {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-cyan), var(--accent-blue));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--bg-abyss);
    font-weight: 700;
}
.pro-status-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.3px;
}
.pro-status-sublabel {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-muted);
    margin-bottom: 12px;
}
.manage-subscription-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--accent-cyan);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: all 0.15s ease;
}
.manage-subscription-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
}
.manage-subscription-link:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}
.manage-subscription-link svg {
    width: 12px;
    height: 12px;
}
/* Legacy badge for non-card contexts */
.usage-pro-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    font-weight: 700;
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.15);
    border: 1px solid var(--accent-cyan);
    padding: 4px 10px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.upgrade-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    margin-top: 12px;
    padding: 10px 16px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 6px;
    color: var(--bg-abyss);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
}
.upgrade-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 6px 20px var(--glow-cyan);
}
.upgrade-btn.hidden {
    display: none;
}

/* Paywall overlay */
.paywall-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(3, 7, 18, 0.95);
    backdrop-filter: blur(12px);
    z-index: 2000;
    justify-content: center;
    align-items: flex-start;
    padding: 20px;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
}
.paywall-overlay.visible {
    display: flex;
}
.paywall-modal {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 20px;
    padding: 40px;
    max-width: 420px;
    width: 100%;
    text-align: center;
    position: relative;
    overflow: hidden;
    animation: paywall-enter 0.3s ease-out;
    margin: auto;
}
@keyframes paywall-enter {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}
.paywall-modal::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-magenta), var(--accent-cyan));
    background-size: 200% 100%;
    animation: gradient-shift 3s ease infinite;
}
.paywall-icon {
    width: 80px;
    height: 80px;
    margin: 0 auto 24px;
    font-size: 64px;
    line-height: 1;
}
.paywall-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 26px;
    font-weight: 700;
    color: var(--text-bright);
    margin-bottom: 12px;
}
.paywall-subtitle {
    font-size: 14px;
    color: var(--text-mid);
    margin-bottom: 24px;
    line-height: 1.6;
}
.paywall-usage {
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 24px;
}
.paywall-usage-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    color: var(--text-bright);
}
.paywall-usage-text .used {
    color: var(--accent-red);
}
.paywall-price {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 42px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    margin-bottom: 8px;
}
.paywall-price-period {
    font-size: 16px;
    color: var(--text-dim);
    margin-bottom: 24px;
}
.paywall-features {
    text-align: left;
    margin-bottom: 28px;
}
.paywall-feature {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    font-size: 13px;
    color: var(--text-mid);
}
.paywall-feature-icon {
    color: var(--accent-green);
    font-size: 14px;
}
.paywall-buttons {
    display: flex;
    flex-direction: column;
    gap: 12px;
}
.paywall-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    padding: 14px 24px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
}
.paywall-btn.stripe {
    background: linear-gradient(135deg, #635bff 0%, #5244e7 100%);
    border: none;
    color: white;
}
.paywall-btn.stripe:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(99, 91, 255, 0.4);
}
.paywall-btn.paypal {
    background: #ffc439;
    border: none;
    color: #003087;
}
.paywall-btn.paypal:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(255, 196, 57, 0.4);
}
.paywall-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none !important;
    box-shadow: none !important;
}
.paywall-dismiss {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    margin-top: 16px;
    padding: 8px;
    transition: color 0.2s;
}
.paywall-dismiss:hover {
    color: var(--text-mid);
}
.paywall-loading {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 0;
}
.paywall-loading.visible {
    display: flex;
}
.paywall-loading-spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border-dim);
    border-top-color: var(--accent-cyan);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}
.paywall-loading-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-mid);
}
.paywall-success {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 0;
}
.paywall-success.visible {
    display: flex;
}
.paywall-success-icon {
    width: 64px;
    height: 64px;
    background: rgba(74, 222, 128, 0.2);
    border: 2px solid var(--accent-green);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 32px;
    color: var(--accent-green);
    animation: success-pop 0.3s ease-out;
}
@keyframes success-pop {
    0% { transform: scale(0); }
    70% { transform: scale(1.1); }
    100% { transform: scale(1); }
}
.paywall-success-text {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 20px;
    font-weight: 600;
    color: var(--text-bright);
}
.paywall-error {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
}
.paywall-error.visible {
    display: flex;
}
.paywall-error-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-red);
    text-align: center;
}
.paywall-retry-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 10px 20px;
    border-radius: 6px;
    color: var(--text-mid);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
}
.paywall-retry-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
}

@media (max-width: 768px) {
    .paywall-modal {
        padding: 28px 20px;
        border-radius: 16px;
    }
    .paywall-title {
        font-size: 22px;
    }
    .paywall-price {
        font-size: 36px;
    }
}

/* Gift claim overlay */
.gift-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(3, 7, 18, 0.95);
    backdrop-filter: blur(12px);
    z-index: 2000;
    justify-content: center;
    align-items: center;
    padding: 20px;
}
.gift-overlay.visible {
    display: flex;
}
.gift-modal {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 20px;
    padding: 40px;
    max-width: 420px;
    width: 100%;
    text-align: center;
    position: relative;
    overflow: hidden;
    animation: gift-enter 0.4s ease-out;
}
@keyframes gift-enter {
    from {
        opacity: 0;
        transform: scale(0.9) translateY(20px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}
.gift-modal::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, var(--accent-green), var(--accent-cyan), var(--accent-green));
    background-size: 200% 100%;
    animation: gradient-shift 3s ease infinite;
}
.gift-icon {
    font-size: 72px;
    margin-bottom: 20px;
    animation: gift-bounce 1s ease-in-out infinite;
}
@keyframes gift-bounce {
    0%, 100% { transform: translateY(0) rotate(0deg); }
    25% { transform: translateY(-8px) rotate(-5deg); }
    75% { transform: translateY(-8px) rotate(5deg); }
}
.gift-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 28px;
    font-weight: 700;
    color: var(--text-bright);
    margin-bottom: 12px;
}
.gift-subtitle {
    font-size: 15px;
    color: var(--text-mid);
    margin-bottom: 24px;
    line-height: 1.6;
}
.gift-duration {
    display: inline-block;
    background: linear-gradient(135deg, rgba(74, 222, 128, 0.15) 0%, rgba(34, 211, 238, 0.15) 100%);
    border: 1px solid var(--accent-green);
    border-radius: 12px;
    padding: 16px 24px;
    margin-bottom: 24px;
}
.gift-duration-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 6px;
}
.gift-duration-value {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 32px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent-green) 0%, var(--accent-cyan) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}
.gift-code-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 20px;
}
.gift-code-value {
    color: var(--accent-cyan);
    font-weight: 600;
    letter-spacing: 1px;
}
.gift-claim-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    padding: 16px 24px;
    background: linear-gradient(135deg, var(--accent-green) 0%, var(--accent-cyan) 100%);
    border: none;
    border-radius: 12px;
    color: var(--bg-abyss);
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 12px;
}
.gift-claim-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 30px var(--glow-green);
}
.gift-claim-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}
.gift-dismiss {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 8px;
    transition: color 0.2s;
}
.gift-dismiss:hover {
    color: var(--text-mid);
}
.gift-loading {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    padding: 40px 0;
}
.gift-loading.visible {
    display: flex;
}
.gift-loading-spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--border-dim);
    border-top-color: var(--accent-green);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 20px var(--glow-green);
}
.gift-loading-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-mid);
}
.gift-success {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 0;
}
.gift-success.visible {
    display: flex;
}
.gift-success-icon {
    width: 72px;
    height: 72px;
    background: rgba(74, 222, 128, 0.2);
    border: 2px solid var(--accent-green);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 36px;
    color: var(--accent-green);
    animation: success-pop 0.3s ease-out;
}
.gift-success-text {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-bright);
    text-align: center;
    line-height: 1.5;
}
.gift-error {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
}
.gift-error.visible {
    display: flex;
}
.gift-error-icon {
    font-size: 48px;
}
.gift-error-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--accent-red);
    text-align: center;
}
.gift-retry-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 10px 20px;
    border-radius: 6px;
    color: var(--text-mid);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
}
.gift-retry-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
}

@media (max-width: 768px) {
    .gift-modal {
        padding: 28px 20px;
        border-radius: 16px;
    }
    .gift-title {
        font-size: 24px;
    }
    .gift-duration-value {
        font-size: 26px;
    }
}
`;
