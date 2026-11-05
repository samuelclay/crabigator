// Landing page HTML served at /
import { faviconPath, faviconIcoPath, touchIconPath } from './assets/home-brand';
import { heroCapture, claudeCapture, codexCapture } from './landing/desktop-capture';
import { appModulesHtml } from './landing/app-modules';
import { notificationPreviewHtml, notificationCss, notificationJs } from './landing/notification-preview';
import { devicePairHtml } from './landing/device-pair';
import { flowPlayerJs } from './landing/flow-player';
import { claimsHtml, cardsHtml } from './landing/features';
import { tabletPreviewHtml, webPreviewHtml, androidPreviewHtml } from './landing/product-previews';
import { productPreviewsCss } from './landing/product-previews-css';
import { homeCss } from './landing/home-css';
import { heroStreamCss, heroStreamJs } from './landing/hero-stream';
import { mcpCss } from './landing/mcp-css';
import { landingJs } from './landing/js';
import { analyticsJs } from './landing/analytics';
import type { RuntimeConfig } from './config';
import { metaPixelHtml, usePublicOrigin } from './html-render';
import { mcpLandingSectionHtml } from './mcp/docs';
import {
    iconCrabigator, iconCrabigatorLogo, iconCrabigatorWordmark,
    iconMicrophone, iconKeyboard, iconWifi, iconClaude, iconCodex, iconOpencode, iconGrok, iconCopy,
} from './landing/icons';

export const landingHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator — Take your agents with you</title>
    <meta name="description" content="Answer permissions, approve plans, and respond to questions from your phone. Watch Claude Code, Codex, opencode, and Grok sessions as they run.">
    <meta property="og:title" content="Crabigator - Control Claude Code, Codex, opencode, and Grok from your phone">
    <meta property="og:description" content="Answer permissions, approve plans, and respond to questions from your phone. Watch Claude Code, Codex, opencode, and Grok sessions as they run.">
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-landing.png?v=0e52372806">
    <meta property="og:url" content="https://drinkcrabigator.com/">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator - Control Claude Code, Codex, opencode, and Grok from your phone">
    <meta name="twitter:description" content="Answer permissions, approve plans, and respond to questions from your phone.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-landing.png?v=0e52372806">
    <link rel="icon" href="${faviconPath}" type="image/svg+xml">
    <link rel="icon" href="${faviconIcoPath}" sizes="16x16 32x32 48x48">
    <link rel="apple-touch-icon" href="${touchIconPath}" sizes="192x192">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;450;500;550;600;650;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap" rel="stylesheet">
    <style>${mcpCss}${homeCss}${productPreviewsCss}${heroStreamCss}${notificationCss}</style>
</head>
<body>
    <header class="site-header">
        <a class="wordmark" href="/" aria-label="Crabigator home">${iconCrabigatorLogo}</a>
        <nav aria-label="Main navigation">
            <a href="#claude-code">Claude Code</a>
            <a href="#codex">Codex</a>
            <a href="#apps">Apps</a>
            <a href="#install">Install</a>
            <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener">GitHub</a>
            <a class="dashboard-link" href="/dashboard" data-track="dashboard_nav" data-label="nav">Open dashboard</a>
        </nav>
    </header>
    <main id="main">
        <section class="hero" aria-labelledby="hero-title">
            <div class="hero-copy">
                <h1 id="hero-title">Take your agents with you.</h1>
                <p class="hero-description">Claude Code, Codex, opencode, and Grok, live on your phone.</p>
                <div class="hero-start">
                    <a class="button-primary" href="#install" data-track="install_cta" data-label="hero">Install Crabigator</a>
                    <a class="button-secondary" href="/dashboard" data-track="dashboard_cta" data-label="hero">Open dashboard</a>
                </div>
            </div>
            <figure class="hero-screens device-pair" aria-label="The same Claude Code session on desktop and iPhone">
                <div class="desktop-frame">
                    <div class="desktop-bar"><span class="window-dots" aria-hidden="true"><i></i><i></i><i></i></span><span>crabigator — claude</span></div>
                    <div class="desktop-capture term-capture" style="--cols:${heroCapture.cols}">${heroCapture.desktopHtml}</div>
                </div>
                <div class="hero-phone phone-frame" role="img" aria-label="The same Claude Code session in Crabigator’s native iPhone app: the same terminal rows, then the session’s recap, git status, the same roadster scene, its stats with the context window, and reply controls">
                    <div class="phone-screen" aria-hidden="true">
                        <div class="phone-status"><span>9:41</span><span class="phone-island"></span><span class="phone-indicators"><span class="signal-bars"><i></i><i></i><i></i><i></i></span>${iconWifi}<span class="phone-battery"></span></span></div>
                        <div class="phone-app-bar"><span class="phone-back">‹</span><div><strong>${iconCrabigator}${iconCrabigatorWordmark}</strong><span>ClayBook · Live</span></div><span class="phone-type">ᴀA</span></div>
                        <div class="phone-terminal">${heroCapture.phoneHtml}</div>
                        ${appModulesHtml(heroCapture.session, heroCapture.flowHtml, heroCapture.flowSize)}
                        <div class="phone-composer">${iconMicrophone}${iconKeyboard}<span>Message</span></div>
                        <div class="phone-home"><span></span></div>
                    </div>
                </div>
            </figure>
        </section>

        <div class="providers" aria-label="Works with Claude Code, Codex, opencode, and Grok">
            <span>${iconClaude} Claude Code</span><span>${iconCodex} Codex</span><span>${iconOpencode} opencode</span><span>${iconGrok} Grok</span>
        </div>

        <section class="section agent" id="claude-code" aria-labelledby="claude-code-title">
            <h2 id="claude-code-title">Claude Code, unchanged.<span>Crabigator wraps claude, so you’re always up to date.</span></h2>
            ${devicePairHtml({
                capture: claudeCapture, command: 'claude', title: 'Fire scene night look',
                label: 'Claude Code at work in Crabigator, editing the fire scene, with a moving train scene in its status bar. Beside it, the same session in the iPhone app shows the same rows as they arrive and the same train scene.',
            })}
        </section>

        <section class="section agent" id="codex" aria-labelledby="codex-title">
            <h2 id="codex-title">Codex, too.<span>Crabigator wraps codex, opencode, and grok.</span></h2>
            ${devicePairHtml({
                capture: codexCapture, command: 'codex', title: 'Add scene-name uniqueness test', reverse: true,
                label: 'Codex at work in Crabigator, adding a test, with a moving surf scene in its status bar. Beside it, the same session in the iPhone app shows the same rows as they arrive and the same surf scene.',
            })}
        </section>

        <section class="section surfaces" id="apps" aria-labelledby="surfaces-title">
            <h2 id="surfaces-title">Your sessions,<span>on every screen.</span></h2>
            <div class="surface-tabs" role="tablist" aria-label="Screens" hidden>
                <button type="button" role="tab" id="tab-iphone" aria-controls="panel-iphone" aria-selected="true">iPhone</button>
                <button type="button" role="tab" id="tab-ipad" aria-controls="panel-ipad" aria-selected="false" tabindex="-1">iPad</button>
                <button type="button" role="tab" id="tab-android" aria-controls="panel-android" aria-selected="false" tabindex="-1">Android</button>
                <button type="button" role="tab" id="tab-web" aria-controls="panel-web" aria-selected="false" tabindex="-1">Web</button>
            </div>
            <div class="surface-panel" id="panel-iphone" role="tabpanel" aria-labelledby="tab-iphone">
                <p class="surface-claim"><strong>Answer from the notification.</strong> No need to unlock.</p>
                <figure class="notification-preview">${notificationPreviewHtml}</figure>
            </div>
            <div class="surface-panel" id="panel-ipad" role="tabpanel" aria-labelledby="tab-ipad">
                <p class="surface-claim"><strong>Read, scroll back, and reply.</strong> Type or speak your next instruction.</p>
                <figure class="tablet-preview">${tabletPreviewHtml}</figure>
            </div>
            <div class="surface-panel" id="panel-android" role="tabpanel" aria-labelledby="tab-android">
                <p class="surface-claim"><strong>Android, too.</strong> The same terminal, modules, and replies.</p>
                <figure class="android-preview">${androidPreviewHtml}</figure>
            </div>
            <div class="surface-panel" id="panel-web" role="tabpanel" aria-labelledby="tab-web">
                <p class="surface-claim"><strong>Every session in a browser tab.</strong> Nothing to install. <a href="/dashboard" data-track="dashboard_cta" data-label="web_section">Open the dashboard</a></p>
                <figure class="web-preview">${webPreviewHtml}</figure>
            </div>
        </section>

${claimsHtml}
${cardsHtml}

        ${mcpLandingSectionHtml()}


        <section class="section install" id="install">
            <div class="install-heading"><h2>See you outside.<span>macOS, Linux, and Windows.</span></h2></div>
            <div class="install-body">
                <div class="install-command">
                    <code id="install-command">npm install -g crabigator</code>
                    <button type="button" data-copy-target="install-command" aria-label="Copy install command">${iconCopy}</button>
                </div>
                <ol class="install-steps">
                    <li>Install Crabigator with the command above.</li>
                    <li>Run <code>crabigator</code>, <code>crabigator codex</code>, <code>crabigator opencode</code>, or <code>crabigator grok</code>.</li>
                    <li>Open the pairing link on your phone.</li>
                </ol>
            </div>
        </section>
        <section class="section updates" id="mobile">
            <div><h2>Keep in touch.<span>News about the mobile apps.</span></h2></div>
            <form class="email-form" id="email-form">
                <label class="sr-only" for="updates-email">Email address</label>
                <input id="updates-email" name="email" type="email" placeholder="you@example.com" autocomplete="email" required>
                <button type="submit">Subscribe</button>
                <p id="email-success" role="status" hidden>You’re on the list.</p>
            </form>
        </section>
    </main>
    <footer class="footer">
        <a class="wordmark" href="/" aria-label="Crabigator home">${iconCrabigatorLogo}</a>
        <p>Made by <a href="https://samuelclay.com" target="_blank" rel="noopener">Samuel Clay</a> in San Francisco.</p>
        <div class="footer-links"><a href="/dashboard">Dashboard</a><a class="footer-link" href="/mcp-tools">MCP</a><a href="https://github.com/samuelclay/crabigator/releases" target="_blank" rel="noopener">Releases</a><a href="/privacy">Privacy</a></div>
    </footer>
    <p id="copy-status" class="sr-only" role="status"></p>
    <script>${analyticsJs}</script>
    <script>${landingJs}${heroStreamJs}${notificationJs}${flowPlayerJs}</script>
</body>
</html>`;

export function renderLandingHtml(runtime: RuntimeConfig, metaPixelId = ''): string {
    const pixel = runtime.capabilities.marketing_analytics ? metaPixelHtml(metaPixelId) : '';
    const disabledStyles = [
        !runtime.capabilities.mcp ? '#mcp,.footer-link[href="/mcp-tools"]{display:none!important}' : '',
        !runtime.capabilities.marketing_analytics ? '.updates{display:none!important}' : '',
    ].join('');
    let html = usePublicOrigin(landingHtml, runtime.origin)
        .replace('</head>', `${pixel}<style>${disabledStyles}</style></head>`);
    if (!runtime.capabilities.marketing_analytics) {
        html = html.replace(`<script>${analyticsJs}</script>`, '');
    }
    return html;
}
