// Home page styles. The MCP reference retains landing/css.ts.
export const homeCss = `
:root {
    color-scheme: dark;
    --paper: #101820; --ink: #edf1f4; --muted: #a3b0ba; --surface: #1b2731;
    --peach: #ffb98a; --line: #344653;
    --bg-abyss: #101820; --bg-deep: #17222b; --bg-surface: #25333e; --bg-card: #1b2731;
    --border-dim: #344653; --text-bright: #edf1f4; --text-mid: #a3b0ba; --text-dim: #a3b0ba;
    --accent-cyan: #9ac9df; --accent-blue: #83b7df; --accent-magenta: #c5a9df; --accent-green: #a7cfb6;
}
*, *::before, *::after { box-sizing: border-box; }
html { scroll-behavior: smooth; scroll-padding-top: 32px; }
body { margin: 0; background: var(--paper); color: var(--ink); font: 16px/1.55 'DM Sans', sans-serif; -webkit-font-smoothing: antialiased; }
h1, h2, h3, p, figure { margin: 0; }
a { color: inherit; text-decoration-thickness: 1px; text-underline-offset: 5px; }
button, input { font: inherit; }
button, a { -webkit-tap-highlight-color: transparent; }
button { cursor: pointer; }
a:focus-visible, button:focus-visible, input:focus-visible { outline: 3px solid var(--peach); outline-offset: 5px; }
button:disabled { cursor: wait; opacity: .6; }
code { font-family: 'JetBrains Mono', monospace; font-size: .82em; }
svg { display: inline-block; width: 20px; height: 20px; vertical-align: middle; }
.icon { fill: currentColor; }
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; border: 0; }
.site-header { display: flex; align-items: center; justify-content: space-between; max-width: 1536px; margin: 0 auto; padding: 28px 48px; gap: 24px; }
.wordmark { display: inline-flex; align-items: center; flex-shrink: 0; text-decoration: none; }
.wordmark .brand-logo { display: block; width: 270px; height: 34px; }
.site-header nav { display: flex; align-items: center; gap: 28px; font-size: 14px; }
.site-header nav a { text-decoration: none; }
.site-header nav a:hover { color: var(--peach); }
.dashboard-link { border: 1px solid var(--line); padding: 10px 17px; border-radius: 6px; }
.hero { display: grid; grid-template-columns: 1fr 1.6fr; align-items: center; max-width: 1536px; margin: 0 auto; padding: 42px 48px 35px; gap: 28px; }
.hero-copy { display: flex; flex-direction: column; gap: 62px; padding-bottom: 30px; }
.hero h1 { font-size: clamp(48px, 4.65vw, 72px); font-weight: 450; letter-spacing: -.06em; line-height: 1.06; }
.hero-description { margin-top: 27px; max-width: 280px; font-size: 18px; color: var(--muted); }
.hero-start > p { font-size: 12px; color: var(--muted); margin-top: 16px; }
.install-link { display: inline-flex; align-items: center; gap: 35px; padding-bottom: 8px; border-bottom: 1px solid var(--peach); color: var(--peach); text-decoration: none; font-size: 17px; font-weight: 550; }
.install-link span { font-size: 24px; line-height: 1; }
.hero-screens { position: relative; min-width: 0; height: 650px; }
.desktop-frame { container-type: inline-size; position: absolute; width: 76%; top: 50px; left: 0; border: 1px solid #52616b; border-radius: 10px; overflow: hidden; background: #101214; box-shadow: 0 30px 70px #0004; }
.desktop-bar, .browser-bar { display: flex; align-items: center; gap: 16px; padding: 13px 16px; background: #28343e; color: #d1dbe1; font-size: 11px; }
.desktop-bar > span:last-child { margin-left: auto; color: #a3b0ba; font-size: 9px; }
.window-dots { display: inline-flex; gap: 5px; }
.window-dots i { width: 7px; height: 7px; border-radius: 50%; background: #d78072; }
.window-dots i:nth-child(2) { background: #d5b477; }
.window-dots i:nth-child(3) { background: #88b99b; }
.desktop-capture { font: 1.2cqw/1.8 'JetBrains Mono', monospace; padding: 24px 18px; min-height: 315px; overflow: hidden; white-space: pre; color: #dedede; }
.desktop-capture .line { min-height: 1.8em; }
.phone-frame { container-type: inline-size; aspect-ratio: 1 / 2.17; padding: 5px; background: #07090c; border: 2px solid #65717a; border-radius: 34px; box-shadow: 0 24px 60px #0006; }
.phone-screen { display: flex; flex-direction: column; width: 100%; height: 100%; overflow: hidden; border-radius: 27px; background: #0d131b; }
.phone-status { position: relative; display: flex; justify-content: space-between; align-items: center; flex-shrink: 0; height: 17cqw; padding: 0 7cqw; color: #edf1f4; font: 600 4.2cqw 'DM Sans', sans-serif; }
.phone-island { position: absolute; width: 30%; height: 6.7cqw; top: 4cqw; left: 35%; border-radius: 20px; background: #000; }
.phone-indicators { display: flex; align-items: center; gap: 1.3cqw; }
.phone-indicators svg { width: 5cqw; height: 5cqw; }
.signal-bars { display: flex; align-items: end; gap: .65cqw; height: 4cqw; }
.signal-bars i { width: 1cqw; height: 45%; background: currentColor; border-radius: 1px; }
.signal-bars i:nth-child(2) { height: 65%; }.signal-bars i:nth-child(3) { height: 82%; }.signal-bars i:nth-child(4) { height: 100%; }
.phone-battery { width: 7cqw; height: 3.7cqw; border: 1px solid #edf1f4; border-radius: 2px; background: #edf1f4; box-shadow: inset -2px 0 #0d131b; }
.phone-app-bar { display: flex; align-items: center; flex-shrink: 0; gap: 4cqw; padding: 2cqw 5cqw 4cqw; border-bottom: 1px solid #29313a; color: #67b3f7; }
.phone-back { font: 300 12cqw/1 sans-serif; }
.phone-app-bar strong { display: flex; align-items: center; gap: 2cqw; font-size: 5cqw; font-weight: 550; }
.phone-app-bar strong svg { width: 6cqw; height: 6cqw; }
.phone-app-bar strong .brand-wordmark { width: 43cqw; height: auto; }
.phone-app-bar div > span { display: block; margin-top: 1cqw; color: #8bd78d; font-size: 3.5cqw; }
.phone-type { margin-left: auto; color: #a8b1ba; font-size: 5cqw; }
.phone-terminal { flex: 1; min-height: 0; overflow: hidden; padding: 5cqw 4cqw; background: #0a0f17; color: #d9e1eb; font: 3.3cqw/1.55 'JetBrains Mono', monospace; white-space: pre-wrap; overflow-wrap: anywhere; }
.phone-terminal .line { min-height: 1.55em; }
.phone-terminal .rule-line { white-space: pre; overflow: hidden; }
.phone-composer { display: flex; align-items: center; flex-shrink: 0; gap: 4cqw; padding: 3cqw 4cqw 1cqw; border-top: 1px solid #29313a; color: #9aa4af; }
.phone-composer svg { width: 5cqw; height: 5cqw; }
.phone-composer > span { flex: 1; border: 1px solid #3c4651; padding: 3cqw; border-radius: 5cqw; color: #77818d; font-size: 4cqw; }
.phone-home { display: flex; justify-content: center; flex-shrink: 0; padding: 3cqw 0 2cqw; }
.phone-home span { width: 34%; height: 1.2cqw; border-radius: 3px; background: #edf1f4; }
.hero-phone { position: absolute; width: 25%; min-width: 200px; right: -8px; top: 110px; }
.hero-screens figcaption { position: absolute; left: 0; right: 0; bottom: 0; display: flex; align-items: center; gap: 20px; color: var(--muted); font-size: 12px; }
.connection-line { flex: 1; height: 1px; background: #52616b; position: relative; }
.connection-line::after { content: ''; position: absolute; width: 6px; height: 6px; border-radius: 50%; background: #a7cfb6; right: 0; top: -2.5px; }
.providers { display: flex; justify-content: center; flex-wrap: wrap; gap: 24px 40px; max-width: 1296px; margin: 0 auto; padding: 35px 24px 80px; font-size: 14px; color: var(--muted); }
.providers span { display: flex; align-items: center; gap: 8px; }
.providers svg { width: 18px; height: 18px; fill: currentColor; }
.section { max-width: 1296px; margin: 0 auto; padding: 100px 48px; }
.section h2 { font-size: clamp(32px, 3.4vw, 48px); line-height: 1.14; letter-spacing: -.045em; font-weight: 450; }
.story-heading { display: flex; align-items: end; justify-content: space-between; gap: 64px; margin-bottom: 46px; }
.story-heading > div { max-width: 340px; }
.story-heading p, .notification-copy > p { color: var(--muted); }
.availability { font-size: 12px; margin-top: 18px; }
.native { padding-top: 45px; }
.tablet-frame { padding: 12px; border: 2px solid #52616b; border-radius: 30px; background: #070b0e; box-shadow: 0 20px 70px #0003; }
.tablet-preview figcaption, .web-preview figcaption { margin-top: 18px; color: var(--muted); font-size: 12px; }
.notifications { display: grid; grid-template-columns: 1fr 1fr; gap: 100px; align-items: center; }
.notification-copy > p { max-width: 310px; margin-top: 26px; }
.notification-copy > .availability { margin-top: 18px; }
.notification-preview { padding: 32px; background: #25343e; border-radius: 12px; }
.notification-crop { aspect-ratio: 750 / 1060; overflow: hidden; border-radius: 18px; }
.notification-crop img { display: block; width: 100%; height: auto; margin-top: 0; }
.text-link { display: inline-block; margin-top: 24px; color: var(--peach); font-size: 14px; }
.browser-frame { border: 1px solid #52616b; border-radius: 10px; overflow: hidden; background: #0d1117; box-shadow: 0 20px 70px #0003; }
.browser-bar { gap: 30px; }
.browser-bar > span:last-child { border-radius: 4px; padding: 4px 20px; background: #17222b; color: #a3b0ba; font-size: 11px; }
/* The existing MCP controls share the home page palette. */
.mcp-section { border: 0; overflow: visible; }
.mcp-section::before { content: none; }
.mcp-intro { grid-template-columns: minmax(0, 1fr) minmax(0, 420px); gap: 80px; }
.mcp-intro-copy { text-align: left; }
.mcp-intro-copy .section-subtitle { margin: 25px 0; color: var(--muted); font-size: 16px; }
.mcp-connect-row { background: transparent; border-color: var(--line); border-radius: 5px; }
.mcp-connect-label { background: transparent; color: var(--muted); font: 12px 'DM Sans', sans-serif; text-transform: none; letter-spacing: 0; }
.mcp-connect-value { color: var(--ink); font-size: 11px; }
.mcp-connect-copy { background: transparent; color: var(--ink); }
.mcp-demo { background: #17222b; color: #dbe7ed; border: 1px solid var(--line); box-shadow: none; }
.mcp-demo-bar { background: #25333e; border: 0; }
.mcp-demo-title { color: #b5c6d2; }
.mcp-demo-call, .mcp-demo .json-key { color: #a9d7e9; }
.mcp-demo-json { color: #dbe7ed; }
.mcp-demo .json-string { color: #a4d6b6; }
.btn-primary { display: inline-block; color: var(--peach); font-size: 14px; text-decoration: underline; }
.ownership { display: grid; grid-template-columns: 1fr 1fr; gap: 90px; }
.ownership-copy { max-width: 420px; color: var(--muted); }
.ownership-copy p + p { margin-top: 18px; }
.ownership-copy a { display: inline-block; margin-top: 26px; color: var(--peach); }
.install { display: grid; grid-template-columns: 1fr 1.1fr; gap: 80px; border-top: 1px solid var(--line); }
.install-heading > p { margin-top: 22px; color: var(--muted); font-size: 14px; }
.install-command { display: flex; align-items: center; justify-content: space-between; gap: 12px; background: var(--surface); padding: 20px; border-radius: 6px; }
.install-command code { overflow-wrap: anywhere; }
.install-command button { background: transparent; color: var(--ink); border: 0; padding: 6px; }
.install-command button.copied { color: #a7cfb6; }
.install-command button.copied::after { content: "Copied"; font-size: 11px; }
.install-command button.copied svg { display: none; }
.install-steps { padding-left: 22px; margin: 28px 0 0; color: var(--muted); font-size: 14px; }
.install-steps li { padding-left: 8px; margin: 16px 0; }
.install-steps code { color: var(--ink); white-space: nowrap; }
.updates { display: flex; justify-content: space-between; gap: 64px; align-items: center; padding-top: 25px; }
.updates h2 { font-size: 26px; }
.updates p { color: var(--muted); font-size: 14px; margin-top: 10px; }
.email-form { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; }
.email-form input { background: transparent; border: 1px solid var(--line); border-radius: 5px; padding: 12px; color: var(--ink); min-width: 220px; font-size: 14px; }
.email-form button { background: var(--peach); color: var(--paper); border: 1px solid var(--peach); border-radius: 5px; padding: 12px 16px; font-size: 14px; }
.email-form p { width: 100%; }
.footer { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 28px; padding: 36px 48px; margin: 0 auto; border-top: 1px solid var(--line); max-width: 1536px; }
.footer > p { font-size: 12px; color: var(--muted); }
.footer-links { display: flex; gap: 24px; font-size: 12px; }
@media (max-width: 1100px) {
    .site-header { padding: 24px 28px; }
    .hero { grid-template-columns: 1fr; padding: 45px 28px 20px; }
    .hero-copy { flex-direction: row; align-items: end; justify-content: space-between; gap: 30px; padding: 0; }
    .hero h1 { font-size: 60px; }
    .hero-description { max-width: none; font-size: 16px; margin-top: 20px; }
    .hero-screens { width: 100%; max-width: 790px; margin: 0 auto; height: 650px; }
    .hero-phone { min-width: 0; width: 25%; right: 0; }
    .section { padding: 80px 28px; }
    .providers { padding-bottom: 40px; }
    .mcp-intro { gap: 40px; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
    .notifications, .ownership, .install { gap: 50px; }
    .updates { gap: 30px; }
    .footer { padding: 32px 28px; }
}
@media (max-width: 768px) {
    .site-header { padding: 18px 20px; gap: 12px; }
    .site-header .wordmark .brand-logo { width: clamp(144px, calc(100vw - 178px), 216px); height: auto; }
    .site-header nav { font-size: 12px; }
    .site-header nav > a:not(.dashboard-link) { display: none; }
    .dashboard-link { padding: 8px 10px; }
}
@media (max-width: 700px) {
    .hero { padding: 36px 24px 18px; gap: 24px; }
    .hero-copy { display: block; }
    .hero h1 { font-size: clamp(42px, 10.8vw, 66px); }
    .hero-description { font-size: 16px; max-width: 260px; }
    .hero-start { margin-top: 28px; }
    .hero-start > p { display: none; }
    .install-link { font-size: 15px; gap: 24px; }
    .hero-screens { height: calc(94vw + 110px); }
    .desktop-frame { top: 22px; width: 94%; border-radius: 7px; }
    .desktop-bar { padding: 9px; gap: 10px; font-size: 8px; }
    .desktop-bar > span:last-child { display: none; }
    .window-dots { gap: 3px; }
    .window-dots i { width: 5px; height: 5px; }
    .desktop-capture { padding: 14px 9px; min-height: 0; }
    .hero-phone { width: 43%; top: 112px; right: -7px; }
    .phone-frame { padding: 4px; border-radius: 18px; }
    .phone-screen { border-radius: 12px; }
    .hero-screens figcaption { font-size: 10px; gap: 14px; }
    .providers { padding: 12px 24px 40px; gap: 18px 25px; font-size: 12px; }
    .providers svg { width: 16px; height: 16px; }
    .section { padding: 60px 24px; }
    .section h2 { font-size: 32px; }
    .story-heading { display: block; margin-bottom: 28px; }
    .story-heading > div { margin-top: 22px; max-width: 420px; }
    .tablet-frame { padding: 5px; border-radius: 12px; }
    .notifications, .ownership, .install { grid-template-columns: 1fr; gap: 32px; }
    .notification-preview { padding: 24px; }
    .notification-copy > p { max-width: 400px; }
    .browser-bar { padding: 10px; gap: 12px; }
    .browser-bar > span:last-child { padding: 4px 8px; font-size: 8px; }
    .mcp-intro { grid-template-columns: minmax(0, 1fr); gap: 36px; }
    .mcp-demo { max-width: 100%; }
    .mcp-connect-label { padding: 10px; }
    .mcp-connect-value { padding: 12px 10px; font-size: 10px; }
    .updates { flex-direction: column; align-items: stretch; gap: 26px; }
    .email-form { align-items: stretch; }
    .email-form input { min-width: 0; flex: 1; width: 150px; }
    .footer { padding: 28px 24px; align-items: start; flex-direction: column; gap: 18px; }
}
@media (prefers-reduced-motion: reduce) { html { scroll-behavior: auto; } }
`;
