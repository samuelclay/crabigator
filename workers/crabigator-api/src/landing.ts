// Landing page HTML served at /
import { landingCss } from './landing/css';
import { landingJs } from './landing/js';
import {
    iconCrabigator,
    iconCrabigatorEncoded,
    iconCrabigatorMono,
    iconCrabigatorPhoneLogo,
    iconCheck,
    iconCheckAlt,
    iconPhone,
    iconPhoneOutline,
    iconPhoneBento,
    iconPlay,
    iconPlayCircle,
    iconChart,
    iconChartBento,
    iconFolder,
    iconFolderBento,
    iconChanges,
    iconMultiDevice,
    iconFeatures,
    iconCloud,
    iconCloudFilled,
    iconCube,
    iconDiamond,
    iconSemanticDiff,
    iconLink,
    iconSearch,
    iconLines,
    iconMouse,
    iconBolt,
    iconDollar,
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
} from './landing/icons';

export const landingHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator - control Claude Code from anywhere</title>
    <meta name="description" content="Answer permissions, approve plans, and respond to questions from your phone. Real-time monitoring and remote control for Claude Code sessions.">
    <link rel="icon" href="data:image/svg+xml,${iconCrabigatorEncoded}">
    <style>${landingCss}</style>
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
            <a href="#pricing" class="nav-link">Pricing</a>
            <a href="#install" class="nav-link">Install</a>
            <a href="/dashboard" class="nav-btn">Open Dashboard</a>
            <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="nav-github">
                ${iconGithubSmall}
            </a>
        </div>
    </nav>

    <!-- Hero Section -->
    <section class="hero">
        <div class="hero-content">
            <div class="hero-text">
                <p class="hero-tagline">${iconPhone} Remote AI Control</p>
                <h1 class="hero-headline">
                    Control <span class="highlight">Claude Code</span> from anywhere
                </h1>
                <p class="hero-subheadline">
                    Answer permissions, approve plans, and respond to questions—all from your phone.
                    Claude Code runs natively on Mac, Windows, or Linux—exactly as intended.
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
                        <span class="terminal-title">crabigator — ~/projects/api</span>
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
                                    <span class="widget-label">◆ Session</span>
                                    <span class="widget-value blue">12m 34s</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">▸ Prompts 7</span>
                                    <span class="widget-value dim">2m ago</span>
                                </div>
                                <div class="widget-row">
                                    <span class="widget-label">⚙ Tools</span>
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
                                        <span class="change-name">handleSSE</span>
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
                                <span class="pw-icon">◆</span> Session
                                <span class="pw-value">12m 34s</span>
                            </div>
                            <div class="phone-widget-row">
                                <span>▸ Prompts 7</span>
                                <span class="pw-dim">2m ago</span>
                            </div>
                            <div class="phone-widget-row">
                                <span>⚙ Tools</span>
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
                                <span class="pc-name">handleSSE</span>
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
                    <span class="cta-label">Stay in the loop</span>
                </div>
                <p class="cta-text">Get updates on new features and mobile app launches.</p>
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
                <p class="section-label">${iconMultiDevice} Multi-device</p>
                <h2 class="section-title">Check in from phone or desktop</h2>
                <p class="section-subtitle">
                    No Tmux. No Tailscale. No Termius. Native integration built for Claude Code.
                    Pair any device in seconds—monitor progress and respond to prompts on the go.
                </p>
                <div class="showcase-status">
                    <div class="status-pills">
                        <span class="status-pill thinking">Thinking</span>
                        <span class="status-pill permission">Permission</span>
                        <span class="status-pill question">Question</span>
                        <span class="status-pill complete">Complete</span>
                    </div>
                    <p class="status-hint">Real-time state updates across all sessions</p>
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
                <p class="section-label">${iconPlayCircle} Interactive</p>
                <h2 class="section-title">Respond from anywhere</h2>
                <p class="section-subtitle">
                    Answer permissions, review plans, and respond to questions—all from your phone or any browser.
                    Add custom instructions before approving to guide Claude's next steps.
                </p>
                <ul class="interactive-features">
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Permission prompts</strong> with one-tap approve or deny</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Add instructions</strong> before approving—guide Claude's approach</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Answer questions</strong> when Claude needs clarification</span>
                    </li>
                    <li>
                        <span class="feature-icon">${iconCheck}</span>
                        <span class="feature-text"><strong>Review plans</strong> before Claude starts implementing</span>
                    </li>
                </ul>
            </div>
        </div>
    </section>

    <!-- Features Bento Grid -->
    <section class="section features-section" id="features">
        <div class="section-header">
            <p class="section-label">${iconFeatures} Features</p>
            <h2 class="section-title">Everything you need to stay connected</h2>
        </div>
        <div class="bento-grid">
            <!-- Row 1-2: Session Stats (tall) + Git Changes (tall) + Cloud Dashboard + Mobile -->
            <div class="bento-card tall">
                <div class="bento-icon">${iconChartBento}</div>
                <h3 class="bento-title">Session Statistics</h3>
                <p class="bento-desc">Real-time metrics that update as Claude works. Track prompts, completions, tool calls, and session duration at a glance.</p>
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
                <h3 class="bento-title">Git Changes</h3>
                <p class="bento-desc">See every file Claude modifies with visual diff bars. Additions in green, deletions in red—know exactly what's changing.</p>
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
                <h3 class="bento-title">Mobile Access</h3>
                <p class="bento-desc">Approve permissions and answer questions from your phone.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon anthropic">${iconCube}</div>
                <h3 class="bento-title">Claude Code</h3>
                <p class="bento-desc">First-class support with deep hook integration.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon openai">${iconDiamond}</div>
                <h3 class="bento-title">Codex CLI</h3>
                <p class="bento-desc">Works seamlessly with OpenAI's Codex CLI.</p>
            </div>

            <!-- Row 3-4: Semantic Diff (wide + tall) + File Links (tall) -->
            <div class="bento-card wide tall">
                <div class="bento-icon">${iconSemanticDiff}</div>
                <h3 class="bento-title">Semantic Diff Parsing</h3>
                <p class="bento-desc">Changes grouped by language with function and method names extracted. See exactly which functions are being modified, not just file names.</p>
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
                <h3 class="bento-title">Clickable File Links</h3>
                <p class="bento-desc">Every file path is a link. Click to open directly in VS Code, Cursor, Zed, or your preferred editor.</p>
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
                <h3 class="bento-title">CLI Inspection</h3>
                <p class="bento-desc">Use <code>crabigator inspect</code> to view running instances. Perfect for automation.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconLines}</div>
                <h3 class="bento-title">Native Scrollback</h3>
                <p class="bento-desc">Uses your terminal's primary buffer—unlike tmux. Scroll naturally.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconMouse}</div>
                <h3 class="bento-title">Mouse Selection</h3>
                <p class="bento-desc">Select and copy text naturally—no tmux capture mode.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon">${iconBolt}</div>
                <h3 class="bento-title">Instant Pairing</h3>
                <p class="bento-desc">Scan a QR code or enter a short code. Connected in seconds.</p>
            </div>
        </div>
    </section>

    <!-- Pricing Section -->
    <section class="section pricing" id="pricing">
        <div class="section-header">
            <p class="section-label">${iconDollar} Pricing</p>
            <h2 class="section-title">Simple, transparent pricing</h2>
        </div>
        <div class="pricing-cards">
            <div class="pricing-card">
                <div class="pricing-inner">
                    <div class="pricing-tier">Free</div>
                    <div class="pricing-amount">$0<span class="pricing-period">/month</span></div>
                    <ul class="pricing-features">
                        <li><span class="check">${iconCheck}</span> Unlimited Claude Code sessions</li>
                        <li><span class="check">${iconCheck}</span> Answer permissions & questions</li>
                        <li><span class="check">${iconCheck}</span> Unlimited web and mobile access</li>
                        <li><span class="check">${iconCheck}</span> Real-time sync</li>
                        <li><span class="check dim">${iconCheck}</span> <span class="dim">30 min/day remote access</span></li>
                    </ul>
                    <a href="#install" class="btn-primary pricing-cta outline">Get Started</a>
                </div>
            </div>
            <div class="pricing-card featured">
                <div class="pricing-inner">
                    <div class="pricing-tier">Pro</div>
                    <div class="pricing-amount">$3<span class="pricing-period">/month</span></div>
                    <ul class="pricing-features">
                        <li><span class="check">${iconCheck}</span> Unlimited Claude Code sessions</li>
                        <li><span class="check">${iconCheck}</span> Answer permissions & questions</li>
                        <li><span class="check">${iconCheck}</span> Unlimited web and mobile access</li>
                        <li><span class="check">${iconCheck}</span> Real-time sync</li>
                        <li><span class="check">${iconCheck}</span> Unlimited remote access</li>
                    </ul>
                    <a href="#install" class="btn-primary pricing-cta">Get Started</a>
                </div>
            </div>
        </div>
    </section>

    <!-- Installation Section -->
    <section class="section install" id="install">
        <div class="section-header">
            <p class="section-label">${iconCheckAlt} Get Started</p>
            <h2 class="section-title">Install in 30 seconds</h2>
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
                    <span class="prompt">$</span> npm install -g crabigator
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
                A Rust TUI wrapper for Claude Code with real-time mobile streaming. MIT licensed.
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
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="github-btn primary">
                    ${iconStar}
                    Star
                </a>
                <a href="https://github.com/samuelclay/crabigator/fork" target="_blank" rel="noopener" class="github-btn">
                    ${iconFork}
                    Fork
                </a>
                <a href="https://github.com/samuelclay/crabigator/issues" target="_blank" rel="noopener" class="github-btn">
                    ${iconIssues}
                    Issues
                </a>
                <a href="https://github.com/samuelclay/crabigator/pulls" target="_blank" rel="noopener" class="github-btn">
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
                <p class="section-label">${iconPhone} Coming Soon</p>
                <h2 class="section-title">Native mobile apps</h2>
                <div class="mobile-features">
                    <div class="mobile-feature">
                        ${iconBell}
                        <div>
                            <strong>Push notifications</strong>
                            <span>Know instantly when Claude needs your approval</span>
                        </div>
                    </div>
                    <div class="mobile-feature">
                        ${iconArrowRight}
                        <div>
                            <strong>Native performance</strong>
                            <span>Smooth 60fps animations and instant response</span>
                        </div>
                    </div>
                    <div class="mobile-feature">
                        ${iconCloudFilled}
                        <div>
                            <strong>Offline support</strong>
                            <span>Review sessions even without internet</span>
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
                <p class="email-privacy">Get notified about product launches and updates.</p>
            </div>
        </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
        <div class="footer-content">
            <div class="footer-cta">
                <span class="footer-cta-text">Ready to control Claude Code from anywhere?</span>
                <a href="#install" class="btn-primary">Install Now</a>
            </div>
            <div class="footer-links">
                <a href="/dashboard" class="footer-link">Dashboard</a>
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="footer-link">GitHub</a>
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener" class="footer-link">Documentation</a>
            </div>
            <div class="footer-meta">
                MIT License
            </div>
        </div>
        <div class="footer-bottom">
            <p class="footer-bottom-text">Built by <a href="https://samuelclay.com" target="_blank" rel="noopener">Samuel Clay</a>. Talk to <a href="https://x.com/samuelclay" target="_blank" rel="noopener">Samuel Clay</a> on X.</p>
        </div>
    </footer>

    <script>${landingJs}</script>
</body>
</html>`;
