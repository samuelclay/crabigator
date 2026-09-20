// Dashboard JavaScript - events
export const eventsJs = `
        // Coalesce screen updates. Grok's TUI redraws many times a second;
        // painting every frame stalls typing in the message box.
        const pendingScreenUpdates = new Map();
        const screenUpdateTimers = new Map();

        function sessionInputIsFocused(sessionId) {
            const input = document.getElementById('input-' + sessionId);
            return !!(input && document.activeElement === input);
        }

        function applyScreenUpdate(sessionId, content) {
            const screenEl = document.getElementById('screen-' + sessionId);
            if (screenEl) screenEl.innerHTML = ansiToHtml(content);
            const sessionData = sessions.get(sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (sessionData?.pinned && terminal) {
                terminal.scrollTop = terminal.scrollHeight;
                sessionData.lastScrollTop = terminal.scrollTop;
            }
        }

        function scheduleScreenUpdate(sessionId, content) {
            pendingScreenUpdates.set(sessionId, content);
            if (screenUpdateTimers.has(sessionId)) return;
            const delay = sessionInputIsFocused(sessionId) ? 250 : 80;
            screenUpdateTimers.set(sessionId, setTimeout(() => {
                screenUpdateTimers.delete(sessionId);
                const latestContent = pendingScreenUpdates.get(sessionId);
                pendingScreenUpdates.delete(sessionId);
                if (latestContent !== undefined) {
                    applyScreenUpdate(sessionId, latestContent);
                }
            }, delay));
        }

        function grokStatusLooksLike(text) {
            return /^(Waiting for response|Thinking|Responding|Preparing\b|Running:|Writing edit|Updating todo)/i.test(String(text || '').trim());
        }

        function grokStableTitle(title) {
            let text = String(title || '').replace(/^↯\s+/, '').trim();
            text = text.replace(/\s+-\s+grok$/i, '').trim();
            text = text.replace(/^-\s+/, '').trim();
            if (!text || /^grok( build)?$/i.test(text)) return null;
            const sep = text.lastIndexOf(' - ');
            if (sep !== -1) {
                const status = text.slice(0, sep).trim();
                const prompt = text.slice(sep + 3).trim();
                if (prompt && grokStatusLooksLike(status)) return '↯  ' + prompt;
            }
            if (grokStatusLooksLike(text)) return null;
            return '↯  ' + text;
        }

        function sessionLooksLikeGrok(sessionId) {
            const sessionData = sessions.get(sessionId);
            const card = document.getElementById('session-' + sessionId);
            const platform = sessionData?.platform || card?.dataset?.platform || '';
            return String(platform).toLowerCase() === 'grok';
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

        const sessionReconnectTimers = new Map();

        function cancelSessionReconnect(sessionId) {
            const timer = sessionReconnectTimers.get(sessionId);
            if (!timer) return;
            clearTimeout(timer);
            sessionReconnectTimers.delete(sessionId);
        }

        function closeSessionSocket(session) {
            if (session?.sessionId) cancelSessionReconnect(session.sessionId);
            const ws = session?.eventSocket;
            if (!ws) return;
            session.eventSocket = null;
            ws.onclose = null;
            ws.onerror = null;
            ws.onmessage = null;
            ws.onopen = null;
            if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
                ws.close();
            }
        }

        function scheduleSessionReconnect(sessionId) {
            if (document.visibilityState === 'hidden') return;
            if (sessionReconnectTimers.has(sessionId)) return;
            sessionReconnectTimers.set(sessionId, setTimeout(() => {
                sessionReconnectTimers.delete(sessionId);
                if (!sessions.has(sessionId)) return;
                connectToSession(sessionId);
            }, 250));
        }

        function connectToSession(sessionId) {
            closeSessionSocket(sessions.get(sessionId));
            console.log('Connecting WebSocket for session:', sessionId);
            const eventSocket = new WebSocket(getWebSocketUrl('/sessions/' + sessionId + '/events'));

            eventSocket.onopen = () => {
                console.log('WebSocket connected for session:', sessionId);
                const screenEl = document.getElementById('screen-' + sessionId);
                if (screenEl && screenEl.innerHTML === 'Connecting...') {
                    screenEl.innerHTML = '<span style="color:#8b949e">Connected, waiting for screen data...</span>';
                }
                // Send viewer heartbeat immediately - this triggers desktop to send screen
                // Must happen after the socket connects so we can receive the screen event.
                sendViewerHeartbeat(sessionId);
            };

            eventSocket.onmessage = (event) => {
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

            eventSocket.onerror = (err) => {
                console.error('WebSocket error for session ' + sessionId, err);
            };

            eventSocket.onclose = () => {
                const session = sessions.get(sessionId);
                if (!session || session.eventSocket !== eventSocket) return;
                session.eventSocket = null;
                // Keep the card. Phone lock and tab freezes drop sockets;
                // tearing down the card is what makes the dashboard reload
                // and lose the session you were looking at.
                if (isDeploying) {
                    scheduleReconnect();
                    return;
                }
                scheduleSessionReconnect(sessionId);
            };

            const session = sessions.get(sessionId);
            if (session) {
                session.eventSocket = eventSocket;
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
                    // Grok's OSC title is a live status line; keep the prompt.
                    {
                        let title = event.title;
                        if (sessionLooksLikeGrok(sessionId)) {
                            title = grokStableTitle(title);
                            if (!title) break;
                        }
                        if (sessionData && sessionData.title === title) {
                            break;
                        }
                        if (sessionData) {
                            sessionData.title = title;
                            sessionData.generatedTitle = title;
                        }
                        updateSessionTitleHierarchy(sessionId);
                        updateTitlesWidget(sessionId, [title]);
                    }
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
                                closeSessionSocket(session);
                                sessions.delete(sessionId);
                                if (activeTerminalId === sessionId) activeTerminalId = null;
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
                            syncRenderedSessions();
                            // Update status
                            const statusEl = document.getElementById('status');
                            if (statusEl) statusEl.textContent = sessionCount(sessions.size);
                        });
                    }
                    break;
                case 'title_history':
                    {
                        let history = event.history;
                        if (sessionLooksLikeGrok(sessionId) && Array.isArray(history)) {
                            const seen = new Set();
                            history = [];
                            for (const item of event.history) {
                                const stable = grokStableTitle(item);
                                if (stable && !seen.has(stable)) {
                                    seen.add(stable);
                                    history.push(stable);
                                }
                            }
                        }
                        updateTitlesWidget(sessionId, history);
                    }
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
