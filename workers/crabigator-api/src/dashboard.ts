// Dashboard HTML served at /dashboard
export const dashboardHtml = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator Dashboard</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
            background: #0d1117;
            color: #c9d1d9;
            min-height: 100vh;
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
        }
        .container {
            display: grid;
            gap: 16px;
            padding: 16px;
        }
        .session-card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            overflow: hidden;
        }
        .session-header {
            padding: 12px 16px;
            border-bottom: 1px solid #30363d;
            display: flex;
            align-items: center;
            gap: 12px;
        }
        .session-header .state {
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 11px;
            font-weight: 500;
            text-transform: uppercase;
        }
        .state.ready { background: #238636; color: #fff; }
        .state.thinking { background: #1f6feb; color: #fff; }
        .state.permission { background: #db6d28; color: #fff; }
        .state.question { background: #a371f7; color: #fff; }
        .state.complete { background: #8b949e; color: #fff; }
        .session-header .title {
            font-size: 13px;
            font-weight: 500;
            color: #58a6ff;
            max-width: 300px;
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
        .session-header .id {
            font-size: 11px;
            color: #6e7681;
            margin-left: auto;
            font-family: monospace;
        }
        .pin-btn {
            background: #21262d;
            border: 1px solid #30363d;
            padding: 3px 8px;
            margin-left: 8px;
            cursor: pointer;
            font-size: 11px;
            border-radius: 4px;
            transition: all 0.15s ease;
            color: #8b949e;
            display: flex;
            align-items: center;
            gap: 4px;
        }
        .pin-btn:hover {
            background: #30363d;
            border-color: #484f58;
        }
        .pin-btn.pinned {
            background: #1f6feb;
            border-color: #58a6ff;
            color: #fff;
            box-shadow: 0 0 8px rgba(88, 166, 255, 0.4);
        }
        .pin-btn.unpinned {
            background: #161b22;
            border-color: #d29922;
            color: #d29922;
            border-style: dashed;
        }
        .pin-btn.unpinned:hover {
            background: #2d2a1f;
            border-color: #e3b341;
        }
        .terminal {
            background: #0d1117;
            padding: 8px;
            height: 350px;
            overflow: auto;
            font-family: 'SF Mono', 'Fira Code', 'Consolas', 'DejaVu Sans Mono', monospace;
            font-size: 12px;
            line-height: 1.4;
            white-space: pre-wrap;
            word-wrap: break-word;
            overflow-wrap: break-word;
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
        }
        .widget-title {
            color: #58a6ff;
            font-weight: 600;
            margin-bottom: 8px;
            font-size: 12px;
        }
        .widget-row {
            display: flex;
            justify-content: space-between;
            padding: 2px 0;
        }
        .widget-label { color: #8b949e; }
        .widget-value { color: #c9d1d9; }
        .widget-value.green { color: #3fb950; }
        .widget-value.red { color: #f85149; }
        .widget-value.cyan { color: #39c5cf; }
        .widget-value.purple { color: #bc8cff; }
        .widget-value.yellow { color: #d29922; }

        /* Git files list */
        .git-files {
            max-height: 120px;
            overflow-y: auto;
        }
        .git-file {
            display: flex;
            gap: 6px;
            padding: 1px 0;
            align-items: center;
        }
        .git-file .path {
            color: #c9d1d9;
            flex: 1;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .git-file .diff {
            margin-left: auto;
            white-space: nowrap;
            display: flex;
            gap: 4px;
            align-items: center;
        }

        /* Changes list */
        .changes-list {
            max-height: 120px;
            overflow-y: auto;
        }
        .change-item {
            display: flex;
            gap: 4px;
            padding: 1px 0;
            align-items: center;
        }
        .change-item .name {
            color: #c9d1d9;
            flex: 1;
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
        .mode-indicator:hover { background: #30363d; }

        /* Layout segmented control */
        .layout-control {
            display: flex;
            gap: 0;
            background: #21262d;
            border: 1px solid #30363d;
            border-radius: 6px;
            overflow: hidden;
        }
        .layout-btn {
            background: transparent;
            border: none;
            padding: 6px 12px;
            color: #8b949e;
            cursor: pointer;
            font-size: 12px;
            border-right: 1px solid #30363d;
        }
        .layout-btn:last-child { border-right: none; }
        .layout-btn:hover { background: #30363d; }
        .layout-btn.active {
            background: #1f6feb;
            color: #fff;
        }

        /* Layout-based container styles */
        .container[data-layout="1"] { grid-template-columns: 1fr; }
        .container[data-layout="2"] { grid-template-columns: repeat(2, 1fr); }
        .container[data-layout="3"] { grid-template-columns: repeat(3, 1fr); }

        /* Adjust terminal heights for compact layouts */
        .container[data-layout="2"] .terminal { height: 250px; }
        .container[data-layout="3"] .terminal { height: 200px; }
        .container[data-layout="fit"] .terminal { height: 150px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🦀 Crabigator Dashboard</h1>
        <button class="refresh-btn" onclick="loadSessions()">↻ Refresh</button>
        <div class="layout-control">
            <button class="layout-btn active" data-layout="1" onclick="setLayout('1')">1</button>
            <button class="layout-btn" data-layout="2" onclick="setLayout('2')">2</button>
            <button class="layout-btn" data-layout="3" onclick="setLayout('3')">3</button>
            <button class="layout-btn" data-layout="fit" onclick="setLayout('fit')">Fit</button>
        </div>
        <div class="status" id="status">Loading...</div>
    </div>
    <div class="container" id="sessions" data-layout="1"></div>

    <script>
        const API_BASE = '/api';
        const sessions = new Map(); // sessionId -> { eventSource, state, element, git, changes, stats }
        let currentLayout = localStorage.getItem('crabigator-layout') || '1';

        function setLayout(layout) {
            currentLayout = layout;
            localStorage.setItem('crabigator-layout', layout);
            const container = document.getElementById('sessions');
            container.dataset.layout = layout;

            // Update button states
            document.querySelectorAll('.layout-btn').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.layout === layout);
            });

            // For 'fit' mode, calculate columns based on session count
            if (layout === 'fit') {
                const count = sessions.size || 1;
                const cols = Math.ceil(Math.sqrt(count));
                container.style.gridTemplateColumns = 'repeat(' + Math.max(cols, 1) + ', 1fr)';
            } else {
                container.style.gridTemplateColumns = '';
            }
        }

        // Recalculate fit layout when session count changes
        function updateFitLayout() {
            if (currentLayout === 'fit') {
                setLayout('fit');
            }
        }

        // ANSI to HTML converter - processes escape sequences including cursor positioning
        function ansiToHtml(text) {
            if (!text) return '';

            // Trim trailing whitespace from each line to prevent background color bleeding
            // Terminal buffers often pad lines with spaces
            text = text.split('\\n').map(line => line.trimEnd()).join('\\n');

            const defaultFg = '#c9d1d9';
            const defaultBg = '#0d1117';
            const colors = {
                30: '#0d1117', 31: '#f85149', 32: '#3fb950', 33: '#d29922',
                34: '#58a6ff', 35: '#bc8cff', 36: '#39c5cf', 37: '#c9d1d9',
                90: '#6e7681', 91: '#ff7b72', 92: '#7ee787', 93: '#e3b341',
                94: '#79c0ff', 95: '#d2a8ff', 96: '#56d4dd', 97: '#ffffff'
            };
            const bgColors = {
                40: '#0d1117', 41: '#f85149', 42: '#3fb950', 43: '#d29922',
                44: '#58a6ff', 45: '#bc8cff', 46: '#39c5cf', 47: '#c9d1d9',
                100: '#6e7681', 101: '#ff7b72', 102: '#7ee787', 103: '#e3b341',
                104: '#79c0ff', 105: '#d2a8ff', 106: '#56d4dd', 107: '#ffffff'
            };

            // Parse extended color: 38;2;R;G;B (24-bit) or 38;5;N (256-color)
            // Returns { color, skip } where skip is additional codes to skip
            function parseExtendedColor(codes, idx) {
                const mode = codes[idx + 1];

                // 24-bit RGB: 38;2;R;G;B
                if (mode === 2 && codes[idx + 4] !== undefined) {
                    const r = codes[idx + 2];
                    const g = codes[idx + 3];
                    const b = codes[idx + 4];
                    return { color: 'rgb(' + r + ',' + g + ',' + b + ')', skip: 4 };
                }

                // 256-color palette: 38;5;N
                if (mode === 5 && codes[idx + 2] !== undefined) {
                    const colorNum = codes[idx + 2];
                    let color;
                    if (colorNum < 16) {
                        const basic = ['#0d1117','#cd3131','#0dbc79','#e5e510','#2472c8','#bc3fbc','#11a8cd','#e5e5e5',
                                      '#666666','#f14c4c','#23d18b','#f5f543','#3b8eea','#d670d6','#29b8db','#ffffff'];
                        color = basic[colorNum];
                    } else if (colorNum < 232) {
                        const n = colorNum - 16;
                        const ri = Math.floor(n/36);
                        const gi = Math.floor((n%36)/6);
                        const bi = n%6;
                        const r = ri === 0 ? 0 : ri * 40 + 55;
                        const g = gi === 0 ? 0 : gi * 40 + 55;
                        const b = bi === 0 ? 0 : bi * 40 + 55;
                        color = 'rgb(' + r + ',' + g + ',' + b + ')';
                    } else {
                        const gray = (colorNum - 232) * 10 + 8;
                        color = 'rgb(' + gray + ',' + gray + ',' + gray + ')';
                    }
                    return { color, skip: 2 };
                }
                return null;
            }

            let result = '';
            let inSpan = false;
            let currentRow = 1;  // Track current row (1-indexed like VT100)
            let i = 0;
            let currentStyle = '';
            let state = {
                fg: null,
                bg: null,
                bold: false,
                dim: false,
                italic: false,
                underline: false,
                inverse: false
            };

            function resetState() {
                state = {
                    fg: null,
                    bg: null,
                    bold: false,
                    dim: false,
                    italic: false,
                    underline: false,
                    inverse: false
                };
            }

            function buildStyle() {
                let fg = state.fg;
                // For inverse, use bright white since we can't render backgrounds
                if (state.inverse) {
                    fg = '#ffffff';
                }

                const styles = [];
                if (fg) styles.push('color:' + fg);
                // Skip background colors - they bleed to container edge due to terminal padding
                if (state.bold) styles.push('font-weight:bold');
                if (state.dim) styles.push('opacity:0.5');
                if (state.italic) styles.push('font-style:italic');
                if (state.underline) styles.push('text-decoration:underline');
                return styles.join(';');
            }

            function applyStyle() {
                const nextStyle = buildStyle();
                if (nextStyle === currentStyle) return;
                if (inSpan) {
                    result += '</span>';
                    inSpan = false;
                }
                if (nextStyle) {
                    result += '<span style="' + nextStyle + '">';
                    inSpan = true;
                }
                currentStyle = nextStyle;
            }

            while (i < text.length) {
                // Check for ESC character (char code 27)
                if (text.charCodeAt(i) === 27 && text[i + 1] === '[') {
                    // Parse CSI sequence: ESC [ params command
                    let j = i + 2;
                    let params = '';
                    while (j < text.length && /[0-9;]/.test(text[j])) {
                        params += text[j];
                        j++;
                    }
                    const command = text[j];
                    j++;

                    if (command === 'm') {
                        // SGR - Select Graphic Rendition
                        const codes = params ? params.split(';').map(Number) : [0];
                        for (let k = 0; k < codes.length; k++) {
                            const code = codes[k];
                            if (code === 0) { resetState(); }
                            else if (code === 1) state.bold = true;
                            else if (code === 2) state.dim = true;
                            else if (code === 3) state.italic = true;
                            else if (code === 4) state.underline = true;
                            else if (code === 7) state.inverse = true;
                            else if (code === 22) { state.bold = false; state.dim = false; }
                            else if (code === 23) state.italic = false;
                            else if (code === 24) state.underline = false;
                            else if (code === 27) state.inverse = false;
                            else if (code === 39) state.fg = null;
                            else if (code === 49) state.bg = null;
                            else if (code === 38) {
                                const result = parseExtendedColor(codes, k);
                                if (result) { state.fg = result.color; k += result.skip; }
                            }
                            else if (code === 48) {
                                const result = parseExtendedColor(codes, k);
                                if (result) { state.bg = result.color; k += result.skip; }
                            }
                            else if (colors[code]) state.fg = colors[code];
                            else if (bgColors[code]) state.bg = bgColors[code];
                        }

                        applyStyle();
                    } else if (command === 'H' || command === 'f') {
                        // CUP - Cursor Position: ESC[row;colH or ESC[row;colf
                        // Also handles ESC[H (home = 1;1)
                        const parts = params ? params.split(';') : [];
                        const newRow = parts[0] ? parseInt(parts[0], 10) : 1;

                        // If moving to a later row, insert newlines for the gap
                        if (newRow > currentRow) {
                            const linesToAdd = newRow - currentRow;
                            result += '\\n'.repeat(linesToAdd);
                        }
                        currentRow = newRow;
                    }
                    // Skip other escape sequences (J, K, etc.) - they don't affect our line-based output
                    i = j;
                    continue;
                }

                // Track newlines in the content
                if (text[i] === '\\n' || text[i] === '\\r') {
                    if (text[i] === '\\n') {
                        // Close span before newline to prevent background color bleeding
                        if (inSpan) {
                            result += '</span>';
                        }
                        result += '\\n';
                        // Reopen span after newline if we had styling
                        if (inSpan) {
                            result += '<span style="' + currentStyle + '">';
                        }
                        currentRow++;
                    }
                    // Skip carriage return - we only care about line feeds
                    i++;
                    continue;
                }

                // Regular character - escape HTML
                const ch = text[i];
                if (ch === '<') result += '&lt;';
                else if (ch === '>') result += '&gt;';
                else if (ch === '&') result += '&amp;';
                else result += ch;
                i++;
            }

            if (inSpan) result += '</span>';

            return result;
        }

        function formatDuration(seconds) {
            if (seconds < 60) return seconds + 's';
            const mins = Math.floor(seconds / 60);
            const secs = seconds % 60;
            if (mins < 60) return mins + 'm ' + secs + 's';
            const hours = Math.floor(mins / 60);
            return hours + 'h ' + (mins % 60) + 'm';
        }

        async function loadSessions() {
            try {
                const resp = await fetch(API_BASE + '/sessions');
                const data = await resp.json();

                document.getElementById('status').textContent =
                    data.sessions.length + ' session(s)';

                const container = document.getElementById('sessions');

                if (data.sessions.length === 0) {
                    for (const [, session] of sessions) {
                        session.eventSource?.close();
                    }
                    sessions.clear();
                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';

                    // Exponential backoff polling when no sessions (e.g., after deploy)
                    if (emptyPollTimeout) clearTimeout(emptyPollTimeout);
                    console.log('No sessions, polling again in ' + (emptyPollDelay / 1000) + 's');
                    document.getElementById('status').textContent =
                        'No sessions (retry in ' + Math.round(emptyPollDelay / 1000) + 's)';
                    emptyPollTimeout = setTimeout(() => {
                        loadSessions();
                        // Double the delay for next attempt, capped at max
                        emptyPollDelay = Math.min(emptyPollDelay * 2, MAX_EMPTY_POLL_DELAY);
                    }, emptyPollDelay);
                    return;
                }

                // Found sessions - reset exponential backoff
                emptyPollDelay = MIN_EMPTY_POLL_DELAY;
                if (emptyPollTimeout) {
                    clearTimeout(emptyPollTimeout);
                    emptyPollTimeout = null;
                }
                const emptyState = container.querySelector('.no-sessions');
                if (emptyState) {
                    emptyState.remove();
                }

                // Close event sources for removed sessions
                for (const [id, session] of sessions) {
                    if (!data.sessions.find(s => s.id === id)) {
                        session.eventSource?.close();
                        sessions.delete(id);
                    }
                }
                updateFitLayout();

                // Add/update sessions
                for (const session of data.sessions) {
                    if (!sessions.has(session.id)) {
                        createSessionCard(session);
                        connectToSession(session.id);
                    } else {
                        updateSessionHeader(session);
                    }
                }
            } catch (err) {
                console.error('Failed to load sessions:', err);
                document.getElementById('status').textContent = 'Error loading sessions';
            }
        }

        function createSessionCard(session) {
            const container = document.getElementById('sessions');
            const card = document.createElement('div');
            card.className = 'session-card';
            card.id = 'session-' + session.id;
            card.innerHTML = \`
                <div class="session-header">
                    <span class="state \${session.state}">\${session.state}</span>
                    <span class="title" id="title-\${session.id}"></span>
                    <span class="cwd">\${session.cwd}</span>
                    <span class="id">\${session.id.slice(0, 8)}</span>
                    <button class="pin-btn pinned" id="pin-\${session.id}" onclick="togglePin('\${session.id}')" title="Auto-scroll to bottom">⇣ Pinned</button>
                </div>
                <div class="terminal" id="terminal-\${session.id}">Connecting...</div>
                <div class="widgets-panel" id="widgets-\${session.id}">
                    <div class="widget" id="stats-\${session.id}">
                        <div class="widget-title"><span style="color:#bc8cff">Stats</span> <span style="float:right;color:#8b949e">○ Ready</span></div>
                        <div class="widget-row"><span class="widget-label">◆ Session</span><span class="widget-value">--</span></div>
                        <div class="widget-row"><span class="widget-label">◇ Thinking</span><span class="widget-value">—</span></div>
                        <div class="widget-row"><span class="widget-label">▸ Prompts 0</span><span class="widget-value"></span></div>
                        <div class="widget-row"><span class="widget-label">◂ Completions 0</span><span class="widget-value"></span></div>
                        <div class="widget-row"><span class="widget-label">⚙ Tools</span><span class="widget-value purple">0</span></div>
                    </div>
                    <div class="widget" id="git-\${session.id}">
                        <div class="widget-title"><span style="color:#7ee787">...</span> <span style="float:right;color:#8b949e">...</span></div>
                        <div class="git-files" style="color:#8b949e">Waiting for data...</div>
                    </div>
                    <div class="widget" id="changes-\${session.id}">
                        <div class="widget-title"><span style="color:#db6d28">Changes</span></div>
                        <div class="changes-list" style="color:#8b949e">Waiting for data...</div>
                    </div>
                </div>
                <div class="input-area">
                    <input type="text" id="input-\${session.id}"
                           placeholder="Type a command or answer..."
                           onkeydown="if(event.key==='Enter')sendAnswer('\${session.id}')">
                    <button onclick="sendAnswer('\${session.id}')">Send</button>
                </div>
            \`;
            container.appendChild(card);
            sessions.set(session.id, { element: card, state: session.state, title: null, git: null, changes: null, stats: null, pinned: true });
            updateFitLayout();

            // Set up scroll tracking for pin/unpin behavior
            const terminal = document.getElementById('terminal-' + session.id);
            if (terminal) {
                let scrollTimeout = null;
                terminal.addEventListener('scroll', () => {
                    const sessionData = sessions.get(session.id);
                    if (!sessionData) return;

                    // Check if scrolled to bottom (with 20px tolerance for overscroll)
                    const atBottom = terminal.scrollHeight - terminal.scrollTop - terminal.clientHeight < 20;

                    if (atBottom && !sessionData.pinned) {
                        // Re-pin when user scrolls to bottom
                        sessionData.pinned = true;
                        updatePinButton(session.id, true);
                    } else if (!atBottom && sessionData.pinned) {
                        // Unpin when user scrolls away from bottom
                        // Use a small delay to avoid flickering during programmatic scrolls
                        if (scrollTimeout) clearTimeout(scrollTimeout);
                        scrollTimeout = setTimeout(() => {
                            const stillNotAtBottom = terminal.scrollHeight - terminal.scrollTop - terminal.clientHeight >= 20;
                            if (stillNotAtBottom && sessionData.pinned) {
                                sessionData.pinned = false;
                                updatePinButton(session.id, false);
                            }
                        }, 50);
                    }
                });
            }
        }

        function updateSessionHeader(session) {
            const card = document.getElementById('session-' + session.id);
            if (!card) return;
            const stateEl = card.querySelector('.state');
            stateEl.className = 'state ' + session.state;
            stateEl.textContent = session.state;
            sessions.get(session.id).state = session.state;
        }

        function updatePinButton(sessionId, pinned) {
            const btn = document.getElementById('pin-' + sessionId);
            if (!btn) return;
            btn.className = 'pin-btn ' + (pinned ? 'pinned' : 'unpinned');
            btn.textContent = pinned ? '⇣ Pinned' : '⇣ Pin';
            btn.title = pinned ? 'Auto-scroll enabled - click to disable' : 'Click to pin to bottom';
        }

        function togglePin(sessionId) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            const terminal = document.getElementById('terminal-' + sessionId);
            if (!terminal) return;

            if (sessionData.pinned) {
                // Unpin
                sessionData.pinned = false;
                updatePinButton(sessionId, false);
            } else {
                // Pin and scroll to bottom
                sessionData.pinned = true;
                updatePinButton(sessionId, true);
                terminal.scrollTop = terminal.scrollHeight;
            }
        }

        function formatElapsed(timestamp) {
            if (!timestamp) return '';
            const now = Date.now() / 1000;
            const secs = Math.floor(now - timestamp);
            if (secs < 60) return 'just now';
            const mins = Math.floor(secs / 60);
            if (mins < 60) return mins + 'm ago';
            const hours = Math.floor(mins / 60);
            return hours + 'h ago';
        }

        function formatStateIndicator(state) {
            switch (state) {
                case 'ready': return '<span style="color:#8b949e">○ Ready</span>';
                case 'thinking': return '<span style="color:#3fb950">⠋</span>';
                case 'permission': return '<span style="color:#d29922">» ? « Perm</span>';
                case 'question': return '<span style="color:#db6d28">» ? « Ask</span>';
                case 'complete': return '<span style="color:#bc8cff">✓ Complete</span>';
                default: return '<span style="color:#8b949e">○ ' + state + '</span>';
            }
        }

        function formatModeIndicator(mode, sessionId) {
            const modeConfig = {
                'plan': { icon: '⏸', label: 'Plan', color: '#a371f7' },
                'auto_accept': { icon: '⏵⏵', label: 'Auto', color: '#3fb950' },
                'normal': { icon: '●', label: 'Normal', color: '#8b949e' }
            };
            const cfg = modeConfig[mode] || modeConfig['normal'];
            return '<span class="mode-indicator" onclick="switchMode(\\'' + sessionId + '\\')" style="cursor:pointer;padding:2px 6px;border-radius:4px;color:' + cfg.color + '" title="Click to switch mode (Shift+Tab)">' + cfg.icon + ' ' + cfg.label + '</span>';
        }

        async function switchMode(sessionId) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/key', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ key: 'shift_tab' })
                });

                if (!resp.ok) {
                    const err = await resp.json();
                    console.error('Mode switch failed:', err);
                }
            } catch (err) {
                console.error('Failed to switch mode:', err);
            }
        }

        function updateStatsWidget(sessionId, stats) {
            const widget = document.getElementById('stats-' + sessionId);
            if (!widget) return;

            const session = sessions.get(sessionId);
            const state = session?.state || 'ready';
            const stateIndicator = formatStateIndicator(state);
            const modeIndicator = formatModeIndicator(stats.mode || 'normal', sessionId);

            // Store mode in session data for tracking
            if (session && stats.mode) {
                session.mode = stats.mode;
            }

            const promptsElapsed = stats.prompts_changed_at ? formatElapsed(stats.prompts_changed_at) : '';
            const completionsElapsed = stats.completions_changed_at ? formatElapsed(stats.completions_changed_at) : '';

            widget.innerHTML = \`
                <div class="widget-title"><span style="color:#bc8cff">Stats</span> <span style="float:right">\${modeIndicator} \${stateIndicator}</span></div>
                <div class="widget-row"><span class="widget-label">◆ Session</span><span class="widget-value" style="color:#58a6ff">\${formatDuration(stats.work_seconds || 0)}</span></div>
                <div class="widget-row"><span class="widget-label">◇ Thinking</span><span class="widget-value" style="color:#3fb950">\${stats.thinking_seconds ? formatDuration(stats.thinking_seconds) : '—'}</span></div>
                <div class="widget-row"><span class="widget-label">▸ Prompts \${stats.prompts || 0}</span><span class="widget-value" style="color:#8b949e">\${promptsElapsed}</span></div>
                <div class="widget-row"><span class="widget-label">◂ Completions \${stats.completions || 0}</span><span class="widget-value" style="color:#8b949e">\${completionsElapsed}</span></div>
                <div class="widget-row"><span class="widget-label">⚙ Tools</span><span class="widget-value purple">\${stats.tools || 0}</span></div>
            \`;
        }

        // Create two-sided progress bar like CLI: ▓▓ (red/deletions) █████ (green/additions)
        function createProgressBar(additions, deletions) {
            if (additions === 0 && deletions === 0) {
                return '<span style="color:#6e7681">·</span>';
            }
            // Log-scale bar widths (like CLI)
            const delBar = deletions > 0 ? Math.floor(Math.log10(deletions)) + 1 : 0;
            const addBar = additions > 0 ? Math.floor(Math.log10(additions)) + 1 : 0;

            let result = '';
            if (delBar > 0) {
                result += '<span style="color:#f85149">' + '▓'.repeat(delBar) + '</span>';
            }
            if (addBar > 0) {
                result += '<span style="color:#3fb950">' + '█'.repeat(addBar) + '</span>';
            }
            return result;
        }

        function getStatusIcon(status) {
            // Map git status to CLI icons and colors
            const s = (status || '').trim();
            if (s === 'M' || status === 'M ' || status === ' M') {
                return { icon: '●', color: '#d29922' };  // yellow for modified
            }
            if (s === 'A') {
                return { icon: '+', color: '#3fb950' };  // green for added
            }
            if (s === 'D') {
                return { icon: '−', color: '#f85149' };  // red for deleted
            }
            if (s === '??' || s === '?') {
                return { icon: '?', color: '#39c5cf' };  // cyan for untracked
            }
            return { icon: '•', color: '#6e7681' };  // gray for other
        }

        function updateGitWidget(sessionId, git) {
            const widget = document.getElementById('git-' + sessionId);
            if (!widget) return;

            const files = git.files || [];
            const totalFiles = files.length;

            // Header: branch on left, "N files" on right (like CLI)
            const branch = git.branch || 'unknown';
            const filesLabel = totalFiles === 1 ? 'file' : 'files';
            const headerRight = totalFiles > 0
                ? '<span style="color:#d29922">' + totalFiles + ' ' + filesLabel + '</span>'
                : '<span style="color:#3fb950">✓ Clean</span>';

            let filesHtml = files.slice(0, 10).map(f => {
                const { icon, color } = getStatusIcon(f.status);
                const bar = createProgressBar(f.additions || 0, f.deletions || 0);
                const delNum = f.deletions > 0 ? '<span style="color:#f85149">−' + f.deletions + '</span>' : '';
                const addNum = f.additions > 0 ? '<span style="color:#3fb950">+' + f.additions + '</span>' : '';

                return \`<div class="git-file">
                    <span style="color:\${color}">\${icon}</span>
                    <span class="path">\${f.path}</span>
                    <span class="diff">\${delNum} \${bar} \${addNum}</span>
                </div>\`;
            }).join('');

            if (files.length > 10) {
                filesHtml += '<div style="color:#8b949e;padding-top:4px">... and ' + (files.length - 10) + ' more</div>';
            }

            widget.innerHTML = \`
                <div class="widget-title"><span style="color:#7ee787">\${branch}</span> <span style="float:right">\${headerRight}</span></div>
                <div class="git-files">\${filesHtml || '<span style="color:#8b949e">No changes</span>'}</div>
            \`;
        }

        // Get icon and color for change type modifier (like CLI)
        function getModifierStyle(changeType) {
            switch (changeType) {
                case 'added': return { modifier: '+', color: '#3fb950' };
                case 'deleted': return { modifier: '-', color: '#f85149' };
                default: return { modifier: '~', color: '#d29922' };  // modified
            }
        }

        // Get icon and color for node kind (like CLI)
        function getKindIcon(kind) {
            switch (kind?.toLowerCase()) {
                case 'function':
                case 'method':
                    return { icon: 'ƒ', color: '#58a6ff' };
                case 'class':
                    return { icon: '◆', color: '#bc8cff' };
                case 'struct':
                    return { icon: '◇', color: '#39c5cf' };
                case 'enum':
                    return { icon: '▣', color: '#d29922' };
                case 'trait':
                    return { icon: '◈', color: '#bc8cff' };
                case 'impl':
                    return { icon: '◊', color: '#39c5cf' };
                case 'module':
                    return { icon: '□', color: '#8b949e' };
                case 'const':
                    return { icon: '•', color: '#8b949e' };
                default:
                    return { icon: '·', color: '#6e7681' };
            }
        }

        function updateChangesWidget(sessionId, changes) {
            const widget = document.getElementById('changes-' + sessionId);
            if (!widget) return;

            const byLanguage = changes.by_language || [];

            if (byLanguage.length === 0) {
                widget.innerHTML = \`
                    <div class="widget-title"><span style="color:#db6d28">Changes</span></div>
                    <div class="changes-list"><span style="color:#8b949e">No changes detected</span></div>
                \`;
                return;
            }

            // Build header: "Language N changes" (like CLI)
            const firstLang = byLanguage[0];
            const totalChanges = byLanguage.reduce((sum, lang) => sum + (lang.changes?.length || 0), 0);
            const changeWord = totalChanges === 1 ? 'change' : 'changes';

            let changesHtml = '';
            let shown = 0;
            const maxShown = 10;

            for (const lang of byLanguage) {
                if (shown >= maxShown) break;

                // Add language header if multiple languages
                if (byLanguage.length > 1) {
                    const langCount = lang.changes?.length || 0;
                    const langWord = langCount === 1 ? 'change' : 'changes';
                    changesHtml += \`<div style="color:#db6d28;margin-top:4px">\${lang.language} <span style="color:#8b949e">\${langCount} \${langWord}</span></div>\`;
                }

                for (const c of (lang.changes || [])) {
                    if (shown >= maxShown) break;

                    const { modifier, color: modColor } = getModifierStyle(c.change_type);
                    const { icon, color: iconColor } = getKindIcon(c.kind);

                    // Format stats like CLI: −N +M
                    let stats = '';
                    if (c.deletions > 0) {
                        stats += '<span style="color:#f85149">−' + c.deletions + '</span>';
                    }
                    if (c.additions > 0) {
                        stats += '<span style="color:#3fb950">+' + c.additions + '</span>';
                    }

                    changesHtml += \`<div class="change-item">
                        <span style="color:\${modColor}">\${modifier}</span><span style="color:\${iconColor}">\${icon}</span>
                        <span class="name">\${c.name}</span>
                        <span style="margin-left:auto;white-space:nowrap">\${stats}</span>
                    </div>\`;
                    shown++;
                }
            }

            if (totalChanges > maxShown) {
                changesHtml += '<div style="color:#8b949e;padding-top:4px">... and ' + (totalChanges - maxShown) + ' more</div>';
            }

            widget.innerHTML = \`
                <div class="widget-title"><span style="color:#db6d28">\${firstLang.language}</span> <span style="color:#8b949e">\${totalChanges} \${changeWord}</span></div>
                <div class="changes-list">\${changesHtml}</div>
            \`;
        }

        function connectToSession(sessionId) {
            console.log('Connecting SSE for session:', sessionId);
            const eventSource = new EventSource(API_BASE + '/sessions/' + sessionId + '/events');

            eventSource.onopen = () => {
                console.log('SSE connected for session:', sessionId);
                const terminal = document.getElementById('terminal-' + sessionId);
                if (terminal && terminal.innerHTML === 'Connecting...') {
                    terminal.innerHTML = '<span style="color:#8b949e">Connected, waiting for screen data...</span>';
                }
            };

            eventSource.onmessage = (event) => {
                try {
                    console.log('SSE event for', sessionId, ':', event.data.substring(0, 100));
                    const data = JSON.parse(event.data);
                    handleSessionEvent(sessionId, data);
                } catch (err) {
                    console.error('Failed to parse event:', err, event.data);
                }
            };

            eventSource.onerror = (err) => {
                console.error('SSE error for session ' + sessionId, err);
                // Clean up so the session can be recreated on next poll
                const session = sessions.get(sessionId);
                if (session) {
                    session.eventSource?.close();
                    sessions.delete(sessionId);
                }
                // Remove the card - it will be recreated if session is still active
                const card = document.getElementById('session-' + sessionId);
                if (card) {
                    card.remove();
                }
                updateFitLayout();
            };

            const session = sessions.get(sessionId);
            if (session) {
                session.eventSource = eventSource;
            }
        }

        function handleSessionEvent(sessionId, event) {
            const terminal = document.getElementById('terminal-' + sessionId);
            const card = document.getElementById('session-' + sessionId);
            if (!terminal || !card) return;

            const sessionData = sessions.get(sessionId);

            switch (event.type) {
                case 'screen':
                    // Full screen update
                    terminal.innerHTML = ansiToHtml(event.content);
                    if (sessionData?.pinned) {
                        terminal.scrollTop = terminal.scrollHeight;
                    }
                    break;
                case 'state':
                    // Update state badge
                    const stateEl = card.querySelector('.state');
                    if (stateEl) {
                        stateEl.className = 'state ' + event.state;
                        stateEl.textContent = event.state;
                    }
                    // Update session state for stats widget
                    if (sessionData) {
                        sessionData.state = event.state;
                        updateStatsWidget(sessionId, sessionData.stats || {});
                    }
                    break;
                case 'scrollback':
                    // Append scrollback diff
                    if (event.diff) {
                        terminal.innerHTML += ansiToHtml(event.diff);
                        if (sessionData?.pinned) {
                            terminal.scrollTop = terminal.scrollHeight;
                        }
                    }
                    break;
                case 'git':
                    updateGitWidget(sessionId, event);
                    break;
                case 'changes':
                    updateChangesWidget(sessionId, event);
                    break;
                case 'stats':
                    updateStatsWidget(sessionId, event);
                    break;
                case 'title':
                    // Update title in header
                    const titleEl = document.getElementById('title-' + sessionId);
                    if (titleEl) {
                        titleEl.textContent = event.title;
                    }
                    // Store in session data
                    if (sessionData) {
                        sessionData.title = event.title;
                    }
                    break;
                case 'desktop_status':
                    // Desktop connected/disconnected
                    if (!event.connected) {
                        // Desktop disconnected - remove session from view
                        console.log('Desktop disconnected for session:', sessionId);
                        const session = sessions.get(sessionId);
                        if (session) {
                            session.eventSource?.close();
                            sessions.delete(sessionId);
                        }
                        card.remove();
                        updateFitLayout();
                        // Update status
                        document.getElementById('status').textContent = sessions.size + ' session(s)';
                    }
                    break;
            }
        }

        async function sendAnswer(sessionId) {
            const input = document.getElementById('input-' + sessionId);
            const text = input.value.trim();
            if (!text) return;

            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ text })
                });

                if (resp.ok) {
                    input.value = '';
                } else {
                    const err = await resp.json();
                    alert('Error: ' + (err.error || 'Failed to send'));
                }
            } catch (err) {
                console.error('Failed to send answer:', err);
                alert('Failed to send: ' + err.message);
            }
        }

        // SSE connection for real-time session list updates with polling fallback
        let sessionListSource = null;
        let sseRetryCount = 0;
        let pollingInterval = null;
        let emptyPollDelay = 2000;  // Start at 2s when no sessions
        let emptyPollTimeout = null;
        const MAX_SSE_RETRIES = 3;
        const MIN_EMPTY_POLL_DELAY = 2000;   // 2 seconds
        const MAX_EMPTY_POLL_DELAY = 30000;  // 30 seconds

        function connectSessionListStream() {
            if (sessionListSource) {
                sessionListSource.close();
            }

            console.log('Connecting to session list SSE...');
            sessionListSource = new EventSource(API_BASE + '/sessions/stream');

            sessionListSource.onopen = () => {
                console.log('Session list SSE connected');
                sseRetryCount = 0;
                // Stop polling if it was active
                if (pollingInterval) {
                    clearInterval(pollingInterval);
                    pollingInterval = null;
                }
                // Stop empty session backoff polling
                if (emptyPollTimeout) {
                    clearTimeout(emptyPollTimeout);
                    emptyPollTimeout = null;
                }
                emptyPollDelay = MIN_EMPTY_POLL_DELAY;
                document.getElementById('status').textContent = 'Connected (real-time)';
            };

            sessionListSource.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    handleSessionListEvent(data);
                } catch (err) {
                    console.error('Failed to parse session list event:', err);
                }
            };

            sessionListSource.onerror = (err) => {
                console.error('Session list SSE error:', err);
                sessionListSource.close();
                sessionListSource = null;
                sseRetryCount++;

                if (sseRetryCount >= MAX_SSE_RETRIES) {
                    // Fall back to polling after too many SSE failures
                    console.log('SSE failed, falling back to polling');
                    document.getElementById('status').textContent = sessions.size + ' session(s) (polling)';
                    if (!pollingInterval) {
                        pollingInterval = setInterval(loadSessions, 10000);
                    }
                } else {
                    document.getElementById('status').textContent = 'Reconnecting...';
                    // Retry SSE with exponential backoff
                    setTimeout(connectSessionListStream, Math.min(1000 * Math.pow(2, sseRetryCount), 10000));
                }
            };
        }

        function handleSessionListEvent(event) {
            const container = document.getElementById('sessions');

            switch (event.type) {
                case 'connected':
                    // Initial connection established - load full session list once
                    console.log('SSE connected, loading initial session list');
                    loadSessions();
                    break;

                case 'created':
                    // New session - add to view immediately
                    console.log('New session created:', event.session?.id);
                    if (event.session && !sessions.has(event.session.id)) {
                        const emptyState = container.querySelector('.no-sessions');
                        if (emptyState) emptyState.remove();
                        createSessionCard(event.session);
                        connectToSession(event.session.id);
                        document.getElementById('status').textContent = sessions.size + ' session(s) (real-time)';
                    }
                    break;

                case 'updated':
                    // Session updated - update header
                    console.log('Session updated:', event.session?.id, event.session?.state);
                    if (event.session && event.session.id) {
                        updateSessionHeader(event.session);
                        // If session became inactive, remove it from view
                        if (event.session.is_active === false) {
                            const session = sessions.get(event.session.id);
                            if (session) {
                                session.eventSource?.close();
                                sessions.delete(event.session.id);
                                const card = document.getElementById('session-' + event.session.id);
                                if (card) card.remove();
                                updateFitLayout();
                                document.getElementById('status').textContent = sessions.size + ' session(s) (real-time)';
                                if (sessions.size === 0) {
                                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                                }
                            }
                        }
                    }
                    break;

                case 'deleted':
                    // Session deleted - remove from view
                    console.log('Session deleted:', event.session?.id);
                    if (event.session && event.session.id) {
                        const session = sessions.get(event.session.id);
                        if (session) {
                            session.eventSource?.close();
                            sessions.delete(event.session.id);
                        }
                        const card = document.getElementById('session-' + event.session.id);
                        if (card) card.remove();
                        updateFitLayout();
                        document.getElementById('status').textContent = sessions.size + ' session(s) (real-time)';
                        if (sessions.size === 0) {
                            container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                        }
                    }
                    break;
            }
        }

        // Initial load and connect to SSE for real-time updates
        setLayout(currentLayout);  // Apply saved layout preference
        loadSessions();
        connectSessionListStream();
    </script>
</body>
</html>`;
