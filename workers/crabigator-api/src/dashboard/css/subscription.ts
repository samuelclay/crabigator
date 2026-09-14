// Billing portal link for accounts that still carry a paid subscription
export const subscriptionCss = `
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
`;
