// Dashboard JavaScript - sessions popover (legacy functions kept for compatibility)
export const sessionsPopoverJs = `
        function updateSessionsCount() {
            const buttonEl = document.getElementById('sessions-btn');
            const countEl = document.getElementById('sessions-count');
            const labelEl = document.querySelector('.sessions-label');
            if (sessionsAreStillLoading()) {
                if (countEl) {
                    countEl.textContent = 'Loading';
                }
                if (labelEl) {
                    labelEl.textContent = 'sessions';
                }
                if (buttonEl) {
                    buttonEl.setAttribute('aria-label', 'Loading sessions');
                    buttonEl.setAttribute('aria-busy', 'true');
                }
                return;
            }

            const count = isFocusedMode()
                ? allSessions.length
                : (visibleSessionIds.size || getRenderableSessions(allSessions).length);
            if (countEl) {
                countEl.textContent = count;
            }
            if (labelEl) {
                labelEl.textContent = count === 1 ? 'session' : 'sessions';
            }
            if (buttonEl) {
                buttonEl.setAttribute('aria-label', count + (count === 1 ? ' session' : ' sessions'));
                buttonEl.setAttribute('aria-busy', 'false');
            }
            // Update sidebar content
            if (typeof updateSidebarContent === 'function') {
                updateSidebarContent();
            }
        }
`;
