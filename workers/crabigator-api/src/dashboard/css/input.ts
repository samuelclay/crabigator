// Input area, no-sessions message, refresh button
export const inputCss = `
/* Input area */
.input-area {
    padding: 14px;
    border-top: 1px solid var(--border-dim);
    display: flex;
    gap: 10px;
    background: var(--bg-card);
}
.input-area input {
    flex: 1;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 10px 14px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    transition: all 0.2s;
}
.input-area input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.input-area input::placeholder {
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
    transition: all 0.2s;
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
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.refresh-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
}
`;
