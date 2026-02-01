// Pairing gate, install card
export const pairingCss = `
/* Pairing gate styles */
.pairing-gate {
    width: 100%;
    max-width: 420px;
    margin: 0 auto;
    padding: 32px 16px;
}
.pairing-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 16px;
    padding: 40px;
    text-align: center;
    position: relative;
    overflow: hidden;
}
.pairing-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-magenta), var(--accent-cyan));
    background-size: 200% 100%;
    animation: gradient-shift 3s ease infinite;
}
@keyframes gradient-shift {
    0%, 100% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
}
.pairing-icon {
    width: 72px;
    height: 72px;
    margin: 0 auto 28px;
    color: var(--accent-cyan);
    filter: drop-shadow(0 0 20px var(--glow-cyan));
}
.pairing-icon svg {
    width: 100%;
    height: 100%;
}
.pairing-icon.spinning svg {
    animation: spin 1s linear infinite;
}
.pairing-card h2 {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 26px;
    font-weight: 700;
    margin-bottom: 14px;
    color: var(--text-bright);
}
.pairing-description {
    color: var(--text-mid);
    font-size: 14px;
    line-height: 1.6;
    margin-bottom: 28px;
}
.pairing-form {
    display: flex;
    gap: 12px;
    margin-bottom: 18px;
}
.pairing-form input {
    flex: 1;
    min-width: 0;
    padding: 14px 18px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 10px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 20px;
    text-align: center;
    letter-spacing: 3px;
    text-transform: uppercase;
    transition: all 0.2s;
}
.pairing-form input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 25px var(--glow-cyan);
}
.pairing-form input::placeholder {
    color: var(--text-dim);
    letter-spacing: 3px;
    font-size: 14px;
}
.pairing-form button {
    font-family: 'JetBrains Mono', monospace;
    flex-shrink: 0;
    padding: 14px 28px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 10px;
    color: var(--bg-abyss);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.pairing-form button:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 30px var(--glow-cyan);
}
.pairing-form button:disabled {
    background: var(--bg-surface);
    color: var(--text-dim);
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}
.pairing-error {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-red);
    font-size: 12px;
    min-height: 20px;
    margin-bottom: 16px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.pairing-help {
    color: var(--text-dim);
    font-size: 12px;
}
.pairing-help code {
    font-family: 'JetBrains Mono', monospace;
    background: var(--bg-surface);
    padding: 3px 8px;
    border-radius: 4px;
    color: var(--accent-cyan);
}

/* Install card (shown on pairing page) */
.install-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 24px;
    margin-top: 24px;
    text-align: center;
}
.install-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin-bottom: 16px;
    color: var(--text-mid);
    font-size: 13px;
}
.install-icon {
    width: 18px;
    height: 18px;
    min-width: 18px;
    min-height: 18px;
    max-width: 18px;
    max-height: 18px;
    color: var(--accent-purple);
    flex-shrink: 0;
}
.install-icon svg {
    width: 100%;
    height: 100%;
}
.install-command {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 12px 16px;
    margin-bottom: 12px;
}
.install-command code {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    color: var(--accent-cyan);
    letter-spacing: 0.5px;
}
.install-copy-btn {
    background: transparent;
    border: none;
    padding: 6px;
    cursor: pointer;
    color: var(--text-dim);
    border-radius: 4px;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
}
.install-copy-btn:hover {
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
}
.install-hint {
    color: var(--text-dim);
    font-size: 12px;
    margin-bottom: 16px;
}
.install-hint code {
    font-family: 'JetBrains Mono', monospace;
    background: var(--bg-card);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text-mid);
}
.install-link {
    display: inline-block;
    color: var(--accent-magenta);
    font-size: 12px;
    text-decoration: none;
    transition: all 0.2s;
}
.install-link:hover {
    color: var(--accent-cyan);
    text-decoration: underline;
}
`;
