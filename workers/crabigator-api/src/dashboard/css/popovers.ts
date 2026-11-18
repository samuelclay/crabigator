// Account button, sessions popover, invite result
export const popoversCss = `
/* The Account button. Its popover is styled in view-panel.ts. */
.settings-container {
    position: relative;
}
.settings-btn {
    font-family: var(--font-ui);
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
    text-transform: none;
    letter-spacing: 0;
}
.settings-btn svg { width: 14px; height: 14px; }

/* Sessions popover container: PR board button + sessions button in a row */
.sessions-container {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
}
.sessions-btn {
    font-family: var(--font-ui);
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
    text-transform: none;
    letter-spacing: 0;
    user-select: none;
    -webkit-user-select: none;
}
@media (hover: hover) and (pointer: fine) {
    .sessions-btn:hover {
        border-color: var(--accent-cyan);
        color: var(--accent-cyan);
        background: rgba(34, 211, 238, 0.05);
    }
}
.sessions-btn:active {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
.sessions-btn.active {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: none;
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
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-cyan);
    text-transform: none;
    letter-spacing: 0;
    flex-shrink: 0;
}
.sessions-group-path {
    font-family: var(--font-ui);
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
    font-family: var(--font-ui);
    font-size: 9px;
    color: var(--text-dim);
    background: var(--bg-abyss);
    padding: 1px 6px;
    border-radius: 8px;
    flex-shrink: 0;
}
.sessions-device-header {
    padding: 10px 12px 4px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 700;
    color: var(--computer-color);
    text-transform: none;
    letter-spacing: 0;
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
    color: var(--computer-color);
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
@media (hover: hover) and (pointer: fine) {
.session-item:hover {
    background: var(--bg-surface);
}
}
.session-item.focused {
    background: rgba(74, 222, 128, 0.08);
    border-left: 2px solid var(--accent-green);
    padding-left: 10px;
}
@media (hover: hover) and (pointer: fine) {
.session-item.focused:hover {
    background: rgba(74, 222, 128, 0.12);
}
}
.session-item-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
}
.session-item-ident {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    flex: 1;
    min-width: 0;
}
.session-item-titles {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
}
.session-item-title {
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-bright);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.session-item-title.official { color: var(--text-bright); }
.session-item-generated-title {
    color: var(--accent-blue);
    font-family: var(--font-ui);
    font-size: 9px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.session-item-state {
    font-family: var(--font-ui);
    font-size: 8px;
    padding: 1px 5px;
    border-radius: 3px;
    text-transform: none;
    flex-shrink: 0;
    letter-spacing: 0.3px;
}
.session-item-state.ready { background: rgba(74, 222, 128, 0.2); color: var(--accent-green); }
.session-item-state.thinking { background: rgba(59, 130, 246, 0.2); color: var(--accent-blue); }
.session-item-state.permission { background: rgba(251, 146, 60, 0.2); color: var(--accent-orange); }
.session-item-state.question { background: rgba(167, 139, 250, 0.2); color: var(--accent-purple); }
.session-item-state.complete { background: rgba(100, 116, 139, 0.15); color: var(--text-dim); }
.session-item-lines {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 3px;
    min-width: 0;
}
.session-item-lines .row-line { font-size: 10px; }
.sessions-empty {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-dim);
    font-family: var(--font-ui);
    font-size: 11px;
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
    font-family: var(--font-ui);
    font-size: 24px;
    font-weight: 700;
    color: var(--accent-yellow);
    text-align: center;
    letter-spacing: 0.15em;
    margin-bottom: 10px;
    text-shadow: 0 0 20px rgba(251, 191, 36, 0.3);
}
.invite-hint {
    font-family: var(--font-ui);
    font-size: 10px;
    color: var(--text-dim);
    text-align: center;
    text-transform: none;
    letter-spacing: 0;
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
@media (hover: hover) and (pointer: fine) {
.invite-link a:hover {
    color: var(--accent-magenta);
    text-decoration: none;
}
}
`;
