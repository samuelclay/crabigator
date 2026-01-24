// Dashboard JavaScript - viewer activity tracking
// Sends heartbeats to server when user is actively viewing the dashboard
// This allows desktop to optimize streaming (reduce frequency when no one watching)

export const viewerActivityJs = `
        // Viewer activity tracking
        let lastActivity = Date.now();
        let viewerHeartbeatInterval = null;
        const VIEWER_INACTIVITY_TIMEOUT = 30000;  // 30 seconds
        const VIEWER_HEARTBEAT_INTERVAL = 5000;   // 5 seconds

        // Track user activity
        function onViewerActivity() {
            lastActivity = Date.now();
        }

        // Send heartbeat for a specific session
        async function sendViewerHeartbeat(sessionId) {
            try {
                await fetch(API_BASE + '/sessions/' + sessionId + '/viewer-active', {
                    method: 'POST'
                });
            } catch {
                // Ignore errors - heartbeats are best-effort
            }
        }

        // Send heartbeat for all active sessions
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

            // Send heartbeats periodically
            // Initial heartbeat for each session is sent in connectToSession's onopen handler
            if (viewerHeartbeatInterval) {
                clearInterval(viewerHeartbeatInterval);
            }
            viewerHeartbeatInterval = setInterval(sendViewerHeartbeats, VIEWER_HEARTBEAT_INTERVAL);
        }

        // Stop viewer activity tracking
        function stopViewerActivityTracking() {
            if (viewerHeartbeatInterval) {
                clearInterval(viewerHeartbeatInterval);
                viewerHeartbeatInterval = null;
            }
        }
`;
