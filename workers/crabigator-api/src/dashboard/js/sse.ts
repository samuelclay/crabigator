// Dashboard JavaScript - session list stream
export const sseJs = `
        let sessionListSocket = null;
        let streamRetryCount = 0;
        let pollingInterval = null;
        let emptyPollDelay = 2000;  // Start at 2s when no sessions
        let emptyPollTimeout = null;
        let serverVersion = null;  // Track server version for deploy detection
        const MAX_STREAM_RETRIES = 3;
        const MIN_EMPTY_POLL_DELAY = 2000;   // 2 seconds
        const MAX_EMPTY_POLL_DELAY = 30000;  // 30 seconds

        function connectSessionListStream() {
            if (sessionListSocket) {
                sessionListSocket.onclose = null;
                sessionListSocket.close();
            }

            console.log('Connecting to session list WebSocket...');
            const socket = new WebSocket(getWebSocketUrl('/sessions/stream'));
            sessionListSocket = socket;

            socket.onopen = () => {
                console.log('Session list WebSocket connected');
                streamRetryCount = 0;
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
                const statusEl = document.getElementById('status');
                if (statusEl) statusEl.textContent = 'Connected';
            };

            socket.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    handleSessionListEvent(data);
                } catch (err) {
                    console.error('Failed to parse session list event:', err);
                }
            };

            socket.onerror = (err) => {
                console.error('Session list WebSocket error:', err);
            };

            socket.onclose = () => {
                if (sessionListSocket !== socket) return;
                sessionListSocket = null;

                // If we were recently connected, this might be a deploy. Verify
                // the build before showing update UI; ordinary reconnects are silent.
                if (hadSessionsBefore && wasRecentlyConnected() && !isDeploying) {
                    void checkVersionAndReload();
                }

                if (isDeploying) {
                    // Use fast deploy reconnect
                    scheduleReconnect();
                } else {
                    streamRetryCount++;
                    if (streamRetryCount >= MAX_STREAM_RETRIES) {
                        // Fall back to polling after too many stream failures
                        console.log('Session list stream failed, falling back to polling');
                        const statusEl = document.getElementById('status');
                        if (statusEl) statusEl.textContent = sessionCount(sessions.size);
                        if (!pollingInterval) {
                            pollingInterval = setInterval(loadSessions, 10000);
                        }
                    } else {
                        const statusEl = document.getElementById('status');
                        if (statusEl) statusEl.textContent = 'Reconnecting...';
                        // Retry the stream with exponential backoff
                        setTimeout(connectSessionListStream, Math.min(1000 * Math.pow(2, streamRetryCount), 10000));
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
                        reloadForVersion(event.version);
                        return;
                    }
                    // Store version on first connect
                    if (event.version) {
                        serverVersion = event.version;
                    }
                    loadSessions();
                    break;

                case 'created':
                    // New session - keep the full list, then render only sessions allowed by plan.
                    if (event.session) {
                        mergeSessionListUpdate(event.session);
                        syncRenderedSessions();
                    }
                    break;

                case 'updated':
                    // Session updated - merge activity/state changes, then recompute visibility.
                    if (event.session && event.session.id) {
                        mergeSessionListUpdate(event.session);
                        syncRenderedSessions();
                    }
                    break;

                case 'deleted':
                    // Session deleted - remove from view
                    if (event.session && event.session.id) {
                        allSessions = allSessions.filter(s => s.id !== event.session.id);
                        syncRenderedSessions();
                        if (sessions.size === 0) {
                            container.innerHTML = '<div class="no-sessions">' + getSessionsEmptyMessage(allSessions.length > 0) + '</div>';
                            // If all sessions disappeared, verify the build before
                            // treating it as a deploy.
                            if (hadSessionsBefore && !isDeploying) {
                                void checkVersionAndReload();
                            }
                        }
                    }
                    break;
            }
        }

        // Detect when tab becomes visible again and check connection
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'visible') {
                // Check if the WebSocket connection is still alive.
                if (sessionListSocket && sessionListSocket.readyState === WebSocket.CLOSED) {
                    console.log('Session list WebSocket closed while tab was hidden, reconnecting...');
                    // Don't show deploy overlay for tab visibility changes - just reconnect silently
                    streamRetryCount = 0;
                    loadSessions();
                    connectSessionListStream();
                } else if (!sessionListSocket) {
                    // No WebSocket connection at all - reconnect
                    console.log('No session list WebSocket, reconnecting...');
                    streamRetryCount = 0;
                    loadSessions();
                    connectSessionListStream();
                }
            }
        });

`;
