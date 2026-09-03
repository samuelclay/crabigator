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
                        groupingMode: groupingMode,
                        projectOrderMode: projectOrderMode,
                        visibleSections: visibleSections
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
                    if (typeof data.projectOrderMode === 'string' && (data.projectOrderMode === 'recent' || data.projectOrderMode === 'alpha')) {
                        projectOrderMode = data.projectOrderMode;
                        localStorage.setItem('crabigator-project-order', projectOrderMode);
                    }
                    if (data.visibleSections && typeof data.visibleSections === 'object') {
                        visibleSections = { ...visibleSections, ...data.visibleSections };
                    }
                }
            } catch {}
            applyFontScale();
            applyTerminalHeight();
            applyTerminalWrap();
            applyWidgetsExpanded();
            applyGrouping();
            applyProjectOrder();
            applySectionVisibility();
        }

        // Session-card section visibility (recap, PRs, commits, git status, changes)
        function toggleSection(section) {
            visibleSections[section] = !visibleSections[section];
            applySectionVisibility();
            saveSettingsToServer();
        }

        function applySectionVisibility() {
            // Sections hidden purely by CSS body classes
            document.body.classList.toggle('hide-section-recap', !visibleSections.recap);
            document.body.classList.toggle('hide-section-prs', !visibleSections.prs);
            document.body.classList.toggle('hide-section-git', !visibleSections.git);

            // Sync checkbox states in the Style popover
            for (const [section, visible] of Object.entries(visibleSections)) {
                const cb = document.getElementById('section-' + section);
                if (cb) cb.checked = visible;
            }

            // Commits and changes render inside the Changes widget, which sizes
            // its header from the data, so re-render it for every session.
            for (const [sessionId, sessionData] of sessions) {
                updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
            }
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
                card.classList.toggle('widgets-collapsed', !widgetsExpanded);
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
            const sidebarSettingsPopover = document.getElementById('sidebar-settings-popover');
            const sidebarSettingsBtn = document.querySelector('.sidebar-settings-btn');

            if (stylePopover && styleBtn && !stylePopover.contains(e.target) && !styleBtn.contains(e.target)) {
                closeStylePopover();
            }
            if (settingsPopover && settingsBtn && !settingsPopover.contains(e.target) && !settingsBtn.contains(e.target)) {
                closeSettingsPopover();
            }
            if (sidebarSettingsPopover && sidebarSettingsBtn && !sidebarSettingsPopover.contains(e.target) && !sidebarSettingsBtn.contains(e.target)) {
                closeSidebarSettings();
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
            const effectiveGroupingMode = getMainGroupingMode();
            container.dataset.grouping = effectiveGroupingMode;

            // Update button states
            document.querySelectorAll('[data-grouping]').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.grouping === groupingMode);
            });

            // Show/hide project order section based on grouping mode
            const projectOrderSection = document.getElementById('project-order-section');
            if (projectOrderSection) {
                projectOrderSection.style.display = effectiveGroupingMode === 'project' ? '' : 'none';
            }
        }

        // Project ordering mode (when grouped by project)
        function setProjectOrder(mode) {
            projectOrderMode = mode;
            localStorage.setItem('crabigator-project-order', mode);
            applyProjectOrder();
            // Re-render the session list to apply ordering
            rerenderSessions();
        }

        function applyProjectOrder() {
            // Update button states
            document.querySelectorAll('[data-project-order]').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.projectOrder === projectOrderMode);
            });
        }

        function getScrollAnchorReferenceY() {
            const headerHeight = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--header-height'), 10);
            return (Number.isFinite(headerHeight) ? headerHeight : 67) + 12;
        }

        function capturePageScrollAnchor() {
            const scrollEl = document.scrollingElement || document.documentElement;
            const fallbackTop = scrollEl ? scrollEl.scrollTop : 0;
            const fallbackLeft = scrollEl ? scrollEl.scrollLeft : 0;
            const referenceY = getScrollAnchorReferenceY();
            const cards = Array.from(document.querySelectorAll('.session-card[id^="session-"]'));
            let best = null;
            let bestDistance = Number.POSITIVE_INFINITY;

            for (const card of cards) {
                const rect = card.getBoundingClientRect();
                if (rect.bottom <= referenceY) {
                    continue;
                }

                const intersectsReference = rect.top <= referenceY && rect.bottom >= referenceY;
                const distance = intersectsReference ? 0 : Math.abs(rect.top - referenceY);
                if (distance < bestDistance) {
                    bestDistance = distance;
                    best = {
                        sessionId: card.id.replace('session-', ''),
                        offsetFromCardTop: referenceY - rect.top,
                    };
                }

                if (intersectsReference) {
                    break;
                }
            }

            return {
                sessionId: best?.sessionId || null,
                offsetFromCardTop: best?.offsetFromCardTop || 0,
                fallbackTop,
                fallbackLeft,
                referenceY,
            };
        }

        function restorePageScrollAnchor(anchor) {
            const scrollEl = document.scrollingElement || document.documentElement;
            if (!scrollEl || !anchor) return;

            const restorePageScroll = () => {
                const maxScrollTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
                let targetTop = anchor.fallbackTop;

                if (anchor.sessionId) {
                    const card = document.getElementById('session-' + anchor.sessionId);
                    if (card) {
                        const cardTop = card.getBoundingClientRect().top + scrollEl.scrollTop;
                        targetTop = cardTop + anchor.offsetFromCardTop - anchor.referenceY;
                    }
                }

                scrollEl.scrollTop = Math.min(Math.max(0, targetTop), maxScrollTop);
                scrollEl.scrollLeft = anchor.fallbackLeft;
            };

            restorePageScroll();
            requestAnimationFrame(restorePageScroll);
            requestAnimationFrame(() => requestAnimationFrame(restorePageScroll));
        }

        function capturePageScrollRestorer() {
            const anchor = capturePageScrollAnchor();
            return () => restorePageScrollAnchor(anchor);
        }

        function preservePageScroll(callback) {
            const restorePageScroll = capturePageScrollRestorer();

            try {
                callback();
            } finally {
                restorePageScroll();
            }
        }

        function rerenderSessions() {
            const container = document.getElementById('sessions');

            // Don't re-render if not paired (preserve pairing gate)
            if (!isPaired) return;

            preservePageScroll(() => {
                if (getMainGroupingMode() === 'project') {
                // Collect all device names across sessions
                const allDevices = new Set();
                for (const [, sessionData] of sessions) {
                    if (sessionData.deviceName) allDevices.add(sessionData.deviceName);
                }
                const multiDevice = allDevices.size > 1;

                if (multiDevice) {
                    // Device > Project > Sessions hierarchy
                    // Build deviceGroups: deviceName -> Map<cwd, { sessions[], mostRecentTime }>
                    const deviceGroups = new Map();
                    for (const [id, sessionData] of sessions) {
                        const card = document.getElementById('session-' + id);
                        if (!card) continue;
                        const cwd = sessionData.cwd || card.querySelector('.cwd')?.textContent || 'Unknown';
                        const device = sessionData.deviceName || 'Unknown';
                        if (!deviceGroups.has(device)) {
                            deviceGroups.set(device, { projects: new Map(), mostRecentTime: 0 });
                        }
                        const dg = deviceGroups.get(device);
                        if (!dg.projects.has(cwd)) {
                            dg.projects.set(cwd, { sessions: [], mostRecentTime: 0 });
                        }
                        const pg = dg.projects.get(cwd);
                        pg.sessions.push({ id, card, startedAt: getSessionStartedTime(sessionData) });
                        const sessionTime = sessionData.lastActivityAt || sessionData.startedAt || 0;
                        if (sessionTime > pg.mostRecentTime) pg.mostRecentTime = sessionTime;
                        if (sessionTime > dg.mostRecentTime) dg.mostRecentTime = sessionTime;
                    }

                    // Sort devices by most recent session
                    const sortedDevices = [...deviceGroups.keys()].sort((a, b) => {
                        return deviceGroups.get(b).mostRecentTime - deviceGroups.get(a).mostRecentTime;
                    });

                    // Clear container
                    container.innerHTML = '';

                    for (const device of sortedDevices) {
                        const dg = deviceGroups.get(device);

                        // Sort projects within this device
                        for (const [, pg] of dg.projects) {
                            pg.sessions.sort((a, b) => a.startedAt - b.startedAt);
                        }
                        let sortedCwds;
                        if (projectOrderMode === 'alpha') {
                            sortedCwds = [...dg.projects.keys()].sort((a, b) => {
                                const nameA = a.split('/').pop()?.toLowerCase() || a;
                                const nameB = b.split('/').pop()?.toLowerCase() || b;
                                return nameA.localeCompare(nameB);
                            });
                        } else {
                            sortedCwds = [...dg.projects.keys()].sort((a, b) => {
                                return dg.projects.get(b).mostRecentTime - dg.projects.get(a).mostRecentTime;
                            });
                        }

                        // Count total sessions for this device
                        let deviceSessionCount = 0;
                        for (const [, pg] of dg.projects) deviceSessionCount += pg.sessions.length;

                        const deviceEl = createDeviceGroup(device, sortedCwds, dg.projects, deviceSessionCount);
                        container.appendChild(deviceEl);
                    }

                    if (deviceGroups.size === 0) {
                        container.innerHTML = '<div class="no-sessions">' + getSessionsEmptyMessage(allSessions.length > 0) + '</div>';
                    }
                } else {
                    // Single device — flat project grouping (no device headers)
                    const groups = new Map();
                    for (const [id, sessionData] of sessions) {
                        const card = document.getElementById('session-' + id);
                        if (!card) continue;
                        const cwd = sessionData.cwd || card.querySelector('.cwd')?.textContent || 'Unknown';
                        if (!groups.has(cwd)) {
                            groups.set(cwd, { sessions: [], mostRecentTime: 0 });
                        }
                        const group = groups.get(cwd);
                        group.sessions.push({ id, card, startedAt: getSessionStartedTime(sessionData) });
                        const sessionTime = sessionData.lastActivityAt || sessionData.startedAt || 0;
                        if (sessionTime > group.mostRecentTime) group.mostRecentTime = sessionTime;
                    }

                    // Add empty groups for historical projects (only in single-device mode)
                    if (!isFocusedMode() && allProjects && allProjects.length > 0 && hiddenSessionCount === 0) {
                        for (const project of allProjects) {
                            if (!groups.has(project.cwd)) {
                                groups.set(project.cwd, { sessions: [], mostRecentTime: project.last_active || 0 });
                            }
                        }
                    }

                    for (const [, group] of groups) {
                        group.sessions.sort((a, b) => a.startedAt - b.startedAt);
                    }

                    let sortedCwds;
                    if (projectOrderMode === 'alpha') {
                        sortedCwds = [...groups.keys()].sort((a, b) => {
                            const nameA = a.split('/').pop()?.toLowerCase() || a;
                            const nameB = b.split('/').pop()?.toLowerCase() || b;
                            return nameA.localeCompare(nameB);
                        });
                    } else {
                        sortedCwds = [...groups.keys()].sort((a, b) => {
                            return groups.get(b).mostRecentTime - groups.get(a).mostRecentTime;
                        });
                    }

                    container.innerHTML = '';
                    for (const cwd of sortedCwds) {
                        const g = groups.get(cwd);
                        const group = createProjectGroup(cwd, g.sessions);
                        container.appendChild(group);
                    }

                    if (groups.size === 0) {
                        container.innerHTML = '<div class="no-sessions">' + getSessionsEmptyMessage(allSessions.length > 0) + '</div>';
                    }
                }
            } else {
                // Flat mode - extract cards from groups if needed and append directly
                // Sort by session startedAt (oldest first, newest last) for determinism
                const sessionList = [];
                for (const [id, sessionData] of sessions) {
                    const card = document.getElementById('session-' + id);
                    if (card) {
                        sessionList.push({ id, card, startedAt: sessionData.startedAt || 0 });
                    }
                }
                sessionList.sort((a, b) => a.startedAt - b.startedAt);

                // Remove project/device groups
                container.querySelectorAll('.project-group, .device-group').forEach(g => g.remove());

                // Re-add cards directly to container in sorted order
                sessionList.forEach(({ card }) => {
                    container.appendChild(card);
                });

                // Show empty state if no sessions
                if (sessions.size === 0 && !container.querySelector('.no-sessions')) {
                    container.innerHTML = '<div class="no-sessions">' + getSessionsEmptyMessage(allSessions.length > 0) + '</div>';
                }
            }

                updateFitLayout();
            });
        }

        function createProjectGroup(cwd, sessionCards) {
            const group = document.createElement('div');
            group.className = 'project-group';
            group.dataset.project = cwd;

            const isCollapsed = collapsedProjects.has(cwd);
            if (isCollapsed) {
                group.classList.add('collapsed');
            }
            if (sessionCards.length === 0) {
                group.classList.add('empty');
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
                        <button class="project-add-btn" onclick="event.stopPropagation(); spawnSession('\${escapeHtml(cwd)}')" title="New terminal in \${escapeHtml(projectName)}">+</button>
                        \${sessionCards.length === 0 ? \`<button class="project-close-btn" onclick="event.stopPropagation(); closeProject('\${escapeHtml(cwd)}')" title="Remove project">×</button>\` : ''}
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

        async function spawnSession(cwd) {
            try {
                const resp = await fetch(API_BASE + '/spawn', {
                    method: 'POST',
                    headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
                    body: JSON.stringify({ cwd }),
                });
                const data = await resp.json().catch(() => ({}));
                if (data.fallback === 'url_scheme' && data.url) {
                    window.location.href = data.url;
                    return;
                }
                if (!resp.ok) {
                    console.error('Spawn failed:', data.error || resp.statusText);
                }
            } catch (err) {
                console.error('Spawn error:', err);
                window.location.href = 'crabigator://spawn?cwd=' + encodeURIComponent(cwd);
            }
        }

        async function closeProject(cwd) {
            try {
                await fetch(API_BASE + '/projects', {
                    method: 'DELETE',
                    headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
                    body: JSON.stringify({ cwd }),
                });
            } catch (err) {
                console.error('Close project error:', err);
            }
            // Remove from allProjects
            if (allProjects) {
                allProjects = allProjects.filter(p => p.cwd !== cwd);
            }
            // Remove the project group from DOM
            const group = document.querySelector(\`.project-group[data-project="\${CSS.escape(cwd)}"]\`);
            if (group) {
                group.remove();
            }
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

        function createDeviceGroup(deviceName, sortedCwds, projectsMap, sessionCount) {
            const group = document.createElement('div');
            group.className = 'device-group';
            group.dataset.device = deviceName;

            const isCollapsed = collapsedDevices.has(deviceName);
            if (isCollapsed) {
                group.classList.add('collapsed');
            }

            const cleanName = deviceName.replace(/\\.local$/, '');

            group.innerHTML = \`
                <div class="device-separator" onclick="toggleDeviceGroup('\${escapeHtml(deviceName)}')">
                    <div class="device-separator-content">
                        <span class="device-collapse-icon">●</span>
                        <span class="device-name">\${escapeHtml(cleanName)}</span>
                        <span class="device-count">\${sessionCount} session\${sessionCount !== 1 ? 's' : ''}</span>
                    </div>
                </div>
                <div class="device-sessions">
                    <div class="device-sessions-inner"></div>
                </div>
            \`;

            const inner = group.querySelector('.device-sessions-inner');
            for (const cwd of sortedCwds) {
                const pg = projectsMap.get(cwd);
                const projectEl = createProjectGroup(cwd, pg.sessions);
                inner.appendChild(projectEl);
            }

            return group;
        }

        function toggleDeviceGroup(deviceName) {
            const group = document.querySelector(\`.device-group[data-device="\${CSS.escape(deviceName)}"]\`);
            if (!group) return;

            const isNowCollapsed = group.classList.toggle('collapsed');

            if (isNowCollapsed) {
                collapsedDevices.add(deviceName);
            } else {
                collapsedDevices.delete(deviceName);
            }
            localStorage.setItem('crabigator-collapsed-devices', JSON.stringify([...collapsedDevices]));
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

            // Toggle empty state styling and close button
            const separatorContent = group.querySelector('.project-separator-content');
            if (count === 0) {
                group.classList.add('empty');
                // Add close button if not already present
                if (separatorContent && !separatorContent.querySelector('.project-close-btn')) {
                    const closeBtn = document.createElement('button');
                    closeBtn.className = 'project-close-btn';
                    closeBtn.title = 'Remove project';
                    closeBtn.textContent = '×';
                    closeBtn.onclick = (e) => { e.stopPropagation(); closeProject(cwd); };
                    separatorContent.appendChild(closeBtn);
                }
            } else {
                group.classList.remove('empty');
                // Remove close button if present
                const closeBtn = separatorContent?.querySelector('.project-close-btn');
                if (closeBtn) closeBtn.remove();
            }
        }

        function updateProjectFitColumns(group, count) {
            const sessionsContent = group.querySelector('.project-sessions-content');
            if (!sessionsContent) return;

            // Calculate columns based on available width
            const containerWidth = document.getElementById('sessions').offsetWidth || window.innerWidth - 32;
            const minCardWidth = 550;  // Minimum card width before adding another column
            const maxCols = Math.min(Math.max(Math.floor(containerWidth / minCardWidth), 1), 4);
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
