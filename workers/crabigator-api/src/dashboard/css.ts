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
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-dim);
    display: flex;
    align-items: flex-start;
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
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    flex-shrink: 0;
}
.session-actions-row {
    display: flex;
    align-items: center;
    gap: 6px;
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

/* Collapsed state */
.session-card.collapsed .terminal,
.session-card.collapsed .prompt-panel,
.session-card.collapsed .widgets-panel,
.session-card.collapsed .input-area,
.session-card.collapsed .info-popover { display: none !important; }
.session-card.collapsed .session-header { border-bottom: none; }
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
    line-height: 1.4;
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
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--bg-card);
    cursor: pointer;
    user-select: none;
    gap: 10px;
    border-bottom: 1px solid var(--border-dim);
    transition: background 0.2s;
}
.widgets-header:hover { background: var(--bg-surface); }
.widgets-header .collapse-btn {
    padding: 2px 6px;
    font-size: 10px;
    flex-shrink: 0;
}
.widgets-summary {
    display: flex;
    align-items: center;
    gap: 14px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    min-width: 0;
    flex: 1;
}
.widgets-summary-title {
    color: var(--accent-cyan);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex-shrink: 1;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.widgets-summary-stats {
    color: var(--text-mid);
    display: flex;
    gap: 8px;
    flex-shrink: 0;
}
.widgets-summary-stats .sep { color: var(--border-dim); }
.widgets-summary-git {
    flex-shrink: 0;
    display: flex;
    gap: 8px;
}
.widgets-summary-git .green { color: var(--accent-green); }
.widgets-summary-git .red { color: var(--accent-red); }

/* Widgets content grid */
.widgets-content {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
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

/* 2-column grid when changes hidden */
.widgets-content.no-changes { grid-template-columns: repeat(2, 1fr); }

/* Title history widget */
.title-history-widget {
    grid-column: 1 / -1;
}
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
    }
    .session-header {
        padding: 12px 14px;
        gap: 10px;
    }
    .terminal {
        font-size: 10px;
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
.prompt-tab-input {
    font-family: 'JetBrains Mono', monospace;
    flex: 0 0 180px;
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
@media (max-width: 768px) {
    .prompt-option-row {
        flex-direction: column;
        align-items: stretch;
    }
    .prompt-tab-input {
        flex: none;
        width: 100%;
        margin-top: 6px;
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

/* Left border accent for single column mode */
.container[data-grouping="project"][data-layout="1"] .project-sessions-content {
    border-left: 3px solid var(--accent-cyan);
    margin-left: 10px;
    background: var(--bg-abyss);
    transition: border-color 0.2s ease;
}
.container[data-grouping="project"][data-layout="1"] .project-group.collapsed .project-sessions-content {
    border-left-color: var(--border-dim);
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
    .container[data-grouping="project"][data-layout="1"] .project-sessions-content {
        margin-left: 6px;
        border-left-width: 2px;
    }
}
`;
