// Landing page HTML served at /
import { notificationImagePath } from './assets/home-images';
import { faviconPath, faviconIcoPath, touchIconPath } from './assets/home-brand';
import { desktopCaptureHtml, phoneCaptureHtml } from './landing/desktop-capture';
import { tabletPreviewHtml, webPreviewHtml } from './landing/product-previews';
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
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-landing.png">
    <meta property="og:url" content="https://drinkcrabigator.com/">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator - Control Claude Code, Codex, opencode, and Grok from your phone">
    <meta name="twitter:description" content="Answer permissions, approve plans, and respond to questions from your phone.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-landing.png">
    <link rel="icon" href="${faviconPath}" type="image/svg+xml">
    <link rel="icon" href="${faviconIcoPath}" sizes="16x16 32x32 48x48">
    <link rel="apple-touch-icon" href="${touchIconPath}" sizes="192x192">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;450;500;550;600;650;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap" rel="stylesheet">
    <style>${mcpCss}${homeCss}${productPreviewsCss}${heroStreamCss}</style>
</head>
<body>
    <header class="site-header">
        <a class="wordmark" href="/" aria-label="Crabigator home">${iconCrabigatorLogo}</a>
        <nav aria-label="Main navigation">
            <a href="#native">Apps</a>
            <a href="#web">Web</a>
            <a href="#install">Install</a>
            <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener">GitHub</a>
            <a class="dashboard-link" href="/dashboard" data-track="dashboard_nav" data-label="nav">Open dashboard</a>
        </nav>
    </header>
    <main id="main">
        <section class="hero" aria-labelledby="hero-title">
            <div class="hero-copy">
                <div class="hero-message">
                    <h1 id="hero-title">Take your agents<br>with you.</h1>
                    <p class="hero-description">Your desktop session, live in a native app.</p>
                </div>
                <div class="hero-start">
                    <a class="install-link" href="#install" data-track="install_cta" data-label="hero">Install Crabigator <span aria-hidden="true">↗</span></a>
                    <p>For Claude Code, Codex, opencode &amp; Grok.</p>
                </div>
            </div>
            <figure class="hero-screens" aria-label="The same Claude Code session on desktop and iPhone">
                <div class="desktop-frame">
                    <div class="desktop-bar"><span class="window-dots" aria-hidden="true"><i></i><i></i><i></i></span><span>Claude Code</span><span>~/projects/crabigator</span></div>
                    <div class="desktop-capture">${desktopCaptureHtml}</div>
                </div>
                <div class="hero-phone phone-frame" role="img" aria-label="The same Claude Code session in Crabigator’s native iPhone app, with a live connection and reply controls">
                    <div class="phone-screen" aria-hidden="true">
                        <div class="phone-status"><span>9:41</span><span class="phone-island"></span><span class="phone-indicators"><span class="signal-bars"><i></i><i></i><i></i><i></i></span>${iconWifi}<span class="phone-battery"></span></span></div>
                        <div class="phone-app-bar"><span class="phone-back">‹</span><div><strong>${iconCrabigator}${iconCrabigatorWordmark}</strong><span>ClayBook · Live</span></div><span class="phone-type">ᴀA</span></div>
                        <div class="phone-terminal">${phoneCaptureHtml}</div>
                        <div class="phone-composer">${iconMicrophone}${iconKeyboard}<span>Message</span></div>
                        <div class="phone-home"><span></span></div>
                    </div>
                </div>
                <figcaption><span>On your desktop</span><span class="connection-line" aria-hidden="true"><span class="stream-packet"></span></span><span>On your phone</span><button class="hero-stream-toggle" type="button" aria-label="Pause terminal animation" hidden>Ⅱ</button></figcaption>
            </figure>
        </section>

        <div class="providers" aria-label="Works with Claude Code, Codex, opencode, and Grok">
            <span>${iconClaude} Claude Code</span><span>${iconCodex} Codex</span><span>${iconOpencode} opencode</span><span>${iconGrok} Grok</span>
        </div>

        <section class="native section" id="native">
            <div class="story-heading">
                <h2>The terminal you know.<br>Made for your hands.</h2>
                <div><p>Native apps for your phone and tablet. Read the live terminal, scroll back, and type or speak your next instruction.</p><p class="availability">iOS and Android apps in development.</p></div>
            </div>
            <figure class="tablet-preview">
                ${tabletPreviewHtml}
                <figcaption>Crabigator on iPad</figcaption>
            </figure>
        </section>

        <section class="notifications section" id="notifications">
            <div class="notification-copy">
                <h2>A tap.<br>And it’s moving again.</h2>
                <p>When your agent needs a decision, answer from the notification. No trip back to your desk.</p>
                <p class="availability">Native app notifications</p>
            </div>
            <figure class="notification-preview"><div class="notification-crop"><img src="${notificationImagePath}" width="750" height="1334" loading="lazy" alt="A Crabigator design-decision notification with Compact rows, Expanded cards, and Adaptive layout as native answer choices"></div></figure>
        </section>

        <section class="web section" id="web">
            <div class="story-heading">
                <h2>Or open a browser.</h2>
                <div><p>The web dashboard puts sessions, pull requests, and terminal controls in one place. Use it from any device, without installing an app.</p><a class="text-link" href="/dashboard" data-track="dashboard_cta" data-label="web_section">Open web dashboard</a></div>
            </div>
            <figure class="web-preview">
                ${webPreviewHtml}
                <figcaption>Web dashboard</figcaption>
            </figure>
        </section>

        ${mcpLandingSectionHtml()}

        <section class="section ownership" id="security">
            <h2>Still your terminal.<br>Still your machine.</h2>
            <div class="ownership-copy">
                <p>Your agents run on your computer. Crabigator streams their terminal output and sends your replies back.</p>
                <p>Written in Rust. MIT licensed. Use the hosted dashboard or run your own.</p>
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener">Read the source</a>
            </div>
        </section>

        <section class="section install" id="install">
            <div class="install-heading"><h2>See you outside.</h2><p>macOS, Linux, and Windows.</p></div>
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
            <div><h2>Keep in touch.</h2><p>New features and news about the mobile apps.</p></div>
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
        <div class="footer-links"><a href="/dashboard">Dashboard</a><a class="footer-link" href="/mcp-tools">MCP</a><a href="https://github.com/samuelclay/crabigator/releases" target="_blank" rel="noopener">Releases</a></div>
    </footer>
    <p id="copy-status" class="sr-only" role="status"></p>
    <script>${analyticsJs}</script>
    <script>${landingJs}${heroStreamJs}</script>
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
