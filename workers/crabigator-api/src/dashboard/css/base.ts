// Base styles: variables, resets, scrollbar, font scaling
export const baseCss = `
* { box-sizing: border-box; margin: 0; padding: 0; max-width: 100%; }

:root {
    --font-scale: 1;
    /* Ocean Depths palette */
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
    --accent-purple: #a78bfa;
    --accent-yellow: #fbbf24;
    --glow-cyan: rgba(34, 211, 238, 0.4);
    --glow-magenta: rgba(232, 121, 249, 0.3);
    --glow-green: rgba(74, 222, 128, 0.3);
}

html {
    overflow-x: hidden;
    width: 100%;
    scroll-behavior: smooth;
    touch-action: pan-x pan-y;
}

/* Prevent double-tap zoom on interactive elements */
button, input, textarea, select, .style-option, .style-popover {
    touch-action: manipulation;
}

/* Scanline overlay for retro terminal feel - desktop only (expensive on mobile GPU) */
@media (hover: hover) {
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
            rgba(0, 0, 0, 0.02) 2px,
            rgba(0, 0, 0, 0.02) 4px
        );
        pointer-events: none;
        z-index: 9999;
    }
}

/* Gradient scrollbar */
* {
    scrollbar-width: thin;
    scrollbar-color: var(--accent-cyan) var(--bg-abyss);
}
::-webkit-scrollbar {
    width: 6px;
    height: 6px;
}
::-webkit-scrollbar-track {
    background: var(--bg-abyss);
}
::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--accent-cyan) 0%, var(--accent-magenta) 100%);
    border-radius: 3px;
}
::-webkit-scrollbar-thumb:hover {
    background: linear-gradient(180deg, var(--accent-cyan) 0%, var(--accent-blue) 100%);
}

/* Font size scaling */
.container {
    zoom: var(--font-scale);
}
@supports not (zoom: 1) {
    .container {
        transform: scale(var(--font-scale));
        transform-origin: top left;
    }
}
`;
