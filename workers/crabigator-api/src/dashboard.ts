import { faviconPath, faviconIcoPath, touchIconPath } from './assets/home-brand';
// Dashboard HTML served at /dashboard
import { dashboardCss } from './dashboard/css';
import { dashboardJs } from './dashboard/js';
import { brandCrabSvg, brandLogoSvg, iconChevronRight, iconPerson, iconSidebar, iconSliders } from './dashboard/icons';
import type { RuntimeConfig } from './config';
import { metaPixelHtml, usePublicOrigin } from './html-render';

export const dashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>Crabigator Dashboard</title>
    <meta name="description" content="Real-time monitoring dashboard for your Claude Code, Codex, opencode, and Grok sessions.">
    <meta property="og:title" content="Crabigator Dashboard">
    <meta property="og:description" content="Real-time monitoring dashboard for your Claude Code, Codex, opencode, and Grok sessions.">
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-dashboard.png?v=0e52372806">
    <meta property="og:url" content="https://drinkcrabigator.com/dashboard">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator Dashboard">
    <meta name="twitter:description" content="Real-time monitoring dashboard for your Claude Code, Codex, opencode, and Grok sessions.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-dashboard.png?v=0e52372806">
    <link rel="icon" type="image/svg+xml" href="${faviconPath}">
    <link rel="icon" href="${faviconIcoPath}" sizes="16x16 32x32 48x48">
    <link rel="apple-touch-icon" href="${touchIconPath}" sizes="192x192">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=DM+Sans:wght@400;500;600;700&display=swap">
    <style>${dashboardCss}</style>
</head>
<body>
    <div class="deploy-overlay" id="deploy-overlay">
        <div class="deploy-spinner"></div>
        <div class="deploy-text">Reconnecting to Crabigator...</div>
        <div class="deploy-subtext">A new version was deployed</div>
        <div class="deploy-countdown" id="deploy-countdown"></div>
    </div>
    <div class="header">
        <div class="header-main">
            <h1><a class="dashboard-brand" href="/" aria-label="Crabigator home">${brandLogoSvg}${brandCrabSvg}</a><span class="dashboard-label">Dashboard</span></h1>
            <button class="refresh-btn" onclick="loadSessions()" aria-label="Refresh sessions" title="Refresh sessions"><span class="header-control-label">↻ Refresh</span></button>
            <div class="filter-indicator" id="filter-indicator">
                <span class="filter-text">Viewing 1 session</span>
                <button class="filter-clear" onclick="clearSessionFilter()" title="Show all sessions">✕</button>
            </div>
            <div class="sessions-container">
                <button class="pr-board-btn active" id="session-list-btn" onclick="showSessionList()" title="Session list">List</button>
                <button class="pr-board-btn" id="pr-board-btn" onclick="togglePrBoard()" title="Cross-session PR board">
                    <span class="pr-board-btn-icon">⑆</span> PRs
                </button>
                <button class="pr-board-btn" id="conductor-btn" onclick="showConductor()">Conductor</button>
                <button class="sessions-btn" id="sessions-btn" aria-label="Loading sessions" aria-busy="true" aria-expanded="false" aria-haspopup="true">
                    <span class="sessions-count" id="sessions-count">Loading</span>
                    <span class="sessions-label">sessions</span>
                </button>
            </div>
            <div class="settings-container">
                <button class="settings-btn" id="settings-btn" aria-label="Account" title="Account" aria-haspopup="dialog" aria-expanded="false" aria-controls="settings-popover" onclick="toggleSettingsPopover()">
                    ${iconPerson}
                    <span class="header-control-label">Account</span>
                </button>
                <div class="settings-popover" id="settings-popover" role="dialog" aria-labelledby="account-title" tabindex="-1">
                    <div class="account-head">
                        <h2 id="account-title">Account</h2>
                        <button type="button" class="vp-done" onclick="closeSettingsPopover(true)">Done</button>
                    </div>
                    <section class="account-group" id="subscription-section" hidden>
                        <h3>Subscription</h3>
                        <div class="account-card">
                            <p class="account-text">Crabigator is now free. This account still has a paid subscription; cancel it any time.</p>
                            <button type="button" class="account-row manage-subscription-link" onclick="openSubscriptionPortal()">
                                <span>Manage subscription</span>
                                ${iconChevronRight}
                            </button>
                        </div>
                    </section>
                    <section class="account-group" id="account-logins-section">
                        <h3>Sign-in</h3>
                        <div class="account-card">
                            <p class="account-text" id="account-logins-status">Pairing code only. Connect GitHub or Google to use the same account on MCP.</p>
                            <div id="account-identities" class="account-identities"></div>
                            <div class="account-connect-row" id="account-connect-row"></div>
                        </div>
                    </section>
                    <section class="account-group" id="mcp-section">
                        <h3>Devices and agents</h3>
                        <div class="account-card">
                            <button type="button" class="account-row account-action" id="generate-invite-btn" onclick="generateInviteCode()">Pair another device</button>
                            <div id="invite-result" class="invite-result"></div>
                            <div class="account-block">
                                <span>Connect an agent with MCP</span>
                                <code id="mcp-url"></code>
                                <a href="/mcp-tools">Tools and examples</a>
                            </div>
                        </div>
                    </section>
                    <section class="account-group">
                        <h3>This device</h3>
                        <div class="account-card">
                            <div class="account-block">
                                <span>This browser</span>
                                <span class="account-muted" id="account-device-meta"></span>
                            </div>
                            <a class="account-row" href="/privacy">Privacy policy</a>
                            <button type="button" class="account-row account-danger" onclick="confirmUnpair()">Unpair this device</button>
                        </div>
                    </section>
                    <p class="account-note">Nothing here changes how a screen looks. Use View on the List, the PR board or an open session for that.</p>
                </div>
            </div>
        </div>
        <div class="filter-bar" id="filter-bar" hidden>
            <div class="filter-chips" id="filter-chips" role="group" aria-label="Which sessions are shown"></div>
            <button type="button" class="view-btn" id="view-btn" aria-haspopup="dialog" aria-expanded="false" aria-controls="view-panel" onclick="toggleViewPanel('auto', this)" title="How this view looks">${iconSliders}<span>View</span></button>
        </div>
    </div>
    <div class="dashboard-layout" id="dashboard-layout" data-sidebar-position="left">
        <div class="sidebar" id="sidebar">
            <div class="sidebar-header">
                <span class="sidebar-title">Sessions</span>
                <button class="sidebar-pin-btn" id="sidebar-pin-btn" onclick="pinSidebar()" title="Keep the session list open beside the List">
                    ${iconSidebar}
                    Sidebar
                </button>
                <button class="sidebar-close-btn" id="sidebar-close-btn" onclick="closeSidebar()" title="Hide sidebar">✕</button>
            </div>
            <div class="sidebar-content" id="sidebar-content"></div>
            <div class="sidebar-resize-handle" id="sidebar-resize-handle"></div>
        </div>
        <div class="sidebar-backdrop" id="sidebar-backdrop"></div>
        <div class="container" id="sessions" data-layout="1" data-grouping="all"></div>
        <div class="filter-note" id="filter-note" role="status" hidden></div>
        <div id="sessions-parked" hidden></div>
        <div class="pr-board" id="pr-board-view" hidden></div>
        <section id="conductor-view" hidden aria-label="Conductor"></section>
    </div>
    <div class="chip-menu" id="chip-menu" role="menu" hidden></div>
    <div class="view-panel" id="view-panel" role="dialog" tabindex="-1" popover="manual"></div>

    <script>${dashboardJs}</script>
</body>
</html>`;

export function renderDashboardHtml(runtime: RuntimeConfig, metaPixelId = ''): string {
    const browserConfig = JSON.stringify({
        capabilities: runtime.capabilities,
        social_providers: runtime.social_providers,
    }).replace(/</g, '\\u003c');
    const disabledStyles = [
        !runtime.capabilities.transcription
            ? '.voice-btn,.voice-cancel-btn,.voice-actions,.voice-overlay{display:none!important}'
            : '',
        !runtime.capabilities.social_login ? '#account-logins-section,.social-login-stack,.pairing-divider{display:none!important}' : '',
    ].join('');
    const pixel = runtime.capabilities.marketing_analytics ? metaPixelHtml(metaPixelId) : '';

    return usePublicOrigin(dashboardHtml, runtime.origin)
        .replace('</head>', `${pixel}<script>window.CRABIGATOR_CONFIG=${browserConfig};</script><style>${disabledStyles}</style></head>`);
}
