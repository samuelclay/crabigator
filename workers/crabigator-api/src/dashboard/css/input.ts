// Input area, no-sessions message, refresh button
export const inputCss = `
/* Input area */
.input-area {
    padding: 14px;
    border-top: 1px solid var(--border-dim);
    display: flex;
    align-items: flex-end;
    gap: 10px;
    background: var(--bg-card);
    border-radius: 0 0 11px 11px;
}
.input-area textarea {
    flex: 1;
    min-width: 0;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 9px 14px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    line-height: 1.35;
    max-height: calc(1.35em * 6 + 20px);
    overflow-y: hidden;
    resize: none;
    transition: border-color 0.2s, box-shadow 0.2s;
}
.input-area textarea:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.input-area textarea::placeholder {
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
    transition: transform 0.2s, box-shadow 0.2s;
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

/* Keyboard shortcut button - mirrors voice-btn style */
.keyboard-container {
    position: relative;
}
.input-area .keyboard-btn {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 50%;
    width: 38px;
    height: 38px;
    min-width: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: border-color 0.2s, color 0.2s, box-shadow 0.2s;
    padding: 0;
    color: var(--text-mid);
    font-size: 0;
    letter-spacing: 0;
    text-transform: none;
    font-weight: normal;
}
.input-area .keyboard-btn svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
}
.input-area .keyboard-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    box-shadow: 0 0 12px var(--glow-cyan);
    transform: none;
}

/* Keyboard shortcuts popover */
.keyboard-popover {
    display: none;
    position: absolute;
    bottom: calc(100% + 12px);
    left: 0;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 10px;
    z-index: 200;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5), 0 0 20px var(--glow-cyan);
    min-width: 220px;
    touch-action: manipulation;
}
.keyboard-popover.visible {
    display: flex;
    flex-direction: column;
    gap: 6px;
}
/* Arrow pointing down toward button (default: popover above) */
.keyboard-popover::after {
    content: '';
    position: absolute;
    bottom: -7px;
    left: 16px;
    width: 12px;
    height: 12px;
    background: var(--bg-card);
    border-right: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    transform: rotate(45deg);
}
/* Drop-down: popover below button */
.keyboard-popover.drop-down {
    bottom: auto;
    top: calc(100% + 12px);
}
/* Arrow pointing up toward button when dropping down */
.keyboard-popover.drop-down::after {
    bottom: auto;
    top: -7px;
    transform: rotate(-135deg);
}
.keyboard-popover.align-right {
    left: auto;
    right: 0;
}
.keyboard-popover.align-right::after {
    left: auto;
    right: 16px;
}
.keyboard-popover-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 4px 4px;
}
.keyboard-popover .key-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 8px 12px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s, box-shadow 0.15s;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-mid);
    text-transform: none;
    letter-spacing: 0;
    font-weight: normal;
    touch-action: manipulation;
    text-align: left;
}
.keyboard-popover .key-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--text-bright);
    transform: none;
    box-shadow: none;
}
.keyboard-popover .key-btn:active {
    background: var(--bg-abyss);
    box-shadow: 0 0 12px var(--glow-cyan);
}
.keyboard-popover .key-btn kbd {
    background: var(--bg-abyss);
    color: var(--accent-cyan);
    border-radius: 4px;
    padding: 2px 6px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    min-width: 50px;
    text-align: center;
    flex-shrink: 0;
}

@media (max-width: 768px) {
    .input-area {
        padding: 10px;
        gap: 6px;
    }
    .input-area .voice-btn,
    .input-area .keyboard-btn {
        width: 32px;
        height: 32px;
        min-width: 32px;
    }
    .input-area .voice-btn svg,
    .input-area .keyboard-btn svg {
        width: 16px;
        height: 16px;
    }
    .input-area textarea {
        padding: 7px 10px;
        font-size: 16px;  /* Prevent iOS focus zoom */
        max-height: calc(1.35em * 6 + 16px);
    }
    .input-area > button:last-child {
        padding: 8px 12px;
        font-size: 11px;
    }
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
    transition: border-color 0.2s, color 0.2s, background 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.refresh-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
`;
