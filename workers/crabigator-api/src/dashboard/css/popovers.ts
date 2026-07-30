// Style popover, settings popover, sessions popover, invite result
export const popoversCss = `
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
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
.sessions-btn.active {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 15px var(--glow-cyan);
}
.sessions-count {
    font-weight: 700;
    color: var(--accent-cyan);
}
.sessions-group {
    border-bottom: 1px solid var(--border-dim);
}
.sessions-group:last-child {
    border-bottom: none;
}
.sessions-group-header {
    padding: 8px 12px;
    background: var(--bg-surface);
    display: flex;
    align-items: center;
    gap: 8px;
}
.sessions-group-name {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
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
    opacity: 0.6;
}
.sessions-group-count {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    color: var(--text-dim);
    background: var(--bg-abyss);
    padding: 1px 6px;
    border-radius: 8px;
    flex-shrink: 0;
}
.sessions-device-header {
    padding: 10px 12px 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    font-weight: 700;
    color: var(--accent-purple);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    gap: 6px;
    border-top: 1px solid var(--border-dim);
}
.sessions-device-header:first-child {
    border-top: none;
}
.sessions-device-dot {
    font-size: 7px;
    color: var(--accent-purple);
}
.sessions-device-section {
    border-bottom: 1px solid var(--border-dim);
    padding-bottom: 4px;
}
.sessions-device-section:last-child {
    border-bottom: none;
    padding-bottom: 0;
}
.sessions-device-projects {
    margin-left: 12px;
    border-left: 2px solid rgba(139, 92, 246, 0.12);
}
.sessions-device-projects .sessions-group {
    border-bottom: none;
}
.session-item {
    padding: 6px 12px;
    cursor: pointer;
    transition: background 0.15s;
    border-bottom: 1px solid rgba(255,255,255,0.03);
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
    padding-left: 10px;
}
.session-item.focused:hover {
    background: rgba(74, 222, 128, 0.12);
}
.session-item-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
}
.session-item-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-bright);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
}
.session-item-state {
    font-family: 'JetBrains Mono', monospace;
    font-size: 8px;
    padding: 1px 5px;
    border-radius: 3px;
    text-transform: uppercase;
    flex-shrink: 0;
    letter-spacing: 0.3px;
}
.session-item-state.ready { background: rgba(74, 222, 128, 0.2); color: var(--accent-green); }
.session-item-state.thinking { background: rgba(59, 130, 246, 0.2); color: var(--accent-blue); }
.session-item-state.permission { background: rgba(251, 146, 60, 0.2); color: var(--accent-orange); }
.session-item-state.question { background: rgba(167, 139, 250, 0.2); color: var(--accent-purple); }
.session-item-state.complete { background: rgba(100, 116, 139, 0.15); color: var(--text-dim); }
.session-item-stats {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 2px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
}
.session-item-stats.dim {
    opacity: 0.3;
}
.si-stat {
    display: flex;
    align-items: center;
    gap: 3px;
    color: var(--text-mid);
}
.si-icon {
    font-size: 8px;
}
.si-elapsed {
    color: var(--text-dim);
    font-size: 8px;
    margin-left: 1px;
}
.sessions-empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
}
@media (max-width: 768px) {
    .style-popover,
    .settings-popover {
        max-height: calc(100vh - var(--header-height, 46px) - 16px);
        max-height: calc(100dvh - var(--header-height, 46px) - 16px);
        overflow-y: auto;
        overscroll-behavior: contain;
        -webkit-overflow-scrolling: touch;
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
`;
