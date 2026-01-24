// Dashboard JavaScript - constants
export const constantsJs = `

        const API_BASE = '/api';
        const sessions = new Map(); // sessionId -> { eventSource, state, element, git, changes, stats }
        let currentLayout = localStorage.getItem('crabigator-layout') || '1';

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        // Font size scaling
        const FONT_SCALES = [0.75, 0.85, 0.9, 1.0, 1.1, 1.25, 1.5];
        let currentFontScaleIndex = 3; // default 1.0
        let isChangingFontSize = false; // Flag to prevent scroll unpinning during zoom changes

        // Terminal height scaling
        const TERMINAL_HEIGHTS = [150, 200, 250, 350, 450, 550, 700];
        let currentHeightIndex = 3; // default 350px

        // Scrollback chunking - only render CHUNK_SIZE lines at a time for performance
        const SCROLLBACK_CHUNK_SIZE = 1000;

        // Load more scrollback lines when user scrolls to top

        // Terminal wrap mode (true = wrap text, false = horizontal scroll)
        let terminalWrapEnabled = true;
`;
