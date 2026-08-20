// Dashboard JavaScript - events
export const eventsJs = `
        // Coalesce screen updates to one render per animation frame
        const pendingScreenUpdates = new Map();

        function scheduleScreenUpdate(sessionId, content) {
            const hadPending = pendingScreenUpdates.has(sessionId);
            pendingScreenUpdates.set(sessionId, content);
            if (!hadPending) {
                requestAnimationFrame(() => {
                    const latestContent = pendingScreenUpdates.get(sessionId);
                    pendingScreenUpdates.delete(sessionId);
                    if (latestContent !== undefined) {
                        const screenEl = document.getElementById('screen-' + sessionId);
                        if (screenEl) screenEl.innerHTML = ansiToHtml(latestContent);
                        const sessionData = sessions.get(sessionId);
                        const terminal = document.getElementById('terminal-' + sessionId);
                        if (sessionData?.pinned && terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                });
            }
        }

        function isActiveRecapTurnState(state) {
            return state === 'thinking' || state === 'permission' || state === 'question';
        }

        function normalizeRecapForSession(sessionData, recap) {
            if (recap?.status === 'updating' && isActiveRecapTurnState(sessionData?.state)) {
                return { ...recap, status: 'waiting' };
            }
            return recap;
        }

        function connectToSession(sessionId) {
            console.log('Connecting SSE for session:', sessionId);
            const eventSource = new EventSource(API_BASE + '/sessions/' + sessionId + '/events' + getAuthQueryParam());

            eventSource.onopen = () => {
                console.log('SSE connected for session:', sessionId);
                const screenEl = document.getElementById('screen-' + sessionId);
                if (screenEl && screenEl.innerHTML === 'Connecting...') {
                    screenEl.innerHTML = '<span style="color:#8b949e">Connected, waiting for screen data...</span>';
                }
                // Send viewer heartbeat immediately - this triggers desktop to send screen
                // Must happen AFTER SSE is connected so we can receive the screen event
                sendViewerHeartbeat(sessionId);
            };

            eventSource.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    const shortId = sessionId.split('-')[0];
                    const typeColors = {
                        screen: '#8b5cf6',
                        state: '#f59e0b',
                        stats: '#10b981',
                        git: '#3b82f6',
                        changes: '#ec4899',
                        scrollback: '#6b7280',
                        scrollback_history: '#6b7280',
                        title: '#06b6d4',
                        title_history: '#06b6d4',
                        slack_threads: '#58d6ff',
                        desktop_status: '#ef4444',
                        prompt: '#f97316',
                        recap: '#22d3ee',
                        recap_history: '#22d3ee',
                        prs: '#a371f7',
                        commit_history: '#db6d28',
                    };
                    const color = typeColors[data.type] || '#9ca3af';
                    if (data.type !== 'screen') {
                        console.log('%c[' + shortId + '] ' + data.type, 'color:' + color + ';font-weight:bold', data);
                    }
                    handleSessionEvent(sessionId, data);
                } catch (err) {
                    console.error('Failed to parse event:', err, event.data);
                }
            };

            eventSource.onerror = (err) => {
                console.error('SSE error for session ' + sessionId, err);
                // Clean up so the session can be recreated on next poll
                const session = sessions.get(sessionId);
                if (session) {
                    session.eventSource?.close();
                    sessions.delete(sessionId);
                    if (activeTerminalId === sessionId) activeTerminalId = null;
                }
                // Remove the card - it will be recreated if session is still active
                const card = document.getElementById('session-' + sessionId);
                if (card) {
                    const cwd = card.querySelector('.cwd')?.textContent;
                    card.remove();
                    // Update project group count if in grouped mode
                    if (groupingMode === 'project' && cwd) {
                        updateProjectGroupCount(cwd);
                    }
                }
                updateFitLayout();

                // If all sessions disconnected, verify the build before showing
                // deploy UI. Otherwise retry silently and recreate live sessions.
                if (sessions.size === 0 && hadSessionsBefore) {
                    if (!isDeploying) {
                        void checkVersionAndReload();
                    }
                    setTimeout(loadSessions, 1000);
                }
                if (isDeploying) {
                    scheduleReconnect();
                }
            };

            const session = sessions.get(sessionId);
            if (session) {
                session.eventSource = eventSource;
            }
        }

        function handleSessionEvent(sessionId, event) {
            const terminal = document.getElementById('terminal-' + sessionId);
            const card = document.getElementById('session-' + sessionId);
            const scrollbackEl = document.getElementById('scrollback-' + sessionId);
            const separatorEl = document.getElementById('separator-' + sessionId);
            const screenEl = document.getElementById('screen-' + sessionId);
            if (!terminal || !card) return;

            const sessionData = sessions.get(sessionId);
            if (sessionData) {
                const nowSeconds = Math.floor(Date.now() / 1000);
                sessionData.lastActivityAt = nowSeconds;
                const listSession = allSessions.find(session => session.id === sessionId);
                if (listSession) {
                    listSession.last_activity_at = nowSeconds;
                }
            }

            switch (event.type) {
                case 'screen':
                    scheduleScreenUpdate(sessionId, event.content);
                    break;
                case 'state':
                    // Update state badge
                    const stateEl = card.querySelector('.state');
                    if (stateEl) {
                        stateEl.className = 'state ' + event.state;
                        stateEl.textContent = event.state;
                    }
                    // Update session state for stats widget
                    if (sessionData) {
                        sessionData.state = event.state;
                        if (sessionData.recap?.status === 'updating' && isActiveRecapTurnState(event.state)) {
                            const waitingRecap = { ...sessionData.recap, status: 'waiting' };
                            sessionData.recap = waitingRecap;
                            updateRecapCard(sessionId, waitingRecap);
                        }
                        // Clear permission/prompt when leaving interactive states
                        // This is a safety net in case the prompt event was missed
                        if (event.state !== 'permission' && event.state !== 'question') {
                            sessionData.permission = null;
                            updatePromptPanel(sessionId, null);
                        }
                        updateStatsWidget(sessionId, sessionData.stats || {});
                        updateSessionSummary(sessionId, sessionData);
                    }
                    scheduleSidebarUpdate();
                    break;
                case 'scrollback':
                    // Append scrollback diff to scrollback section (chunked)
                    if (event.diff) {
                        appendScrollback(sessionId, event.diff);
                    }
                    break;
                case 'scrollback_history':
                    // Full scrollback history for late joiners (chunked - only render last N lines)
                    if (event.content) {
                        const lines = event.content.split('\\n');
                        renderScrollback(sessionId, lines);
                    }
                    break;
                case 'git':
                    if (sessionData) {
                        sessionData.git = event;
                        updateSessionSummary(sessionId, sessionData);
                        if (sessionData.prs?.length) {
                            updatePrList(sessionId, sessionData.prs);
                        } else {
                            updateSessionTitleHierarchy(sessionId);
                            updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
                            scheduleSidebarUpdate();
                        }
                    }
                    updateGitWidget(sessionId, event);
                    break;
                case 'changes':
                    if (sessionData) {
                        sessionData.changes = event;
                        updateSessionSummary(sessionId, sessionData);
                    }
                    updateChangesWidget(sessionId, event);
                    break;
                case 'commit_history':
                    if (sessionData) {
                        sessionData.commitHistory = event.history || [];
                        updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
                    }
                    break;
                case 'stats':
                    // Store stats in session data
                    if (sessionData) {
                        sessionData.stats = event;
                        updateSessionSummary(sessionId, sessionData);
                    }
                    updateStatsWidget(sessionId, event);
                    // Update suggestion in input field (always sync — absent field means no suggestion)
                    updateInputSuggestion(sessionId, event.suggestion || null);
                    scheduleSidebarUpdate();
                    break;
                case 'title':
                    // Keep the assistant's title separately. A primary PR title,
                    // when present, is the official title shown above it.
                    if (sessionData) {
                        sessionData.title = event.title;
                        sessionData.generatedTitle = event.title;
                    }
                    updateSessionTitleHierarchy(sessionId);
                    updateTitlesWidget(sessionId, [event.title]);
                    if (sessionData) {
                        updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
                    }
                    scheduleSidebarUpdate();
                    break;
                case 'desktop_status':
                    // Desktop connected/disconnected
                    if (!event.connected) {
                        // Check for version change FIRST - likely a deploy
                        // Do this before DOM cleanup that might throw
                        void checkVersionAndReload();

                        // Wrap the whole teardown in preservePageScroll so the
                        // scroll position is captured BEFORE the card is removed.
                        // Otherwise removing a card above the viewport collapses
                        // the page and pins the viewport to the top — which fires
                        // for every session at once whenever WebSockets drop (e.g.
                        // a deploy), hoisting the user to the top repeatedly.
                        preservePageScroll(() => {
                            // Desktop disconnected - remove session from view
                            const session = sessions.get(sessionId);
                            if (session) {
                                session.eventSource?.close();
                                sessions.delete(sessionId);
                                if (activeTerminalId === sessionId) activeTerminalId = null;
                            }
                            // Also remove from allSessions for accurate count
                            const allIdx = allSessions.findIndex(s => s.id === sessionId);
                            if (allIdx !== -1) {
                                allSessions.splice(allIdx, 1);
                                updateSessionsCount();
                                updateSessionLimitBanner();
                                if (typeof updateUsageDisplay === 'function') {
                                    updateUsageDisplay();
                                }
                            }
                            const cwd = card.querySelector('.cwd')?.textContent;
                            card.remove();
                            // Update project group count if in grouped mode
                            if (groupingMode === 'project' && cwd) {
                                updateProjectGroupCount(cwd);
                            }
                            updateFitLayout();
                            syncRenderedSessions();
                            // Update status
                            const statusEl = document.getElementById('status');
                            if (statusEl) statusEl.textContent = sessionCount(sessions.size);
                        });
                    }
                    break;
                case 'title_history':
                    updateTitlesWidget(sessionId, event.history);
                    if (sessionData) {
                        updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
                    }
                    scheduleSidebarUpdate();
                    break;
                case 'slack_threads':
                    if (sessionData) {
                        sessionData.slackThreads = event.threads || [];
                        updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
                    }
                    break;
                case 'prompt':
                    // Interactive prompt (question or permission)
                    updatePromptPanel(sessionId, event.prompt);
                    break;
                case 'recap':
                    const recap = normalizeRecapForSession(sessionData, event);
                    if (sessionData) {
                        sessionData.recap = recap;
                    }
                    updateRecapCard(sessionId, recap);
                    break;
                case 'recap_history':
                    updateRecapHistoryWidget(sessionId, event.history || []);
                    if (sessionData) {
                        updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
                    }
                    break;
                case 'prs':
                    updatePrList(sessionId, event.prs || []);
                    break;
            }
        }

        function updatePromptPanel(sessionId, prompt) {
`;
