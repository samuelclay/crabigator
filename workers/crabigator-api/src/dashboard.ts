// Dashboard HTML served at /dashboard
import { dashboardCss } from './dashboard/css';
import { dashboardJs } from './dashboard/js';
import { faviconSvg, iconChevronRight } from './dashboard/icons';
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
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-dashboard.png">
    <meta property="og:url" content="https://drinkcrabigator.com/dashboard">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator Dashboard">
    <meta name="twitter:description" content="Real-time monitoring dashboard for your Claude Code, Codex, opencode, and Grok sessions.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-dashboard.png">
    <link rel="icon" href="data:image/svg+xml,${faviconSvg}">
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Space+Grotesk:wght@400;500;600;700&display=swap">
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
        <h1>🦀 Crabigator Dashboard</h1>
        <button class="refresh-btn" onclick="loadSessions()">↻ Refresh</button>
        <div class="filter-indicator" id="filter-indicator">
            <span class="filter-text">Viewing 1 session</span>
            <button class="filter-clear" onclick="clearSessionFilter()" title="Show all sessions">✕</button>
        </div>
        <div class="sessions-container">
            <button class="pr-board-btn" id="pr-board-btn" onclick="togglePrBoard()" title="Cross-session PR board">
                <span class="pr-board-btn-icon">⑆</span> PRs
            </button>
            <button class="sessions-btn" id="sessions-btn" aria-label="Loading sessions" aria-busy="true" aria-expanded="false" aria-haspopup="true">
                <span class="sessions-count" id="sessions-count">Loading</span>
                <span class="sessions-label">sessions</span>
            </button>
        </div>
        <div class="style-container">
            <button class="style-btn" id="style-btn" onclick="toggleStylePopover()">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="3"/>
                    <path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/>
                </svg>
                Style
            </button>
            <div class="style-popover" id="style-popover">
                <div class="style-section">
                    <div class="style-section-label">Columns</div>
                    <div class="style-options">
                        <button class="style-option active" data-layout="1" onclick="setLayout('1')">1</button>
                        <button class="style-option" data-layout="2" onclick="setLayout('2')">2</button>
                        <button class="style-option" data-layout="3" onclick="setLayout('3')">3</button>
                        <button class="style-option" data-layout="4" onclick="setLayout('4')">4</button>
                        <button class="style-option" data-layout="fit" onclick="setLayout('fit')">Fit</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Text Size</div>
                    <div class="font-size-control">
                        <button class="font-size-btn decrease" onclick="adjustFontSize(-1)" title="Smaller">A</button>
                        <div class="font-size-value" id="font-label">100%</div>
                        <button class="font-size-btn increase" onclick="adjustFontSize(1)" title="Larger">A</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Terminal Height</div>
                    <div class="font-size-control">
                        <button class="font-size-btn decrease" onclick="adjustTerminalHeight(-1)" title="Shorter">−</button>
                        <div class="font-size-value" id="height-label">350px</div>
                        <button class="font-size-btn increase" onclick="adjustTerminalHeight(1)" title="Taller">+</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Text Wrap</div>
                    <div class="style-options">
                        <button class="style-option active" data-wrap="wrap" onclick="setTerminalWrap(true)">Wrap</button>
                        <button class="style-option" data-wrap="scroll" onclick="setTerminalWrap(false)">Scroll</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Widgets</div>
                    <div class="style-options">
                        <button class="style-option active" data-widgets="expanded" onclick="setWidgetsExpanded(true)">Expanded</button>
                        <button class="style-option" data-widgets="collapsed" onclick="setWidgetsExpanded(false)">Collapsed</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Grouping</div>
                    <div class="style-options">
                        <button class="style-option active" data-grouping="all" onclick="setGrouping('all')">All</button>
                        <button class="style-option" data-grouping="project" onclick="setGrouping('project')">By Project</button>
                    </div>
                </div>
                <div class="style-section" id="project-order-section" style="display:none">
                    <div class="style-section-label">Project Order</div>
                    <div class="style-options">
                        <button class="style-option active" data-project-order="recent" onclick="setProjectOrder('recent')">Most Recent</button>
                        <button class="style-option" data-project-order="alpha" onclick="setProjectOrder('alpha')">Alphabetical</button>
                    </div>
                </div>
                <div class="style-section">
                    <div class="style-section-label">Visible Sections</div>
                    <div class="sidebar-checkbox-list">
                        <label class="sidebar-checkbox"><input type="checkbox" id="section-recap" onchange="toggleSection('recap')" checked><span class="sidebar-checkbox-icon" style="color:#22d3ee">◈</span><span class="sidebar-checkbox-label">Recap</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="section-prs" onchange="toggleSection('prs')" checked><span class="sidebar-checkbox-icon" style="color:#a371f7">⇄</span><span class="sidebar-checkbox-label">Pull requests</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="section-commits" onchange="toggleSection('commits')" checked><span class="sidebar-checkbox-icon" style="color:#db6d28">●</span><span class="sidebar-checkbox-label">Commits</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="section-git" onchange="toggleSection('git')" checked><span class="sidebar-checkbox-icon" style="color:#7ee787">⎇</span><span class="sidebar-checkbox-label">Git status</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="section-changes" onchange="toggleSection('changes')" checked><span class="sidebar-checkbox-icon" style="color:#ec4899">Δ</span><span class="sidebar-checkbox-label">Changes</span></label>
                    </div>
                </div>
            </div>
        </div>
        <div class="settings-container">
            <button class="settings-btn" id="settings-btn" onclick="toggleSettingsPopover()">
                <svg viewBox="0 0 16 16" fill="currentColor">
                    <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm2-3a2 2 0 11-4 0 2 2 0 014 0zm4 8c0 1-1 1-1 1H3s-1 0-1-1 1-4 6-4 6 3 6 4zm-1-.004c0-.246-.178-.987-.985-1.728C11.267 10.564 9.95 10 8 10s-3.267.564-4.015 1.268C3.178 12.01 3 12.75 3 12.996h10z"/>
                </svg>
                Account
            </button>
            <div class="settings-popover" id="settings-popover">
                <div class="settings-section" id="subscription-section" hidden>
                    <div class="settings-section-label">Subscription</div>
                    <p class="settings-description">Crabigator is now free. This account still has a paid subscription; cancel it any time.</p>
                    <button class="manage-subscription-link" onclick="openSubscriptionPortal()">
                        Manage subscription
                        ${iconChevronRight}
                    </button>
                </div>
                <div class="settings-divider" id="subscription-divider" hidden></div>
                <div class="settings-section" id="account-logins-section">
                    <div class="settings-section-label">Sign-in</div>
                    <p class="settings-description" id="account-logins-status">Pairing code only. Connect GitHub or Google to use the same account on MCP.</p>
                    <div id="account-identities" class="account-identities"></div>
                    <div class="account-connect-row" id="account-connect-row"></div>
                </div>
                <div class="settings-divider"></div>
                <div class="settings-section" id="mcp-section">
                    <div class="settings-section-label">MCP</div>
                    <p class="settings-description">Connect Claude, Cursor, Grok, or another agent to this account. Sign in with GitHub or Google when the client opens a browser.</p>
                    <p class="settings-description"><code id="mcp-url"></code></p>
                    <p class="settings-description"><a href="/mcp-tools">See every tool and example output</a></p>
                </div>
                <div class="settings-divider"></div>
                <div class="settings-section">
                    <div class="settings-section-label">Pair another device</div>
                    <p class="settings-description">Generate a code to pair another phone, tablet, or browser.</p>
                    <button class="settings-action-btn" id="generate-invite-btn" onclick="generateInviteCode()">
                        Generate pairing code
                    </button>
                    <div id="invite-result" class="invite-result"></div>
                </div>
                <div class="settings-divider"></div>
                <div class="settings-section">
                    <div class="settings-section-label">This device</div>
                    <p class="settings-description">Removes access from this browser. You'll need a new pairing code to reconnect.</p>
                    <button class="settings-danger-btn" onclick="clearPairing()">
                        Unpair this device
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div class="dashboard-layout" id="dashboard-layout" data-sidebar-position="left">
        <div class="sidebar" id="sidebar">
            <div class="sidebar-header">
                <span class="sidebar-title">Sessions</span>
                <button class="sidebar-settings-btn" onclick="toggleSidebarSettings()" title="Sidebar settings">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
                        <circle cx="12" cy="12" r="3"/>
                        <path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/>
                    </svg>
                    Settings
                </button>
                <button class="sidebar-pin-btn" id="sidebar-pin-btn" onclick="pinSidebar()" title="Pin as sidebar">
                    <svg viewBox="0 0 256 256" fill="currentColor" width="12" height="12"><path d="M216,40H40A16,16,0,0,0,24,56V200a16,16,0,0,0,16,16H216a16,16,0,0,0,16-16V56A16,16,0,0,0,216,40ZM40,56H80V200H40ZM216,200H96V56H216V200Z"/></svg>
                    Sidebar
                </button>
                <button class="sidebar-close-btn" id="sidebar-close-btn" onclick="closeSidebar()" title="Hide sidebar">✕</button>
            </div>
            <div class="sidebar-settings-popover" id="sidebar-settings-popover">
                <div class="sidebar-settings-section">
                    <div class="sidebar-settings-label">Position</div>
                    <div class="style-options">
                        <button class="style-option sb-opt-position" data-position="left" onclick="setSidebarPosition('left')">Left</button>
                        <button class="style-option sb-opt-position" data-position="right" onclick="setSidebarPosition('right')">Right</button>
                    </div>
                </div>
                <div class="sidebar-settings-section">
                    <div class="sidebar-settings-label">Density</div>
                    <div class="style-options">
                        <button class="style-option sb-opt-density" data-density="compact" onclick="setSidebarDensity('compact')">Compact</button>
                        <button class="style-option sb-opt-density" data-density="comfortable" onclick="setSidebarDensity('comfortable')">Comfortable</button>
                    </div>
                </div>
                <div class="sidebar-settings-section">
                    <div class="sidebar-settings-label">Click Action</div>
                    <div class="style-options">
                        <button class="style-option sb-opt-click" data-click="focus" onclick="setSessionClickAction('focus')">Focus</button>
                        <button class="style-option sb-opt-click" data-click="scroll" onclick="setSessionClickAction('scroll')">Scroll</button>
                    </div>
                </div>
                <div class="sidebar-settings-section">
                    <div class="sidebar-settings-label">Visible Stats</div>
                    <div class="sidebar-checkbox-list">
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-sessionTime" onchange="toggleSidebarStat('sessionTime')"><span class="sidebar-checkbox-icon" style="color:#58a6ff">◉</span><span class="sidebar-checkbox-label">Session time</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-thinkingTime" onchange="toggleSidebarStat('thinkingTime')"><span class="sidebar-checkbox-icon" style="color:#3fb950">◐</span><span class="sidebar-checkbox-label">Thinking time</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-prompts" onchange="toggleSidebarStat('prompts')"><span class="sidebar-checkbox-icon" style="color:#8b949e">⟩</span><span class="sidebar-checkbox-label">Prompt count</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-promptRecency" onchange="toggleSidebarStat('promptRecency')"><span class="sidebar-checkbox-icon" style="color:#8b949e">◷</span><span class="sidebar-checkbox-label">Last prompt</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-completions" onchange="toggleSidebarStat('completions')"><span class="sidebar-checkbox-icon" style="color:#8b949e">⋖</span><span class="sidebar-checkbox-label">Completion count</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-completionRecency" onchange="toggleSidebarStat('completionRecency')"><span class="sidebar-checkbox-icon" style="color:#8b949e">◷</span><span class="sidebar-checkbox-label">Last completion</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-tools" onchange="toggleSidebarStat('tools')"><span class="sidebar-checkbox-icon" style="color:#f0883e">⚒</span><span class="sidebar-checkbox-label">Tools</span></label>
                        <label class="sidebar-checkbox"><input type="checkbox" id="sb-stat-compactions" onchange="toggleSidebarStat('compactions')"><span class="sidebar-checkbox-icon" style="color:#e879f9">⊜</span><span class="sidebar-checkbox-label">Compactions</span></label>
                    </div>
                </div>
            </div>
            <div class="sidebar-content" id="sidebar-content"></div>
            <div class="sidebar-resize-handle" id="sidebar-resize-handle"></div>
        </div>
        <div class="sidebar-backdrop" id="sidebar-backdrop"></div>
        <div class="container" id="sessions" data-layout="1" data-grouping="all"></div>
        <div class="pr-board" id="pr-board-view" hidden></div>
    </div>

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
