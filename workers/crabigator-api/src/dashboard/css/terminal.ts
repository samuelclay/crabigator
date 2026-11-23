// Terminal, scroll mode, ANSI classes
export const terminalCss = `
/* Keep the navigation fixed to its own viewport, including in a multi-session grid. */
.terminal-frame { position:relative; min-width:0; }
.shared-dashboard .terminal-frame, .session-fullscreen .terminal-frame { display:flex; flex-direction:column; flex:1; min-height:0; }
.prompt-jump {
    position:absolute; right:10px; z-index:1; display:grid; grid-template-columns:auto auto;
    justify-content:start; column-gap:8px; max-width:min(48%, 18rem); padding:5px 12px;
    border:1px solid var(--border-dim,#374654); border-radius:14px;
    background:#19232df5; color:var(--text-bright,#edf3f7); cursor:pointer; text-align:left;
    font:500 12px/1.3 var(--font-ui,system-ui); box-shadow:0 2px 7px #0003;
}
.prompt-jump[hidden] { display:none; }
.prompt-jump.previous { top:6px; } .prompt-jump.next { bottom:6px; }
.prompt-jump:hover, .prompt-jump:focus-visible { border-color:var(--accent-cyan,#79c9df); }
.prompt-count { color:var(--accent-cyan,#79c9df); font-variant-numeric:tabular-nums; }
.prompt-excerpt { grid-column:1/-1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; color:#b8c6d1; }
@media(pointer:coarse) { .prompt-jump { min-height:44px; align-content:center; } }
/* Terminal */
.terminal {
    background: #0b1218;
    padding: 12px;
    overflow-y: hidden;
    overflow-x: hidden;
    /* Scroll restoration is handled once for each complete terminal update. */
    overflow-anchor: none;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    line-height: 1.3;
    border: 1px solid transparent;
    transition: height 0.25s ease-out, box-shadow 0.2s ease, border-color 0.2s ease;
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
/* Paragraphs keep two cells of indentation on every client-wrapped line. */
.terminal:not(.scroll-mode) .terminal-paragraph {
    padding-left: 2ch;
}
.terminal:not(.scroll-mode) .terminal-paragraph > .reflow-row {
    display: inline;
    white-space: normal;
}
.terminal:not(.scroll-mode) .terminal-paragraph .reflow-indent {
    display: none;
}
.reflow-gap { display: none; }
.terminal:not(.scroll-mode) .terminal-paragraph .reflow-gap { display: inline; }
/* Fallback symbol fonts have proportional advances, even inside monospace text. */
.terminal-symbol {
    display: inline-block;
    width: 1ch;
    text-align: center;
}
/* Scroll active - click to enable scrolling */
.terminal.scroll-active {
    overflow-y: auto;
    border-color: var(--accent-cyan);
    box-shadow: none;
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
.terminal .terminal-link {
    color: inherit;
    text-decoration: underline;
    text-decoration-color: var(--accent-cyan);
    text-underline-offset: 2px;
    cursor: pointer;
}
@media (hover: hover) and (pointer: fine) {
.terminal .terminal-link:hover {
    color: var(--accent-cyan);
}
}
`;
