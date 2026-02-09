// Dashboard JavaScript - scrollback
export const scrollbackJs = `
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
            scrollbackEl.insertAdjacentHTML('afterbegin', newHtml);
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

        // Render initial scrollback - buffer only, render lazily on scroll activate
        function renderScrollback(sessionId, lines) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            // Store full buffer (rendered lazily when scroll is activated)
            sessionData.scrollbackBuffer = lines;
            sessionData.scrollbackRendered = 0;

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
            const separatorEl = document.getElementById('separator-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl) return;

            const lines = sessionData.scrollbackBuffer;
            const linesToRender = Math.min(SCROLLBACK_CHUNK_SIZE, lines.length);
            const startIdx = lines.length - linesToRender;
            const visibleLines = lines.slice(startIdx);

            scrollbackEl.innerHTML = ansiToHtml(visibleLines.join('\\n') + '\\n');
            scrollbackEl.classList.add('has-content');
            if (separatorEl) separatorEl.classList.add('visible');

            sessionData.scrollbackRendered = linesToRender;
            updateScrollbackIndicator(sessionId, linesToRender, lines.length);

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
            const newLines = newContent.split('\\n');
            if (!sessionData.scrollbackBuffer) sessionData.scrollbackBuffer = [];
            sessionData.scrollbackBuffer.push(...newLines);

            // Only render to DOM if scroll is active on this terminal
            if (activeTerminalId !== sessionId) return;

            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const separatorEl = document.getElementById('separator-' + sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!scrollbackEl) return;

            scrollbackEl.insertAdjacentHTML('beforeend', ansiToHtml(newContent));
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

`;
