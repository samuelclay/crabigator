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
            grid-template-columns: repeat(auto-fit, minmax(800px, 1fr));
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
        .terminal {
            background: #0d1117;
            padding: 8px;
            height: 350px;
            overflow: auto;
            font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
            font-size: 12px;
            line-height: 1.15;
            white-space: pre-wrap;
            word-wrap: break-word;
        }
        .terminal .ansi-bright { font-weight: bold; }
        .terminal .ansi-dim { opacity: 0.7; }
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
            gap: 8px;
            padding: 1px 0;
        }
        .git-file .status {
            color: #3fb950;
            width: 20px;
        }
        .git-file .status.modified { color: #d29922; }
        .git-file .status.deleted { color: #f85149; }
        .git-file .status.untracked { color: #8b949e; }
        .git-file .path {
            color: #c9d1d9;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .git-file .diff {
            margin-left: auto;
            white-space: nowrap;
        }
        .git-file .additions { color: #3fb950; }
        .git-file .deletions { color: #f85149; }

        /* Changes list */
        .changes-list {
            max-height: 120px;
            overflow-y: auto;
        }
        .change-item {
            display: flex;
            gap: 8px;
            padding: 1px 0;
        }
        .change-item .kind {
            color: #bc8cff;
            width: 60px;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        .change-item .name {
            color: #c9d1d9;
            flex: 1;
            overflow: hidden;
            text-overflow: ellipsis;
        }
        .change-item .type {
            color: #8b949e;
            font-size: 10px;
        }
        .change-item .type.added { color: #3fb950; }
        .change-item .type.modified { color: #d29922; }
        .change-item .type.deleted { color: #f85149; }

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
    </style>
</head>
<body>
    <div class="header">
        <h1>🦀 Crabigator Dashboard</h1>
        <button class="refresh-btn" onclick="loadSessions()">↻ Refresh</button>
        <div class="status" id="status">Loading...</div>
    </div>
    <div class="container" id="sessions"></div>

    <script>
        const API_BASE = '/api';
        const sessions = new Map(); // sessionId -> { eventSource, state, element, git, changes, stats }
        const deadSessions = new Set(); // Sessions that disconnected - don't recreate them

        // ANSI to HTML converter - processes escape sequences
        function ansiToHtml(text) {
            if (!text) return '';

            const colors = {
                30: '#0d1117', 31: '#f85149', 32: '#3fb950', 33: '#d29922',
                34: '#58a6ff', 35: '#bc8cff', 36: '#39c5cf', 37: '#c9d1d9',
                90: '#6e7681', 91: '#ff7b72', 92: '#7ee787', 93: '#e3b341',
                94: '#79c0ff', 95: '#d2a8ff', 96: '#56d4dd', 97: '#ffffff'
            };
            const bgColors = {
                40: '#0d1117', 41: '#f85149', 42: '#3fb950', 43: '#d29922',
                44: '#58a6ff', 45: '#bc8cff', 46: '#39c5cf', 47: '#c9d1d9'
            };

            function parse256Color(codes, idx) {
                if (codes[idx + 1] === 5 && codes[idx + 2] !== undefined) {
                    const colorNum = codes[idx + 2];
                    if (colorNum < 16) {
                        const basic = ['#0d1117','#cd3131','#0dbc79','#e5e510','#2472c8','#bc3fbc','#11a8cd','#e5e5e5',
                                      '#666666','#f14c4c','#23d18b','#f5f543','#3b8eea','#d670d6','#29b8db','#ffffff'];
                        return basic[colorNum];
                    }
                    if (colorNum < 232) {
                        const n = colorNum - 16;
                        return 'rgb(' + Math.floor(n/36)*51 + ',' + Math.floor((n%36)/6)*51 + ',' + (n%6)*51 + ')';
                    }
                    const gray = (colorNum - 232) * 10 + 8;
                    return 'rgb(' + gray + ',' + gray + ',' + gray + ')';
                }
                return null;
            }

            let result = '';
            let inSpan = false;
            let i = 0;

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

                        if (inSpan) {
                            result += '</span>';
                            inSpan = false;
                        }

                        let styles = [];
                        for (let k = 0; k < codes.length; k++) {
                            const code = codes[k];
                            if (code === 0) { styles = []; }
                            else if (code === 1) styles.push('font-weight:bold');
                            else if (code === 2) styles.push('opacity:0.6');
                            else if (code === 3) styles.push('font-style:italic');
                            else if (code === 4) styles.push('text-decoration:underline');
                            else if (code === 38) {
                                const color = parse256Color(codes, k);
                                if (color) { styles.push('color:' + color); k += 2; }
                            }
                            else if (code === 48) {
                                const color = parse256Color(codes, k);
                                if (color) { styles.push('background:' + color); k += 2; }
                            }
                            else if (colors[code]) styles.push('color:' + colors[code]);
                            else if (bgColors[code]) styles.push('background:' + bgColors[code]);
                        }

                        if (styles.length > 0) {
                            result += '<span style="' + styles.join(';') + '">';
                            inSpan = true;
                        }
                    }
                    // Skip the escape sequence
                    i = j;
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
                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                    return;
                }

                // Close event sources for removed sessions
                for (const [id, session] of sessions) {
                    if (!data.sessions.find(s => s.id === id)) {
                        session.eventSource?.close();
                        sessions.delete(id);
                    }
                }

                // Add/update sessions (skip dead ones)
                for (const session of data.sessions) {
                    // Skip sessions we know are disconnected
                    if (deadSessions.has(session.id)) {
                        continue;
                    }
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
                    <span class="cwd">\${session.cwd}</span>
                    <span class="id">\${session.id.slice(0, 8)}</span>
                </div>
                <div class="terminal" id="terminal-\${session.id}">Connecting...</div>
                <div class="widgets-panel" id="widgets-\${session.id}">
                    <div class="widget" id="stats-\${session.id}">
                        <div class="widget-title">Stats</div>
                        <div class="widget-row"><span class="widget-label">Session</span><span class="widget-value">--</span></div>
                        <div class="widget-row"><span class="widget-label">Prompts</span><span class="widget-value">--</span></div>
                        <div class="widget-row"><span class="widget-label">Tools</span><span class="widget-value">--</span></div>
                        <div class="widget-row"><span class="widget-label">Thinking</span><span class="widget-value">--</span></div>
                    </div>
                    <div class="widget" id="git-\${session.id}">
                        <div class="widget-title">Git</div>
                        <div class="git-files">Waiting for data...</div>
                    </div>
                    <div class="widget" id="changes-\${session.id}">
                        <div class="widget-title">Changes</div>
                        <div class="changes-list">Waiting for data...</div>
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
            sessions.set(session.id, { element: card, state: session.state, git: null, changes: null, stats: null });
        }

        function updateSessionHeader(session) {
            const card = document.getElementById('session-' + session.id);
            if (!card) return;
            const stateEl = card.querySelector('.state');
            stateEl.className = 'state ' + session.state;
            stateEl.textContent = session.state;
            sessions.get(session.id).state = session.state;
        }

        function updateStatsWidget(sessionId, stats) {
            const widget = document.getElementById('stats-' + sessionId);
            if (!widget) return;

            widget.innerHTML = \`
                <div class="widget-title">Stats</div>
                <div class="widget-row"><span class="widget-label">Session</span><span class="widget-value">\${formatDuration(stats.work_seconds || 0)}</span></div>
                <div class="widget-row"><span class="widget-label">Prompts</span><span class="widget-value cyan">\${stats.prompts || 0}</span></div>
                <div class="widget-row"><span class="widget-label">Tools</span><span class="widget-value purple">\${stats.tools || 0}</span></div>
                <div class="widget-row"><span class="widget-label">Thinking</span><span class="widget-value">\${formatDuration(stats.thinking_seconds || 0)}</span></div>
            \`;
        }

        function updateGitWidget(sessionId, git) {
            const widget = document.getElementById('git-' + sessionId);
            if (!widget) return;

            const files = git.files || [];
            const totalFiles = files.length;
            const totalAdditions = files.reduce((sum, f) => sum + (f.additions || 0), 0);
            const totalDeletions = files.reduce((sum, f) => sum + (f.deletions || 0), 0);

            let filesHtml = files.slice(0, 10).map(f => {
                const statusClass = f.status === 'M ' || f.status === ' M' ? 'modified' :
                                   f.status === 'D ' ? 'deleted' :
                                   f.status === '??' ? 'untracked' : '';
                const statusChar = f.status === '??' ? '?' : f.status.trim() || 'M';
                return \`<div class="git-file">
                    <span class="status \${statusClass}">\${statusChar}</span>
                    <span class="path">\${f.path}</span>
                    <span class="diff"><span class="additions">+\${f.additions || 0}</span> <span class="deletions">-\${f.deletions || 0}</span></span>
                </div>\`;
            }).join('');

            if (files.length > 10) {
                filesHtml += '<div style="color:#8b949e;padding-top:4px">... and ' + (files.length - 10) + ' more files</div>';
            }

            widget.innerHTML = \`
                <div class="widget-title">Git <span style="color:#8b949e;font-weight:normal">(\${git.branch || 'unknown'})</span></div>
                <div style="margin-bottom:6px;color:#8b949e">\${totalFiles} files · <span class="widget-value green">+\${totalAdditions}</span> <span class="widget-value red">-\${totalDeletions}</span></div>
                <div class="git-files">\${filesHtml || '<span style="color:#8b949e">No changes</span>'}</div>
            \`;
        }

        function updateChangesWidget(sessionId, changes) {
            const widget = document.getElementById('changes-' + sessionId);
            if (!widget) return;

            const allChanges = [];
            for (const lang of (changes.by_language || [])) {
                for (const change of (lang.changes || [])) {
                    allChanges.push({ ...change, language: lang.language });
                }
            }

            let changesHtml = allChanges.slice(0, 10).map(c => {
                const typeClass = c.change_type === 'added' ? 'added' :
                                 c.change_type === 'deleted' ? 'deleted' : 'modified';
                return \`<div class="change-item">
                    <span class="kind">\${c.kind}</span>
                    <span class="name">\${c.name}</span>
                    <span class="type \${typeClass}">\${c.change_type}</span>
                </div>\`;
            }).join('');

            if (allChanges.length > 10) {
                changesHtml += '<div style="color:#8b949e;padding-top:4px">... and ' + (allChanges.length - 10) + ' more</div>';
            }

            widget.innerHTML = \`
                <div class="widget-title">Changes</div>
                <div class="changes-list">\${changesHtml || '<span style="color:#8b949e">No changes detected</span>'}</div>
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
                const terminal = document.getElementById('terminal-' + sessionId);
                if (terminal) {
                    terminal.innerHTML = '<span style="color:#f85149">[Connection error - desktop may be offline]</span>';
                }
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

            switch (event.type) {
                case 'screen':
                    // Full screen update
                    terminal.innerHTML = ansiToHtml(event.content);
                    terminal.scrollTop = terminal.scrollHeight;
                    break;
                case 'state':
                    // Update state badge
                    const stateEl = card.querySelector('.state');
                    if (stateEl) {
                        stateEl.className = 'state ' + event.state;
                        stateEl.textContent = event.state;
                    }
                    break;
                case 'scrollback':
                    // Append scrollback diff
                    if (event.diff) {
                        terminal.innerHTML += ansiToHtml(event.diff);
                        terminal.scrollTop = terminal.scrollHeight;
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
                        // Track dead sessions so they don't reappear on refresh
                        deadSessions.add(sessionId);
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

        // Initial load
        loadSessions();

        // Refresh session list every 30 seconds
        setInterval(loadSessions, 30000);
    </script>
</body>
</html>`;
