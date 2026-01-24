// Dashboard CSS styles
export const dashboardCss = `
        * { box-sizing: border-box; margin: 0; padding: 0; max-width: 100%; }
        :root {
            --font-scale: 1;
        }
        html {
            overflow-x: hidden;
            width: 100%;
        }
        /* Font size scaling */
        .container {
            zoom: var(--font-scale);
        }
        @supports not (zoom: 1) {
            .container {
                transform: scale(var(--font-scale));
                transform-origin: top left;
            }
        }
        .deploy-overlay {
            display: none;
            position: fixed;
            inset: 0;
            background: rgba(13, 17, 23, 0.95);
            z-index: 1000;
            justify-content: center;
            align-items: center;
            flex-direction: column;
            gap: 24px;
        }
        .deploy-overlay.visible { display: flex; }
        .deploy-spinner {
            width: 48px;
            height: 48px;
            border: 3px solid #30363d;
            border-top-color: #58a6ff;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }
        @keyframes spin { to { transform: rotate(360deg); } }
        .deploy-text {
            font-size: 18px;
            color: #c9d1d9;
            text-align: center;
        }
        .deploy-subtext {
            font-size: 13px;
            color: #8b949e;
            text-align: center;
        }
        .deploy-countdown {
            font-size: 12px;
            color: #6e7681;
            font-family: monospace;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
            background: #0d1117;
            color: #c9d1d9;
            min-height: 100vh;
            overflow-x: hidden;
            width: 100%;
            max-width: 100%;
            position: relative;
        }
        .header {
            background: #161b22;
            border-bottom: 1px solid #30363d;
            padding: 16px 24px;
            display: flex;
            align-items: center;
            gap: 16px;
            position: sticky;
            top: 0;
            z-index: 100;
            max-width: 100%;
        }
        .header h1 {
            font-size: 20px;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 8px;
        }
        .header .status {
            font-size: 12px;
            color: #8b949e;
            margin-left: auto;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            min-width: 0;
        }
        .container {
            padding: 16px;
            column-gap: 16px;
            max-width: 100%;
            overflow-x: hidden;
        }
        .session-card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            overflow: hidden;
            position: relative;
            break-inside: avoid;
            margin-bottom: 16px;
            max-width: 100%;
        }
        .session-header {
            padding: 12px 16px;
            border-bottom: 1px solid #30363d;
            display: flex;
            align-items: flex-start;
            gap: 12px;
        }
        .session-info {
            flex: 1;
            min-width: 0;
            display: flex;
            flex-direction: column;
            gap: 2px;
        }
        .session-header .title {
            font-size: 13px;
            font-weight: 500;
            color: #58a6ff;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .session-header .cwd {
            font-size: 12px;
            color: #8b949e;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .session-actions {
            display: flex;
            flex-direction: column;
            align-items: flex-end;
            gap: 4px;
            flex-shrink: 0;
        }
        .session-actions-row {
            display: flex;
            align-items: center;
            gap: 4px;
        }
        .session-header .state {
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 11px;
            font-weight: 500;
            text-transform: uppercase;
            flex-shrink: 0;
        }
        .state.ready { background: #238636; color: #fff; }
        .state.thinking { background: #1f6feb; color: #fff; }
        .state.permission { background: #db6d28; color: #fff; }
        .state.question { background: #a371f7; color: #fff; }
        .state.complete { background: #8b949e; color: #fff; }
        .state.interrupted { background: #f85149; color: #fff; }
        .info-btn {
            background: transparent;
            border: none;
            padding: 4px 6px;
            cursor: pointer;
            font-size: 12px;
            color: #6e7681;
            border-radius: 4px;
        }
        .info-btn:hover { background: #21262d; color: #8b949e; }
        .info-popover {
            display: none;
            position: absolute;
            right: 8px;
            left: 8px;
            top: 48px;
            background: #21262d;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 12px 16px;
            font-family: monospace;
            font-size: 11px;
            color: #c9d1d9;
            z-index: 100;
            box-shadow: 0 8px 24px rgba(0,0,0,0.4);
            user-select: text;
        }
        .info-popover.visible { display: block; }
        .info-popover-row {
            display: flex;
            justify-content: space-between;
            padding: 4px 0;
            gap: 16px;
        }
        .info-popover-row:not(:last-child) {
            border-bottom: 1px solid #30363d;
        }
        .info-popover-label {
            color: #8b949e;
            flex-shrink: 0;
        }
        .info-popover-value {
            color: #c9d1d9;
            text-align: right;
            word-break: break-all;
        }
        .info-popover-value.copyable {
            cursor: pointer;
        }
        .info-popover-value.copyable:hover {
            color: #58a6ff;
        }
        .pin-indicator {
            font-size: 11px;
            color: #6e7681;
            display: flex;
            align-items: center;
            gap: 4px;
        }
        .pin-indicator.pinned {
            color: #3fb950;
        }
        .pin-btn {
            background: #161b22;
            border: 1px dashed #d29922;
            padding: 3px 8px;
            cursor: pointer;
            font-size: 11px;
            border-radius: 4px;
            transition: all 0.15s ease;
            color: #d29922;
            display: flex;
            align-items: center;
            gap: 4px;
        }
        .pin-btn:hover {
            background: #2d2a1f;
            border-color: #e3b341;
            color: #e3b341;
        }
        .collapse-btn {
            background: transparent;
            border: none;
            padding: 4px 6px;
            cursor: pointer;
            font-size: 12px;
            color: #6e7681;
            border-radius: 4px;
            transition: transform 0.2s ease;
        }
        .collapse-btn:hover { background: #21262d; color: #8b949e; }
        .collapse-btn.collapsed { transform: rotate(-90deg); }
        .session-card.collapsed .terminal,
        .session-card.collapsed .prompt-panel,
        .session-card.collapsed .widgets-panel,
        .session-card.collapsed .input-area,
        .session-card.collapsed .info-popover { display: none !important; }
        .session-card.collapsed .session-header { border-bottom: none; }
        .session-summary {
            display: none;
            padding: 8px 16px;
            background: #161b22;
            border-top: 1px solid #21262d;
            font-size: 11px;
            color: #8b949e;
            gap: 16px;
            flex-wrap: wrap;
        }
        .session-card.collapsed .session-summary { display: flex; }
        .summary-item {
            display: flex;
            align-items: center;
            gap: 4px;
        }
        .summary-item .label { color: #6e7681; }
        .summary-item .value { color: #c9d1d9; }
        .summary-item .green { color: #3fb950; }
        .summary-item .red { color: #f85149; }
        .summary-item .purple { color: #bc8cff; }
        .summary-item .blue { color: #58a6ff; }
        .terminal {
            background: #0d1117;
            padding: 8px;
            overflow-y: auto;
            overflow-x: hidden;
            font-family: 'SF Mono', 'Fira Code', 'Consolas', 'DejaVu Sans Mono', monospace;
            font-size: 12px;
            line-height: 1;
            transition: height 0.25s ease-out;
        }
        .terminal .line {
            white-space: pre-wrap;
            word-break: break-word;
        }
        /* Scroll mode (horizontal scroll instead of wrap) */
        .terminal.scroll-mode {
            overflow-x: auto;
        }
        .terminal.scroll-mode .line {
            white-space: pre;
            max-width: none;
            word-break: normal;
        }
        .terminal-scrollback {
            display: none;
            opacity: 0.7;
            text-align: left;
        }
        .terminal-scrollback.has-content {
            display: block;
        }
        .terminal-separator {
            display: none;  /* Hidden until scrollback content available */
            text-align: center;
            color: #484f58;
            font-size: 10px;
            margin: 8px 0;
            user-select: none;
        }
        .terminal-separator.visible {
            display: block;
        }
        .terminal-screen {
            /* Current screen content */
        }
        .terminal .line:empty::before {
            content: ' ';
            white-space: pre;
        }
        .terminal .split-line {
            display: flex;
            justify-content: space-between;
            gap: 1em;
        }
        .terminal .split-line > span:first-child {
            white-space: pre;
            min-width: 0;
        }
        .terminal .split-line > span:last-child {
            white-space: pre;
            flex-shrink: 0;
        }
        .terminal .rule-line {
            white-space: pre;
            overflow: hidden;
        }
        .terminal span { box-decoration-break: clone; -webkit-box-decoration-break: clone; }
        .terminal .ansi-bright { font-weight: bold; }
        .terminal .ansi-dim { opacity: 0.5; }
        .terminal .ansi-italic { font-style: italic; }
        .terminal .ansi-underline { text-decoration: underline; }

        /* Widgets panel */
        .widgets-panel {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 1px;
            background: #30363d;
            border-top: 1px solid #30363d;
        }
        .widget {
            background: #161b22;
            padding: 12px;
            font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
            font-size: 11px;
            min-width: 0;
            overflow: hidden;
        }
        .widget-title {
            color: #58a6ff;
            font-weight: 600;
            margin-bottom: 8px;
            font-size: 12px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .widget-row {
            display: flex;
            justify-content: space-between;
            padding: 2px 0;
            min-width: 0;
            gap: 8px;
        }
        .widget-label { color: #8b949e; flex-shrink: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        .widget-value { color: #c9d1d9; flex-shrink: 0; }
        .widget-value.green { color: #3fb950; }
        .widget-value.red { color: #f85149; }
        .widget-value.cyan { color: #39c5cf; }
        .widget-value.purple { color: #bc8cff; }
        .widget-value.yellow { color: #d29922; }

        /* Hide empty changes widget */
        .widget.hidden-changes { display: none; }

        /* 2-column grid when changes hidden (Stats + Git only) */
        .widgets-panel.no-changes { grid-template-columns: repeat(2, 1fr); }

        /* Title history widget - spans full width at bottom */
        .title-history-widget {
            grid-column: 1 / -1;
        }
        .titles-list {
            display: flex;
            flex-direction: column;
            gap: 2px;
        }
        .titles-list .title-entry {
            color: #8b949e;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            font-size: 10px;
        }

        /* Stack widgets vertically for multi-column layouts */
        .container[data-layout="2"] .widgets-panel,
        .container[data-layout="3"] .widgets-panel,
        .container[data-layout="fit"] .widgets-panel {
            grid-template-columns: 1fr;
        }

        /* Mobile: always stack widgets */
        @media (max-width: 768px) {
            .widgets-panel { grid-template-columns: 1fr !important; }
            .header {
                flex-wrap: wrap;
                padding: 12px 12px;
                gap: 8px;
            }
            .header h1 {
                font-size: 16px;
            }
            .style-popover {
                right: -8px;
                min-width: 200px;
            }
            .container {
                padding: 8px;
            }
            .session-card {
                min-width: 0;  /* Allow shrinking */
                max-width: 100%;
            }
            .session-header {
                padding: 10px 12px;
                gap: 8px;
            }
            .terminal {
                font-size: 10px;  /* Smaller on mobile for better fit */
                padding: 6px;
            }
            .widget {
                padding: 10px;
            }
            .input-area {
                padding: 10px;
            }
            .input-area input {
                font-size: 16px;  /* Prevent iOS zoom on focus */
            }
            .permission-bar {
                padding: 10px 12px;
            }
            .perm-btn {
                padding: 8px 12px;
                font-size: 12px;
            }
            .perm-hint {
                display: none;  /* Hide hint on mobile to save space */
            }
            .refresh-btn {
                padding: 4px 8px;
                font-size: 12px;
            }
        }

        /* Git files list */
        .git-file {
            display: flex;
            gap: 6px;
            padding: 1px 0;
            align-items: center;
            min-width: 0;
        }
        .git-file .path {
            color: #c9d1d9;
            flex: 1;
            min-width: 0;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .git-file .diff {
            flex-shrink: 0;
            white-space: nowrap;
            display: flex;
            gap: 4px;
            align-items: center;
        }

        /* Changes list */
        .change-item {
            display: flex;
            gap: 4px;
            padding: 1px 0;
            align-items: center;
            min-width: 0;
        }
        .change-item .name {
            color: #c9d1d9;
            flex: 1;
            min-width: 0;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .input-area {
            padding: 12px;
            border-top: 1px solid #30363d;
            display: flex;
            gap: 8px;
        }
        .input-area input {
            flex: 1;
            background: #0d1117;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 8px 12px;
            color: #c9d1d9;
            font-size: 14px;
        }
        .input-area input:focus {
            outline: none;
            border-color: #58a6ff;
        }
        .input-area button {
            background: #238636;
            border: none;
            border-radius: 6px;
            padding: 8px 16px;
            color: #fff;
            font-weight: 500;
            cursor: pointer;
        }
        .input-area button:hover { background: #2ea043; }
        .input-area button:disabled { background: #30363d; cursor: not-allowed; }
        .no-sessions {
            text-align: center;
            padding: 48px;
            color: #8b949e;
        }
        .refresh-btn {
            background: #21262d;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 6px 12px;
            color: #c9d1d9;
            cursor: pointer;
            font-size: 13px;
        }
        .refresh-btn:hover { background: #30363d; }

        /* Style popover */
        .style-btn {
            background: #21262d;
            border: 1px solid #30363d;
            padding: 6px 12px;
            color: #8b949e;
            cursor: pointer;
            font-size: 12px;
            border-radius: 6px;
            display: flex;
            align-items: center;
            gap: 6px;
            transition: all 0.15s ease;
        }
        .style-btn:hover { background: #30363d; color: #c9d1d9; }
        .style-btn.active { background: #30363d; color: #c9d1d9; border-color: #58a6ff; }
        .style-btn svg { width: 14px; height: 14px; }
        .style-popover {
            display: none;
            position: absolute;
            top: calc(100% + 8px);
            right: 0;
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 12px;
            padding: 16px;
            min-width: 220px;
            box-shadow: 0 16px 32px rgba(0,0,0,0.4);
            z-index: 200;
        }
        .style-popover.visible { display: block; }
        .style-section {
            margin-bottom: 16px;
        }
        .style-section:last-child { margin-bottom: 0; }
        .style-section-label {
            font-size: 10px;
            font-weight: 600;
            color: #6e7681;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-bottom: 8px;
        }
        .style-options {
            display: flex;
            gap: 0;
            background: #21262d;
            border: 1px solid #30363d;
            border-radius: 8px;
            overflow: hidden;
        }
        .style-option {
            flex: 1;
            background: transparent;
            border: none;
            padding: 10px 12px;
            color: #8b949e;
            cursor: pointer;
            font-size: 12px;
            font-weight: 500;
            border-right: 1px solid #30363d;
            transition: all 0.15s ease;
        }
        .style-option:last-child { border-right: none; }
        .style-option:hover { background: #30363d; color: #c9d1d9; }
        .style-option.active {
            background: #1f6feb;
            color: #fff;
        }
        .font-size-control {
            display: flex;
            align-items: center;
            gap: 8px;
        }
        .font-size-btn {
            background: #21262d;
            border: 1px solid #30363d;
            width: 36px;
            height: 36px;
            border-radius: 8px;
            color: #8b949e;
            cursor: pointer;
            font-weight: 600;
            display: flex;
            align-items: center;
            justify-content: center;
            transition: all 0.15s ease;
        }
        .font-size-btn:hover { background: #30363d; color: #c9d1d9; }
        .font-size-btn:active { background: #1f6feb; color: #fff; }
        .font-size-btn.decrease { font-size: 12px; }
        .font-size-btn.increase { font-size: 18px; }
        .font-size-value {
            flex: 1;
            text-align: center;
            font-size: 14px;
            font-weight: 500;
            color: #c9d1d9;
            background: #0d1117;
            padding: 8px;
            border-radius: 6px;
            border: 1px solid #21262d;
        }
        .style-container {
            position: relative;
        }

        /* Settings button and popover */
        .settings-container {
            position: relative;
        }
        .settings-btn {
            background: #21262d;
            border: 1px solid #30363d;
            padding: 6px 12px;
            color: #8b949e;
            cursor: pointer;
            font-size: 12px;
            border-radius: 6px;
            display: flex;
            align-items: center;
            gap: 6px;
            transition: all 0.15s ease;
        }
        .settings-btn:hover { background: #30363d; color: #c9d1d9; }
        .settings-btn.active { background: #30363d; color: #c9d1d9; border-color: #58a6ff; }
        .settings-btn svg { width: 14px; height: 14px; }
        .settings-popover {
            display: none;
            position: absolute;
            top: calc(100% + 8px);
            right: 0;
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 12px;
            padding: 16px;
            min-width: 280px;
            box-shadow: 0 16px 32px rgba(0,0,0,0.4);
            z-index: 200;
        }
        .settings-popover.visible { display: block; }
        .settings-section {
            margin-bottom: 0;
        }
        .settings-section-label {
            font-size: 11px;
            font-weight: 600;
            color: #c9d1d9;
            margin-bottom: 6px;
        }
        .settings-description {
            font-size: 11px;
            color: #6e7681;
            margin-bottom: 12px;
            line-height: 1.4;
        }
        .settings-divider {
            height: 1px;
            background: #30363d;
            margin: 16px 0;
        }
        .settings-action-btn {
            width: 100%;
            background: linear-gradient(135deg, #238636 0%, #2ea043 100%);
            border: none;
            padding: 10px 16px;
            color: #fff;
            cursor: pointer;
            font-size: 13px;
            font-weight: 500;
            border-radius: 8px;
            transition: all 0.15s ease;
        }
        .settings-action-btn:hover { opacity: 0.9; transform: translateY(-1px); }
        .settings-action-btn:disabled { opacity: 0.6; cursor: not-allowed; transform: none; }
        .settings-danger-btn {
            width: 100%;
            background: transparent;
            border: 1px solid #f8514966;
            padding: 8px 16px;
            color: #f85149;
            cursor: pointer;
            font-size: 12px;
            border-radius: 6px;
            transition: all 0.15s ease;
        }
        .settings-danger-btn:hover { background: #f8514922; }
        .invite-result {
            margin-top: 12px;
            padding: 12px;
            background: #0d1117;
            border: 1px solid #30363d;
            border-radius: 8px;
            display: none;
        }
        .invite-result.visible { display: block; }
        .invite-code {
            font-family: 'SF Mono', monospace;
            font-size: 20px;
            font-weight: 700;
            color: #fbbf24;
            text-align: center;
            letter-spacing: 0.1em;
            margin-bottom: 8px;
        }
        .invite-hint {
            font-size: 10px;
            color: #6e7681;
            text-align: center;
        }
        .invite-link {
            margin-top: 8px;
            text-align: center;
        }
        .invite-link a {
            font-size: 11px;
            color: #58a6ff;
            text-decoration: none;
        }
        .invite-link a:hover { text-decoration: underline; }

        /* Layout-based container styles (CSS columns for masonry) */
        .container[data-layout="1"] { column-count: 1; }
        .container[data-layout="2"] { column-count: 2; }
        .container[data-layout="3"] { column-count: 3; }

        /* Adjust terminal heights for compact layouts */
        .container[data-layout="2"] .terminal { height: 250px; }
        .container[data-layout="3"] .terminal { height: 200px; }
        .container[data-layout="fit"] .terminal { height: 150px; }

        /* Stack widgets vertically in narrow layouts */
        .container[data-layout="2"] .widgets-panel,
        .container[data-layout="3"] .widgets-panel,
        .container[data-layout="fit"] .widgets-panel {
            grid-template-columns: 1fr;
        }

        /* Prompt panel (questions, permissions) */
        .prompt-panel {
            display: none;
            padding: 16px;
            background: linear-gradient(180deg, #1c2128 0%, #161b22 100%);
            border-bottom: 1px solid #30363d;
        }
        .prompt-panel.visible { display: block; }
        .prompt-header {
            color: #d29922;
            font-size: 12px;
            font-weight: 500;
            margin-bottom: 8px;
        }
        .prompt-question {
            color: #c9d1d9;
            font-size: 13px;
            margin-bottom: 12px;
            line-height: 1.4;
            white-space: pre-wrap;
        }
        .prompt-options {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }
        .prompt-option {
            padding: 10px 12px;
            background: #21262d;
            border: 1px solid #30363d;
            border-radius: 6px;
            cursor: pointer;
            transition: all 0.15s ease;
        }
        .prompt-option:hover {
            background: #30363d;
            border-color: #484f58;
        }
        .prompt-option.selected {
            border-color: #58a6ff;
            background: #1f6feb20;
        }
        .prompt-option-number {
            color: #8b949e;
            font-size: 12px;
            margin-right: 8px;
        }
        .prompt-option-label {
            color: #c9d1d9;
            font-weight: 500;
        }
        .prompt-option-desc {
            color: #8b949e;
            font-size: 12px;
            margin-top: 4px;
            padding-left: 20px;
        }
        .prompt-other {
            margin-top: 12px;
            display: flex;
            gap: 8px;
        }
        .prompt-other input {
            flex: 1;
            padding: 8px 12px;
            background: #0d1117;
            border: 1px solid #30363d;
            border-radius: 6px;
            color: #c9d1d9;
            font-size: 13px;
        }
        .prompt-other button {
            padding: 8px 16px;
            background: #238636;
            border: none;
            border-radius: 6px;
            color: #fff;
            cursor: pointer;
        }
        .prompt-other button:hover {
            background: #2ea043;
        }

        /* Pairing gate styles */
        .pairing-gate {
            width: 100%;
            max-width: 400px;
        }
        .pairing-card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 12px;
            padding: 32px;
            text-align: center;
        }
        .pairing-icon {
            width: 64px;
            height: 64px;
            margin: 0 auto 24px;
            color: #58a6ff;
        }
        .pairing-icon svg {
            width: 100%;
            height: 100%;
        }
        .pairing-card h2 {
            font-size: 24px;
            font-weight: 600;
            margin-bottom: 12px;
            color: #c9d1d9;
        }
        .pairing-description {
            color: #8b949e;
            font-size: 14px;
            line-height: 1.5;
            margin-bottom: 24px;
        }
        .pairing-form {
            display: flex;
            gap: 12px;
            margin-bottom: 16px;
        }
        .pairing-form input {
            flex: 1;
            min-width: 0;
            padding: 12px 16px;
            background: #0d1117;
            border: 1px solid #30363d;
            border-radius: 8px;
            color: #c9d1d9;
            font-size: 18px;
            font-family: monospace;
            text-align: center;
            letter-spacing: 2px;
            text-transform: uppercase;
        }
        .pairing-form input:focus {
            outline: none;
            border-color: #58a6ff;
            box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.2);
        }
        .pairing-form input::placeholder {
            color: #484f58;
            letter-spacing: 2px;
        }
        .pairing-form button {
            flex-shrink: 0;
            padding: 12px 24px;
            background: #238636;
            border: none;
            border-radius: 8px;
            color: #fff;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.15s ease;
            white-space: nowrap;
        }
        .pairing-form button:hover {
            background: #2ea043;
        }
        .pairing-form button:disabled {
            background: #21262d;
            color: #8b949e;
            cursor: not-allowed;
        }
        .pairing-error {
            color: #f85149;
            font-size: 13px;
            min-height: 20px;
            margin-bottom: 16px;
        }
        .pairing-help {
            color: #8b949e;
            font-size: 12px;
        }
        .pairing-help code {
            background: #21262d;
            padding: 2px 6px;
            border-radius: 4px;
            font-family: monospace;
            color: #c9d1d9;
        }
`;
