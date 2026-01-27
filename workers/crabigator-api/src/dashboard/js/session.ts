// Dashboard JavaScript - session
export const sessionJs = `
        async function loadSessions() {
            try {
                const resp = await fetch(API_BASE + '/sessions', { headers: getAuthHeaders() });
                if (handleAuthFailure(resp)) return;
                if (!resp.ok) throw new Error('Failed to fetch sessions');
                const data = await resp.json();

                // Store all sessions for popover
                allSessions = data.sessions;

                // Filter to single session if specified via URL parameter
                let filteredSessions = data.sessions;
                if (singleSessionId) {
                    filteredSessions = data.sessions.filter(s => s.id === singleSessionId);
                    // Force single-column layout for single session view
                    document.getElementById('sessions').dataset.layout = '1';
                }
                // Always update session count in sessions button
                updateSessionsCount();

                const container = document.getElementById('sessions');

                if (filteredSessions.length === 0) {
                    // If we had sessions recently and now have none, likely a deploy
                    if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                        showDeployOverlay();
                    }

                    for (const [, session] of sessions) {
                        session.eventSource?.close();
                    }
                    sessions.clear();
                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';

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
                <div class="session-summary" id="summary-\${session.id}">
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
                            <input type="text" id="input-\${session.id}"
                                   placeholder="Type a command or answer..."
                                   oninput="handleInputChange('\${session.id}', this.value)"
                                   onkeydown="if(event.key==='Enter')sendAnswer('\${session.id}')">
                            <button type="button" onclick="sendAnswer('\${session.id}')">Send</button>
                        </div>
                    </div>
                </div>
            \`;
            // Insert card into the correct location based on grouping mode
            if (groupingMode === 'project') {
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
            }
        }

`;
