// Dashboard HTML served at /dashboard
import { dashboardCss } from './dashboard/css';
import { dashboardJs } from './dashboard/js';
import { faviconSvg } from './dashboard/icons';

export const dashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>Crabigator Dashboard</title>
    <meta name="description" content="Real-time monitoring dashboard for your Claude Code sessions.">
    <meta property="og:title" content="Crabigator Dashboard">
    <meta property="og:description" content="Real-time monitoring dashboard for your Claude Code sessions.">
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-dashboard.png">
    <meta property="og:url" content="https://drinkcrabigator.com/dashboard">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator Dashboard">
    <meta name="twitter:description" content="Real-time monitoring dashboard for your Claude Code sessions.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-dashboard.png">
    <link rel="icon" href="data:image/svg+xml,${faviconSvg}">
    <style>${dashboardCss}</style>
    <!-- Meta Pixel Code -->
    <script>
    !function(f,b,e,v,n,t,s)
    {if(f.fbq)return;n=f.fbq=function(){n.callMethod?
    n.callMethod.apply(n,arguments):n.queue.push(arguments)};
    if(!f._fbq)f._fbq=n;n.push=n;n.loaded=!0;n.version='2.0';
    n.queue=[];t=b.createElement(e);t.async=!0;
    t.src=v;s=b.getElementsByTagName(e)[0];
    s.parentNode.insertBefore(t,s)}(window, document,'script',
    'https://connect.facebook.net/en_US/fbevents.js');
    fbq('init', '386521575130337');
    fbq('track', 'PageView');
    </script>
    <noscript><img height="1" width="1" style="display:none"
    src="https://www.facebook.com/tr?id=386521575130337&ev=PageView&noscript=1"
    /></noscript>
    <!-- End Meta Pixel Code -->
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
            <button class="sessions-btn" id="sessions-btn" onclick="toggleSessionsPopover()">
                <span class="sessions-count" id="sessions-count">0</span>
                <span class="sessions-label">sessions</span>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="12" height="12">
                    <path d="M6 9l6 6 6-6"/>
                </svg>
            </button>
            <div class="sessions-popover" id="sessions-popover">
                <div class="sessions-popover-content" id="sessions-popover-content"></div>
            </div>
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
                <div class="settings-section usage-display" id="usage-display">
                    <div class="usage-header">
                        <span class="usage-label">Usage Today</span>
                        <span class="usage-time" id="usage-time">--:-- left</span>
                    </div>
                    <div class="usage-bar-container">
                        <div class="usage-bar" id="usage-bar" style="width: 0%"></div>
                    </div>
                    <button class="upgrade-btn" id="upgrade-btn" onclick="showUpgradeModal()">
                        Upgrade to Pro
                    </button>
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
    <div class="container" id="sessions" data-layout="1" data-grouping="all"></div>

    <!-- Gift Claim Overlay -->
    <div class="gift-overlay" id="gift-overlay">
        <div class="gift-modal">
            <div class="gift-content" id="gift-content">
                <div class="gift-icon">🎁</div>
                <h2 class="gift-title">You've Been Gifted!</h2>
                <p class="gift-subtitle">Someone has sent you a gift subscription to Crabigator Pro.</p>
                <div class="gift-duration">
                    <div class="gift-duration-label">Gift Duration</div>
                    <div class="gift-duration-value" id="gift-duration-text">Month</div>
                </div>
                <div class="gift-code-label">Gift Code: <span class="gift-code-value" id="gift-code-display">XXXXXXXX</span></div>
                <button class="gift-claim-btn" id="gift-claim-btn" onclick="claimGift()">
                    Claim Your Gift
                </button>
                <button class="gift-dismiss" onclick="dismissGift()">Maybe later</button>
            </div>
            <div class="gift-loading" id="gift-loading">
                <div class="gift-loading-spinner"></div>
                <div class="gift-loading-text">Loading gift...</div>
            </div>
            <div class="gift-success" id="gift-success">
                <div class="gift-success-icon">✓</div>
                <div class="gift-success-text" id="gift-success-text">Gift claimed successfully!</div>
            </div>
            <div class="gift-error" id="gift-error">
                <div class="gift-error-icon">😢</div>
                <div class="gift-error-text" id="gift-error-text">Something went wrong</div>
                <button class="gift-retry-btn" onclick="closeGiftOverlay()">Close</button>
            </div>
        </div>
    </div>

    <!-- Paywall Overlay -->
    <div class="paywall-overlay" id="paywall-overlay">
        <div class="paywall-modal">
            <div class="paywall-content" id="paywall-content">
                <div class="paywall-icon">🦀</div>
                <h2 class="paywall-title" id="paywall-title">Upgrade to Pro</h2>
                <p class="paywall-subtitle" id="paywall-subtitle">Get unlimited dashboard access and never worry about limits again.</p>
                <div class="paywall-usage" id="paywall-usage-section" style="display: none;">
                    <span class="paywall-usage-text"><span class="used" id="paywall-usage">10:00</span> / 10:00 used today</span>
                </div>
                <div class="paywall-price">$3</div>
                <div class="paywall-price-period">per month</div>
                <div class="paywall-features">
                    <div class="paywall-feature">
                        <span class="paywall-feature-icon">✓</span>
                        <span>Unlimited dashboard viewing</span>
                    </div>
                    <div class="paywall-feature">
                        <span class="paywall-feature-icon">✓</span>
                        <span>Real-time session monitoring</span>
                    </div>
                    <div class="paywall-feature">
                        <span class="paywall-feature-icon">✓</span>
                        <span>Answer permissions from anywhere</span>
                    </div>
                    <div class="paywall-feature">
                        <span class="paywall-feature-icon">✓</span>
                        <span>Multi-device support</span>
                    </div>
                </div>
                <div class="paywall-buttons">
                    <button class="paywall-btn stripe" id="paywall-stripe-btn" onclick="initiateStripePayment()">
                        Pay with Card
                    </button>
                    <button class="paywall-btn paypal" id="paywall-paypal-btn" onclick="initiatePayPalPayment()">
                        Pay with PayPal
                    </button>
                </div>
                <button class="paywall-dismiss" onclick="dismissPaywall()">Maybe later</button>
            </div>
            <div class="paywall-loading" id="paywall-loading">
                <div class="paywall-loading-spinner"></div>
                <div class="paywall-loading-text">Verifying payment...</div>
            </div>
            <div class="paywall-success" id="paywall-success">
                <div class="paywall-success-icon">✓</div>
                <div class="paywall-success-text">Welcome to Pro!</div>
            </div>
            <div class="paywall-error" id="paywall-error">
                <div class="paywall-error-text" id="paywall-error-text">Payment processing failed. Please try again.</div>
                <button class="paywall-retry-btn" onclick="showPaywallContent()">Try Again</button>
            </div>
        </div>
    </div>

    <script>${dashboardJs}</script>
</body>
</html>`;
