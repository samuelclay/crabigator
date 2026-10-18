// Dashboard JavaScript - scrollback
export const scrollbackJs = `
        // Events end with a row delimiter, not an extra blank terminal row.
        function scrollbackLines(content) {
            if (!content) return [];
            const lines = content.split('\\n');
            if (lines[lines.length - 1] === '') lines.pop();
            return lines;
        }

        function scrollbackHtml(lines) {
            // Even a history consisting of one blank row must occupy a cell row.
            return lines.length ? ansiToHtml(lines.join('\\n') || '\\x1b[0m') : '';
        }

        function loadMoreScrollback(sessionId) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData || !sessionData.scrollbackBuffer) return;

            const buffer = sessionData.scrollbackBuffer;
            const alreadyRendered = sessionData.scrollbackRendered || 0;
            const totalLines = buffer.length;

            if (alreadyRendered >= totalLines) return; // Already showing everything

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl || !terminal) return;

            // Calculate how many more lines to load
            const linesToLoad = Math.min(SCROLLBACK_CHUNK_SIZE, totalLines - alreadyRendered);
            const startIdx = totalLines - alreadyRendered - linesToLoad;
            const endIdx = totalLines - alreadyRendered;

            // Get the lines to prepend (they go at the beginning)
            const newLines = buffer.slice(startIdx, endIdx);
            const newHtml = scrollbackHtml(newLines);

            preserveTerminalScroll(sessionId, () => {
                scrollbackEl.insertAdjacentHTML('afterbegin', newHtml);
                scrollbackEl.classList.add('has-content');
                sessionData.scrollbackRendered = alreadyRendered + linesToLoad;
            });
        }

        // Find the retained rows when the server trims the start of history.
        // Prefix matching keeps this linear even for thousands of blank rows.
        function scrollbackOverlap(previous, next) {
            if (!next.length) return 0;
            const prefix = new Array(next.length).fill(0);
            for (let i = 1, matched = 0; i < next.length; i++) {
                while (matched && next[i] !== next[matched]) matched = prefix[matched - 1];
                if (next[i] === next[matched]) matched++;
                prefix[i] = matched;
            }
            let matched = 0;
            for (const line of previous) {
                while (matched && (matched === next.length || line !== next[matched])) matched = prefix[matched - 1];
                if (line === next[matched]) matched++;
            }
            return matched;
        }

        // Render initial scrollback - buffer only, render lazily on scroll activate
        function renderScrollback(sessionId, lines) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            // Another viewer joining can resend the same history. Keep the
            // reader's loaded range and scroll position in that case.
            const previous = sessionData.scrollbackBuffer;
            if (previous && previous.length === lines.length && previous.every((line, i) => line === lines[i])) return;

            const rendered = sessionData.scrollbackRendered || 0;
            const overlap = scrollbackOverlap(previous || [], lines);
            preserveTerminalScroll(sessionId, () => {
                sessionData.scrollbackBuffer = lines;
                const scrollbackEl = document.getElementById('scrollback-' + sessionId);
                if (!scrollbackEl) return;
                if (rendered && overlap) {
                    // Keep overlapping DOM rows, including the row being read.
                    const removed = Math.max(0, rendered - overlap);
                    for (let i = 0; i < removed; i++) scrollbackEl.firstElementChild?.remove();
                    scrollbackEl.insertAdjacentHTML('beforeend', scrollbackHtml(lines.slice(overlap)));
                    sessionData.scrollbackRendered = rendered - removed + lines.length - overlap;
                    scrollbackEl.classList.toggle('has-content', sessionData.scrollbackRendered > 0);
                } else {
                    sessionData.scrollbackRendered = 0;
                    scrollbackEl.innerHTML = '';
                    scrollbackEl.classList.remove('has-content');
                    if (rendered || activeTerminalId === sessionId) flushScrollback(sessionId, Math.max(rendered, SCROLLBACK_CHUNK_SIZE));
                }
            });
        }

        // Flush buffered scrollback to the DOM (called on scroll activate)
        function flushScrollback(sessionId, count = SCROLLBACK_CHUNK_SIZE) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData || !sessionData.scrollbackBuffer || sessionData.scrollbackRendered > 0) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            if (!scrollbackEl) return;

            const lines = sessionData.scrollbackBuffer;
            if (!lines.length) return;
            const linesToRender = Math.min(count, lines.length);
            const startIdx = lines.length - linesToRender;
            const visibleLines = lines.slice(startIdx);

            preserveTerminalScroll(sessionId, () => {
                scrollbackEl.innerHTML = scrollbackHtml(visibleLines);
                scrollbackEl.classList.add('has-content');
                sessionData.scrollbackRendered = linesToRender;
            });
        }

        // Append history; terminals with no displayed history remain lazy.
        function appendScrollback(sessionId, newContent) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            // Always buffer
            const newLines = scrollbackLines(newContent);
            if (!sessionData.scrollbackBuffer) sessionData.scrollbackBuffer = [];
            sessionData.scrollbackBuffer.push(...newLines);

            // Once history is displayed, keep it current even while another
            // terminal is active. Unopened history remains lazy.
            if (activeTerminalId !== sessionId && !sessionData.scrollbackRendered) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            if (!scrollbackEl) return;

            preserveTerminalScroll(sessionId, () => {
                scrollbackEl.insertAdjacentHTML('beforeend', scrollbackHtml(newLines));
                scrollbackEl.classList.add('has-content');
                sessionData.scrollbackRendered = (sessionData.scrollbackRendered || 0) + newLines.length;
            });
        }

`;
