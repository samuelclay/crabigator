// Dashboard JavaScript - cross-session PR board
// A web port of the CLI board (src/prs_board.rs + src/ui/pr_cells.rs): the
// same recency sections, repository groups, row anatomy, colors, and view
// toggles, rendered from the durable D1 record instead of /tmp mirrors.
// Search greps session recaps (transcripts only exist on the desktop).
export const prBoardJs = `
        let prBoardVisible = false;
        let prBoardTimer = null;
        let prBoardThrobTimer = null;
        let prBoardEntries = [];
        let prBoardSessions = [];
        let prBoardLoaded = false;
        let prBoardQuery = '';
        let prBoardExpanded = false;
        let prBoardRendered = [];

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
            orange: '#d7af5f',
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
                + '<span class="prb-searchwrap"><input id="prb-search" placeholder="/ search" spellcheck="false" autocomplete="off"><span id="prb-matches"></span></span>'
                + '</div><div class="prb-body" id="prb-body"></div>';
        }

        function prbBindSearch() {
            const input = document.getElementById('prb-search');
            if (!input) return;
            input.addEventListener('input', () => {
                prBoardQuery = input.value;
                renderPrBoardBody();
            });
            input.addEventListener('keydown', e => {
                if (e.key === 'Escape') {
                    prBoardQuery = '';
                    input.value = '';
                    prBoardExpanded = false;
                    input.blur();
                    renderPrBoardBody();
                } else if (e.key === 'Tab') {
                    // Tab flips the recap previews between one snippet and
                    // every matching line, like the CLI's transcript context.
                    e.preventDefault();
                    prBoardExpanded = !prBoardExpanded;
                    renderPrBoardBody();
                }
                e.stopPropagation();
            });
        }

        // The CLI's keyboard shortcuts, active while the board is open.
        document.addEventListener('keydown', e => {
            if (!prBoardVisible || e.metaKey || e.ctrlKey || e.altKey) return;
            const tag = e.target && e.target.tagName;
            if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target && e.target.isContentEditable)) return;
            if (e.key === '/') {
                e.preventDefault();
                const el = document.getElementById('prb-search');
                if (el) el.focus();
            } else if (e.key === 'r') prBoardToggleRecap();
            else if (e.key === 'a') prBoardCycleAge();
            else if (e.key === 's') prBoardToggleLive();
            else if (e.key === '+' || e.key === '=') prBoardDays(1);
            else if (e.key === '-' || e.key === '_') prBoardDays(-1);
            else if (e.key === 'Escape' && prBoardQuery) {
                prBoardQuery = '';
                prBoardExpanded = false;
                const el = document.getElementById('prb-search');
                if (el) el.value = '';
                renderPrBoardBody();
            } else if (e.key === 'q' || e.key === 'Escape') togglePrBoard();
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
        function prbLatestSessionTitle(sessions) {
            const sorted = [...sessions].sort((a, b) => prbSessionFreshness(b) - prbSessionFreshness(a));
            for (const s of sorted) {
                const plain = prbStripMarker(s.title);
                if (plain && plain !== s.dir_name) return prbMarker(s.platform) + plain;
            }
            return '';
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

        // The web quick look: focus the dashboard on the row's live session.
        function prBoardPeek(sessionId) {
            togglePrBoard();
            focusOnSession(sessionId);
        }
        function prbActivityHtml(sessions, now) {
            const cell = prbActivityCell(sessions, now);
            const live = sessions.find(s => s.active && s.session_id);
            if (!live) return '<span class="prb-activity">' + cell + '</span>';
            return '<span class="prb-activity prb-peek" onclick="prBoardPeek(\\'' + escapeHtml(live.session_id)
                + '\\')" title="Peek at this session on the dashboard">' + cell + '</span>';
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

        function prbDiffFiles(pr) {
            const parts = [];
            if (pr.additions || pr.deletions) {
                parts.push('<span class="prb-add">+' + (pr.additions || 0)
                    + '</span> <span class="prb-del">-' + (pr.deletions || 0) + '</span>');
            }
            if (pr.changed_files) {
                parts.push(pr.changed_files + ' file' + (pr.changed_files === 1 ? '' : 's'));
            }
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
        // Slack links pair with recap rows on the right, like the CLI's
        // detail column; without local Slack metadata, the channel ID from the
        // permalink is the label (matching the CLI's all-sessions view).
        function prbSlackLinks(pr) {
            const urls = [];
            if (pr.slack_origin_url) urls.push(pr.slack_origin_url);
            for (const u of pr.slack_comment_urls || []) {
                if (!urls.includes(u)) urls.push(u);
            }
            return urls.map(u => {
                const m = /\\/archives\\/([A-Z0-9]+)/.exec(u);
                return '<a class="prb-slack" href="' + escapeHtml(u)
                    + '" target="_blank" rel="noopener noreferrer">#' + escapeHtml(m ? m[1] : 'thread') + '</a>';
            });
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

        // ── Search (metadata + recap previews) ─────────────────────────

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
        // Inline excerpts confirming why a search hit, from session recaps.
        // Needs 3+ characters, like the CLI's transcript search.
        function prbPreviewLines(sessions, q) {
            if (q.length < 3) return [];
            const out = [];
            const seen = new Set();
            for (const s of sessions) {
                const key = s.session_id || s.dir_name;
                if (seen.has(key)) continue;
                seen.add(key);
                const lines = prbRecapText(s.recap).filter(line => line.toLowerCase().includes(q));
                if (!lines.length) continue;
                const count = lines.length + ' match' + (lines.length === 1 ? '' : 'es');
                const head = '⌕ <span class="prb-pv-dir">' + escapeHtml(s.dir_name || '') + '</span> · ';
                if (!prBoardExpanded) {
                    out.push('<div class="prb-pv">' + head + '<span class="prb-pv-text">'
                        + prbHighlight(lines[lines.length - 1], q) + '</span> · ' + count + '</div>');
                } else {
                    out.push('<div class="prb-pv">' + head + count + '</div>');
                    for (const line of lines) {
                        out.push('<div class="prb-pv prb-pv-ctx">│ <span class="prb-pv-text">'
                            + prbHighlight(line, q) + '</span></div>');
                    }
                }
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
                + (item.stale ? ' prb-stale' : '') + '">';
            html += '<div class="prb-l1"><span class="prb-l1-left">' + star + ident + '</span>'
                + prbActivityHtml(sessions, now)
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

        function prbSessionRowHtml(item, now) {
            const s = item.session;
            const plain = prbStripMarker(s.title) || s.dir_name || 'session';
            const title = prbMarker(s.platform) + plain;
            let html = '<div class="prb-row">';
            html += '<div class="prb-l1"><span class="prb-l1-left">'
                + '<span class="prb-diamond">◇</span>'
                + '<span class="prb-ident">' + escapeHtml(title) + '</span>'
                + '<span class="prb-branch">⎇ ' + escapeHtml(s.branch || '(no branch)') + '</span></span>'
                + prbActivityHtml([s], now)
                + '<span class="prb-status"></span></div>';
            if (prBoardViewPrefs.detail === 1 && s.recap && s.recap.headline) {
                for (const row of prbRecapRows(s.recap, now)) {
                    html += prbDlRowHtml(row, row.right);
                }
            }
            html += item.previews.join('') + '</div>';
            return html;
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
                    });
                }
            }

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
                workspaces.push({ kind: 'session', session: s, activity: prbSessionFreshness(s) });
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
                matches.textContent = q ? matched + ' match' + (matched === 1 ? '' : 'es') : '';
            }

            if (!visible.length && !visibleWs.length) {
                body.innerHTML = '<div class="prb-empty">' + (prBoardLoaded
                    ? 'No live sessions or tracked PRs.'
                    : '<span class="prb-throb" style="color:' + PRB_C.green + '">⠋</span> Loading sessions…')
                    + '</div>';
                prBoardRendered = [];
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
                            : prbSessionRowHtml(item, now);
                    }
                }
            }
            body.innerHTML = html;

            body.querySelectorAll('[data-act]').forEach(el => {
                const item = prBoardRendered[Number(el.dataset.idx)];
                if (!item || item.kind !== 'pr') return;
                el.onclick = ev => {
                    ev.preventDefault();
                    ev.stopPropagation();
                    postPrOverride(item.entry.pr, el.dataset.act === 'dismiss'
                        ? 'dismissed'
                        : (item.primary ? 'secondary' : 'primary'));
                    renderPrBoardBody();
                    rerenderAllPrLists();
                };
            });
        }
`;
