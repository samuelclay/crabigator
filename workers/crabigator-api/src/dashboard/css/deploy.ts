// Deploy overlay styles
export const deployCss = `
/* Deploy overlay */
.deploy-overlay {
    display: none;
    position: fixed;
    inset: 0;
    background: rgba(3, 7, 18, 0.95);
    backdrop-filter: blur(8px);
    z-index: 1000;
    justify-content: center;
    align-items: center;
    flex-direction: column;
    gap: 24px;
}
.deploy-overlay.visible { display: flex; }
.deploy-spinner {
    width: 48px;
    height: 48px;
    border: 3px solid var(--border-dim);
    border-top-color: var(--accent-cyan);
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 20px var(--glow-cyan);
}
@keyframes spin { to { transform: rotate(360deg); } }
.deploy-text {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 18px;
    color: var(--text-bright);
    text-align: center;
}
.deploy-subtext {
    font-size: 13px;
    color: var(--text-mid);
    text-align: center;
}
.deploy-countdown {
    font-size: 12px;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
}

body {
    font-family: 'Space Grotesk', -apple-system, BlinkMacSystemFont, sans-serif;
    background: var(--bg-abyss);
    color: var(--text-bright);
    min-height: 100vh;
    overflow-x: hidden;
    width: 100%;
    max-width: 100%;
    position: relative;
    -webkit-font-smoothing: antialiased;
}
`;
