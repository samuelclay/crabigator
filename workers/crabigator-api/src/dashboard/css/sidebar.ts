// Sidebar styles
export const sidebarCss = `
/* Dashboard layout wrapper */
.dashboard-layout {
    margin-top: var(--header-height, 67px);
    transition: padding 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* Sidebar - fixed position so it stays in viewport during scroll */
.sidebar {
    position: fixed;
    top: var(--header-height, 67px);
    left: 0;
    width: var(--sidebar-width, 300px);
    bottom: 0;
    background: var(--bg-deep);
    border-right: 1px solid var(--border-dim);
    display: flex;
    flex-direction: column;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.3s ease;
    z-index: 50;
    overflow: hidden;
}
/* Push main content right to make room for fixed sidebar */
.dashboard-layout {
    padding-left: var(--sidebar-width, 300px);
}
.dashboard-layout.sidebar-collapsed {
    padding-left: 0;
}
/* Right-positioned sidebar */
.dashboard-layout[data-sidebar-position="right"] .sidebar {
    left: auto;
    right: 0;
    border-right: none;
    border-left: 1px solid var(--border-dim);
}
.dashboard-layout[data-sidebar-position="right"] {
    padding-left: 0;
    padding-right: var(--sidebar-width, 300px);
}
.dashboard-layout[data-sidebar-position="right"].sidebar-collapsed {
    padding-right: 0;
}

/* Collapsed state */
.sidebar.collapsed {
    transform: translateX(-100%);
    pointer-events: none;
}
.dashboard-layout[data-sidebar-position="right"] .sidebar.collapsed {
    transform: translateX(100%);
}

/* Sidebar header - sticky */
.sidebar-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-dim);
    background: var(--bg-deep);
    position: sticky;
    top: 0;
    z-index: 10;
    flex-shrink: 0;
}
.sidebar-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent-cyan);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex: 1;
}
.sidebar-settings-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s;
    font-size: 10px;
    flex-shrink: 0;
}
.sidebar-collapse-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    font-size: 12px;
    flex-shrink: 0;
}
.sidebar-settings-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
.sidebar-settings-btn.active {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 10px rgba(34, 211, 238, 0.2);
}
.sidebar-collapse-btn:hover {
    border-color: var(--accent-red);
    color: var(--accent-red);
    background: rgba(248, 113, 113, 0.05);
}

/* Sidebar settings popover */
.sidebar-settings-popover {
    display: none;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border-dim);
    padding: 16px 14px;
    flex-shrink: 0;
    max-height: 50vh;
    overflow-y: auto;
}
.sidebar-settings-popover.visible {
    display: block;
}
.sidebar-settings-section {
    margin-bottom: 16px;
}
.sidebar-settings-section:last-child {
    margin-bottom: 0;
}
.sidebar-settings-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    font-weight: 600;
    color: var(--accent-cyan);
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 8px;
}

/* Checkbox list for stats */
.sidebar-checkbox-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
}
.sidebar-checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    transition: background 0.15s;
}
.sidebar-checkbox:hover {
    background: var(--bg-surface);
}
.sidebar-checkbox input[type="checkbox"] {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border: 1px solid var(--border-dim);
    border-radius: 3px;
    background: var(--bg-surface);
    cursor: pointer;
    position: relative;
    flex-shrink: 0;
    transition: all 0.2s;
}
.sidebar-checkbox input[type="checkbox"]:checked {
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border-color: transparent;
}
.sidebar-checkbox input[type="checkbox"]:checked::after {
    content: '✓';
    position: absolute;
    top: -1px;
    left: 2px;
    font-size: 10px;
    color: var(--bg-abyss);
    font-weight: 700;
}
.sidebar-checkbox-icon {
    font-size: 10px;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
}
.sidebar-checkbox-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-mid);
}

/* Sidebar scrollable content */
.sidebar-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
}

/* Drag resize handle */
.sidebar-resize-handle {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 5px;
    right: -3px;
    cursor: col-resize;
    z-index: 20;
    transition: background 0.2s;
}
.sidebar-resize-handle:hover,
.sidebar-resize-handle.active {
    background: var(--accent-cyan);
    opacity: 0.4;
}
.dashboard-layout[data-sidebar-position="right"] .sidebar-resize-handle {
    right: auto;
    left: -3px;
}

/* Active sidebar session */
.sidebar .session-item.sidebar-active {
    background: rgba(34, 211, 238, 0.08);
    border-left: 2px solid var(--accent-cyan);
    padding-left: 10px;
}

/* Show sidebar button in header */
.show-sidebar-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 8px 14px;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 11px;
    border-radius: 6px;
    display: none;
    align-items: center;
    gap: 8px;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    white-space: nowrap;
}
.show-sidebar-btn.visible {
    display: flex;
}
.show-sidebar-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
.show-sidebar-btn svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
}
.show-sidebar-count {
    font-weight: 700;
    color: var(--accent-cyan);
}

/* Session highlight animation when scrolled to */
@keyframes session-highlight {
    0% { outline: 2px solid transparent; outline-offset: 2px; }
    8% { outline: 2px solid var(--accent-cyan); outline-offset: 2px; box-shadow: 0 0 30px rgba(34, 211, 238, 0.4); }
    50% { outline: 2px solid var(--accent-cyan); outline-offset: 2px; box-shadow: 0 0 15px rgba(34, 211, 238, 0.2); }
    100% { outline: 2px solid transparent; outline-offset: 2px; box-shadow: none; }
}
.session-card.highlight {
    animation: session-highlight 2.5s ease-out;
    z-index: 5;
    position: relative;
}
/* For grouped sessions that don't have individual borders */
@keyframes session-highlight-grouped {
    0% { outline: 2px solid transparent; outline-offset: -2px; background: transparent; }
    8% { outline: 2px solid var(--accent-cyan); outline-offset: -2px; background: rgba(34, 211, 238, 0.15); box-shadow: 0 0 30px rgba(34, 211, 238, 0.3); }
    50% { outline: 2px solid var(--accent-cyan); outline-offset: -2px; background: rgba(34, 211, 238, 0.08); box-shadow: 0 0 10px rgba(34, 211, 238, 0.15); }
    100% { outline: 2px solid transparent; outline-offset: -2px; background: transparent; box-shadow: none; }
}
.container[data-grouping="project"] .session-card.highlight {
    animation: session-highlight-grouped 2.5s ease-out;
    z-index: 5;
    position: relative;
}

/* Density: compact */
.sidebar.compact .session-item {
    padding: 4px 12px;
}
.sidebar.compact .sessions-group-header {
    padding: 6px 12px;
}
.sidebar.compact .session-item-title {
    font-size: 10px;
}
.sidebar.compact .session-item-stats {
    font-size: 8px;
    gap: 8px;
    margin-top: 1px;
}
.sidebar.compact .sessions-device-header {
    padding: 6px 12px 2px;
    font-size: 9px;
}

/* Density: comfortable (default, no overrides needed) */

/* Mobile overlay - sidebar becomes overlay instead of pushing content */
@media (max-width: 768px) {
    .dashboard-layout {
        padding-left: 0 !important;
        padding-right: 0 !important;
    }
    .sidebar {
        z-index: 150;
        box-shadow: 20px 0 40px rgba(0, 0, 0, 0.5);
    }
    .dashboard-layout[data-sidebar-position="right"] .sidebar {
        box-shadow: -20px 0 40px rgba(0, 0, 0, 0.5);
    }
    .sidebar-backdrop {
        display: none;
        position: fixed;
        top: var(--header-height, 67px);
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.5);
        z-index: 149;
    }
    .sidebar-backdrop.visible {
        display: block;
    }
    .sidebar-resize-handle {
        display: none;
    }
}
`;
