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

            // Check for gift claim via URL parameter
            const handledGift = await handleGiftParam();
            // Note: Gift handling shows overlay but doesn't block normal init
            // User can dismiss and still use the dashboard

            // Check pairing status - if not paired, show pairing gate
            if (!checkPairingStatus()) {
                showPairingGate();
                const statusEl = document.getElementById('status');
                if (statusEl) statusEl.textContent = 'Not paired';
            } else {
                // Paired - load sessions normally
                initSidebar();
                loadSessions();
                connectSessionListStream();
                // Start viewer activity tracking for streaming optimization
                startViewerActivityTracking();
                // Initialize usage tracking and paywall
                initUsageTracking();
                // Check for pending gift that was claimed before pairing
                resolvePendingGift();
            }
        })();
`;
