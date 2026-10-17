// Pairing gate, install card
export const pairingCss = `
/* Keep setup centered without an empty session sidebar. */
body:has(.pairing-gate) .sidebar,
body:has(.pairing-gate) .header > :not(h1) { display: none; }
body:has(.pairing-gate) .dashboard-layout { padding-inline: 0; }

/* Pairing gate styles */
.pairing-gate {
    width: 100%;
    max-width: 480px;
    margin: 0 auto;
    padding: 32px 16px;
}
.pairing-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 10px;
    padding: 32px;
    text-align: center;
    position: relative;
    overflow: hidden;
}
.pairing-icon {
    width: 72px;
    height: 72px;
    margin: 0 auto 28px;
    color: var(--accent-cyan);

}
.pairing-icon svg {
    width: 100%;
    height: 100%;
}
.pairing-icon.spinning svg {
    animation: spin 1s linear infinite;
}
.pairing-card h2 {
    font-family: 'DM Sans', sans-serif;
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
    font-family: var(--font-ui);
    font-size: 20px;
    text-align: center;
    letter-spacing: 3px;
    text-transform: uppercase;
    transition: all 0.2s;
}
.pairing-form input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: none;
}
.pairing-form input::placeholder {
    color: var(--text-dim);
    letter-spacing: 3px;
    font-size: 14px;
}
.pairing-form button {
    font-family: var(--font-ui);
    flex-shrink: 0;
    padding: 14px 28px;
    background: var(--accent-peach);
    border: none;
    border-radius: 10px;
    color: var(--bg-abyss);
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
    text-transform: none;
    letter-spacing: 0;
}
@media (hover: hover) and (pointer: fine) {
.pairing-form button:hover {
    transform: none;
    box-shadow: none;
}
}
.pairing-form button:disabled {
    background: var(--bg-surface);
    color: var(--text-dim);
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}
.pairing-error {
    font-family: var(--font-ui);
    color: var(--accent-red);
    font-size: 12px;
    min-height: 20px;
    margin-bottom: 16px;
    text-transform: none;
    letter-spacing: 0;
}
.pairing-help {
    color: var(--text-dim);
    font-size: 12px;
}
.pairing-help code {
    font-family: var(--font-ui);
    background: var(--bg-surface);
    padding: 3px 8px;
    border-radius: 4px;
    color: var(--accent-cyan);
}
.social-login-stack {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 22px;
}
.social-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    padding: 12px 16px;
    border-radius: 10px;
    border: 1px solid var(--border-dim);
    background: var(--bg-abyss);
    color: var(--text-bright);
    font-family: 'DM Sans', sans-serif;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    text-decoration: none;
    transition: all 0.2s;
}
@media (hover: hover) and (pointer: fine) {
.social-btn:hover {
    border-color: var(--accent-cyan);
    box-shadow: none;
}
}
.pairing-divider {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-dim);
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: none;
    margin: 8px 0 22px;
}
.pairing-divider::before,
.pairing-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border-dim);
}
.account-identities {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 8px 0 12px;
    color: var(--text-mid);
    font-size: 12px;
}
.account-connect-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}
.account-connect-row .social-btn {
    width: auto;
    padding: 8px 12px;
    font-size: 12px;
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
    font-family: var(--font-ui);
    font-size: 14px;
    color: var(--accent-cyan);
    letter-spacing: 0;
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
@media (hover: hover) and (pointer: fine) {
.install-copy-btn:hover {
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
}
}
.install-hint {
    color: var(--text-dim);
    font-size: 12px;
    margin-bottom: 16px;
}
.install-hint code {
    font-family: var(--font-ui);
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
@media (hover: hover) and (pointer: fine) {
.install-link:hover {
    color: var(--accent-cyan);
    text-decoration: underline;
}
}
`;
