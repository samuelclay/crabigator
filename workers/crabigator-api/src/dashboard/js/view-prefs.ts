// Dashboard JavaScript - what each view shows, and the stored choices behind it.
// No imports here: scripts/test-view-prefs.mjs runs these functions directly.
export const viewPrefsJs = `
        // The board's time bands in the terminal board's own colours: xterm
        // 51, 45, 39, 33, 27 and 25, then 245 for every age.
        const VIEW_BANDS = [
            { hours: 1, label: '1h', menu: 'Last hour', color: '#00ffff' },
            { hours: 3, label: '3h', menu: 'Last 3 hours', color: '#00d7ff' },
            { hours: 6, label: '6h', menu: 'Last 6 hours', color: '#00afff' },
            { hours: 9, label: '9h', menu: 'Last 9 hours', color: '#0087ff' },
            { hours: 12, label: '12h', menu: 'Last 12 hours', color: '#005fff' },
            { hours: 24, label: '24h', menu: 'Last 24 hours', color: '#005faf' },
            { hours: null, label: 'All', menu: 'All ages', color: '#8a8a8a' },
        ];

        // Which band a stored activity window names. Anything that is not
        // one of the six windows means every age.
        function viewBandIndex(maxAgeHours) {
            const index = VIEW_BANDS.findIndex(band => band.hours !== null && band.hours === maxAgeHours);
            return index === -1 ? VIEW_BANDS.length - 1 : index;
        }

        // The filters and PR board choices, in the terminal board's
        // [pr_board] keys. maxAgeHours null shows every age, and computer ''
        // shows every computer. 'detail' is the old recap switch: it only
        // seeds the PR board's row set the first time (see viewSets).
        const PRB_VIEW_DEFAULTS = { detail: 0, maxAgeHours: 24, liveOnly: false, view: 'sessions', computer: '' };
        let prBoardViewPrefs = (() => {
            let stored = {};
            try {
                stored = JSON.parse(localStorage.getItem('crabigatorPrBoardView') || '{}') || {};
                delete stored.lingerDays;
            } catch (e) { stored = {}; }
            const prefs = Object.assign({}, PRB_VIEW_DEFAULTS, stored);
            prefs.maxAgeHours = VIEW_BANDS[viewBandIndex(prefs.maxAgeHours)].hours;
            prefs.computer = typeof prefs.computer === 'string' ? prefs.computer : '';
            return prefs;
        })();
        function savePrBoardView() {
            try { localStorage.setItem('crabigatorPrBoardView', JSON.stringify(prBoardViewPrefs)); } catch (e) {}
        }

        // ── What a row shows ───────────────────────────────────────────
        // One chip per item, in this order. The List and the PR board each
        // keep their own set; the session screen's stats strip keeps a third.
        const ROW_ITEMS = [
            ['recap', 'Recap'],
            ['sessionTime', 'Session time'],
            ['thinkingTime', 'Thinking time'],
            ['promptRecency', 'Last prompt'],
            ['completionRecency', 'Last completion'],
            ['tools', 'Tools'],
            ['compactions', 'Compactions'],
            ['context', 'Context'],
            ['git', 'Git status'],
            ['changes', 'Changes'],
        ];
        // The dashboard's List could always show the two counts as well.
        const LIST_ROW_ITEMS = ROW_ITEMS.concat([
            ['prompts', 'Prompt count'],
            ['completions', 'Completion count'],
        ]);
        const SESSION_STATS_ITEMS = [
            ['sessionTime', 'Session time'],
            ['thinkingTime', 'Thinking time'],
            ['prompts', 'Prompts'],
            ['completions', 'Completions'],
            ['tools', 'Tools'],
            ['compactions', 'Compactions'],
            ['context', 'Context'],
            // The dashboard's strip could always show the two ages as well.
            ['promptRecency', 'Last prompt'],
            ['completionRecency', 'Last completion'],
        ];

        // A set that has never been saved starts from the old "Visible stats"
        // switches and the old recap switch, so each screen looks as it did.

        // Context is the exception, because its old switch started off. On a
        // List row and in the stats strip it starts on when that switch was
        // on, or when no other stat the set shows had been hidden: context
        // is not worth a line where a row shows little else. A stat that
        // never had a switch counts as shown.
        function contextStartsOn(old, items) {
            if (old.context) return true;
            return items.every(([key]) => key === 'context' || old[key] === undefined || !!old[key]);
        }
        function deriveListRow(oldStats) {
            const old = oldStats || {};
            return {
                // The List always drew the recap under a row's title, so it starts on.
                recap: true,
                sessionTime: !!old.sessionTime,
                thinkingTime: !!old.thinkingTime,
                promptRecency: !!old.promptRecency,
                completionRecency: !!old.completionRecency,
                tools: !!old.tools,
                compactions: !!old.compactions,
                context: contextStartsOn(old, LIST_ROW_ITEMS),
                git: false,
                changes: false,
                prompts: !!old.prompts,
                completions: !!old.completions,
            };
        }
        // PR board rows showed the two ages and no stats. Context starts off
        // there, so the board keeps its height.
        function deriveBoardRow(showRecaps) {
            const set = {};
            for (const [key] of ROW_ITEMS) set[key] = false;
            set.recap = !!showRecaps;
            set.promptRecency = true;
            set.completionRecency = true;
            return set;
        }
        function deriveSessionStats(oldStats) {
            const old = oldStats || {};
            const set = {};
            for (const [key] of SESSION_STATS_ITEMS) set[key] = !!old[key];
            // Where the old screen had no switch for a count, it starts on.
            if (old.prompts === undefined) set.prompts = true;
            if (old.completions === undefined) set.completions = true;
            set.context = contextStartsOn(old, SESSION_STATS_ITEMS);
            return set;
        }
        // A saved set wins outright: after the first change the old switches
        // are no longer consulted for it.
        function readViewSet(raw, items, derive) {
            let saved = null;
            try { saved = raw ? JSON.parse(raw) : null; } catch (e) { saved = null; }
            if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return derive();
            const set = {};
            for (const [key] of items) set[key] = saved[key] === true;
            return set;
        }

        const VIEW_SET_KEYS = {
            list: 'crabigator-list-row',
            board: 'crabigator-board-row',
            session: 'crabigator-session-stats',
        };
        function storedViewSet(which) {
            try { return localStorage.getItem(VIEW_SET_KEYS[which]); } catch (e) { return null; }
        }
        const viewSets = {
            list: readViewSet(storedViewSet('list'), LIST_ROW_ITEMS, () => deriveListRow(sidebarVisibleStats)),
            board: readViewSet(storedViewSet('board'), ROW_ITEMS, () => deriveBoardRow(prBoardViewPrefs.detail === 1)),
            session: readViewSet(storedViewSet('session'), SESSION_STATS_ITEMS, () => deriveSessionStats(sidebarVisibleStats)),
        };
        function toggleViewSetItem(which, key) {
            const set = viewSets[which];
            if (!set || !(key in set)) return;
            set[key] = !set[key];
            try { localStorage.setItem(VIEW_SET_KEYS[which], JSON.stringify(set)); } catch (e) {}
        }

        // ── Which sessions the List shows ──────────────────────────────
        // The filter bar's time window and computer, applied to the List.

        // When a List session last mattered, by the PR board's rule: now
        // while it works or waits on you, else its last prompt, else its start.
        function listSessionFreshness(sessionId, now) {
            const live = sessions.get(sessionId) || {};
            const listed = allSessions.find(session => session.id === sessionId) || {};
            const state = live.state || listed.state;
            if (state === 'thinking' || state === 'permission' || state === 'question') return now;
            const stats = Object.assign({}, listed.stats, live.stats);
            return stats.prompts_changed_at || listed.prompts_changed_at
                || getSessionStartedTime({ started_at: live.startedAt || listed.started_at });
        }
        // An open terminal always stays on the List, so nothing you are
        // reading can vanish. A session that is working, or waiting on a
        // question or a permission, never leaves for being old: its freshness
        // is "now". The computer filter still applies to it.
        function sessionInFilter(sessionId, now) {
            if (isFocusedMode() || expandedSessions.has(sessionId)) return true;
            const computer = prBoardViewPrefs.computer;
            if (computer) {
                const listed = allSessions.find(session => session.id === sessionId) || {};
                const name = (sessions.get(sessionId) || {}).deviceName || listed.device_name;
                if (prbDeviceLabel({ device_name: name }) !== computer) return false;
            }
            return prbBucketIndex(now - listSessionFreshness(sessionId, now)) <= viewBandIndex(prBoardViewPrefs.maxAgeHours);
        }
        function listSessionsInFilter() {
            const now = Date.now() / 1000;
            const shown = new Map();
            for (const [id, data] of sessions) {
                if (sessionInFilter(id, now)) shown.set(id, data);
            }
            return shown;
        }

        // ── Group by ───────────────────────────────────────────────────
        // One picker over two stored choices: group by computer (on or off)
        // and projects ('all' or 'project').
        function groupByChoice(byComputer, projectMode) {
            const byProject = projectMode === 'project';
            if (byComputer) return byProject ? 'both' : 'computer';
            return byProject ? 'project' : 'none';
        }
        function groupByPrefs(choice) {
            return {
                byComputer: choice === 'computer' || choice === 'both',
                projectMode: choice === 'project' || choice === 'both' ? 'project' : 'all',
            };
        }

        // ── Size and spacing ───────────────────────────────────────────
        // Row spacing keeps the session list's stored density values.
        const ROW_SPACINGS = [['compact', 'Tight'], ['comfortable', 'Normal'], ['relaxed', 'Relaxed']];
        // A name for each stored terminal height, so the number is not bare.
        const TERMINAL_HEIGHT_NAMES = {
            150: 'Shortest', 200: 'Shorter', 250: 'Short', 350: 'Medium', 450: 'Tall', 550: 'Taller', 700: 'Tallest',
        };
        function terminalHeightLabel(pixels) {
            const name = TERMINAL_HEIGHT_NAMES[pixels];
            return (name ? name + ' · ' : '') + pixels + ' px';
        }
`;
