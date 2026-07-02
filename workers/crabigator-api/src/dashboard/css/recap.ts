// Recap card + history widget styling. The recap card sits inside the
// session body, directly under the terminal screen, so it reads as a
// "what just happened" handoff for the live PTY view above it.
export const recapCss = `
.session-recap {
    display: grid;
    grid-template-columns: auto 1fr auto;
    grid-template-rows: auto auto;
    grid-template-areas:
        "status headline meta"
        ".      bullets  bullets";
    column-gap: 10px;
    row-gap: 4px;
    padding: 10px 16px 12px;
    background:
        linear-gradient(135deg, rgba(34, 211, 238, 0.06), rgba(34, 211, 238, 0) 60%),
        var(--bg-deep);
    border-top: 1px solid rgba(34, 211, 238, 0.18);
    cursor: pointer;
    position: relative;
    transition: background 0.15s ease;
    max-height: 92px;
    overflow: hidden;
}
.session-recap:hover {
    background:
        linear-gradient(135deg, rgba(34, 211, 238, 0.10), rgba(34, 211, 238, 0) 60%),
        var(--bg-deep);
}
.session-recap::before {
    content: '';
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 2px;
    background: linear-gradient(180deg, var(--accent-cyan), rgba(34, 211, 238, 0.05));
    opacity: 0.85;
}
.session-recap.empty,
.session-recap.disabled,
.session-recap.waiting:not(.has-content) {
    display: none;
}

.session-recap-status {
    grid-area: status;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    white-space: nowrap;
    padding-top: 1px;
}
.session-recap-status .rs-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    box-shadow: 0 0 8px currentColor;
    flex: 0 0 auto;
}
.session-recap-status .rs-label {
    font-weight: 600;
}

.session-recap-headline {
    grid-area: headline;
    color: var(--text-bright);
    font-size: 13px;
    line-height: 1.4;
    font-weight: 500;
    /* The headline can wrap to two lines before the card itself caps height. */
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
}

.session-recap-bullets {
    grid-area: bullets;
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--text-mid);
    font-size: 11.5px;
    line-height: 1.4;
    overflow: hidden;
}
.session-recap-bullets .rb-line {
    position: relative;
    padding-left: 12px;
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
}
.session-recap-bullets .rb-line::before {
    content: '·';
    position: absolute;
    left: 2px;
    color: var(--accent-cyan);
    opacity: 0.65;
}
.session-recap-bullets .rb-hint {
    color: var(--text-dim);
    font-style: italic;
}

.session-recap-meta {
    grid-area: meta;
    align-self: start;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    font-size: 10px;
    color: var(--text-dim);
    white-space: nowrap;
}
.session-recap-meta .rd-age { color: var(--text-dim); }
.session-recap-meta .rd-delta { letter-spacing: 0.3px; }
.session-recap-meta .rd-add { color: var(--accent-green); }
.session-recap-meta .rd-del { color: var(--accent-red); }

/* Distinct status accents pick up the appropriate left rail and label color. */
.session-recap.failed::before {
    background: linear-gradient(180deg, var(--accent-yellow), rgba(251, 191, 36, 0.05));
}
.session-recap.failed {
    background:
        linear-gradient(135deg, rgba(251, 191, 36, 0.07), rgba(251, 191, 36, 0) 60%),
        var(--bg-deep);
}
.session-recap.updating::before {
    background: linear-gradient(180deg, var(--accent-yellow), rgba(251, 191, 36, 0.05));
    animation: recap-pulse 1.6s ease-in-out infinite;
}
.session-recap.missing-key::before {
    background: linear-gradient(180deg, var(--text-mid), rgba(148, 163, 184, 0.05));
}
@keyframes recap-pulse {
    0%, 100% { opacity: 0.45; }
    50% { opacity: 1; }
}

/* Recap history widget — sits in widgets-content next to title-history. */
.recap-history-widget .widget-title { display: flex; justify-content: space-between; align-items: baseline; }
.recap-history-widget .recap-history-count {
    color: var(--text-dim);
    font-size: 11px;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
}
.recap-history-widget .recaps-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 320px;
    overflow-y: auto;
    padding-right: 4px;
}
.recap-history-widget .recap-entry {
    padding: 8px 10px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-left: 2px solid rgba(34, 211, 238, 0.55);
    border-radius: 4px;
}
.recap-history-widget .rh-headline {
    color: var(--text-bright);
    font-size: 12px;
    line-height: 1.4;
    margin-bottom: 4px;
    word-break: break-word;
}
.recap-history-widget .rh-bullet {
    color: var(--text-mid);
    font-size: 11px;
    line-height: 1.4;
    padding-left: 10px;
    position: relative;
    word-break: break-word;
}
.recap-history-widget .rh-bullet::before {
    content: '·';
    position: absolute;
    left: 0;
    color: var(--accent-cyan);
    opacity: 0.55;
}
.recap-history-widget .rh-delta {
    margin-top: 4px;
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    font-size: 10px;
    color: var(--text-dim);
}
.recap-history-widget .rh-delta .rd-add { color: var(--accent-green); }
.recap-history-widget .rh-delta .rd-del { color: var(--accent-red); }

/* Slightly tighter recap header on narrow cards. */
@media (max-width: 720px) {
    .session-recap {
        grid-template-columns: auto 1fr;
        grid-template-areas:
            "status headline"
            ".      bullets"
            "meta   meta";
        max-height: 110px;
    }
    .session-recap-meta { justify-self: end; padding-top: 2px; }
}
`;
