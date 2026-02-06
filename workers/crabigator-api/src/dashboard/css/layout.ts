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
    border-radius: 12px;
    margin-bottom: 16px;
    break-inside: avoid;
}
.container[data-grouping="project"][data-layout="2"] .session-header,
.container[data-grouping="project"][data-layout="3"] .session-header,
.container[data-grouping="project"][data-layout="4"] .session-header,
.container[data-grouping="project"][data-layout="fit"] .session-header {
    border-radius: 11px 11px 0 0;
}
.container[data-grouping="project"][data-layout="2"] .input-area,
.container[data-grouping="project"][data-layout="3"] .input-area,
.container[data-grouping="project"][data-layout="4"] .input-area,
.container[data-grouping="project"][data-layout="fit"] .input-area {
    border-radius: 0 0 11px 11px;
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
`;
