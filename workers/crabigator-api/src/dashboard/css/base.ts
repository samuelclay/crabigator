// Base styles: variables, resets, scrollbar, font scaling
export const baseCss = `
* { box-sizing: border-box; margin: 0; padding: 0; max-width: 100%; }

:root {
    color-scheme: dark;
    --font-scale: 1;
    --font-ui: 'DM Sans', -apple-system, BlinkMacSystemFont, sans-serif;
    --bg-abyss: #101820;
    --bg-deep: #131e27;
    --bg-surface: #25333e;
    --bg-card: #1b2731;
    --border-dim: #344653;
    --border-glow: #607786;
    --text-bright: #edf1f4;
    --text-mid: #b4bfc7;
    --text-dim: #91a1ad;
    --accent-peach: #ffb98a;
    --accent-cyan: #9ac9df;
    --accent-blue: #83b7df;
    --accent-magenta: #c5a9df;
    --accent-green: #a7cfb6;
    --accent-orange: #edb583;
    --accent-red: #f29a98;
    --accent-purple: #c5b0df;
    --accent-yellow: #e7c78c;
}

html {
    overflow-x: hidden;
    width: 100%;
    /* Live output must not move the page through browser scroll anchoring. */
    overflow-anchor: none;
    scroll-padding-top: calc(var(--header-height, 67px) + 12px);
    touch-action: pan-x pan-y;
}

/* Prevent double-tap zoom on interactive elements */
button, a, [role="button"], [onclick], input, textarea, select, .style-option, .style-popover {
    touch-action: manipulation;
}

/* Keep the interface quiet around the live terminal colors. */
* { scrollbar-width: thin; scrollbar-color: #526574 var(--bg-abyss); }
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: var(--bg-abyss); }
::-webkit-scrollbar-thumb { background: #526574; border-radius: 3px; }
@media (hover: hover) and (pointer: fine) {
::-webkit-scrollbar-thumb:hover { background: #718696; }
}
button, input, textarea, select { font-family: var(--font-ui); }
a { color: var(--accent-cyan); }
:focus-visible { outline: 2px solid var(--accent-peach); outline-offset: 3px; }
@media (prefers-reduced-motion: reduce) {
    html { scroll-behavior: auto; }
    *, *::before, *::after { animation-duration: .01ms !important; animation-iteration-count: 1 !important; transition-duration: .01ms !important; }
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
