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

            // Remember scroll position to maintain view after prepending
            const prevScrollHeight = terminal.scrollHeight;

            // Prepend new content
            scrollbackEl.insertAdjacentHTML('afterbegin', newHtml);
            scrollbackEl.classList.add('has-content');

            // Update rendered count
            sessionData.scrollbackRendered = alreadyRendered + linesToLoad;

            // Restore scroll position (content was added above, so scroll down by the added height)
            const newScrollHeight = terminal.scrollHeight;
            terminal.scrollTop += (newScrollHeight - prevScrollHeight);
            sessionData.lastScrollTop = terminal.scrollTop;

        }

        // Render initial scrollback - buffer only, render lazily on scroll activate
        function renderScrollback(sessionId, lines) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            // Another viewer joining can resend the same history. Keep the
            // reader's loaded range and scroll position in that case.
            const previous = sessionData.scrollbackBuffer;
            if (previous && previous.length === lines.length && previous.every((line, i) => line === lines[i])) return;

            // Store full buffer (rendered lazily when scroll is activated)
            sessionData.scrollbackBuffer = lines;
            sessionData.scrollbackRendered = 0;
            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            if (scrollbackEl) {
                scrollbackEl.innerHTML = '';
                scrollbackEl.classList.remove('has-content');
            }

            // If scroll is already active on this terminal, render now
            if (activeTerminalId === sessionId) {
                flushScrollback(sessionId);
            }
        }

        // Flush buffered scrollback to the DOM (called on scroll activate)
        function flushScrollback(sessionId) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData || !sessionData.scrollbackBuffer || sessionData.scrollbackRendered > 0) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl) return;

            const lines = sessionData.scrollbackBuffer;
            if (!lines.length) return;
            const linesToRender = Math.min(SCROLLBACK_CHUNK_SIZE, lines.length);
            const startIdx = lines.length - linesToRender;
            const visibleLines = lines.slice(startIdx);

            scrollbackEl.innerHTML = scrollbackHtml(visibleLines);
            scrollbackEl.classList.add('has-content');

            sessionData.scrollbackRendered = linesToRender;

            if (sessionData.pinned && terminal) {
                terminal.scrollTop = terminal.scrollHeight;
                sessionData.lastScrollTop = terminal.scrollTop;
            }
        }

        // Append new scrollback content - buffer only unless scroll is active
        function appendScrollback(sessionId, newContent) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            // Always buffer
            const newLines = scrollbackLines(newContent);
            if (!sessionData.scrollbackBuffer) sessionData.scrollbackBuffer = [];
            sessionData.scrollbackBuffer.push(...newLines);

            // Only render to DOM if scroll is active on this terminal
            if (activeTerminalId !== sessionId) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl) return;

            scrollbackEl.insertAdjacentHTML('beforeend', scrollbackHtml(newLines));
            scrollbackEl.classList.add('has-content');

            sessionData.scrollbackRendered = (sessionData.scrollbackRendered || 0) + newLines.length;

            // Auto-scroll if pinned
            if (sessionData.pinned && terminal) {
                terminal.scrollTop = terminal.scrollHeight;
                sessionData.lastScrollTop = terminal.scrollTop;
            }
        }

`;
