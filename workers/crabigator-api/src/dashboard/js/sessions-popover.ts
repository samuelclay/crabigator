// Dashboard JavaScript - sessions popover
export const sessionsPopoverJs = `
        // Duration update interval for sessions popover
        let sessionsPopoverDurationInterval = null;

        function toggleSessionsPopover() {
            const popover = document.getElementById('sessions-popover');
            const btn = document.getElementById('sessions-btn');
            const isVisible = popover.classList.toggle('visible');
            btn.classList.toggle('active', isVisible);

            // Close other popovers
            if (isVisible) {
                closeStylePopover();
                closeSettingsPopover();
                updateSessionsPopover();
                // Start duration update interval
                sessionsPopoverDurationInterval = setInterval(updateSessionsPopoverDurations, 60000);
            } else {
                // Stop duration updates when closed
                if (sessionsPopoverDurationInterval) {
                    clearInterval(sessionsPopoverDurationInterval);
                    sessionsPopoverDurationInterval = null;
                }
            }
        }

        function closeSessionsPopover() {
            const popover = document.getElementById('sessions-popover');
            const btn = document.getElementById('sessions-btn');
            if (popover) popover.classList.remove('visible');
            if (btn) btn.classList.remove('active');
            if (sessionsPopoverDurationInterval) {
                clearInterval(sessionsPopoverDurationInterval);
                sessionsPopoverDurationInterval = null;
            }
        }

        function updateSessionsPopover() {
            const content = document.getElementById('sessions-popover-content');
            if (!content) return;

            if (allSessions.length === 0) {
                content.innerHTML = '<div class="sessions-empty">No active sessions</div>';
                return;
            }

            // Group sessions by cwd
            const groups = new Map();
            for (const session of allSessions) {
                const cwd = session.cwd || 'Unknown';
                if (!groups.has(cwd)) {
                    groups.set(cwd, []);
                }

                // Get live data from sessions Map if available
                const liveData = sessions.get(session.id);

                groups.get(cwd).push({
                    id: session.id,
                    title: liveData?.title || session.title || 'Untitled',
                    branch: liveData?.git?.branch || '',
                    state: liveData?.state || session.state || 'ready',
                    startedAt: session.started_at
                });
            }

            // Render groups
            let html = '';
            for (const [cwd, sessionList] of groups) {
                const projectName = cwd.split('/').pop() || cwd;
                html += \`
                    <div class="sessions-group">
                        <div class="sessions-group-header">
                            <span class="sessions-group-name">\${escapeHtml(projectName)}</span>
                            <span class="sessions-group-path">\${escapeHtml(cwd)}</span>
                            <span class="sessions-group-count">\${sessionList.length}</span>
                        </div>
                \`;

                for (const session of sessionList) {
                    const duration = session.startedAt ? formatSessionDuration(session.startedAt) : '';
                    const isFocused = singleSessionId && session.id === singleSessionId;
                    html += \`
                        <div class="session-item\${isFocused ? ' focused' : ''}" onclick="selectSessionFromPopover('\${session.id}')">
                            <div class="session-item-row">
                                <span class="session-item-title">\${escapeHtml(session.title)}</span>
                                <span class="session-item-state \${session.state}">\${session.state}</span>
                            </div>
                            <div class="session-item-meta">
                                \${session.branch ? '<span class="session-item-branch">' + escapeHtml(session.branch) + '</span>' : ''}
                                \${duration ? '<span class="session-item-duration" data-started="' + session.startedAt + '">' + duration + '</span>' : ''}
                            </div>
                        </div>
                    \`;
                }
                html += '</div>';
            }

            content.innerHTML = html;
        }

        function formatSessionDuration(startedAt) {
            if (!startedAt) return '';
            const now = Math.floor(Date.now() / 1000);
            const elapsed = now - startedAt;
            if (elapsed < 60) return 'just now';
            const mins = Math.floor(elapsed / 60);
            if (mins < 60) return mins + 'm';
            const hours = Math.floor(mins / 60);
            const remainingMins = mins % 60;
            if (hours < 24) return hours + 'h ' + remainingMins + 'm';
            const days = Math.floor(hours / 24);
            return days + 'd ' + (hours % 24) + 'h';
        }

        function updateSessionsPopoverDurations() {
            document.querySelectorAll('.session-item-duration[data-started]').forEach(el => {
                const startedAt = parseInt(el.dataset.started);
                if (startedAt) {
                    el.textContent = formatSessionDuration(startedAt);
                }
            });
        }

        function selectSessionFromPopover(sessionId) {
            closeSessionsPopover();
            focusOnSession(sessionId);
        }

        function updateSessionsCount() {
            const countEl = document.getElementById('sessions-count');
            const labelEl = document.querySelector('.sessions-label');
            const count = allSessions.length;
            if (countEl) {
                countEl.textContent = count;
            }
            if (labelEl) {
                labelEl.textContent = count === 1 ? 'session' : 'sessions';
            }
            // Update popover content if it's open
            const popover = document.getElementById('sessions-popover');
            if (popover && popover.classList.contains('visible')) {
                updateSessionsPopover();
            }
        }
`;
