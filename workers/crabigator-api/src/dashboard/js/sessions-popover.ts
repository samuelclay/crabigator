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
                    startedAt: session.started_at,
                    stats: liveData?.stats || null
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
                    const isFocused = singleSessionId && session.id === singleSessionId;
                    const stats = session.stats;

                    // Format stats for display
                    const sessionTime = stats?.work_seconds ? formatDuration(stats.work_seconds) : '';
                    const thinkingTime = stats?.thinking_seconds ? formatDuration(stats.thinking_seconds) : '';
                    const promptsCount = stats?.prompts || 0;
                    const promptsElapsed = stats?.prompts_changed_at ? formatElapsed(stats.prompts_changed_at) : '';
                    const completionsCount = stats?.completions || 0;
                    const completionsElapsed = stats?.completions_changed_at ? formatElapsed(stats.completions_changed_at) : '';

                    html += \`
                        <div class="session-item\${isFocused ? ' focused' : ''}" onclick="selectSessionFromPopover('\${session.id}')">
                            <div class="session-item-row">
                                <span class="session-item-title">\${escapeHtml(session.title)}</span>
                                <span class="session-item-state \${session.state}">\${session.state}</span>
                            </div>
                            <div class="session-item-stats">
                                <span class="si-stat"><span style="color:#58a6ff">◉</span> \${sessionTime || '—'}</span>
                                <span class="si-stat"><span style="color:#3fb950">◐</span> \${thinkingTime || '—'}</span>
                                <span class="si-stat"><span style="color:#8b949e">⟩</span> \${promptsCount}\${promptsElapsed ? ' <span class="si-elapsed">' + promptsElapsed + '</span>' : ''}</span>
                                <span class="si-stat"><span style="color:#8b949e">⋗</span> \${completionsCount}\${completionsElapsed ? ' <span class="si-elapsed">' + completionsElapsed + '</span>' : ''}</span>
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
