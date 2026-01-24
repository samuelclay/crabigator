// Dashboard JavaScript - layout
export const layoutJs = `
        function setLayout(layout) {
            currentLayout = layout;
            localStorage.setItem('crabigator-layout', layout);
            const container = document.getElementById('sessions');
            container.dataset.layout = layout;

            // Update button states
            document.querySelectorAll('.style-option[data-layout]').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.layout === layout);
            });

            // For 'fit' mode, calculate columns based on available width
            if (layout === 'fit') {
                const count = sessions.size || 1;
                const containerWidth = container.offsetWidth || window.innerWidth - 32;
                const minCardWidth = 450;  // Minimum comfortable card width
                const maxCols = Math.max(Math.floor(containerWidth / minCardWidth), 1);
                // Don't use more columns than we have sessions
                const cols = Math.min(maxCols, count);
                container.style.columnCount = cols;
                // Update per-group fit columns for project grouping mode
                if (typeof updateAllProjectFitColumns === 'function') {
                    updateAllProjectFitColumns();
                }
            } else {
                container.style.columnCount = '';
            }

            // After layout change, scroll pinned sessions to bottom
            requestAnimationFrame(() => {
                for (const [id, sessionData] of sessions) {
                    if (sessionData.pinned) {
                        const terminal = document.getElementById('terminal-' + id);
                        if (terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                }
            });
        }

        // Recalculate fit layout when session count changes
        function updateFitLayout() {
            if (currentLayout === 'fit') {
                setLayout('fit');
            }
        }

        // Recalculate fit layout on window resize
        let resizeTimeout;
        window.addEventListener('resize', () => {
            if (currentLayout === 'fit') {
                clearTimeout(resizeTimeout);
                resizeTimeout = setTimeout(() => setLayout('fit'), 100);
            }
        });

`;
