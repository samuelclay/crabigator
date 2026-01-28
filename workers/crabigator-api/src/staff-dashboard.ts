/**
 * Staff-only telemetry dashboard
 * Shows telemetry data in charts and tables with real-time updates
 */

import { faviconStaffSvg, iconCrabigator, iconGift } from './dashboard/icons';
import { staffDashboardCss } from './staff-dashboard/css';
import { staffDashboardJs } from './staff-dashboard/js';

export const staffDashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator Staff Dashboard</title>
    <link rel="icon" href="data:image/svg+xml,${faviconStaffSvg}">
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"></script>
    <style>
${staffDashboardCss}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1><span class="logo-icon">${iconCrabigator}</span> Crabigator Staff Dashboard</h1>
            <div class="status">
                <div class="status-dot"></div>
                <span>Live</span>
                <span id="last-update"></span>
            </div>
        </header>

        <!-- App Telemetry Section -->
        <div class="collapsible-section" id="section-telemetry" data-section="telemetry">
            <div class="section-header" onclick="toggleSection('telemetry')">
                <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 9l6 6 6-6"/>
                </svg>
                <div class="section-name">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <rect x="2" y="3" width="20" height="14" rx="2"/>
                        <path d="M8 21h8M12 17v4"/>
                    </svg>
                    App Telemetry
                </div>
                <div class="section-summary">
                    <span class="summary-item">24h: <span class="summary-value" id="sum-checks-24h">-</span> checks</span>
                    <span class="summary-item">all: <span class="summary-value" id="sum-checks-all">-</span></span>
                    <span class="summary-item"><span class="summary-value" id="sum-devices-all">-</span> devices</span>
                    <span class="summary-item">v<span class="summary-value" id="sum-version">-</span></span>
                </div>
            </div>
            <div class="section-content">
                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-label">Total Devices</div>
                        <div class="stat-value" id="total-devices">-</div>
                        <div class="stat-change" id="devices-change"></div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-label">Update Checks (24h)</div>
                        <div class="stat-value" id="checks-24h">-</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-label">Update Checks (7d)</div>
                        <div class="stat-value" id="checks-7d">-</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-label">Latest Version</div>
                        <div class="stat-value" id="latest-version">-</div>
                    </div>
                </div>

                <div class="charts-grid">
                    <div class="chart-card">
                        <div class="chart-title">Update Checks Over Time</div>
                        <div class="chart-container">
                            <canvas id="checks-chart"></canvas>
                        </div>
                    </div>
                    <div class="chart-card">
                        <div class="chart-title">Version Distribution</div>
                        <div class="chart-container">
                            <canvas id="version-chart"></canvas>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- Website Analytics Section -->
        <div class="collapsible-section" id="section-analytics" data-section="analytics">
            <div class="section-header" onclick="toggleSection('analytics')">
                <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 9l6 6 6-6"/>
                </svg>
                <div class="section-name">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 20V10M12 20V4M6 20v-6"/>
                    </svg>
                    Website Analytics
                </div>
                <div class="section-summary">
                    <span class="summary-item">24h: <span class="summary-value" id="sum-visitors-24h">-</span> visitors</span>
                    <span class="summary-item">all: <span class="summary-value" id="sum-visitors-all">-</span></span>
                    <span class="summary-item"><span class="summary-value" id="sum-signups-all">-</span> signups</span>
                </div>
            </div>
            <div class="section-content">
                <div class="analytics-section">
                    <div class="section-title">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M18 20V10M12 20V4M6 20v-6"/>
                        </svg>
                        Website Analytics
                        <span id="analytics-update" class="status" style="font-size: 12px; margin-left: auto;"></span>
                    </div>

                    <!-- Summary Stats -->
            <div class="analytics-stats-grid">
                <div class="stat-card">
                    <div class="stat-label">Visitors (24h)</div>
                    <div class="stat-value" id="analytics-visitors-24h">-</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">Page Views (24h)</div>
                    <div class="stat-value" id="analytics-pageviews-24h">-</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">Avg Time on Page</div>
                    <div class="stat-value" id="analytics-avg-time">-</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">Bounce Rate</div>
                    <div class="stat-value" id="analytics-bounce-rate">-</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">Avg Scroll Depth</div>
                    <div class="stat-value" id="analytics-scroll-depth">-</div>
                </div>
            </div>

            <!-- Charts Row 1: Visitors Over Time + Traffic Sources -->
            <div class="analytics-charts-row">
                <div class="chart-card">
                    <div class="chart-title">Visitors Over Time (30 days)</div>
                    <div class="chart-container">
                        <canvas id="visitors-chart"></canvas>
                    </div>
                </div>
                <div class="chart-card">
                    <div class="chart-title">Traffic Sources</div>
                    <div class="chart-container">
                        <canvas id="sources-chart"></canvas>
                    </div>
                </div>
            </div>

            <!-- Referrer Details Table -->
            <div class="table-container" style="margin-bottom: 24px;">
                <div class="table-header">
                    <div class="table-title">Referrer Details (30 days)</div>
                    <div class="table-count" id="referrer-count">0 domains</div>
                </div>
                <div style="max-height: 300px; overflow-y: auto;">
                    <table>
                        <thead>
                            <tr><th>Category</th><th>Domain</th><th style="text-align: right;">Visitors</th></tr>
                        </thead>
                        <tbody id="referrer-tbody">
                            <tr><td colspan="3" class="loading">Loading...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Charts Row 2: Device/Browser + Countries -->
            <div class="analytics-charts-row equal">
                <div class="chart-card">
                    <div class="chart-title">Devices & Browsers</div>
                    <div class="chart-container" style="display: flex; gap: 16px;">
                        <div style="flex: 1;"><canvas id="devices-chart"></canvas></div>
                        <div style="flex: 1;"><canvas id="browsers-chart"></canvas></div>
                    </div>
                </div>
                <div class="chart-card">
                    <div class="chart-title">Top Countries</div>
                    <div class="chart-container">
                        <canvas id="countries-chart"></canvas>
                    </div>
                </div>
            </div>

            <!-- Charts Row 3: NPM Downloads -->
            <div class="analytics-charts-row">
                <div class="chart-card">
                    <div class="chart-title">NPM Downloads (30 days)</div>
                    <div class="chart-container">
                        <canvas id="npm-chart"></canvas>
                    </div>
                </div>
                <div class="chart-card">
                    <div class="chart-title">Conversion Funnel</div>
                    <div class="funnel-stages" id="funnel-stages">
                        <div class="funnel-stage">
                            <div class="funnel-bar" id="funnel-visit" style="width: 100%;"></div>
                            <div class="funnel-label">
                                <span class="funnel-name">Page Visit</span>
                                <span class="funnel-count" id="funnel-visit-count">-</span>
                            </div>
                        </div>
                        <div class="funnel-stage">
                            <div class="funnel-bar" id="funnel-install" style="width: 50%;"></div>
                            <div class="funnel-label">
                                <span class="funnel-name">npm install</span>
                                <span class="funnel-count" id="funnel-install-count">-</span>
                                <span class="funnel-rate" id="funnel-install-rate"></span>
                            </div>
                        </div>
                        <div class="funnel-stage">
                            <div class="funnel-bar" id="funnel-session" style="width: 30%;"></div>
                            <div class="funnel-label">
                                <span class="funnel-name">First Session</span>
                                <span class="funnel-count" id="funnel-session-count">-</span>
                                <span class="funnel-rate" id="funnel-session-rate"></span>
                            </div>
                        </div>
                        <div class="funnel-stage">
                            <div class="funnel-bar" id="funnel-subscriber" style="width: 10%;"></div>
                            <div class="funnel-label">
                                <span class="funnel-name">Subscriber</span>
                                <span class="funnel-count" id="funnel-subscriber-count">-</span>
                                <span class="funnel-rate" id="funnel-subscriber-rate"></span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Email Signups Table -->
            <div class="signups-table-container">
                <div class="table-header">
                    <span class="table-title">Recent Email Signups</span>
                    <span class="table-count" id="signups-count">-</span>
                </div>
                <div class="table-scroll">
                    <table>
                        <thead>
                            <tr>
                                <th>Email</th>
                                <th>Source</th>
                                <th>Campaign</th>
                                <th>Promo</th>
                                <th>Signed Up</th>
                            </tr>
                        </thead>
                        <tbody id="signups-table">
                            <tr><td colspan="5" class="loading">Loading...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
                </div>
            </div>
        </div>

        <!-- Gift Management Section -->
        <div class="collapsible-section" id="section-gifts" data-section="gifts">
            <div class="section-header" onclick="toggleSection('gifts')">
                <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 9l6 6 6-6"/>
                </svg>
                <div class="section-name">
                    <span class="section-icon">${iconGift}</span>
                    Gift Subscriptions
                </div>
                <div class="section-summary">
                    <span class="summary-item">24h: <span class="summary-value" id="sum-gifts-24h">-</span> gifts</span>
                    <span class="summary-item">all: <span class="summary-value" id="sum-gifts-all">-</span></span>
                    <span class="summary-item"><span class="summary-value" id="sum-claimed-all">-</span> claimed</span>
                </div>
            </div>
            <div class="section-content">
                <div class="gifts-section">
            <div class="section-title"><span class="section-icon">${iconGift}</span> Gift Subscriptions</div>

            <div class="gift-create-card">
                <form class="gift-form" id="gift-form">
                    <div class="form-group">
                        <label for="duration">Duration</label>
                        <select id="duration" name="duration_type" required>
                            <option value="day">Day</option>
                            <option value="week">Week</option>
                            <option value="month" selected>Month</option>
                            <option value="year">Year</option>
                            <option value="forever">Forever</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label for="email">Recipient Email (optional)</label>
                        <input type="email" id="email" name="recipient_email" placeholder="user@example.com">
                    </div>
                    <button type="submit" class="btn btn-primary" id="create-gift-btn">
                        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                            <path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/>
                        </svg>
                        Create Gift
                    </button>
                </form>

                <div class="gift-result" id="gift-result">
                    <div class="gift-result-header">
                        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                            <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/>
                        </svg>
                        Gift Created!
                    </div>
                    <div class="gift-url-row">
                        <div class="gift-url" id="gift-url"></div>
                        <button type="button" class="btn btn-secondary btn-small" id="copy-url-btn" onclick="copyGiftUrl()">
                            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
                                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
                            </svg>
                            Copy
                        </button>
                        <button type="button" class="btn btn-primary btn-small" id="send-email-btn" style="display:none" onclick="sendGiftEmail()">
                            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                                <path d="M1.75 2h12.5c.966 0 1.75.784 1.75 1.75v8.5A1.75 1.75 0 0114.25 14H1.75A1.75 1.75 0 010 12.25v-8.5C0 2.784.784 2 1.75 2zM1.5 12.251c0 .138.112.25.25.25h12.5a.25.25 0 00.25-.25V5.809L8.38 9.397a.75.75 0 01-.76 0L1.5 5.809v6.442zm13-8.181v-.32a.25.25 0 00-.25-.25H1.75a.25.25 0 00-.25.25v.32L8 7.88l6.5-3.81z"/>
                            </svg>
                            Send Email
                        </button>
                        <span class="copy-success" id="copy-success">Copied!</span>
                    </div>
                </div>
            </div>

            <div class="table-container">
                <div class="table-header">
                    <div class="table-title">All Gifts</div>
                    <div class="table-count" id="gifts-count"></div>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Code</th>
                            <th>Email</th>
                            <th>Duration</th>
                            <th>Created</th>
                            <th>Status</th>
                            <th>Sessions</th>
                            <th>Total Time</th>
                            <th>Avg Time</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody id="gifts-table">
                        <tr><td colspan="9" class="loading">Loading...</td></tr>
                    </tbody>
                </table>
            </div>
                </div>
            </div>
        </div>

        <!-- Recent Telemetry Section -->
        <div class="collapsible-section" id="section-devices" data-section="devices">
            <div class="section-header" onclick="toggleSection('devices')">
                <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 9l6 6 6-6"/>
                </svg>
                <div class="section-name">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <rect x="5" y="2" width="14" height="20" rx="2"/>
                        <path d="M12 18h.01"/>
                    </svg>
                    Recent Telemetry
                </div>
                <div class="section-summary">
                    <span class="summary-item">24h: <span class="summary-value" id="sum-machines-24h">-</span> machines</span>
                    <span class="summary-item">all: <span class="summary-value" id="sum-machines-all">-</span></span>
                    <span class="summary-item">last: <span class="summary-value" id="sum-telemetry-last">-</span></span>
                </div>
            </div>
            <div class="section-content">
                <div class="table-container">
                    <div class="table-header">
                        <div class="table-title">Recent Telemetry</div>
                        <div class="table-count" id="table-count"></div>
                    </div>
                    <table>
                        <thead>
                            <tr>
                                <th>Device ID</th>
                                <th>Machine</th>
                                <th>OS</th>
                                <th>OS Version</th>
                                <th>CLI Version</th>
                                <th>Version</th>
                                <th>Timezone</th>
                                <th>Time</th>
                            </tr>
                        </thead>
                        <tbody id="telemetry-table">
                            <tr><td colspan="8" class="loading">Loading...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>

    <!-- Toast notification -->
    <div class="toast" id="toast"></div>

    <script>
${staffDashboardJs}
    </script>
</body>
</html>
`;
