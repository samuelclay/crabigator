// Dashboard JavaScript - viewer activity tracking
// Sends heartbeats to server when user is actively viewing the dashboard
// This allows desktop to optimize streaming (reduce frequency when no one watching)

export const viewerActivityJs = `
        // Viewer activity tracking
        let lastActivity = Date.now();
        let viewerHeartbeatInterval = null;
        let usageHeartbeatInterval = null;
        const VIEWER_INACTIVITY_TIMEOUT = 30000;  // 30 seconds
        const VIEWER_HEARTBEAT_INTERVAL = 5000;   // 5 seconds

        // Track user activity
        function onViewerActivity() {
            lastActivity = Date.now();
        }

        // Send heartbeat for a specific session (for desktop notifications)
        async function sendViewerHeartbeat(sessionId) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/viewer-active', {
                    method: 'POST',
                    headers: getAuthHeaders()
                });
                if (resp) handleAuthFailure(resp);
            } catch {
                // Ignore errors - heartbeats are best-effort
            }
        }

        // Send usage heartbeat (ONE per browser tab, for billing)
        async function sendUsageHeartbeat() {
            // Only send if user has been active recently
            if (Date.now() - lastActivity > VIEWER_INACTIVITY_TIMEOUT) {
                return;
            }

            try {
                await fetch(API_BASE + '/usage/heartbeat', {
                    method: 'POST',
                    headers: getAuthHeaders()
                });
            } catch {
                // Ignore errors - heartbeats are best-effort
            }
        }

        // Send heartbeat for all active sessions (for desktop notifications)
        async function sendViewerHeartbeats() {
            // Only send if user has been active recently
            if (Date.now() - lastActivity > VIEWER_INACTIVITY_TIMEOUT) {
                return;
            }

            // Send heartbeat to all connected sessions
            for (const [sessionId] of sessions) {
                sendViewerHeartbeat(sessionId);
            }
        }

        // Start viewer activity tracking
        function startViewerActivityTracking() {
            // Listen for user activity events
            ['scroll', 'mousemove', 'touchstart', 'keydown', 'click'].forEach(event => {
                document.addEventListener(event, onViewerActivity, { passive: true });
            });

            // Send viewer heartbeats periodically (for desktop notifications)
            if (viewerHeartbeatInterval) {
                clearInterval(viewerHeartbeatInterval);
            }
            viewerHeartbeatInterval = setInterval(sendViewerHeartbeats, VIEWER_HEARTBEAT_INTERVAL);

            // Send SINGLE usage heartbeat per browser tab (for billing)
            if (usageHeartbeatInterval) {
                clearInterval(usageHeartbeatInterval);
            }
            usageHeartbeatInterval = setInterval(sendUsageHeartbeat, VIEWER_HEARTBEAT_INTERVAL);
            sendUsageHeartbeat();  // Send initial heartbeat immediately
        }

        // Stop viewer activity tracking
        function stopViewerActivityTracking() {
            if (viewerHeartbeatInterval) {
                clearInterval(viewerHeartbeatInterval);
                viewerHeartbeatInterval = null;
            }
            if (usageHeartbeatInterval) {
                clearInterval(usageHeartbeatInterval);
                usageHeartbeatInterval = null;
            }
        }
`;
