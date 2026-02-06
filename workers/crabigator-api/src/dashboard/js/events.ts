// Dashboard JavaScript - events
export const eventsJs = `
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
                        desktop_status: '#ef4444',
                        prompt: '#f97316',
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

                // If all sessions disconnected and we had sessions before, likely a deploy
                if (sessions.size === 0 && hadSessionsBefore && !isDeploying) {
                    showDeployOverlay();
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

            switch (event.type) {
                case 'screen':
                    // Full screen update - only update the screen section
                    if (screenEl) {
                        screenEl.innerHTML = ansiToHtml(event.content);
                    }
                    if (sessionData?.pinned) {
                        terminal.scrollTop = terminal.scrollHeight;
                        sessionData.lastScrollTop = terminal.scrollTop;
                    }
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
                        // Clear permission/prompt when leaving interactive states
                        // This is a safety net in case the prompt event was missed
                        if (event.state !== 'permission' && event.state !== 'question') {
                            sessionData.permission = null;
                            updatePromptPanel(sessionId, null);
                        }
                        updateStatsWidget(sessionId, sessionData.stats || {});
                        updateSessionSummary(sessionId, sessionData);
                    }
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
                case 'stats':
                    // Store stats in session data
                    if (sessionData) {
                        sessionData.stats = event;
                        updateSessionSummary(sessionId, sessionData);
                    }
                    updateStatsWidget(sessionId, event);
                    // Update suggestion in input field (always sync — absent field means no suggestion)
                    updateInputSuggestion(sessionId, event.suggestion || null);
                    break;
                case 'title':
                    // Update title in header
                    const titleEl = document.getElementById('title-' + sessionId);
                    if (titleEl) {
                        titleEl.textContent = event.title;
                    }
                    // Store in session data
                    if (sessionData) {
                        sessionData.title = event.title;
                    }
                    // Update titles widget with single title if no history yet
                    updateTitlesWidget(sessionId, [event.title]);
                    break;
                case 'desktop_status':
                    // Desktop connected/disconnected
                    if (!event.connected) {
                        // Check for version change FIRST - likely a deploy
                        // Do this before DOM cleanup that might throw
                        checkVersionAndReload();

                        // Desktop disconnected - remove session from view
                        const session = sessions.get(sessionId);
                        if (session) {
                            session.eventSource?.close();
                            sessions.delete(sessionId);
                        }
                        // Also remove from allSessions for accurate count
                        const allIdx = allSessions.findIndex(s => s.id === sessionId);
                        if (allIdx !== -1) {
                            allSessions.splice(allIdx, 1);
                            updateSessionsCount();
                        }
                        const cwd = card.querySelector('.cwd')?.textContent;
                        card.remove();
                        // Update project group count if in grouped mode
                        if (groupingMode === 'project' && cwd) {
                            updateProjectGroupCount(cwd);
                        }
                        updateFitLayout();
                        // Update status
                        const statusEl = document.getElementById('status');
                        if (statusEl) statusEl.textContent = sessionCount(sessions.size);
                    }
                    break;
                case 'title_history':
                    updateTitlesWidget(sessionId, event.history);
                    break;
                case 'prompt':
                    // Interactive prompt (question or permission)
                    updatePromptPanel(sessionId, event.prompt);
                    break;
            }
        }

        function updatePromptPanel(sessionId, prompt) {
`;
