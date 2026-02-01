// Gift claim overlay
export const giftCss = `
/* Gift claim overlay */
.gift-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(3, 7, 18, 0.95);
    backdrop-filter: blur(12px);
    z-index: 2000;
    justify-content: center;
    align-items: center;
    padding: 20px;
}
.gift-overlay.visible {
    display: flex;
}
.gift-modal {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 20px;
    padding: 40px;
    max-width: 420px;
    width: 100%;
    text-align: center;
    position: relative;
    overflow: hidden;
    animation: gift-enter 0.4s ease-out;
}
@keyframes gift-enter {
    from {
        opacity: 0;
        transform: scale(0.9) translateY(20px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}
.gift-modal::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, var(--accent-green), var(--accent-cyan), var(--accent-green));
    background-size: 200% 100%;
    animation: gradient-shift 3s ease infinite;
}
.gift-icon {
    font-size: 72px;
    margin-bottom: 20px;
    animation: gift-bounce 1s ease-in-out infinite;
}
@keyframes gift-bounce {
    0%, 100% { transform: translateY(0) rotate(0deg); }
    25% { transform: translateY(-8px) rotate(-5deg); }
    75% { transform: translateY(-8px) rotate(5deg); }
}
.gift-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 28px;
    font-weight: 700;
    color: var(--text-bright);
    margin-bottom: 12px;
}
.gift-subtitle {
    font-size: 15px;
    color: var(--text-mid);
    margin-bottom: 24px;
    line-height: 1.6;
}
.gift-duration {
    display: inline-block;
    background: linear-gradient(135deg, rgba(74, 222, 128, 0.15) 0%, rgba(34, 211, 238, 0.15) 100%);
    border: 1px solid var(--accent-green);
    border-radius: 12px;
    padding: 16px 24px;
    margin-bottom: 24px;
}
.gift-duration-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 6px;
}
.gift-duration-value {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 32px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent-green) 0%, var(--accent-cyan) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}
.gift-code-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 20px;
}
.gift-code-value {
    color: var(--accent-cyan);
    font-weight: 600;
    letter-spacing: 1px;
}
.gift-claim-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    padding: 16px 24px;
    background: linear-gradient(135deg, var(--accent-green) 0%, var(--accent-cyan) 100%);
    border: none;
    border-radius: 12px;
    color: var(--bg-abyss);
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 12px;
}
.gift-claim-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 30px var(--glow-green);
}
.gift-claim-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}
.gift-dismiss {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    padding: 8px;
    transition: color 0.2s;
}
.gift-dismiss:hover {
    color: var(--text-mid);
}
.gift-loading {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    padding: 40px 0;
}
.gift-loading.visible {
    display: flex;
}
.gift-loading-spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--border-dim);
    border-top-color: var(--accent-green);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 20px var(--glow-green);
}
.gift-loading-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-mid);
}
.gift-success {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 0;
}
.gift-success.visible {
    display: flex;
}
.gift-success-icon {
    width: 72px;
    height: 72px;
    background: rgba(74, 222, 128, 0.2);
    border: 2px solid var(--accent-green);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 36px;
    color: var(--accent-green);
    animation: success-pop 0.3s ease-out;
}
.gift-success-text {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-bright);
    text-align: center;
    line-height: 1.5;
}
.gift-error {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
}
.gift-error.visible {
    display: flex;
}
.gift-error-icon {
    font-size: 48px;
}
.gift-error-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--accent-red);
    text-align: center;
}
.gift-retry-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 10px 20px;
    border-radius: 6px;
    color: var(--text-mid);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
}
.gift-retry-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
}

@media (max-width: 768px) {
    .gift-modal {
        padding: 28px 20px;
        border-radius: 16px;
    }
    .gift-title {
        font-size: 24px;
    }
    .gift-duration-value {
        font-size: 26px;
    }
}
`;
