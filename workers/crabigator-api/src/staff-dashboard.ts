/**
 * Staff-only telemetry dashboard
 * Shows telemetry data in charts and tables with real-time updates
 */

export const staffDashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator Staff Dashboard</title>
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

        h1 span {
            font-size: 28px;
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
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1><span>🦀</span> Crabigator Staff Dashboard</h1>
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

    <script>
        // Chart instances
        let checksChart = null;
        let versionChart = null;

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
    </script>
</body>
</html>
`;
