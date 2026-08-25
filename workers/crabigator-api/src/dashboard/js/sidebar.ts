// Dashboard JavaScript - sidebar
export const sidebarJs = `
        let sidebarActiveSessionId = null;
        let sidebarUpdateTimer = null;
        let sidebarLastRenderedHtml = null;

        // Throttled sidebar update - batches rapid stream events into single re-renders
        function scheduleSidebarUpdate() {
            if (sidebarUpdateTimer) return;
            sidebarUpdateTimer = setTimeout(() => {
                sidebarUpdateTimer = null;
                updateSidebarContent();
            }, 500);
        }

        function initSidebar() {
            const sidebar = document.getElementById('sidebar');
            const layout = document.getElementById('dashboard-layout');
            if (!sidebar || !layout) return;

            // Measure actual header height and set CSS variable
            const header = document.querySelector('.header');
            if (header) {
                const headerHeight = header.getBoundingClientRect().height;
                document.documentElement.style.setProperty('--header-height', headerHeight + 'px');
            }

            // Apply saved position
            layout.dataset.sidebarPosition = sidebarPosition;

            // Apply saved width
            document.documentElement.style.setProperty('--sidebar-width', sidebarWidth + 'px');

            // Apply saved density
            sidebar.classList.toggle('compact', sidebarDensity === 'compact');

            // Apply mode
            applySidebarMode();

            // Init resize drag
            initSidebarResize();

            // Init scroll spy
            initSidebarScrollSpy();

            // Session-item taps are delegated to the (stable) content element
            // instead of inline onclick on each item. The list re-renders while
            // live sessions stream, and if the tapped element is replaced
            // between finger-down and the browser's synthesized click, the
            // click never fires — so capture the session at pointerdown and
            // act on pointerup.
            const sidebarContent = document.getElementById('sidebar-content');
            if (sidebarContent) {
                let tap = null;
                sidebarContent.addEventListener('pointerdown', (e) => {
                    const item = e.target.closest('.session-item');
                    tap = item ? { id: item.dataset.sessionId, x: e.clientX, y: e.clientY } : null;
                });
                sidebarContent.addEventListener('pointerup', (e) => {
                    const started = tap;
                    tap = null;
                    if (!started || !started.id) return;
                    // A drag (scrolling the list) is not a tap.
                    if (Math.abs(e.clientX - started.x) > 10 || Math.abs(e.clientY - started.y) > 10) return;
                    handleSessionClick(started.id);
                });
                sidebarContent.addEventListener('pointercancel', () => { tap = null; });
            }

            // Any click inside a session card highlights it in the sidebar
            const sessionsContainer = document.getElementById('sessions');
            if (sessionsContainer) {
                sessionsContainer.addEventListener('click', (e) => {
                    const card = e.target.closest('.session-card');
                    if (!card) return;
                    const sessionId = card.id.replace('session-', '');
                    if (sessionId && sessionId !== sidebarActiveSessionId) {
                        sidebarActiveSessionId = sessionId;
                        updateSidebarActiveState();
                        const sidebarItem = document.querySelector('.sidebar .session-item[data-session-id="' + sessionId + '"]');
                        if (sidebarItem) sidebarItem.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                    }
                });
            }

            // Render content
            updateSidebarContent();

            // Update settings controls to match state
            updateSidebarSettingsControls();
        }

        function applySidebarMode() {
            const sidebar = document.getElementById('sidebar');
            const layout = document.getElementById('dashboard-layout');
            if (!sidebar || !layout) return;

            if (sidebarPinned) {
                // Pinned mode: persistent sidebar pushing content
                sidebar.classList.remove('popover-mode');
                sidebar.classList.remove('collapsed');
                layout.classList.remove('sidebar-collapsed');
                layout.classList.remove('sidebar-popover');
            } else {
                // Popover mode: sidebar starts hidden, toggled by sessions button
                sidebar.classList.add('popover-mode');
                sidebar.classList.add('collapsed');
                layout.classList.add('sidebar-collapsed');
                layout.classList.add('sidebar-popover');
            }
            updateSessionsButtonState();
            updateSidebarBackdrop();
        }

        function toggleSidebar() {
            const sidebar = document.getElementById('sidebar');
            if (!sidebar) return;

            if (sidebarPinned) {
                // If pinned, toggle acts as close/unpin
                closeSidebar();
                return;
            }

            // Popover mode: toggle visibility
            const isCollapsed = sidebar.classList.contains('collapsed');
            if (isCollapsed) {
                // Open popover
                sidebar.classList.remove('collapsed');
                closeStylePopover();
                closeSettingsPopover();
                updateSidebarContent();
            } else {
                // Close popover
                sidebar.classList.add('collapsed');
                closeSidebarSettings();
            }
            updateSessionsButtonState();
            updateSidebarBackdrop();
        }

        function pinSidebar() {
            sidebarPinned = true;
            localStorage.setItem('crabigator-sidebar-pinned', 'true');
            applySidebarMode();
            // Recalculate fit layout
            requestAnimationFrame(() => {
                if (typeof updateFitLayout === 'function') updateFitLayout();
            });
        }

        function closeSidebar() {
            sidebarPinned = false;
            localStorage.setItem('crabigator-sidebar-pinned', 'false');
            closeSidebarSettings();
            applySidebarMode();
            // Recalculate fit layout
            requestAnimationFrame(() => {
                if (typeof updateFitLayout === 'function') updateFitLayout();
            });
        }

        function updateSessionsButtonState() {
            const btn = document.getElementById('sessions-btn');
            const container = btn?.closest('.sessions-container');
            if (!btn) return;
            const sidebar = document.getElementById('sidebar');
            const isOpen = sidebar && !sidebar.classList.contains('collapsed');
            btn.classList.toggle('active', isOpen && !sidebarPinned);
            // Hide sessions button entirely when sidebar is pinned
            if (container) container.classList.toggle('sidebar-pinned-hidden', sidebarPinned);
        }

        function updateSidebarBackdrop() {
            const backdrop = document.getElementById('sidebar-backdrop');
            if (!backdrop) return;
            const sidebar = document.getElementById('sidebar');
            const isVisible = sidebar && !sidebar.classList.contains('collapsed');
            backdrop.classList.toggle('visible', isVisible && window.innerWidth <= 768);
        }

        function toggleSidebarSettings() {
            const popover = document.getElementById('sidebar-settings-popover');
            const btn = document.querySelector('.sidebar-settings-btn');
            if (!popover) return;

            const isVisible = popover.classList.toggle('visible');
            if (btn) btn.classList.toggle('active', isVisible);
        }

        function closeSidebarSettings() {
            const popover = document.getElementById('sidebar-settings-popover');
            const btn = document.querySelector('.sidebar-settings-btn');
            if (popover) popover.classList.remove('visible');
            if (btn) btn.classList.remove('active');
        }

        function setSidebarPosition(pos) {
            sidebarPosition = pos;
            localStorage.setItem('crabigator-sidebar-position', pos);
            const layout = document.getElementById('dashboard-layout');
            if (layout) layout.dataset.sidebarPosition = pos;
            updateSidebarSettingsControls();
        }

        function setSidebarDensity(density) {
            sidebarDensity = density;
            localStorage.setItem('crabigator-sidebar-density', density);
            const sidebar = document.getElementById('sidebar');
            if (sidebar) {
                sidebar.classList.toggle('compact', density === 'compact');
            }
            updateSidebarSettingsControls();
        }

        function setSessionClickAction(action) {
            sessionClickAction = action;
            localStorage.setItem('crabigator-click-action', action);
            updateSidebarSettingsControls();
        }

        function toggleSidebarStat(stat) {
            sidebarVisibleStats[stat] = !sidebarVisibleStats[stat];
            localStorage.setItem('crabigator-sidebar-stats', JSON.stringify(sidebarVisibleStats));
            updateSidebarContent();
            applySessionCardStatsVisibility();
            updateSidebarSettingsControls();
        }

        function applySessionCardStatsVisibility(root = document) {
            root.querySelectorAll('[data-card-stats]').forEach(element => {
                const stats = element.dataset.cardStats.split(' ');
                element.style.display = stats.some(stat => sidebarVisibleStats[stat]) ? '' : 'none';
            });
        }

        function updateSidebarSettingsControls() {
            // Position
            document.querySelectorAll('.sb-opt-position').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.position === sidebarPosition);
            });
            // Density
            document.querySelectorAll('.sb-opt-density').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.density === sidebarDensity);
            });
            // Click action
            document.querySelectorAll('.sb-opt-click').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.click === sessionClickAction);
            });
            // Stat checkboxes
            for (const [stat, visible] of Object.entries(sidebarVisibleStats)) {
                const cb = document.getElementById('sb-stat-' + stat);
                if (cb) cb.checked = visible;
            }
        }

        function handleSessionClick(sessionId) {
            if (isFocusedMode() || sessionClickAction === 'focus') {
                focusOnSession(sessionId);
            } else {
                scrollToSession(sessionId);
            }
        }

        function scrollToSession(sessionId) {
            // Suppress scroll spy so the programmatic scrollIntoView doesn't override our selection
            if (typeof window.suppressScrollSpy === 'function') window.suppressScrollSpy();

            const card = document.getElementById('session-' + sessionId);
            if (!card) return;

            // If session's project group is collapsed, expand it first
            const projectGroup = card.closest('.project-group');
            if (projectGroup && projectGroup.classList.contains('collapsed')) {
                projectGroup.classList.remove('collapsed');
                const cwd = projectGroup.dataset.project;
                if (cwd) {
                    collapsedProjects.delete(cwd);
                    localStorage.setItem('crabigator-collapsed-projects', JSON.stringify([...collapsedProjects]));
                }
            }

            // Also expand device group if collapsed
            const deviceGroup = card.closest('.device-group');
            if (deviceGroup && deviceGroup.classList.contains('collapsed')) {
                deviceGroup.classList.remove('collapsed');
                const device = deviceGroup.dataset.device;
                if (device) {
                    collapsedDevices.delete(device);
                    localStorage.setItem('crabigator-collapsed-devices', JSON.stringify([...collapsedDevices]));
                }
            }

            // If session card body is collapsed, expand it
            const body = document.getElementById('body-' + sessionId);
            const collapseBtn = document.getElementById('collapse-btn-' + sessionId);
            if (body && body.style.display === 'none') {
                body.style.display = '';
                if (collapseBtn) collapseBtn.textContent = '▼';
                const sessionData = sessions.get(sessionId);
                if (sessionData) {
                    const collapsedSessions = new Set(JSON.parse(localStorage.getItem('crabigator-collapsed-sessions') || '[]'));
                    collapsedSessions.delete(sessionId);
                    localStorage.setItem('crabigator-collapsed-sessions', JSON.stringify([...collapsedSessions]));
                }
            }

            // Smooth scroll to the card
            card.scrollIntoView({ behavior: 'smooth', block: 'center' });

            // Add highlight animation (remove first + reflow to retrigger if already highlighted)
            card.classList.remove('highlight');
            void card.offsetHeight;
            card.classList.add('highlight');
            card.addEventListener('animationend', () => {
                card.classList.remove('highlight');
            }, { once: true });

            // Update active session in sidebar
            sidebarActiveSessionId = sessionId;
            updateSidebarActiveState();
        }

        function updateSidebarActiveState() {
            // Remove previous active state
            document.querySelectorAll('.sidebar .session-item.sidebar-active').forEach(el => {
                el.classList.remove('sidebar-active');
            });
            // Add active state to current
            if (sidebarActiveSessionId) {
                const item = document.querySelector('.sidebar .session-item[data-session-id="' + sidebarActiveSessionId + '"]');
                if (item) item.classList.add('sidebar-active');
            }
        }

        function renderSidebarSessionItem(session) {
            const isFocused = sessionMatchesFocus(session);
            const isActive = sidebarActiveSessionId === session.id;
            const stats = session.stats;
            const vs = sidebarVisibleStats;

            const sessionTime = stats?.work_seconds ? formatDuration(stats.work_seconds) : '';
            const thinkingTime = stats?.thinking_seconds ? formatDuration(stats.thinking_seconds) : '';
            const promptsCount = stats?.prompts || 0;
            const promptsChangedAt = stats?.prompts_changed_at || 0;
            const promptsElapsed = promptsChangedAt ? formatElapsed(promptsChangedAt) : '—';
            const completionsCount = stats?.completions || 0;
            const completionsChangedAt = stats?.completions_changed_at || 0;
            const completionsElapsed = completionsChangedAt ? formatElapsed(completionsChangedAt) : '—';

            const hasStats = sessionTime || thinkingTime || promptsCount > 0 || completionsCount > 0
                || promptsChangedAt || completionsChangedAt;

            let statsHtml = '';
            if (vs.sessionTime) statsHtml += '<span class="si-stat"><span class="si-icon" style="color:#58a6ff">◉</span>' + (sessionTime || '—') + '</span>';
            if (vs.thinkingTime) statsHtml += '<span class="si-stat"><span class="si-icon" style="color:#3fb950">◐</span>' + (thinkingTime || '—') + '</span>';
            if (vs.prompts || vs.promptRecency) {
                statsHtml += '<span class="si-stat"><span class="si-icon" style="color:#8b949e">⟩</span>'
                    + (vs.prompts ? promptsCount : '')
                    + (vs.promptRecency ? '<span class="si-elapsed" data-recency-timestamp="' + promptsChangedAt + '">' + promptsElapsed + '</span>' : '')
                    + '</span>';
            }
            if (vs.completions || vs.completionRecency) {
                statsHtml += '<span class="si-stat"><span class="si-icon" style="color:#8b949e">⋖</span>'
                    + (vs.completions ? completionsCount : '')
                    + (vs.completionRecency ? '<span class="si-elapsed" data-recency-timestamp="' + completionsChangedAt + '">' + completionsElapsed + '</span>' : '')
                    + '</span>';
            }

            // Tools and compactions don't have matching data in popover stats, show count if available
            // (These stats are available in the session card widgets, but the popover format uses simpler stats)

            const classes = ['session-item'];
            if (isFocused) classes.push('focused');
            if (isActive) classes.push('sidebar-active');

            const isLoading = session.title === 'Untitled' && !sessions.has(session.id);
            const titleHtml = isLoading
                ? '<span class="session-item-title sidebar-shimmer"></span>'
                : '<span class="session-item-titles">'
                    + '<span class="session-item-title' + (session.hasOfficialTitle ? ' official' : '') + '">'
                    + escapeHtml(session.title) + '</span>'
                    + (session.generatedTitle
                        ? '<span class="session-item-generated-title">' + escapeHtml(session.generatedTitle) + '</span>'
                        : '')
                    + '</span>';

            return \`
                <div class="\${classes.join(' ')}" data-session-id="\${session.id}">
                    <div class="session-item-row">
                        \${titleHtml}
                        <span class="session-item-state \${session.state}">\${session.state}</span>
                    </div>
                    \${statsHtml ? '<div class="session-item-stats' + (hasStats ? '' : ' dim') + '">' + statsHtml + '</div>' : ''}
                </div>
            \`;
        }

        function updateSidebarContent() {
            const content = document.getElementById('sidebar-content');
            if (!content) return;

            // Replacing innerHTML destroys every session item, which kills any
            // tap in progress on mobile — so skip the write when nothing
            // visible changed.
            const render = (html) => {
                if (html === sidebarLastRenderedHtml) return false;
                sidebarLastRenderedHtml = html;
                content.innerHTML = html;
                return true;
            };

            const sidebarSessions = getSidebarSessions(allSessions);

            if (sidebarSessions.length === 0) {
                render('<div class="sessions-empty">' + getSessionsEmptyMessage(allSessions.length > 0) + '</div>');
                return;
            }

            function cleanDeviceName(name) {
                if (!name) return '';
                return name.replace(/\\.local$/, '');
            }

            // Group sessions by cwd, tracking timestamps for sorting
            const groups = new Map();
            for (const session of sidebarSessions) {
                const cwd = session.cwd || 'Unknown';
                if (!groups.has(cwd)) groups.set(cwd, { sessions: [], mostRecentTime: 0 });

                const liveData = sessions.get(session.id);
                const titleHierarchy = sessionTitleHierarchy(liveData, session.title || 'Untitled');
                const startedAt = getSessionStartedTime(session);
                const activityAt = getSessionActivityTime(session);
                const g = groups.get(cwd);
                g.sessions.push({
                    id: session.id,
                    client_session_id: session.client_session_id,
                    title: titleHierarchy.main || 'Untitled',
                    generatedTitle: titleHierarchy.generated,
                    hasOfficialTitle: titleHierarchy.hasOfficial,
                    state: liveData?.state || session.state || 'ready',
                    stats: liveData?.stats || session.stats || null,
                    deviceName: session.device_name || liveData?.deviceName || null,
                    startedAt,
                    last_activity_at: activityAt
                });
                if (startedAt > g.mostRecentTime) g.mostRecentTime = startedAt;
            }

            // Sort sessions within each group by startedAt, newest first for quick switching.
            for (const [, g] of groups) {
                g.sessions.sort((a, b) => b.startedAt - a.startedAt);
            }

            // Sort projects using same logic as main content
            function sortedProjectKeys(projectMap) {
                if (projectOrderMode === 'alpha') {
                    return [...projectMap.keys()].sort((a, b) => {
                        const nameA = a.split('/').pop()?.toLowerCase() || a;
                        const nameB = b.split('/').pop()?.toLowerCase() || b;
                        return nameA.localeCompare(nameB);
                    });
                }
                return [...projectMap.keys()].sort((a, b) => {
                    return projectMap.get(b).mostRecentTime - projectMap.get(a).mostRecentTime;
                });
            }

            const allDeviceNames = new Set(sidebarSessions.map(s => s.device_name).filter(Boolean));
            const multiDevice = allDeviceNames.size > 1;

            let html = '';

            if (multiDevice) {
                const deviceGroups = new Map();
                for (const [cwd, g] of groups) {
                    for (const session of g.sessions) {
                        const device = session.deviceName || 'Unknown';
                        if (!deviceGroups.has(device)) deviceGroups.set(device, { projects: new Map(), mostRecentTime: 0 });
                        const dg = deviceGroups.get(device);
                        if (!dg.projects.has(cwd)) dg.projects.set(cwd, { sessions: [], mostRecentTime: 0 });
                        const pg = dg.projects.get(cwd);
                        pg.sessions.push(session);
                        if (session.startedAt > pg.mostRecentTime) pg.mostRecentTime = session.startedAt;
                        if (session.startedAt > dg.mostRecentTime) dg.mostRecentTime = session.startedAt;
                    }
                }

                // Sort devices by most recent session (matching main content)
                const sortedDevices = [...deviceGroups.keys()].sort((a, b) => {
                    return deviceGroups.get(b).mostRecentTime - deviceGroups.get(a).mostRecentTime;
                });

                for (const device of sortedDevices) {
                    const dg = deviceGroups.get(device);
                    const sortedCwds = sortedProjectKeys(dg.projects);
                    html += \`
                        <div class="sessions-device-section">
                            <div class="sessions-device-header"><span class="sessions-device-dot">●</span> \${escapeHtml(cleanDeviceName(device))}</div>
                            <div class="sessions-device-projects">
                    \`;
                    for (const cwd of sortedCwds) {
                        const pg = dg.projects.get(cwd);
                        const projectName = cwd.split('/').pop() || cwd;
                        html += \`
                            <div class="sessions-group">
                                <div class="sessions-group-header">
                                    <span class="sessions-group-name">\${escapeHtml(projectName)}</span>
                                    <span class="sessions-group-path">\${escapeHtml(cwd)}</span>
                                    <span class="sessions-group-count">\${pg.sessions.length}</span>
                                </div>
                        \`;
                        for (const session of pg.sessions) {
                            html += renderSidebarSessionItem(session);
                        }
                        html += '</div>';
                    }
                    html += '</div></div>';
                }
            } else {
                const sortedCwds = sortedProjectKeys(groups);
                for (const cwd of sortedCwds) {
                    const g = groups.get(cwd);
                    const projectName = cwd.split('/').pop() || cwd;
                    html += \`
                        <div class="sessions-group">
                            <div class="sessions-group-header">
                                <span class="sessions-group-name">\${escapeHtml(projectName)}</span>
                                <span class="sessions-group-path">\${escapeHtml(cwd)}</span>
                                <span class="sessions-group-count">\${g.sessions.length}</span>
                            </div>
                    \`;
                    for (const session of g.sessions) {
                        html += renderSidebarSessionItem(session);
                    }
                    html += '</div>';
                }
            }

            if (render(html)) updateSidebarActiveState();
        }

        function initSidebarScrollSpy() {
            let scrollSpyTimer = null;
            let scrollSpyEnabled = true;

            // Track which session cards are visible using IntersectionObserver
            const visibleCards = new Map(); // sessionId -> intersectionRatio

            // Only re-enable scroll spy on real user scrolling (wheel/touch), not programmatic scrollIntoView
            window.addEventListener('wheel', () => { scrollSpyEnabled = true; }, { passive: true });
            window.addEventListener('touchmove', () => { scrollSpyEnabled = true; }, { passive: true });

            // Suppress scroll spy after clicks - stays suppressed until next wheel/touch
            window.suppressScrollSpy = function() {
                scrollSpyEnabled = false;
            };

            const observer = new IntersectionObserver((entries) => {
                for (const entry of entries) {
                    const id = entry.target.id.replace('session-', '');
                    if (entry.isIntersecting) {
                        visibleCards.set(id, entry.intersectionRatio);
                    } else {
                        visibleCards.delete(id);
                    }
                }
                // Throttle updates
                if (scrollSpyTimer) return;
                scrollSpyTimer = setTimeout(() => {
                    scrollSpyTimer = null;
                    if (scrollSpyEnabled) {
                        updateScrollSpyActive();
                    }
                }, 150);
            }, {
                rootMargin: '-' + (parseInt(getComputedStyle(document.documentElement).getPropertyValue('--header-height')) || 67) + 'px 0px 0px 0px',
                threshold: [0, 0.25, 0.5, 0.75, 1]
            });

            function updateScrollSpyActive() {
                if (visibleCards.size === 0) return;
                // Pick the card with highest intersection ratio
                let bestId = null;
                let bestRatio = 0;
                for (const [id, ratio] of visibleCards) {
                    if (ratio > bestRatio) {
                        bestRatio = ratio;
                        bestId = id;
                    }
                }
                if (bestId && bestId !== sidebarActiveSessionId) {
                    sidebarActiveSessionId = bestId;
                    updateSidebarActiveState();
                    // Scroll the sidebar item into view if needed
                    const sidebarItem = document.querySelector('.sidebar .session-item[data-session-id="' + bestId + '"]');
                    if (sidebarItem) {
                        sidebarItem.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                    }
                }
            }

            // Observe all existing session cards
            function observeAllCards() {
                observer.disconnect();
                visibleCards.clear();
                document.querySelectorAll('.session-card[id^="session-"]').forEach(card => {
                    observer.observe(card);
                });
            }

            // Re-observe when sessions change (MutationObserver on the container)
            const container = document.getElementById('sessions');
            if (container) {
                const mutObs = new MutationObserver(() => {
                    // Debounce re-observation
                    setTimeout(observeAllCards, 200);
                });
                mutObs.observe(container, { childList: true, subtree: true });
            }

            // Also observe on first load
            observeAllCards();
        }

        function initSidebarResize() {
            const handle = document.getElementById('sidebar-resize-handle');
            if (!handle) return;

            let isResizing = false;
            let startX = 0;
            let startWidth = 0;

            handle.addEventListener('mousedown', (e) => {
                isResizing = true;
                startX = e.clientX;
                startWidth = sidebarWidth;
                handle.classList.add('active');
                document.body.style.cursor = 'col-resize';
                document.body.style.userSelect = 'none';
                e.preventDefault();
            });

            document.addEventListener('mousemove', (e) => {
                if (!isResizing) return;

                let delta;
                if (sidebarPosition === 'left') {
                    delta = e.clientX - startX;
                } else {
                    delta = startX - e.clientX;
                }

                const newWidth = Math.min(450, Math.max(220, startWidth + delta));
                sidebarWidth = newWidth;
                document.documentElement.style.setProperty('--sidebar-width', newWidth + 'px');
            });

            document.addEventListener('mouseup', () => {
                if (!isResizing) return;
                isResizing = false;
                handle.classList.remove('active');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                localStorage.setItem('crabigator-sidebar-width', sidebarWidth.toString());

                // Recalculate fit layout after resize
                if (typeof updateFitLayout === 'function') updateFitLayout();
            });
        }
`;
