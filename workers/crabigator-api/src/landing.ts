// Landing page HTML served at /
import { landingCss } from './landing/css';
import { landingJs } from './landing/js';

export const landingHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator - Monitor and control Claude Code from anywhere</title>
    <meta name="description" content="Answer permissions, approve plans, and respond to questions from your phone. Real-time monitoring and remote control for Claude Code sessions.">
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512'><ellipse fill='%23f97316' cx='256' cy='294' rx='144' ry='121'/><circle fill='%23f97316' cx='82' cy='113' r='70'/><circle fill='%23f97316' cx='430' cy='113' r='70'/><path fill='%23ea580c' d='M400,294c0-67-65-121-144-121v242c80,0,144-54,144-121z'/><path fill='%23ea580c' d='M379,266c-42,0-77-34-77-77c0-3,0-6,1-8c1-11,11-19,22-18s19,11,18,22c0,1,0,3,0,4c0,20,16,37,37,37c1,0,2,0,3,0c11-1,21,7,21,18c1,11-7,21-18,22c-2,0-5,0-7,0z'/><path fill='%23ea580c' d='M492,332c-5,0-11-2-15-6c-20-22-50-36-83-40c-11-1-19-11-18-22s11-19,22-18c43,5,81,24,108,53c8,8,7,21-1,28c-4,4-9,5-13,5z'/><path fill='%23ea580c' d='M483,411c-7,0-13-3-17-9c-17-25-44-45-77-54c-11-3-17-14-14-25s14-17,25-14c42,12,77,37,99,71c6,9,3,22-6,28c-3,2-7,3-10,3z'/><path fill='%23ea580c' d='M435,469c-7,0-15-4-18-11c-14-28-40-52-73-65c-10-4-15-16-11-26s16-15,26-11c43,17,76,47,94,85c5,10,1,22-9,27c-3,1-6,2-9,2z'/><path fill='%23f97316' d='M133,266c-42,0-77-34-77-77c0-3,0-6,1-8c1-11,11-19,22-18s19,11,18,22c0,1,0,3,0,4c0,20,16,37,37,37c1,0,2,0,3,0c11-1,21,7,21,18c1,11-7,21-18,22c-2,0-5,0-7,0z'/><path fill='%23f97316' d='M20,332c-5,0-10-2-14-5c-8-8-9-20-1-28c27-29,65-48,108-53c11-1,21,7,22,18s-7,21-18,22c-33,4-63,18-83,40c-4,4-9,6-14,6z'/><path fill='%23f97316' d='M29,411c-4,0-8-1-11-3c-9-6-12-18-6-28c22-34,57-59,99-71c11-3,22,3,25,14s-3,22-14,25c-33,10-60,29-77,54c-4,6-10,9-16,9z'/><path fill='%23f97316' d='M77,469c-3,0-6-1-9-2c-10-5-14-17-9-27c18-38,51-68,94-85c10-4,22,1,26,11s-1,22-11,26c-33,13-59,37-73,65c-3,7-11,11-18,11z'/></svg>">
    <style>${landingCss}</style>
</head>
<body>
    <!-- Navigation -->
    <nav class="nav">
        <a href="/" class="nav-logo">
            <svg viewBox="0 0 512 512"><ellipse fill="currentColor" cx="256" cy="294" rx="144" ry="121"/><circle fill="currentColor" cx="82" cy="113" r="70"/><circle fill="currentColor" cx="430" cy="113" r="70"/><path fill="currentColor" d="M379,266c-42,0-77-34-77-77c0-3,0-6,1-8c1-11,11-19,22-18s19,11,18,22c0,1,0,3,0,4c0,20,16,37,37,37c1,0,2,0,3,0c11-1,21,7,21,18c1,11-7,21-18,22c-2,0-5,0-7,0z"/><path fill="currentColor" d="M492,332c-5,0-11-2-15-6c-20-22-50-36-83-40c-11-1-19-11-18-22s11-19,22-18c43,5,81,24,108,53c8,8,7,21-1,28c-4,4-9,5-13,5z"/><path fill="currentColor" d="M483,411c-7,0-13-3-17-9c-17-25-44-45-77-54c-11-3-17-14-14-25s14-17,25-14c42,12,77,37,99,71c6,9,3,22-6,28c-3,2-7,3-10,3z"/><path fill="currentColor" d="M435,469c-7,0-15-4-18-11c-14-28-40-52-73-65c-10-4-15-16-11-26s16-15,26-11c43,17,76,47,94,85c5,10,1,22-9,27c-3,1-6,2-9,2z"/><path fill="currentColor" d="M133,266c-42,0-77-34-77-77c0-3,0-6,1-8c1-11,11-19,22-18s19,11,18,22c0,1,0,3,0,4c0,20,16,37,37,37c1,0,2,0,3,0c11-1,21,7,21,18c1,11-7,21-18,22c-2,0-5,0-7,0z"/><path fill="currentColor" d="M20,332c-5,0-10-2-14-5c-8-8-9-20-1-28c27-29,65-48,108-53c11-1,21,7,22,18s-7,21-18,22c-33,4-63,18-83,40c-4,4-9,6-14,6z"/><path fill="currentColor" d="M29,411c-4,0-8-1-11-3c-9-6-12-18-6-28c22-34,57-59,99-71c11-3,22,3,25,14s-3,22-14,25c-33,10-60,29-77,54c-4,6-10,9-16,9z"/><path fill="currentColor" d="M77,469c-3,0-6-1-9-2c-10-5-14-17-9-27c18-38,51-68,94-85c10-4,22,1,26,11s-1,22-11,26c-33,13-59,37-73,65c-3,7-11,11-18,11z"/></svg>
            Crabigator
        </a>
        <div class="nav-links">
            <a href="#features" class="nav-link">Features</a>
            <a href="#pricing" class="nav-link">Pricing</a>
            <a href="#install" class="nav-link">Install</a>
            <a href="/dashboard" class="nav-btn">Open Dashboard</a>
            <a href="https://github.com/anthropics/crabigator" target="_blank" rel="noopener" class="nav-github">
                <svg viewBox="0 0 16 16" fill="currentColor">
                    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
                </svg>
            </a>
        </div>
    </nav>

    <!-- Hero Section -->
    <section class="hero">
        <div class="hero-content">
            <div class="hero-text">
                <p class="hero-tagline"><svg class="icon" viewBox="0 0 256 256"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM140,208H116a8,8,0,0,1,0-16h24a8,8,0,0,1,0,16Z"/></svg> Remote AI Control</p>
                <h1 class="hero-headline">
                    Control <span class="highlight">Claude Code</span> from anywhere
                </h1>
                <p class="hero-subheadline">
                    Answer permissions, approve plans, and respond to questions—all from your phone.
                    Never miss a prompt when Claude needs your input.
                </p>
                <div class="hero-ctas">
                    <a href="#install" class="btn-primary">Install Now</a>
                    <a href="/dashboard" class="btn-secondary">View Demo Dashboard</a>
                </div>
                <p class="hero-price">
                    <span class="price">$3/month</span>
                    <span>·</span>
                    <span>Unlimited sessions</span>
                    <span>·</span>
                    <span>Unlimited viewers</span>
                    <span>·</span>
                    <span>Cancel anytime</span>
                </p>
                <div class="hero-email-card">
                    <div class="hero-email-header">
                        <svg class="hero-email-icon" viewBox="0 0 20 20" fill="currentColor">
                            <path d="M2.003 5.884L10 9.882l7.997-3.998A2 2 0 0016 4H4a2 2 0 00-1.997 1.884z"/>
                            <path d="M18 8.118l-8 4-8-4V14a2 2 0 002 2h12a2 2 0 002-2V8.118z"/>
                        </svg>
                        <span class="hero-email-label">Stay in the loop</span>
                    </div>
                    <form class="hero-email-form" id="hero-email-form">
                        <input type="email" class="hero-email-input" placeholder="you@example.com" required>
                        <button type="submit" class="hero-email-btn">Subscribe</button>
                    </form>
                    <div class="hero-email-success" id="hero-email-success">
                        <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> You're on the list!
                    </div>
                </div>
            </div>

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
                            <span class="terminal-prompt"><svg class="icon" viewBox="0 0 256 256"><path d="M181.66,133.66l-80,80A8,8,0,0,1,88,208V48a8,8,0,0,1,13.66-5.66l80,80A8,8,0,0,1,181.66,133.66Z"/></svg></span>
                            <span class="terminal-cmd"> crabigator</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-output">Starting Claude Code session...</span>
                        </div>
                        <div class="terminal-line">
                            <span class="terminal-success"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> Connected to drinkcrabigator.com</span>
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
                                <div class="widget-header"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M232,208a8,8,0,0,1-8,8H32a8,8,0,0,1-8-8V48a8,8,0,0,1,16,0V156.69l50.34-50.35a8,8,0,0,1,11.32,0L128,132.69,180.69,80H160a8,8,0,0,1,0-16h40a8,8,0,0,1,8,8v40a8,8,0,0,1-16,0V91.31l-58.34,58.35a8,8,0,0,1-11.32,0L96,123.31l-56,56V200H224A8,8,0,0,1,232,208Z"/></svg> Session Stats</div>
                                <div class="widget-row">
                                    <span>Duration</span>
                                    <span class="widget-value">12m 34s</span>
                                </div>
                                <div class="widget-row">
                                    <span>Prompts</span>
                                    <span class="widget-value">7</span>
                                </div>
                                <div class="widget-row">
                                    <span>Tool calls</span>
                                    <span class="widget-value">23</span>
                                </div>
                            </div>
                            <div class="widget-card">
                                <div class="widget-header"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg> Git Status</div>
                                <div class="widget-row">
                                    <span>Branch</span>
                                    <span class="widget-value">main</span>
                                </div>
                                <div class="widget-row">
                                    <span>Changes</span>
                                    <span><span class="widget-value green">+142</span> <span class="widget-value red">-38</span></span>
                                </div>
                                <div class="widget-row">
                                    <span>Modified</span>
                                    <span class="widget-value">3 files</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </section>

    <!-- Device Showcase -->
    <section class="section showcase">
        <div class="showcase-content">
            <div class="showcase-text">
                <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M224,72H208V48a24,24,0,0,0-24-24H40A24,24,0,0,0,16,48V152a24,24,0,0,0,24,24H152v24a24,24,0,0,0,24,24h48a24,24,0,0,0,24-24V96A24,24,0,0,0,224,72ZM40,160a8,8,0,0,1-8-8V48a8,8,0,0,1,8-8H184a8,8,0,0,1,8,8V72H176a24,24,0,0,0-24,24v64Zm192,40a8,8,0,0,1-8,8H176a8,8,0,0,1-8-8V96a8,8,0,0,1,8-8h48a8,8,0,0,1,8,8Z"/></svg> Multi-device</p>
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
                                <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> Refactored auth module<br>
                                <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> Added unit tests
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
                <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm36.71,118.79-48,36A8,8,0,0,1,104,172V100a8,8,0,0,1,12.71-6.47l48,36a8,8,0,0,1,0,12.94Z"/></svg> Interactive</p>
                <h2 class="section-title">Respond from anywhere</h2>
                <p class="section-subtitle">
                    Answer permissions, review plans, and respond to questions—all from your phone or any browser.
                    Add custom instructions before approving to guide Claude's next steps.
                </p>
                <ul class="interactive-features">
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Permission prompts</strong> with one-tap approve or deny</span>
                    </li>
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Add instructions</strong> before approving—guide Claude's approach</span>
                    </li>
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Answer questions</strong> when Claude needs clarification</span>
                    </li>
                    <li>
                        <span class="feature-icon"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span>
                        <span class="feature-text"><strong>Review plans</strong> before Claude starts implementing</span>
                    </li>
                </ul>
            </div>
        </div>
    </section>

    <!-- Features Bento Grid -->
    <section class="section features-section" id="features">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M176,48H80A56,56,0,0,0,24,104v48a56,56,0,0,0,56,56h96a56,56,0,0,0,56-56V104A56,56,0,0,0,176,48ZM80,176a24,24,0,1,1,24-24A24,24,0,0,1,80,176Zm96,0a24,24,0,1,1,24-24A24,24,0,0,1,176,176Z"/></svg> Features</p>
            <h2 class="section-title">Everything you need to stay connected</h2>
        </div>
        <div class="bento-grid">
            <!-- Row 1-2: Session Stats (tall) + Git Changes (tall) + Cloud Dashboard + Mobile -->
            <div class="bento-card tall">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M232,208a8,8,0,0,1-8,8H32a8,8,0,0,1-8-8V48a8,8,0,0,1,16,0V156.69l50.34-50.35a8,8,0,0,1,11.32,0L128,132.69,180.69,80H160a8,8,0,0,1,0-16h40a8,8,0,0,1,8,8v40a8,8,0,0,1-16,0V91.31l-58.34,58.35a8,8,0,0,1-11.32,0L96,123.31l-56,56V200H224A8,8,0,0,1,232,208Z"/></svg></div>
                <h3 class="bento-title">Session Statistics</h3>
                <p class="bento-desc">Real-time metrics that update as Claude works. Track prompts, completions, tool calls, and session duration at a glance.</p>
                <div class="bento-visual">
                    <div class="mini-widget">
                        <div class="widget-header-mini"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M232,208a8,8,0,0,1-8,8H32a8,8,0,0,1-8-8V48a8,8,0,0,1,16,0V156.69l50.34-50.35a8,8,0,0,1,11.32,0L128,132.69,180.69,80H160a8,8,0,0,1,0-16h40a8,8,0,0,1,8,8v40a8,8,0,0,1-16,0V91.31l-58.34,58.35a8,8,0,0,1-11.32,0L96,123.31l-56,56V200H224A8,8,0,0,1,232,208Z"/></svg> Session Stats</div>
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
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg></div>
                <h3 class="bento-title">Git Changes</h3>
                <p class="bento-desc">See every file Claude modifies with visual diff bars. Additions in green, deletions in red—know exactly what's changing.</p>
                <div class="bento-visual">
                    <div class="mini-widget">
                        <div class="widget-header-mini"><svg class="icon-widget" viewBox="0 0 256 256"><path d="M216,72H131.31L104,44.69A15.86,15.86,0,0,0,92.69,40H40A16,16,0,0,0,24,56V200.62A15.4,15.4,0,0,0,39.38,216H216.89A15.13,15.13,0,0,0,232,200.89V88A16,16,0,0,0,216,72Z"/></svg> 6 files</div>
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
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M160.06,40A88.1,88.1,0,0,0,81.29,88.67,64,64,0,1,0,72,216h88a88,88,0,0,0,.06-176Z"/></svg></div>
                <h3 class="bento-title">Cloud Dashboard</h3>
                <p class="bento-desc">View sessions and respond to prompts from any browser. No VPN needed.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM140,208H116a8,8,0,0,1,0-16h24a8,8,0,0,1,0,16Z"/></svg></div>
                <h3 class="bento-title">Mobile Access</h3>
                <p class="bento-desc">Approve permissions and answer questions from your phone.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon anthropic"><svg viewBox="0 0 256 256"><path d="M232,80.18v95.64a16,16,0,0,1-8.32,14l-80,43.41a16,16,0,0,1-15.36,0l-80-43.41a16,16,0,0,1-8.32-14V80.18a16,16,0,0,1,8.32-14l80-43.41a16,16,0,0,1,15.36,0l80,43.41A16,16,0,0,1,232,80.18Z"/></svg></div>
                <h3 class="bento-title">Claude Code</h3>
                <p class="bento-desc">First-class support with deep hook integration.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon openai"><svg viewBox="0 0 256 256"><path d="M240,128a15.85,15.85,0,0,1-4.67,11.28l-96.05,96.06a16,16,0,0,1-22.56,0l-96-96.06a16,16,0,0,1,0-22.56l96.05-96.06a16,16,0,0,1,22.56,0l96.05,96.06A15.85,15.85,0,0,1,240,128Z"/></svg></div>
                <h3 class="bento-title">Codex CLI</h3>
                <p class="bento-desc">Works seamlessly with OpenAI's Codex CLI.</p>
            </div>

            <!-- Row 3-4: Semantic Diff (wide + tall) + File Links (tall) -->
            <div class="bento-card wide tall">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M200,204.5V232a8,8,0,0,1-16,0V204.5a63.67,63.67,0,0,0-35.38-57.25l-48.4-24.19A79.58,79.58,0,0,1,56,51.5V24a8,8,0,0,1,16,0V51.5a63.67,63.67,0,0,0,35.38,57.25l48.4,24.19A79.58,79.58,0,0,1,200,204.5ZM164.62,64.75,142.47,53.68A79.58,79.58,0,0,0,98.22,35.94,63.67,63.67,0,0,1,72,51.5V72a8,8,0,0,1-16,0V51.5a79.58,79.58,0,0,0-32.78-64.34A8,8,0,0,0,8,0V51.5A79.58,79.58,0,0,0,52.25,123.06l22.13,11.07A79.58,79.58,0,0,1,118.6,151.87,63.67,63.67,0,0,0,144,136.31V116a8,8,0,0,1,16,0v20.31a79.58,79.58,0,0,1,32.78,64.34A8,8,0,0,0,208,188V136.31A79.58,79.58,0,0,0,164.62,64.75Z"/></svg></div>
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
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M208,32H48A16,16,0,0,0,32,48V208a16,16,0,0,0,16,16H208a16,16,0,0,0,16-16V48A16,16,0,0,0,208,32ZM144,176H112a40,40,0,0,1,0-80h32a8,8,0,0,1,0,16H112a24,24,0,0,0,0,48h32a8,8,0,0,1,0,16Zm48-56H168a8,8,0,0,1,0-16h24a24,24,0,0,0,0-48H168a8,8,0,0,1,0-16h24a40,40,0,0,1,0,80Z"/></svg></div>
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
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M168,112a56,56,0,1,1-56-56A56.06,56.06,0,0,1,168,112Zm61.66,117.66a8,8,0,0,1-11.32,0l-50.06-50.07a88,88,0,1,1,11.32-11.31l50.06,50.06A8,8,0,0,1,229.66,229.66Z"/></svg></div>
                <h3 class="bento-title">CLI Inspection</h3>
                <p class="bento-desc">Use <code>crabigator inspect</code> to view running instances. Perfect for automation.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M224,128a8,8,0,0,1-8,8H40a8,8,0,0,1,0-16H216A8,8,0,0,1,224,128ZM40,72H216a8,8,0,0,0,0-16H40a8,8,0,0,0,0,16Zm104,112H40a8,8,0,0,0,0,16H144a8,8,0,0,0,0-16Z"/></svg></div>
                <h3 class="bento-title">Native Scrollback</h3>
                <p class="bento-desc">Uses your terminal's primary buffer—unlike tmux. Scroll naturally.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M144,16H112A64.07,64.07,0,0,0,48,80v96a64.07,64.07,0,0,0,64,64h32a64.07,64.07,0,0,0,64-64V80A64.07,64.07,0,0,0,144,16Zm-8,64a8,8,0,0,1-16,0V48a8,8,0,0,1,16,0Z"/></svg></div>
                <h3 class="bento-title">Mouse Selection</h3>
                <p class="bento-desc">Select and copy text naturally—no tmux capture mode.</p>
            </div>
            <div class="bento-card small">
                <div class="bento-icon"><svg viewBox="0 0 256 256"><path d="M213.85,125.46l-112,120a8,8,0,0,1-13.69-7l14.66-73.33L45.19,143.49a8,8,0,0,1-3-13l112-120a8,8,0,0,1,13.69,7L153.18,90.9l57.63,21.61a8,8,0,0,1,3,12.95Z"/></svg></div>
                <h3 class="bento-title">Instant Pairing</h3>
                <p class="bento-desc">Scan a QR code or enter a short code. Connected in seconds.</p>
            </div>
        </div>
    </section>

    <!-- Pricing Section -->
    <section class="section pricing" id="pricing">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm12,152h-4v8a8,8,0,0,1-16,0v-8H104a8,8,0,0,1,0-16h36a12,12,0,0,0,0-24H116a28,28,0,0,1,0-56h4V72a8,8,0,0,1,16,0v8h16a8,8,0,0,1,0,16H116a12,12,0,0,0,0,24h24a28,28,0,0,1,0,56Z"/></svg> Pricing</p>
            <h2 class="section-title">Simple, transparent pricing</h2>
        </div>
        <div class="pricing-cards">
            <div class="pricing-card">
                <div class="pricing-inner">
                    <div class="pricing-tier">Free</div>
                    <div class="pricing-amount">$0<span class="pricing-period">/month</span></div>
                    <ul class="pricing-features">
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited Claude Code sessions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Answer permissions & questions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited web and mobile access</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Real-time sync</li>
                        <li><span class="check dim"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> <span class="dim">30 min/day remote access</span></li>
                    </ul>
                    <a href="#install" class="btn-primary pricing-cta outline">Get Started</a>
                </div>
            </div>
            <div class="pricing-card featured">
                <div class="pricing-inner">
                    <div class="pricing-tier">Pro</div>
                    <div class="pricing-amount">$3<span class="pricing-period">/month</span></div>
                    <ul class="pricing-features">
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited Claude Code sessions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Answer permissions & questions</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited web and mobile access</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Real-time sync</li>
                        <li><span class="check"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg></span> Unlimited remote access</li>
                    </ul>
                    <a href="#install" class="btn-primary pricing-cta">Get Started</a>
                </div>
            </div>
        </div>
    </section>

    <!-- Installation Section -->
    <section class="section install" id="install">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm49.53,85.8-58.67,56a8,8,0,0,1-11.05,0l-29.33-28a8,8,0,0,1,11.05-11.62L116,149.91l53.14-50.69a8,8,0,0,1,11.06.06A8,8,0,0,1,177.53,109.8Z"/></svg> Get Started</p>
            <h2 class="section-title">Install in 30 seconds</h2>
        </div>
        <div class="install-content">
            <div class="install-terminal">
                <div class="install-terminal-bar">
                    <span class="install-terminal-label">Terminal</span>
                    <button class="copy-btn" id="copy-btn" onclick="copyInstallCommand()">
                        <svg viewBox="0 0 16 16" fill="currentColor">
                            <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
                            <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
                        </svg>
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
                    <p class="install-step-text">Scan QR code to pair your phone</p>
                </div>
            </div>
        </div>
    </section>

    <!-- Open Source Section -->
    <section class="section open-source">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M208,104a79.86,79.86,0,0,0-29.27-61.81,80,80,0,0,0-101.46,0A79.86,79.86,0,0,0,48,104c0,44.18,36,80,80,80s80-35.82,80-80ZM128,56a48,48,0,0,1,32,83.89V128a8,8,0,0,0-8-8H104a8,8,0,0,0-8,8v11.89A48,48,0,0,1,128,56ZM80,232a8,8,0,0,1,0-16h96a8,8,0,0,1,0,16Zm88-32H88a8,8,0,0,1-8-8V160a8,8,0,0,1,8-8H168a8,8,0,0,1,8,8v32A8,8,0,0,1,168,200Z"/></svg> Open Source</p>
            <h2 class="section-title">Free and open source</h2>
        </div>
        <div class="open-source-content">
            <p class="open-source-text">
                Crabigator is MIT licensed. View the source, submit issues, contribute
                features, or fork it for your own needs. The cloud dashboard is optional —
                the local TUI works standalone.
            </p>
            <div class="open-source-links">
                <a href="https://github.com/anthropics/crabigator" target="_blank" rel="noopener" class="open-source-link primary">
                    View on GitHub
                </a>
                <a href="https://github.com/anthropics/crabigator#readme" target="_blank" rel="noopener" class="open-source-link secondary">
                    Read the Docs
                </a>
                <a href="https://github.com/anthropics/crabigator/issues" target="_blank" rel="noopener" class="open-source-link secondary">
                    Report an Issue
                </a>
            </div>
        </div>
    </section>

    <!-- Mobile Apps Coming Soon -->
    <section class="section mobile-apps">
        <div class="section-header">
            <p class="section-label"><svg class="icon" viewBox="0 0 256 256"><path d="M176,16H80A24,24,0,0,0,56,40V216a24,24,0,0,0,24,24h96a24,24,0,0,0,24-24V40A24,24,0,0,0,176,16ZM140,208H116a8,8,0,0,1,0-16h24a8,8,0,0,1,0,16Z"/></svg> Coming Soon</p>
            <h2 class="section-title">Native mobile apps</h2>
            <p class="section-subtitle">
                Get notified when Crabigator launches on iOS and Android.
                Push notifications when Claude needs you, native performance, and offline support.
            </p>
        </div>
        <form class="email-form" id="email-form">
            <input type="email" class="email-input" placeholder="you@example.com" required>
            <button type="submit" class="email-btn">Notify Me</button>
        </form>
        <div class="email-success" id="email-success">
            <svg class="icon" viewBox="0 0 256 256"><path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z"/></svg> You're on the list! We'll email you when mobile apps launch.
        </div>
        <p class="email-privacy">We'll only email you about the mobile app launch. No spam.</p>
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
                <a href="https://github.com/anthropics/crabigator" target="_blank" rel="noopener" class="footer-link">GitHub</a>
                <a href="https://github.com/anthropics/crabigator#readme" target="_blank" rel="noopener" class="footer-link">Documentation</a>
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
