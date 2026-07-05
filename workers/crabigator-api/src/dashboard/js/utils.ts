// Dashboard JavaScript - utils
export const utilsJs = `

        function formatDuration(seconds) {
            if (seconds < 60) return seconds + 's';
            const mins = Math.floor(seconds / 60);
            if (mins < 60) return mins + 'm';
            const hours = Math.floor(mins / 60);
            return hours + 'h ' + (mins % 60) + 'm';
        }

        function sessionCount(n) {
            return n + (n === 1 ? ' session' : ' sessions');
        }

        function clearSessionFilter() {
            setSessionFocus(null);
        }

        function focusOnSession(sessionId) {
            setSessionFocus(sessionId);
        }

        function updateFilterIndicator() {
            const filterIndicator = document.getElementById('filter-indicator');
            if (!filterIndicator) return;

            filterIndicator.classList.toggle('visible', isFocusedMode());
            if (!isFocusedMode()) return;

            const filterText = filterIndicator.querySelector('.filter-text');
            if (filterText) {
                filterText.textContent = 'Focused: ' + singleSessionId.slice(0, 8);
            }
        }

        function applyFocusMode() {
            const container = document.getElementById('sessions');
            if (!container) return;

            container.classList.toggle('focused-mode', isFocusedMode());
            if (isFocusedMode()) {
                container.dataset.layout = '1';
                container.dataset.grouping = 'all';
                container.style.columnCount = '';
            }
            updateFilterIndicator();
        }

        function setSessionFocus(sessionId, options = {}) {
            const nextSessionId = sessionId || null;
            const replace = options.replace === true;

            const url = new URL(window.location.href);
            if (nextSessionId) {
                url.searchParams.set('session', nextSessionId);
            } else {
                url.searchParams.delete('session');
            }

            const historyMethod = replace ? 'replaceState' : 'pushState';
            window.history[historyMethod]({}, '', url.pathname + url.search + url.hash);

            singleSessionId = nextSessionId;
            resetVisibleSessionLock();
            applyFocusMode();

            if (!isFocusedMode() && typeof setLayout === 'function') {
                setLayout(currentLayout);
            }
            if (typeof applyGrouping === 'function') {
                applyGrouping();
            }

            if (typeof syncRenderedSessions === 'function') {
                syncRenderedSessions();
            }
            if (typeof updateSessionsCount === 'function') {
                updateSessionsCount();
            }
            if (typeof updateSidebarActiveState === 'function') {
                updateSidebarActiveState();
            }

            const sidebar = document.getElementById('sidebar');
            if (sidebar && !sidebarPinned) {
                sidebar.classList.add('collapsed');
                if (typeof closeSidebarSettings === 'function') closeSidebarSettings();
                if (typeof updateSessionsButtonState === 'function') updateSessionsButtonState();
                if (typeof updateSidebarBackdrop === 'function') updateSidebarBackdrop();
            }
        }

        window.addEventListener('popstate', () => {
            singleSessionId = readFocusedSessionId();
            resetVisibleSessionLock();
            applyFocusMode();

            if (!isFocusedMode() && typeof setLayout === 'function') {
                setLayout(currentLayout);
            }
            if (typeof applyGrouping === 'function') {
                applyGrouping();
            }
            if (typeof syncRenderedSessions === 'function') {
                syncRenderedSessions();
            }
            if (typeof updateSessionsCount === 'function') {
                updateSessionsCount();
            }
        });

`;
