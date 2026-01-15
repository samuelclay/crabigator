// Dashboard JavaScript - init
export const initJs = `
        // Initial load and connect to SSE for real-time updates
        setLayout(currentLayout);  // Apply saved layout preference
        loadSettingsFromServer();  // Load style preferences
        loadSessions();
        connectSessionListStream();
`;
