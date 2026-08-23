// Dashboard JavaScript - cross-session PR board
// A web port of the CLI board (src/prs_board.rs + src/ui/pr_cells.rs): the
// same recency sections, repository groups, row anatomy, colors, ordering,
// view toggles, keyboard shortcuts, and quick look pane, rendered from the
// durable D1 record instead of /tmp mirrors. Search greps each live session's
// scrollback through the Worker, falling back to recaps for sessions that
// have ended and no longer hold one.
export const prBoardJs = `
        let prBoardVisible = false;
        let prBoardTimer = null;
        let prBoardThrobTimer = null;
        let prBoardEntries = [];
        let prBoardSessions = [];
        let prBoardSlack = new Map();
        let prBoardLoaded = false;
        let prBoardQuery = '';
        let prBoardExpanded = false;
        let prBoardRendered = [];

        // Scrollback search results, keyed by session, for the query that
        // produced them. The desktop board greps each session's local
        // scrollback.log; the Worker greps the same stream for the web.
        let prBoardHits = new Map();
        let prBoardHitsQuery = '';
        let prBoardSearchTimer = null;

        // Row selection and the ⏎-toggled quick look pane. prBoardPeekScroll
        // anchors the pane into the session's transcript; null mirrors the
        // live screen.
        let prBoardSelected = null;
        let prBoardPeekOpen = false;
        let prBoardPeekScroll = null;
        let prBoardPeekSessionId = null;
        let prBoardPeekSource = null;
        let prBoardPeekHeartbeat = null;
        let prBoardPeekScreen = '';
        let prBoardPeekLines = [];

        // View preferences, mirroring the CLI's [pr_board] config keys.
        const PRB_VIEW_DEFAULTS = { detail: 0, maxAgeHours: null, lingerDays: 1, liveOnly: false };
        let prBoardViewPrefs = (() => {
            try {
                return Object.assign({}, PRB_VIEW_DEFAULTS,
                    JSON.parse(localStorage.getItem('crabigatorPrBoardView') || '{}'));
            } catch (e) { return Object.assign({}, PRB_VIEW_DEFAULTS); }
        })();
        function savePrBoardView() {
            try { localStorage.setItem('crabigatorPrBoardView', JSON.stringify(prBoardViewPrefs)); } catch (e) {}
        }

        // xterm-256 palette hexes shared with the CLI renderer.
        const PRB_C = {
            purple: '#af87ff', gray: '#8a8a8a', darkGray: '#585858',
            green: '#5fff5f', lightGreen: '#87d787', red: '#ff5f5f', yellow: '#ffd700',
            orange: '#d7af5f', cyan: '#00d7ff',
        };
        // Activity recency: one cyan-blue hue at falling intensity, gray after a day.
        const PRB_BUCKETS = [
            { max: 3600, label: 'Last hour', bg: '#00ffff', text: '#000000', hours: 1, ageLabel: '1h' },
            { max: 10800, label: '1–3 hours', bg: '#00d7ff', text: '#000000', hours: 3, ageLabel: '3h' },
            { max: 21600, label: '3–6 hours', bg: '#00afff', text: '#000000', hours: 6, ageLabel: '6h' },
            { max: 32400, label: '6–9 hours', bg: '#0087ff', text: '#000000', hours: 9, ageLabel: '9h' },
            { max: 43200, label: '9–12 hours', bg: '#005fff', text: '#ffffff', hours: 12, ageLabel: '12h' },
            { max: 86401, label: '12–24 hours', bg: '#005faf', text: '#ffffff', hours: 24, ageLabel: '24h' },
            { max: Infinity, label: 'Older', bg: '#585858', text: '#ffffff', hours: null, ageLabel: 'all' },
        ];
        const PRB_THROBBER = ['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏'];
        // Search needs this many characters, like the CLI: one or two letters
        // match nearly every line and would light up the whole board.
        const PRB_QUERY_MIN = 3;
        // The expanded preview shows this many of the most recent matches.
        const PRB_PREVIEW_MATCHES = 3;

        function togglePrBoard() {
            prBoardVisible = !prBoardVisible;
            const board = document.getElementById('pr-board-view');
            const sessionsEl = document.getElementById('sessions');
            const btn = document.getElementById('pr-board-btn');
            if (board) board.hidden = !prBoardVisible;
            if (sessionsEl) sessionsEl.style.display = prBoardVisible ? 'none' : '';
            if (btn) btn.classList.toggle('active', prBoardVisible);
            if (prBoardVisible) {
                if (board && !board.dataset.shell) {
                    board.dataset.shell = '1';
                    board.innerHTML = prbShellHtml();
                    prbBindSearch();
                }
                if (!prOverridesLoadStarted) {
                    prOverridesLoadStarted = true;
                    loadPrOverrides();
                }
                renderPrBoard();
                loadPrBoard();
                prBoardTimer = setInterval(loadPrBoard, 15000);
                prBoardThrobTimer = setInterval(prbTickThrobber, 100);
            } else {
                prbClosePeek();
                if (prBoardTimer) { clearInterval(prBoardTimer); prBoardTimer = null; }
                if (prBoardThrobTimer) { clearInterval(prBoardThrobTimer); prBoardThrobTimer = null; }
            }
        }

        function prbShellHtml() {
            return '<div class="prb-head">'
                + '<span class="prb-hdr">⑆ Crabigator PR board</span>'
                + '<span class="prb-counts" id="prb-counts"></span>'
                + '<span class="prb-ctl" id="prb-ctl-live" onclick="prBoardToggleLive()" title="Only sessions running right now, or the full durable history (s)"></span>'
                + '<span class="prb-ctl" id="prb-ctl-days" title="How long finished primary PRs linger (+/-)"></span>'
                + '<span class="prb-ctl" id="prb-ctl-recap" onclick="prBoardToggleRecap()" title="Show per-session recaps (r)"></span>'
                + '<span class="prb-ctl" id="prb-ctl-age" onclick="prBoardCycleAge()" title="Hide rows idle longer than this (a)"></span>'
                + '<span class="prb-keys"><u>↑↓</u> select · <u>⏎</u> peek · <u>/</u> search · <u>+/-</u> days · <u>q</u> quit</span>'
                + '<span class="prb-searchwrap"><input id="prb-search" placeholder="/ search" spellcheck="false" autocomplete="off"><span id="prb-matches"></span></span>'
                + '</div><div class="prb-body" id="prb-body"></div>'
                + '<div class="prb-peek" id="prb-peek" hidden>'
                + '<div class="prb-peek-top"><span class="prb-peek-title" id="prb-peek-title"></span>'
                + '<span class="prb-peek-keys" id="prb-peek-keys"></span></div>'
                + '<div class="prb-peek-body" id="prb-peek-body"></div></div>';
        }

        function prbBindSearch() {
            const input = document.getElementById('prb-search');
            if (!input) return;
            input.addEventListener('input', () => {
                prBoardQuery = input.value;
                prbScheduleSearch();
                renderPrBoardBody();
            });
            input.addEventListener('keydown', e => {
                if (e.key === 'Escape') {
                    prbClearSearch();
                    input.blur();
                } else if (e.key === 'Tab') {
                    // Tab flips the previews between one snippet and the last
                    // few matches with context, like the CLI's Tab context.
                    e.preventDefault();
                    prBoardExpanded = !prBoardExpanded;
                    renderPrBoardBody();
                }
                e.stopPropagation();
            });
        }

        function prbClearSearch() {
            prBoardQuery = '';
            prBoardExpanded = false;
            prBoardHits = new Map();
            prBoardHitsQuery = '';
            const el = document.getElementById('prb-search');
            if (el) el.value = '';
            renderPrBoardBody();
        }

        // The CLI's keyboard shortcuts, active while the board is open.
        document.addEventListener('keydown', e => {
            if (!prBoardVisible || e.metaKey || e.ctrlKey || e.altKey) return;
            const tag = e.target && e.target.tagName;
            const typing = tag === 'INPUT' || tag === 'TEXTAREA'
                || (e.target && e.target.isContentEditable);
            if (typing) return;
            if (prBoardPeekOpen && prbPeekKey(e)) return;
            if (e.key === 'ArrowUp' || e.key === 'k') {
                e.preventDefault();
                prbStepSelection(-1);
            } else if (e.key === 'ArrowDown' || e.key === 'j') {
                e.preventDefault();
                prbStepSelection(1);
            } else if (e.key === 'Enter') {
                e.preventDefault();
                prbTogglePeek();
            } else if (e.key === '/') {
                e.preventDefault();
                const el = document.getElementById('prb-search');
                if (el) el.focus();
            } else if (e.key === 'r') prBoardToggleRecap();
            else if (e.key === 'a') prBoardCycleAge();
            else if (e.key === 's') prBoardToggleLive();
            else if (e.key === '+' || e.key === '=') prBoardDays(1);
            else if (e.key === '-' || e.key === '_') prBoardDays(-1);
            else if (e.key === 'Escape' && prBoardQuery) prbClearSearch();
            else if (e.key === 'q' || e.key === 'Escape') togglePrBoard();
        });

        function prBoardToggleLive() {
            prBoardViewPrefs.liveOnly = !prBoardViewPrefs.liveOnly;
            savePrBoardView();
            renderPrBoard();
        }
        function prBoardToggleRecap() {
            prBoardViewPrefs.detail = prBoardViewPrefs.detail === 1 ? 0 : 1;
            savePrBoardView();
            renderPrBoard();
        }
        function prBoardCycleAge() {
            const cycle = [null, 1, 3, 6, 9, 12, 24];
            prBoardViewPrefs.maxAgeHours = cycle[(cycle.indexOf(prBoardViewPrefs.maxAgeHours) + 1) % cycle.length];
            savePrBoardView();
            renderPrBoard();
        }
        function prBoardDays(delta) {
            // The linger window filters server-side, so a change refetches.
            prBoardViewPrefs.lingerDays = Math.min(90, Math.max(0, (prBoardViewPrefs.lingerDays || 0) + delta));
            savePrBoardView();
            updatePrBoardControls();
            loadPrBoard();
        }

        async function loadPrBoard() {
            const body = document.getElementById('prb-body');
            try {
                const res = await fetch('/api/prs/board?days=' + (prBoardViewPrefs.lingerDays || 0),
                    { headers: getAuthHeaders() });
                if (!res.ok) {
                    if (body) body.innerHTML = '<div class="prb-empty">Could not load the PR board ('
                        + res.status + '). Pair this browser from a session first.</div>';
                    return;
                }
                const data = await res.json();
                prBoardEntries = data.prs || [];
                prBoardSessions = data.sessions || [];
                prBoardSlack = new Map((data.slack || []).map(t => [t.url, t]));
                prBoardLoaded = true;
                renderPrBoardBody();
            } catch (e) {
                if (body) body.innerHTML = '<div class="prb-empty">Could not load the PR board.</div>';
            }
        }

        function updatePrBoardControls() {
            const set = (id, active, html) => {
                const el = document.getElementById(id);
                if (!el) return;
                el.classList.toggle('active', active);
                el.innerHTML = html;
            };
            set('prb-ctl-live', prBoardViewPrefs.liveOnly,
                '<u>s</u> ' + (prBoardViewPrefs.liveOnly ? 'live' : 'all sessions'));
            const days = prBoardViewPrefs.lingerDays;
            set('prb-ctl-days', days !== 1,
                '<span class="prb-step" onclick="prBoardDays(-1);event.stopPropagation()">−</span> '
                + (days === 0 ? 'open only' : 'primary done ≤ ' + days + 'd')
                + ' <span class="prb-step" onclick="prBoardDays(1);event.stopPropagation()">+</span>');
            set('prb-ctl-recap', prBoardViewPrefs.detail === 1,
                '<u>r</u> ' + (prBoardViewPrefs.detail === 1 ? 'recap' : 'compact'));
            const bucket = prBoardViewPrefs.maxAgeHours === null
                ? null : PRB_BUCKETS.find(b => b.hours === prBoardViewPrefs.maxAgeHours);
            set('prb-ctl-age', !!bucket, '<u>a</u> ' + (bucket ? 'age ≤ ' + bucket.ageLabel : 'all ages'));
        }

        function renderPrBoard() {
            updatePrBoardControls();
            renderPrBoardBody();
        }

        // ── Formatting shared with the CLI ─────────────────────────────

        // Coarse ages so the board doesn't look like it's thrashing.
        function prbAge(secs) {
            if (secs <= 89) return '1m';
            if (secs < 3600) return Math.ceil(secs / 60) + 'm';
            if (secs < 86400) return Math.floor(secs / 3600) + 'h';
            return Math.floor(secs / 86400) + 'd';
        }
        function prbBucketIndex(ageSecs) {
            for (let i = 0; i < PRB_BUCKETS.length; i++) {
                if (ageSecs < PRB_BUCKETS[i].max) return i;
            }
            return PRB_BUCKETS.length - 1;
        }
        function prbRecencyColor(ageSecs) {
            return PRB_BUCKETS[prbBucketIndex(ageSecs)].bg;
        }
        function prbAgePart(icon, ts, now) {
            if (!ts) return '<span style="color:' + PRB_C.darkGray + '">' + icon + ' —</span>';
            const age = Math.max(0, now - ts);
            return '<span style="color:' + prbRecencyColor(age) + '">' + icon + ' ' + prbAge(age) + '</span>';
        }
        // Cut text to a character budget, ellipsizing when it doesn't fit.
        function prbTruncate(text, max) {
            if (max <= 0) return '';
            return text.length <= max ? text : text.slice(0, Math.max(1, max - 1)) + '…';
        }

        function prbThrobFrame() { return Math.floor(Date.now() / 100) % PRB_THROBBER.length; }
        function prbTickThrobber() {
            const els = document.querySelectorAll('#pr-board-view .prb-throb');
            if (!els.length) return;
            const ch = PRB_THROBBER[prbThrobFrame()];
            els.forEach(el => { el.textContent = ch; });
        }

        function prbSessionState(s) {
            const st = String(s.state || '').toLowerCase();
            return ['ready', 'thinking', 'permission', 'question', 'complete', 'interrupted'].includes(st)
                ? st : (s.active ? 'ready' : 'complete');
        }
        // Pick the state that needs the user's attention when a PR represents
        // more than one session.
        function prbAggregateState(sessions) {
            const rank = { permission: 5, question: 4, thinking: 3, interrupted: 2, ready: 1, complete: 0 };
            let best = null;
            for (const s of sessions) {
                const st = prbSessionState(s);
                if (best === null || rank[st] > rank[best]) best = st;
            }
            return best;
        }
        function prbStateIcon(state) {
            if (state === null) return '';
            if (state === 'thinking') {
                return '<span class="prb-throb" style="color:' + PRB_C.green + '">'
                    + PRB_THROBBER[prbThrobFrame()] + '</span>';
            }
            const map = {
                ready: ['○', PRB_C.gray], permission: ['!', PRB_C.yellow],
                question: ['?', PRB_C.orange], complete: ['✓', PRB_C.purple],
                interrupted: ['⊘', PRB_C.red],
            };
            const m = map[state] || ['○', PRB_C.gray];
            return '<span style="color:' + m[1] + '">' + m[0] + '</span>';
        }
        // "{state} ⟩ prompt-age ⋖ completion-age", each age keeping its own
        // recency color because the two events can be hours apart.
        function prbActivityCell(sessions, now) {
            const prompted = Math.max(0, ...sessions.map(s => s.prompts_changed_at || 0));
            const completed = Math.max(0, ...sessions.map(s => s.completions_changed_at || 0));
            return prbStateIcon(prbAggregateState(sessions)) + ' '
                + prbAgePart('⟩', prompted, now) + ' ' + prbAgePart('⋖', completed, now);
        }
        function prbSessionFreshness(s) {
            return Math.max(s.completions_changed_at || 0, s.prompts_changed_at || 0);
        }
        function prbActivityTime(sessions) {
            return Math.max(0, ...sessions.map(s => prbSessionFreshness(s)));
        }

        // Provider markers: ⟁ Codex, ᛝ Claude, both when sessions mix.
        function prbStripMarker(title) {
            return String(title || '').replace(/^[⟁ᛝ]+\\s+/, '').trim();
        }
        function prbMarker(platform) {
            return platform === 'codex' ? '⟁ ' : 'ᛝ ';
        }
        function prbProviderMarkers(sessions) {
            const codex = sessions.some(s => s.platform === 'codex');
            const claude = sessions.some(s => s.platform !== 'codex');
            if (codex && claude) return '⟁ᛝ ';
            if (codex) return '⟁ ';
            if (claude) return 'ᛝ ';
            return '';
        }
        // The newest title among a PR's sessions, by when it was set — the
        // same choice the CLI makes, so a stale session can't win.
        function prbLatestSessionTitle(sessions) {
            let best = null;
            for (const s of sessions) {
                const plain = prbStripMarker(s.title);
                if (!plain || plain === s.dir_name) continue;
                const setAt = s.title_set_at || 0;
                if (!best || setAt >= best.setAt) best = { setAt, text: prbMarker(s.platform) + plain };
            }
            return best ? best.text : '';
        }
        // The official PR title leads; the generated session title moves to
        // the metadata row when it says something different.
        function prbPrTitles(pr, sessions) {
            const gen = prbLatestSessionTitle(sessions);
            const prTitle = String(pr.title || '').trim();
            if (!prTitle) {
                return {
                    title: gen || (prbProviderMarkers(sessions) + (pr.repo || 'PR')),
                    generated: '',
                };
            }
            return { title: prTitle, generated: gen && prbStripMarker(gen) !== prTitle ? gen : '' };
        }

        // A row stands for one session on a PR row, or the session itself on
        // a workspace row.
        function prbRowSessions(item) {
            return item.kind === 'pr' ? item.sessions : [item.session];
        }
        function prbActivityHtml(item, idx, now) {
            const cell = prbActivityCell(prbRowSessions(item), now);
            if (!prbPeekSession(item)) return '<span class="prb-activity">' + cell + '</span>';
            return '<span class="prb-activity prb-peek-open" data-act="peek" data-idx="' + idx
                + '" title="Quick look at this session (⏎)">' + cell + '</span>';
        }

        function prbStateLabel(pr) {
            if (pr.state === 'MERGED') return { label: 'merged', color: PRB_C.purple };
            if (pr.state === 'CLOSED') return { label: 'closed', color: PRB_C.red };
            if (pr.is_draft) return { label: 'draft', color: PRB_C.gray };
            if (pr.state === 'OPEN') return { label: 'open', color: PRB_C.lightGreen };
            // No state = never enriched; the desktop retries automatically.
            if (pr.fetch_error) return { label: 'error', color: PRB_C.red, title: pr.fetch_error };
            if (!pr.refreshed_at) return { label: 'fetch…', color: PRB_C.gray };
            return null;
        }

        // Sort rank for the GitHub state that needs the most attention, and
        // the CLI's whole-board ordering built on it: attention first, then
        // primaries, then recency of discussion.
        function prbAttentionRank(pr) {
            if (!pr.state) return 2;
            if (pr.state === 'MERGED') return 6;
            if (pr.state !== 'OPEN') return 7;
            if (pr.mergeable === 'CONFLICTING' || (pr.checks_failed || 0) > 0) return 0;
            if (pr.review_decision === 'CHANGES_REQUESTED') return 1;
            if (pr.is_draft) return 2;
            if ((pr.checks_pending || 0) > 0) return 3;
            if (pr.review_decision !== 'APPROVED') return 4;
            return 5;
        }
        function prbSortEntries(entries) {
            entries.sort((a, b) => prbAttentionRank(a.entry.pr) - prbAttentionRank(b.entry.pr)
                || (b.primary ? 1 : 0) - (a.primary ? 1 : 0)
                || (b.entry.pr.last_mentioned_at || 0) - (a.entry.pr.last_mentioned_at || 0));
        }

        function prbLink(url, inner) {
            if (!url) return inner;
            return '<a class="prb-a" href="' + escapeHtml(url)
                + '" target="_blank" rel="noopener noreferrer">' + inner + '</a>';
        }

        // GitHub status cells: state, CI, unresolved threads, merge, dismiss.
        function prbStatusCells(pr, idx) {
            const cells = [];
            const st = prbStateLabel(pr);
            if (st) {
                cells.push('<span style="color:' + st.color + '"'
                    + (st.title ? ' title="' + escapeHtml(st.title) + '"' : '') + '>' + st.label + '</span>');
            }
            if (pr.checks_total) {
                let label = '✓ CI';
                let color = PRB_C.green;
                if (pr.checks_failed > 0) {
                    label = '✗' + pr.checks_failed + ' CI';
                    color = PRB_C.red;
                } else if (pr.checks_pending > 0) {
                    label = '●' + pr.checks_pending + ' CI';
                    color = PRB_C.yellow;
                }
                cells.push(prbLink(pr.ci_url, '<span style="color:' + color + '">' + label + '</span>'));
            }
            if (pr.unresolved_comments) {
                cells.push(prbLink(pr.comments_url,
                    '<span style="color:' + PRB_C.orange + '">💬' + pr.unresolved_comments + '</span>'));
            }
            if (pr.mergeable === 'CONFLICTING') {
                cells.push('<span style="color:' + PRB_C.red + '">conflicts</span>');
            } else if (pr.mergeable === 'MERGEABLE') {
                const behind = pr.merge_state_status === 'BEHIND';
                cells.push('<span style="color:' + (behind ? PRB_C.yellow : PRB_C.green) + '">'
                    + (behind ? 'behind' : 'clean') + '</span>');
            }
            cells.push('<span class="prb-x" data-act="dismiss" data-idx="' + idx
                + '" title="Dismiss this PR everywhere">✕</span>');
            return cells.join('');
        }

        // The diff and file count both point at the PR's Files-changed tab.
        function prbDiffText(additions, deletions) {
            if (!additions && !deletions) return '';
            return '<span class="prb-add">+' + (additions || 0)
                + '</span> <span class="prb-del">-' + (deletions || 0) + '</span>';
        }
        function prbFilesText(count) {
            return count ? count + ' file' + (count === 1 ? '' : 's') : '';
        }
        function prbDiffFiles(pr) {
            const parts = [];
            const diff = prbDiffText(pr.additions, pr.deletions);
            if (diff) parts.push(diff);
            const files = prbFilesText(pr.changed_files);
            if (files) parts.push(files);
            if (!parts.length) return '';
            const inner = parts.join('&nbsp; ');
            return pr.url
                ? '<a class="prb-difffiles" href="' + escapeHtml(pr.url + '/files')
                    + '" target="_blank" rel="noopener noreferrer">' + inner + '</a>'
                : '<span class="prb-difffiles">' + inner + '</span>';
        }

        // ── Recap detail (the CLI's r toggle) ──────────────────────────

        function prbLatestRecap(sessions) {
            let best = null;
            for (const s of sessions) {
                const r = s.recap;
                if (r && r.headline && (!best || (r.generated_at || 0) > (best.generated_at || 0))) best = r;
            }
            return best;
        }
        function prbLabeled(label, value) {
            const v = String(value || '').trim();
            return v.toLowerCase().startsWith(label.toLowerCase()) ? v : label + ' ' + v;
        }
        function prbDelta(recap) {
            const a = recap.additions || 0, d = recap.deletions || 0;
            if (!a && !d) return '';
            if (a >= 0 && d >= 0) {
                return ' · Δ <span class="prb-add">+' + a + '</span> <span class="prb-del">-' + d + '</span>';
            }
            const net = a - d;
            return ' · Δ net ' + (net >= 0 ? '+' : '') + net;
        }
        function prbRecapRows(recap, now) {
            const ageSecs = recap.generated_at
                ? Math.max(0, now - Math.floor(recap.generated_at / 1000)) : null;
            const age = ageSecs === null ? ''
                : '<span style="color:' + prbRecencyColor(ageSecs) + '">' + prbAge(ageSecs) + '</span>';
            const rows = [{
                icon: '↪', iconColor: PRB_C.darkGray,
                html: escapeHtml(recap.headline) + prbDelta(recap), right: age,
            }];
            for (const b of recap.bullets || []) {
                rows.push({ icon: '•', iconColor: PRB_C.darkGray, html: escapeHtml(b) });
            }
            for (const n of recap.next_prompt_notes || []) {
                rows.push({ icon: '', iconColor: '', html: escapeHtml(prbLabeled('Next:', n)) });
            }
            for (const a of recap.artifacts || []) {
                rows.push({ icon: '', iconColor: '', html: escapeHtml(prbLabeled('Artifact:', a)) });
            }
            return rows;
        }
        function prbDlRowHtml(row, right) {
            const icon = row && row.icon
                ? '<span class="prb-dl-icon" style="color:' + row.iconColor + '">' + row.icon + '</span>' : '';
            const left = row
                ? icon + '<span class="prb-dl-text"'
                    + (row.title ? ' title="' + escapeHtml(row.title) + '"' : '') + '>' + row.html + '</span>'
                : '';
            return '<div class="prb-dl"><span class="prb-dl-left">' + left
                + '</span><span class="prb-dl-right">' + (right || '') + '</span></div>';
        }
        // Slack links pair with recap rows on the right, like the CLI's detail
        // column. Sessions stream the channel name and poster they learned
        // locally, so the label reads "#channel · Author"; without that the
        // channel ID in the permalink stands in.
        function prbSlackPostedAt(url) {
            const m = /\\/p(\\d{10})/.exec(url);
            return m ? Number(m[1]) : 0;
        }
        function prbSlackLabel(url) {
            const thread = prBoardSlack.get(url);
            const idMatch = /\\/archives\\/([A-Z0-9]+)/.exec(url);
            const channel = (thread && thread.channel) || (idMatch ? idMatch[1] : '');
            const label = channel ? '#' + String(channel).replace(/^#/, '') : '#thread';
            const author = thread && thread.author;
            return author ? label + ' · ' + author : label;
        }
        function prbSlackLinks(pr) {
            const origin = pr.slack_origin_url || '';
            const urls = [];
            if (origin) urls.push(origin);
            for (const u of pr.slack_comment_urls || []) {
                if (!urls.includes(u)) urls.push(u);
            }
            // Origin first, then oldest thread to newest, like the CLI.
            urls.sort((a, b) => (a === origin ? 0 : 1) - (b === origin ? 0 : 1)
                || prbSlackPostedAt(a) - prbSlackPostedAt(b));
            return urls.map(u => '<a class="prb-slack" href="' + escapeHtml(u)
                + '" target="_blank" rel="noopener noreferrer">' + escapeHtml(prbSlackLabel(u)) + '</a>');
        }
        function prbPrDetailHtml(pr, sessions, now) {
            const rows = [];
            if (pr.ai_note && pr.state === 'OPEN') {
                rows.push({
                    icon: '✦', iconColor: PRB_C.yellow, html: escapeHtml(pr.ai_note),
                    title: pr.ai_confidence ? pr.ai_confidence + ' confidence it is done' : '',
                });
            }
            const recap = prbLatestRecap(sessions);
            if (recap) rows.push(...prbRecapRows(recap, now));
            const slack = prbSlackLinks(pr);
            let html = '';
            for (let i = 0; i < Math.max(rows.length, slack.length); i++) {
                const row = rows[i];
                html += prbDlRowHtml(row, ((row && row.right) || '') + (slack[i] || ''));
            }
            return html;
        }

        // ── Search (scrollback, with recaps for ended sessions) ────────

        function prbPrMatches(entry, q) {
            const pr = entry.pr;
            return String(pr.number).includes(q)
                || String(pr.repo || '').toLowerCase().includes(q)
                || String(pr.owner || '').toLowerCase().includes(q)
                || String(pr.title || '').toLowerCase().includes(q)
                || String(pr.branch || '').toLowerCase().includes(q);
        }
        function prbWorkspaceMatches(s, q) {
            return String(s.repo_owner || '').toLowerCase().includes(q)
                || String(s.repo_name || '').toLowerCase().includes(q)
                || String(s.branch || '').toLowerCase().includes(q)
                || String(s.title || '').toLowerCase().includes(q);
        }
        function prbHighlight(text, q) {
            const lower = text.toLowerCase();
            let out = '', i = 0;
            while (true) {
                const idx = lower.indexOf(q, i);
                if (idx === -1) { out += escapeHtml(text.slice(i)); break; }
                out += escapeHtml(text.slice(i, idx))
                    + '<mark class="prb-mark">' + escapeHtml(text.slice(idx, idx + q.length)) + '</mark>';
                i = idx + q.length;
            }
            return out;
        }
        function prbRecapText(recap) {
            if (!recap || !recap.headline) return [];
            const lines = [recap.headline, ...(recap.bullets || [])];
            for (const n of recap.next_prompt_notes || []) lines.push(prbLabeled('Next:', n));
            for (const a of recap.artifacts || []) lines.push(prbLabeled('Artifact:', a));
            return lines;
        }
        // Trim a matched line so the first occurrence stays visible, dropping
        // the front behind a "…" when the match sits far to the right.
        function prbWindowAroundMatch(line, q, max) {
            const trimmed = line.trim();
            const idx = trimmed.toLowerCase().indexOf(q);
            if (idx === -1) return prbTruncate(trimmed, max);
            const lead = Math.min(15, Math.floor(max / 3));
            if (idx + lead <= max) return prbTruncate(trimmed, max);
            return prbTruncate('…' + trimmed.slice(idx - lead), max);
        }
        // Characters that fit across the board, for the snippet budget.
        function prbCharBudget() {
            const body = document.getElementById('prb-body');
            const width = (body && body.clientWidth) || 900;
            return Math.max(20, Math.floor(width / 6.6));
        }
        // Ask the Worker to grep every live session's scrollback, debounced so
        // typing doesn't fan out a request per keystroke.
        function prbScheduleSearch() {
            const q = prBoardQuery.trim();
            if (prBoardSearchTimer) clearTimeout(prBoardSearchTimer);
            if (q.length < PRB_QUERY_MIN) {
                prBoardHits = new Map();
                prBoardHitsQuery = '';
                return;
            }
            prBoardSearchTimer = setTimeout(() => prbRunSearch(q), 200);
        }
        async function prbRunSearch(q) {
            try {
                const res = await fetch('/api/prs/search?q=' + encodeURIComponent(q),
                    { headers: getAuthHeaders() });
                if (!res.ok) return;
                const data = await res.json();
                if (prBoardQuery.trim() !== q) return;
                prBoardHits = new Map((data.results || []).map(hit => [hit.session_id, hit]));
                prBoardHitsQuery = q;
                renderPrBoardBody();
            } catch (e) {
                // A failed search just leaves the metadata filter in place.
            }
        }
        function prbPreviewHtml(dirName, total, rows, collapsed, q) {
            const count = total + ' match' + (total === 1 ? '' : 'es');
            const head = '⌕ <span class="prb-pv-dir">' + escapeHtml(dirName || '') + '</span> · ';
            if (!prBoardExpanded) {
                const budget = Math.max(10,
                    prbCharBudget() - (8 + String(dirName || '').length + count.length + 3));
                const snippet = prbWindowAroundMatch(collapsed, q, budget);
                return ['<div class="prb-pv">' + head + '<span class="prb-pv-text">'
                    + prbHighlight(snippet, q) + '</span> · ' + count + '</div>'];
            }
            const out = ['<div class="prb-pv">' + head + count + '</div>'];
            for (const row of rows) {
                if (row.gap_before) out.push('<div class="prb-pv prb-pv-ctx prb-pv-gap">⋯</div>');
                const text = prbTruncate(row.text.replace(/\\s+$/, ''), prbCharBudget());
                out.push('<div class="prb-pv prb-pv-ctx">│ <span class="'
                    + (row.is_match ? 'prb-pv-text' : 'prb-pv-dim') + '">'
                    + (row.is_match ? prbHighlight(text, q) : escapeHtml(text)) + '</span></div>');
            }
            return out;
        }
        // Inline excerpts confirming why a search hit. Live sessions match on
        // their scrollback, exactly like the CLI; a session that has ended
        // keeps no scrollback in the cloud, so its recap is what is left.
        function prbPreviewLines(sessions, q) {
            if (q.length < PRB_QUERY_MIN) return [];
            const out = [];
            const seen = new Set();
            for (const s of sessions) {
                const key = s.session_id || s.dir_name;
                if (seen.has(key)) continue;
                seen.add(key);
                const hit = prBoardHitsQuery === q ? prBoardHits.get(s.session_id) : null;
                if (hit && hit.rows && hit.rows.length) {
                    out.push(...prbPreviewHtml(s.dir_name, hit.total, hit.rows, hit.collapsed, q));
                    continue;
                }
                const lines = prbRecapText(s.recap).filter(line => line.toLowerCase().includes(q));
                if (!lines.length) continue;
                const rows = lines.slice(-PRB_PREVIEW_MATCHES)
                    .map(text => ({ text, is_match: true, gap_before: false }));
                out.push(...prbPreviewHtml(
                    s.dir_name, lines.length, rows, lines[lines.length - 1], q));
            }
            return out;
        }

        // ── Rows ───────────────────────────────────────────────────────

        function prbRepoMatches(s, entry) {
            return !!entry.repo
                && String(s.repo_owner || '').toLowerCase() === String(entry.owner || '').toLowerCase()
                && String(s.repo_name || '').toLowerCase() === String(entry.repo || '').toLowerCase();
        }

        function prbPrRowHtml(item, idx, now) {
            const pr = item.entry.pr;
            const sessions = item.sessions;
            const titles = prbPrTitles(pr, sessions);
            const star = '<span class="prb-star ' + (item.primary ? 'primary' : 'secondary')
                + '" data-act="flip" data-idx="' + idx + '" title="'
                + (item.primary ? 'Primary — click to make secondary' : 'Secondary — click to make primary')
                + '">' + (item.primary ? '★' : '☆') + '</span>';
            const ident = '<a class="prb-ident" href="' + escapeHtml(pr.url || '')
                + '" target="_blank" rel="noopener noreferrer">'
                + escapeHtml(pr.number + ': ' + titles.title) + '</a>';
            let html = '<div class="prb-row' + (item.primary ? '' : ' prb-secondary')
                + (item.stale ? ' prb-stale' : '')
                + (item.key === prBoardSelected ? ' prb-sel' : '')
                + '" data-key="' + escapeHtml(item.key) + '">';
            html += '<div class="prb-l1"><span class="prb-l1-left">' + star + ident + '</span>'
                + prbActivityHtml(item, idx, now)
                + '<span class="prb-status">' + prbStatusCells(pr, idx) + '</span></div>';

            const gen = titles.generated
                ? '<span class="prb-gen">' + escapeHtml(titles.generated) + '</span>' : '';
            const branch = pr.branch
                ? '<span class="prb-branch">⎇ ' + escapeHtml(pr.branch) + '</span>' : '';
            const meta = prbDiffFiles(pr);
            if (gen || branch || meta) {
                html += '<div class="prb-l2"><span class="prb-l2-left">' + gen + branch + '</span>'
                    + '<span class="prb-l2-right">' + meta + '</span></div>';
            }
            if (prBoardViewPrefs.detail === 1) html += prbPrDetailHtml(pr, sessions, now);
            html += item.previews.join('') + '</div>';
            return html;
        }

        function prbSessionRowHtml(item, idx, now) {
            const s = item.session;
            const plain = prbStripMarker(s.title) || s.dir_name || 'session';
            const title = prbMarker(s.platform) + plain;
            // Session rows carry the same diff and file-count columns as PR
            // rows, filled from the session's uncommitted worktree changes.
            const diff = prbDiffText(s.additions, s.deletions);
            const files = prbFilesText(s.uncommitted);
            let html = '<div class="prb-row' + (item.key === prBoardSelected ? ' prb-sel' : '')
                + '" data-key="' + escapeHtml(item.key) + '">';
            html += '<div class="prb-l1"><span class="prb-l1-left">'
                + '<span class="prb-diamond">◇</span>'
                + '<span class="prb-ident">' + escapeHtml(title) + '</span>'
                + (diff ? '<span class="prb-wsdiff">' + diff + '</span>' : '')
                + (files ? '<span class="prb-wsfiles">' + escapeHtml(files) + '</span>' : '')
                + '<span class="prb-branch">⎇ ' + escapeHtml(s.branch || '(no branch)') + '</span></span>'
                + prbActivityHtml(item, idx, now)
                + '<span class="prb-status"></span></div>';
            if (prBoardViewPrefs.detail === 1 && s.recap && s.recap.headline) {
                for (const row of prbRecapRows(s.recap, now)) {
                    html += prbDlRowHtml(row, row.right);
                }
            }
            html += item.previews.join('') + '</div>';
            return html;
        }

        // ── Selection and the quick look pane ──────────────────────────

        // A row can be peeked when its session is still running: only a live
        // session has a screen to mirror, matching the CLI's local-mirror rule.
        function prbPeekSession(item) {
            return prbRowSessions(item).find(s => s && s.active && s.session_id) || null;
        }
        function prbSelectable() {
            return prBoardRendered
                .map((item, index) => (prbPeekSession(item) ? index : -1))
                .filter(index => index >= 0);
        }
        function prbSelectedIndex(selectable) {
            return selectable.findIndex(index => prBoardRendered[index].key === prBoardSelected);
        }
        // The selectable row one step away, clamped at the ends; entering the
        // list from nowhere starts at the nearer edge.
        function prbStepSelection(step) {
            const selectable = prbSelectable();
            if (!selectable.length) return;
            const position = prbSelectedIndex(selectable);
            const next = position === -1
                ? (step > 0 ? 0 : selectable.length - 1)
                : Math.min(Math.max(position + step, 0), selectable.length - 1);
            prbSelect(prBoardRendered[selectable[next]].key);
        }
        function prbSelect(key) {
            if (prBoardSelected === key) return;
            prBoardSelected = key;
            prbPaintSelection();
            if (prBoardPeekOpen) {
                prBoardPeekScroll = null;
                prbSyncPeekStream();
            }
        }
        function prbPaintSelection() {
            const body = document.getElementById('prb-body');
            if (!body) return;
            let selected = null;
            body.querySelectorAll('.prb-row').forEach(row => {
                const on = row.dataset.key === prBoardSelected;
                row.classList.toggle('prb-sel', on);
                if (on) selected = row;
            });
            if (selected) selected.scrollIntoView({ block: 'nearest' });
        }
        function prbSelectedItem() {
            return prBoardRendered.find(item => item.key === prBoardSelected) || null;
        }

        function prbTogglePeek() {
            if (prBoardPeekOpen) { prbClosePeek(); return; }
            const selectable = prbSelectable();
            if (!selectable.length) return;
            // A selection that has gone away falls back to the first live row.
            const position = Math.max(0, prbSelectedIndex(selectable));
            prBoardSelected = prBoardRendered[selectable[position]].key;
            prBoardPeekOpen = true;
            prBoardPeekScroll = null;
            const pane = document.getElementById('prb-peek');
            if (pane) pane.hidden = false;
            prbPaintSelection();
            prbSyncPeekStream();
        }
        function prbClosePeek() {
            prBoardPeekOpen = false;
            prBoardPeekScroll = null;
            const pane = document.getElementById('prb-peek');
            if (pane) pane.hidden = true;
            prbStopPeekStream();
        }
        function prbStopPeekStream() {
            if (prBoardPeekSource) { prBoardPeekSource.close(); prBoardPeekSource = null; }
            if (prBoardPeekHeartbeat) { clearInterval(prBoardPeekHeartbeat); prBoardPeekHeartbeat = null; }
            prBoardPeekSessionId = null;
            prBoardPeekScreen = '';
            prBoardPeekLines = [];
        }
        // Terminal styling carries no meaning in the transcript window, which
        // the CLI also reads ANSI-stripped.
        function prbStripAnsi(text) {
            return String(text || '')
                .replace(/\\x1b\\][^\\x07\\x1b]*(?:\\x07|\\x1b\\\\)/g, '')
                .replace(/\\x1b\\[[0-9;?]*[ -\\/]*[@-~]/g, '')
                .replace(/\\x1b[@-Z\\\\-_]/g, '')
                .replace(/\\r/g, '');
        }
        // Point the pane at whichever session the selected row stands for,
        // opening one stream at a time.
        function prbSyncPeekStream() {
            if (!prBoardPeekOpen) return;
            const item = prbSelectedItem();
            const session = item ? prbPeekSession(item) : null;
            const sessionId = session ? session.session_id : null;
            if (sessionId === prBoardPeekSessionId) { prbRenderPeek(); return; }
            prbStopPeekStream();
            prBoardPeekSessionId = sessionId;
            if (!sessionId) { prbRenderPeek(); return; }
            const source = new EventSource(
                API_BASE + '/sessions/' + sessionId + '/events' + getAuthQueryParam());
            prBoardPeekSource = source;
            source.onopen = () => sendViewerHeartbeat(sessionId);
            source.onmessage = event => {
                let data;
                try { data = JSON.parse(event.data); } catch (e) { return; }
                if (data.type === 'screen') {
                    prBoardPeekScreen = data.content || '';
                } else if (data.type === 'scrollback_history') {
                    prBoardPeekLines = prbStripAnsi(data.content).split('\\n');
                } else if (data.type === 'scrollback' && data.diff) {
                    prBoardPeekLines = prBoardPeekLines.concat(prbStripAnsi(data.diff).split('\\n'));
                } else {
                    return;
                }
                prbRenderPeek();
            };
            // Keep the desktop streaming this session's screen while we watch.
            prBoardPeekHeartbeat = setInterval(() => sendViewerHeartbeat(sessionId), 10000);
            prbRenderPeek();
        }
        // Visible rows inside the pane, so scrolling steps by real lines.
        function prbPeekInterior() {
            const body = document.getElementById('prb-peek-body');
            if (!body) return 1;
            const lineHeight = parseFloat(getComputedStyle(body).lineHeight) || 16;
            return Math.max(1, Math.floor(body.clientHeight / lineHeight));
        }
        // Step the pane's anchor. null is the live screen; scrolling up anchors
        // a fixed window into the transcript so new output can't shift what is
        // being read, and scrolling past the tail returns to the live screen.
        function prbStepPeekScroll(current, delta, total, interior) {
            const tailTop = Math.max(0, total - interior);
            if (current === null) {
                return delta < 0 && tailTop > 0 ? Math.max(0, tailTop - Math.abs(delta)) : null;
            }
            if (delta < 0) return Math.max(0, current - Math.abs(delta));
            const next = current + Math.abs(delta);
            return next < tailTop ? next : null;
        }
        function prbPeekKey(e) {
            const interior = prbPeekInterior();
            const pageStep = Math.max(1, interior - 1);
            let delta = null;
            if (e.key === 'ArrowUp' || e.key === 'k') delta = -1;
            else if (e.key === 'ArrowDown' || e.key === 'j') delta = 1;
            else if (e.key === 'PageUp') delta = -pageStep;
            else if (e.key === 'PageDown') delta = pageStep;
            if (delta !== null || e.key === 'Home' || e.key === 'End') {
                e.preventDefault();
                const total = prBoardPeekLines.length;
                let next;
                if (e.key === 'Home') next = total > interior ? 0 : prBoardPeekScroll;
                else if (e.key === 'End') next = null;
                else next = prbStepPeekScroll(prBoardPeekScroll, delta, total, interior);
                if (next !== prBoardPeekScroll) {
                    prBoardPeekScroll = next;
                    prbRenderPeek();
                }
                return true;
            }
            if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
                e.preventDefault();
                prbStepSelection(e.key === 'ArrowLeft' ? -1 : 1);
                return true;
            }
            if (e.key === 'Enter' || e.key === 'Escape') {
                e.preventDefault();
                prbClosePeek();
                return true;
            }
            return false;
        }
        function prbRenderPeek() {
            if (!prBoardPeekOpen) return;
            const titleEl = document.getElementById('prb-peek-title');
            const keysEl = document.getElementById('prb-peek-keys');
            const bodyEl = document.getElementById('prb-peek-body');
            if (!titleEl || !keysEl || !bodyEl) return;
            const item = prbSelectedItem();
            const session = item ? prbPeekSession(item) : null;
            const name = session
                ? (prbStripMarker(session.title) || session.dir_name || 'session')
                : 'quick look';
            const dir = session && session.dir_name && session.dir_name !== name
                ? ' · ' + session.dir_name : '';
            const scrolled = prBoardPeekScroll !== null;
            const interior = prbPeekInterior();
            titleEl.innerHTML = '<span class="prb-peek-glyph">' + (scrolled ? '≡' : '▶') + '</span> '
                + escapeHtml(name) + '<span class="prb-peek-dir">' + escapeHtml(dir) + '</span>';
            if (!session) {
                keysEl.textContent = '⏎ close';
                bodyEl.innerHTML = '<span class="prb-peek-empty">'
                    + 'no live screen — the session ended or isn\\'t captured</span>';
                return;
            }
            if (scrolled) {
                const back = Math.max(0, prBoardPeekLines.length - (prBoardPeekScroll + interior));
                keysEl.textContent = back + ' back · ↓ live · ⏎ close';
                const anchored = prBoardPeekLines.slice(prBoardPeekScroll, prBoardPeekScroll + interior);
                bodyEl.innerHTML = anchored.map(line => '<div class="prb-peek-line">'
                    + (escapeHtml(line) || '&nbsp;') + '</div>').join('');
                return;
            }
            keysEl.textContent = '←→ switch · ↑↓ scroll · ⏎ close';
            if (!prBoardPeekScreen) {
                bodyEl.innerHTML = '<span class="prb-peek-empty">waiting for the session\\'s screen…</span>';
                return;
            }
            const lines = prBoardPeekScreen.split('\\n');
            bodyEl.innerHTML = ansiToHtml(lines.slice(Math.max(0, lines.length - interior)).join('\\n'));
        }

        // ── The board itself ───────────────────────────────────────────

        function renderPrBoardBody() {
            const body = document.getElementById('prb-body');
            if (!body) return;
            const now = Math.floor(Date.now() / 1000);
            const q = prBoardQuery.trim().toLowerCase();
            const maxBucket = prBoardViewPrefs.maxAgeHours === null
                ? PRB_BUCKETS.length - 1
                : PRB_BUCKETS.findIndex(b => b.hours === prBoardViewPrefs.maxAgeHours);

            // Dispositions applied server-side too, but a toggle made just now
            // should reshape the board before the next fetch. Each PR fans out
            // into one row per contributing session, like the CLI board;
            // sessions that ended render stale so the live one stands out.
            const entries = [];
            for (const e of prBoardEntries) {
                const disposition = prDisposition(e.pr);
                if (disposition === 'dismissed') continue;
                const primary = disposition === 'primary';
                const sessions = e.sessions || [];
                const rowSessions = sessions.length ? sessions.map(s => [s]) : [[]];
                for (const rowOf of rowSessions) {
                    entries.push({
                        kind: 'pr',
                        entry: e,
                        primary,
                        sessions: rowOf,
                        stale: !rowOf.some(s => s.active),
                        activity: prbActivityTime(rowOf),
                        key: e.owner + '/' + e.repo + '#' + e.number + '@'
                            + (rowOf[0] ? (rowOf[0].session_id || rowOf[0].dir_name) : ''),
                    });
                }
            }
            // Attention first, then primaries, then recency of discussion —
            // the CLI's order, which also breaks ties inside a repository.
            prbSortEntries(entries);

            const activeRepos = new Set();
            for (const s of prBoardSessions) {
                if (s.active && s.repo_name) {
                    activeRepos.add(((s.repo_owner || '') + '/' + s.repo_name).toLowerCase());
                }
            }

            // Active sessions with no visible PR row in their repository keep
            // their own ◇ row, like the CLI's workspace rows.
            const workspaces = [];
            for (const s of prBoardSessions) {
                if (!s.active) continue;
                const represented = prBoardEntries.some(e => prbRepoMatches(s, e)
                    && (e.sessions || []).some(es => es.session_id === s.session_id));
                if (represented) continue;
                workspaces.push({
                    kind: 'session',
                    session: s,
                    activity: prbSessionFreshness(s),
                    key: 'ws:' + (s.session_id || s.dir_name),
                });
            }

            const visible = [];
            for (const item of entries) {
                if (prBoardViewPrefs.liveOnly) {
                    const live = !item.stale
                        || activeRepos.has((item.entry.owner + '/' + item.entry.repo).toLowerCase());
                    if (!live) continue;
                }
                if (prbBucketIndex(now - item.activity) > maxBucket) continue;
                item.previews = prbPreviewLines(item.sessions, q);
                if (q && !prbPrMatches(item.entry, q) && !item.previews.length) continue;
                visible.push(item);
            }
            const visibleWs = [];
            for (const item of workspaces) {
                if (prbBucketIndex(now - item.activity) > maxBucket) continue;
                item.previews = prbPreviewLines([item.session], q);
                if (q && !prbWorkspaceMatches(item.session, q) && !item.previews.length) continue;
                visibleWs.push(item);
            }

            const sessionKeys = new Set();
            for (const item of visible) {
                for (const s of item.sessions) sessionKeys.add(s.session_id || s.dir_name);
            }
            for (const item of visibleWs) {
                sessionKeys.add(item.session.session_id || item.session.dir_name);
            }
            const counts = document.getElementById('prb-counts');
            if (counts) counts.textContent = visible.length + ' PRs · ' + sessionKeys.size + ' sessions';
            const matched = visible.length + visibleWs.length;
            const matches = document.getElementById('prb-matches');
            if (matches) {
                matches.textContent = q
                    ? matched + ' match' + (matched === 1 ? '' : 'es') + ' · Tab context · Esc clears'
                    : '';
            }

            if (!visible.length && !visibleWs.length) {
                body.innerHTML = '<div class="prb-empty">' + (prBoardLoaded
                    ? 'No live sessions or tracked PRs.'
                    : '<span class="prb-throb" style="color:' + PRB_C.green + '">⠋</span> Loading live sessions…')
                    + '</div>';
                prBoardRendered = [];
                if (prBoardPeekOpen) prbSyncPeekStream();
                return;
            }

            // Recency sections → repositories → rows, all newest-first.
            const buckets = new Map();
            const addRow = (repoName, item) => {
                const bucketIdx = prbBucketIndex(now - item.activity);
                if (!buckets.has(bucketIdx)) buckets.set(bucketIdx, new Map());
                const repos = buckets.get(bucketIdx);
                const key = repoName.toLowerCase();
                if (!repos.has(key)) repos.set(key, { name: repoName, rows: [] });
                repos.get(key).rows.push(item);
            };
            for (const item of visible) {
                addRow(item.entry.owner + '/' + item.entry.repo, item);
            }
            for (const item of visibleWs) {
                const s = item.session;
                const name = s.repo_name || s.dir_name || 'unknown';
                addRow(s.repo_owner ? s.repo_owner + '/' + name : name, item);
            }

            prBoardRendered = [];
            let html = '';
            for (const bucketIdx of [...buckets.keys()].sort((a, b) => a - b)) {
                const bucket = PRB_BUCKETS[bucketIdx];
                html += '<div class="prb-bucket" style="background:' + bucket.bg
                    + ';color:' + bucket.text + '">● ' + bucket.label + '</div>';
                const repos = [...buckets.get(bucketIdx).values()];
                for (const repo of repos) repo.rows.sort((a, b) => b.activity - a.activity);
                repos.sort((a, b) => b.rows[0].activity - a.rows[0].activity);
                for (const repo of repos) {
                    html += '<div class="prb-repo">' + escapeHtml(repo.name) + '</div>';
                    for (const item of repo.rows) {
                        const idx = prBoardRendered.length;
                        prBoardRendered.push(item);
                        html += item.kind === 'pr'
                            ? prbPrRowHtml(item, idx, now)
                            : prbSessionRowHtml(item, idx, now);
                    }
                }
            }
            body.innerHTML = html;

            body.querySelectorAll('[data-act]').forEach(el => {
                const item = prBoardRendered[Number(el.dataset.idx)];
                if (!item) return;
                el.onclick = ev => {
                    ev.preventDefault();
                    ev.stopPropagation();
                    if (el.dataset.act === 'peek') {
                        prBoardSelected = item.key;
                        if (prBoardPeekOpen) { prbPaintSelection(); prBoardPeekScroll = null; prbSyncPeekStream(); }
                        else prbTogglePeek();
                        return;
                    }
                    if (item.kind !== 'pr') return;
                    postPrOverride(item.entry.pr, el.dataset.act === 'dismiss'
                        ? 'dismissed'
                        : (item.primary ? 'secondary' : 'primary'));
                    renderPrBoardBody();
                    rerenderAllPrLists();
                };
            });
            // A selected row can disappear between refreshes; the pane follows
            // whatever the selection now points at.
            if (prBoardPeekOpen) prbSyncPeekStream();
        }
`;
