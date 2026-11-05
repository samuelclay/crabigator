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
.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; border: 0; }
.site-header { display: flex; align-items: center; justify-content: space-between; max-width: 1536px; margin: 0 auto; padding: 28px 48px; gap: 24px; }
.wordmark { display: inline-flex; align-items: center; flex-shrink: 0; text-decoration: none; }
.wordmark .brand-logo { display: block; width: 270px; height: 34px; }
.site-header nav { display: flex; align-items: center; gap: 28px; font-size: 14px; }
.site-header nav a { text-decoration: none; }
.site-header nav a:hover { color: var(--peach); }
.dashboard-link { border: 1px solid var(--line); padding: 10px 17px; border-radius: 6px; }
.hero { max-width: 1296px; margin: 0 auto; padding: 64px 48px 40px; }
.hero h1 { font-size: clamp(48px, 6.2vw, 88px); font-weight: 450; letter-spacing: -.06em; line-height: 1.02; }
.hero-description { margin-top: 22px; font-size: 21px; color: var(--muted); }
.hero-start { display: flex; flex-wrap: wrap; gap: 12px; margin-top: 34px; }
.button-primary, .button-secondary { display: inline-flex; align-items: center; padding: 12px 20px; border-radius: 8px; font-size: 15px; font-weight: 550; text-decoration: none; }
.button-primary { background: var(--peach); color: var(--paper); }
.button-secondary { border: 1px solid var(--line); }
.button-primary:hover { background: #ffcaa5; }.button-secondary:hover { border-color: var(--muted); }
.hero-screens { position: relative; margin-top: 150px; }
.hero-screens > .stream-toggle { position: absolute; left: 0; top: -40px; }
.desktop-frame { container-type: inline-size; position: relative; border: 1px solid #52616b; border-radius: 10px; overflow: hidden; background: #101214; box-shadow: 0 30px 70px #0004; }
.desktop-bar, .browser-bar { display: flex; align-items: center; gap: 16px; padding: 13px 16px; background: #28343e; color: #d1dbe1; font-size: 11px; }
.desktop-bar > span:last-child { flex: 1; margin-right: 52px; text-align: center; color: #a3b0ba; }
.window-dots { display: inline-flex; gap: 5px; }
.window-dots i { width: 7px; height: 7px; border-radius: 50%; background: #d78072; }
.window-dots i:nth-child(2) { background: #d5b477; }
.window-dots i:nth-child(3) { background: #88b99b; }
.term-capture { overflow: hidden; white-space: pre; color: #dedede; font-family: 'JetBrains Mono', monospace; line-height: 1.45; }
.term-capture .line, .term-capture .rule-line { height: 1.45em; overflow: hidden; }
.term-capture .bar-rows { position: relative; width: calc(var(--cols) * 1ch); }
.term-capture .flow-cells { position: absolute; top: 0; right: 0; width: calc(var(--flow-cols) * 1ch); height: 100%; }
.desktop-capture { padding: 20px 18px; }
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
.phone-terminal { display: flex; flex-direction: column; justify-content: end; flex: 1; min-height: 0; overflow: hidden; padding: 4cqw 4cqw 2cqw; background: #0b0f14; color: #d9e1eb; font: 3.3cqw/1.55 'JetBrains Mono', monospace; white-space: pre; }
.phone-terminal .line, .phone-terminal .rule-line { min-height: 1.55em; overflow: hidden; }
.phone-terminal .line { text-overflow: ellipsis; }
/* The iOS session modules: an open drawer, the summary bar, then the composer. */
.app-modules { flex-shrink: 0; background: #1b2129; font-family: -apple-system, BlinkMacSystemFont, 'DM Sans', sans-serif; }
.app-drawer { display: flex; border-top: 1px solid #ffffff17; }
.app-module-rows { flex: 1; min-width: 0; }
.app-module { display: flex; align-items: center; gap: .55em; height: 3.67em; padding: 0 .85em; border-bottom: .5px solid #ffffff12; font-size: var(--module-size); white-space: nowrap; }
.app-module > i { font-style: normal; font-weight: 600; font-size: 1.1em; width: .6em; }
.app-module > b { font-size: .92em; font-weight: 600; }
.app-module > span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; color: #8a8a8a; }
.app-module > small { color: #8a8a8a; font-size: .84em; }
.app-module.recap { color: #00d7ff; }.app-module.git { color: #5fff5f; }.app-module.changes { color: #d7af5f; }
.app-flow { position: relative; flex-shrink: 0; width: var(--flow-width); border-left: 1px solid #ffffff12; background: #0b0f14; }
.app-modules .app-flow svg { position: absolute; inset: 0; width: 100%; height: 100%; }
.app-summary { display: flex; align-items: center; gap: .8em; height: 3.67em; padding-left: .5em; border-top: .5px solid #ffffff17; font: var(--module-size) 'JetBrains Mono', monospace; white-space: nowrap; overflow: hidden; }
.app-summary > span { display: inline-flex; gap: .35em; }
.app-chip { justify-content: center; min-width: 3.4em; padding: .2em .3em; border-radius: .4em; }
.app-summary > .app-done { color: #af87ff; font-size: 1.2em; }
.app-summary > .app-ask { justify-content: center; width: 1.3em; border-radius: .25em; background: #ff8700; color: #101419; font-weight: 700; font-size: 1.2em; }
.app-summary > .app-open { margin-left: auto; padding: 0 1.1em; height: 100%; align-items: center; background: #d7af5f0d; color: #d7af5f; font: 600 1.05em sans-serif; }
.app-summary .c-blue { color: #00afff; }.app-summary .c-mint { color: #5fff5f; }.app-summary .c-orange { color: #d7af5f; }.app-summary .c-deep { color: #ff8700; }.app-summary .c-purple { color: #af87ff; }.app-summary .c-title { color: #5fafff; }
.phone-frame .app-modules { --module-size: 2.85cqw; --flow-width: 36%; }
.phone-composer { display: flex; align-items: center; flex-shrink: 0; gap: 4cqw; padding: 3cqw 4cqw 1cqw; border-top: 1px solid #29313a; color: #9aa4af; }
.phone-composer svg { width: 5cqw; height: 5cqw; }
.phone-composer > span { flex: 1; border: 1px solid #3c4651; padding: 3cqw; border-radius: 5cqw; color: #77818d; font-size: 4cqw; }
.phone-home { display: flex; justify-content: center; flex-shrink: 0; padding: 3cqw 0 2cqw; }
.phone-home span { width: 34%; height: 1.2cqw; border-radius: 3px; background: #edf1f4; }
.hero-phone { position: absolute; width: 21%; right: -3%; top: -118px; }
.providers { display: flex; justify-content: center; flex-wrap: wrap; gap: 24px 40px; max-width: 1296px; margin: 0 auto; padding: 35px 24px 80px; font-size: 14px; color: var(--muted); }
.providers span { display: flex; align-items: center; gap: 8px; }
.providers svg { width: 18px; height: 18px; fill: currentColor; }
.section { max-width: 1296px; margin: 0 auto; padding: 100px 48px; }
.section h2 { font-size: clamp(32px, 3.4vw, 48px); line-height: 1.14; letter-spacing: -.045em; font-weight: 450; }
.section h2 > span { display: block; color: #7f8e99; }
.agent > h2, .surfaces > h2, .ownership > h2, .extras > h2 { text-align: center; }
.agent { padding-top: 70px; padding-bottom: 70px; }
.device-pair.agent-pair { position: relative; display: flex; align-items: flex-end; justify-content: space-between; margin-top: 52px; }
.agent-pair.reverse { flex-direction: row-reverse; }
.desktop-frame.pair-desktop { flex: none; width: 73%; }
.pair-phone { flex: none; width: 24%; }
.agent-pair > .stream-toggle { position: absolute; right: 0; top: -40px; }
.desktop-capture, .pair-capture { font-size: calc((100cqw - 36px) / (var(--cols) * .6)); }
.pair-capture { padding: 16px 18px; }
.pair-phone .phone-app-bar strong { font-size: 4.4cqw; color: #65b1f7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.pair-phone .phone-app-bar div { min-width: 0; }
.tablet-frame { padding: 12px; border: 2px solid #52616b; border-radius: 30px; background: #070b0e; box-shadow: 0 20px 70px #0003; }
.surface-tabs { display: flex; width: max-content; margin: 34px auto 0; padding: 4px; border: 1px solid var(--line); border-radius: 999px; }
.surface-tabs[hidden] { display: none; }
.surface-tabs button { padding: 8px 22px; border: 0; border-radius: 999px; background: transparent; color: var(--muted); font-size: 15px; }
.surface-tabs button[aria-selected="true"] { background: var(--ink); color: var(--paper); }
.surface-panel { margin-top: 34px; }
.surface-panel[hidden] { display: none; }
.surface-claim { margin-bottom: 30px; color: var(--muted); text-align: center; font-size: 17px; }
.surface-claim strong { color: var(--ink); font-weight: 550; }
.surface-claim a { margin-left: 6px; color: var(--peach); }
.notification-preview { display: flex; justify-content: center; padding: 48px 32px; background: #1b2731; border-radius: 12px; }
.notification-phone { width: min(340px, 100%); }
.notification-phone .phone-screen { position: relative; }
.notification-app { display: flex; flex-direction: column; flex: 1; min-height: 0; filter: blur(.45cqw) brightness(.7); }
.notification-app .phone-app-bar strong { font-size: 4.6cqw; color: #65b1f7; }
.notification-card, .notification-menu { position: absolute; left: 3.5cqw; right: 3.5cqw; font-family: -apple-system, BlinkMacSystemFont, 'DM Sans', sans-serif; }
.notification-card { top: 15cqw; padding: 4.5cqw 4.5cqw 4cqw; border-radius: 7cqw; background: #161b22f2; box-shadow: 0 2cqw 8cqw #0008; color: #edf1f4; }
.notification-head { display: flex; align-items: center; gap: 2cqw; color: #a6adb6; font-size: 3.6cqw; font-weight: 500; }
.notification-mark { padding: .3cqw 1.2cqw; border-radius: 1.2cqw; font: 2.8cqw 'JetBrains Mono', monospace; }
.notification-label { margin-top: 3.5cqw; color: #e5b94e; font-size: 3.3cqw; font-weight: 600; letter-spacing: .06em; }
.notification-question { margin: 1.4cqw 0 3cqw; font-size: 4.5cqw; font-weight: 600; }
.notification-option { display: flex; flex-direction: column; gap: .6cqw; margin-top: 1.8cqw; padding: 2.8cqw 3.5cqw; border-radius: 3.2cqw; background: #ffffff0f; font-size: 4.1cqw; }
.notification-option span { color: #9aa1aa; font-size: 3.4cqw; }
.notification-open { margin-top: 3.4cqw; color: #e5b94e; font-size: 4cqw; font-weight: 500; }
.notification-menu { top: 117cqw; right: 40cqw; display: flex; flex-direction: column; padding: 1.5cqw 0; border-radius: 6cqw; background: #e9e9ebe8; box-shadow: 0 2cqw 8cqw #0006; color: #111; font-size: 4.4cqw; }
.notification-menu span { padding: 2.6cqw 6cqw; }
.notification-menu span + span { border-top: .5px solid #0000001a; }
.browser-frame { border: 1px solid #52616b; border-radius: 10px; overflow: hidden; background: #0d1117; box-shadow: 0 20px 70px #0003; }
.browser-bar { gap: 30px; }
.browser-bar > span:last-child { border-radius: 4px; padding: 4px 20px; background: #17222b; color: #a3b0ba; font-size: 11px; }
/* The existing MCP controls share the home page palette. */
.mcp-section { border: 0; overflow: visible; }
.mcp-section::before { content: none; }
.mcp-intro { grid-template-columns: minmax(0, 1fr) minmax(0, 420px); gap: 80px; }
.mcp-intro-copy { text-align: left; }
.mcp-intro-copy .section-title { margin-bottom: 28px; }
.mcp-connect-row { background: transparent; border-color: var(--line); border-radius: 5px; }
.mcp-connect-label { background: transparent; color: var(--muted); font: 12px 'DM Sans', sans-serif; text-transform: none; letter-spacing: 0; }
.mcp-connect-value { color: var(--ink); font-size: 11px; }
.mcp-connect-copy { background: transparent; color: var(--ink); }
.mcp-demo { background: #17222b; color: #dbe7ed; border: 1px solid var(--line); box-shadow: none; }
.mcp-demo-bar { background: #25333e; border: 0; }
.mcp-demo-title { color: #b5c6d2; }
.mcp-demo-call { color: #a9d7e9; }
.mcp-demo-json { color: #dbe7ed; }
.btn-primary { display: inline-block; color: var(--peach); font-size: 14px; text-decoration: underline; }
.claims { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 40px; margin-top: 56px; }
.claims strong, .card strong { display: block; font-weight: 550; font-size: 17px; }
.claims p, .card p { margin-top: 8px; color: var(--muted); font-size: 15px; }
.text-link { display: table; margin: 40px auto 0; color: var(--peach); font-size: 15px; }
.cards { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 28px; margin-top: 56px; }
.card-shot { container-type: inline-size; display: flex; flex-direction: column; justify-content: center; aspect-ratio: 16 / 7; margin-bottom: 18px; padding: 18px; overflow: hidden; border: 1px solid var(--line); border-radius: 10px; background: #0b0f14; }
.card-term { font-size: calc((100cqw - 2px) / (var(--cols) * .6)); }
.card-scenes { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; }
.scene-frames { position: relative; display: block; aspect-ratio: 120 / 87; }
.scene-frames > svg { position: absolute; inset: 0; width: 100%; height: 100%; }
.card-mcp { font: 13px/1.5 'JetBrains Mono', monospace; color: #dbe7ed; }
.card-mcp code { color: #a9d7e9; font-size: 13px; }
.card-mcp pre { margin: 8px 0 0; white-space: pre-wrap; }
.card-mcp .mcp-jk { color: #a9d7e9; }.card-mcp .mcp-js { color: #a4d6b6; }.card-mcp .mcp-jb { color: #ffb98a; }
.card p a { color: var(--peach); }
.install { display: grid; grid-template-columns: 1fr 1.1fr; gap: 80px; border-top: 1px solid var(--line); }
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
.email-form { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; }
.email-form input { background: transparent; border: 1px solid var(--line); border-radius: 5px; padding: 12px; color: var(--ink); min-width: 220px; font-size: 14px; }
.email-form button { background: var(--peach); color: var(--paper); border: 1px solid var(--peach); border-radius: 5px; padding: 12px 16px; font-size: 14px; }
.email-form p { width: 100%; color: var(--muted); font-size: 14px; }
.footer { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 28px; padding: 36px 48px; margin: 0 auto; border-top: 1px solid var(--line); max-width: 1536px; }
.footer > p { font-size: 12px; color: var(--muted); }
.footer-links { display: flex; gap: 24px; font-size: 12px; }
@media (max-width: 1100px) {
    .site-header { padding: 24px 28px; }
    .hero { padding: 45px 28px 20px; }
    .hero-phone { right: 0; }
    .section { padding: 80px 28px; }
    .providers { padding-bottom: 40px; }
    .mcp-intro { gap: 40px; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
    .install { gap: 50px; }
    .claims { grid-template-columns: repeat(2, minmax(0, 1fr)); }
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
    .hero { padding: 36px 24px 18px; }
    .hero h1 { font-size: clamp(42px, 10.8vw, 66px); }
    .hero-description { font-size: 17px; }
    .hero-start { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 28px; }
    .button-primary, .button-secondary { justify-content: center; padding: 12px 8px; font-size: 15px; }
    .hero-screens { margin-top: 40px; }
    .desktop-frame { border-radius: 7px; }
    .desktop-bar { padding: 9px; gap: 10px; font-size: 8px; }
    .desktop-bar > span:last-child { display: none; }
    .window-dots { gap: 3px; }
    .window-dots i { width: 5px; height: 5px; }
    .desktop-capture, .pair-capture { font-size: calc((100cqw - 18px) / (var(--cols) * .6)); }
    .desktop-capture { padding: 14px 9px; }
    .pair-capture { padding: 9px; }
    .device-pair.agent-pair, .agent-pair.reverse { display: block; margin-top: 36px; }
    .desktop-frame.pair-desktop { width: 100%; }
    .agent-pair > .stream-toggle, .hero-screens > .stream-toggle { top: auto; bottom: 0; left: auto; right: 0; }
    .hero-phone, .pair-phone { position: relative; top: auto; right: auto; width: 44%; margin: -30% 0 0 4%; z-index: 1; }
    .phone-frame { padding: 4px; border-radius: 18px; }
    .phone-screen { border-radius: 12px; }
    .providers { padding: 12px 24px 40px; gap: 18px 25px; font-size: 12px; }
    .providers svg { width: 16px; height: 16px; }
    .section { padding: 60px 24px; }
    .section h2 { font-size: 32px; }
    .tablet-frame { padding: 5px; border-radius: 12px; }
    .install { grid-template-columns: 1fr; gap: 32px; }
    .claims, .cards { grid-template-columns: minmax(0, 1fr); gap: 28px; }
    .card-shot { aspect-ratio: auto; min-height: 120px; }
    .card-term { font-size: 2.78cqw; }
    .notification-preview { padding: 24px; }
    .surface-tabs button { padding: 7px 16px; }
    .surface-claim { font-size: 15px; }
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
