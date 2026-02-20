// Dashboard JavaScript - sessions popover
export const sessionsPopoverJs = `
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
            }
        }

        function closeSessionsPopover() {
            const popover = document.getElementById('sessions-popover');
            const btn = document.getElementById('sessions-btn');
            if (popover) popover.classList.remove('visible');
            if (btn) btn.classList.remove('active');
        }

        function updateSessionsPopover() {
            const content = document.getElementById('sessions-popover-content');
            if (!content) return;

            if (allSessions.length === 0) {
                content.innerHTML = '<div class="sessions-empty">No active sessions</div>';
                return;
            }

            // Strip .local suffix from device names for display
            function cleanDeviceName(name) {
                if (!name) return '';
                return name.replace(/\\.local$/, '');
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
                    state: liveData?.state || session.state || 'ready',
                    stats: liveData?.stats || session.stats || null,
                    deviceName: session.device_name || liveData?.deviceName || null
                });
            }

            // Render a single session item
            function renderSessionItem(session) {
                const isFocused = singleSessionId && session.id === singleSessionId;
                const stats = session.stats;
                const sessionTime = stats?.work_seconds ? formatDuration(stats.work_seconds) : '';
                const thinkingTime = stats?.thinking_seconds ? formatDuration(stats.thinking_seconds) : '';
                const promptsCount = stats?.prompts || 0;
                const promptsElapsed = stats?.prompts_changed_at ? formatElapsed(stats.prompts_changed_at) : '';
                const completionsCount = stats?.completions || 0;
                const completionsElapsed = stats?.completions_changed_at ? formatElapsed(stats.completions_changed_at) : '';

                // Dim the stats row when no meaningful data
                const hasStats = sessionTime || thinkingTime || promptsCount > 0 || completionsCount > 0;

                return \`
                    <div class="session-item\${isFocused ? ' focused' : ''}" onclick="selectSessionFromPopover('\${session.id}')">
                        <div class="session-item-row">
                            <span class="session-item-title">\${escapeHtml(session.title)}</span>
                            <span class="session-item-state \${session.state}">\${session.state}</span>
                        </div>
                        <div class="session-item-stats\${hasStats ? '' : ' dim'}">
                            <span class="si-stat"><span class="si-icon" style="color:#58a6ff">◉</span>\${sessionTime || '—'}</span>
                            <span class="si-stat"><span class="si-icon" style="color:#3fb950">◐</span>\${thinkingTime || '—'}</span>
                            <span class="si-stat"><span class="si-icon" style="color:#8b949e">⟩</span>\${promptsCount}\${promptsElapsed ? '<span class="si-elapsed">' + promptsElapsed + '</span>' : ''}</span>
                            <span class="si-stat"><span class="si-icon" style="color:#8b949e">⋗</span>\${completionsCount}\${completionsElapsed ? '<span class="si-elapsed">' + completionsElapsed + '</span>' : ''}</span>
                        </div>
                    </div>
                \`;
            }

            // Check if there are multiple devices
            const allDeviceNames = new Set(allSessions.map(s => s.device_name).filter(Boolean));
            const multiDevice = allDeviceNames.size > 1;

            let html = '';

            if (multiDevice) {
                // Device > Project > Sessions
                const deviceGroups = new Map();
                for (const [cwd, sessionList] of groups) {
                    for (const session of sessionList) {
                        const device = session.deviceName || 'Unknown';
                        if (!deviceGroups.has(device)) deviceGroups.set(device, new Map());
                        const dg = deviceGroups.get(device);
                        if (!dg.has(cwd)) dg.set(cwd, []);
                        dg.get(cwd).push(session);
                    }
                }

                for (const [device, projects] of deviceGroups) {
                    html += \`
                        <div class="sessions-device-section">
                            <div class="sessions-device-header"><span class="sessions-device-dot">●</span> \${escapeHtml(cleanDeviceName(device))}</div>
                            <div class="sessions-device-projects">
                    \`;
                    for (const [cwd, sessionList] of projects) {
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
                            html += renderSessionItem(session);
                        }
                        html += '</div>';
                    }
                    html += '</div></div>';
                }
            } else {
                // Single device — project groups only
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
                        html += renderSessionItem(session);
                    }
                    html += '</div>';
                }
            }

            content.innerHTML = html;
        }

        function selectSessionFromPopover(sessionId) {
            closeSessionsPopover();
            handleSessionClick(sessionId);
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
            // Update sidebar content
            if (typeof updateSidebarContent === 'function') {
                updateSidebarContent();
            }
        }
`;
