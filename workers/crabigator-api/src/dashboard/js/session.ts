// Dashboard JavaScript - session
export const sessionJs = `
        async function loadSessions() {
            try {
                const resp = await fetch(API_BASE + '/sessions');
                if (!resp.ok) throw new Error('Failed to fetch sessions');
                const data = await resp.json();

                document.getElementById('status').textContent =
                    data.sessions.length + ' session(s)';

                const container = document.getElementById('sessions');

                if (data.sessions.length === 0) {
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
                        document.getElementById('status').textContent =
                            'No sessions (retry in ' + Math.round(emptyPollDelay / 1000) + 's)';
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
                    if (!data.sessions.find(s => s.id === id)) {
                        session.eventSource?.close();
                        sessions.delete(id);
                    }
                }
                updateFitLayout();

                // Add/update sessions
                for (const session of data.sessions) {
                    if (!sessions.has(session.id)) {
                        createSessionCard(session);
                        connectToSession(session.id);
                    } else {
                        updateSessionHeader(session);
                    }
                }
            } catch (err) {
                console.error('Failed to load sessions:', err);
                document.getElementById('status').textContent = 'Error loading sessions';
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
                        <div class="session-actions-row">
                            <span class="state \${session.state}" id="state-\${session.id}">\${session.state}</span>
                            <button class="collapse-btn" id="collapse-btn-\${session.id}" onclick="toggleCollapse('\${session.id}')" title="Collapse/expand">▼</button>
                        </div>
                        <div class="session-actions-row">
                            <button class="info-btn" id="info-btn-\${session.id}" onclick="toggleInfoPopover('\${session.id}')" title="Session info">ⓘ</button>
                            <span class="pin-indicator pinned" id="pin-\${session.id}" title="Auto-scroll enabled">⇣ pinned</span>
                        </div>
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
                </div>
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
                        <div class="widgets-summary" id="widgets-summary-\${session.id}">
                            <span class="widgets-summary-title" id="widgets-title-\${session.id}"></span>
                            <span class="widgets-summary-stats">
                                <span id="widgets-session-\${session.id}">--</span>
                                <span class="sep">·</span>
                                <span id="widgets-thinking-\${session.id}">—</span>
                                <span class="sep">·</span>
                                <span id="widgets-idle-\${session.id}">—</span>
                            </span>
                            <span class="widgets-summary-git" id="widgets-git-\${session.id}"></span>
                        </div>
                        <button class="collapse-btn" id="widgets-btn-\${session.id}" title="Toggle widgets">▼</button>
                    </div>
                    <div class="widgets-content" id="widgets-content-\${session.id}">
                    <div class="widget title-history-widget" id="titles-\${session.id}" style="display:none">
                        <div class="widget-title"><span style="color:#58a6ff">—</span></div>
                        <div class="titles-list"></div>
                    </div>
                    <div class="widget" id="stats-\${session.id}">
                        <div class="widget-title"><span style="color:#bc8cff">Stats</span> <span style="float:right;color:#8b949e">○ Ready</span></div>
                        <div class="widget-row"><span class="widget-label">◆ Session</span><span class="widget-value">--</span></div>
                        <div class="widget-row"><span class="widget-label">◇ Thinking</span><span class="widget-value">—</span></div>
                        <div class="widget-row"><span class="widget-label">▸ Prompts 0</span><span class="widget-value"></span></div>
                        <div class="widget-row"><span class="widget-label">◂ Completions 0</span><span class="widget-value"></span></div>
                        <div class="widget-row"><span class="widget-label">⚙ Tools</span><span class="widget-value purple">0</span></div>
                    </div>
                    <div class="widget" id="git-\${session.id}">
                        <div class="widget-title"><span style="color:#7ee787">...</span> <span style="float:right;color:#8b949e">...</span></div>
                        <div class="git-files" style="color:#8b949e">Waiting for data...</div>
                    </div>
                    <div class="widget" id="changes-\${session.id}">
                        <div class="widget-title"><span style="color:#db6d28">Changes</span></div>
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
