// Staff dashboard CSS
export const staffDashboardCss = `
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

        /* Analytics Section */
        .analytics-section {
            margin-top: 32px;
            padding-top: 24px;
            border-top: 1px solid var(--border);
        }

        .analytics-section .section-title {
            display: flex;
            align-items: center;
            gap: 8px;
            font-size: 18px;
            font-weight: 600;
            margin-bottom: 20px;
        }

        .analytics-section .section-title svg {
            width: 20px;
            height: 20px;
            color: var(--accent);
        }

        .analytics-stats-grid {
            display: grid;
            grid-template-columns: repeat(5, 1fr);
            gap: 12px;
            margin-bottom: 20px;
        }

        @media (max-width: 900px) {
            .analytics-stats-grid {
                grid-template-columns: repeat(3, 1fr);
            }
        }

        @media (max-width: 600px) {
            .analytics-stats-grid {
                grid-template-columns: repeat(2, 1fr);
            }
        }

        .analytics-charts-row {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 16px;
            margin-bottom: 16px;
        }

        .analytics-charts-row.equal {
            grid-template-columns: 1fr 1fr;
        }

        @media (max-width: 900px) {
            .analytics-charts-row,
            .analytics-charts-row.equal {
                grid-template-columns: 1fr;
            }
        }

        .funnel-container {
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px;
            margin-bottom: 16px;
        }

        .funnel-title {
            font-size: 14px;
            font-weight: 600;
            margin-bottom: 16px;
        }

        .funnel-stages {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        .funnel-stage {
            display: flex;
            align-items: center;
            gap: 12px;
        }

        .funnel-bar {
            height: 28px;
            background: linear-gradient(90deg, var(--accent) 0%, rgba(88, 166, 255, 0.3) 100%);
            border-radius: 4px;
            transition: width 0.3s ease;
            min-width: 20px;
        }

        .funnel-label {
            display: flex;
            align-items: center;
            gap: 8px;
            min-width: 180px;
        }

        .funnel-name {
            font-size: 13px;
            color: var(--text-muted);
        }

        .funnel-count {
            font-size: 14px;
            font-weight: 600;
        }

        .funnel-rate {
            font-size: 12px;
            color: var(--green);
            margin-left: auto;
        }

        .signups-table-container {
            margin-top: 16px;
        }

        .signups-table-container .table-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 8px;
        }

        .signups-table-container .table-title {
            font-size: 14px;
            font-weight: 600;
        }

        .signups-table-container .table-count {
            font-size: 12px;
            color: var(--text-muted);
        }

        .signups-table-container table {
            width: 100%;
            border-collapse: collapse;
            font-size: 13px;
        }

        .signups-table-container th,
        .signups-table-container td {
            padding: 8px 12px;
            text-align: left;
            border-bottom: 1px solid var(--border);
        }

        .signups-table-container th {
            font-weight: 500;
            color: var(--text-muted);
            font-size: 11px;
            text-transform: uppercase;
        }

        /* Collapsible Sections */
        .collapsible-section {
            margin-bottom: 24px;
        }

        .section-header {
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 12px 16px;
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            cursor: pointer;
            user-select: none;
            transition: background 0.15s;
        }

        .section-header:hover {
            background: rgba(255,255,255,0.03);
        }

        .section-header .chevron {
            width: 16px;
            height: 16px;
            color: var(--text-muted);
            transition: transform 0.2s;
            flex-shrink: 0;
        }

        .collapsible-section.collapsed .section-header .chevron {
            transform: rotate(-90deg);
        }

        .section-header .section-name {
            font-size: 14px;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 8px;
        }

        .section-header .section-name svg {
            width: 18px;
            height: 18px;
            color: var(--accent);
        }

        .section-header .section-name .section-icon svg {
            width: 18px;
            height: 18px;
            fill: var(--accent);
        }

        .section-header .section-summary {
            margin-left: auto;
            font-size: 13px;
            color: var(--text-muted);
            display: flex;
            gap: 16px;
        }

        .section-header .section-summary .summary-item {
            display: flex;
            align-items: center;
            gap: 4px;
        }

        .section-header .section-summary .summary-value {
            color: var(--text);
            font-weight: 500;
        }

        .section-content {
            overflow: hidden;
            max-height: 2000px;
            transition: max-height 0.35s cubic-bezier(0.4, 0, 0.2, 1);
        }

        .collapsible-section.collapsed .section-content {
            max-height: 0;
        }

        .section-content-inner {
            background: var(--surface);
            border: 1px solid var(--border);
            border-top: none;
            border-radius: 0 0 8px 8px;
            padding: 16px;
        }

        .collapsible-section.collapsed .section-content-inner {
            border-color: transparent;
        }

        .collapsible-section.collapsed .section-header {
            border-radius: 8px;
            transition: border-radius 0.1s 0.25s;
        }

        .collapsible-section:not(.collapsed) .section-header {
            border-bottom-left-radius: 0;
            border-bottom-right-radius: 0;
            border-bottom: 1px solid var(--border);
            transition: border-radius 0s;
        }

        /* Adjust inner elements when in collapsible section */
        .section-content .stats-grid,
        .section-content .charts-grid,
        .section-content .analytics-stats-grid,
        .section-content .analytics-charts-row {
            margin-bottom: 16px;
        }

        .section-content .stats-grid:last-child,
        .section-content .charts-grid:last-child,
        .section-content .table-container:last-child {
            margin-bottom: 0;
        }

        .section-content .analytics-section {
            margin-top: 0;
            padding-top: 0;
            border-top: none;
        }

        .section-content .gifts-section {
            margin-bottom: 0;
        }

        .section-content .section-title {
            display: none;
        }
`;
