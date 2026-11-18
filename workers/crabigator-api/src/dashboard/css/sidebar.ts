// Sidebar styles
export const sidebarCss = `
.session-item-computer { color: var(--computer-color); font-size: 11px; }
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
    opacity: 0;
}
.sidebar.popover-mode.collapsed {
    transform: translateY(-10px);
}
.dashboard-layout[data-sidebar-position="right"] .sidebar.collapsed:not(.popover-mode) {
    transform: translateX(100%);
}

/* Popover mode - dropdown from sessions button */
.sidebar.popover-mode {
    position: fixed;
    top: calc(var(--header-height, 67px) + 10px);
    right: 16px;
    left: auto;
    bottom: auto;
    max-height: calc(100vh - var(--header-height, 67px) - 32px);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    box-shadow: none;
    z-index: 200;
}
/* Don't push content in popover mode */
.dashboard-layout.sidebar-popover {
    padding-left: 0 !important;
    padding-right: 0 !important;
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
    font-family: var(--font-ui);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-bright);
    text-transform: none;
    letter-spacing: 0;
    flex: 1;
}
.sidebar-pin-btn,
.sidebar-close-btn {
    font-family: var(--font-ui);
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
.sidebar-close-btn {
    width: 28px;
    padding: 0;
    justify-content: center;
    font-size: 12px;
}
.sidebar-pin-btn svg { width: 12px; height: 12px; flex: none; }
/* Show pin button in popover mode, close button in pinned mode */
.sidebar.popover-mode .sidebar-pin-btn { display: flex; }
.sidebar.popover-mode .sidebar-close-btn { display: none; }
.sidebar:not(.popover-mode) .sidebar-pin-btn { display: none; }
.sidebar:not(.popover-mode) .sidebar-close-btn { display: flex; }
@media (hover: hover) and (pointer: fine) {
.sidebar-pin-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
}
@media (hover: hover) and (pointer: fine) {
.sidebar-close-btn:hover {
    border-color: var(--accent-red);
    color: var(--accent-red);
    background: rgba(248, 113, 113, 0.05);
}
}

/* Shimmer placeholder for loading titles */
@keyframes sidebar-shimmer {
    0% { background-position: -100px 0; }
    100% { background-position: 200px 0; }
}
.sidebar-shimmer {
    display: inline-block;
    width: 80px;
    height: 10px;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--bg-surface) 0%, var(--border-dim) 50%, var(--bg-surface) 100%);
    background-size: 200px 100%;
    animation: sidebar-shimmer 1.5s ease-in-out infinite;
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
.sidebar-resize-handle.active {
    background: var(--accent-cyan);
    opacity: 0.4;
}
@media (hover: hover) and (pointer: fine) {
.sidebar-resize-handle:hover {
    background: var(--accent-cyan);
    opacity: 0.4;
}
}
.dashboard-layout[data-sidebar-position="right"] .sidebar-resize-handle {
    right: auto;
    left: -3px;
}
.sidebar.popover-mode .sidebar-resize-handle {
    display: none;
}

/* Indent session items from project group headers */
.sidebar .sessions-group .session-item {
    margin-left: 12px;
    border-left: 2px solid rgba(34, 211, 238, 0.08);
}

/* Active sidebar session */
.sidebar .session-item.sidebar-active {
    background: #23313b;
    border-left: 2px solid var(--accent-cyan);
}
.sidebar .session-item.focused {
    background: #2b302f;
    border-left: 2px solid var(--accent-peach);
}
.sidebar .session-item.focused.sidebar-active {
    background: #2b302f;
}

/* Sessions button hidden when sidebar is pinned (toggled via JS) */
.sessions-container.sidebar-pinned-hidden {
    display: none;
}

/* Session highlight animation when scrolled to */
@keyframes session-highlight {
    0% { outline: 2px solid transparent; outline-offset: 2px; }
    8% { outline: 2px solid var(--accent-cyan); outline-offset: 2px; box-shadow: none; }
    50% { outline: 2px solid var(--accent-cyan); outline-offset: 2px; box-shadow: none; }
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
    8% { outline: 2px solid var(--accent-cyan); outline-offset: -2px; background: rgba(34, 211, 238, 0.15); box-shadow: none; }
    50% { outline: 2px solid var(--accent-cyan); outline-offset: -2px; background: #23313b; box-shadow: none; }
    100% { outline: 2px solid transparent; outline-offset: -2px; background: transparent; box-shadow: none; }
}
.container[data-grouping="project"] .session-card.highlight {
    animation: session-highlight-grouped 2.5s ease-out;
    z-index: 5;
    position: relative;
}

/* Row spacing in the session list: Tight, Normal (the default) and Relaxed. */
body[data-row-spacing="compact"] .sidebar .session-item {
    padding: 4px 12px;
}
body[data-row-spacing="compact"] .sidebar .sessions-group-header {
    padding: 6px 12px;
}
body[data-row-spacing="compact"] .sidebar .session-item-title {
    font-size: 10px;
}
body[data-row-spacing="compact"] .sidebar .session-item-generated-title {
    font-size: 8px;
}
body[data-row-spacing="compact"] .sidebar .session-item-lines {
    gap: 1px;
    margin-top: 1px;
}
body[data-row-spacing="compact"] .sidebar .session-item-lines .row-line {
    font-size: 8px;
}
body[data-row-spacing="compact"] .sidebar .sessions-device-header {
    padding: 6px 12px 2px;
    font-size: 9px;
}
body[data-row-spacing="relaxed"] .sidebar .session-item {
    padding: 11px 12px;
}
body[data-row-spacing="relaxed"] .sidebar .session-item-lines {
    gap: 4px;
    margin-top: 6px;
}

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
