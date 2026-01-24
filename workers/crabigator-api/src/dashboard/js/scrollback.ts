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
            const newLines = newContent.split('\\n');
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

`;
