/**
 * Staff-only telemetry dashboard
 * Shows telemetry data in charts and tables with real-time updates
 */

import { faviconStaffSvg, iconCrabigator, iconGift } from './dashboard/icons';

export const staffDashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator Staff Dashboard</title>
    <link rel="icon" href="data:image/svg+xml,${faviconStaffSvg}">
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"></script>
    <style>
        :root {
            --bg: #0d1117;
            --surface: #161b22;
            --border: #30363d;
            --text: #e6edf3;
            --text-muted: #8b949e;
            --accent: #58a6ff;
            --green: #3fb950;
            --orange: #d29922;
            --red: #f85149;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
            background: var(--bg);
            color: var(--text);
            line-height: 1.5;
        }

        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 24px;
        }

        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 24px;
            padding-bottom: 16px;
            border-bottom: 1px solid var(--border);
        }

        h1 {
            font-size: 24px;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 12px;
        }

        h1 .logo-icon {
            display: inline-flex;
        }

        h1 .logo-icon svg {
            width: 32px;
            height: 32px;
        }

        .status {
            display: flex;
            align-items: center;
            gap: 8px;
            font-size: 14px;
            color: var(--text-muted);
        }

        .status-dot {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: var(--green);
            animation: pulse 2s infinite;
        }

        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }

        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 16px;
            margin-bottom: 24px;
        }

        .stat-card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px;
        }

        .stat-label {
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-muted);
            margin-bottom: 4px;
        }

        .stat-value {
            font-size: 28px;
            font-weight: 600;
        }

        .stat-change {
            font-size: 12px;
            margin-top: 4px;
        }

        .stat-change.positive { color: var(--green); }
        .stat-change.negative { color: var(--red); }

        .charts-grid {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 16px;
            margin-bottom: 24px;
        }

        @media (max-width: 900px) {
            .charts-grid {
                grid-template-columns: 1fr;
            }
        }

        .chart-card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px;
        }

        .chart-title {
            font-size: 14px;
            font-weight: 600;
            margin-bottom: 12px;
            color: var(--text-muted);
        }

        .chart-container {
            position: relative;
            height: 250px;
        }

        .table-container {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            overflow: hidden;
        }

        .table-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 16px;
            border-bottom: 1px solid var(--border);
        }

        .table-title {
            font-size: 14px;
            font-weight: 600;
        }

        .table-count {
            font-size: 12px;
            color: var(--text-muted);
        }

        table {
            width: 100%;
            border-collapse: collapse;
        }

        th, td {
            padding: 12px 16px;
            text-align: left;
            border-bottom: 1px solid var(--border);
        }

        th {
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-muted);
            font-weight: 500;
            background: rgba(255,255,255,0.02);
        }

        td {
            font-size: 13px;
            font-family: 'SF Mono', 'Monaco', 'Inconsolata', monospace;
        }

        tr:hover {
            background: rgba(255,255,255,0.02);
        }

        .device-id {
            max-width: 200px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .version-badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 12px;
            background: rgba(88, 166, 255, 0.15);
            color: var(--accent);
        }

        .os-badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 12px;
        }

        .os-badge.darwin { background: rgba(63, 185, 80, 0.15); color: var(--green); }
        .os-badge.linux { background: rgba(210, 153, 34, 0.15); color: var(--orange); }
        .os-badge.windows { background: rgba(88, 166, 255, 0.15); color: var(--accent); }

        .time-ago {
            color: var(--text-muted);
        }

        .loading {
            text-align: center;
            padding: 40px;
            color: var(--text-muted);
        }

        .error {
            text-align: center;
            padding: 40px;
            color: var(--red);
        }

        /* Gift Management Section */
        .section-title {
            font-size: 18px;
            font-weight: 600;
            margin-bottom: 16px;
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .section-title .section-icon {
            display: inline-flex;
        }

        .section-title .section-icon svg {
            width: 24px;
            height: 24px;
            fill: var(--accent);
        }

        .gifts-section {
            margin-bottom: 32px;
        }

        .gift-create-card {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 16px;
        }

        .gift-form {
            display: flex;
            gap: 12px;
            align-items: flex-end;
            flex-wrap: wrap;
        }

        .form-group {
            display: flex;
            flex-direction: column;
            gap: 6px;
        }

        .form-group label {
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: var(--text-muted);
        }

        .form-group select,
        .form-group input {
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 10px 14px;
            color: var(--text);
            font-size: 14px;
            min-width: 160px;
            transition: border-color 0.2s, box-shadow 0.2s;
        }

        .form-group select:focus,
        .form-group input:focus {
            outline: none;
            border-color: var(--accent);
            box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.15);
        }

        .form-group input::placeholder {
            color: var(--text-muted);
        }

        .btn {
            padding: 10px 20px;
            border-radius: 6px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            border: none;
            transition: all 0.2s;
            display: inline-flex;
            align-items: center;
            gap: 6px;
        }

        .btn-primary {
            background: var(--accent);
            color: #fff;
        }

        .btn-primary:hover {
            background: #4393e6;
        }

        .btn-primary:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        .btn-secondary {
            background: transparent;
            border: 1px solid var(--border);
            color: var(--text);
        }

        .btn-secondary:hover {
            background: rgba(255,255,255,0.05);
            border-color: var(--text-muted);
        }

        .btn-small {
            padding: 6px 12px;
            font-size: 12px;
        }

        .btn-icon {
            padding: 6px 10px;
        }

        .gift-result {
            margin-top: 16px;
            padding: 16px;
            background: rgba(63, 185, 80, 0.08);
            border: 1px solid rgba(63, 185, 80, 0.3);
            border-radius: 6px;
            display: none;
        }

        .gift-result.visible {
            display: block;
            animation: slideIn 0.3s ease-out;
        }

        @keyframes slideIn {
            from {
                opacity: 0;
                transform: translateY(-8px);
            }
            to {
                opacity: 1;
                transform: translateY(0);
            }
        }

        .gift-result-header {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 12px;
            color: var(--green);
            font-weight: 500;
        }

        .gift-url-row {
            display: flex;
            gap: 8px;
            align-items: center;
        }

        .gift-url {
            flex: 1;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 10px 12px;
            font-family: 'SF Mono', 'Monaco', 'Inconsolata', monospace;
            font-size: 13px;
            color: var(--text);
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .copy-success {
            color: var(--green);
            font-size: 12px;
            margin-left: 8px;
            opacity: 0;
            transition: opacity 0.2s;
        }

        .copy-success.visible {
            opacity: 1;
        }

        /* Gift Status Badges */
        .status-badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 12px;
            font-weight: 500;
        }

        .status-badge.unclaimed {
            background: rgba(139, 148, 158, 0.15);
            color: var(--text-muted);
        }

        .status-badge.pending {
            background: rgba(210, 153, 34, 0.15);
            color: var(--orange);
        }

        .status-badge.claimed {
            background: rgba(63, 185, 80, 0.15);
            color: var(--green);
        }

        .duration-badge {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 12px;
            background: rgba(163, 113, 247, 0.15);
            color: #a371f7;
        }

        .gift-code {
            font-family: 'SF Mono', 'Monaco', 'Inconsolata', monospace;
            font-weight: 600;
            letter-spacing: 0.5px;
        }

        .email-truncate {
            max-width: 180px;
            display: inline-block;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            vertical-align: middle;
            font-size: 12px;
        }

        .text-muted {
            color: var(--text-muted);
        }

        .actions-cell {
            display: flex;
            gap: 6px;
        }

        .empty-state {
            text-align: center;
            padding: 40px;
            color: var(--text-muted);
        }

        .empty-state .empty-icon {
            display: block;
            margin: 0 auto 8px;
        }

        .empty-state .empty-icon svg {
            width: 32px;
            height: 32px;
            fill: var(--text-muted);
        }

        /* Toast notification */
        .toast {
            position: fixed;
            bottom: 24px;
            right: 24px;
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 12px 20px;
            display: flex;
            align-items: center;
            gap: 10px;
            box-shadow: 0 8px 24px rgba(0,0,0,0.4);
            transform: translateY(100px);
            opacity: 0;
            transition: all 0.3s ease-out;
            z-index: 1000;
        }

        .toast.visible {
            transform: translateY(0);
            opacity: 1;
        }

        .toast.success {
            border-color: var(--green);
        }

        .toast.error {
            border-color: var(--red);
        }

        /* Loading spinner */
        .spin {
            animation: spin 1s linear infinite;
        }

        @keyframes spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
        }
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

        <!-- Gift Management Section -->
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
                        <th>Version</th>
                        <th>Timezone</th>
                        <th>Time</th>
                    </tr>
                </thead>
                <tbody id="telemetry-table">
                    <tr><td colspan="6" class="loading">Loading...</td></tr>
                </tbody>
            </table>
        </div>
    </div>

    <!-- Toast notification -->
    <div class="toast" id="toast"></div>

    <script>
        // Chart instances
        let checksChart = null;
        let versionChart = null;

        // Current gift state
        let currentGiftId = null;
        let currentGiftEmail = null;

        // Show toast notification
        function showToast(message, type = 'success') {
            const toast = document.getElementById('toast');
            toast.textContent = message;
            toast.className = 'toast ' + type + ' visible';
            setTimeout(() => {
                toast.classList.remove('visible');
            }, 3000);
        }

        // Format relative time
        function timeAgo(timestamp) {
            const now = Math.floor(Date.now() / 1000);
            const diff = now - timestamp;

            if (diff < 60) return 'just now';
            if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
            if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
            return Math.floor(diff / 86400) + 'd ago';
        }

        // Format timezone offset
        function formatTimezone(offset) {
            if (offset === null) return '-';
            const hours = Math.floor(Math.abs(offset) / 60);
            const mins = Math.abs(offset) % 60;
            const sign = offset <= 0 ? '+' : '-';
            return 'UTC' + sign + hours + (mins ? ':' + mins.toString().padStart(2, '0') : '');
        }

        // Render telemetry table
        function renderTable(rows) {
            const tbody = document.getElementById('telemetry-table');
            document.getElementById('table-count').textContent = rows.length + ' entries';

            if (rows.length === 0) {
                tbody.innerHTML = '<tr><td colspan="6" class="loading">No telemetry data yet</td></tr>';
                return;
            }

            tbody.innerHTML = rows.map(row => \`
                <tr>
                    <td class="device-id" title="\${row.device_id}">\${row.device_id.substring(0, 8)}...</td>
                    <td>\${row.machine_name || '-'}</td>
                    <td><span class="os-badge \${row.os || ''}">\${row.os || '-'}</span></td>
                    <td><span class="version-badge">\${row.app_version}</span></td>
                    <td>\${formatTimezone(row.timezone_offset)}</td>
                    <td class="time-ago">\${timeAgo(row.created_at)}</td>
                </tr>
            \`).join('');
        }

        // Update stats
        function updateStats(stats) {
            document.getElementById('total-devices').textContent = stats.total_devices.toLocaleString();
            document.getElementById('checks-24h').textContent = stats.checks_24h.toLocaleString();
            document.getElementById('checks-7d').textContent = stats.checks_7d.toLocaleString();
            document.getElementById('latest-version').textContent = stats.top_version || '-';

            if (stats.new_devices_24h > 0) {
                document.getElementById('devices-change').textContent = '+' + stats.new_devices_24h + ' today';
                document.getElementById('devices-change').className = 'stat-change positive';
            }
        }

        // Update checks chart
        function updateChecksChart(data) {
            const ctx = document.getElementById('checks-chart').getContext('2d');

            if (checksChart) {
                checksChart.data.labels = data.labels;
                checksChart.data.datasets[0].data = data.values;
                checksChart.update();
            } else {
                checksChart = new Chart(ctx, {
                    type: 'line',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            label: 'Update Checks',
                            data: data.values,
                            borderColor: '#58a6ff',
                            backgroundColor: 'rgba(88, 166, 255, 0.1)',
                            fill: true,
                            tension: 0.3
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false }
                        },
                        scales: {
                            x: {
                                grid: { color: '#30363d' },
                                ticks: { color: '#8b949e' }
                            },
                            y: {
                                beginAtZero: true,
                                grid: { color: '#30363d' },
                                ticks: { color: '#8b949e' }
                            }
                        }
                    }
                });
            }
        }

        // Update version chart
        function updateVersionChart(data) {
            const ctx = document.getElementById('version-chart').getContext('2d');

            if (versionChart) {
                versionChart.data.labels = data.labels;
                versionChart.data.datasets[0].data = data.values;
                versionChart.update();
            } else {
                versionChart = new Chart(ctx, {
                    type: 'doughnut',
                    data: {
                        labels: data.labels,
                        datasets: [{
                            data: data.values,
                            backgroundColor: [
                                '#58a6ff',
                                '#3fb950',
                                '#d29922',
                                '#f85149',
                                '#a371f7',
                                '#8b949e'
                            ]
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: {
                                position: 'right',
                                labels: { color: '#e6edf3' }
                            }
                        }
                    }
                });
            }
        }

        // Fetch and update data
        async function fetchData() {
            try {
                const response = await fetch('/api/staff/telemetry');
                if (!response.ok) throw new Error('Failed to fetch');
                const data = await response.json();

                renderTable(data.recent);
                updateStats(data.stats);
                updateChecksChart(data.checks_by_day);
                updateVersionChart(data.version_distribution);

                document.getElementById('last-update').textContent =
                    '· Updated ' + new Date().toLocaleTimeString();
            } catch (err) {
                console.error('Fetch error:', err);
            }
        }

        // Initial load and auto-refresh every 5 seconds
        fetchData();
        setInterval(fetchData, 5000);

        // ============================================
        // Gift Management
        // ============================================

        // Fetch and render gifts
        async function fetchGifts() {
            try {
                const response = await fetch('/api/staff/gifts');
                if (!response.ok) throw new Error('Failed to fetch gifts');
                const data = await response.json();
                renderGiftsTable(data.gifts || []);
            } catch (err) {
                console.error('Fetch gifts error:', err);
            }
        }

        // Render gifts table
        function renderGiftsTable(gifts) {
            const tbody = document.getElementById('gifts-table');
            const countEl = document.getElementById('gifts-count');
            countEl.textContent = gifts.length + ' gift' + (gifts.length !== 1 ? 's' : '');

            if (gifts.length === 0) {
                tbody.innerHTML = '<tr><td colspan="9" class="empty-state"><span class="empty-icon">${iconGift}</span>No gifts created yet</td></tr>';
                return;
            }

            tbody.innerHTML = gifts.map(gift => {
                const statusClass = gift.status;
                const statusLabel = gift.status.charAt(0).toUpperCase() + gift.status.slice(1);
                const emailDisplay = gift.recipient_email
                    ? '<span class="email-truncate" title="' + gift.recipient_email + '">' + gift.recipient_email + '</span>'
                    : '<span class="text-muted">-</span>';

                return \`
                    <tr>
                        <td><span class="gift-code">\${gift.id}</span></td>
                        <td>\${emailDisplay}</td>
                        <td><span class="duration-badge">\${gift.duration_type}</span></td>
                        <td class="time-ago">\${timeAgo(gift.created_at)}</td>
                        <td><span class="status-badge \${statusClass}">\${statusLabel}</span></td>
                        <td>\${gift.session_count || '-'}</td>
                        <td>\${gift.total_duration_formatted || '-'}</td>
                        <td>\${gift.avg_duration_formatted || '-'}</td>
                        <td class="actions-cell">
                            <button class="btn btn-secondary btn-icon" onclick="copyUrl('\${gift.url}')" title="Copy URL">
                                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                                    <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
                                    <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
                                </svg>
                            </button>
                            \${gift.recipient_email && !gift.email_sent_at ? \`
                                <button class="btn btn-secondary btn-icon" onclick="sendEmail('\${gift.id}')" title="Send Email">
                                    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                                        <path d="M1.75 2h12.5c.966 0 1.75.784 1.75 1.75v8.5A1.75 1.75 0 0114.25 14H1.75A1.75 1.75 0 010 12.25v-8.5C0 2.784.784 2 1.75 2zM1.5 12.251c0 .138.112.25.25.25h12.5a.25.25 0 00.25-.25V5.809L8.38 9.397a.75.75 0 01-.76 0L1.5 5.809v6.442zm13-8.181v-.32a.25.25 0 00-.25-.25H1.75a.25.25 0 00-.25.25v.32L8 7.88l6.5-3.81z"/>
                                    </svg>
                                </button>
                            \` : ''}
                        </td>
                    </tr>
                \`;
            }).join('');
        }

        // Create gift form handler
        document.getElementById('gift-form').addEventListener('submit', async (e) => {
            e.preventDefault();

            const btn = document.getElementById('create-gift-btn');
            btn.disabled = true;
            btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" class="spin"><circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="30" stroke-linecap="round"/></svg> Creating...';

            const formData = new FormData(e.target);
            const data = {
                duration_type: formData.get('duration_type'),
                recipient_email: formData.get('recipient_email') || undefined
            };

            try {
                const response = await fetch('/api/staff/gifts', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });

                if (!response.ok) {
                    const error = await response.json();
                    throw new Error(error.error || 'Failed to create gift');
                }

                const gift = await response.json();

                // Show result
                currentGiftId = gift.id;
                currentGiftEmail = data.recipient_email;

                document.getElementById('gift-url').textContent = gift.url;
                document.getElementById('gift-result').classList.add('visible');

                // Show send email button if email provided
                const sendBtn = document.getElementById('send-email-btn');
                sendBtn.style.display = data.recipient_email ? 'inline-flex' : 'none';

                // Clear form
                document.getElementById('email').value = '';

                // Refresh gifts table
                fetchGifts();

                showToast('Gift created successfully!');
            } catch (err) {
                showToast(err.message, 'error');
            } finally {
                btn.disabled = false;
                btn.innerHTML = '<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 4a.5.5 0 0 1 .5.5v3h3a.5.5 0 0 1 0 1h-3v3a.5.5 0 0 1-1 0v-3h-3a.5.5 0 0 1 0-1h3v-3A.5.5 0 0 1 8 4z"/></svg> Create Gift';
            }
        });

        // Copy gift URL from result panel
        function copyGiftUrl() {
            const url = document.getElementById('gift-url').textContent;
            copyUrl(url);
            document.getElementById('copy-success').classList.add('visible');
            setTimeout(() => {
                document.getElementById('copy-success').classList.remove('visible');
            }, 2000);
        }

        // Copy any URL
        function copyUrl(url) {
            navigator.clipboard.writeText(url).then(() => {
                showToast('URL copied to clipboard!');
            }).catch(() => {
                showToast('Failed to copy URL', 'error');
            });
        }

        // Send email for current gift
        async function sendGiftEmail() {
            if (!currentGiftId) return;
            await sendEmail(currentGiftId);
            document.getElementById('send-email-btn').style.display = 'none';
        }

        // Send email for a gift
        async function sendEmail(giftId) {
            try {
                const response = await fetch('/api/staff/gifts/' + giftId + '/send-email', {
                    method: 'POST'
                });

                if (!response.ok) {
                    const error = await response.json();
                    throw new Error(error.error || 'Failed to send email');
                }

                const result = await response.json();
                showToast('Email sent to ' + result.sent_to);
                fetchGifts();
            } catch (err) {
                showToast(err.message, 'error');
            }
        }

        // Initial gifts load and refresh
        fetchGifts();
        setInterval(fetchGifts, 5000);
    </script>
</body>
</html>
`;
