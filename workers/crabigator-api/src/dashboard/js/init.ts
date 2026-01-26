// Dashboard JavaScript - init
export const initJs = `
        // Initial load (wrapped in async IIFE for await support)
        (async function() {
            // Single session mode - show filter indicator
            if (singleSessionId) {
                document.getElementById('sessions').dataset.layout = '1';
                // Show filter indicator
                const filterIndicator = document.getElementById('filter-indicator');
                if (filterIndicator) {
                    filterIndicator.classList.add('visible');
                    const filterText = filterIndicator.querySelector('.filter-text');
                    if (filterText) {
                        filterText.textContent = 'Filtered: ' + singleSessionId.slice(0, 8);
                    }
                }
            } else {
                setLayout(currentLayout);  // Apply saved layout preference
            }
            applyGrouping();           // Apply saved grouping preference
            loadSettingsFromServer();  // Load style preferences

            // Check for auto-setup via URL parameter (for Chrome MCP)
            const handledSetup = await handleSetupParam();
            if (handledSetup) {
                // Setup in progress or completed - don't continue normal init
                return;
            }

            // Check pairing status - if not paired, show pairing gate
            if (!checkPairingStatus()) {
                showPairingGate();
                document.getElementById('status').textContent = 'Not paired';
            } else {
                // Paired - load sessions normally
                loadSessions();
                connectSessionListStream();
                // Start viewer activity tracking for streaming optimization
                startViewerActivityTracking();
                // Initialize usage tracking and paywall
                initUsageTracking();
            }
        })();
`;
