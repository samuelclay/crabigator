// Dashboard JavaScript - init
export const initJs = `
        // Initial load
        setLayout(currentLayout);  // Apply saved layout preference
        loadSettingsFromServer();  // Load style preferences

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
        }
`;
