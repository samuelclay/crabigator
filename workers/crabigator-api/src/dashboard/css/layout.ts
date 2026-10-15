// Layout styles, project grouping
export const layoutCss = `
/* Layout-based container styles */
.container[data-layout="1"] { column-count: 1; }
.container[data-layout="2"] { column-count: 2; }
.container[data-layout="3"] { column-count: 3; }
.container[data-layout="4"] { column-count: 4; }

/* Adjust terminal heights */
.container[data-layout="2"] .terminal { height: 250px; }
.container[data-layout="3"] .terminal { height: 200px; }
.container[data-layout="4"] .terminal { height: 150px; }
.container[data-layout="fit"] .terminal { height: 150px; }

/* Stack widgets in narrow layouts */
.container[data-layout="2"] .widgets-content,
.container[data-layout="3"] .widgets-content,
.container[data-layout="4"] .widgets-content,
.container[data-layout="fit"] .widgets-content {
    grid-template-columns: 1fr;
}

/* Project grouping */
.container[data-grouping="project"] {
    column-count: 1 !important;
}
.project-group {
    margin-bottom: 12px;
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
.container[data-grouping="project"][data-layout="4"] .project-sessions-content {
    column-count: 4;
    column-gap: 16px;
}
.container[data-grouping="project"][data-layout="fit"] .project-sessions-content {
    column-count: var(--group-fit-columns, 1);
    column-gap: 16px;
}
/* Restore card styling in multi-column project mode */
.container[data-grouping="project"][data-layout="2"] .session-card,
.container[data-grouping="project"][data-layout="3"] .session-card,
.container[data-grouping="project"][data-layout="4"] .session-card,
.container[data-grouping="project"][data-layout="fit"] .session-card {
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    margin-bottom: 16px;
    break-inside: avoid;
}
.container[data-grouping="project"][data-layout="2"] .session-header,
.container[data-grouping="project"][data-layout="3"] .session-header,
.container[data-grouping="project"][data-layout="4"] .session-header,
.container[data-grouping="project"][data-layout="fit"] .session-header {
    border-radius: 7px 7px 0 0;
}
.container[data-grouping="project"][data-layout="2"] .input-area,
.container[data-grouping="project"][data-layout="3"] .input-area,
.container[data-grouping="project"][data-layout="4"] .input-area,
.container[data-grouping="project"][data-layout="fit"] .input-area {
    border-radius: 0 0 7px 7px;
}
.container[data-grouping="project"][data-layout="2"] .project-sessions-inner,
.container[data-grouping="project"][data-layout="3"] .project-sessions-inner,
.container[data-grouping="project"][data-layout="4"] .project-sessions-inner,
.container[data-grouping="project"][data-layout="fit"] .project-sessions-inner {
    border-left: none;
    margin-left: 0;
}
.container[data-grouping="project"][data-layout="2"] .project-sessions-content,
.container[data-grouping="project"][data-layout="3"] .project-sessions-content,
.container[data-grouping="project"][data-layout="4"] .project-sessions-content,
.container[data-grouping="project"][data-layout="fit"] .project-sessions-content {
    padding: 10px 12px 0;
}

/* Project separator */
.project-separator {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    background: var(--bg-deep);
    cursor: pointer;
    user-select: none;
    transition: all 0.2s;
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    margin: 0;
    position: relative;
}
.project-separator:hover {
    background: var(--bg-deep);
}
.project-separator:active {
    background: var(--bg-deep);
}
.project-separator-content {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
    position: relative;
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
    text-transform: none;
    letter-spacing: 0;
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
.project-add-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    color: var(--text-dim);
    font-size: 14px;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    flex-shrink: 0;
    margin-left: auto;
}
.project-add-btn:hover {
    background: var(--bg-surface);
    color: var(--accent-green);
    border-color: var(--accent-green);
}
.spawn-menu {
    display: none;
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    flex-direction: column;
    min-width: 148px;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 6px;
    box-shadow: 0 16px 40px rgba(0,0,0,0.45);
    z-index: 30;
}
.spawn-menu.visible {
    display: flex;
}
.spawn-menu button {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: none;
    color: var(--text-dim);
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 11px;
}
.spawn-menu button:hover {
    background: var(--bg-surface);
    color: var(--accent-cyan);
}
.project-close-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    color: var(--text-dim);
    font-size: 14px;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s;
    flex-shrink: 0;
}
.project-close-btn:hover {
    background: var(--bg-surface);
    color: var(--accent-red, #f85149);
    border-color: var(--accent-red, #f85149);
}
.project-group.empty {
    opacity: 0.6;
}
.project-group.empty:hover {
    opacity: 0.85;
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
.container[data-grouping="project"] .session-header {
    border-radius: 0;
}
.container[data-grouping="project"] .input-area {
    border-radius: 0;
}
.container[data-grouping="project"] .session-card:first-child {
    border-top: none;
}

/* Single column mode spacing */
.container[data-grouping="project"][data-layout="1"] .project-sessions-content {
    background: var(--bg-abyss);
}

/* Device grouping (wraps project groups when multiple devices) */
.device-group {
    margin-bottom: 12px;
}
.device-group:last-child {
    margin-bottom: 0;
}
.device-separator {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    background: var(--bg-deep);
    cursor: pointer;
    user-select: none;
    transition: all 0.2s;
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    position: relative;
}
.device-separator:hover {
    background: var(--bg-deep);
}
.device-separator-content {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
}
.device-collapse-icon {
    font-size: 8px;
    color: var(--accent-purple);
    transition: opacity 0.3s;
}
.device-group.collapsed .device-collapse-icon {
    opacity: 0.5;
}
.device-name {
    font-family: 'JetBrains Mono', monospace;
    font-weight: 700;
    color: var(--accent-purple);
    font-size: 11px;
    flex-shrink: 0;
    text-transform: none;
    letter-spacing: 0;
}
.device-count {
    font-family: 'JetBrains Mono', monospace;
    color: var(--text-dim);
    font-size: 9px;
    flex-shrink: 0;
    background: var(--bg-surface);
    padding: 2px 8px;
    border-radius: 10px;
    border: 1px solid var(--border-dim);
}
.device-sessions {
    display: grid;
    grid-template-rows: 1fr;
    transition: grid-template-rows 0.35s cubic-bezier(0.4, 0, 0.2, 1);
    overflow: hidden;
}
.device-sessions-inner {
    min-height: 0;
    overflow: hidden;
}
.device-group.collapsed .device-sessions {
    grid-template-rows: 0fr;
}
/* Nested project separators inside device groups get subtler styling */
.device-group .project-separator {
    border-top: none;
}
.device-group .project-group:first-child .project-separator {
    border-top: none;
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
    .container[data-grouping="project"] .session-header {
        border-radius: 7px 7px 0 0;
    }
    .container[data-grouping="project"] .input-area {
        border-radius: 0 0 7px 7px;
    }
    .container[data-grouping="project"] .session-card:last-child {
        margin-bottom: 0;
    }
    .terminal {
        line-height: 1.4;
    }
}

/* Phones: grouped sessions also run edge-to-edge */
@media (max-width: 600px) {
    /* Keep desktop column preferences without squeezing phone sessions. */
    .container[data-layout],
    .container[data-grouping][data-layout] .project-sessions-content {
        column-count: 1 !important;
    }
    .container[data-grouping="project"] .session-card,
    .container[data-grouping="project"][data-layout="2"] .session-card,
    .container[data-grouping="project"][data-layout="3"] .session-card,
    .container[data-grouping="project"][data-layout="4"] .session-card,
    .container[data-grouping="project"][data-layout="fit"] .session-card {
        border-left: none;
        border-right: none;
        border-radius: 0;
    }
    .container[data-grouping="project"] .session-header,
    .container[data-grouping="project"][data-layout="2"] .session-header,
    .container[data-grouping="project"][data-layout="3"] .session-header,
    .container[data-grouping="project"][data-layout="4"] .session-header,
    .container[data-grouping="project"][data-layout="fit"] .session-header,
    .container[data-grouping="project"] .input-area,
    .container[data-grouping="project"][data-layout="2"] .input-area,
    .container[data-grouping="project"][data-layout="3"] .input-area,
    .container[data-grouping="project"][data-layout="4"] .input-area,
    .container[data-grouping="project"][data-layout="fit"] .input-area {
        border-radius: 0;
    }
    .container[data-grouping="project"][data-layout="2"] .project-sessions-content,
    .container[data-grouping="project"][data-layout="3"] .project-sessions-content,
    .container[data-grouping="project"][data-layout="4"] .project-sessions-content,
    .container[data-grouping="project"][data-layout="fit"] .project-sessions-content {
        padding-left: 0;
        padding-right: 0;
    }
}
`;
