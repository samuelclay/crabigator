// Dashboard JavaScript - sse
export const sseJs = `
        let sessionListSource = null;
        let sseRetryCount = 0;
        let pollingInterval = null;
        let emptyPollDelay = 2000;  // Start at 2s when no sessions
        let emptyPollTimeout = null;
        let serverVersion = null;  // Track server version for deploy detection
        const MAX_SSE_RETRIES = 3;
        const MIN_EMPTY_POLL_DELAY = 2000;   // 2 seconds
        const MAX_EMPTY_POLL_DELAY = 30000;  // 30 seconds

        function connectSessionListStream() {
            if (sessionListSource) {
                sessionListSource.close();
            }

            console.log('Connecting to session list SSE...');
            sessionListSource = new EventSource(API_BASE + '/sessions/stream' + getAuthQueryParam());

            sessionListSource.onopen = () => {
                console.log('Session list SSE connected');
                sseRetryCount = 0;
                // Stop polling if it was active
                if (pollingInterval) {
                    clearInterval(pollingInterval);
                    pollingInterval = null;
                }
                // Stop empty session backoff polling
                if (emptyPollTimeout) {
                    clearTimeout(emptyPollTimeout);
                    emptyPollTimeout = null;
                }
                emptyPollDelay = MIN_EMPTY_POLL_DELAY;
                document.getElementById('status').textContent = 'Connected';
            };

            sessionListSource.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    handleSessionListEvent(data);
                } catch (err) {
                    console.error('Failed to parse session list event:', err);
                }
            };

            sessionListSource.onerror = (err) => {
                console.error('Session list SSE error:', err);
                sessionListSource.close();
                sessionListSource = null;

                // If we were recently connected, this might be a deploy
                if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                    showDeployOverlay();
                }

                if (isDeploying) {
                    // Use fast deploy reconnect
                    scheduleReconnect();
                } else {
                    sseRetryCount++;
                    if (sseRetryCount >= MAX_SSE_RETRIES) {
                        // Fall back to polling after too many SSE failures
                        console.log('SSE failed, falling back to polling');
                        document.getElementById('status').textContent = sessionCount(sessions.size);
                        if (!pollingInterval) {
                            pollingInterval = setInterval(loadSessions, 10000);
                        }
                    } else {
                        document.getElementById('status').textContent = 'Reconnecting...';
                        // Retry SSE with exponential backoff
                        setTimeout(connectSessionListStream, Math.min(1000 * Math.pow(2, sseRetryCount), 10000));
                    }
                }
            };
        }

        const sessionListColors = {
            connected: '#10b981',
            created: '#3b82f6',
            updated: '#f59e0b',
            deleted: '#ef4444',
        };

        function handleSessionListEvent(event) {
            const container = document.getElementById('sessions');
            const color = sessionListColors[event.type] || '#9ca3af';
            const shortId = event.session?.id?.split('-')[0] || '';
            const label = shortId ? '[' + shortId + '] ' + event.type : event.type;
            console.log('%c' + label, 'color:' + color + ';font-weight:bold', event);

            switch (event.type) {
                case 'connected':
                    // Check for version mismatch on reconnect (deploy detected)
                    if (serverVersion !== null && event.version && event.version !== serverVersion) {
                        console.log('%cversion mismatch', 'color:#ef4444', serverVersion, '->', event.version);
                        // Use cache-busting URL to ensure fresh HTML/JS/CSS
                        const url = new URL(location.href);
                        url.searchParams.set('v', event.version);
                        location.href = url.toString();
                        return;
                    }
                    // Store version on first connect
                    if (event.version) {
                        serverVersion = event.version;
                    }
                    loadSessions();
                    break;

                case 'created':
                    // New session - add to view immediately at the top
                    if (event.session && !sessions.has(event.session.id)) {
                        const emptyState = container.querySelector('.no-sessions');
                        if (emptyState) emptyState.remove();
                        createSessionCard(event.session, true);
                        connectToSession(event.session.id);
                        document.getElementById('status').textContent = sessionCount(sessions.size);
                    }
                    break;

                case 'updated':
                    // Session updated - update header
                    if (event.session && event.session.id) {
                        updateSessionHeader(event.session);
                        // If session became inactive, remove it from view
                        if (event.session.is_active === false) {
                            const session = sessions.get(event.session.id);
                            if (session) {
                                session.eventSource?.close();
                                sessions.delete(event.session.id);
                                const card = document.getElementById('session-' + event.session.id);
                                const cwd = event.session.cwd;
                                if (card) card.remove();
                                // Update project group count if in grouped mode
                                if (groupingMode === 'project' && cwd) {
                                    updateProjectGroupCount(cwd);
                                }
                                updateFitLayout();
                                document.getElementById('status').textContent = sessionCount(sessions.size);
                                if (sessions.size === 0) {
                                    container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                                }
                            }
                        }
                    }
                    break;

                case 'deleted':
                    // Session deleted - remove from view
                    if (event.session && event.session.id) {
                        const session = sessions.get(event.session.id);
                        if (session) {
                            session.eventSource?.close();
                            sessions.delete(event.session.id);
                        }
                        const card = document.getElementById('session-' + event.session.id);
                        const cwd = event.session.cwd;
                        if (card) card.remove();
                        // Update project group count if in grouped mode
                        if (groupingMode === 'project' && cwd) {
                            updateProjectGroupCount(cwd);
                        }
                        updateFitLayout();
                        document.getElementById('status').textContent = sessionCount(sessions.size);
                        if (sessions.size === 0) {
                            container.innerHTML = '<div class="no-sessions">No active sessions</div>';
                            // If all sessions gone and we had sessions before, likely a deploy
                            if (hadSessionsBefore && !isDeploying) {
                                showDeployOverlay();
                                scheduleReconnect();
                            }
                        }
                    }
                    break;
            }
        }

        // Detect when tab becomes visible again and check connection
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'visible') {
                // Check if SSE connection is still alive
                if (sessionListSource && sessionListSource.readyState === EventSource.CLOSED) {
                    console.log('SSE connection closed while tab was hidden, reconnecting...');
                    // Don't show deploy overlay for tab visibility changes - just reconnect silently
                    sseRetryCount = 0;
                    loadSessions();
                    connectSessionListStream();
                } else if (!sessionListSource) {
                    // No SSE connection at all - reconnect
                    console.log('No SSE connection, reconnecting...');
                    sseRetryCount = 0;
                    loadSessions();
                    connectSessionListStream();
                }
            }
        });

`;
