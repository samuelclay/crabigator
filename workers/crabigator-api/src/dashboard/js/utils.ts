// Dashboard JavaScript - utils
export const utilsJs = `

        function formatDuration(seconds) {
            if (seconds < 60) return seconds + 's';
            const mins = Math.floor(seconds / 60);
            const secs = seconds % 60;
            if (mins < 60) return mins + 'm ' + secs + 's';
            const hours = Math.floor(mins / 60);
            return hours + 'h ' + (mins % 60) + 'm';
        }

        function sessionCount(n) {
            return n + (n === 1 ? ' session' : ' sessions');
        }

        function clearSessionFilter() {
            // Remove the session parameter and reload
            const url = new URL(window.location.href);
            url.searchParams.delete('session');
            window.location.href = url.toString();
        }

        function focusOnSession(sessionId) {
            // Navigate to single-session view
            const url = new URL(window.location.href);
            url.searchParams.set('session', sessionId);
            window.location.href = url.toString();
        }

`;
