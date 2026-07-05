// Dashboard JavaScript - session
import { iconMicrophone, iconKeyboard, iconClose, iconPencil } from '../icons';

export const sessionJs = `
        function getSessionActivityTime(session) {
            const raw = session?.last_activity_at || session?.lastActivityAt || session?.last_seen_at || session?.last_seen || session?.started_at || session?.startedAt || 0;
            return raw > 1000000000000 ? raw / 1000 : raw;
        }

        function getSessionStartedTime(session) {
            const raw = session?.started_at || session?.startedAt || 0;
            return raw > 1000000000000 ? raw / 1000 : raw;
        }

        function sortSessionsByRecentActivity(sessionList) {
            return [...sessionList].sort((a, b) => {
                const activityDelta = getSessionActivityTime(b) - getSessionActivityTime(a);
                if (activityDelta !== 0) return activityDelta;
                return (b.started_at || 0) - (a.started_at || 0);
            });
        }

        function sortSessionsByNewestStart(sessionList) {
            return [...sessionList].sort((a, b) => {
                const startedDelta = getSessionStartedTime(b) - getSessionStartedTime(a);
                if (startedDelta !== 0) return startedDelta;
                return getSessionActivityTime(b) - getSessionActivityTime(a);
            });
        }

        function getSessionVisibilityLimit() {
            return isProUser ? Infinity : FREE_VISIBLE_SESSION_LIMIT;
        }

        function getRenderableSessions(sessionList) {
            const candidates = singleSessionId
                ? sessionList.filter(session => sessionMatchesFocus(session))
                : [...sessionList];
            const limit = getSessionVisibilityLimit();

            let renderable;
            if (!Number.isFinite(limit)) {
                // Pro / single-session view: no cap, no lock.
                renderable = candidates;
            } else if (lockedVisibleSessionIds) {
                // Free tier with the lock already established at first render.
                // Only show the originally-chosen IDs that still exist as
                // active sessions. Don't promote hidden sessions just because
                // their last_activity_at bumped — that's the churn we want
                // to avoid. A page reload picks a fresh top-N.
                renderable = candidates.filter(s => lockedVisibleSessionIds.has(s.id));
            } else if (candidates.length > 0) {
                // Free tier, first non-empty render: pick the top N by recent
                // activity and freeze that selection for the lifetime of the page.
                renderable = sortSessionsByRecentActivity(candidates).slice(0, limit);
                lockedVisibleSessionIds = new Set(renderable.map(s => s.id));
                console.info(
                    '[crabigator] locked visible sessions (' + renderable.length + '/' +
                    candidates.length + '):',
                    renderable.map(s => s.id.split('-')[0] + ' @ ' + (s.cwd || '?'))
                );
            } else {
                // No candidates yet — leave the lock null so a later non-empty
                // render still has a chance to set it.
                renderable = [];
            }

            hiddenSessionCount = Number.isFinite(limit)
                ? Math.max(0, candidates.length - renderable.length)
                : 0;
            visibleSessionIds = new Set(renderable.map(session => session.id));
            return renderable;
        }

        function getSidebarSessions(sessionList) {
            const candidates = isFocusedMode()
                ? [...sessionList]
                : getRenderableSessions(sessionList);
            return sortSessionsByNewestStart(candidates);
        }

        function mergeSessionListUpdate(sessionUpdate) {
            if (!sessionUpdate?.id) return;

            if (sessionUpdate.is_active === false || sessionUpdate.ended_at != null) {
                allSessions = allSessions.filter(session => session.id !== sessionUpdate.id);
                return;
            }

            const idx = allSessions.findIndex(session => session.id === sessionUpdate.id);
            if (idx === -1) {
                if (!sessionUpdate.cwd || !sessionUpdate.platform || !sessionUpdate.started_at) {
                    loadSessions();
                    return;
                }
                allSessions.push(sessionUpdate);
            } else {
                allSessions[idx] = { ...allSessions[idx], ...sessionUpdate };
            }
        }

        function updateSessionLimitBanner() {
            const banner = document.getElementById('session-limit-banner');
            const countEl = document.getElementById('session-limit-hidden-count');
            const detailEl = document.getElementById('session-limit-detail');
            if (!banner) return;

            if (isProUser || isFocusedMode() || hiddenSessionCount <= 0) {
                banner.hidden = true;
                return;
            }

            const total = allSessions.length;
            banner.hidden = false;
            if (countEl) {
                countEl.textContent = hiddenSessionCount + ' active session' + (hiddenSessionCount === 1 ? '' : 's') + ' hidden';
            }
            if (detailEl) {
                detailEl.textContent = 'Free accounts show the ' + FREE_VISIBLE_SESSION_LIMIT + ' most recently active sessions. Upgrade to Pro to see all ' + total + ' active sessions at once.';
            }
        }

        function syncRenderedSessions() {
            const container = document.getElementById('sessions');
            if (!container) return;

            preservePageScroll(() => {
                const renderableSessions = getRenderableSessions(allSessions);
                const renderableIds = new Set(renderableSessions.map(session => session.id));
                let needsRerender = false;

                updateSessionsCount();
                updateSessionLimitBanner();
                if (typeof updateUsageDisplay === 'function') {
                    updateUsageDisplay();
                }

                for (const [id, session] of sessions) {
                    if (!renderableIds.has(id)) {
                        console.info(
                            '[crabigator] removing session card',
                            id.split('-')[0],
                            lockedVisibleSessionIds && !lockedVisibleSessionIds.has(id)
                                ? '(not in locked set)'
                                : '(no longer active)'
                        );
                        session.eventSource?.close();
                        sessions.delete(id);
                        if (activeTerminalId === id) activeTerminalId = null;
                        const card = document.getElementById('session-' + id);
                        if (card) card.remove();
                        needsRerender = true;
                    }
                }

                if (renderableSessions.length === 0) {
                    if (sessions.size === 0) {
                        const message = isFocusedMode() && allSessions.length > 0
                            ? 'Focused session is not active'
                            : 'No active sessions';
                        container.innerHTML = '<div class="no-sessions">' + message + '</div>';
                    }
                    updateFitLayout();
                    return;
                }

                const emptyState = container.querySelector('.no-sessions');
                if (emptyState) {
                    emptyState.remove();
                }

                for (const session of renderableSessions) {
                    if (!sessions.has(session.id)) {
                        console.info('[crabigator] creating session card', session.id.split('-')[0], session.cwd);
                        createSessionCard(session);
                        connectToSession(session.id);
                        needsRerender = true;
                    } else {
                        const sessionData = sessions.get(session.id);
                        const previousCwd = sessionData?.cwd || null;
                        const previousDeviceName = sessionData?.deviceName || null;

                        updateSessionHeader(session);

                        if (sessionData) {
                            if (session.cwd) {
                                sessionData.cwd = session.cwd;
                            }
                            if (session.started_at) {
                                sessionData.startedAt = session.started_at;
                            }
                            if (session.device_name !== undefined) {
                                sessionData.deviceName = session.device_name || null;
                            }
                            sessionData.lastActivityAt = getSessionActivityTime(session);

                            if (
                                groupingMode === 'project' &&
                                ((session.cwd && session.cwd !== previousCwd) ||
                                    (session.device_name !== undefined && (session.device_name || null) !== previousDeviceName))
                            ) {
                                needsRerender = true;
                            }
                        }
                    }
                }

                if (needsRerender) {
                    rerenderSessions();
                }
            });
        }

        async function loadSessions() {
            try {
                // Fetch sessions and projects in parallel
                const [resp, projectsResp] = await Promise.all([
                    fetch(API_BASE + '/sessions', { headers: getAuthHeaders() }),
                    fetch(API_BASE + '/projects', { headers: getAuthHeaders() }).catch(() => null),
                ]);
                if (handleAuthFailure(resp)) return;
                if (!resp.ok) throw new Error('Failed to fetch sessions');
                const data = await resp.json();

                // Store projects for history display
                if (projectsResp && projectsResp.ok) {
                    const projectsData = await projectsResp.json();
                    allProjects = projectsData.projects || [];
                }

                // Store all sessions for sidebar and visibility-limit accounting
                allSessions = data.sessions;

                // Filter the main content to the focused session if specified, then apply the Free visibility cap.
                const filteredSessions = getRenderableSessions(data.sessions);
                if (isFocusedMode()) {
                    applyFocusMode();
                }
                // Always update session count in sessions button
                updateSessionsCount();
                updateSessionLimitBanner();

                const container = document.getElementById('sessions');

                if (filteredSessions.length === 0) {
                    const hasAnyActiveSession = data.sessions.length > 0;

                    // If we had sessions recently and now have none, likely a deploy
                    if (!hasAnyActiveSession && hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                        showDeployOverlay();
                    }

                    for (const [, session] of sessions) {
                        session.eventSource?.close();
                    }
                    sessions.clear();
                    activeTerminalId = null;
                    container.innerHTML = '<div class="no-sessions">' + (
                        isFocusedMode() && hasAnyActiveSession
                            ? 'Focused session is not active'
                            : 'No active sessions'
                    ) + '</div>';

                    if (hasAnyActiveSession) {
                        hadSessionsBefore = true;
                        lastSuccessfulConnection = Date.now();
                        emptyPollDelay = MIN_EMPTY_POLL_DELAY;
                        if (emptyPollTimeout) {
                            clearTimeout(emptyPollTimeout);
                            emptyPollTimeout = null;
                        }
                        updateSessionsCount();
                        updateFitLayout();
                        return;
                    }

                    // Use deploy reconnect if deploying, otherwise normal backoff
                    if (isDeploying) {
                        scheduleReconnect();
                    } else {
                        // Exponential backoff polling when no sessions
                        if (emptyPollTimeout) clearTimeout(emptyPollTimeout);
                        console.log('No sessions, polling again in ' + (emptyPollDelay / 1000) + 's');
                        updateSessionsCount();
                        emptyPollTimeout = setTimeout(() => {
                            loadSessions();
                            emptyPollDelay = Math.min(emptyPollDelay * 2, MAX_EMPTY_POLL_DELAY);
                        }, emptyPollDelay);
                    }
                    return;
                }

                // Found sessions - we're connected
                hadSessionsBefore = true;
                lastSuccessfulConnection = Date.now();
                if (isDeploying) {
                    hideDeployOverlay();
                    // Reconnect session list SSE stream after deploy
                    if (!sessionListSource) {
                        sseRetryCount = 0;
                        connectSessionListStream();
                    }
                }

                // Reset exponential backoff
                emptyPollDelay = MIN_EMPTY_POLL_DELAY;
                if (emptyPollTimeout) {
                    clearTimeout(emptyPollTimeout);
                    emptyPollTimeout = null;
                }
                const emptyState = container.querySelector('.no-sessions');
                if (emptyState) {
                    emptyState.remove();
                }

                // Close event sources for removed sessions
                for (const [id, session] of sessions) {
                    if (!filteredSessions.find(s => s.id === id)) {
                        session.eventSource?.close();
                        sessions.delete(id);
                        if (activeTerminalId === id) activeTerminalId = null;
                    }
                }
                updateFitLayout();

                // Add/update sessions
                for (const session of filteredSessions) {
                    if (!sessions.has(session.id)) {
                        createSessionCard(session);
                        connectToSession(session.id);
                    } else {
                        updateSessionHeader(session);
                    }
                }

                // Re-render project groups now that all session data (incl. device names) is loaded
                if (groupingMode === 'project') {
                    rerenderSessions();
                }
            } catch (err) {
                console.error('Failed to load sessions:', err);
                updateSessionsCount();
                // Network error might mean deploy - only show overlay if we were recently connected
                if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                    showDeployOverlay();
                }
                if (isDeploying) {
                    scheduleReconnect();
                } else if (hadSessionsBefore) {
                    // Silent retry without overlay
                    setTimeout(loadSessions, 2000);
                }
            }
        }

        function createSessionCard(session, insertAtTop = false) {
            const container = document.getElementById('sessions');
            const card = document.createElement('div');
            card.className = 'session-card';
            card.id = 'session-' + session.id;
            const startedAt = formatStartedAt(session.started_at);
            card.innerHTML = \`
                <div class="session-header">
                    <div class="session-info">
                        <span class="title" id="title-\${session.id}"></span>
                        <span class="cwd">\${session.cwd}</span>
                        <span class="branch" id="branch-\${session.id}" style="color:#7ee787; font-size:11px;"></span>
                    </div>
                    <div class="session-actions">
                        <button class="info-btn" id="info-btn-\${session.id}" onclick="toggleInfoPopover('\${session.id}')" title="Session info">ⓘ</button>
                        <span class="pin-indicator pinned" id="pin-\${session.id}" title="Auto-scroll enabled">⇣</span>
                        <span class="state \${session.state}" id="state-\${session.id}">\${session.state}</span>
                        <button class="collapse-btn" id="collapse-btn-\${session.id}" onclick="toggleCollapse('\${session.id}')" title="Collapse/expand">▼</button>
                    </div>
                </div>
                <div class="session-summary" id="summary-\${session.id}"
                    <span class="summary-item"><span class="label">Stats:</span> <span class="value" id="summary-stats-\${session.id}">—</span></span>
                    <span class="summary-item"><span class="label">Git:</span> <span class="value" id="summary-git-\${session.id}">—</span></span>
                    <span class="summary-item"><span class="label">Changes:</span> <span class="value" id="summary-changes-\${session.id}">—</span></span>
                </div>
                <div class="info-popover" id="info-popover-\${session.id}">
                    <div class="info-popover-row">
                        <span class="info-popover-label">Session ID</span>
                        <span class="info-popover-value copyable" onclick="copyToClipboard('\${session.id}')" title="Click to copy">\${session.id}</span>
                    </div>
                    <div class="info-popover-row">
                        <span class="info-popover-label">Started</span>
                        <span class="info-popover-value">\${startedAt}</span>
                    </div>
                    <div class="info-popover-row">
                        <span class="info-popover-label">Directory</span>
                        <span class="info-popover-value copyable" onclick="copyToClipboard('\${session.cwd}')" title="Click to copy">\${session.cwd}</span>
                    </div>
                    <div class="info-popover-row" id="info-model-\${session.id}" style="display:none">
                        <span class="info-popover-label">Model</span>
                        <span class="info-popover-value" id="info-model-value-\${session.id}">—</span>
                    </div>
                    <div class="info-popover-row" id="info-platform-\${session.id}" style="display:none">
                        <span class="info-popover-label">Platform</span>
                        <span class="info-popover-value" id="info-platform-value-\${session.id}">—</span>
                    </div>
                    <div class="info-popover-actions">
                        <button class="focus-session-btn" onclick="focusOnSession('\${session.id}')" title="View only this session">
                            <svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12">
                                <path d="M10.5 8a2.5 2.5 0 11-5 0 2.5 2.5 0 015 0z"/>
                                <path fill-rule="evenodd" d="M0 8s3-5.5 8-5.5S16 8 16 8s-3 5.5-8 5.5S0 8 0 8zm8 3.5a3.5 3.5 0 100-7 3.5 3.5 0 000 7z"/>
                            </svg>
                            Focus
                        </button>
                    </div>
                </div>
                <div class="session-body" id="body-\${session.id}">
                    <div class="session-body-inner">
                        <div class="terminal" id="terminal-\${session.id}" style="height:\${TERMINAL_HEIGHTS[currentHeightIndex]}px">
                            <div class="terminal-scrollback" id="scrollback-\${session.id}"></div>
                            <div class="terminal-separator" id="separator-\${session.id}">─── scrollback ───</div>
                            <div class="terminal-screen" id="screen-\${session.id}">Connecting...</div>
                        </div>
                        <div class="session-recap empty" id="recap-\${session.id}"
                             onclick="onRecapClick('\${session.id}', event)"
                             title="Click to expand session • shift+click to view full history">
                            <div class="session-recap-status" id="recap-status-\${session.id}"></div>
                            <div class="session-recap-headline" id="recap-headline-\${session.id}"></div>
                            <div class="session-recap-bullets" id="recap-bullets-\${session.id}"></div>
                            <div class="session-recap-meta" id="recap-meta-\${session.id}"></div>
                        </div>
                        <div class="prompt-panel" id="prompt-\${session.id}">
                            <div class="prompt-header" id="prompt-header-\${session.id}"></div>
                            <div class="prompt-question" id="prompt-question-\${session.id}"></div>
                            <div class="prompt-options" id="prompt-options-\${session.id}"></div>
                            <div class="prompt-other" id="prompt-other-\${session.id}" style="display:none">
                                <input type="text" id="prompt-input-\${session.id}" placeholder="Type your response..."
                                       onkeydown="if(event.key==='Enter'){event.preventDefault();sendOtherAnswer('\${session.id}');}">
                                <button type="button" onclick="sendOtherAnswer('\${session.id}')">Send</button>
                            </div>
                        </div>
                        <div class="widgets-panel" id="widgets-\${session.id}">
                            <div class="widgets-header" onclick="toggleWidgets('\${session.id}')">
                                <div class="widgets-header-row1">
                                    <span class="widgets-title" id="widgets-title-\${session.id}">Session</span>
                                    <span class="widgets-state" id="widgets-state-\${session.id}">○ Ready</span>
                                    <span class="widgets-header-spacer"></span>
                                    <button class="collapse-btn" id="widgets-btn-\${session.id}" title="Toggle Git & Changes widgets">▼</button>
                                </div>
                                <div class="widgets-header-row2">
                                    <span class="wh-stat" data-tooltip="Session"><span class="wh-icon" style="color:#58a6ff">◉</span><span class="wh-value" id="widgets-session-\${session.id}">--</span></span>
                                    <span class="wh-stat" data-tooltip="Thinking"><span class="wh-icon" style="color:#3fb950">◐</span><span class="wh-value" id="widgets-thinking-\${session.id}">—</span></span>
                                    <span class="wh-stat" id="widgets-idle-wrapper-\${session.id}" style="display:none" data-tooltip="Idle"><span class="wh-icon" style="color:#8b949e">◌</span><span class="wh-value" id="widgets-idle-\${session.id}">—</span></span>
                                    <span class="wh-stat" data-tooltip="Prompts"><span class="wh-icon" style="color:#8b949e">⟩</span><span class="wh-value" id="widgets-prompts-\${session.id}">0</span><span class="wh-elapsed" id="widgets-prompts-elapsed-\${session.id}"></span></span>
                                    <span class="wh-stat" data-tooltip="Completions"><span class="wh-icon" style="color:#8b949e">⋗</span><span class="wh-value" id="widgets-completions-\${session.id}">0</span><span class="wh-elapsed" id="widgets-completions-elapsed-\${session.id}"></span></span>
                                    <span class="wh-stat" data-tooltip="Tool calls"><span class="wh-icon" style="color:#f0883e">⚒</span><span class="wh-value" id="widgets-tools-\${session.id}">—</span></span>
                                    <span class="wh-stat" data-tooltip="Compactions"><span class="wh-icon" style="color:#e879f9">⊜</span><span class="wh-value" id="widgets-compactions-\${session.id}">0</span><span class="wh-elapsed" id="widgets-compactions-elapsed-\${session.id}"></span></span>
                                    <span class="wh-git" id="widgets-git-\${session.id}"></span>
                                </div>
                            </div>
                            <div class="widgets-content" id="widgets-content-\${session.id}">
                            <div class="widget recap-history-widget" id="recaps-\${session.id}" style="display:none">
                                <div class="widget-title"><span style="color:#22d3ee">Recap history</span> <span class="recap-history-count" id="recap-count-\${session.id}"></span></div>
                                <div class="recaps-list"></div>
                            </div>
                            <div class="widget title-history-widget" id="titles-\${session.id}" style="display:none">
                                <div class="widget-title"><span style="color:#58a6ff">—</span></div>
                                <div class="titles-list"></div>
                            </div>
                            <div class="widget" id="git-\${session.id}">
                                <div class="widget-title"><span style="color:#7ee787" id="git-branch-\${session.id}">...</span> <span style="float:right;color:#8b949e" id="git-filecount-\${session.id}">...</span></div>
                                <div class="git-files" style="color:#8b949e">Waiting for data...</div>
                            </div>
                            <div class="widget" id="changes-\${session.id}">
                                <div class="widget-title"><span style="color:#db6d28">Changes</span> <span style="float:right;color:#8b949e" id="changes-count-\${session.id}"></span></div>
                                <div class="changes-list" style="color:#8b949e">Waiting for data...</div>
                            </div>
                            </div>
                        </div>
                        <div class="input-area">
                            <button type="button" class="voice-btn" id="voice-btn-\${session.id}"
                                    onclick="toggleVoiceRecording('\${session.id}')"
                                    title="Voice input">
                                ${iconMicrophone}
                            </button>
                            <div class="keyboard-container" id="keyboard-container-\${session.id}">
                                <button type="button" class="keyboard-btn"
                                        onclick="toggleKeyboardPopover('\${session.id}')"
                                        title="Keyboard shortcuts">
                                    ${iconKeyboard}
                                </button>
                                <div class="keyboard-popover">
                                    <div class="keyboard-popover-title">Send Key</div>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'shift_tab')"><kbd>⇧Tab</kbd> Cycle mode</button>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'escape')"><kbd>Esc</kbd> Cancel / exit</button>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'up')"><kbd>↑</kbd> Navigate up</button>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'down')"><kbd>↓</kbd> Navigate down</button>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'ctrl_c')"><kbd>Ctrl+C</kbd> Interrupt</button>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'tab')"><kbd>Tab</kbd> Autocomplete</button>
                                    <button class="key-btn" onclick="sendSessionKey('\${session.id}', 'enter')"><kbd>Enter</kbd> Submit</button>
                                </div>
                            </div>
                            <button type="button" class="voice-cancel-btn" id="voice-cancel-btn-\${session.id}"
                                    onclick="cancelVoiceRecording('\${session.id}')"
                                    title="Cancel recording"
                                    style="display:none">
                                ${iconClose}
                            </button>
                            <div class="voice-overlay" id="voice-overlay-\${session.id}"></div>
                            <input type="text" id="input-\${session.id}"
                                   placeholder="Type a command or answer..."
                                   oninput="handleInputChange('\${session.id}', this.value)"
                                   onkeydown="handleInputKeydown(event, '\${session.id}')">
                            <div class="voice-actions" id="voice-actions-\${session.id}" style="display:none">
                                <button type="button" class="voice-edit-btn"
                                        onclick="stopAndEditVoice('\${session.id}')"
                                        title="Edit before sending">
                                    ${iconPencil}
                                    Edit
                                </button>
                                <button type="button" class="voice-send-btn"
                                        onclick="stopAndSendVoice('\${session.id}')">
                                    Send
                                </button>
                            </div>
                            <button type="button" id="send-btn-\${session.id}" onclick="sendAnswer('\${session.id}')" disabled>Send</button>
                        </div>
                    </div>
                </div>
            \`;
            // Insert card into the correct location based on grouping mode
            if (getMainGroupingMode() === 'project') {
                // Find or create project group for this cwd
                let group = container.querySelector(\`.project-group[data-project="\${CSS.escape(session.cwd)}"]\`);
                if (!group) {
                    // Create new group
                    group = createProjectGroup(session.cwd, []);
                    if (insertAtTop && container.firstChild) {
                        container.insertBefore(group, container.firstChild);
                    } else {
                        container.appendChild(group);
                    }
                }
                const sessionsContent = group.querySelector('.project-sessions-content');
                if (insertAtTop && sessionsContent.firstChild) {
                    sessionsContent.insertBefore(card, sessionsContent.firstChild);
                } else {
                    sessionsContent.appendChild(card);
                }
                updateProjectGroupCount(session.cwd);
            } else {
                // Flat mode
                if (insertAtTop && container.firstChild) {
                    container.insertBefore(card, container.firstChild);
                } else {
                    container.appendChild(card);
                }
            }
            sessions.set(session.id, {
                element: card,
                state: session.state,
                title: null,
                git: null,
                changes: null,
                stats: null,
                permission: null,
                pinned: true,
                lastScrollTop: 0,
                startedAt: session.started_at,
                cwd: session.cwd,
                deviceName: session.device_name || null,
                lastActivityAt: getSessionActivityTime(session),
                // Scrollback chunking: store full buffer, render only visible portion
                scrollbackBuffer: [],      // Full scrollback lines
                scrollbackRendered: 0,     // How many lines currently rendered
            });
            applyCollapsedState(session.id);
            applyWidgetsCollapsedState(session.id);
            restoreInput(session.id);
            updateFitLayout();

            // Set up scroll tracking for pin/unpin behavior
            // Only unpin on explicit scroll UP gesture, re-pin when scrolling down to bottom
            const terminal = document.getElementById('terminal-' + session.id);
            if (terminal) {
                terminal.addEventListener('scroll', () => {
                    const sessionData = sessions.get(session.id);
                    if (!sessionData) return;

                    // Skip scroll handling during font size changes to preserve pin state
                    if (isChangingFontSize) return;

                    // Skip pin/unpin logic if this terminal isn't scroll-active
                    if (activeTerminalId !== session.id) return;

                    const currentScrollTop = terminal.scrollTop;
                    const prevScrollTop = sessionData.lastScrollTop || 0;
                    const scrollDelta = currentScrollTop - prevScrollTop;
                    const scrollingUp = scrollDelta < 0;
                    const scrollingDown = scrollDelta > 0;

                    // Update last scroll position
                    sessionData.lastScrollTop = currentScrollTop;

                    const distFromBottom = terminal.scrollHeight - currentScrollTop - terminal.clientHeight;
                    const atBottom = distFromBottom < 5;

                    // Ignore scroll bounce: small upward movements near the bottom are likely
                    // elastic bounce, not intentional scrolling. Require scrolling up past
                    // bounce threshold (50px from bottom) to unpin.
                    const pastBounceZone = distFromBottom > 50;

                    if (scrollingUp && sessionData.pinned && pastBounceZone) {
                        // Unpin only on explicit scroll UP past the bounce zone
                        sessionData.pinned = false;
                        updatePinButton(session.id, false);
                    } else if (scrollingDown && atBottom && !sessionData.pinned) {
                        // Re-pin when scrolling DOWN and reaching bottom
                        sessionData.pinned = true;
                        updatePinButton(session.id, true);
                    }

                    // Load more scrollback when scrolling near the top
                    if (currentScrollTop < 100 && sessionData.scrollbackBuffer && sessionData.scrollbackRendered < sessionData.scrollbackBuffer.length) {
                        loadMoreScrollback(session.id);
                    }
                });

                // Click to activate terminal scrolling
                terminal.addEventListener('click', () => {
                    activateTerminalScroll(session.id);
                });
            }
        }

`;
