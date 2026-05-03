// Usage display, pro subscriber, paywall overlay
export const paywallCss = `
/* Usage display in settings popover */
.usage-display {
    padding: 16px 0;
}
.usage-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
}
.usage-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-bright);
}
.usage-time {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-mid);
}
.usage-bar-container {
    height: 8px;
    background: var(--bg-abyss);
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--border-dim);
}
.usage-bar {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue));
    border-radius: 3px;
    transition: width 0.3s ease, background 0.3s ease;
}
.usage-bar.warning {
    background: linear-gradient(90deg, var(--accent-orange), var(--accent-yellow));
}
.usage-bar.critical {
    background: linear-gradient(90deg, var(--accent-red), var(--accent-orange));
}
.usage-note {
    margin-top: 10px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-dim);
}
.session-limit-banner {
    margin: 16px 16px 0;
    padding: 22px 24px;
    border: 1px solid rgba(251, 146, 60, 0.42);
    border-radius: 10px;
    background:
        linear-gradient(135deg, rgba(251, 146, 60, 0.13), rgba(34, 211, 238, 0.07)),
        var(--bg-deep);
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.04);
}
.session-limit-kicker {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    font-weight: 700;
    color: var(--accent-orange);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    margin-bottom: 8px;
}
.session-limit-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 24px;
    font-weight: 700;
    color: var(--text-bright);
    margin-bottom: 6px;
}
.session-limit-detail {
    max-width: 780px;
    color: var(--text-mid);
    font-size: 14px;
    line-height: 1.55;
}
.session-limit-upgrade {
    margin-top: 16px;
    font-family: 'JetBrains Mono', monospace;
    padding: 10px 14px;
    border: none;
    border-radius: 6px;
    background: linear-gradient(135deg, var(--accent-orange), var(--accent-yellow));
    color: var(--bg-abyss);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    cursor: pointer;
}
.session-limit-upgrade:hover {
    transform: translateY(-1px);
    box-shadow: 0 10px 24px rgba(251, 146, 60, 0.25);
}
/* Pro subscriber card */
.pro-status-card {
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.08) 0%, rgba(59, 130, 246, 0.08) 100%);
    border: 1px solid rgba(34, 211, 238, 0.3);
    border-radius: 8px;
    padding: 14px 16px;
    position: relative;
    overflow: hidden;
}
.pro-status-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue), var(--accent-purple));
}
.pro-status-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}
.pro-status-icon {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-cyan), var(--accent-blue));
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--bg-abyss);
    font-weight: 700;
}
.pro-status-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.3px;
}
.pro-status-sublabel {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 12px;
}
.manage-subscription-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--accent-cyan);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    transition: all 0.15s ease;
}
.manage-subscription-link:hover {
    color: var(--text-primary);
    text-decoration: underline;
}
.manage-subscription-link:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}
.manage-subscription-link svg {
    width: 12px;
    height: 12px;
}
/* Legacy badge for non-card contexts */
.usage-pro-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    font-weight: 700;
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.15);
    border: 1px solid var(--accent-cyan);
    padding: 4px 10px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.upgrade-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    margin-top: 12px;
    padding: 10px 16px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    border: none;
    border-radius: 6px;
    color: var(--bg-abyss);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: all 0.2s;
}
.upgrade-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 6px 20px var(--glow-cyan);
}
.upgrade-btn.hidden {
    display: none;
}

/* Paywall overlay */
.paywall-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(3, 7, 18, 0.95);
    backdrop-filter: blur(12px);
    z-index: 2000;
    justify-content: center;
    align-items: flex-start;
    padding: 20px;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
}
.paywall-overlay.visible {
    display: flex;
}
.paywall-modal {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 20px;
    padding: 40px;
    max-width: 420px;
    width: 100%;
    text-align: center;
    position: relative;
    overflow: hidden;
    animation: paywall-enter 0.3s ease-out;
    margin: auto;
}
@keyframes paywall-enter {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}
.paywall-modal::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-magenta), var(--accent-cyan));
    background-size: 200% 100%;
    animation: gradient-shift 3s ease infinite;
}
.paywall-icon {
    width: 80px;
    height: 80px;
    margin: 0 auto 24px;
    font-size: 64px;
    line-height: 1;
}
.paywall-title {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 26px;
    font-weight: 700;
    color: var(--text-bright);
    margin-bottom: 12px;
}
.paywall-subtitle {
    font-size: 14px;
    color: var(--text-mid);
    margin-bottom: 24px;
    line-height: 1.6;
}
.paywall-usage {
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 24px;
}
.paywall-usage-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    color: var(--text-bright);
}
.paywall-usage-text .used {
    color: var(--accent-red);
}
.paywall-price {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 42px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    margin-bottom: 8px;
}
.paywall-price-period {
    font-size: 16px;
    color: var(--text-dim);
    margin-bottom: 24px;
}
.paywall-features {
    text-align: left;
    margin-bottom: 28px;
}
.paywall-feature {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    font-size: 13px;
    color: var(--text-mid);
}
.paywall-feature-icon {
    color: var(--accent-green);
    font-size: 14px;
}
.paywall-buttons {
    display: flex;
    flex-direction: column;
    gap: 12px;
}
.paywall-btn {
    font-family: 'JetBrains Mono', monospace;
    width: 100%;
    padding: 14px 24px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
}
.paywall-btn.stripe {
    background: linear-gradient(135deg, #635bff 0%, #5244e7 100%);
    border: none;
    color: white;
}
.paywall-btn.stripe:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(99, 91, 255, 0.4);
}
.paywall-btn.paypal {
    background: #ffc439;
    border: none;
    color: #003087;
}
.paywall-btn.paypal:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(255, 196, 57, 0.4);
}
.paywall-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none !important;
    box-shadow: none !important;
}
.paywall-dismiss {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 11px;
    cursor: pointer;
    margin-top: 16px;
    padding: 8px;
    transition: color 0.2s;
}
.paywall-dismiss:hover {
    color: var(--text-mid);
}
.paywall-loading {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 0;
}
.paywall-loading.visible {
    display: flex;
}
.paywall-loading-spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border-dim);
    border-top-color: var(--accent-cyan);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}
.paywall-loading-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-mid);
}
.paywall-success {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 0;
}
.paywall-success.visible {
    display: flex;
}
.paywall-success-icon {
    width: 64px;
    height: 64px;
    background: rgba(74, 222, 128, 0.2);
    border: 2px solid var(--accent-green);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 32px;
    color: var(--accent-green);
    animation: success-pop 0.3s ease-out;
}
@keyframes success-pop {
    0% { transform: scale(0); }
    70% { transform: scale(1.1); }
    100% { transform: scale(1); }
}
.paywall-success-text {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 20px;
    font-weight: 600;
    color: var(--text-bright);
}
.paywall-error {
    display: none;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 20px 0;
}
.paywall-error.visible {
    display: flex;
}
.paywall-error-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-red);
    text-align: center;
}
.paywall-retry-btn {
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
.paywall-retry-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
}

@media (max-width: 768px) {
    .session-limit-banner {
        margin: 12px 12px 0;
        padding: 18px 16px;
    }
    .session-limit-title {
        font-size: 20px;
    }
    .session-limit-detail {
        font-size: 13px;
    }
    .paywall-modal {
        padding: 28px 20px;
        border-radius: 16px;
    }
    .paywall-title {
        font-size: 22px;
    }
    .paywall-price {
        font-size: 36px;
    }
}
`;
