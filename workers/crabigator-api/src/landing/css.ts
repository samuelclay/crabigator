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

/* Terminal texture patterns - subtle ~20% opacity */

/* 1. Dot Grid - CTA section */
.cta-section::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background-image: radial-gradient(circle, rgba(34, 211, 238, 0.2) 1px, transparent 1px);
    background-size: 16px 16px;
    pointer-events: none;
}
.cta-section > * { position: relative; z-index: 1; }

/* 2. Scanlines - Showcase section */
.showcase::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background: repeating-linear-gradient(
        0deg,
        transparent 0px,
        transparent 2px,
        rgba(34, 211, 238, 0.15) 2px,
        rgba(34, 211, 238, 0.15) 4px
    );
    pointer-events: none;
}
.showcase > * { position: relative; z-index: 1; }

/* 3. Neon Palm Tree - Interactive section (WebGL canvas created dynamically) */
.interactive > * { position: relative; z-index: 2; }

/* 4. ASCII grid - Pricing section - sparse terminal chars on monospace grid */
.pricing::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='320' height='240'%3E%3Cstyle%3Etext%7Bfill:rgba(34,211,238,0.2);font-family:monospace;font-size:10px%7D%3C/style%3E%3Ctext x='40' y='30'%3E%24%3E%3C/text%3E%3Ctext x='160' y='30'%3E0x7F%3C/text%3E%3Ctext x='280' y='30'%3EEOF%3C/text%3E%3Ctext x='0' y='60'%3E::1%3C/text%3E%3Ctext x='120' y='60'%3E%26%26%3C/text%3E%3Ctext x='240' y='90'%3E%7C%7C%3C/text%3E%3Ctext x='80' y='90'%3ENULL%3C/text%3E%3Ctext x='200' y='120'%3E%23!%3C/text%3E%3Ctext x='40' y='150'%3Esudo%3C/text%3E%3Ctext x='280' y='150'%3E~/%3C/text%3E%3Ctext x='160' y='180'%3E0xFF%3C/text%3E%3Ctext x='0' y='180'%3Epipe%3C/text%3E%3Ctext x='120' y='210'%3E%24PATH%3C/text%3E%3Ctext x='240' y='210'%3Ebin%3C/text%3E%3Ctext x='80' y='240'%3E0x00%3C/text%3E%3C/svg%3E");
    background-size: 320px 240px;
    pointer-events: none;
}
.pricing > * { position: relative; z-index: 1; }

/* 5. Vertical lines (RGB subpixel) - Open Source section */
.open-source::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background: repeating-linear-gradient(
        90deg,
        rgba(248, 113, 113, 0.12) 0px,
        rgba(248, 113, 113, 0.12) 1px,
        rgba(74, 222, 128, 0.12) 1px,
        rgba(74, 222, 128, 0.12) 2px,
        rgba(59, 130, 246, 0.12) 2px,
        rgba(59, 130, 246, 0.12) 3px,
        transparent 3px,
        transparent 6px
    );
    pointer-events: none;
}
.open-source > * { position: relative; z-index: 1; }

/* 6. Binary/hex grid - Mobile Apps section - sparse hex on monospace grid */
.mobile-apps::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='240' height='180'%3E%3Cstyle%3Etext%7Bfill:rgba(34,211,238,0.2);font-family:monospace;font-size:10px%7D%3C/style%3E%3Ctext x='24' y='18'%3E01%3C/text%3E%3Ctext x='120' y='18'%3EFF%3C/text%3E%3Ctext x='216' y='18'%3E10%3C/text%3E%3Ctext x='72' y='54'%3E7A%3C/text%3E%3Ctext x='168' y='54'%3EC2%3C/text%3E%3Ctext x='0' y='72'%3E00%3C/text%3E%3Ctext x='144' y='90'%3E3F%3C/text%3E%3Ctext x='48' y='90'%3E11%3C/text%3E%3Ctext x='192' y='108'%3EAB%3C/text%3E%3Ctext x='96' y='126'%3EE4%3C/text%3E%3Ctext x='24' y='144'%3E5D%3C/text%3E%3Ctext x='168' y='144'%3E8B%3C/text%3E%3Ctext x='72' y='162'%3E00%3C/text%3E%3Ctext x='216' y='180'%3E01%3C/text%3E%3C/svg%3E");
    background-size: 240px 180px;
    pointer-events: none;
}
.mobile-apps > * { position: relative; z-index: 1; }

/* Custom scrollbar */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: var(--bg-abyss); }
::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    border-radius: 3px;
}

a { color: var(--accent-cyan); text-decoration: none; transition: all 0.2s; }
a:hover { color: var(--accent-magenta); text-decoration: none; }
a.btn-primary, a.btn-primary:hover { color: #0a0f1a; }
a.btn-secondary, a.btn-secondary:hover { color: var(--text-mid); }
a.btn-secondary:hover { color: var(--text-bright); }

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
.nav-link.active {
    color: var(--accent-cyan);
    border-color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
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
    min-height: auto;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 120px 32px 40px;
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

/* Dot grid texture - same as cta-section for continuous background */
.hero::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background-image: radial-gradient(circle, rgba(34, 211, 238, 0.2) 1px, transparent 1px);
    background-size: 16px 16px;
    pointer-events: none;
}

.hero-content {
    max-width: 1400px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: minmax(300px, 420px) 1fr;
    gap: 40px;
    align-items: center;
    position: relative;
    z-index: 1;
}

.hero-text {
    max-width: 420px;
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

/* Hero Install */
.hero-install {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-bottom: 32px;
}
.hero-install-command {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 14px 20px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 16px;
    width: fit-content;
    transition: all 0.3s;
}
.hero-install-command:hover {
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.hero-install-prompt {
    color: var(--accent-green);
    font-weight: 600;
}
.hero-install-text {
    color: var(--text-bright);
}
.hero-copy-btn {
    background: transparent;
    border: none;
    padding: 6px;
    cursor: pointer;
    color: var(--text-dim);
    transition: all 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
}
.hero-copy-btn:hover {
    color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.1);
}
.hero-copy-btn.copied {
    color: var(--accent-green);
}
.hero-copy-btn svg {
    width: 16px;
    height: 16px;
}
.hero-install-steps {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
}
.hero-step {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--text-mid);
}
.hero-step code {
    background: var(--bg-surface);
    padding: 2px 8px;
    border-radius: 4px;
    color: var(--accent-cyan);
    border: 1px solid var(--border-dim);
}
.hero-step-link {
    color: var(--accent-cyan);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: all 0.2s;
}
.hero-step-link:hover {
    color: var(--accent-cyan);
    border-bottom-color: var(--accent-cyan);
}
.hero-step-num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: linear-gradient(135deg, var(--accent-magenta) 0%, var(--accent-blue) 100%);
    color: white;
    border-radius: 50%;
    font-size: 11px;
    font-weight: 700;
}
.hero-demo-btn {
    width: fit-content;
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
    color: #0a0f1a;
}
.btn-primary:hover {
    transform: translateY(-2px);
    box-shadow: 0 10px 40px var(--glow-cyan);
    color: #0a0f1a;
}
.btn-primary:hover::after {
    transform: translateX(4px);
    color: #0a0f1a;
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
    color: var(--text-bright);
    border-color: var(--accent-cyan);
    background: rgba(34, 211, 238, 0.08);
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

/* === CTA SECTION === */
.cta-section {
    padding: 24px 32px 48px;
    background: linear-gradient(180deg, var(--bg-abyss) 0%, var(--bg-deep) 100%);
    position: relative;
}
.cta-content {
    display: flex;
    justify-content: center;
}
.cta-card {
    max-width: 480px;
    width: 100%;
    padding: 32px 40px;
    background: rgba(3, 7, 18, 0.7);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(6, 182, 212, 0.2);
    border-radius: 16px;
    text-align: center;
    position: relative;
}
.cta-card::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 16px;
    padding: 1px;
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.3) 0%, rgba(139, 92, 246, 0.1) 100%);
    -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    pointer-events: none;
}
.cta-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin-bottom: 12px;
}
.cta-icon {
    width: 20px;
    height: 20px;
    color: var(--accent-cyan);
}
.cta-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-bright);
    text-transform: uppercase;
    letter-spacing: 1px;
}
.cta-text {
    font-size: 14px;
    color: var(--text-mid);
    margin-bottom: 20px;
}
.cta-form {
    display: flex;
    gap: 10px;
}
.cta-input {
    flex: 1;
    padding: 14px 18px;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    transition: all 0.2s;
}
.cta-input:focus {
    outline: none;
    border-color: var(--accent-cyan);
    box-shadow: 0 0 20px var(--glow-cyan);
}
.cta-input::placeholder {
    color: var(--text-dim);
}
.cta-btn {
    font-family: 'JetBrains Mono', monospace;
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    color: var(--bg-abyss);
    border: none;
    border-radius: 8px;
    padding: 14px 28px;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.2s;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    white-space: nowrap;
}
.cta-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 25px var(--glow-cyan);
}
.cta-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
}
.cta-success {
    display: none;
    text-align: center;
    padding: 14px 16px;
    color: var(--accent-green);
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    font-weight: 500;
}
.cta-success.visible {
    display: block;
}

@media (max-width: 768px) {
    .cta-section { padding: 16px 16px 32px; }
    .cta-card { padding: 24px 20px; }
    .cta-form { flex-direction: column; }
    .cta-btn { width: 100%; }
}

/* Hero Devices */
.hero-devices {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    flex-direction: row-reverse;
}
.hero-terminal {
    position: relative;
    flex: 0 0 auto;
    max-width: 580px;
}

/* Hero Phone */
.hero-phone {
    position: relative;
    flex: 0 0 auto;
    margin-right: -20px;
    z-index: 10;
    width: 220px;
    background: linear-gradient(145deg, #2a2a2a 0%, #1a1a1a 50%, #0a0a0a 100%);
    border: 2px solid #3a3a3a;
    border-radius: 36px;
    padding: 8px;
    box-shadow:
        0 25px 50px rgba(0, 0, 0, 0.5),
        inset 0 1px 1px rgba(255, 255, 255, 0.1),
        0 0 0 1px rgba(255, 255, 255, 0.05);
    z-index: 10;
}
.hero-phone .phone-notch {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    width: 60px;
    height: 20px;
    background: #000;
    border-radius: 12px;
    z-index: 10;
}
.hero-phone .phone-screen {
    background: var(--bg-abyss);
    border-radius: 24px;
    padding: 36px 12px 16px;
    min-height: 380px;
    display: flex;
    flex-direction: column;
    gap: 8px;
}
.phone-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-dim);
    margin-bottom: 4px;
}
.phone-logo {
    width: 16px;
    height: 16px;
}
.phone-title {
    font-size: 10px;
    color: var(--text-mid);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.phone-state {
    font-size: 8px;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 600;
}
.phone-state.thinking {
    background: rgba(251, 191, 36, 0.2);
    color: #fbbf24;
}

/* Phone terminal output */
.phone-terminal {
    background: rgba(13, 17, 23, 0.8);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 10px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 8px;
    line-height: 1.5;
    margin-bottom: 4px;
}
.pt-line {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0;
    animation: typeIn 0.3s ease forwards;
}
.pt-line:nth-child(1) { animation-delay: 0.2s; }
.pt-line:nth-child(2) { animation-delay: 0.4s; }
.pt-line:nth-child(3) { animation-delay: 0.6s; }
.pt-line:nth-child(4) { animation-delay: 0.8s; }
.pt-line:nth-child(5) { animation-delay: 1s; }
.pt-line:nth-child(6) { animation-delay: 1.2s; }
.pt-prompt {
    color: var(--accent-green);
}
.pt-dim {
    color: var(--text-dim);
}
.pt-success {
    color: var(--accent-green);
}
.pt-thinking {
    display: inline-flex;
    gap: 2px;
}
.pt-thinking span {
    width: 3px;
    height: 3px;
    background: var(--accent-cyan);
    border-radius: 50%;
    animation: pulse 1.4s ease-in-out infinite;
}
.pt-thinking span:nth-child(2) { animation-delay: 0.2s; }
.pt-thinking span:nth-child(3) { animation-delay: 0.4s; }

/* Phone widgets */
.phone-widget {
    background: rgba(26, 35, 50, 0.6);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    padding: 10px;
    font-size: 9px;
}
.phone-widget-header {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--accent-cyan);
    font-size: 9px;
    margin-bottom: 6px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.phone-widget-header .pw-value {
    margin-left: auto;
    color: #58a6ff;
    text-transform: none;
}
.phone-widget-header .pw-icon {
    color: var(--accent-cyan);
}
.phone-widget-row {
    display: flex;
    justify-content: space-between;
    color: var(--text-dim);
    padding: 2px 0;
    font-size: 9px;
}
.pw-dim { color: var(--text-dim); }
.pw-sparkline {
    color: #f0883e;
    font-family: 'JetBrains Mono', monospace;
    letter-spacing: -1px;
    font-size: 8px;
}
.pw-branch {
    color: #7ee787;
    text-transform: none;
    letter-spacing: 0;
}
.pw-files {
    margin-left: auto;
    color: #d29922;
    text-transform: none;
    letter-spacing: 0;
}
.pw-lang {
    color: #db6d28;
    text-transform: none;
    letter-spacing: 0;
}
.pw-count {
    margin-left: auto;
    color: var(--text-dim);
    text-transform: none;
    letter-spacing: 0;
}

/* Phone git files */
.phone-git-file {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: 9px;
}
.pf-status { color: #d29922; width: 10px; }
.pf-path {
    color: var(--text-mid);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
}
.pf-diff {
    display: flex;
    gap: 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 8px;
}
.pf-diff .del { color: #f85149; }
.pf-diff .add { color: #3fb950; }

/* Phone changes */
.phone-change {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 2px 0;
    font-size: 9px;
}
.pc-mod { color: #d29922; width: 10px; }
.pc-icon { color: #58a6ff; width: 10px; }
.pc-name { color: var(--text-mid); }
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
.widget-card.changes {
    grid-column: 1 / -1;
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
.widget-state {
    margin-left: auto;
    font-size: 9px;
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.5px;
}
.widget-state.thinking {
    background: rgba(251, 191, 36, 0.2);
    color: #fbbf24;
}
.widget-branch {
    color: #7ee787;
    text-transform: none;
    letter-spacing: 0;
}
.widget-files {
    margin-left: auto;
    color: #d29922;
    text-transform: none;
    letter-spacing: 0;
}
.widget-lang {
    color: #db6d28;
    text-transform: none;
    letter-spacing: 0;
}
.widget-lang.rust {
    color: #f97316;
}
.widget-count {
    color: var(--text-dim);
    text-transform: none;
    letter-spacing: 0;
    margin-left: auto;
}
.widget-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--text-dim);
    padding: 4px 0;
    white-space: nowrap;
    gap: 8px;
}
.widget-label { color: var(--text-dim); }
.widget-value { color: var(--text-bright); font-weight: 500; }
.widget-value.blue { color: #58a6ff; }
.widget-value.green { color: var(--accent-green); }
.widget-value.red { color: var(--accent-red); }
.widget-value.dim { color: var(--text-dim); font-weight: normal; }
.widget-sparkline {
    font-family: 'JetBrains Mono', monospace;
    color: #f0883e;
    letter-spacing: -1px;
}

/* Git files in widget */
.git-files {
    font-size: 11px;
}
.git-file {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
}
.git-status {
    width: 12px;
    text-align: center;
}
.git-status.modified { color: #d29922; }
.git-status.added { color: #3fb950; }
.git-status.deleted { color: #f85149; }
.git-path {
    color: var(--text-mid);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.git-diff {
    display: flex;
    align-items: center;
    gap: 0;
    margin-left: auto;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
}
.git-diff .del-num {
    color: #f85149;
    min-width: 24px;
    text-align: right;
    padding-right: 4px;
}
.git-diff .add-num {
    color: #3fb950;
    min-width: 24px;
    text-align: left;
    padding-left: 4px;
}
.git-diff .bars {
    display: inline-flex;
    gap: 0;
}
.git-diff .bar-del {
    color: #f85149;
    width: 24px;
    text-align: right;
}
.git-diff .bar-add {
    color: #3fb950;
    width: 24px;
    text-align: left;
}

/* Semantic changes in widget */
.changes-lang-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid var(--border-dim);
    font-size: 11px;
}
.changes-list {
    font-size: 11px;
}
.change-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 0;
}
.change-mod {
    width: 10px;
    text-align: center;
}
.change-mod { color: #d29922; }
.change-item:has(.change-mod:contains('+')) .change-mod { color: #3fb950; }
.change-icon {
    width: 14px;
    text-align: center;
}
.change-icon.fn { color: #58a6ff; }
.change-icon.cls { color: #bc8cff; }
.change-icon.struct { color: #39c5cf; }
.change-name {
    color: var(--text-mid);
    flex: 1;
}
.change-stats {
    margin-left: auto;
    font-family: 'JetBrains Mono', monospace;
}
.change-stats .del { color: #f85149; }
.change-stats .add { color: #3fb950; }

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
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
}
.status-pill.thinking { background: rgba(10, 15, 26, 0.85); color: var(--accent-blue); border: 1px solid var(--accent-blue); }
.status-pill.complete { background: rgba(10, 15, 26, 0.85); color: var(--accent-green); border: 1px solid var(--accent-green); }
.status-pill.permission { background: rgba(10, 15, 26, 0.85); color: var(--accent-orange); border: 1px solid var(--accent-orange); }
.status-pill.question { background: rgba(10, 15, 26, 0.85); color: var(--accent-magenta); border: 1px solid var(--accent-magenta); }

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
.showcase-text .section-label {
    justify-content: flex-start;
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
    position: relative;
    overflow: hidden;
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
}
/* Background on pseudo-element so WebGL canvas can show through */
.interactive::after {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--bg-abyss);
    z-index: -1;
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
.interactive-text .section-label {
    justify-content: flex-start;
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

/* === WHY CRABIGATOR === */
.why-crabigator {
    padding: 80px 24px;
    background: linear-gradient(180deg, var(--bg-deep) 0%, var(--bg-abyss) 100%);
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    position: relative;
    overflow: hidden;
}

.why-crabigator::before {
    content: '?';
    position: absolute;
    right: -40px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 400px;
    font-weight: 800;
    color: rgba(34, 211, 238, 0.03);
    pointer-events: none;
    font-family: 'Space Grotesk', sans-serif;
}

.why-inner {
    max-width: 1000px;
    margin: 0 auto;
    text-align: center;
}

.why-question {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 36px;
    font-weight: 700;
    color: var(--text-bright);
    margin-bottom: 48px;
}

.why-equation {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    gap: 24px;
    flex-wrap: wrap;
    margin-bottom: 48px;
}

.why-term {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    min-width: 120px;
}

.why-icon {
    width: 80px;
    height: 80px;
    border-radius: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.3s ease;
}

.why-icon svg {
    width: 44px;
    height: 44px;
}

.why-icon-claude {
    background: linear-gradient(135deg, #e2a07f 0%, #d4896b 100%);
    box-shadow: 0 8px 32px rgba(226, 160, 127, 0.4);
}
.why-icon-claude svg { fill: #1a0f0a; }

.why-icon-nav {
    background: linear-gradient(135deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
    box-shadow: 0 8px 32px rgba(34, 211, 238, 0.4);
}
.why-icon-nav svg { fill: var(--bg-abyss); }

.why-icon-crab {
    background: linear-gradient(135deg, #fb923c 0%, #f97316 100%);
    box-shadow: 0 8px 32px rgba(249, 115, 22, 0.4);
}
.why-icon-crab svg { fill: #1a0f0a; }

.why-icon-gator {
    background: linear-gradient(135deg, #4ade80 0%, #22c55e 100%);
    box-shadow: 0 8px 32px rgba(74, 222, 128, 0.4);
}
.why-icon-gator svg { fill: #0a1a0f; width: 60px; height: 60px; margin: auto; }

.why-term:hover .why-icon {
    transform: translateY(-6px) scale(1.08);
}

.why-plus {
    font-size: 32px;
    color: var(--text-dim);
    font-weight: 300;
    margin-top: 24px;
}

.why-label {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-bright);
}

.why-sub {
    font-size: 12px;
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
}

.why-tagline {
    font-size: 28px;
    font-weight: 700;
    color: var(--accent-cyan);
    font-family: 'JetBrains Mono', monospace;
    letter-spacing: 4px;
    text-transform: uppercase;
    text-shadow: 0 0 40px rgba(34, 211, 238, 0.5);
}

@media (max-width: 768px) {
    .why-crabigator { padding: 48px 16px; }
    .why-question { font-size: 28px; margin-bottom: 32px; }
    .why-equation { gap: 8px; }
    .why-term { min-width: 70px; }
    .why-icon { width: 56px; height: 56px; border-radius: 14px; }
    .why-icon svg { width: 32px; height: 32px; }
    .why-icon-gator svg { width: 48px; height: 48px; }
    .why-plus { font-size: 18px; margin-top: 12px; }
    .why-label { font-size: 12px; }
    .why-sub { font-size: 9px; }
    .why-tagline { font-size: 16px; letter-spacing: 1px; }
    .why-crabigator::before { font-size: 200px; right: -30px; }
}

@media (max-width: 480px) {
    .why-crabigator { padding: 40px 12px; }
    .why-question { font-size: 24px; margin-bottom: 24px; }
    .why-equation { gap: 2px; margin-bottom: 32px; flex-wrap: nowrap; }
    .why-term { min-width: 0; flex: 1 1 0; gap: 4px; }
    .why-icon { width: 44px; height: 44px; border-radius: 12px; }
    .why-icon svg { width: 24px; height: 24px; }
    .why-icon-gator svg { width: 36px; height: 36px; }
    .why-plus { font-size: 12px; margin-top: 10px; flex-shrink: 0; }
    .why-label { font-size: 10px; }
    .why-sub { font-size: 8px; }
    .why-tagline { font-size: 14px; letter-spacing: 1px; }
    .why-crabigator::before { display: none; }
}

/* === SECURITY SECTION === */
.security {
    background: var(--bg-abyss);
    position: relative;
    overflow: hidden;
}

/* Circuit board texture overlay */
.security::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background-image:
        linear-gradient(90deg, rgba(74, 222, 128, 0.08) 1px, transparent 1px),
        linear-gradient(rgba(74, 222, 128, 0.08) 1px, transparent 1px),
        radial-gradient(circle at 20% 30%, rgba(74, 222, 128, 0.05) 0%, transparent 50%),
        radial-gradient(circle at 80% 70%, rgba(34, 211, 238, 0.05) 0%, transparent 50%);
    background-size: 40px 40px, 40px 40px, 100% 100%, 100% 100%;
    pointer-events: none;
}
.security > * { position: relative; z-index: 1; }

.security .section-header {
    margin-bottom: 48px;
}

.security-headline {
    font-size: clamp(28px, 4vw, 42px);
    font-weight: 700;
    letter-spacing: -1px;
    margin-bottom: 16px;
    color: var(--text-bright);
}

.security-subtitle {
    font-size: 16px;
    color: var(--text-mid);
    max-width: 700px;
    margin: 0 auto 56px;
    line-height: 1.7;
}

/* Data flow diagram */
.security-flow {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    margin-bottom: 64px;
    flex-wrap: nowrap;
}

.flow-node {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 24px 32px;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    min-width: 160px;
    position: relative;
    transition: all 0.3s;
}

.flow-node:hover {
    border-color: var(--accent-green);
    box-shadow: 0 0 30px rgba(74, 222, 128, 0.2);
}

.flow-node-icon {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, rgba(74, 222, 128, 0.2) 0%, rgba(74, 222, 128, 0.05) 100%);
    border-radius: 12px;
}

.flow-node-icon svg {
    width: 28px;
    height: 28px;
    fill: var(--accent-green);
}

.flow-node-icon.cloud {
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.2) 0%, rgba(34, 211, 238, 0.05) 100%);
}

.flow-node-icon.cloud svg {
    fill: var(--accent-cyan);
}

.flow-node-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-bright);
    text-align: center;
}

.flow-node-desc {
    font-size: 11px;
    color: var(--text-dim);
    text-align: center;
}

.flow-node-badge {
    position: absolute;
    top: -10px;
    right: -10px;
    background: linear-gradient(135deg, var(--accent-green) 0%, #22c55e 100%);
    color: #0a1a0f;
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    font-weight: 700;
    padding: 4px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* Flow connectors with animated dashes */
.flow-connector {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    position: relative;
}

.flow-line {
    width: 80px;
    height: 3px;
    background: linear-gradient(90deg,
        transparent 0%,
        var(--accent-cyan) 20%,
        var(--accent-cyan) 80%,
        transparent 100%
    );
    border-radius: 2px;
    position: relative;
    overflow: hidden;
}

/* Animated particles flowing on the line */
.flow-line::before {
    content: '';
    position: absolute;
    top: 0;
    left: -50%;
    width: 50%;
    height: 100%;
    background: linear-gradient(90deg,
        transparent,
        rgba(255, 255, 255, 0.8),
        transparent
    );
    animation: flow-particle 2s linear infinite;
}

.flow-line::after {
    content: '';
    position: absolute;
    top: 0;
    right: -50%;
    width: 50%;
    height: 100%;
    background: linear-gradient(90deg,
        transparent,
        rgba(255, 255, 255, 0.8),
        transparent
    );
    animation: flow-particle-reverse 2s linear infinite;
    animation-delay: 1s;
}

@keyframes flow-particle {
    0% { left: -50%; }
    100% { left: 150%; }
}

@keyframes flow-particle-reverse {
    0% { right: -50%; }
    100% { right: 150%; }
}

.flow-lock {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    background: var(--bg-surface);
    border: 2px solid var(--accent-green);
    border-radius: 50%;
}

.flow-lock svg {
    width: 12px;
    height: 12px;
    fill: var(--accent-green);
}

.flow-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--accent-cyan);
    text-transform: uppercase;
    letter-spacing: 1px;
    white-space: nowrap;
}

/* Security features grid */
.security-features {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 24px;
    max-width: 900px;
    margin: 0 auto;
}

.security-feature {
    display: flex;
    gap: 16px;
    padding: 24px;
    background: var(--bg-card);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    transition: all 0.3s;
}

.security-feature:hover {
    border-color: var(--accent-green);
    transform: translateY(-2px);
}

.security-feature-icon {
    width: 44px;
    height: 44px;
    min-width: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, rgba(74, 222, 128, 0.2) 0%, rgba(74, 222, 128, 0.05) 100%);
    border-radius: 10px;
}

.security-feature-icon svg {
    width: 22px;
    height: 22px;
    fill: var(--accent-green);
}

.security-feature-content {
    flex: 1;
}

.security-feature-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-bright);
    margin-bottom: 6px;
}

.security-feature-desc {
    font-size: 13px;
    color: var(--text-mid);
    line-height: 1.5;
}

/* Security section responsive */
@media (max-width: 900px) {
    .security-flow {
        display: grid;
        grid-template-columns: 1fr auto 1fr;
        grid-template-rows: auto auto;
        gap: 56px 8px;
        align-items: center;
        justify-items: center;
        max-width: 500px;
        margin: 0 auto 64px;
    }

    /* Desktop: top left */
    .flow-node:nth-child(1) {
        grid-column: 1;
        grid-row: 1;
        justify-self: end;
    }

    /* First connector: diagonal to center right */
    .flow-connector:nth-child(2) {
        grid-column: 2;
        grid-row: 1;
        transform: rotate(30deg);
    }

    /* Cloudflare: center right */
    .flow-node:nth-child(3) {
        grid-column: 3;
        grid-row: 1 / 3;
        align-self: center;
    }

    /* Second connector: diagonal to bottom left */
    .flow-connector:nth-child(4) {
        grid-column: 2;
        grid-row: 2;
        transform: rotate(-30deg);
    }

    /* Phone: bottom left */
    .flow-node:nth-child(5) {
        grid-column: 1;
        grid-row: 2;
        justify-self: end;
    }

    .flow-line {
        width: 50px;
    }

    .security-features {
        grid-template-columns: repeat(2, 1fr);
        gap: 16px;
    }

    .security-feature {
        flex-direction: column;
        text-align: center;
        gap: 12px;
    }

    .security-feature-icon {
        margin: 0 auto;
    }
}

@media (max-width: 600px) {
    .security-flow {
        max-width: 380px;
        gap: 48px 6px;
    }

    .flow-node {
        min-width: 130px;
        padding: 16px 18px;
    }

    .flow-line {
        width: 40px;
    }

    .flow-label {
        font-size: 9px;
    }

    .security-features {
        gap: 12px;
    }

    .security-feature {
        padding: 14px;
    }

    .security-feature-icon {
        width: 32px;
        height: 32px;
        min-width: 32px;
    }

    .security-feature-icon svg {
        width: 16px;
        height: 16px;
    }

    .security-feature-title {
        font-size: 13px;
    }

    .security-feature-desc {
        font-size: 12px;
    }
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

/* === OPEN SOURCE / GITHUB === */
.open-source {
    background: var(--bg-abyss);
    padding: 80px 32px;
}
.github-card {
    max-width: 600px;
    margin: 0 auto;
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 28px;
}
.github-header {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 16px;
}
.github-logo {
    width: 32px;
    height: 32px;
    color: var(--text-bright);
    flex-shrink: 0;
}
.github-repo {
    font-family: 'JetBrains Mono', monospace;
    font-size: 20px;
    font-weight: 600;
}
.github-org {
    color: var(--text-mid);
}
.github-sep {
    color: var(--text-dim);
    margin: 0 2px;
}
.github-name {
    color: var(--accent-cyan);
}
.github-desc {
    color: var(--text-mid);
    font-size: 14px;
    line-height: 1.6;
    margin-bottom: 20px;
}
.github-meta {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-bottom: 24px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
}
.github-lang {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-mid);
}
.lang-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
}
.lang-dot.rust {
    background: #dea584;
}
.github-license {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-mid);
}
.github-license svg {
    width: 14px;
    height: 14px;
}
.github-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 20px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border-dim);
}
.github-btn {
    font-family: 'JetBrains Mono', monospace;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    color: var(--text-mid);
    font-size: 12px;
    font-weight: 500;
    transition: all 0.2s;
    text-decoration: none;
}
.github-btn svg {
    width: 16px;
    height: 16px;
}
.github-btn:hover {
    background: var(--bg-card);
    border-color: var(--text-dim);
    color: var(--text-bright);
}
.github-btn.primary {
    background: linear-gradient(135deg, #238636 0%, #2ea043 100%);
    border-color: #238636;
    color: #ffffff;
}
.github-btn.primary:hover {
    background: linear-gradient(135deg, #2ea043 0%, #3fb950 100%);
    box-shadow: 0 0 20px rgba(46, 160, 67, 0.3);
}
.github-btn.primary svg {
    color: #fbbf24;
}
.github-links {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 20px;
}
.github-link {
    font-family: 'JetBrains Mono', monospace;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-dim);
    font-size: 12px;
    text-decoration: none;
    transition: color 0.2s;
}
.github-link svg {
    width: 14px;
    height: 14px;
}
.github-link:hover {
    color: var(--accent-cyan);
}

/* === MOBILE APPS === */
.mobile-apps {
    background: linear-gradient(180deg, var(--bg-deep) 0%, var(--bg-abyss) 100%);
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    padding: 80px 32px;
    overflow: hidden;
}
.mobile-content {
    max-width: 1100px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 60px;
    align-items: center;
}
.mobile-phones {
    display: flex;
    justify-content: center;
    gap: 24px;
    perspective: 1000px;
}
.phone {
    width: 180px;
    background: linear-gradient(180deg, #1a1a2e 0%, #16213e 100%);
    border-radius: 28px;
    padding: 8px;
    box-shadow: 0 25px 50px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.1);
    position: relative;
    transition: transform 0.3s ease;
}
.phone:hover {
    transform: translateY(-8px);
}
.phone.iphone {
    transform: rotate(-5deg);
}
.phone.iphone:hover {
    transform: rotate(-5deg) translateY(-8px);
}
.phone.android {
    transform: rotate(5deg);
}
.phone.android:hover {
    transform: rotate(5deg) translateY(-8px);
}
.phone-notch {
    width: 80px;
    height: 20px;
    background: #0a0a0a;
    border-radius: 0 0 12px 12px;
    margin: 0 auto;
    position: absolute;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 10;
}
.phone-screen {
    background: var(--bg-abyss);
    border-radius: 20px;
    padding: 36px 12px 16px;
    min-height: 320px;
    display: flex;
    flex-direction: column;
    gap: 12px;
}
.phone.android .phone-screen {
    padding-top: 16px;
}
.app-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-dim);
}
.app-logo {
    font-size: 16px;
}
.app-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-bright);
}
.app-notification {
    background: var(--bg-card);
    border-radius: 10px;
    padding: 10px;
    display: flex;
    gap: 10px;
    align-items: flex-start;
    border-left: 3px solid var(--accent-orange);
}
.notif-icon {
    width: 24px;
    height: 24px;
    color: var(--accent-orange);
    flex-shrink: 0;
}
.notif-icon svg {
    width: 100%;
    height: 100%;
}
.notif-content {
    min-width: 0;
}
.notif-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    font-weight: 600;
    color: var(--accent-orange);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
}
.notif-body {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--text-mid);
}
.app-session {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
}
.session-state {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    padding: 4px 8px;
    border-radius: 4px;
    text-transform: uppercase;
}
.session-state.thinking {
    background: rgba(59, 130, 246, 0.2);
    color: var(--accent-blue);
}
.session-path {
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    color: var(--text-dim);
}
.app-actions {
    display: flex;
    gap: 8px;
    margin-top: auto;
}
.app-btn {
    flex: 1;
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    font-weight: 600;
    padding: 10px 8px;
    border-radius: 8px;
    border: none;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.app-btn.approve {
    background: linear-gradient(135deg, #238636 0%, #2ea043 100%);
    color: white;
}
.app-btn.deny {
    background: var(--bg-surface);
    color: var(--text-mid);
    border: 1px solid var(--border-dim);
}
.app-terminal {
    background: var(--bg-surface);
    border-radius: 8px;
    padding: 10px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
}
.term-line {
    color: var(--text-mid);
    margin-bottom: 4px;
}
.term-line:last-child {
    margin-bottom: 0;
}
.term-prompt {
    color: var(--accent-cyan);
}
.term-file {
    color: var(--accent-magenta);
}
.term-added {
    color: var(--accent-green);
}
.term-removed {
    color: var(--accent-red);
    margin-left: 6px;
}
.app-stats {
    display: flex;
    justify-content: space-around;
    padding: 12px 0;
    border-top: 1px solid var(--border-dim);
    margin-top: auto;
}
.stat {
    text-align: center;
}
.stat-value {
    font-family: 'JetBrains Mono', monospace;
    font-size: 16px;
    font-weight: 700;
    color: var(--accent-cyan);
    display: block;
}
.stat-label {
    font-size: 8px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.phone-label {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-top: 16px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
}
.phone-label svg {
    width: 14px;
    height: 14px;
}
.mobile-info {
    text-align: left;
}
.mobile-info .section-label {
    justify-content: flex-start;
}
.mobile-info .section-title {
    text-align: left;
    margin-bottom: 24px;
}
.mobile-features {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-bottom: 32px;
}
.mobile-feature {
    display: flex;
    gap: 16px;
    align-items: flex-start;
}
.mobile-feature svg {
    width: 24px;
    height: 24px;
    color: var(--accent-magenta);
    flex-shrink: 0;
    margin-top: 2px;
}
.mobile-feature strong {
    display: block;
    color: var(--text-bright);
    font-size: 15px;
    margin-bottom: 4px;
}
.mobile-feature span {
    color: var(--text-mid);
    font-size: 13px;
    line-height: 1.5;
}
.mobile-info .email-form {
    margin: 0;
    max-width: none;
}
.mobile-info .email-privacy {
    text-align: left;
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
    margin-top: 16px;
}
.email-success {
    display: none;
    text-align: center;
    padding: 16px;
    background: rgba(74, 222, 128, 0.1);
    border: 1px solid var(--accent-green);
    border-radius: 8px;
    color: var(--accent-green);
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
}
.email-success.visible { display: block; }

/* Mobile apps section - responsive */
@media (max-width: 900px) {
    .mobile-content {
        grid-template-columns: 1fr;
        gap: 48px;
    }
    .mobile-phones {
        order: -1;
    }
    .mobile-info {
        text-align: center;
    }
    .mobile-info .section-label {
        justify-content: center;
    }
    .mobile-info .section-title {
        text-align: center;
    }
    .mobile-features {
        max-width: 400px;
        margin: 0 auto 32px;
    }
    .mobile-feature {
        text-align: left;
    }
    .mobile-info .email-form {
        max-width: 400px;
        margin: 0 auto;
    }
    .mobile-info .email-privacy {
        text-align: center;
    }
}
@media (max-width: 500px) {
    .phone {
        width: 150px;
    }
    .phone-screen {
        min-height: 260px;
        padding: 30px 10px 12px;
    }
    .mobile-phones {
        gap: 16px;
    }
    .email-form {
        flex-direction: column;
    }
    .email-btn {
        width: 100%;
    }
}

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
@media (max-width: 1200px) {
    .hero-content {
        grid-template-columns: minmax(250px, 320px) 1fr;
        gap: 20px;
    }
    .hero-text {
        max-width: 320px;
    }
    .hero-headline {
        font-size: clamp(36px, 5vw, 56px);
    }
    .hero-terminal {
        max-width: 480px;
    }
}

@media (max-width: 1024px) {
    .hero-content {
        grid-template-columns: 1fr;
        gap: 48px;
    }
    .hero-text {
        max-width: 600px;
        margin: 0 auto;
        text-align: center;
    }
    .hero-ctas {
        justify-content: center;
    }
    .hero-install-command {
        width: auto;
    }
    .hero-price {
        justify-content: center;
    }
    .hero-devices {
        /* JS handles the phone+terminal scaling */
        min-width: 0;
        max-width: 100vw;
        margin: 0;
    }
    .hero-phone {
        /* JS handles positioning */
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
        position: relative;
        max-width: 600px;
        margin: 0 auto;
        text-align: center;
    }
    .interactive-text .section-label {
        justify-content: center;
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
        padding: 100px 0 60px;
        min-height: auto;
        overflow: visible;
    }
    .hero-content {
        padding: 0 16px;
        overflow: visible;
    }
    .hero-text {
        max-width: 100%;
        text-align: left;
        margin: 0;
        overflow-wrap: break-word;
    }
    .hero-headline { font-size: 32px; letter-spacing: -1px; }
    .hero-subheadline { font-size: 15px; }
    .hero-install-command { font-size: 14px; padding: 12px 16px; gap: 10px; }
    .hero-ctas { flex-direction: column; align-items: stretch; }
    .hero-price { font-size: 11px; gap: 8px; justify-content: center; }
    .btn-primary, .btn-secondary { width: auto; display: flex; justify-content: center; text-align: center; padding: 14px 24px; }
    /* hero-devices scaling handled by JavaScript */
    .hero-devices {
        overflow: visible;
        max-width: 100vw;
        box-sizing: border-box;
    }
    .hero-phone {
        left: 0 !important;
        position: relative !important;
    }
    .hero-terminal {
        /* JS will set dimensions */
    }
    .terminal-window {
        transform: none;
    }
    .terminal-widgets { grid-template-columns: 1fr; }
    .widget-card.changes { display: none; }

    .section { padding: 80px 16px; }
    .section-title { font-size: 28px; }

    .bento-grid {
        grid-template-columns: repeat(2, 1fr);
        gap: 10px;
    }
    .bento-card.large,
    .bento-card.medium,
    .bento-card.small {
        grid-column: span 1;
        grid-row: span 1;
    }
    .bento-card.tall,
    .bento-card.wide,
    .bento-card.wide.tall {
        grid-column: span 2;
        grid-row: span 1;
    }
    .bento-card {
        padding: 14px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .bento-card.small {
        padding: 14px;
    }
    .bento-card.tall,
    .bento-card.wide.tall {
        padding: 16px;
        display: grid;
        grid-template-columns: 1fr 1fr;
        grid-template-rows: auto auto 1fr;
        gap: 4px 16px;
        align-items: start;
    }
    .bento-card.tall .bento-icon,
    .bento-card.wide.tall .bento-icon {
        grid-column: 1;
        grid-row: 1;
        width: 32px;
        height: 32px;
        margin-bottom: 0;
    }
    .bento-card.tall .bento-title,
    .bento-card.wide.tall .bento-title {
        grid-column: 1;
        grid-row: 2;
        font-size: 14px;
    }
    .bento-card.tall .bento-desc,
    .bento-card.wide.tall .bento-desc {
        grid-column: 1;
        grid-row: 3;
        font-size: 11px;
    }
    .bento-card.tall .bento-visual,
    .bento-card.wide.tall .bento-visual {
        grid-column: 2;
        grid-row: 1 / 4;
        display: flex;
        align-items: center;
        margin-top: 0;
    }
    /* Alternate: visual on left for 2nd and 4th tall cards */
    .bento-grid > .bento-card.tall:nth-child(2),
    .bento-grid > .bento-card.tall:nth-child(8) {
        direction: rtl;
    }
    .bento-grid > .bento-card.tall:nth-child(2) > *,
    .bento-grid > .bento-card.tall:nth-child(8) > * {
        direction: ltr;
    }
    .bento-icon {
        width: 36px;
        height: 36px;
        margin-bottom: 4px;
    }
    .bento-icon svg { width: 20px; height: 20px; }
    .bento-title { font-size: 14px; margin-bottom: 4px; }
    .bento-desc { font-size: 12px; line-height: 1.5; }
    .bento-card.small .bento-visual { display: none; }
    .mini-widget.semantic { flex-direction: column; gap: 12px; }

    /* Multi-device section mobile */
    .showcase { padding: 60px 16px; }
    .status-pills {
        display: flex;
        flex-wrap: nowrap;
        gap: 6px;
        justify-content: flex-start;
    }
    .status-pill {
        padding: 4px 8px;
        font-size: 9px;
        white-space: nowrap;
    }
    .showcase-status { margin-top: 24px; }
    .status-hint { font-size: 12px; margin-top: 8px; }
    .showcase-devices {
        height: auto;
        display: flex;
        flex-direction: column;
        gap: 16px;
        margin-top: 32px;
    }
    .device-phone {
        position: static;
        transform: none;
        width: 100%;
        max-width: 240px;
        margin: 0 auto;
        padding: 8px;
        border-width: 2px;
        border-radius: 24px;
    }
    .device-phone::before {
        width: 60px;
        height: 18px;
        top: 8px;
    }
    .phone-screen {
        border-radius: 18px;
        padding: 28px 12px 12px;
        min-height: auto;
    }
    .session-card { padding: 10px; margin-bottom: 8px; }
    .session-path { font-size: 10px; }
    .session-state { font-size: 8px; padding: 2px 6px; }
    .session-preview { font-size: 9px; padding: 8px; }
    .device-browser {
        position: static;
        transform: none;
        width: 100%;
        max-width: 280px;
        margin: 0 auto;
    }
    .browser-content { min-height: auto; padding: 12px; }
    .browser-url { font-size: 9px; padding: 4px 8px; }

    .pricing-cards { grid-template-columns: 1fr; }
    .pricing-card { padding: 32px 24px; }
    .pricing-amount { font-size: 48px; }

    .install-steps {
        grid-template-columns: 1fr;
        gap: 12px;
    }
    .install-step {
        display: flex;
        align-items: center;
        text-align: left;
        padding: 16px;
        gap: 16px;
    }
    .install-step-num {
        margin: 0;
        width: 36px;
        height: 36px;
        min-width: 36px;
        font-size: 14px;
    }
    .install-step-text {
        font-size: 13px;
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
