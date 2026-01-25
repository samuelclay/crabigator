// Dashboard JavaScript - style
export const styleJs = `
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
                        terminalHeightIndex: currentHeightIndex,
                        terminalWrapEnabled: terminalWrapEnabled,
                        widgetsExpanded: widgetsExpanded,
                        groupingMode: groupingMode
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
                    if (typeof data.terminalWrapEnabled === 'boolean') {
                        terminalWrapEnabled = data.terminalWrapEnabled;
                    }
                    if (typeof data.widgetsExpanded === 'boolean') {
                        widgetsExpanded = data.widgetsExpanded;
                    }
                    if (typeof data.groupingMode === 'string' && (data.groupingMode === 'all' || data.groupingMode === 'project')) {
                        const oldMode = groupingMode;
                        groupingMode = data.groupingMode;
                        localStorage.setItem('crabigator-grouping', groupingMode);
                        // Re-render if mode changed and sessions exist
                        if (oldMode !== groupingMode && sessions.size > 0) {
                            setTimeout(() => rerenderSessions(), 0);
                        }
                    }
                }
            } catch {}
            applyFontScale();
            applyTerminalHeight();
            applyTerminalWrap();
            applyWidgetsExpanded();
            applyGrouping();
        }

        // Terminal wrap mode
        function setTerminalWrap(enabled) {
            terminalWrapEnabled = enabled;
            applyTerminalWrap();
            saveSettingsToServer();
        }

        function applyTerminalWrap() {
            // Update button states
            document.querySelectorAll('[data-wrap]').forEach(btn => {
                const isWrap = btn.getAttribute('data-wrap') === 'wrap';
                btn.classList.toggle('active', isWrap === terminalWrapEnabled);
            });

            // Toggle scroll-mode class on all terminals
            document.querySelectorAll('.terminal').forEach(terminal => {
                terminal.classList.toggle('scroll-mode', !terminalWrapEnabled);
            });
        }

        // Widgets panel visibility
        function setWidgetsExpanded(expanded) {
            widgetsExpanded = expanded;
            // Clear per-session overrides so all sessions use global setting
            localStorage.removeItem('widgetsCollapsedSessions');
            applyWidgetsExpanded();
            saveSettingsToServer();
        }

        function applyWidgetsExpanded() {
            // Update button states in style popover
            document.querySelectorAll('[data-widgets]').forEach(btn => {
                const isExpanded = btn.getAttribute('data-widgets') === 'expanded';
                btn.classList.toggle('active', isExpanded === widgetsExpanded);
            });

            // Apply to all session cards
            document.querySelectorAll('.session-card').forEach(card => {
                if (widgetsExpanded) {
                    card.classList.remove('widgets-collapsed');
                } else {
                    card.classList.add('widgets-collapsed');
                }
            });
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
            const stylePopover = document.getElementById('style-popover');
            const styleBtn = document.getElementById('style-btn');
            const settingsPopover = document.getElementById('settings-popover');
            const settingsBtn = document.getElementById('settings-btn');

            if (stylePopover && styleBtn && !stylePopover.contains(e.target) && !styleBtn.contains(e.target)) {
                closeStylePopover();
            }
            if (settingsPopover && settingsBtn && !settingsPopover.contains(e.target) && !settingsBtn.contains(e.target)) {
                closeSettingsPopover();
            }
        });

        // Session grouping mode
        function setGrouping(mode) {
            groupingMode = mode;
            localStorage.setItem('crabigator-grouping', mode);
            applyGrouping();
            // Re-render the session list to apply grouping
            rerenderSessions();
        }

        function applyGrouping() {
            const container = document.getElementById('sessions');
            container.dataset.grouping = groupingMode;

            // Update button states
            document.querySelectorAll('[data-grouping]').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.grouping === groupingMode);
            });
        }

        function rerenderSessions() {
            const container = document.getElementById('sessions');

            // Don't re-render if not paired (preserve pairing gate)
            if (!isPaired) return;

            if (groupingMode === 'project') {
                // Group sessions by cwd
                const groups = new Map();
                for (const [id, sessionData] of sessions) {
                    const card = document.getElementById('session-' + id);
                    if (!card) continue;
                    const cwd = card.querySelector('.cwd')?.textContent || 'Unknown';
                    if (!groups.has(cwd)) {
                        groups.set(cwd, []);
                    }
                    groups.get(cwd).push({ id, card });
                }

                // Clear container
                container.innerHTML = '';

                // Create project groups
                for (const [cwd, sessionCards] of groups) {
                    const group = createProjectGroup(cwd, sessionCards);
                    container.appendChild(group);
                }

                // Show empty state if no sessions
                if (groups.size === 0) {
                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                }
            } else {
                // Flat mode - extract cards from groups if needed and append directly
                const existingCards = [];
                container.querySelectorAll('.session-card').forEach(card => {
                    existingCards.push(card);
                });

                // Remove project groups
                container.querySelectorAll('.project-group').forEach(g => g.remove());

                // Re-add cards directly to container
                existingCards.forEach(card => {
                    container.appendChild(card);
                });

                // Show empty state if no sessions
                if (sessions.size === 0 && !container.querySelector('.no-sessions')) {
                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                }
            }

            updateFitLayout();
        }

        function createProjectGroup(cwd, sessionCards) {
            const group = document.createElement('div');
            group.className = 'project-group';
            group.dataset.project = cwd;

            const isCollapsed = collapsedProjects.has(cwd);
            if (isCollapsed) {
                group.classList.add('collapsed');
            }

            // Get project name (last path component)
            const projectName = cwd.split('/').pop() || cwd;

            group.innerHTML = \`
                <div class="project-separator" onclick="toggleProjectGroup('\${escapeHtml(cwd)}')">
                    <div class="project-separator-content">
                        <span class="project-collapse-icon">▼</span>
                        <span class="project-name">\${escapeHtml(projectName)}</span>
                        <span class="project-path">\${escapeHtml(cwd)}</span>
                        <span class="project-count">\${sessionCards.length} session\${sessionCards.length !== 1 ? 's' : ''}</span>
                    </div>
                </div>
                <div class="project-sessions">
                    <div class="project-sessions-inner">
                        <div class="project-sessions-content"></div>
                    </div>
                </div>
            \`;

            const sessionsContent = group.querySelector('.project-sessions-content');
            sessionCards.forEach(({ card }) => {
                sessionsContent.appendChild(card);
            });

            // Set initial fit columns for this group
            updateProjectFitColumns(group, sessionCards.length);

            return group;
        }

        function toggleProjectGroup(cwd) {
            const group = document.querySelector(\`.project-group[data-project="\${CSS.escape(cwd)}"]\`);
            if (!group) return;

            const isNowCollapsed = group.classList.toggle('collapsed');

            if (isNowCollapsed) {
                collapsedProjects.add(cwd);
            } else {
                collapsedProjects.delete(cwd);
            }
            localStorage.setItem('crabigator-collapsed-projects', JSON.stringify([...collapsedProjects]));
        }

        function updateProjectGroupCount(cwd) {
            const group = document.querySelector(\`.project-group[data-project="\${CSS.escape(cwd)}"]\`);
            if (!group) return;

            const sessionsContent = group.querySelector('.project-sessions-content');
            const count = sessionsContent ? sessionsContent.querySelectorAll('.session-card').length : 0;
            const countEl = group.querySelector('.project-count');
            if (countEl) {
                countEl.textContent = count + ' session' + (count !== 1 ? 's' : '');
            }

            // Update fit columns for this group
            updateProjectFitColumns(group, count);

            // Remove empty groups
            if (count === 0) {
                group.remove();
            }
        }

        function updateProjectFitColumns(group, count) {
            const sessionsContent = group.querySelector('.project-sessions-content');
            if (!sessionsContent) return;

            // Calculate columns based on available width
            const containerWidth = document.getElementById('sessions').offsetWidth || window.innerWidth - 32;
            const minCardWidth = 450;  // Minimum comfortable card width
            const maxCols = Math.max(Math.floor(containerWidth / minCardWidth), 1);
            // Don't use more columns than we have sessions in this group
            const cols = Math.min(maxCols, count || 1);
            sessionsContent.style.setProperty('--group-fit-columns', cols);
        }

        function updateAllProjectFitColumns() {
            document.querySelectorAll('.project-group').forEach(group => {
                const sessionsContent = group.querySelector('.project-sessions-content');
                const count = sessionsContent ? sessionsContent.querySelectorAll('.session-card').length : 0;
                updateProjectFitColumns(group, count);
            });
        }

        // Settings popover
        function toggleSettingsPopover() {
            const popover = document.getElementById('settings-popover');
            const btn = document.getElementById('settings-btn');
            const isVisible = popover.classList.toggle('visible');
            btn.classList.toggle('active', isVisible);
            // Close style popover if open
            if (isVisible) closeStylePopover();
        }

        function closeSettingsPopover() {
            const popover = document.getElementById('settings-popover');
            const btn = document.getElementById('settings-btn');
            if (popover) popover.classList.remove('visible');
            if (btn) btn.classList.remove('active');
        }

        async function generateInviteCode() {
            const btn = document.getElementById('generate-invite-btn');
            const result = document.getElementById('invite-result');

            if (!mobileToken) {
                result.innerHTML = '<div style="color: #f85149; font-size: 12px;">Not paired yet</div>';
                result.classList.add('visible');
                return;
            }

            btn.disabled = true;
            btn.textContent = 'Generating…';

            try {
                const response = await fetch('/api/pairing/invite', {
                    method: 'POST',
                    headers: getAuthHeaders()
                });

                if (handleAuthFailure(response)) return;
                if (!response.ok) {
                    throw new Error('Failed to generate invite code');
                }

                const data = await response.json();
                const pairUrl = window.location.origin + '/pair/' + data.token;

                result.innerHTML = \`
                    <div class="invite-code">\${data.code}</div>
                    <div class="invite-hint">Enter this code on your other device</div>
                    <div class="invite-link">
                        <a href="\${pairUrl}" target="_blank">Or open this link →</a>
                    </div>
                \`;
                result.classList.add('visible');

            } catch (err) {
                result.innerHTML = '<div style="color: #f85149; font-size: 12px;">' + err.message + '</div>';
                result.classList.add('visible');
            } finally {
                btn.disabled = false;
                btn.textContent = 'Generate pairing code';
            }
        }

`;
