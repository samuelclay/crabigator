// Landing page CSS styles - Creative "Terminal Brutalism meets Ocean Depths" design
export const landingCss = `
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700;800&family=Space+Grotesk:wght@400;500;600;700&display=swap');

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
    --bg-abyss: #030712;
    --bg-deep: #0a0f1a;
    --bg-surface: #111827;
    --bg-card: #1a2332;
    --border-dim: #1e293b;
    --border-glow: #0ea5e9;
    --text-bright: #f1f5f9;
    --text-mid: #94a3b8;
    --text-dim: #64748b;
    --accent-cyan: #22d3ee;
    --accent-blue: #3b82f6;
    --accent-magenta: #e879f9;
    --accent-green: #4ade80;
    --accent-orange: #fb923c;
    --accent-red: #f87171;
    --glow-cyan: rgba(34, 211, 238, 0.4);
    --glow-magenta: rgba(232, 121, 249, 0.3);
}

html { scroll-behavior: smooth; }

body {
    font-family: 'Space Grotesk', sans-serif;
    background: var(--bg-abyss);
    color: var(--text-bright);
    line-height: 1.6;
    -webkit-font-smoothing: antialiased;
    overflow-x: hidden;
    max-width: 100vw;
}

html {
    overflow-x: hidden;
}

/* Scanline overlay */
body::before {
    content: '';
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: repeating-linear-gradient(
        0deg,
        transparent,
        transparent 2px,
        rgba(0, 0, 0, 0.03) 2px,
        rgba(0, 0, 0, 0.03) 4px
    );
    pointer-events: none;
    z-index: 9999;
}

/* Custom scrollbar */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: var(--bg-abyss); }
::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    border-radius: 3px;
}

a { color: var(--accent-cyan); text-decoration: none; transition: all 0.2s; }
a:hover { color: var(--accent-magenta); text-decoration: none; }

code, .mono {
    font-family: 'JetBrains Mono', monospace;
}

/* SVG Icons */
.icon {
    display: inline-block;
    width: 1em;
    height: 1em;
    vertical-align: -0.125em;
    fill: currentColor;
}
.icon-sm { width: 0.875em; height: 0.875em; }
.icon-lg { width: 1.25em; height: 1.25em; }
.icon-widget { width: 14px; height: 14px; vertical-align: -2px; }

/* === NAVIGATION === */
.nav {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    background: rgba(3, 7, 18, 0.8);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border-dim);
    padding: 16px 32px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    z-index: 1000;
}
.nav-logo {
    font-family: 'JetBrains Mono', monospace;
    font-size: 20px;
    font-weight: 700;
    color: var(--text-bright);
    display: flex;
    align-items: center;
    gap: 10px;
    letter-spacing: -0.5px;
}
.nav-logo::before {
    content: '>';
    color: var(--accent-cyan);
    animation: blink 1s step-end infinite;
}
.nav-logo svg {
    width: 24px;
    height: 24px;
    fill: var(--accent-orange);
}
@keyframes blink {
    50% { opacity: 0; }
}
.nav-logo:hover { text-decoration: none; }
.nav-links {
    display: flex;
    align-items: center;
    gap: 8px;
}
.nav-link {
    font-family: 'JetBrains Mono', monospace;
    color: var(--text-dim);
    font-size: 13px;
    padding: 8px 16px;
    border: 1px solid transparent;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.nav-link:hover {
    color: var(--accent-cyan);
    border-color: var(--border-dim);
    background: rgba(34, 211, 238, 0.05);
}
.nav-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--accent-cyan);
    color: var(--accent-cyan);
    padding: 10px 20px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 1px;
    position: relative;
    overflow: hidden;
}
.nav-btn::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(34, 211, 238, 0.2), transparent);
    transition: left 0.5s;
}
.nav-btn:hover::before {
    left: 100%;
}
.nav-btn:hover {
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.nav-github {
    display: flex;
    color: var(--text-dim);
    padding: 8px;
    transition: all 0.2s;
}
.nav-github:hover {
    color: var(--text-bright);
}
.nav-github svg { width: 20px; height: 20px; }

/* === HERO SECTION === */
.hero {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 120px 32px 80px;
    position: relative;
    overflow: hidden;
    max-width: 100%;
}

/* Animated sonar rings background */
.hero::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 800px;
    height: 800px;
    transform: translate(-50%, -50%);
    background:
        radial-gradient(circle, transparent 30%, rgba(34, 211, 238, 0.03) 31%, transparent 32%),
        radial-gradient(circle, transparent 50%, rgba(34, 211, 238, 0.02) 51%, transparent 52%),
        radial-gradient(circle, transparent 70%, rgba(232, 121, 249, 0.02) 71%, transparent 72%);
    animation: sonar 4s ease-out infinite;
    pointer-events: none;
}
@keyframes sonar {
    0% { transform: translate(-50%, -50%) scale(0.5); opacity: 1; }
    100% { transform: translate(-50%, -50%) scale(1.5); opacity: 0; }
}

.hero-content {
    max-width: 1400px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 80px;
    align-items: center;
    position: relative;
    z-index: 1;
}

.hero-text {
    max-width: 600px;
}

.hero-tagline {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-cyan);
    text-transform: uppercase;
    letter-spacing: 3px;
    margin-bottom: 24px;
    display: flex;
    align-items: center;
    gap: 12px;
}
.hero-tagline svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
}
.hero-tagline::before {
    content: '';
    width: 40px;
    height: 1px;
    background: linear-gradient(90deg, var(--accent-cyan), transparent);
}

.hero-headline {
    font-size: clamp(48px, 6vw, 72px);
    font-weight: 700;
    line-height: 1.05;
    margin-bottom: 24px;
    letter-spacing: -2px;
}
.hero-headline .highlight {
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.hero-subheadline {
    font-size: 18px;
    color: var(--text-mid);
    line-height: 1.7;
    margin-bottom: 40px;
    max-width: 500px;
}

.hero-ctas {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 32px;
}

.btn-primary {
    font-family: 'JetBrains Mono', monospace;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    color: #0a0f1a;
    padding: 16px 32px;
    font-size: 14px;
    font-weight: 700;
    border: none;
    cursor: pointer;
    transition: all 0.3s;
    text-transform: uppercase;
    letter-spacing: 1px;
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 10px;
}
.btn-primary::after {
    content: '→';
    transition: transform 0.3s;
}
.btn-primary:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 40px var(--glow-cyan);
}
.btn-primary:hover::after {
    transform: translateX(4px);
}

.btn-primary.outline {
    background: transparent;
    color: var(--text-mid);
    border: 1px solid var(--border-dim);
}
.btn-primary.outline:hover {
    color: var(--accent-cyan);
    border-color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.05);
    box-shadow: 0 10px 40px var(--glow-cyan);
}

.btn-secondary {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    color: var(--text-mid);
    padding: 16px 32px;
    font-size: 14px;
    font-weight: 500;
    border: 1px solid var(--border-dim);
    cursor: pointer;
    transition: all 0.3s;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.btn-secondary:hover {
    color: var(--accent-magenta);
    border-color: var(--accent-magenta);
    background: rgba(232, 121, 249, 0.05);
}

.hero-price {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
}
.hero-price .price {
    color: var(--accent-green);
    font-weight: 700;
    font-size: 16px;
}

/* Hero Email Card */
.hero-email-card {
    margin-top: 40px;
    max-width: 420px;
    padding: 20px 24px;
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.06) 0%, rgba(139, 92, 246, 0.04) 100%);
    border: 1px solid rgba(6, 182, 212, 0.2);
    border-radius: 12px;
    position: relative;
}
.hero-email-card::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 12px;
    padding: 1px;
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.3) 0%, rgba(139, 92, 246, 0.1) 100%);
    -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    pointer-events: none;
}
.hero-email-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
}
.hero-email-icon {
    width: 18px;
    height: 18px;
    color: var(--accent-cyan);
    opacity: 0.9;
}
.hero-email-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-mid);
    text-transform: uppercase;
    letter-spacing: 1px;
}
.hero-email-form {
    display: flex;
    gap: 10px;
}
.hero-email-input {
    flex: 1;
    padding: 12px 16px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    transition: all 0.2s;
}
.hero-email-input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.hero-email-input::placeholder {
    color: var(--text-dim);
}
.hero-email-btn {
    font-family: 'JetBrains Mono', monospace;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    color: var(--bg-abyss);
    border: none;
    border-radius: 8px;
    padding: 12px 24px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    white-space: nowrap;
}
.hero-email-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px var(--glow-cyan);
}
.hero-email-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
}
.hero-email-success {
    display: none;
    text-align: center;
    padding: 14px 16px;
    color: var(--accent-green);
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    font-weight: 500;
}
.hero-email-success.visible {
    display: block;
}

/* Hero Terminal */
.hero-terminal {
    position: relative;
}
.terminal-window {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    overflow: hidden;
    box-shadow:
        0 0 0 1px rgba(34, 211, 238, 0.1),
        0 20px 60px rgba(0, 0, 0, 0.5),
        0 0 100px rgba(34, 211, 238, 0.1);
    transform: perspective(1000px) rotateY(-5deg) rotateX(2deg);
    transition: transform 0.5s;
}
.terminal-window:hover {
    transform: perspective(1000px) rotateY(0deg) rotateX(0deg);
}
.terminal-bar {
    background: var(--bg-surface);
    padding: 12px 16px;
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid var(--border-dim);
}
.terminal-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
}
.terminal-dot.red { background: #ff5f57; }
.terminal-dot.yellow { background: #febc2e; }
.terminal-dot.green { background: #28c840; }
.terminal-title {
    flex: 1;
    text-align: center;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-dim);
}
.terminal-content {
    padding: 24px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    line-height: 1.8;
    min-height: 380px;
}
.terminal-line {
    margin-bottom: 4px;
    opacity: 0;
    animation: typeIn 0.3s ease forwards;
}
.terminal-line:nth-child(1) { animation-delay: 0.2s; }
.terminal-line:nth-child(2) { animation-delay: 0.4s; }
.terminal-line:nth-child(3) { animation-delay: 0.6s; }
.terminal-line:nth-child(4) { animation-delay: 0.8s; }
.terminal-line:nth-child(5) { animation-delay: 1s; }
.terminal-line:nth-child(6) { animation-delay: 1.2s; }
@keyframes typeIn {
    from { opacity: 0; transform: translateX(-10px); }
    to { opacity: 1; transform: translateX(0); }
}
.terminal-prompt { color: var(--accent-green); }
.terminal-cmd { color: var(--text-bright); }
.terminal-output { color: var(--text-mid); }
.terminal-success { color: var(--accent-cyan); }
.terminal-thinking {
    display: inline-flex;
    gap: 4px;
}
.terminal-thinking span {
    width: 4px;
    height: 4px;
    background: var(--accent-cyan);
    border-radius: 50%;
    animation: pulse 1.4s ease-in-out infinite;
}
.terminal-thinking span:nth-child(2) { animation-delay: 0.2s; }
.terminal-thinking span:nth-child(3) { animation-delay: 0.4s; }
@keyframes pulse {
    0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
    40% { opacity: 1; transform: scale(1.2); }
}

/* Widget cards in terminal */
.terminal-widgets {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin-top: 20px;
}
.widget-card {
    background: rgba(26, 35, 50, 0.8);
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    padding: 14px;
    transition: all 0.3s;
}
.widget-card:hover {
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px rgba(34, 211, 238, 0.1);
}
.widget-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--accent-cyan);
}
.widget-row {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-dim);
    padding: 4px 0;
}
.widget-value { color: var(--text-bright); font-weight: 500; }
.widget-value.green { color: var(--accent-green); }
.widget-value.red { color: var(--accent-red); }

/* === FEATURES BENTO GRID === */
.section {
    padding: 120px 32px;
    position: relative;
    overflow: hidden;
    max-width: 100%;
}

.features-section {
    padding: 120px 24px;
}

.section-header {
    max-width: 800px;
    margin: 0 auto 64px;
    text-align: center;
}

.section-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-magenta);
    text-transform: uppercase;
    letter-spacing: 3px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
}
.section-label svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
}

.section-title {
    font-size: clamp(32px, 5vw, 48px);
    font-weight: 700;
    letter-spacing: -1px;
    line-height: 1.1;
    margin-bottom: 16px;
}

.section-subtitle {
    font-size: 18px;
    color: var(--text-mid);
    max-width: 600px;
    margin: 0 auto;
}

/* Bento grid */
.bento-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
    max-width: 1200px;
    margin: 0 auto;
}

.bento-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 32px;
    position: relative;
    overflow: hidden;
    transition: all 0.4s;
    display: flex;
    flex-direction: column;
}
.bento-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, transparent, var(--accent-cyan), transparent);
    opacity: 0;
    transition: opacity 0.3s;
}
.bento-card:hover {
    border-color: var(--border-glow);
    transform: translateY(-4px);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3), 0 0 40px var(--glow-cyan);
}
.bento-card:hover::before {
    opacity: 1;
}

/* Bento card sizes */
.bento-card.large { grid-column: span 2; }
.bento-card.wide { grid-column: span 3; }
.bento-card.medium { grid-column: span 2; }
.bento-card.small { grid-column: span 1; }
.bento-card.tall { grid-row: span 2; }
.bento-card.wide.tall { grid-column: span 3; grid-row: span 2; }

.bento-icon {
    width: 48px;
    height: 48px;
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.2) 0%, rgba(232, 121, 249, 0.1) 100%);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 24px;
    margin-bottom: 20px;
    border: 1px solid var(--border-dim);
}
.bento-icon svg {
    width: 24px;
    height: 24px;
    fill: currentColor;
    color: var(--accent-cyan);
}

.bento-title {
    font-size: 20px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--text-bright);
}

.bento-desc {
    font-size: 15px;
    color: var(--text-mid);
    line-height: 1.6;
    flex: 1;
}

.bento-visual {
    margin-top: 24px;
    flex: 1;
    display: flex;
    align-items: flex-end;
}

/* Visual elements for bento cards */
.mini-terminal {
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    padding: 16px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    width: 100%;
}

/* Mini widget styles */
.mini-widget {
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 16px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    width: 100%;
}
.widget-header-mini {
    color: var(--accent-cyan);
    font-size: 11px;
    margin-bottom: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.widget-row-mini {
    display: flex;
    justify-content: space-between;
    color: var(--text-dim);
    padding: 4px 0;
}
.widget-row-mini .val {
    color: var(--text-bright);
}
.widget-row-mini .val-bar {
    width: 60px;
    height: 8px;
    background: var(--bg-deep);
    border-radius: 4px;
    overflow: hidden;
}
.widget-row-mini .bar {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, var(--accent-cyan), var(--accent-blue));
    border-radius: 4px;
}

/* Sparkline for tool calls */
.sparkline {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 14px;
}
.spark {
    width: 4px;
    background: #d4a574;
    border-radius: 1px;
}

/* File diff rows */
.file-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0;
    color: var(--text-dim);
}
.file-name {
    color: var(--text-mid);
}
.diff-bar {
    display: flex;
    gap: 1px;
}
.diff-bar .add {
    height: 10px;
    background: var(--accent-green);
    border-radius: 2px;
}
.diff-bar .del {
    height: 10px;
    background: var(--accent-red);
    border-radius: 2px;
}

/* Semantic diff widget */
.mini-widget.semantic {
    display: flex;
    gap: 20px;
}
.lang-group {
    flex: 1;
}
.lang-header {
    color: var(--text-dim);
    font-size: 10px;
    margin-bottom: 8px;
}
.lang-tag {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 9px;
    font-weight: 600;
    margin-right: 6px;
}
.lang-tag.js {
    background: rgba(250, 204, 21, 0.2);
    color: #facc15;
}
.lang-tag.rs {
    background: rgba(251, 146, 60, 0.2);
    color: #fb923c;
}
.func-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 0;
    color: var(--text-mid);
    font-size: 10px;
}
.func-icon {
    color: var(--accent-cyan);
    font-size: 9px;
}
.func-diff {
    margin-left: auto;
    color: var(--text-dim);
}
.func-diff .green {
    color: var(--accent-green);
}
.func-diff .red {
    color: var(--accent-red);
}

/* File links widget */
.mini-widget.file-links {
    display: flex;
    flex-direction: column;
    gap: 6px;
}
.link-row {
    display: flex;
    align-items: center;
    padding: 6px 10px;
    background: var(--bg-deep);
    border-radius: 4px;
    transition: all 0.2s;
}
.link-row:hover {
    background: rgba(34, 211, 238, 0.1);
}
.link-path {
    color: var(--accent-cyan);
}
.link-line {
    color: var(--text-dim);
}

/* Icon variants */
.bento-icon.anthropic {
    background: linear-gradient(135deg, rgba(204, 120, 97, 0.3) 0%, rgba(204, 120, 97, 0.1) 100%);
    color: #cc7861;
}
.bento-icon.openai {
    background: linear-gradient(135deg, rgba(16, 163, 127, 0.3) 0%, rgba(16, 163, 127, 0.1) 100%);
    color: #10a37f;
}

.status-pills {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
}
.status-pill {
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 11px;
    font-family: 'JetBrains Mono', monospace;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.status-pill.thinking { background: rgba(59, 130, 246, 0.2); color: var(--accent-blue); border: 1px solid var(--accent-blue); }
.status-pill.complete { background: rgba(74, 222, 128, 0.2); color: var(--accent-green); border: 1px solid var(--accent-green); }
.status-pill.permission { background: rgba(251, 146, 60, 0.2); color: var(--accent-orange); border: 1px solid var(--accent-orange); }
.status-pill.question { background: rgba(232, 121, 249, 0.2); color: var(--accent-magenta); border: 1px solid var(--accent-magenta); }

.showcase-status {
    margin-top: 32px;
}
.status-hint {
    margin-top: 12px;
    font-size: 13px;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
}

/* === DEVICE SHOWCASE === */
.showcase {
    background: var(--bg-deep);
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    overflow: hidden;
}

.showcase-content {
    max-width: 1400px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1.2fr;
    gap: 80px;
    align-items: center;
}

.showcase-text {
    max-width: 500px;
}

.showcase-devices {
    position: relative;
    height: 600px;
}

.device-phone {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 280px;
    background: var(--bg-surface);
    border: 3px solid #2a2a2a;
    border-radius: 36px;
    padding: 12px;
    box-shadow: 0 30px 60px rgba(0, 0, 0, 0.4);
    z-index: 2;
}
.device-phone::before {
    content: '';
    position: absolute;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    width: 80px;
    height: 24px;
    background: #1a1a1a;
    border-radius: 20px;
}
.phone-screen {
    background: var(--bg-abyss);
    border-radius: 26px;
    padding: 40px 16px 16px;
    min-height: 480px;
}

.device-browser {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%) translateX(20px);
    width: 500px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 30px 60px rgba(0, 0, 0, 0.4);
}
.browser-bar {
    background: var(--bg-card);
    padding: 10px 14px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--border-dim);
}
.browser-dots {
    display: flex;
    gap: 6px;
}
.browser-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--border-dim);
}
.browser-url {
    flex: 1;
    background: var(--bg-abyss);
    padding: 6px 12px;
    border-radius: 6px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
}
.browser-content {
    padding: 20px;
    min-height: 350px;
    background: var(--bg-abyss);
}

/* Session cards */
.session-card {
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 14px;
    margin-bottom: 12px;
    transition: all 0.3s;
}
.session-card:hover {
    border-color: var(--accent-cyan);
}
.session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}
.session-path {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-cyan);
}
.session-state {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    padding: 3px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
}
.session-state.thinking { background: var(--accent-blue); color: white; }
.session-state.permission { background: var(--accent-orange); color: white; }
.session-state.complete { background: var(--text-dim); color: white; }
.session-preview {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-dim);
    background: var(--bg-abyss);
    padding: 10px;
    border-radius: 4px;
    line-height: 1.5;
}

/* === INTERACTIVE SECTION === */
.interactive {
    background: var(--bg-abyss);
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
}
.interactive-content {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 80px;
    align-items: center;
}
.interactive-visual {
    display: flex;
    justify-content: center;
}
.interactive-text {
    max-width: 500px;
}
.interactive-features {
    list-style: none;
    margin-top: 32px;
    display: flex;
    flex-direction: column;
    gap: 16px;
}
.interactive-features li {
    display: flex;
    align-items: flex-start;
    gap: 12px;
}
.feature-icon {
    color: var(--accent-cyan);
    font-size: 16px;
    flex-shrink: 0;
    margin-top: 2px;
}
.feature-icon svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
}
.feature-text {
    font-size: 15px;
    color: var(--text-mid);
    line-height: 1.5;
}
.feature-text strong {
    color: var(--text-bright);
    font-weight: 600;
}

/* Prompt Mockup */
.prompt-mockup {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 24px;
    width: 100%;
    max-width: 440px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}
.prompt-mockup-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-dim);
}
.prompt-mockup-state {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    padding: 4px 10px;
    background: var(--accent-orange);
    color: white;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
}
.prompt-mockup-path {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-dim);
}
.prompt-mockup-question {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-mid);
    margin-bottom: 20px;
    line-height: 1.5;
}
.prompt-mockup-question code {
    background: var(--bg-abyss);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--accent-cyan);
}
.prompt-mockup-options {
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.prompt-mockup-row {
    display: flex;
    gap: 10px;
    align-items: center;
}
.prompt-mockup-option {
    flex: 1;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    padding: 12px 14px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    cursor: pointer;
    transition: all 0.2s;
}
.prompt-mockup-option:hover {
    border-color: var(--accent-cyan);
    background: var(--bg-card);
}
.prompt-mockup-option.selected {
    border-color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.prompt-mockup-option.no-input {
    flex: 1;
}
.option-num {
    color: var(--text-dim);
    margin-right: 8px;
}
.prompt-mockup-input {
    font-family: 'JetBrains Mono', monospace;
    width: 160px;
    padding: 10px 12px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-size: 11px;
    transition: all 0.2s;
}
.prompt-mockup-input:focus {
    border-color: var(--accent-cyan);
    outline: none;
    box-shadow: 0 0 15px var(--glow-cyan);
}
.prompt-mockup-input::placeholder {
    color: var(--text-dim);
}

/* === PRICING === */
.pricing {
    background: var(--bg-abyss);
    position: relative;
}
.pricing::before {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 600px;
    height: 600px;
    background: radial-gradient(circle, rgba(34, 211, 238, 0.05) 0%, transparent 70%);
    pointer-events: none;
}

.pricing-cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
    max-width: 800px;
    margin: 0 auto;
}

.pricing-card {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 16px;
    padding: 40px;
    position: relative;
    overflow: hidden;
}
.pricing-card.featured {
    border: 2px solid var(--accent-cyan);
    box-shadow: 0 0 60px var(--glow-cyan);
}
.pricing-card.featured::before {
    content: '';
    position: absolute;
    inset: 0;
    background: radial-gradient(ellipse at 30% 0%, rgba(34, 211, 238, 0.15) 0%, transparent 50%),
                radial-gradient(ellipse at 70% 100%, rgba(139, 92, 246, 0.1) 0%, transparent 50%);
    opacity: 0.8;
    animation: pulse-glow 4s ease-in-out infinite alternate;
}
@keyframes pulse-glow {
    0% { opacity: 0.5; }
    100% { opacity: 1; }
}
.pricing-inner {
    position: relative;
    z-index: 1;
}
.pricing-tier {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 2px;
    color: var(--text-dim);
    margin-bottom: 12px;
}
.pricing-card.featured .pricing-tier {
    color: var(--accent-cyan);
}
.pricing-amount {
    font-size: 64px;
    font-weight: 800;
    letter-spacing: -2px;
    margin-bottom: 8px;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--text-bright) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}
.pricing-period {
    font-size: 18px;
    font-weight: 400;
    color: var(--text-mid);
}
.pricing-features {
    list-style: none;
    margin: 32px 0;
}
.pricing-features li {
    padding: 12px 0;
    color: var(--text-mid);
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 15px;
    border-bottom: 1px solid var(--border-dim);
}
.pricing-features li:last-child { border-bottom: none; }
.pricing-features .check {
    color: var(--accent-cyan);
    font-weight: bold;
    font-size: 18px;
    display: flex;
    align-items: center;
}
.pricing-features .check svg {
    width: 20px;
    height: 20px;
    fill: currentColor;
}
.pricing-features .check.dim {
    color: var(--text-dim);
}
.pricing-features .dim {
    color: var(--text-dim);
}
.pricing-cta {
    width: 100%;
    justify-content: center;
}
.pricing-note {
    font-size: 13px;
    color: var(--text-dim);
    margin-top: 20px;
    text-align: center;
}

/* === INSTALL === */
.install {
    background: var(--bg-deep);
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
}

.install-content {
    max-width: 700px;
    margin: 0 auto;
}

.install-terminal {
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    overflow: hidden;
    margin-bottom: 48px;
}
.install-terminal-bar {
    background: var(--bg-surface);
    padding: 14px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-dim);
}
.install-terminal-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 1px;
}
.copy-btn {
    font-family: 'JetBrains Mono', monospace;
    background: transparent;
    border: 1px solid var(--border-dim);
    padding: 8px 16px;
    border-radius: 6px;
    color: var(--text-mid);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
    display: flex;
    align-items: center;
    gap: 8px;
}
.copy-btn:hover {
    border-color: var(--accent-cyan);
    color: var(--accent-cyan);
}
.copy-btn.copied {
    border-color: var(--accent-green);
    color: var(--accent-green);
}
.copy-btn svg { width: 14px; height: 14px; }
.install-terminal-content {
    padding: 24px 28px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 18px;
}
.install-terminal-content .prompt { color: var(--accent-green); }

.install-steps {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 24px;
}
.install-step {
    text-align: center;
    padding: 24px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    transition: all 0.3s;
}
.install-step:hover {
    border-color: var(--accent-magenta);
}
.install-step-num {
    width: 40px;
    height: 40px;
    background: linear-gradient(135deg, var(--accent-magenta) 0%, var(--accent-blue) 100%);
    color: white;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'JetBrains Mono', monospace;
    font-weight: 700;
    font-size: 16px;
    margin: 0 auto 16px;
}
.install-step-text {
    font-size: 14px;
    color: var(--text-mid);
}
.install-step-text code {
    background: var(--bg-abyss);
    padding: 3px 8px;
    border-radius: 4px;
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-cyan);
    font-size: 13px;
}

/* === OPEN SOURCE === */
.open-source {
    background: var(--bg-abyss);
}
.open-source-content {
    max-width: 700px;
    margin: 0 auto;
    text-align: center;
}
.open-source-text {
    font-size: 18px;
    color: var(--text-mid);
    margin-bottom: 32px;
    line-height: 1.7;
}
.open-source-links {
    display: flex;
    justify-content: center;
    gap: 16px;
    flex-wrap: wrap;
}
.open-source-link {
    font-family: 'JetBrains Mono', monospace;
    padding: 12px 24px;
    border-radius: 8px;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 1px;
    transition: all 0.2s;
}
.open-source-link.primary {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    color: var(--text-bright);
}
.open-source-link.primary:hover {
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.open-source-link.secondary {
    color: var(--text-dim);
}
.open-source-link.secondary:hover {
    color: var(--accent-magenta);
}

/* === MOBILE APPS === */
.mobile-apps {
    background: var(--bg-deep);
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
}
.email-form {
    max-width: 500px;
    margin: 0 auto;
    display: flex;
    gap: 12px;
}
.email-input {
    flex: 1;
    padding: 16px 20px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    transition: all 0.2s;
}
.email-input:focus {
    outline: none;
    border-color: var(--accent-magenta);
    box-shadow: 0 0 20px var(--glow-magenta);
}
.email-input::placeholder {
    color: var(--text-dim);
}
.email-btn {
    font-family: 'JetBrains Mono', monospace;
    background: linear-gradient(135deg, var(--accent-magenta) 0%, var(--accent-blue) 100%);
    color: white;
    padding: 16px 28px;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.3s;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.email-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 30px var(--glow-magenta);
}
.email-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
}
.email-privacy {
    font-size: 12px;
    color: var(--text-dim);
    margin-top: 20px;
    text-align: center;
}
.email-success {
    display: none;
    text-align: center;
    padding: 20px;
    background: rgba(74, 222, 128, 0.1);
    border: 1px solid var(--accent-green);
    border-radius: 8px;
    color: var(--accent-green);
    font-family: 'JetBrains Mono', monospace;
}
.email-success.visible { display: block; }

/* === FOOTER === */
.footer {
    padding: 64px 32px 32px;
    border-top: 1px solid var(--border-dim);
}
.footer-content {
    max-width: 1200px;
    margin: 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 32px;
}
.footer-cta {
    display: flex;
    align-items: center;
    gap: 20px;
}
.footer-cta-text {
    font-size: 18px;
    color: var(--text-mid);
}
.footer-links {
    display: flex;
    gap: 32px;
}
.footer-link {
    font-family: 'JetBrains Mono', monospace;
    color: var(--text-dim);
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.footer-link:hover { color: var(--accent-cyan); }
.footer-meta {
    font-size: 13px;
    color: var(--text-dim);
}
.footer-bottom {
    max-width: 1200px;
    margin: 48px auto 0;
    padding-top: 24px;
    border-top: 1px solid var(--border-dim);
    text-align: center;
}
.footer-bottom-text {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-dim);
}
.footer-bottom-text span {
    color: var(--accent-cyan);
}

/* === RESPONSIVE === */
@media (max-width: 1024px) {
    .hero-content {
        grid-template-columns: 1fr;
        gap: 48px;
    }
    .hero-terminal {
        max-width: 600px;
        margin: 0 auto;
    }
    .terminal-window {
        transform: none;
    }
    .bento-grid {
        grid-template-columns: repeat(2, 1fr);
    }
    .bento-card.large { grid-column: span 1; }
    .bento-card.wide { grid-column: span 2; }
    .bento-card.wide.tall { grid-column: span 2; }
    .bento-card.medium { grid-column: span 1; }
    .bento-card.small { grid-column: span 1; }
    .bento-card.tall { grid-row: span 1; }
    .mini-widget.semantic { flex-direction: column; gap: 16px; }
    .showcase-content {
        grid-template-columns: 1fr;
        gap: 48px;
    }
    .showcase-devices {
        height: 500px;
        max-width: 600px;
        margin: 0 auto;
    }
    .interactive-content {
        grid-template-columns: 1fr;
        gap: 48px;
    }
    .interactive-visual {
        order: -1;
    }
    .interactive-text {
        max-width: 600px;
        margin: 0 auto;
        text-align: center;
    }
    .interactive-features {
        text-align: left;
        max-width: 400px;
        margin: 32px auto 0;
    }
}

@media (max-width: 768px) {
    .nav { padding: 12px 16px; }
    .nav-links { gap: 4px; }
    .nav-link { display: none; }
    .nav-btn { padding: 8px 16px; font-size: 11px; }
    .nav-logo { font-size: 16px; }
    .nav-logo svg { width: 20px; height: 20px; }

    .hero {
        padding: 100px 16px 60px;
        min-height: auto;
    }
    .hero-headline { font-size: 32px; letter-spacing: -1px; }
    .hero-subheadline { font-size: 15px; }
    .hero-ctas { flex-direction: column; }
    .hero-price { font-size: 11px; gap: 8px; justify-content: center; }
    .btn-primary, .btn-secondary { width: 100%; justify-content: center; padding: 14px 24px; }
    .terminal-widgets { grid-template-columns: 1fr; }

    .section { padding: 80px 16px; }
    .section-title { font-size: 28px; }

    .bento-grid {
        grid-template-columns: 1fr;
    }
    .bento-card.large,
    .bento-card.wide,
    .bento-card.medium,
    .bento-card.small { grid-column: span 1; }
    .bento-card { padding: 24px; }
    .mini-widget.semantic { flex-direction: column; gap: 12px; }

    .showcase-devices {
        height: auto;
        display: flex;
        flex-direction: column;
        gap: 24px;
    }
    .device-phone, .device-browser {
        position: static;
        transform: none;
        width: 100%;
        max-width: 320px;
        margin: 0 auto;
    }
    .device-browser { max-width: 100%; }

    .pricing-cards { grid-template-columns: 1fr; }
    .pricing-card { padding: 32px 24px; }
    .pricing-amount { font-size: 48px; }

    .install-steps {
        grid-template-columns: 1fr;
        gap: 16px;
    }

    .email-form { flex-direction: column; }
    .hero-email-card { max-width: 100%; }
    .hero-email-form { flex-direction: column; }
    .hero-email-btn { width: 100%; }

    .prompt-mockup { padding: 16px; }
    .prompt-mockup-row { flex-direction: column; }
    .prompt-mockup-input { width: 100%; }

    .footer-content {
        flex-direction: column;
        text-align: center;
    }
    .footer-cta { flex-direction: column; }
    .footer-links { flex-wrap: wrap; justify-content: center; gap: 16px; }
    .section-label { font-size: 11px; letter-spacing: 2px; }
    .section-title { word-break: break-word; }
}
`;
