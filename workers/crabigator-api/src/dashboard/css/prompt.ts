// Prompt panel, question tabs, inline inputs
export const promptCss = `
/* Prompt panel (questions, permissions) */
.prompt-panel {
    display: none;
    padding: 18px;
    background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-deep) 100%);
    border-bottom: 1px solid var(--border-dim);
}
.prompt-panel.visible { display: block; }
.prompt-header {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-orange);
    font-size: 11px;
    font-weight: 600;
    margin-bottom: 10px;
    text-transform: uppercase;
    letter-spacing: 1px;
    display: flex;
    align-items: center;
    gap: 8px;
}
.prompt-header::before {
    content: '';
    width: 6px;
    height: 6px;
    background: var(--accent-orange);
    border-radius: 50%;
    box-shadow: 0 0 8px rgba(251, 146, 60, 0.5);
    animation: pulse-dot 2s ease-in-out infinite;
}
@keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
}
/* Question tabs for multi-question prompts */
.question-tab {
    color: var(--text-dim);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    background: transparent;
}
.question-tab.current {
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.15);
}
.prompt-question {
    color: var(--text-bright);
    font-size: 14px;
    margin-bottom: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
}
.prompt-options {
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.prompt-option {
    font-family: 'JetBrains Mono', monospace;
    padding: 12px 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
}
.prompt-option:hover {
    background: var(--bg-card);
    border-color: var(--accent-cyan);
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-option.selected {
    border-color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.prompt-option-number {
    color: var(--text-dim);
    font-size: 11px;
    margin-right: 10px;
}
.prompt-option-label {
    color: var(--text-bright);
    font-weight: 500;
    font-size: 12px;
}
.prompt-option-desc {
    color: var(--text-mid);
    font-size: 11px;
    margin-top: 6px;
    padding-left: 24px;
}
.prompt-other {
    margin-top: 14px;
    display: flex;
    gap: 10px;
}
.prompt-other input {
    flex: 1;
    padding: 10px 14px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    transition: all 0.2s;
}
.prompt-other input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-other button {
    font-family: 'JetBrains Mono', monospace;
    padding: 10px 18px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 8px;
    color: var(--bg-abyss);
    font-weight: 700;
    font-size: 11px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
}
.prompt-other button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 20px var(--glow-cyan);
}

/* Inline tab instruction inputs */
.prompt-option-row {
    display: flex;
    align-items: center;
    gap: 10px;
}
.prompt-option-row .prompt-option {
    flex: 1;
    min-width: 0;
}
.prompt-tab-wrapper {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 0 0 auto;
}
.prompt-tab-input {
    font-family: 'JetBrains Mono', monospace;
    width: 180px;
    padding: 10px 12px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-size: 11px;
    transition: all 0.2s;
}
.prompt-tab-input:focus {
    border-color: var(--accent-cyan);
    outline: none;
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-tab-input::placeholder {
    color: var(--text-dim);
    font-size: 10px;
}
.prompt-tab-send {
    font-family: 'JetBrains Mono', monospace;
    padding: 8px 12px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 6px;
    color: var(--bg-abyss);
    font-weight: 700;
    font-size: 10px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
    white-space: nowrap;
}
.prompt-tab-send:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px var(--glow-cyan);
}
@media (max-width: 768px) {
    .prompt-option-row {
        flex-direction: column;
        align-items: stretch;
    }
    .prompt-tab-wrapper {
        margin-top: 6px;
    }
    .prompt-tab-input {
        flex: 1;
        width: auto;
    }
}
`;
