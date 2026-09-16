// Landing page HTML served at /
import { landingCss } from './landing/css';
import { mcpCss } from './landing/mcp-css';
import { landingJs } from './landing/js';
import { palmWebglJs } from './landing/palm-webgl';
import { analyticsJs } from './landing/analytics';
import type { RuntimeConfig } from './config';
import { metaPixelHtml, usePublicOrigin } from './html-render';
import { mcpLandingSectionHtml } from './mcp/docs';
import {
    iconCrabigator,
    iconCrabigatorEncoded,
    iconCrabigatorMono,
    iconCrabigatorPhoneLogo,
    iconCheck,
    iconPhone,
    iconPhoneOutline,
    iconPhoneBento,
    iconPlay,
    iconChart,
    iconChartBento,
    iconFolder,
    iconFolderBento,
    iconChanges,
    iconCloud,
    iconCloudFilled,
    iconCube,
    iconDiamond,
    iconClaude,
    iconCodex,
    iconOpencode,
    iconGrok,
    iconSemanticDiff,
    iconLink,
    iconSearch,
    iconLines,
    iconMouse,
    iconBolt,
    iconMail,
    iconBell,
    iconClock,
    iconArrowRight,
    iconCircle,
    iconCompass,
    iconGator,
    iconGithubSmall,
    iconGithubLogo,
    iconCopy,
    iconLicense,
    iconStar,
    iconFork,
    iconIssues,
    iconPullRequest,
    iconBook,
    iconTag,
    iconChat,
    iconLock,
    iconKey,
    iconTerminal,
    iconGhost,
    iconDesktop,
    iconCloudEdge,
} from './landing/icons';

function brandName(
    kind: 'claude' | 'codex' | 'opencode' | 'grok',
    label: string,
    after = '',
    before = '',
): string {
    const mark = { claude: iconClaude, codex: iconCodex, opencode: iconOpencode, grok: iconGrok }[kind];
    return `<span class="brand-token">${before}<span class="brand brand-${kind}">${mark}${label}</span>${after}</span>`;
}

function sessionChip(glyph: string, bg: string, fg: string): string {
    return `<span class="session-chip" style="background:${bg};color:${fg}">${glyph}</span>`;
}

export const landingHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator - control Claude Code, Codex, opencode, and Grok from your phone</title>
    <meta name="description" content="Answer permissions, approve plans, and respond to questions from your phone. Watch Claude Code, Codex, opencode, and Grok sessions as they run.">
    <meta property="og:title" content="Crabigator - Control Claude Code, Codex, opencode, and Grok from your phone">
    <meta property="og:description" content="Answer permissions, approve plans, and respond to questions from your phone. Watch Claude Code, Codex, opencode, and Grok sessions as they run.">
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-landing.png">
    <meta property="og:url" content="https://drinkcrabigator.com/">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator - Control Claude Code, Codex, opencode, and Grok from your phone">
    <meta name="twitter:description" content="Answer permissions, approve plans, and respond to questions from your phone.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-landing.png">
    <link rel="icon" href="data:image/svg+xml,${iconCrabigatorEncoded}">
    <style>${landingCss}${mcpCss}</style>
</head>
<body>
    <!-- Navigation -->
    <nav class="nav">
        <a href="/" class="nav-logo">
            ${iconCrabigator}
            Crabigator
        </a>
        <div class="nav-links">
            <a href="#features" class="nav-link">Features</a>
            <a href="#pr-board" class="nav-link">PRs</a>
            <a href="#mcp" class="nav-link">MCP</a>
            <a href="#security" class="nav-link">Security</a>
            <a href="#install" class="nav-link">Install</a>
            <a href="/dashboard" class="nav-btn" data-track="dashboard_nav" data-label="nav">Open Dashboard</a>
            <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="nav-github">
                ${iconGithubSmall}
            </a>
        </div>
    </nav>

    <!-- Hero Section -->
    <section class="hero">
        <div class="hero-content">
            <div class="hero-text">
                <h1 class="hero-headline">
                    Control ${brandName('claude', 'Claude Code', ',')} ${brandName('codex', 'Codex', ',')} ${brandName('opencode', 'opencode')} ${brandName('grok', 'Grok', '', '&amp; ')} from your phone
                </h1>
                <p class="hero-subheadline">
                    Answer permissions, approve plans, and questions without sitting at the computer.
                    The agent still runs on your Mac, Windows, or Linux machine.
                </p>
                <div class="hero-install">
                    <div class="hero-install-command">
                        <span class="hero-install-prompt">$</span>
                        <span class="hero-install-text">npm install -g crabigator</span>
                        <button class="hero-copy-btn" id="hero-copy-btn" onclick="copyHeroInstall()">
                            ${iconCopy}
                        </button>
                    </div>
                    <div class="hero-install-steps">
                        <div class="hero-step"><span class="hero-step-num">1</span> Install via npm</div>
                        <div class="hero-step"><span class="hero-step-num">2</span> Run <code>crabigator</code> instead of <code>claude</code></div>
                        <div class="hero-step"><span class="hero-step-num">3</span> Click pairing link in terminal</div>
                        <div class="hero-step"><span class="hero-step-num">4</span> <a href="/dashboard" class="hero-step-link">Open dashboard</a> on your phone</div>
                    </div>
                </div>
            </div>

            <div class="hero-devices">
                <div class="hero-terminal">
                    <div class="terminal-window">
                    <div class="terminal-bar">
                        <div class="terminal-dot red"></div>
                        <div class="terminal-dot yellow"></div>
                        <div class="terminal-dot green"></div>
                        <span class="terminal-title">crabigator  ~/projects/api</span>
                    </div>
                    <div class="terminal-content">
                        <div class="terminal-line">
                            <span class="terminal-prompt">${iconPlay}</span>
                            <span class="terminal-cmd"> crabigator</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">Starting Claude Code session...</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-success">${iconCheck} Connected to drinkcrabigator.com</span>
                        </div>
                        <div class="terminal-line" style="margin-top: 16px;">
                            <span class="terminal-output">╭─ Claude is thinking </span>
                            <span class="terminal-thinking"><span></span><span></span><span></span></span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">│ Analyzing codebase structure</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">│ Reading src/app.rs</span>
                        </div>

                        <div class="terminal-widgets">
                            <div class="widget-card">
                                <div class="widget-header">${iconChart} Session Stats <span class="widget-state thinking">thinking</span></div>
                                <div class="widget-row">
                                    <span class="widget-label">◉ Session</span>
                                    <span class="widget-value blue">12m 34s</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">⟩ Prompts 7</span>
                                    <span class="widget-value dim">2m ago</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">⚒ Tools</span>
                                    <span class="widget-sparkline">▁▂▄▅▇█▆▄▂▃▅▄</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">⊜ Compactions 1</span>
                                    <span class="widget-value dim">5m ago</span>
                                </div>
                            </div>
                            <div class="widget-card">
                                <div class="widget-header">
                                    ${iconFolder}
                                    <span class="widget-branch">main</span>
                                    <span class="widget-files">3 files</span>
                                </div>
                                <div class="git-files">
                                    <div class="git-file">
                                        <span class="git-status modified">●</span>
                                        <span class="git-path">src/app.rs</span>
                                        <span class="git-diff"><span class="del-num">−42</span><span class="bars"><span class="bar-del">▓▓</span><span class="bar-add">███</span></span><span class="add-num">+87</span></span>
                                    </div>
                                    <div class="git-file">
                                        <span class="git-status modified">●</span>
                                        <span class="git-path">src/ui.rs</span>
                                        <span class="git-diff"><span class="del-num">−12</span><span class="bars"><span class="bar-del">▓▓</span><span class="bar-add">██</span></span><span class="add-num">+34</span></span>
                                    </div>
                                    <div class="git-file">
                                        <span class="git-status added">+</span>
                                        <span class="git-path">src/update.rs</span>
                                        <span class="git-diff"><span class="del-num"></span><span class="bars"><span class="bar-del"></span><span class="bar-add">██</span></span><span class="add-num">+21</span></span>
                                    </div>
                                </div>
                            </div>
                            <div class="widget-card changes">
                                <div class="widget-header">
                                    ${iconChanges}
                                    <span class="widget-lang">TypeScript</span>
                                    <span class="widget-count">4 changes</span>
                                </div>
                                <div class="changes-list">
                                    <div class="change-item">
                                        <span class="change-mod">~</span><span class="change-icon fn">ƒ</span>
                                        <span class="change-name">handleViewerSocket</span>
                                        <span class="change-stats"><span class="del">−8</span> <span class="add">+24</span></span>
                                    </div>
                                    <div class="change-item">
                                        <span class="change-mod">+</span><span class="change-icon fn">ƒ</span>
                                        <span class="change-name">parseEvent</span>
                                        <span class="change-stats"><span class="add">+42</span></span>
                                    </div>
                                    <div class="change-item">
                                        <span class="change-mod">~</span><span class="change-icon cls">◆</span>
                                        <span class="change-name">SessionDO</span>
                                        <span class="change-stats"><span class="del">−3</span> <span class="add">+18</span></span>
                                    </div>
                                </div>
                                <div class="changes-lang-header">
                                    <span class="widget-lang rust">Rust</span>
                                    <span class="widget-count">2 changes</span>
                                </div>
                                <div class="changes-list">
                                    <div class="change-item">
                                        <span class="change-mod">~</span><span class="change-icon fn">ƒ</span>
                                        <span class="change-name">update_widgets</span>
                                        <span class="change-stats"><span class="del">−15</span> <span class="add">+31</span></span>
                                    </div>
                                    <div class="change-item">
                                        <span class="change-mod">+</span><span class="change-icon struct">◇</span>
                                        <span class="change-name">UpdateState</span>
                                        <span class="change-stats"><span class="add">+26</span></span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                </div>
                <div class="hero-phone">
                    <div class="phone-notch"></div>
                    <div class="phone-screen">
                        <div class="phone-header">
                            ${iconCrabigatorPhoneLogo}
                            <span class="phone-title">~/projects/api</span>
                            <span class="phone-state thinking">thinking</span>
                        </div>
                        <div class="phone-terminal">
                            <div class="pt-line"><span class="pt-prompt">❯</span> crabigator</div>
                            <div class="pt-line pt-dim">Starting Claude Code session...</div>
                            <div class="pt-line pt-success">✓ Streaming</div>
                            <div class="pt-line pt-dim" style="margin-top: 8px;">╭─ Claude is thinking <span class="pt-thinking"><span></span><span></span><span></span></span></div>
                            <div class="pt-line pt-dim">│ Analyzing codebase structure</div>
                            <div class="pt-line pt-dim">│ Reading src/app.rs</div>
                        </div>
                        <div class="phone-widget">
                            <div class="phone-widget-header">
                                <span class="pw-icon">◉</span> Session
                                <span class="pw-value">12m 34s</span>
                            </div>
                            <div class="phone-widget-row">
                                <span>⟩ Prompts 7</span>
                                <span class="pw-dim">2m ago</span>
                            </div>
                            <div class="phone-widget-row">
                                <span>⚒ Tools</span>
                                <span class="pw-sparkline">▁▂▄▃▅▇█▆▄▂▁▃▅▄</span>
                            </div>
                        </div>
                        <div class="phone-widget">
                            <div class="phone-widget-header">
                                <span class="pw-branch">main</span>
                                <span class="pw-files">3 files</span>
                            </div>
                            <div class="phone-git-file">
                                <span class="pf-status">●</span>
                                <span class="pf-path">src/app.rs</span>
                                <span class="pf-diff"><span class="del">−42</span><span class="add">+87</span></span>
                            </div>
                            <div class="phone-git-file">
                                <span class="pf-status">●</span>
                                <span class="pf-path">src/ui.rs</span>
                                <span class="pf-diff"><span class="del">−12</span><span class="add">+34</span></span>
                            </div>
                        </div>
                        <div class="phone-widget">
                            <div class="phone-widget-header">
                                <span class="pw-lang">TypeScript</span>
                                <span class="pw-count">4 changes</span>
                            </div>
                            <div class="phone-change">
                                <span class="pc-mod">~</span><span class="pc-icon">ƒ</span>
                                <span class="pc-name">handleViewerSocket</span>
                            </div>
                            <div class="phone-change">
                                <span class="pc-mod">+</span><span class="pc-icon">ƒ</span>
                                <span class="pc-name">parseEvent</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>

    <!-- Stay in the Loop CTA -->
    <section class="section cta-section">
        <div class="cta-content">
            <div class="cta-card">
                <div class="cta-header">
                    ${iconMail}
                    <span class="cta-label">Email updates</span>
                </div>
                <p class="cta-text">News about new features and the mobile apps.</p>
                <form class="cta-form" id="hero-email-form">
                    <input type="email" class="cta-input" placeholder="you@example.com" required>
                    <button type="submit" class="cta-btn">Subscribe</button>
                </form>
                <div class="cta-success" id="hero-email-success">
                    ${iconCheck} You're on the list!
                </div>
            </div>
        </div>
    </section>

    <!-- Device Showcase -->
    <section class="section showcase">
        <div class="showcase-content">
            <div class="showcase-text">
                <h2 class="section-title">Check in from phone or desktop</h2>
                <p class="section-subtitle">
                    No Tmux. No Tailscale. No Termius. Built for ${brandName('claude', 'Claude Code', ',')} ${brandName('codex', 'Codex', ',')} ${brandName('opencode', 'opencode', ',')} and ${brandName('grok', 'Grok')}.
                    Pair a phone in seconds. Watch progress and answer prompts without sitting at the computer.
                </p>
                <div class="showcase-status">
                    <div class="status-pills">
                        <span class="status-pill thinking">Thinking</span>
                        <span class="status-pill permission">Permission</span>
                        <span class="status-pill question">Question</span>
                        <span class="status-pill complete">Complete</span>
                    </div>
                    <p class="status-hint">State updates as each session changes</p>
                </div>
            </div>
            <div class="showcase-devices">
                <div class="device-phone">
                    <div class="phone-screen">
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/api</span>
                                <span class="session-state thinking">thinking</span>
                            </div>
                            <div class="session-preview">
                                Reading config.ts...<br>
                                Analyzing function signatures
                            </div>
                        </div>
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/web</span>
                                <span class="session-state permission">permission</span>
                            </div>
                            <div class="session-preview">
                                Allow: npm install<br>
                                [Allow] [Deny]
                            </div>
                        </div>
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/cli</span>
                                <span class="session-state complete">complete</span>
                            </div>
                            <div class="session-preview">
                                ${iconCheck} Refactored auth module<br>
                                ${iconCheck} Added unit tests
                            </div>
                        </div>
                    </div>
                </div>
                <div class="device-browser">
                    <div class="browser-bar">
                        <div class="browser-dots">
                            <div class="browser-dot"></div>
                            <div class="browser-dot"></div>
                            <div class="browser-dot"></div>
                        </div>
                        <div class="browser-url">drinkcrabigator.com/dashboard</div>
                    </div>
                    <div class="browser-content">
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/api</span>
                                <span class="session-state thinking">thinking</span>
                            </div>
                            <div class="session-preview">
                                ╭─ Claude is thinking...<br>
                                │ Reading config.ts<br>
                                │ Analyzing function signatures
                            </div>
                        </div>
                        <div class="session-card">
                            <div class="session-header">
                                <span class="session-path">~/projects/web</span>
                                <span class="session-state permission">permission</span>
                            </div>
                            <div class="session-preview">
                                Claude wants to: npm install lodash<br>
                                <span style="color: #4ade80;">[Allow]</span> <span style="color: #f87171;">[Deny]</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>

    <!-- Interactive Section -->
    <section class="section interactive">
        <div class="interactive-content">
            <div class="interactive-visual">
                <div class="prompt-mockup">
                    <div class="prompt-mockup-header">
                        <span class="prompt-mockup-state">Permission Request</span>
                        <span class="prompt-mockup-path">~/projects/api</span>
                    </div>
                    <div class="prompt-mockup-question">
                        Claude wants to run: <code>pnpm install lodash</code>
                    </div>
                    <div class="prompt-mockup-options">
                        <div class="prompt-mockup-row">
                            <div class="prompt-mockup-option selected">
                                <span class="option-num">1.</span> Yes, allow this
                            </div>
                            <input type="text" class="prompt-mockup-input" placeholder="+ instructions" value="use npm instead">
                        </div>
                        <div class="prompt-mockup-row">
                            <div class="prompt-mockup-option no-input">
                                <span class="option-num">2.</span> Yes, don't ask again
                            </div>
                        </div>
                        <div class="prompt-mockup-row">
                            <div class="prompt-mockup-option">
                                <span class="option-num">3.</span> No, deny this
                            </div>
                            <input type="text" class="prompt-mockup-input" placeholder="+ instructions">
                        </div>
                    </div>
                </div>
            </div>
            <div class="interactive-text">
                <h2 class="section-title">Answer from your phone</h2>
                <p class="section-subtitle">
                    Answer permissions, review plans, and questions from your phone or a browser.
                    You can add instructions before you approve.
                </p>
                <ul class="interactive-features">
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Permission prompts</strong> with one-tap approve or deny</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Add instructions</strong> when you approve</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Answer questions</strong> when the agent asks</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Review plans</strong> before it starts coding</span>
                    </li>
                </ul>
            </div>
        </div>
    </section>

    <!-- PR board -->
    <section class="section pr-board-section" id="pr-board">
        <div class="pr-board-content">
            <div class="pr-board-text">
                <h2 class="section-title">PR Dashboard</h2>
                <p class="section-subtitle">
                    Run <code>crabigator pr</code> to see every live session in one place.
                    Pull requests sit on top. Each session has its own glyph.
                </p>
                <ul class="interactive-features">
                    <li>
                        <span class="feature-icon">${iconPullRequest}</span>
                        <span class="feature-text"><strong>One block per PR</strong>, with its sessions listed underneath</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Press p</strong> to flip to session view</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconSearch}</span>
                        <span class="feature-text"><strong>Search</strong> greps every live transcript</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconPlay}</span>
                        <span class="feature-text"><strong>Enter</strong> opens a live look at a session</span>
                    </li>
                </ul>
                <div class="pr-board-cmd">
                    <span class="hero-install-prompt">$</span>
                    <span class="hero-install-text">crabigator pr</span>
                </div>
            </div>
            <div class="pr-board-visual">
                <div class="terminal-window pr-board-window">
                    <div class="terminal-bar">
                        <div class="terminal-dot red"></div>
                        <div class="terminal-dot yellow"></div>
                        <div class="terminal-dot green"></div>
                        <span class="terminal-title">crabigator pr</span>
                    </div>
                    <div class="prb-mock">
                        <div class="prb-mock-head">
                            <span class="prb-mock-hdr">PR board</span>
                            <span class="prb-mock-counts">3 PRs · 5 sessions</span>
                            <span class="prb-mock-ctl on">live</span>
                            <span class="prb-mock-ctl">recaps</span>
                            <span class="prb-mock-ctl">24h</span>
                            <span class="prb-mock-search">/ search</span>
                        </div>
                        <div class="prb-mock-bucket hour">Last hour</div>
                        <div class="prb-mock-repo">crabigator</div>
                        <div class="prb-mock-block selected">
                            <div class="prb-mock-pr">
                                <span class="prb-mock-star">★</span>
                                <span class="prb-mock-ident"><span class="prb-mock-num">#412</span> Stream recaps to the dashboard</span>
                                <span class="prb-mock-age hour">2m</span>
                                <span class="prb-mock-diff"><span class="add">+18</span> <span class="del">−4</span></span>
                                <span class="prb-mock-files">3</span>
                                <span class="prb-mock-ci ok">CI ✓</span>
                            </div>
                            <div class="prb-mock-sess peeking">
                                <span class="prb-mock-diamond">◆</span>
                                <span class="prb-mock-sess-title">${sessionChip('▀▄▀', '#7A1024', '#FFD2A8')} Rewrite the viewer handshake</span>
                                <span class="prb-mock-state thinking">thinking</span>
                            </div>
                            <div class="prb-mock-sess">
                                <span class="prb-mock-diamond">◆</span>
                                <span class="prb-mock-sess-title">${sessionChip('◕‿◕', '#001848', '#5EF0FF')} Pairing from a second laptop</span>
                                <span class="prb-mock-state permission">permission</span>
                            </div>
                        </div>
                        <div class="prb-mock-block">
                            <div class="prb-mock-pr">
                                <span class="prb-mock-star">★</span>
                                <span class="prb-mock-ident"><span class="prb-mock-num">#408</span> Drop the eyebrows from the homepage</span>
                                <span class="prb-mock-age hour">18m</span>
                                <span class="prb-mock-diff"><span class="add">+61</span> <span class="del">−61</span></span>
                                <span class="prb-mock-files">2</span>
                                <span class="prb-mock-ci merged">merged</span>
                            </div>
                            <div class="prb-mock-sess">
                                <span class="prb-mock-diamond">◆</span>
                                <span class="prb-mock-sess-title">${sessionChip('╭◈╮', '#2A1760', '#E4D4FF')} Write the homepage in plain language</span>
                                <span class="prb-mock-state complete">complete</span>
                            </div>
                        </div>
                        <div class="prb-mock-bucket later">1–3 hours</div>
                        <div class="prb-mock-repo">api</div>
                        <div class="prb-mock-block">
                            <div class="prb-mock-pr">
                                <span class="prb-mock-star">★</span>
                                <span class="prb-mock-ident"><span class="prb-mock-num">#2469</span> Wire wait_for_attention</span>
                                <span class="prb-mock-age later">2h</span>
                                <span class="prb-mock-diff"><span class="add">+120</span> <span class="del">−12</span></span>
                                <span class="prb-mock-files">8</span>
                                <span class="prb-mock-ci ok">CI ✓</span>
                            </div>
                            <div class="prb-mock-sess">
                                <span class="prb-mock-diamond">◆</span>
                                <span class="prb-mock-sess-title">${sessionChip('≈△≈', '#C45C12', '#1A1208')} Add the MCP wait tool</span>
                                <span class="prb-mock-state thinking">thinking</span>
                            </div>
                            <div class="prb-mock-sess">
                                <span class="prb-mock-diamond">◆</span>
                                <span class="prb-mock-sess-title">${sessionChip('⌈✦⌉', '#082838', '#F0C040')} Fix the pairing timeout</span>
                                <span class="prb-mock-state complete">complete</span>
                            </div>
                        </div>
                        <div class="prb-mock-peek">
                            <div class="prb-mock-peek-head">
                                ${sessionChip('▀▄▀', '#7A1024', '#FFD2A8')}
                                <span class="prb-mock-peek-title">Rewrite the viewer handshake</span>
                                <span class="prb-mock-peek-path">~/projects/crabigator</span>
                            </div>
                            <div class="prb-mock-peek-body">
                                <div>╭─ Claude is thinking <span class="terminal-thinking"><span></span><span></span><span></span></span></div>
                                <div>│ Reading src/prs_board.rs</div>
                                <div>│ Drawing session glyphs on each row</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>

    <!-- Features Bento Grid -->
    <section class="section features-section" id="features">
        <div class="section-header">
            <h2 class="section-title">What Crabigator adds</h2>
        </div>
        <div class="bento-grid">
            <!-- Row 1-2: Session Stats (tall) + Git Changes (tall) + Cloud Dashboard + Mobile -->
            <div class="bento-card tall">
                <div class="bento-icon">${iconChartBento}</div>
                <h3 class="bento-title">Session stats</h3>
                <p class="bento-desc">Prompts, completions, tool calls, and how long the session has been running.</p>
                <div class="bento-visual">
                    <div class="mini-widget">
                        <div class="widget-header-mini">${iconChart} Session Stats</div>
                        <div class="widget-row-mini">
                            <span>Session</span>
                            <span class="val">34m</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Thinking</span>
                            <span class="val">16m</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Prompts</span>
                            <span class="val">17</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Completions</span>
                            <span class="val">16</span>
                        </div>
                        <div class="widget-row-mini">
                            <span>Tools</span>
                            <span class="sparkline">
                                <span class="spark" style="height: 4px;"></span>
                                <span class="spark" style="height: 8px;"></span>
                                <span class="spark" style="height: 6px;"></span>
                                <span class="spark" style="height: 12px;"></span>
                                <span class="spark" style="height: 10px;"></span>
                                <span class="spark" style="height: 14px;"></span>
                                <span class="spark" style="height: 8px;"></span>
                                <span class="spark" style="height: 6px;"></span>
                                <span class="spark" style="height: 10px;"></span>
                                <span class="spark" style="height: 12px;"></span>
                                <span class="spark" style="height: 8px;"></span>
                                <span class="spark" style="height: 4px;"></span>
                            </span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="bento-card tall">
                <div class="bento-icon">${iconFolderBento}</div>
                <h3 class="bento-title">Git changes</h3>
                <p class="bento-desc">Every file the agent touches. Additions in green, deletions in red.</p>
                <div class="bento-visual">
                    <div class="mini-widget">
                        <div class="widget-header-mini">${iconFolder} 6 files</div>
                        <div class="file-row">
                            <span class="file-name">● css.ts</span>
                            <span class="diff-bar"><span class="del" style="width: 8px;"></span><span class="add" style="width: 45px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">? update.rs</span>
                            <span class="diff-bar"><span class="add" style="width: 40px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">● landing.ts</span>
                            <span class="diff-bar"><span class="del" style="width: 12px;"></span><span class="add" style="width: 30px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">● js.ts</span>
                            <span class="diff-bar"><span class="del" style="width: 10px;"></span><span class="add" style="width: 18px;"></span></span>
                        </div>
                        <div class="file-row">
                            <span class="file-name">● main.rs</span>
                            <span class="diff-bar"><span class="del" style="width: 3px;"></span><span class="add" style="width: 9px;"></span></span>
                        </div>
                    </div>
                </div>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconCloud}</div>
                <h3 class="bento-title">Cloud Dashboard</h3>
                <p class="bento-desc">View sessions and respond to prompts from any browser. No VPN needed.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconPhoneBento}</div>
                <h3 class="bento-title">From your phone</h3>
                <p class="bento-desc">Approve permissions and answer questions from your phone.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon anthropic">${iconCube}</div>
                <h3 class="bento-title">${brandName('claude', 'Claude Code')}</h3>
                <p class="bento-desc">Hooks into Claude Code for stats, permissions, and session state.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon openai">${iconDiamond}</div>
                <h3 class="bento-title">${brandName('codex', 'Codex', ',')} ${brandName('opencode', 'opencode')} ${brandName('grok', 'Grok', '', '&amp; ')}</h3>
                <p class="bento-desc">Also works with Codex, opencode, and Grok Build.</p>
            </div>

            <!-- Row 3-4: Semantic Diff (wide + tall) + File Links (tall) -->
            <div class="bento-card wide tall">
                <div class="bento-icon">${iconSemanticDiff}</div>
                <h3 class="bento-title">Diffs by function</h3>
                <p class="bento-desc">Groups changes by language and names the function or method that changed.</p>
                <div class="bento-visual">
                    <div class="mini-widget semantic">
                        <div class="lang-group">
                            <div class="lang-header"><span class="lang-tag js">JavaScript</span> 5 changes</div>
                            <div class="func-row"><span class="func-icon">~ƒ</span> handleEmailSignup <span class="func-diff"><span class="red">-20</span> <span class="green">+33</span></span></div>
                            <div class="func-row"><span class="func-icon">~ƒ</span> copyInstallCommand <span class="func-diff"><span class="red">-8</span> <span class="green">+12</span></span></div>
                            <div class="func-row"><span class="func-icon">+ƒ</span> initDashboard <span class="func-diff"><span class="green">+45</span></span></div>
                        </div>
                        <div class="lang-group">
                            <div class="lang-header"><span class="lang-tag rs">Rust</span> 4 changes</div>
                            <div class="func-row"><span class="func-icon">~ƒ</span> main <span class="func-diff"><span class="red">-2</span> <span class="green">+5</span></span></div>
                            <div class="func-row"><span class="func-icon">+ƒ</span> default_true <span class="func-diff"><span class="green">+2</span></span></div>
                            <div class="func-row"><span class="func-icon">~◇</span> Default for Config <span class="func-diff"><span class="red">-1</span> <span class="green">+3</span></span></div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="bento-card tall">
                <div class="bento-icon">${iconLink}</div>
                <h3 class="bento-title">File links</h3>
                <p class="bento-desc">File paths are links. Click one to open it in VS Code, Cursor, or Zed.</p>
                <div class="bento-visual">
                    <div class="mini-widget file-links">
                        <div class="link-row"><span class="link-path">src/app.rs</span><span class="link-line">:142</span></div>
                        <div class="link-row"><span class="link-path">src/config.ts</span><span class="link-line">:38</span></div>
                        <div class="link-row"><span class="link-path">tests/fixture.rs</span><span class="link-line">:256</span></div>
                        <div class="link-row"><span class="link-path">src/main.rs</span><span class="link-line">:89</span></div>
                    </div>
                </div>
            </div>

            <!-- Row 5: CLI + Scrollback + Mouse + ? -->
            <div class="bento-card small">
                <div class="bento-icon">${iconSearch}</div>
                <h3 class="bento-title">Inspect</h3>
                <p class="bento-desc">Use <code>crabigator inspect</code> to list running sessions from a script.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconLines}</div>
                <h3 class="bento-title">Scrollback</h3>
                <p class="bento-desc">Uses your terminal's own scrollback, not a tmux pane.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconMouse}</div>
                <h3 class="bento-title">Mouse selection</h3>
                <p class="bento-desc">Select and copy text as you would in any terminal. No capture mode.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconBolt}</div>
                <h3 class="bento-title">Pairing</h3>
                <p class="bento-desc">Scan a QR code or type a short code.</p>
            </div>
        </div>
    </section>

    ${mcpLandingSectionHtml()}

    <!-- Security Section -->
    <section class="section security" id="security">
        <div class="section-header">
            <h2 class="security-headline">Text-only streaming. Nothing stored.</h2>
            <p class="security-subtitle">
                Only terminal output is sent. No file access, no credentials, no system resources.
                Nothing is kept after your session ends.
            </p>
        </div>

        <!-- Data Flow Diagram -->
        <div class="security-flow">
            <div class="flow-node">
                <div class="flow-node-icon">
                    ${iconDesktop}
                </div>
                <span class="flow-node-title">Desktop</span>
                <span class="flow-node-desc">Streams output</span>
            </div>

            <div class="flow-connector">
                <span class="flow-label">TLS 1.3+</span>
                <div class="flow-line"></div>
                <div class="flow-lock">${iconLock}</div>
            </div>

            <div class="flow-node">
                <div class="flow-node-icon cloud">
                    ${iconCloudEdge}
                </div>
                <span class="flow-node-title">Cloudflare Edge</span>
                <span class="flow-node-desc">Memory only</span>
                <span class="flow-node-badge">Ephemeral</span>
            </div>

            <div class="flow-connector">
                <span class="flow-label">TLS 1.3+</span>
                <div class="flow-line"></div>
                <div class="flow-lock">${iconLock}</div>
            </div>

            <div class="flow-node">
                <div class="flow-node-icon">
                    ${iconPhone}
                </div>
                <span class="flow-node-title">Phone / Web</span>
                <span class="flow-node-desc">Views & responds</span>
            </div>
        </div>

        <!-- Security Features Grid -->
        <div class="security-features">
            <div class="security-feature">
                <div class="security-feature-icon">
                    ${iconLock}
                </div>
                <div class="security-feature-content">
                    <div class="security-feature-title">TLS 1.3+ Encryption</div>
                    <div class="security-feature-desc">Traffic is encrypted with TLS 1.3 on Cloudflare's network.</div>
                </div>
            </div>

            <div class="security-feature">
                <div class="security-feature-icon">
                    ${iconTerminal}
                </div>
                <div class="security-feature-content">
                    <div class="security-feature-title">Text-Only Streaming</div>
                    <div class="security-feature-desc">Only terminal output is sent. No filesystem access, no credentials.</div>
                </div>
            </div>

            <div class="security-feature">
                <div class="security-feature-icon">
                    ${iconGhost}
                </div>
                <div class="security-feature-content">
                    <div class="security-feature-title">Ephemeral Data</div>
                    <div class="security-feature-desc">Session data lives in memory only. When you disconnect, it's gone.</div>
                </div>
            </div>

            <div class="security-feature">
                <div class="security-feature-icon">
                    ${iconKey}
                </div>
                <div class="security-feature-content">
                    <div class="security-feature-title">HMAC-SHA256 Auth</div>
                    <div class="security-feature-desc">Devices pair with HMAC. No passwords on the server.</div>
                </div>
            </div>
        </div>
    </section>

    <!-- Installation Section -->
    <section class="section install" id="install">
        <div class="section-header">
            <h2 class="section-title">Install with npm</h2>
        </div>
        <div class="install-content">
            <div class="install-terminal">
                <div class="install-terminal-bar">
                    <span class="install-terminal-label">Terminal</span>
                    <button class="copy-btn" id="copy-btn" onclick="copyInstallCommand()">
                        ${iconCopy}
                        Copy
                    </button>
                </div>
                <div class="install-terminal-content">
                    <span class="prompt">$</span> npm i -g crabigator
                </div>
            </div>
            <div class="install-steps">
                <div class="install-step">
                    <div class="install-step-num">1</div>
                    <p class="install-step-text">Install via npm</p>
                </div>
                <div class="install-step">
                    <div class="install-step-num">2</div>
                    <p class="install-step-text">Run <code>crabigator</code> instead of <code>claude</code></p>
                </div>
                <div class="install-step">
                    <div class="install-step-num">3</div>
                    <p class="install-step-text">Click the pairing link to connect your phone</p>
                </div>
            </div>
        </div>
    </section>

    <!-- Open Source Section -->
    <section class="section open-source" id="open-source">
        <div class="github-card">
            <div class="github-header">
                ${iconGithubLogo}
                <div class="github-repo">
                    <span class="github-org">samuelclay</span>
                    <span class="github-sep">/</span>
                    <span class="github-name">crabigator</span>
                </div>
            </div>
            <p class="github-desc">
                A Rust TUI wrapper for Claude Code, Codex, opencode, and Grok. Streams the session to your phone. MIT licensed.
            </p>
            <div class="github-meta">
                <span class="github-lang">
                    <span class="lang-dot rust"></span>
                    Rust
                </span>
                <span class="github-license">
                    ${iconLicense}
                    MIT
                </span>
            </div>
            <div class="github-actions">
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="github-btn primary" data-track="github" data-label="star">
                    ${iconStar}
                    Star
                </a>
                <a href="https://github.com/samuelclay/crabigator/fork" target="_blank" rel="noopener" class="github-btn" data-track="github" data-label="fork">
                    ${iconFork}
                    Fork
                </a>
                <a href="https://github.com/samuelclay/crabigator/issues" target="_blank" rel="noopener" class="github-btn" data-track="github" data-label="issues">
                    ${iconIssues}
                    Issues
                </a>
                <a href="https://github.com/samuelclay/crabigator/pulls" target="_blank" rel="noopener" class="github-btn" data-track="github" data-label="prs">
                    ${iconPullRequest}
                    PRs
                </a>
            </div>
            <div class="github-links">
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener" class="github-link">
                    ${iconBook}
                    README
                </a>
                <a href="https://github.com/samuelclay/crabigator/blob/main/LICENSE" target="_blank" rel="noopener" class="github-link">
                    ${iconLicense}
                    LICENSE
                </a>
                <a href="https://github.com/samuelclay/crabigator/releases" target="_blank" rel="noopener" class="github-link">
                    ${iconTag}
                    Releases
                </a>
                <a href="https://github.com/samuelclay/crabigator/wiki" target="_blank" rel="noopener" class="github-link">
                    ${iconChat}
                    Discussions
                </a>
            </div>
        </div>
    </section>

    <!-- Why Crabigator Easter Egg -->
    <section class="why-crabigator">
        <div class="why-inner">
            <div class="why-question">Why "Crabigator"?</div>
            <div class="why-equation">
                <div class="why-term">
                    <div class="why-icon why-icon-claude">
                        ${iconCircle}
                    </div>
                    <span class="why-label">Claude</span>
                    <span class="why-sub">the AI</span>
                </div>
                <span class="why-plus">+</span>
                <div class="why-term">
                    <div class="why-icon why-icon-nav">
                        ${iconCompass}
                    </div>
                    <span class="why-label">Navigator</span>
                    <span class="why-sub">remote control</span>
                </div>
                <span class="why-plus">+</span>
                <div class="why-term">
                    <div class="why-icon why-icon-crab">
                        ${iconCrabigatorMono}
                    </div>
                    <span class="why-label">Crab</span>
                    <span class="why-sub">Rust's mascot</span>
                </div>
                <span class="why-plus">+</span>
                <div class="why-term">
                    <div class="why-icon why-icon-gator">
                        ${iconGator}
                    </div>
                    <span class="why-label">Alligator</span>
                    <span class="why-sub">Claude at Cal Academy</span>
                </div>
            </div>
            <div class="why-tagline">The Claude Navigator</div>
        </div>
    </section>

    <!-- Mobile Apps Coming Soon -->
    <section class="section mobile-apps" id="mobile">
        <div class="mobile-content">
            <div class="mobile-phones">
                <!-- iPhone -->
                <div class="phone iphone">
                    <div class="phone-notch"></div>
                    <div class="phone-screen">
                        <div class="app-header">
                            <span class="app-logo">🦀</span>
                            <span class="app-title">Crabigator</span>
                        </div>
                        <div class="app-notification">
                            <div class="notif-icon">
                                ${iconBell}
                            </div>
                            <div class="notif-content">
                                <div class="notif-title">Permission Required</div>
                                <div class="notif-body">Claude wants to run: git commit</div>
                            </div>
                        </div>
                        <div class="app-session">
                            <div class="session-state thinking">Thinking...</div>
                            <div class="session-path">~/projects/app</div>
                        </div>
                        <div class="app-actions">
                            <button class="app-btn approve">Approve</button>
                            <button class="app-btn deny">Deny</button>
                        </div>
                    </div>
                    <div class="phone-label">
                        ${iconClock}
                        iOS
                    </div>
                </div>
                <!-- Android -->
                <div class="phone android">
                    <div class="phone-screen">
                        <div class="app-header">
                            <span class="app-logo">🦀</span>
                            <span class="app-title">Crabigator</span>
                        </div>
                        <div class="app-terminal">
                            <div class="term-line"><span class="term-prompt">❯</span> Analyzing codebase...</div>
                            <div class="term-line"><span class="term-file">src/main.rs</span> modified</div>
                            <div class="term-line"><span class="term-added">+42</span> <span class="term-removed">-12</span></div>
                        </div>
                        <div class="app-stats">
                            <div class="stat"><span class="stat-value">3</span><span class="stat-label">Sessions</span></div>
                            <div class="stat"><span class="stat-value">47</span><span class="stat-label">Prompts</span></div>
                            <div class="stat"><span class="stat-value">2.1h</span><span class="stat-label">Time</span></div>
                        </div>
                    </div>
                    <div class="phone-label">
                        ${iconPhoneOutline}
                        Android
                    </div>
                </div>
            </div>
            <div class="mobile-info">
                <h2 class="section-title">iOS and Android apps</h2>
                <div class="mobile-features">
                    <div class="mobile-feature">
                        ${iconBell}
                        <div>
                            <strong>Push notifications</strong>
                            <span>A notification when Claude needs approval</span>
                        </div>
                    </div>
                    <div class="mobile-feature">
                        ${iconArrowRight}
                        <div>
                            <strong>A real app</strong>
                            <span>Not a browser tab</span>
                        </div>
                    </div>
                    <div class="mobile-feature">
                        ${iconCloudFilled}
                        <div>
                            <strong>Offline</strong>
                            <span>Read past sessions without internet</span>
                        </div>
                    </div>
                </div>
                <form class="email-form" id="email-form">
                    <input type="email" class="email-input" placeholder="you@example.com" required>
                    <button type="submit" class="email-btn">Notify Me</button>
                </form>
                <div class="email-success" id="email-success">
                    ${iconCheck} You're on the list!
                </div>
                <p class="email-privacy">We'll email you when the apps are ready.</p>
            </div>
        </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
        <div class="footer-content">
            <div class="footer-cta">
                <span class="footer-cta-text">Install Crabigator</span>
                <a href="#install" class="btn-primary" data-track="install_cta" data-label="footer">Install</a>
            </div>
            <div class="footer-links">
                <a href="/dashboard" class="footer-link">Dashboard</a>
                <a href="/mcp-tools" class="footer-link">MCP</a>
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="footer-link">GitHub</a>
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener" class="footer-link">Documentation</a>
            </div>
            <div class="footer-meta">
                MIT License
            </div>
        </div>
        <div class="footer-bottom">
            <p class="footer-bottom-text">Built by <a href="https://samuelclay.com" target="_blank" rel="noopener">Samuel Clay</a> in San Francisco. Talk to <a href="https://x.com/samuelclay" target="_blank" rel="noopener">@samuelclay on X</a>.</p>
        </div>
    </footer>

    <script>${analyticsJs}</script>
    <script>${landingJs}</script>
    <script>${palmWebglJs}</script>
</body>
</html>`;

export function renderLandingHtml(runtime: RuntimeConfig, metaPixelId = ''): string {
    const pixel = runtime.capabilities.marketing_analytics ? metaPixelHtml(metaPixelId) : '';
    const disabledStyles = [
        !runtime.capabilities.mcp ? '#mcp,.nav-link[href="#mcp"],.footer-link[href="/mcp-tools"]{display:none!important}' : '',
        !runtime.capabilities.marketing_analytics ? '.cta-form,.email-form{display:none!important}' : '',
    ].join('');
    let html = usePublicOrigin(landingHtml, runtime.origin)
        .replace('</head>', `${pixel}<style>${disabledStyles}</style></head>`);
    if (!runtime.capabilities.marketing_analytics) {
        html = html.replace(`<script>${analyticsJs}</script>`, '');
    }
    return html;
}
