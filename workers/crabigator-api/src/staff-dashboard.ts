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

        h1 .logo-icon {
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
            width: 32px;
            height: 32px;
            fill: var(--text-muted);
            display: block;
            margin: 0 auto 8px;
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
            <h1><svg class="logo-icon" viewBox="0 0 512 512"><path fill="#ea580c" d="M379.204,266.236c-1.879,0-3.78-0.07-5.648-0.206c-11.05-0.806-19.356-10.418-18.55-21.468c0.807-11.05,10.416-19.352,21.469-18.55c0.9,0.066,1.819,0.099,2.729,0.099c20.173,0,36.585-16.412,36.585-36.585c0-1.337-0.072-2.687-0.215-4.01c-1.193-11.016,6.77-20.913,17.786-22.106c11.014-1.19,20.913,6.77,22.105,17.786c0.298,2.751,0.449,5.553,0.449,8.329C455.914,231.824,421.502,266.236,379.204,266.236z"/><path fill="#f97316" d="M415.008,97.65v-53.37c4.962-1.106,10.113-1.709,15.408-1.709c38.929,0,70.489,31.558,70.489,70.487s-31.558,70.487-70.489,70.487c-38.929,0-70.487-31.558-70.487-70.487c0-5.295,0.603-10.446,1.709-15.408L415.008,97.65L415.008,97.65z"/><path fill="#ea580c" d="M491.941,331.928c-5.383,0-10.752-2.153-14.704-6.409c-20.311-21.868-49.921-36.124-83.371-40.14c-11.001-1.321-18.849-11.31-17.528-22.311c1.321-11.001,11.302-18.851,22.311-17.528c42.871,5.148,81.222,23.853,107.989,52.672c7.541,8.119,7.073,20.813-1.046,28.354C501.726,330.153,496.828,331.928,491.941,331.928z"/><path fill="#ea580c" d="M482.529,411.418c-6.567,0-13-3.219-16.844-9.136c-16.534-25.457-43.78-44.737-76.72-54.289c-10.642-3.086-16.767-14.215-13.68-24.856c3.086-10.642,14.212-16.766,24.856-13.681c42.186,12.234,77.414,37.438,99.195,70.969c6.035,9.292,3.396,21.718-5.897,27.753C490.063,410.369,486.274,411.418,482.529,411.418z"/><path fill="#ea580c" d="M434.652,469.431c-7.463,0-14.63-4.182-18.087-11.357c-13.569-28.15-40.168-51.891-72.98-65.132c-10.275-4.146-15.242-15.837-11.096-26.112c4.148-10.276,15.839-15.243,26.113-11.096c42.575,17.181,75.996,47.339,94.109,84.92c4.811,9.982,0.619,21.972-9.362,26.783C440.54,468.79,437.573,469.431,434.652,469.431z"/><path fill="#f97316" d="M132.796,266.236c-42.298,0-76.709-34.412-76.709-76.709c0-2.775,0.151-5.577,0.449-8.329c1.193-11.016,11.097-18.972,22.105-17.786c11.016,1.193,18.979,11.091,17.786,22.106c-0.143,1.323-0.215,2.672-0.215,4.01c0,20.172,16.412,36.585,36.586,36.585c0.91,0,1.828-0.033,2.729-0.099c11.062-0.804,20.663,7.499,21.469,18.55c0.805,11.05-7.499,20.663-18.55,21.468C136.575,266.167,134.675,266.236,132.796,266.236z"/><path fill="#f97316" d="M20.058,331.928c-4.887,0-9.785-1.775-13.649-5.363c-8.119-7.541-8.587-20.235-1.046-28.354c26.769-28.819,65.12-47.524,107.989-52.672c11.009-1.317,20.989,6.527,22.311,17.528c1.321,11.001-6.527,20.991-17.528,22.311c-33.451,4.017-63.06,18.272-83.373,40.141C30.81,329.775,25.44,331.928,20.058,331.928z"/><path fill="#f97316" d="M29.47,411.418c-3.746,0-7.534-1.047-10.909-3.239c-9.293-6.035-11.932-18.461-5.897-27.753c21.78-33.531,57.008-58.736,99.195-70.969c10.644-3.087,21.77,3.039,24.856,13.681c3.087,10.641-3.039,21.77-13.681,24.856c-32.938,9.552-60.186,28.832-76.72,54.289C42.472,408.197,36.036,411.418,29.47,411.418z"/><path fill="#f97316" d="M77.347,469.431c-2.922,0-5.888-0.641-8.696-1.994c-9.982-4.811-14.173-16.803-9.362-26.783c18.112-37.58,51.534-67.739,94.109-84.92c10.277-4.146,21.967,0.821,26.113,11.096c4.146,10.275-0.821,21.966-11.096,26.112c-32.813,13.241-59.412,36.982-72.98,65.132C91.978,465.247,84.81,469.431,77.347,469.431z"/><ellipse fill="#fb923c" cx="255.996" cy="294.45" rx="144.436" ry="120.976"/><path fill="#fb923c" d="M96.991,97.65v-53.37c-4.962-1.106-10.113-1.709-15.408-1.709c-38.929,0-70.487,31.558-70.487,70.487s31.558,70.487,70.487,70.487s70.487-31.558,70.487-70.487c0-5.295-0.602-10.446-1.709-15.408L96.991,97.65L96.991,97.65z"/><path fill="#f97316" d="M400.43,294.451c0-66.813-64.664-120.975-144.431-120.976v241.952C335.767,415.428,400.43,361.265,400.43,294.451z"/></svg> Crabigator Staff Dashboard</h1>
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
            <div class="section-title"><svg class="section-icon" viewBox="0 0 256 256"><path d="M216,72H180.92c.39-.33.79-.65,1.17-1A29.53,29.53,0,0,0,192,49.57,32.62,32.62,0,0,0,158.44,16,29.53,29.53,0,0,0,137,25.91a54.94,54.94,0,0,0-9,14.48,54.94,54.94,0,0,0-9-14.48A29.53,29.53,0,0,0,97.56,16,32.62,32.62,0,0,0,64,49.57,29.53,29.53,0,0,0,73.91,71c.38.33.78.65,1.17,1H40A16,16,0,0,0,24,88v32a16,16,0,0,0,16,16v64a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V136a16,16,0,0,0,16-16V88A16,16,0,0,0,216,72ZM149,36.51a13.69,13.69,0,0,1,10-4.5h.49A16.62,16.62,0,0,1,176,49.08a13.69,13.69,0,0,1-4.5,10c-9.49,8.4-25.24,11.36-35,12.4C137.7,60.89,141,45.5,149,36.51Zm-64.09.36A16.63,16.63,0,0,1,96.59,32h.49a13.69,13.69,0,0,1,10,4.5c8.39,9.48,11.35,25.2,12.39,34.92-9.72-1-25.44-4-34.92-12.39a13.69,13.69,0,0,1-4.5-10A16.6,16.6,0,0,1,84.87,36.87ZM40,88h80v32H40Zm16,48h64v64H56Zm144,64H136V136h64Zm16-80H136V88h80v32Z"/></svg> Gift Subscriptions</div>

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
                tbody.innerHTML = '<tr><td colspan="9" class="empty-state"><svg class="empty-icon" viewBox="0 0 256 256"><path d="M216,72H180.92c.39-.33.79-.65,1.17-1A29.53,29.53,0,0,0,192,49.57,32.62,32.62,0,0,0,158.44,16,29.53,29.53,0,0,0,137,25.91a54.94,54.94,0,0,0-9,14.48,54.94,54.94,0,0,0-9-14.48A29.53,29.53,0,0,0,97.56,16,32.62,32.62,0,0,0,64,49.57,29.53,29.53,0,0,0,73.91,71c.38.33.78.65,1.17,1H40A16,16,0,0,0,24,88v32a16,16,0,0,0,16,16v64a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V136a16,16,0,0,0,16-16V88A16,16,0,0,0,216,72ZM149,36.51a13.69,13.69,0,0,1,10-4.5h.49A16.62,16.62,0,0,1,176,49.08a13.69,13.69,0,0,1-4.5,10c-9.49,8.4-25.24,11.36-35,12.4C137.7,60.89,141,45.5,149,36.51Zm-64.09.36A16.63,16.63,0,0,1,96.59,32h.49a13.69,13.69,0,0,1,10,4.5c8.39,9.48,11.35,25.2,12.39,34.92-9.72-1-25.44-4-34.92-12.39a13.69,13.69,0,0,1-4.5-10A16.6,16.6,0,0,1,84.87,36.87ZM40,88h80v32H40Zm16,48h64v64H56Zm144,64H136V136h64Zm16-80H136V88h80v32Z"/></svg>No gifts created yet</td></tr>';
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
