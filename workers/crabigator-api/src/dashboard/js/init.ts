// Dashboard JavaScript - init
export const initJs = `
        // Initial load (wrapped in async IIFE for await support)
        (async function() {
            // Single session focus mode renders only the focused card in the main view.
            if (isFocusedMode()) {
                applyFocusMode();
            } else {
                setLayout(currentLayout);  // Apply saved layout preference
            }
            applyGrouping();           // Apply saved grouping preference
            updateFilterIndicator();
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
                const statusEl = document.getElementById('status');
                if (statusEl) statusEl.textContent = 'Not paired';
            } else {
                // Paired - load sessions normally
                initSidebar();
                refreshAccountMenu();
                loadSessions();
                connectSessionListStream();
                // Start viewer activity tracking for streaming optimization
                startViewerActivityTracking();
                // Offer the billing portal to accounts that still pay
                if (BILLING_ENABLED) fetchSubscription();
            }
        })();
`;
