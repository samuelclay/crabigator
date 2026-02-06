// Dashboard JavaScript - constants
export const constantsJs = `

        const API_BASE = '/api';
        const sessions = new Map(); // sessionId -> { eventSource, state, element, git, changes, stats }
        let allSessions = []; // All sessions from API (for popover)
        let allProjects = []; // All known projects from API (for history)
        let currentLayout = localStorage.getItem('crabigator-layout') || 'fit';

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        // Font size scaling
        const FONT_SCALES = [0.75, 0.85, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5];
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

        // Widgets panel visibility (true = expanded, false = collapsed by default)
        let widgetsExpanded = false;

        // Session grouping mode ('all' = flat list, 'project' = grouped by working directory)
        let groupingMode = localStorage.getItem('crabigator-grouping') || 'project';

        // Project ordering mode when grouped ('recent' = most recently active first, 'alpha' = alphabetical)
        let projectOrderMode = localStorage.getItem('crabigator-project-order') || 'recent';

        // Track collapsed state for project groups
        const collapsedProjects = new Set(JSON.parse(localStorage.getItem('crabigator-collapsed-projects') || '[]'));

        // Single session filter mode (from ?session=xxx URL parameter)
        const singleSessionId = new URLSearchParams(window.location.search).get('session');

        // Spinner frames for thinking indicator
        const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let spinnerFrameIndex = 0;

        // Start the global spinner animation
        setInterval(() => {
            const spinners = document.querySelectorAll('.thinking-spinner');
            if (spinners.length === 0) return;
            spinnerFrameIndex = (spinnerFrameIndex + 1) % SPINNER_FRAMES.length;
            const frame = SPINNER_FRAMES[spinnerFrameIndex];
            spinners.forEach(el => {
                el.textContent = frame;
            });
        }, 80);
`;
