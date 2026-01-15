// Dashboard JavaScript
export const dashboardJs = `
        const API_BASE = '/api';
        const sessions = new Map(); // sessionId -> { eventSource, state, element, git, changes, stats }
        let currentLayout = localStorage.getItem('crabigator-layout') || '1';

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        // Font size scaling
        const FONT_SCALES = [0.75, 0.85, 0.9, 1.0, 1.1, 1.25, 1.5];
        let currentFontScaleIndex = 3; // default 1.0
        let isChangingFontSize = false; // Flag to prevent scroll unpinning during zoom changes

        // Terminal height scaling
        const TERMINAL_HEIGHTS = [150, 200, 250, 350, 450, 550, 700];
        let currentHeightIndex = 3; // default 350px

        // Scrollback chunking - only render CHUNK_SIZE lines at a time for performance
        const SCROLLBACK_CHUNK_SIZE = 1000;

        // Load more scrollback lines when user scrolls to top
        function loadMoreScrollback(sessionId) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData || !sessionData.scrollbackBuffer) return;

            const buffer = sessionData.scrollbackBuffer;
            const alreadyRendered = sessionData.scrollbackRendered || 0;
            const totalLines = buffer.length;

            if (alreadyRendered >= totalLines) return; // Already showing everything

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const separatorEl = document.getElementById('separator-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl || !terminal) return;

            // Calculate how many more lines to load
            const linesToLoad = Math.min(SCROLLBACK_CHUNK_SIZE, totalLines - alreadyRendered);
            const startIdx = totalLines - alreadyRendered - linesToLoad;
            const endIdx = totalLines - alreadyRendered;

            // Get the lines to prepend (they go at the beginning)
            const newLines = buffer.slice(startIdx, endIdx);
            const newHtml = ansiToHtml(newLines.join('\\n') + '\\n');

            // Remember scroll position to maintain view after prepending
            const prevScrollHeight = terminal.scrollHeight;

            // Prepend new content
            scrollbackEl.innerHTML = newHtml + scrollbackEl.innerHTML;
            scrollbackEl.classList.add('has-content');
            if (separatorEl) separatorEl.classList.add('visible');

            // Update rendered count
            sessionData.scrollbackRendered = alreadyRendered + linesToLoad;

            // Restore scroll position (content was added above, so scroll down by the added height)
            const newScrollHeight = terminal.scrollHeight;
            terminal.scrollTop += (newScrollHeight - prevScrollHeight);
            sessionData.lastScrollTop = terminal.scrollTop;

            // Show indicator if there's more to load
            updateScrollbackIndicator(sessionId, sessionData.scrollbackRendered, totalLines);
        }

        // Render initial scrollback (last CHUNK_SIZE lines)
        function renderScrollback(sessionId, lines) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const separatorEl = document.getElementById('separator-' + sessionId);
            if (!scrollbackEl) return;

            // Store full buffer
            sessionData.scrollbackBuffer = lines;

            // Render only the last CHUNK_SIZE lines
            const linesToRender = Math.min(SCROLLBACK_CHUNK_SIZE, lines.length);
            const startIdx = lines.length - linesToRender;
            const visibleLines = lines.slice(startIdx);

            scrollbackEl.innerHTML = ansiToHtml(visibleLines.join('\\n') + '\\n');
            scrollbackEl.classList.add('has-content');
            if (separatorEl) separatorEl.classList.add('visible');

            sessionData.scrollbackRendered = linesToRender;
            updateScrollbackIndicator(sessionId, linesToRender, lines.length);
        }

        // Append new scrollback content
        function appendScrollback(sessionId, newContent) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const separatorEl = document.getElementById('separator-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl) return;

            // Split into lines and add to buffer
            const newLines = newContent.split('\\n').filter(line => line.length > 0 || newContent.includes('\\n\\n'));
            if (!sessionData.scrollbackBuffer) sessionData.scrollbackBuffer = [];
            sessionData.scrollbackBuffer.push(...newLines);

            // Append to rendered content (new content always visible at bottom of scrollback)
            scrollbackEl.innerHTML += ansiToHtml(newContent);
            scrollbackEl.classList.add('has-content');
            if (separatorEl) separatorEl.classList.add('visible');

            sessionData.scrollbackRendered = (sessionData.scrollbackRendered || 0) + newLines.length;

            // Auto-scroll if pinned
            if (sessionData.pinned && terminal) {
                terminal.scrollTop = terminal.scrollHeight;
                sessionData.lastScrollTop = terminal.scrollTop;
            }
        }

        // Update indicator showing how much scrollback is available
        function updateScrollbackIndicator(sessionId, rendered, total) {
            const separatorEl = document.getElementById('separator-' + sessionId);
            if (!separatorEl) return;

            if (rendered < total) {
                const remaining = total - rendered;
                separatorEl.textContent = \`─── scrollback (\${remaining} more lines) ───\`;
            } else {
                separatorEl.textContent = '─── scrollback ───';
            }
        }

        function adjustFontSize(delta) {
            const newIndex = Math.max(0, Math.min(FONT_SCALES.length - 1, currentFontScaleIndex + delta));
            if (newIndex !== currentFontScaleIndex) {
                currentFontScaleIndex = newIndex;
                applyFontScale();
                saveSettingsToServer();
            }
        }

        function applyFontScale() {
            const scale = FONT_SCALES[currentFontScaleIndex];

            // Set flag to prevent scroll events from unpinning during zoom change
            isChangingFontSize = true;

            document.documentElement.style.setProperty('--font-scale', scale);
            const label = document.getElementById('font-label');
            if (label) label.textContent = Math.round(scale * 100) + '%';

            // Scroll pinned sessions to bottom after zoom
            requestAnimationFrame(() => {
                for (const [id, sessionData] of sessions) {
                    if (sessionData.pinned) {
                        const terminal = document.getElementById('terminal-' + id);
                        if (terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                }
                // Clear flag after scroll positions are restored
                setTimeout(() => { isChangingFontSize = false; }, 100);
            });
        }

        function adjustTerminalHeight(delta) {
            const newIndex = Math.max(0, Math.min(TERMINAL_HEIGHTS.length - 1, currentHeightIndex + delta));
            if (newIndex !== currentHeightIndex) {
                currentHeightIndex = newIndex;
                applyTerminalHeight();
                saveSettingsToServer();
            }
        }

        function applyTerminalHeight() {
            const height = TERMINAL_HEIGHTS[currentHeightIndex];
            const label = document.getElementById('height-label');
            if (label) label.textContent = height + 'px';

            // Apply height directly to all terminal elements
            document.querySelectorAll('.terminal').forEach(terminal => {
                terminal.style.height = height + 'px';
            });

            // Scroll pinned sessions to bottom after height change
            requestAnimationFrame(() => {
                for (const [id, sessionData] of sessions) {
                    if (sessionData.pinned) {
                        const terminal = document.getElementById('terminal-' + id);
                        if (terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                }
            });
        }

        async function saveSettingsToServer() {
            try {
                await fetch(API_BASE + '/settings', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        fontScaleIndex: currentFontScaleIndex,
                        terminalHeightIndex: currentHeightIndex
                    }),
                    credentials: 'same-origin'
                });
            } catch {}
        }

        async function loadSettingsFromServer() {
            try {
                const resp = await fetch(API_BASE + '/settings', {
                    credentials: 'same-origin'
                });
                if (resp.ok) {
                    const data = await resp.json();
                    if (typeof data.fontScaleIndex === 'number') {
                        currentFontScaleIndex = Math.max(0, Math.min(FONT_SCALES.length - 1, data.fontScaleIndex));
                    }
                    if (typeof data.terminalHeightIndex === 'number') {
                        currentHeightIndex = Math.max(0, Math.min(TERMINAL_HEIGHTS.length - 1, data.terminalHeightIndex));
                    }
                }
            } catch {}
            applyFontScale();
            applyTerminalHeight();
        }

        // Style popover
        function toggleStylePopover() {
            const popover = document.getElementById('style-popover');
            const btn = document.getElementById('style-btn');
            const isVisible = popover.classList.toggle('visible');
            btn.classList.toggle('active', isVisible);
        }

        function closeStylePopover() {
            const popover = document.getElementById('style-popover');
            const btn = document.getElementById('style-btn');
            popover.classList.remove('visible');
            btn.classList.remove('active');
        }

        // Close popover when clicking outside
        document.addEventListener('click', (e) => {
            const popover = document.getElementById('style-popover');
            const btn = document.getElementById('style-btn');
            if (popover && btn && !popover.contains(e.target) && !btn.contains(e.target)) {
                closeStylePopover();
            }
        });

        // Deploy detection and reconnection
        let isDeploying = false;
        let deployReconnectDelay = 500;
        const MAX_RECONNECT_DELAY = 10000;
        let reconnectTimeout = null;
        let hadSessionsBefore = false;
        let lastSuccessfulConnection = 0;  // Timestamp of last successful API call
        const DEPLOY_DETECTION_WINDOW = 30000;  // Only detect deploy if connected within last 30s

        function wasRecentlyConnected() {
            return Date.now() - lastSuccessfulConnection < DEPLOY_DETECTION_WINDOW;
        }

        function showDeployOverlay() {
            isDeploying = true;
            document.getElementById('deploy-overlay').classList.add('visible');
            updateDeployCountdown();
        }

        function hideDeployOverlay() {
            isDeploying = false;
            deployReconnectDelay = 500;
            document.getElementById('deploy-overlay').classList.remove('visible');
        }

        function updateDeployCountdown() {
            const el = document.getElementById('deploy-countdown');
            if (el && isDeploying) {
                el.textContent = 'Retrying in ' + (deployReconnectDelay / 1000).toFixed(1) + 's...';
            }
        }

        function scheduleReconnect() {
            if (reconnectTimeout) clearTimeout(reconnectTimeout);
            updateDeployCountdown();
            reconnectTimeout = setTimeout(() => {
                loadSessions();
                deployReconnectDelay = Math.min(deployReconnectDelay * 1.5, MAX_RECONNECT_DELAY);
            }, deployReconnectDelay);
        }

        // Input preservation - save to localStorage and server
        const inputCache = new Map(); // sessionId -> text

        function saveInputLocally(sessionId, text) {
            inputCache.set(sessionId, text);
            try {
                const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                stored[sessionId] = text;
                localStorage.setItem('crabigator-inputs', JSON.stringify(stored));
            } catch {}
        }

        function getLocalInput(sessionId) {
            try {
                const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                return stored[sessionId] || '';
            } catch { return ''; }
        }

        function clearLocalInput(sessionId) {
            inputCache.delete(sessionId);
            try {
                const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                delete stored[sessionId];
                localStorage.setItem('crabigator-inputs', JSON.stringify(stored));
            } catch {}
        }

        async function saveInputToServer(sessionId, text) {
            try {
                await fetch(API_BASE + '/sessions/' + sessionId + '/draft', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ text })
                });
            } catch {}
        }

        async function getServerInput(sessionId) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/draft');
                if (resp.ok) {
                    const data = await resp.json();
                    return data.text || '';
                }
            } catch {}
            return '';
        }

        async function restoreInput(sessionId) {
            // Try local first, then server
            let text = getLocalInput(sessionId);
            if (!text) {
                text = await getServerInput(sessionId);
            }
            if (text) {
                const input = document.getElementById('input-' + sessionId);
                if (input && !input.value) {
                    input.value = text;
                }
            }
        }

        // Debounce server saves
        const inputSaveTimers = new Map();
        function handleInputChange(sessionId, text) {
            // Always save locally immediately
            saveInputLocally(sessionId, text);

            // Debounce server save (500ms)
            if (inputSaveTimers.has(sessionId)) {
                clearTimeout(inputSaveTimers.get(sessionId));
            }
            inputSaveTimers.set(sessionId, setTimeout(() => {
                saveInputToServer(sessionId, text);
                inputSaveTimers.delete(sessionId);
            }, 500));
        }

        function setLayout(layout) {
            currentLayout = layout;
            localStorage.setItem('crabigator-layout', layout);
            const container = document.getElementById('sessions');
            container.dataset.layout = layout;

            // Update button states
            document.querySelectorAll('.style-option[data-layout]').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.layout === layout);
            });

            // For 'fit' mode, calculate columns based on session count
            if (layout === 'fit') {
                const count = sessions.size || 1;
                const cols = Math.ceil(Math.sqrt(count));
                container.style.columnCount = Math.max(cols, 1);
            } else {
                container.style.columnCount = '';
            }

            // After layout change, scroll pinned sessions to bottom
            requestAnimationFrame(() => {
                for (const [id, sessionData] of sessions) {
                    if (sessionData.pinned) {
                        const terminal = document.getElementById('terminal-' + id);
                        if (terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                }
            });
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

            // Wrap each line in a div, detecting special line types
            const boxDrawingChars = '─━═╌╍┄┅┈┉';

            // Helper: find HTML index corresponding to visible character position
            function findHtmlIndex(html, visiblePos) {
                let visible = 0;
                let inTag = false;
                for (let i = 0; i < html.length; i++) {
                    if (html[i] === '<') inTag = true;
                    else if (html[i] === '>') inTag = false;
                    else if (!inTag) {
                        if (visible === visiblePos) return i;
                        visible++;
                    }
                }
                return html.length;
            }

            // Helper: split HTML at visible character boundaries, handling open spans
            function splitHtmlForFlexbox(html, leftEnd, rightStart) {
                const leftIdx = findHtmlIndex(html, leftEnd);
                const rightIdx = findHtmlIndex(html, rightStart);

                let leftHtml = html.slice(0, leftIdx);
                let rightHtml = html.slice(rightIdx);

                // Check if we need to close/reopen a span at the split
                // Count open spans in left part
                const leftSpanOpens = (leftHtml.match(/<span[^>]*>/g) || []).length;
                const leftSpanCloses = (leftHtml.match(/<\\/span>/g) || []).length;
                const unclosedSpans = leftSpanOpens - leftSpanCloses;

                if (unclosedSpans > 0) {
                    // Find the last unclosed span's style
                    const spanMatches = leftHtml.match(/<span[^>]*>/g) || [];
                    const lastSpan = spanMatches[spanMatches.length - 1];
                    // Close it in left, reopen in right
                    leftHtml += '</span>';
                    rightHtml = lastSpan + rightHtml;
                }

                return [leftHtml, rightHtml];
            }

            const lines = result.split('\\n');
            result = lines.map(lineHtml => {
                // Strip HTML tags to analyze plain text content
                const plain = lineHtml.replace(/<[^>]+>/g, '').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');

                // Check for decorative rule line (>60% box chars)
                let boxCount = 0;
                for (const c of plain) {
                    if (boxDrawingChars.includes(c)) boxCount++;
                }
                if (plain.length > 10 && boxCount / plain.length > 0.6) {
                    return '<div class="rule-line">' + lineHtml + '</div>';
                }

                // Check for split line (text + 10+ spaces + text)
                // Use non-greedy match for left, greedy for spaces, then remaining text
                const splitMatch = plain.match(/^(.+?)( {10,})(.+)$/);
                if (splitMatch) {
                    const leftLen = splitMatch[1].length;
                    const gapLen = splitMatch[2].length;
                    const rightStart = leftLen + gapLen;

                    const [leftHtml, rightHtml] = splitHtmlForFlexbox(lineHtml, leftLen, rightStart);

                    return '<div class="line split-line"><span>' + leftHtml + '</span><span>' + rightHtml + '</span></div>';
                }

                return '<div class="line">' + lineHtml + '</div>';
            }).join('');

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
                if (!resp.ok) throw new Error('Failed to fetch sessions');
                const data = await resp.json();

                document.getElementById('status').textContent =
                    data.sessions.length + ' session(s)';

                const container = document.getElementById('sessions');

                if (data.sessions.length === 0) {
                    // If we had sessions recently and now have none, likely a deploy
                    if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                        showDeployOverlay();
                    }

                    for (const [, session] of sessions) {
                        session.eventSource?.close();
                    }
                    sessions.clear();
                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';

                    // Use deploy reconnect if deploying, otherwise normal backoff
                    if (isDeploying) {
                        scheduleReconnect();
                    } else {
                        // Exponential backoff polling when no sessions
                        if (emptyPollTimeout) clearTimeout(emptyPollTimeout);
                        console.log('No sessions, polling again in ' + (emptyPollDelay / 1000) + 's');
                        document.getElementById('status').textContent =
                            'No sessions (retry in ' + Math.round(emptyPollDelay / 1000) + 's)';
                        emptyPollTimeout = setTimeout(() => {
                            loadSessions();
                            emptyPollDelay = Math.min(emptyPollDelay * 2, MAX_EMPTY_POLL_DELAY);
                        }, emptyPollDelay);
                    }
                    return;
                }

                // Found sessions - we're connected
                hadSessionsBefore = true;
                lastSuccessfulConnection = Date.now();
                if (isDeploying) {
                    hideDeployOverlay();
                    // Reconnect session list SSE stream after deploy
                    if (!sessionListSource) {
                        sseRetryCount = 0;
                        connectSessionListStream();
                    }
                }

                // Reset exponential backoff
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
                // Network error might mean deploy - only show overlay if we were recently connected
                if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                    showDeployOverlay();
                }
                if (isDeploying) {
                    scheduleReconnect();
                } else if (hadSessionsBefore) {
                    // Silent retry without overlay
                    setTimeout(loadSessions, 2000);
                }
            }
        }

        function createSessionCard(session) {
            const container = document.getElementById('sessions');
            const card = document.createElement('div');
            card.className = 'session-card';
            card.id = 'session-' + session.id;
            const startedAt = formatStartedAt(session.started_at);
            card.innerHTML = \`
                <div class="session-header">
                    <div class="session-info">
                        <span class="title" id="title-\${session.id}"></span>
                        <span class="cwd">\${session.cwd}</span>
                        <span class="branch" id="branch-\${session.id}" style="color:#7ee787; font-size:11px;"></span>
                    </div>
                    <div class="session-actions">
                        <div class="session-actions-row">
                            <span class="state \${session.state}" id="state-\${session.id}">\${session.state}</span>
                            <button class="collapse-btn" id="collapse-btn-\${session.id}" onclick="toggleCollapse('\${session.id}')" title="Collapse/expand">▼</button>
                        </div>
                        <div class="session-actions-row">
                            <button class="info-btn" id="info-btn-\${session.id}" onclick="toggleInfoPopover('\${session.id}')" title="Session info">ⓘ</button>
                            <span class="pin-indicator pinned" id="pin-\${session.id}" title="Auto-scroll enabled">⇣ pinned</span>
                        </div>
                    </div>
                </div>
                <div class="session-summary" id="summary-\${session.id}">
                    <span class="summary-item"><span class="label">Stats:</span> <span class="value" id="summary-stats-\${session.id}">—</span></span>
                    <span class="summary-item"><span class="label">Git:</span> <span class="value" id="summary-git-\${session.id}">—</span></span>
                    <span class="summary-item"><span class="label">Changes:</span> <span class="value" id="summary-changes-\${session.id}">—</span></span>
                </div>
                <div class="info-popover" id="info-popover-\${session.id}">
                    <div class="info-popover-row">
                        <span class="info-popover-label">Session ID</span>
                        <span class="info-popover-value copyable" onclick="copyToClipboard('\${session.id}')" title="Click to copy">\${session.id}</span>
                    </div>
                    <div class="info-popover-row">
                        <span class="info-popover-label">Started</span>
                        <span class="info-popover-value">\${startedAt}</span>
                    </div>
                    <div class="info-popover-row">
                        <span class="info-popover-label">Directory</span>
                        <span class="info-popover-value copyable" onclick="copyToClipboard('\${session.cwd}')" title="Click to copy">\${session.cwd}</span>
                    </div>
                    <div class="info-popover-row" id="info-model-\${session.id}" style="display:none">
                        <span class="info-popover-label">Model</span>
                        <span class="info-popover-value" id="info-model-value-\${session.id}">—</span>
                    </div>
                    <div class="info-popover-row" id="info-platform-\${session.id}" style="display:none">
                        <span class="info-popover-label">Platform</span>
                        <span class="info-popover-value" id="info-platform-value-\${session.id}">—</span>
                    </div>
                </div>
                <div class="terminal" id="terminal-\${session.id}" style="height:\${TERMINAL_HEIGHTS[currentHeightIndex]}px">
                    <div class="terminal-scrollback" id="scrollback-\${session.id}"></div>
                    <div class="terminal-separator" id="separator-\${session.id}">─── scrollback ───</div>
                    <div class="terminal-screen" id="screen-\${session.id}">Connecting...</div>
                </div>
                <div class="prompt-panel" id="prompt-\${session.id}">
                    <div class="prompt-header" id="prompt-header-\${session.id}"></div>
                    <div class="prompt-question" id="prompt-question-\${session.id}"></div>
                    <div class="prompt-options" id="prompt-options-\${session.id}"></div>
                    <div class="prompt-other" id="prompt-other-\${session.id}" style="display:none">
                        <input type="text" id="prompt-input-\${session.id}" placeholder="Type your response..."
                               onkeydown="if(event.key==='Enter'){event.preventDefault();sendOtherAnswer('\${session.id}');}">
                        <button type="button" onclick="sendOtherAnswer('\${session.id}')">Send</button>
                    </div>
                </div>
                <div class="widgets-panel" id="widgets-\${session.id}">
                    <div class="widget title-history-widget" id="titles-\${session.id}" style="display:none">
                        <div class="widget-title"><span style="color:#58a6ff">—</span></div>
                        <div class="titles-list"></div>
                    </div>
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
                           oninput="handleInputChange('\${session.id}', this.value)"
                           onkeydown="if(event.key==='Enter')sendAnswer('\${session.id}')">
                    <button type="button" onclick="sendAnswer('\${session.id}')">Send</button>
                </div>
            \`;
            container.appendChild(card);
            sessions.set(session.id, {
                element: card,
                state: session.state,
                title: null,
                git: null,
                changes: null,
                stats: null,
                permission: null,
                pinned: true,
                lastScrollTop: 0,
                // Scrollback chunking: store full buffer, render only visible portion
                scrollbackBuffer: [],      // Full scrollback lines
                scrollbackRendered: 0,     // How many lines currently rendered
            });
            applyCollapsedState(session.id);
            restoreInput(session.id);
            updateFitLayout();

            // Set up scroll tracking for pin/unpin behavior
            // Only unpin on explicit scroll UP gesture, re-pin when scrolling down to bottom
            const terminal = document.getElementById('terminal-' + session.id);
            if (terminal) {
                terminal.addEventListener('scroll', () => {
                    const sessionData = sessions.get(session.id);
                    if (!sessionData) return;

                    // Skip scroll handling during font size changes to preserve pin state
                    if (isChangingFontSize) return;

                    const currentScrollTop = terminal.scrollTop;
                    const prevScrollTop = sessionData.lastScrollTop || 0;
                    const scrollDelta = currentScrollTop - prevScrollTop;
                    const scrollingUp = scrollDelta < 0;
                    const scrollingDown = scrollDelta > 0;

                    // Update last scroll position
                    sessionData.lastScrollTop = currentScrollTop;

                    const distFromBottom = terminal.scrollHeight - currentScrollTop - terminal.clientHeight;
                    const atBottom = distFromBottom < 5;

                    // Ignore scroll bounce: small upward movements near the bottom are likely
                    // elastic bounce, not intentional scrolling. Require scrolling up past
                    // bounce threshold (50px from bottom) to unpin.
                    const pastBounceZone = distFromBottom > 50;

                    if (scrollingUp && sessionData.pinned && pastBounceZone) {
                        // Unpin only on explicit scroll UP past the bounce zone
                        sessionData.pinned = false;
                        updatePinButton(session.id, false);
                    } else if (scrollingDown && atBottom && !sessionData.pinned) {
                        // Re-pin when scrolling DOWN and reaching bottom
                        sessionData.pinned = true;
                        updatePinButton(session.id, true);
                    }

                    // Load more scrollback when scrolling near the top
                    if (currentScrollTop < 100 && sessionData.scrollbackBuffer && sessionData.scrollbackRendered < sessionData.scrollbackBuffer.length) {
                        loadMoreScrollback(session.id);
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
            const sessionData = sessions.get(session.id);
            if (sessionData) {
                sessionData.state = session.state;
                updateSessionSummary(session.id, sessionData);
            }
        }

        function updatePinButton(sessionId, pinned) {
            const el = document.getElementById('pin-' + sessionId);
            if (!el) return;
            const parent = el.parentNode;
            if (!parent) return;

            if (pinned) {
                // Show neutral indicator
                const indicator = document.createElement('span');
                indicator.id = 'pin-' + sessionId;
                indicator.className = 'pin-indicator pinned';
                indicator.title = 'Auto-scroll enabled';
                indicator.textContent = '⇣ pinned';
                parent.replaceChild(indicator, el);
            } else {
                // Show clickable button
                const btn = document.createElement('button');
                btn.id = 'pin-' + sessionId;
                btn.className = 'pin-btn';
                btn.title = 'Click to pin to bottom';
                btn.textContent = '⇣ Pin';
                btn.onclick = () => togglePin(sessionId);
                parent.replaceChild(btn, el);
            }
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
                sessionData.lastScrollTop = terminal.scrollTop;
            }
        }

        // Collapse state persistence
        function getCollapsedSessions() {
            try {
                return JSON.parse(localStorage.getItem('collapsedSessions') || '{}');
            } catch { return {}; }
        }

        function setCollapsedSession(sessionId, collapsed) {
            const state = getCollapsedSessions();
            if (collapsed) {
                state[sessionId] = true;
            } else {
                delete state[sessionId];
            }
            localStorage.setItem('collapsedSessions', JSON.stringify(state));
        }

        function isSessionCollapsed(sessionId) {
            return getCollapsedSessions()[sessionId] === true;
        }

        function toggleCollapse(sessionId) {
            const card = document.getElementById('session-' + sessionId);
            const btn = document.getElementById('collapse-btn-' + sessionId);
            if (!card || !btn) return;

            const isCollapsed = card.classList.contains('collapsed');
            if (isCollapsed) {
                card.classList.remove('collapsed');
                btn.classList.remove('collapsed');
                setCollapsedSession(sessionId, false);
            } else {
                card.classList.add('collapsed');
                btn.classList.add('collapsed');
                setCollapsedSession(sessionId, true);
                // Update summary when collapsing
                const sessionData = sessions.get(sessionId);
                if (sessionData) {
                    updateSessionSummary(sessionId, sessionData);
                }
            }
            updateFitLayout();
        }

        function applyCollapsedState(sessionId) {
            if (isSessionCollapsed(sessionId)) {
                const card = document.getElementById('session-' + sessionId);
                const btn = document.getElementById('collapse-btn-' + sessionId);
                if (card) card.classList.add('collapsed');
                if (btn) btn.classList.add('collapsed');
            }
        }

        function updateSessionSummary(sessionId, sessionData) {
            // Stats summary
            const statsEl = document.getElementById('summary-stats-' + sessionId);
            if (statsEl) {
                const s = sessionData.stats || {};
                const parts = [];
                // Show state first
                const state = sessionData.state || s.state;
                if (state) parts.push(state);
                if (s.prompts) parts.push(s.prompts + ' prompts');
                if (s.tools) parts.push(s.tools + ' tools');
                if (s.thinking_seconds) parts.push(formatDuration(s.thinking_seconds) + ' thinking');
                statsEl.textContent = parts.length ? parts.join(', ') : '—';
            }

            // Git summary
            const gitEl = document.getElementById('summary-git-' + sessionId);
            if (gitEl && sessionData.git) {
                const g = sessionData.git;
                const adds = g.files?.reduce((sum, f) => sum + (f.additions || 0), 0) || 0;
                const dels = g.files?.reduce((sum, f) => sum + (f.deletions || 0), 0) || 0;
                const fileCount = g.files?.length || 0;
                if (fileCount > 0) {
                    gitEl.innerHTML = '<span class="green">+' + adds + '</span> <span class="red">-' + dels + '</span> in ' + fileCount + ' file' + (fileCount !== 1 ? 's' : '');
                } else {
                    gitEl.textContent = 'clean';
                }
            }

            // Changes summary
            const changesEl = document.getElementById('summary-changes-' + sessionId);
            if (changesEl && sessionData.changes) {
                const c = sessionData.changes;
                const total = c.by_language?.reduce((sum, l) => sum + l.changes.length, 0) || 0;
                if (total > 0) {
                    const langs = c.by_language?.map(l => l.language).join(', ') || '';
                    changesEl.innerHTML = '<span class="purple">' + total + '</span> change' + (total !== 1 ? 's' : '') + (langs ? ' (' + langs + ')' : '');
                } else {
                    changesEl.textContent = 'none';
                }
            }
        }

        let activePopover = null;

        function toggleInfoPopover(sessionId) {
            const popover = document.getElementById('info-popover-' + sessionId);
            if (!popover) return;

            // Close any other open popover
            if (activePopover && activePopover !== popover) {
                activePopover.classList.remove('visible');
            }

            const isVisible = popover.classList.contains('visible');
            if (isVisible) {
                popover.classList.remove('visible');
                activePopover = null;
            } else {
                popover.classList.add('visible');
                activePopover = popover;
            }
        }

        function copyToClipboard(text) {
            navigator.clipboard.writeText(text).then(() => {
                // Brief visual feedback could be added here
                console.log('Copied to clipboard:', text);
            }).catch(err => {
                console.error('Failed to copy:', err);
            });
        }

        // Close popover when clicking outside
        document.addEventListener('click', (e) => {
            if (!activePopover) return;

            const target = e.target;
            // Check if click is on info button or inside popover
            const isInfoBtn = target.closest('.info-btn');
            const isInsidePopover = target.closest('.info-popover');

            if (!isInfoBtn && !isInsidePopover) {
                activePopover.classList.remove('visible');
                activePopover = null;
            }
        });

        function formatElapsed(timestamp) {
            if (!timestamp) return '';
            const now = Date.now() / 1000;
            const secs = Math.floor(now - timestamp);
            if (secs < 60) return 'just now';
            const mins = Math.floor(secs / 60);
            if (mins < 60) return mins + 'm ago';
            const hours = Math.floor(mins / 60);
            if (hours < 24) return hours + 'h ago';
            const days = Math.floor(hours / 24);
            return days + 'd ago';
        }

        function formatShortDate(timestamp) {
            if (!timestamp) return '';
            const date = new Date(timestamp * 1000);
            const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
            const month = months[date.getMonth()];
            const day = date.getDate();
            let hours = date.getHours();
            const mins = date.getMinutes().toString().padStart(2, '0');
            const ampm = hours >= 12 ? 'PM' : 'AM';
            hours = hours % 12 || 12;
            return month + ' ' + day + ', ' + hours + ':' + mins + ' ' + ampm;
        }

        function formatStartedAt(timestamp) {
            if (!timestamp) return 'Unknown';
            return formatElapsed(timestamp) + ' · ' + formatShortDate(timestamp);
        }

        function formatTime(timestamp) {
            if (!timestamp) return '—';
            const date = new Date(timestamp * 1000);
            return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        }

        function formatStateIndicator(state) {
            switch (state) {
                case 'ready': return '<span style="color:#8b949e">○ Ready</span>';
                case 'thinking': return '<span style="color:#3fb950">⠋</span>';
                case 'permission': return '<span style="color:#d29922">» ? « Perm</span>';
                case 'question': return '<span style="color:#db6d28">» ? « Ask</span>';
                case 'complete': return '<span style="color:#bc8cff">✓ Complete</span>';
                case 'interrupted': return '<span style="color:#f85149">⊘ Interrupted</span>';
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

            // Store mode and permission in session data for tracking
            if (session && stats.mode) {
                session.mode = stats.mode;
            }
            if (session) {
                session.permission = stats.permission || null;
            }

            // Update info popover with model/platform if available
            if (stats.model) {
                const modelRow = document.getElementById('info-model-' + sessionId);
                const modelValue = document.getElementById('info-model-value-' + sessionId);
                if (modelRow && modelValue) {
                    modelRow.style.display = '';
                    modelValue.textContent = stats.model;
                }
            }
            if (stats.platform) {
                const platformRow = document.getElementById('info-platform-' + sessionId);
                const platformValue = document.getElementById('info-platform-value-' + sessionId);
                if (platformRow && platformValue) {
                    platformRow.style.display = '';
                    platformValue.textContent = stats.platform;
                }
            }

            const promptsElapsed = stats.prompts_changed_at ? formatElapsed(stats.prompts_changed_at) : '';
            const completionsElapsed = stats.completions_changed_at ? formatElapsed(stats.completions_changed_at) : '';
            const compressionsElapsed = stats.compressions_changed_at ? formatElapsed(stats.compressions_changed_at) : '';

            const compressionsRow = stats.compressions > 0
                ? \`<div class="widget-row"><span class="widget-label">⊜ Compactions \${stats.compressions}</span><span class="widget-value" style="color:#8b949e">\${compressionsElapsed}</span></div>\`
                : '';

            // Show idle time when in complete/question state and idle > 60s
            let idleRow = '';
            const isIdleState = ['complete', 'question', 'interrupted'].includes(session?.state);
            if (isIdleState && stats.idle_since) {
                const idleSecs = Math.floor(Date.now() / 1000 - stats.idle_since);
                if (idleSecs >= 60) {
                    idleRow = \`<div class="widget-row"><span class="widget-label">◇ Idle</span><span class="widget-value" style="color:#8b949e">\${formatDuration(idleSecs)}</span></div>\`;
                }
            }

            const newHtml = \`
                <div class="widget-title"><span style="color:#bc8cff">Stats</span> <span style="float:right">\${stateIndicator}</span></div>
                <div class="widget-row"><span class="widget-label">◆ Session</span><span class="widget-value" style="color:#58a6ff">\${formatDuration(stats.work_seconds || 0)}</span></div>
                <div class="widget-row"><span class="widget-label">◇ Thinking</span><span class="widget-value" style="color:#3fb950">\${stats.thinking_seconds ? formatDuration(stats.thinking_seconds) : '—'}</span></div>
                <div class="widget-row"><span class="widget-label">▸ Prompts \${stats.prompts || 0}</span><span class="widget-value" style="color:#8b949e">\${promptsElapsed}</span></div>
                <div class="widget-row"><span class="widget-label">◂ Completions \${stats.completions || 0}</span><span class="widget-value" style="color:#8b949e">\${completionsElapsed}</span></div>
                <div class="widget-row"><span class="widget-label">⚙ Tools</span><span class="widget-value">\${renderToolsSparkline(stats.tool_timestamps, stats.session_start, Date.now() / 1000)}</span></div>
                \${compressionsRow}
                \${idleRow}
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
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

        // Unicode block characters for sparkline (8 levels + empty)
        const SPARKLINE_BLOCKS = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        const SPARKLINE_MAX = 10; // Fixed max: 10 tools = full height

        // Render tools sparkline histogram like CLI
        function renderToolsSparkline(timestamps, sessionStart, now) {
            if (!timestamps || !sessionStart || timestamps.length === 0) {
                return '<span style="color:#8b949e">—</span>';
            }

            const numBins = 20; // Number of histogram buckets
            const duration = now - sessionStart;
            if (duration <= 0) {
                return '<span style="color:#8b949e">—</span>';
            }

            const binSize = duration / numBins;
            const bins = new Array(numBins).fill(0);

            // Bin the timestamps
            for (const ts of timestamps) {
                if (ts >= sessionStart && ts <= now) {
                    const binIdx = Math.min(Math.floor((ts - sessionStart) / binSize), numBins - 1);
                    bins[binIdx]++;
                }
            }

            // Check if there's any activity
            const hasActivity = bins.some(c => c > 0);
            if (!hasActivity) {
                return '<span style="color:#8b949e">' + ' '.repeat(numBins) + '</span>';
            }

            // Build sparkline
            let sparkline = '';
            for (const count of bins) {
                if (count === 0) {
                    sparkline += SPARKLINE_BLOCKS[0];
                } else {
                    // Scale to 1-8 range using fixed max (absolute scale)
                    const scaled = Math.ceil((count / SPARKLINE_MAX) * 8);
                    const level = Math.max(1, Math.min(8, scaled));
                    sparkline += SPARKLINE_BLOCKS[level];
                }
            }

            return '<span style="color:#f0883e">' + sparkline + '</span>';
        }

        // Count digits in a number
        function digitCount(n) {
            if (n === 0) return 1;
            return Math.floor(Math.log10(n)) + 1;
        }

        // Create non-breaking spaces for HTML (regular spaces collapse)
        function nbsp(count) {
            return '&nbsp;'.repeat(Math.max(0, count));
        }

        // Compute column widths for git file diffs
        function computeGitColumnWidths(files) {
            let maxDel = 0;
            let maxAdd = 0;
            for (const f of files) {
                maxDel = Math.max(maxDel, f.deletions || 0);
                maxAdd = Math.max(maxAdd, f.additions || 0);
            }
            // Width for number columns: sign + digits
            const delNumWidth = maxDel > 0 ? 1 + digitCount(maxDel) : 0;
            const addNumWidth = maxAdd > 0 ? 1 + digitCount(maxAdd) : 0;
            // Bar width: symmetric based on max of both (log scale)
            const maxBar = Math.max(
                maxDel > 0 ? digitCount(maxDel) : 0,
                maxAdd > 0 ? digitCount(maxAdd) : 0
            );
            return { delNumWidth, addNumWidth, barWidth: maxBar };
        }

        // Compute column widths for semantic changes
        function computeChangesColumnWidths(byLanguage) {
            let maxDel = 0;
            let maxAdd = 0;
            for (const lang of byLanguage) {
                for (const c of (lang.changes || [])) {
                    maxDel = Math.max(maxDel, c.deletions || 0);
                    maxAdd = Math.max(maxAdd, c.additions || 0);
                }
            }
            // Width for number columns: sign + digits
            const delNumWidth = maxDel > 0 ? 1 + digitCount(maxDel) : 0;
            const addNumWidth = maxAdd > 0 ? 1 + digitCount(maxAdd) : 0;
            return { delNumWidth, addNumWidth };
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
            const branch = git.branch || 'unknown';

            // Update branch in header
            const branchEl = document.getElementById('branch-' + sessionId);
            if (branchEl) {
                branchEl.textContent = ' ' + branch;
            }

            // Compact display for clean repos - just header, no body
            if (totalFiles === 0) {
                const newHtml = \`<div class="widget-title"><span style="color:#7ee787">\${branch}</span> <span style="color:#3fb950">✓ Clean</span></div>\`;
                if (widget.innerHTML !== newHtml) {
                    widget.innerHTML = newHtml;
                }
                return;
            }

            // Header: branch on left, "N files" on right (like CLI)
            const filesLabel = totalFiles === 1 ? 'file' : 'files';
            const headerRight = '<span style="color:#d29922">' + totalFiles + ' ' + filesLabel + '</span>';

            // Compute column widths for alignment
            const { delNumWidth, addNumWidth, barWidth } = computeGitColumnWidths(files);

            let filesHtml = files.map(f => {
                const { icon, color } = getStatusIcon(f.status);
                const del = f.deletions || 0;
                const add = f.additions || 0;

                // 4-column layout: [del num] [bars] [add num]
                // Build deletion number (right-aligned)
                const delNumStr = del > 0 ? '−' + del : '';
                const delNumPad = delNumWidth - delNumStr.length;
                const delNumHtml = delNumWidth > 0
                    ? \`<span style="color:#f85149">\${nbsp(delNumPad)}\${delNumStr}</span>\`
                    : '';

                // Build combined bar (red left-padded, green right-padded, touching in middle)
                const delBarLen = del > 0 ? digitCount(del) : 0;
                const addBarLen = add > 0 ? digitCount(add) : 0;
                const delBarPad = barWidth - delBarLen;
                const addBarPad = barWidth - addBarLen;
                const redBars = '▓'.repeat(delBarLen);
                const greenBars = '█'.repeat(addBarLen);
                const barsHtml = barWidth > 0
                    ? \`\${nbsp(delBarPad)}<span style="display:inline-flex;gap:0"><span style="color:#f85149">\${redBars}</span><span style="color:#3fb950">\${greenBars}</span></span>\${nbsp(addBarPad)}\`
                    : '';

                // Build addition number (left-aligned)
                const addNumStr = add > 0 ? '+' + add : '';
                const addNumPad = addNumWidth - addNumStr.length;
                const addNumHtml = addNumWidth > 0
                    ? \`<span style="color:#3fb950">\${addNumStr}\${nbsp(addNumPad)}</span>\`
                    : '';

                return \`<div class="git-file">
                    <span style="color:\${color}">\${icon}</span>
                    <span class="path">\${f.path}</span>
                    <span class="diff">\${delNumHtml}\${barsHtml}\${addNumHtml}</span>
                </div>\`;
            }).join('');

            const newHtml = \`
                <div class="widget-title"><span style="color:#7ee787">\${branch}</span> <span style="float:right">\${headerRight}</span></div>
                <div class="git-files">\${filesHtml}</div>
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
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
            const widgetsPanel = document.getElementById('widgets-' + sessionId);
            if (!widget) return;

            const byLanguage = changes.by_language || [];

            if (byLanguage.length === 0) {
                // Hide widget entirely when no changes
                widget.classList.add('hidden-changes');
                widgetsPanel?.classList.add('no-changes');
                return;
            }

            // Show widget when there are changes
            widget.classList.remove('hidden-changes');
            widgetsPanel?.classList.remove('no-changes');

            // Build header: "Language N changes" (like CLI)
            const firstLang = byLanguage[0];
            const totalChanges = byLanguage.reduce((sum, lang) => sum + (lang.changes?.length || 0), 0);
            const changeWord = totalChanges === 1 ? 'change' : 'changes';

            // Compute column widths for alignment
            const { delNumWidth, addNumWidth } = computeChangesColumnWidths(byLanguage);

            let changesHtml = '';

            for (const lang of byLanguage) {
                // Add language header if multiple languages
                if (byLanguage.length > 1) {
                    const langCount = lang.changes?.length || 0;
                    const langWord = langCount === 1 ? 'change' : 'changes';
                    changesHtml += \`<div style="color:#db6d28;margin-top:4px;font-size:11px">\${lang.language} <span style="color:#8b949e">\${langCount} \${langWord}</span></div>\`;
                }

                for (const c of (lang.changes || [])) {
                    const { modifier, color: modColor } = getModifierStyle(c.change_type);
                    const { icon, color: iconColor } = getKindIcon(c.kind);
                    const del = c.deletions || 0;
                    const add = c.additions || 0;

                    // Build deletion number (right-aligned)
                    const delNumStr = del > 0 ? '−' + del : '';
                    const delNumPad = delNumWidth - delNumStr.length;
                    const delNumHtml = delNumWidth > 0
                        ? \`<span style="color:#f85149">\${nbsp(delNumPad)}\${delNumStr}</span>\`
                        : '';

                    // Build addition number (left-aligned)
                    const addNumStr = add > 0 ? '+' + add : '';
                    const addNumPad = addNumWidth - addNumStr.length;
                    const addNumHtml = addNumWidth > 0
                        ? \`<span style="color:#3fb950">\${addNumStr}\${nbsp(addNumPad)}</span>\`
                        : '';

                    changesHtml += \`<div class="change-item">
                        <span style="color:\${modColor}">\${modifier}</span><span style="color:\${iconColor}">\${icon}</span>
                        <span class="name">\${c.name}</span>
                        <span class="stats" style="margin-left:auto">\${delNumHtml}&nbsp;\${addNumHtml}</span>
                    </div>\`;
                }
            }

            const newHtml = \`
                <div class="widget-title"><span style="color:#db6d28">\${firstLang.language}</span> <span style="color:#8b949e">\${totalChanges} \${changeWord}</span></div>
                <div class="changes-list">\${changesHtml}</div>
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
        }

        function updateTitlesWidget(sessionId, titleHistory) {
            const widget = document.getElementById('titles-' + sessionId);
            if (!widget) return;

            // Hide widget if no titles
            if (!titleHistory || titleHistory.length === 0) {
                widget.style.display = 'none';
                return;
            }

            // Show widget
            widget.style.display = '';

            // Latest title is the widget title
            const latestTitle = titleHistory[titleHistory.length - 1];
            const escapedLatest = latestTitle.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

            // Previous titles are the history content (all except the last one)
            const previousTitles = titleHistory.slice(0, -1);

            let newHtml;
            if (previousTitles.length === 0) {
                // Just one title - show it as the module title, no content
                newHtml = \`
                    <div class="widget-title"><span style="color:#58a6ff">\${escapedLatest}</span></div>
                \`;
            } else {
                // Multiple titles - latest as title, previous as history (newest first)
                const historyHtml = previousTitles.slice().reverse().map(title => {
                    const escaped = title.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
                    return \`<div class="title-entry">\${escaped}</div>\`;
                }).join('');

                newHtml = \`
                    <div class="widget-title"><span style="color:#58a6ff">\${escapedLatest}</span></div>
                    <div class="titles-list">\${historyHtml}</div>
                \`;
            }

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
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

                // If all sessions disconnected and we had sessions before, likely a deploy
                if (sessions.size === 0 && hadSessionsBefore && !isDeploying) {
                    showDeployOverlay();
                    scheduleReconnect();
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
            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const separatorEl = document.getElementById('separator-' + sessionId);
            const screenEl = document.getElementById('screen-' + sessionId);
            if (!terminal || !card) return;

            const sessionData = sessions.get(sessionId);

            switch (event.type) {
                case 'screen':
                    // Full screen update - only update the screen section
                    if (screenEl) {
                        screenEl.innerHTML = ansiToHtml(event.content);
                    }
                    if (sessionData?.pinned) {
                        terminal.scrollTop = terminal.scrollHeight;
                        sessionData.lastScrollTop = terminal.scrollTop;
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
                        // Clear permission when leaving permission state
                        if (event.state !== 'permission') {
                            sessionData.permission = null;
                        }
                        updateStatsWidget(sessionId, sessionData.stats || {});
                        updateSessionSummary(sessionId, sessionData);
                    }
                    break;
                case 'scrollback':
                    // Append scrollback diff to scrollback section (chunked)
                    if (event.diff) {
                        appendScrollback(sessionId, event.diff);
                    }
                    break;
                case 'scrollback_history':
                    // Full scrollback history for late joiners (chunked - only render last N lines)
                    if (event.content) {
                        const lines = event.content.split('\\n').filter(line => line.length > 0);
                        renderScrollback(sessionId, lines);
                    }
                    break;
                case 'git':
                    if (sessionData) {
                        sessionData.git = event;
                        updateSessionSummary(sessionId, sessionData);
                    }
                    updateGitWidget(sessionId, event);
                    break;
                case 'changes':
                    if (sessionData) {
                        sessionData.changes = event;
                        updateSessionSummary(sessionId, sessionData);
                    }
                    updateChangesWidget(sessionId, event);
                    break;
                case 'stats':
                    // Store stats in session data
                    if (sessionData) {
                        sessionData.stats = event;
                        updateSessionSummary(sessionId, sessionData);
                    }
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
                    // Update titles widget with single title if no history yet
                    updateTitlesWidget(sessionId, [event.title]);
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
                case 'title_history':
                    updateTitlesWidget(sessionId, event.history);
                    break;
                case 'prompt':
                    // Interactive prompt (question or permission)
                    updatePromptPanel(sessionId, event.prompt);
                    break;
            }
        }

        function updatePromptPanel(sessionId, prompt) {
            const panel = document.getElementById('prompt-' + sessionId);
            const headerEl = document.getElementById('prompt-header-' + sessionId);
            const questionEl = document.getElementById('prompt-question-' + sessionId);
            const optionsEl = document.getElementById('prompt-options-' + sessionId);
            const otherEl = document.getElementById('prompt-other-' + sessionId);

            if (!panel) return;

            if (!prompt) {
                // Clear/hide prompt panel
                panel.classList.remove('visible');
                return;
            }

            panel.classList.add('visible');

            // Helper to render options as styled divs with numbers
            function renderOptions(options) {
                return options.map((opt, i) => {
                    const num = i + 1;
                    const desc = opt.description
                        ? '<div class="prompt-option-desc">' + escapeHtml(opt.description) + '</div>'
                        : '';
                    return '<div class="prompt-option" onclick="sendPromptAnswer(\\'' + sessionId + '\\', \\'' + opt.value + '\\')">' +
                           '<span class="prompt-option-number">' + num + '.</span>' +
                           '<span class="prompt-option-label">' + escapeHtml(opt.label) + '</span>' +
                           desc +
                           '</div>';
                }).join('');
            }

            if (prompt.prompt_type === 'question') {
                // AskUserQuestion prompt
                const q = prompt.questions[0];
                headerEl.textContent = q.header || 'Question';
                questionEl.textContent = q.question;

                // Render options
                optionsEl.innerHTML = renderOptions(q.options);

                // Show "Other" input if allowed
                if (q.allows_other !== false) {
                    otherEl.style.display = 'flex';
                } else {
                    otherEl.style.display = 'none';
                }

            } else if (prompt.prompt_type === 'permission') {
                // Permission prompt
                headerEl.textContent = 'Permission: ' + prompt.tool_name;

                // Show tool info
                let desc = 'Allow this action?';
                if (prompt.tool_input?.command) desc = 'Command: ' + prompt.tool_input.command;
                else if (prompt.tool_input?.file_path) desc = 'File: ' + prompt.tool_input.file_path;
                else if (prompt.tool_input?.description) desc = prompt.tool_input.description;
                questionEl.textContent = desc;

                // Render options
                optionsEl.innerHTML = renderOptions(prompt.options);

                // Hide "Other" input for simple permission prompts
                // Show only if there's a "Tab to add additional instructions" hint (detected by allows_other)
                otherEl.style.display = prompt.allows_other ? 'flex' : 'none';

            } else if (prompt.prompt_type === 'exit_plan') {
                // ExitPlanMode prompt - options parsed from screen
                headerEl.textContent = 'Exit Plan Mode';
                questionEl.textContent = 'Choose how to proceed:';

                // Render options
                optionsEl.innerHTML = renderOptions(prompt.options);

                // Hide "Other" input for simple exit plan prompts
                otherEl.style.display = prompt.allows_other ? 'flex' : 'none';
            }
        }

        function scrollToSession(sessionId) {
            const card = document.getElementById('session-' + sessionId);
            if (card) {
                card.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        }

        async function sendPromptAnswer(sessionId, value) {
            console.log('[sendPromptAnswer] called for session:', sessionId, 'value:', value);
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ text: value })
                });

                if (resp.ok) {
                    console.log('[sendPromptAnswer] success');
                    // Hide prompt panel immediately for responsive feel
                    const panel = document.getElementById('prompt-' + sessionId);
                    if (panel) panel.classList.remove('visible');
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    console.error('Failed to send prompt answer:', err);
                }
            } catch (err) {
                console.error('Failed to send prompt answer:', err);
            }
        }

        async function sendOtherAnswer(sessionId) {
            console.log('[sendOtherAnswer] called for session:', sessionId);
            const input = document.getElementById('prompt-input-' + sessionId);
            console.log('[sendOtherAnswer] input element:', input);
            const text = input?.value?.trim();
            console.log('[sendOtherAnswer] text value:', text);
            if (!text) {
                console.log('[sendOtherAnswer] empty text, returning');
                return;
            }

            try {
                console.log('[sendOtherAnswer] sending:', text);
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ text: text })
                });

                if (resp.ok) {
                    console.log('[sendOtherAnswer] success');
                    input.value = '';
                    input.blur(); // Hide mobile keyboard
                    // Hide prompt panel
                    const panel = document.getElementById('prompt-' + sessionId);
                    if (panel) panel.classList.remove('visible');
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    console.error('Failed to send other answer:', err);
                }
            } catch (err) {
                console.error('Failed to send other answer:', err);
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
                    input.blur(); // Hide mobile keyboard
                    // Cancel any pending debounced save to prevent race condition
                    if (inputSaveTimers.has(sessionId)) {
                        clearTimeout(inputSaveTimers.get(sessionId));
                        inputSaveTimers.delete(sessionId);
                    }
                    clearLocalInput(sessionId);
                    saveInputToServer(sessionId, ''); // Clear server draft too
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    alert('Error: ' + (err.error || 'Failed to send'));
                }
            } catch (err) {
                console.error('Failed to send answer:', err);
                alert('Failed to send: ' + err.message);
            }
        }

        function escapeHtml(text) {
            return text
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#39;');
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

                // If we were recently connected, this might be a deploy
                if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                    showDeployOverlay();
                }

                if (isDeploying) {
                    // Use fast deploy reconnect
                    scheduleReconnect();
                } else {
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

        // Detect when tab becomes visible again and check connection
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'visible') {
                // Check if SSE connection is still alive
                if (sessionListSource && sessionListSource.readyState === EventSource.CLOSED) {
                    console.log('SSE connection closed while tab was hidden, reconnecting...');
                    // Don't show deploy overlay for tab visibility changes - just reconnect silently
                    sseRetryCount = 0;
                    loadSessions();
                    connectSessionListStream();
                } else if (!sessionListSource) {
                    // No SSE connection at all - reconnect
                    console.log('No SSE connection, reconnecting...');
                    sseRetryCount = 0;
                    loadSessions();
                    connectSessionListStream();
                }
            }
        });

        // Initial load and connect to SSE for real-time updates
        setLayout(currentLayout);  // Apply saved layout preference
        loadSettingsFromServer();  // Load style preferences
        loadSessions();
        connectSessionListStream();
`;
