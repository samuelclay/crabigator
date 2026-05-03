// Dashboard JavaScript - sessions popover (legacy functions kept for compatibility)
export const sessionsPopoverJs = `
        function updateSessionsCount() {
            const countEl = document.getElementById('sessions-count');
            const labelEl = document.querySelector('.sessions-label');
            const count = visibleSessionIds.size || getRenderableSessions(allSessions).length;
            if (countEl) {
                countEl.textContent = count;
            }
            if (labelEl) {
                labelEl.textContent = count === 1 ? 'session' : 'sessions';
            }
            // Update sidebar content
            if (typeof updateSidebarContent === 'function') {
                updateSidebarContent();
            }
        }
`;
