// Dashboard JavaScript - events
export const eventsJs = `
        function connectToSession(sessionId) {
            console.log('Connecting SSE for session:', sessionId);
            const eventSource = new EventSource(API_BASE + '/sessions/' + sessionId + '/events');

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
                    console.log('SSE event for', sessionId, ':', event.data.substring(0, 100));
                    const data = JSON.parse(event.data);
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
                    card.remove();
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
                        // Clear permission when leaving permission state
                        if (event.state !== 'permission') {
                            sessionData.permission = null;
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
                        const lines = event.content.split('\\n').filter(line => line.length > 0);
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
                        // Desktop disconnected - remove session from view
                        console.log('Desktop disconnected for session:', sessionId);
                        const session = sessions.get(sessionId);
                        if (session) {
                            session.eventSource?.close();
                            sessions.delete(sessionId);
                        }
                        card.remove();
                        updateFitLayout();
                        // Update status
                        document.getElementById('status').textContent = sessions.size + ' session(s)';
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
