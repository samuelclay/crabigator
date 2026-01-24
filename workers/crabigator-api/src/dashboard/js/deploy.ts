// Dashboard JavaScript - deploy
export const deployJs = `
        // Deploy detection and reconnection
        let isDeploying = false;
        let deployReconnectDelay = 500;
        const MAX_RECONNECT_DELAY = 10000;
        let reconnectTimeout = null;
        let hadSessionsBefore = false;
        let lastSuccessfulConnection = 0;  // Timestamp of last successful API call
        const DEPLOY_DETECTION_WINDOW = 30000;  // Only detect deploy if connected within last 30s

        function wasRecentlyConnected() {
            return Date.now() - lastSuccessfulConnection < DEPLOY_DETECTION_WINDOW;
        }

        function showDeployOverlay() {
            isDeploying = true;
            document.getElementById('deploy-overlay').classList.add('visible');
            updateDeployCountdown();
        }

        function hideDeployOverlay() {
            isDeploying = false;
            deployReconnectDelay = 500;
            document.getElementById('deploy-overlay').classList.remove('visible');
        }

        function updateDeployCountdown() {
            const el = document.getElementById('deploy-countdown');
            if (el && isDeploying) {
                el.textContent = 'Retrying in ' + (deployReconnectDelay / 1000).toFixed(1) + 's...';
            }
        }

        function scheduleReconnect() {
            if (reconnectTimeout) clearTimeout(reconnectTimeout);
            updateDeployCountdown();
            reconnectTimeout = setTimeout(() => {
                // Reconnect SSE - it calls loadSessions() on 'connected' event
                // and receives real-time 'created' events for sessions that reconnect
                sseRetryCount = 0;
                connectSessionListStream();
                deployReconnectDelay = Math.min(deployReconnectDelay * 1.5, MAX_RECONNECT_DELAY);
            }, deployReconnectDelay);
        }

`;
