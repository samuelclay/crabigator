// Dashboard JavaScript - sessions popover (legacy functions kept for compatibility)
export const sessionsPopoverJs = `
        function updateSessionsCount() {
            const buttonEl = document.getElementById('sessions-btn');
            const countEl = document.getElementById('sessions-count');
            const labelEl = document.querySelector('.sessions-label');
            // Only write when the value changed. This runs on every streamed
            // session event; unconditionally replacing the button's text nodes
            // can swallow an in-progress tap on mobile (the node under the
            // finger disappears before the browser synthesizes the click).
            const setText = (el, value) => {
                if (el && el.textContent !== String(value)) el.textContent = value;
            };
            if (sessionsAreStillLoading()) {
                setText(countEl, 'Loading');
                setText(labelEl, 'sessions');
                if (buttonEl) {
                    buttonEl.setAttribute('aria-label', 'Loading sessions');
                    buttonEl.setAttribute('aria-busy', 'true');
                }
                return;
            }

            const count = isFocusedMode()
                ? allSessions.length
                : (visibleSessionIds.size || getRenderableSessions(allSessions).length);
            setText(countEl, count);
            setText(labelEl, count === 1 ? 'session' : 'sessions');
            if (buttonEl) {
                buttonEl.setAttribute('aria-label', count + (count === 1 ? ' session' : ' sessions'));
                buttonEl.setAttribute('aria-busy', 'false');
            }
            // Batch sidebar re-renders instead of rebuilding on every event.
            if (typeof scheduleSidebarUpdate === 'function') {
                scheduleSidebarUpdate();
            }
        }
`;
