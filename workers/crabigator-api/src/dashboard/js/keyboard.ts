// Dashboard JavaScript - keyboard shortcuts popover
export const keyboardJs = `
        function toggleKeyboardPopover(sessionId) {
            // Close all other keyboard popovers
            document.querySelectorAll('.keyboard-popover.visible').forEach(p => {
                if (p.parentElement.closest('.session-card')?.id !== 'session-' + sessionId) {
                    p.classList.remove('visible');
                }
            });

            const container = document.querySelector('#session-' + sessionId + ' .keyboard-container');
            if (!container) return;
            const popover = container.querySelector('.keyboard-popover');
            if (!popover) return;

            if (popover.classList.contains('visible')) {
                popover.classList.remove('visible');
                return;
            }

            // Determine positioning: up or down
            const rect = container.getBoundingClientRect();
            if (rect.top < 350) {
                popover.classList.add('drop-down');
            } else {
                popover.classList.remove('drop-down');
            }

            // Determine alignment: left or right
            if (rect.left > window.innerWidth - 280) {
                popover.classList.add('align-right');
            } else {
                popover.classList.remove('align-right');
            }

            popover.classList.add('visible');

            // Scroll so the full popover is visible
            requestAnimationFrame(() => {
                popover.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            });
        }

        async function sendSessionKey(sessionId, key) {
            // Close popover
            const popover = document.querySelector('#session-' + sessionId + ' .keyboard-popover');
            if (popover) popover.classList.remove('visible');

            try {
                await fetch(API_BASE + '/sessions/' + sessionId + '/key', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ key })
                });
            } catch {}
        }

        // Close keyboard popover on outside click
        document.addEventListener('click', function(e) {
            if (!e.target.closest('.keyboard-container')) {
                document.querySelectorAll('.keyboard-popover.visible').forEach(p => {
                    p.classList.remove('visible');
                });
            }
        });
`;
