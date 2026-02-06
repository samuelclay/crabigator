// Terminal, scroll mode, ANSI classes
export const terminalCss = `
/* Terminal */
.terminal {
    background: var(--bg-abyss);
    padding: 12px;
    overflow-y: hidden;
    overflow-x: hidden;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    line-height: 1.3;
    transition: height 0.25s ease-out, box-shadow 0.2s ease;
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
/* Scroll active - click to enable scrolling */
.terminal.scroll-active {
    overflow-y: auto;
    box-shadow: inset 0 0 20px var(--glow-cyan);
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
`;
