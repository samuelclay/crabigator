// Dashboard JavaScript - constants
export const constantsJs = `

        const API_BASE = '/api';
        const sessions = new Map(); // sessionId -> { eventSource, state, element, git, changes, stats }
        let allSessions = []; // All sessions from API (for popover)
        let allProjects = []; // All known projects from API (for history)
        const FREE_VISIBLE_SESSION_LIMIT = 3;
        let isProUser = false;
        let hiddenSessionCount = 0;
        let visibleSessionIds = new Set();
        // Frozen set of session IDs chosen at first render. Free-tier dashboards
        // pick the top-N by recent activity once and keep them locked until the
        // page reloads, so SSE activity on hidden sessions can't churn the cards.
        // null = not yet picked; Set = locked.
        let lockedVisibleSessionIds = null;
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
        let activeTerminalId = null; // Session ID of terminal with scroll activated

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

        // Session-card section visibility (recap panel, PR list, commit history, git status, changes)
        let visibleSections = { recap: true, prs: true, commits: true, git: true, changes: true };

        // Session grouping mode ('all' = flat list, 'project' = grouped by working directory)
        let groupingMode = localStorage.getItem('crabigator-grouping') || 'project';

        // Project ordering mode when grouped ('recent' = most recently active first, 'alpha' = alphabetical)
        let projectOrderMode = localStorage.getItem('crabigator-project-order') || 'recent';

        // Track collapsed state for project groups
        const collapsedProjects = new Set(JSON.parse(localStorage.getItem('crabigator-collapsed-projects') || '[]'));

        // Track collapsed state for device groups
        const collapsedDevices = new Set(JSON.parse(localStorage.getItem('crabigator-collapsed-devices') || '[]'));

        // Single session focus mode (from ?session=xxx URL parameter)
        function readFocusedSessionId() {
            return new URLSearchParams(window.location.search).get('session');
        }

        let singleSessionId = readFocusedSessionId();

        function isFocusedMode() {
            return !!singleSessionId;
        }

        function sessionMatchesFocus(session, focusId = singleSessionId) {
            if (!session || !focusId) return false;
            return session.id === focusId || session.client_session_id === focusId;
        }

        function getMainGroupingMode() {
            return isFocusedMode() ? 'all' : groupingMode;
        }

        function resetVisibleSessionLock() {
            lockedVisibleSessionIds = null;
            visibleSessionIds = new Set();
            hiddenSessionCount = 0;
        }

        // Sidebar state
        let sidebarPinned = localStorage.getItem('crabigator-sidebar-pinned') === 'true'; // default false (popover mode)
        let sidebarPosition = localStorage.getItem('crabigator-sidebar-position') || 'left';
        let sidebarDensity = localStorage.getItem('crabigator-sidebar-density') || 'comfortable';
        let sidebarWidth = parseInt(localStorage.getItem('crabigator-sidebar-width') || '300', 10);
        let sessionClickAction = localStorage.getItem('crabigator-click-action') || 'scroll';
        let sidebarVisibleStats = JSON.parse(localStorage.getItem('crabigator-sidebar-stats') || JSON.stringify({
            sessionTime: true, thinkingTime: true, prompts: true,
            completions: true, tools: true, compactions: true
        }));

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
