// Dashboard JavaScript - session-state
export const sessionStateJs = `
        function updateSessionHeader(session) {
            const card = document.getElementById('session-' + session.id);
            if (!card) return;
            const stateEl = card.querySelector('.state');
            stateEl.className = 'state ' + session.state;
            stateEl.textContent = session.state;
            const sessionData = sessions.get(session.id);
            if (sessionData) {
                sessionData.state = session.state;
                updateSessionSummary(session.id, sessionData);
            }
        }

        function updatePinButton(sessionId, pinned) {
            const el = document.getElementById('pin-' + sessionId);
            if (!el) return;
            const parent = el.parentNode;
            if (!parent) return;

            if (pinned) {
                // Show neutral indicator
                const indicator = document.createElement('span');
                indicator.id = 'pin-' + sessionId;
                indicator.className = 'pin-indicator pinned';
                indicator.title = 'Auto-scroll enabled';
                indicator.textContent = '⇣ pinned';
                parent.replaceChild(indicator, el);
            } else {
                // Show clickable button
                const btn = document.createElement('button');
                btn.id = 'pin-' + sessionId;
                btn.className = 'pin-btn';
                btn.title = 'Click to pin to bottom';
                btn.textContent = '⇣ Pin';
                btn.onclick = () => togglePin(sessionId);
                parent.replaceChild(btn, el);
            }
        }

        function togglePin(sessionId) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;

            const terminal = document.getElementById('terminal-' + sessionId);
            if (!terminal) return;

            if (sessionData.pinned) {
                // Unpin
                sessionData.pinned = false;
                updatePinButton(sessionId, false);
            } else {
                // Pin and scroll to bottom
                sessionData.pinned = true;
                updatePinButton(sessionId, true);
                terminal.scrollTop = terminal.scrollHeight;
                sessionData.lastScrollTop = terminal.scrollTop;
            }
        }

        // Collapse state persistence
        function getCollapsedSessions() {
            try {
                return JSON.parse(localStorage.getItem('collapsedSessions') || '{}');
            } catch { return {}; }
        }

        function setCollapsedSession(sessionId, collapsed) {
            const state = getCollapsedSessions();
            if (collapsed) {
                state[sessionId] = true;
            } else {
                delete state[sessionId];
            }
            localStorage.setItem('collapsedSessions', JSON.stringify(state));
        }

        function isSessionCollapsed(sessionId) {
            return getCollapsedSessions()[sessionId] === true;
        }

        function toggleCollapse(sessionId) {
            const card = document.getElementById('session-' + sessionId);
            const btn = document.getElementById('collapse-btn-' + sessionId);
            if (!card || !btn) return;

            const isCollapsed = card.classList.contains('collapsed');
            if (isCollapsed) {
                card.classList.remove('collapsed');
                btn.classList.remove('collapsed');
                setCollapsedSession(sessionId, false);
            } else {
                card.classList.add('collapsed');
                btn.classList.add('collapsed');
                setCollapsedSession(sessionId, true);
                // Update summary when collapsing
                const sessionData = sessions.get(sessionId);
                if (sessionData) {
                    updateSessionSummary(sessionId, sessionData);
                }
            }
            updateFitLayout();
        }

        function applyCollapsedState(sessionId) {
            if (isSessionCollapsed(sessionId)) {
                const card = document.getElementById('session-' + sessionId);
                const btn = document.getElementById('collapse-btn-' + sessionId);
                if (card) card.classList.add('collapsed');
                if (btn) btn.classList.add('collapsed');
            }
        }

        function updateSessionSummary(sessionId, sessionData) {
            // Stats summary
            const statsEl = document.getElementById('summary-stats-' + sessionId);
            if (statsEl) {
                const s = sessionData.stats || {};
                const parts = [];
                // Show state first
                const state = sessionData.state || s.state;
                if (state) parts.push(state);
                if (s.prompts) parts.push(s.prompts + ' prompts');
                if (s.tools) parts.push(s.tools + ' tools');
                if (s.thinking_seconds) parts.push(formatDuration(s.thinking_seconds) + ' thinking');
                statsEl.textContent = parts.length ? parts.join(', ') : '—';
            }

            // Git summary
            const gitEl = document.getElementById('summary-git-' + sessionId);
            if (gitEl && sessionData.git) {
                const g = sessionData.git;
                const adds = g.files?.reduce((sum, f) => sum + (f.additions || 0), 0) || 0;
                const dels = g.files?.reduce((sum, f) => sum + (f.deletions || 0), 0) || 0;
                const fileCount = g.files?.length || 0;
                if (fileCount > 0) {
                    gitEl.innerHTML = '<span class="green">+' + adds + '</span> <span class="red">-' + dels + '</span> in ' + fileCount + ' file' + (fileCount !== 1 ? 's' : '');
                } else {
                    gitEl.textContent = 'clean';
                }
            }

            // Changes summary
            const changesEl = document.getElementById('summary-changes-' + sessionId);
            if (changesEl && sessionData.changes) {
                const c = sessionData.changes;
                const total = c.by_language?.reduce((sum, l) => sum + l.changes.length, 0) || 0;
                if (total > 0) {
                    const langs = c.by_language?.map(l => l.language).join(', ') || '';
                    changesEl.innerHTML = '<span class="purple">' + total + '</span> change' + (total !== 1 ? 's' : '') + (langs ? ' (' + langs + ')' : '');
                } else {
                    changesEl.textContent = 'none';
                }
            }
        }

        let activePopover = null;

        function toggleInfoPopover(sessionId) {
            const popover = document.getElementById('info-popover-' + sessionId);
            if (!popover) return;

            // Close any other open popover
            if (activePopover && activePopover !== popover) {
                activePopover.classList.remove('visible');
            }

            const isVisible = popover.classList.contains('visible');
            if (isVisible) {
                popover.classList.remove('visible');
                activePopover = null;
            } else {
                popover.classList.add('visible');
                activePopover = popover;
            }
        }

        function copyToClipboard(text) {
            navigator.clipboard.writeText(text).then(() => {
                // Brief visual feedback could be added here
                console.log('Copied to clipboard:', text);
            }).catch(err => {
                console.error('Failed to copy:', err);
            });
        }

        // Close popover when clicking outside
        document.addEventListener('click', (e) => {
            if (!activePopover) return;

            const target = e.target;
            // Check if click is on info button or inside popover
            const isInfoBtn = target.closest('.info-btn');
            const isInsidePopover = target.closest('.info-popover');

            if (!isInfoBtn && !isInsidePopover) {
                activePopover.classList.remove('visible');
                activePopover = null;
            }
        });

`;
