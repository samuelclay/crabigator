// The terminal drawer keeps the existing live renderers and their DOM nodes.
export const modulesJs = `
        const openSessionModules = new Map();
        let fullscreenSessionId = null;
        let fullscreenReturnFocus = null;

        function initSessionModules(sessionId) {
            const card = document.getElementById('session-' + sessionId);
            const panel = document.getElementById('widgets-' + sessionId);
            if (!panel || card.classList.contains('has-modules')) return;
            card.classList.add('has-modules');
            const header = panel.querySelector('.widgets-header');
            const metrics = header.querySelector('.widgets-header-row2');
            const mark = document.getElementById('widgets-mark-' + sessionId);
            const state = document.getElementById('widgets-state-' + sessionId);
            const toggle = document.getElementById('widgets-btn-' + sessionId);
            header.replaceChildren(mark, metrics, toggle);
            metrics.prepend(state);
            header.setAttribute('role', 'button');
            header.tabIndex = 0;
            header.setAttribute('aria-label', 'Session summary: show or hide modules');
            header.setAttribute('aria-controls', 'widgets-content-' + sessionId);
            header.onkeydown = event => {
                if (event.target === header && (event.key === 'Enter' || event.key === ' ')) {
                    event.preventDefault(); toggleWidgets(sessionId);
                }
            };
            toggle.textContent = '⌃';
            toggle.tabIndex = -1;
            toggle.setAttribute('aria-hidden', 'true');
            toggle.title = 'Show or hide session modules';
            const drawer = document.getElementById('widgets-content-' + sessionId);
            const sections = [
                ['recap', 'Recap', ['recap-', 'recaps-']],
                ['prs', 'Pull requests', ['recap-prs-']],
                ['git', 'Git', ['git-']],
                ['changes', 'Changes', ['changes-']],
                ['commits', 'Commits', []],
                ['titles', 'Title history', ['titles-']]
            ];
            const saved = openSessionModules.get(sessionId) || new Set();
            openSessionModules.set(sessionId, saved);
            sections.forEach(([key, title, prefixes], index) => {
                const section = document.createElement('details');
                section.className = 'session-module module-' + key;
                section.id = 'module-' + key + '-' + sessionId;
                section.style.setProperty('--module-order', index);
                section.open = saved.has(key);
                section.innerHTML = '<summary><span class="module-arrow">›</span><span class="module-label">' + title
                    + '</span><span class="module-preview"></span><span class="module-age"></span></summary><div class="module-body"></div>';
                const body = section.querySelector('.module-body');
                prefixes.forEach(prefix => {
                    const node = document.getElementById(prefix + sessionId);
                    if (node) body.appendChild(node);
                });
                section.addEventListener('toggle', () => {
                    if (section.open) saved.add(key); else saved.delete(key);
                });
                if (key === 'recap') {
                    const list = document.createElement('div');
                    list.className = 'module-recaps';
                    body.appendChild(list);
                }
                drawer.appendChild(section);
            });
            const recap = document.getElementById('recap-' + sessionId);
            recap.removeAttribute('onclick');
            recap.removeAttribute('title');
            recap.classList.add('expanded');
            panel.prepend(drawer);
            const fullscreen = document.createElement('button');
            fullscreen.className = 'fullscreen-session-btn';
            fullscreen.type = 'button'; fullscreen.textContent = '⛶';
            fullscreen.title = 'Fullscreen terminal';
            fullscreen.setAttribute('aria-label', 'Fullscreen terminal');
            fullscreen.onclick = () => toggleSessionFullscreen(sessionId);
            card.querySelector('.session-actions').prepend(fullscreen);
            const terminal = document.getElementById('terminal-' + sessionId);
            const resize = new ResizeObserver(entries => {
                fitSummaryAges(sessionId);
                if (entries.some(entry => entry.target === terminal) && sessions.get(sessionId)?.pinned) {
                    preserveTerminalScroll(sessionId, () => {});
                }
            });
            resize.observe(metrics);
            resize.observe(terminal);
            const data = sessions.get(sessionId);
            if (data) data.moduleResize = resize;
            syncModuleDrawer(sessionId);
            refreshSessionModules(sessionId);
        }

        function syncModuleDrawer(sessionId) {
            const card = document.getElementById('session-' + sessionId);
            if (!card?.classList.contains('has-modules')) return;
            const collapsed = card.classList.contains('widgets-collapsed');
            const header = card.querySelector('.widgets-header');
            header.setAttribute('aria-expanded', String(!collapsed));
            const drawer = document.getElementById('widgets-content-' + sessionId);
            drawer.inert = collapsed;
            drawer.setAttribute('aria-hidden', String(collapsed));
        }

        function moduleSummary(sessionId, key, visible, preview, timestamp = 0) {
            const section = document.getElementById('module-' + key + '-' + sessionId);
            if (!section) return;
            section.hidden = !visible;
            section.querySelector('.module-preview').textContent = preview || '';
            updateRecencyElement(section.querySelector('.module-age'), timestamp);
        }

        function moduleRecapEntries(data) {
            // The roster sends a short latest recap without its prompt number.
            // Match that to history by timestamp; full updates also match by turn.
            const entries = [];
            [...(data.recapHistory || []), data.recap?.latest].forEach(entry => {
                if (!entry?.headline?.trim() || /completed the turn without captured transcript output/i.test(entry.headline)) return;
                const index = entries.findIndex(previous =>
                    (entry.prompt_count != null && previous.prompt_count === entry.prompt_count)
                    || (entry.generated_at && previous.generated_at === entry.generated_at));
                if (index < 0) entries.push(entry);
                else entries[index] = { ...entries[index], ...entry };
            });
            return entries.sort((a, b) => b.generated_at - a.generated_at);
        }

        function refreshSessionModules(sessionId) {
            const data = sessions.get(sessionId);
            updateSessionHeaderContext(sessionId);
            if (!data?.element.classList.contains('has-modules')) return;
            const entries = moduleRecapEntries(data);
            const latest = entries[0];
            moduleSummary(sessionId, 'recap', visibleSections.recap && entries.length, latest?.headline, (latest?.generated_at || 0) / 1000);
            const recapList = data.element.querySelector('.module-recaps');
            const recapsHtml = entries.map(entry => {
                const timestamp = Number(entry.generated_at || 0) / 1000;
                return '<div class="module-recap-row"><span class="module-recap-number">'
                    + (entry.prompt_count != null ? '#' + escapeHtml(String(entry.prompt_count)) : '')
                    + '</span><span class="module-recap-headline">' + escapeHtml(entry.headline)
                    + '</span><span class="module-age" data-recency-timestamp="' + timestamp + '">'
                    + escapeHtml(formatElapsed(timestamp)) + '</span></div>';
            }).join('');
            if (recapList && recapList.innerHTML !== recapsHtml) recapList.innerHTML = recapsHtml;
            const prs = (data.prs || []).filter(pr => prDisposition(pr, data) !== 'dismissed');
            moduleSummary(sessionId, 'prs', visibleSections.prs && prs.length, prs.length + ' PR' + (prs.length === 1 ? '' : 's'),
                Math.max(0, ...prs.map(pr => pr.updated_at || pr.refreshed_at || 0)) / 1000);
            const files = data.git?.files || [];
            moduleSummary(sessionId, 'git', visibleSections.git && !!data.git, (data.git?.branch || '') + ' · ' + (files.length ? files.length + (files.length === 1 ? ' file' : ' files') : 'Clean'));
            const count = (data.changes?.by_language || []).reduce((sum, lang) => sum + (lang.changes?.length || 0), 0);
            const threads = data.slackThreads || [];
            moduleSummary(sessionId, 'changes', visibleSections.changes && (count || threads.length), count ? count + (count === 1 ? ' change' : ' changes') : threads.length + ' Slack threads');
            const commits = data.commitHistory || [];
            moduleSummary(sessionId, 'commits', visibleSections.commits && commits.length, commits[commits.length - 1]?.subject, Math.max(0, ...commits.map(commit => commit.timestamp || 0)));
            const commitBody = document.querySelector('#module-commits-' + sessionId + ' .module-body');
            const html = renderCommitHistory(commits, false);
            if (commitBody && commitBody.innerHTML !== html) commitBody.innerHTML = html;
            const titles = document.getElementById('titles-' + sessionId);
            moduleSummary(sessionId, 'titles', titles && titles.style.display !== 'none', 'Previous session titles');
            fitSummaryAges(sessionId);
        }

        function fitSummaryAges(sessionId) {
            const row = document.querySelector('#widgets-' + sessionId + ' .widgets-header-row2');
            if (!row || !row.clientWidth) return;
            const ages = ['prompts', 'completions'].map(key => document.getElementById('widgets-' + key + '-elapsed-' + sessionId))
                .filter(el => el && el.getClientRects().length);
            ages.forEach(el => {
                const full = formatElapsed(Number(el.dataset.recencyTimestamp));
                el.textContent = full;
                el.setAttribute('aria-label', full);
            });
            if (row.scrollWidth <= row.clientWidth) return;
            const newest = ages.filter(el => el.textContent.endsWith(' ago')).sort((a, b) => Number(b.dataset.recencyTimestamp) - Number(a.dataset.recencyTimestamp))[0];
            ages.forEach(el => { if (el !== newest) el.textContent = el.textContent.replace(/ ago$/, ''); });
            if (row.scrollWidth > row.clientWidth && newest) newest.textContent = newest.textContent.replace(/ ago$/, '');
        }

        function toggleSessionFullscreen(sessionId) {
            if (fullscreenSessionId === sessionId) { exitSessionFullscreen(); return; }
            if (fullscreenSessionId) exitSessionFullscreen();
            preserveTerminalScroll(sessionId, () => {
                const card = document.getElementById('session-' + sessionId);
                if (!card) return;
                fullscreenReturnFocus = document.activeElement;
                fullscreenSessionId = sessionId;
                card.classList.add('session-fullscreen');
                card.setAttribute('popover', 'manual');
                card.setAttribute('role', 'dialog');
                card.setAttribute('aria-modal', 'true');
                card.setAttribute('aria-label', 'Fullscreen session terminal');
                if (card.showPopover) card.showPopover();
                document.body.classList.add('session-fullscreen-open');
                const button = card.querySelector('.fullscreen-session-btn');
                button.textContent = '×'; button.title = 'Exit fullscreen'; button.setAttribute('aria-label', 'Exit fullscreen');
                button.focus({ preventScroll: true });
                activateTerminalScroll(sessionId);
                fitSummaryAges(sessionId);
            });
        }

        function restoreSessionFullscreen() {
            if (!fullscreenSessionId) return;
            const card = document.getElementById('session-' + fullscreenSessionId);
            if (!card) { exitSessionFullscreen(); return; }
            if (card.showPopover && !card.matches(':popover-open')) card.showPopover();
        }

        function exitSessionFullscreen() {
            if (!fullscreenSessionId) return;
            preserveTerminalScroll(fullscreenSessionId, () => {
                const card = document.getElementById('session-' + fullscreenSessionId);
                if (card) {
                    if (card.hidePopover && card.matches(':popover-open')) card.hidePopover();
                    card.removeAttribute('popover'); card.removeAttribute('role'); card.removeAttribute('aria-modal'); card.removeAttribute('aria-label');
                    card.classList.remove('session-fullscreen');
                    const button = card.querySelector('.fullscreen-session-btn');
                    button.textContent = '⛶'; button.title = 'Fullscreen terminal'; button.setAttribute('aria-label', 'Fullscreen terminal');
                }
                document.body.classList.remove('session-fullscreen-open');
                fullscreenSessionId = null;
                if (fullscreenReturnFocus?.isConnected) fullscreenReturnFocus.focus({ preventScroll: true });
                fullscreenReturnFocus = null;
            });
        }

        document.addEventListener('keydown', event => {
            if (!fullscreenSessionId) return;
            if (event.key === 'Escape') { event.preventDefault(); event.stopImmediatePropagation(); exitSessionFullscreen(); }
            if (event.key === 'Tab') {
                const card = document.getElementById('session-' + fullscreenSessionId);
                const focusable = [...card.querySelectorAll('button, summary, input, textarea, a[href], [tabindex="0"]')]
                    .filter(el => el.getClientRects().length && !el.closest('[inert]') && !el.disabled);
                const first = focusable[0], last = focusable[focusable.length - 1];
                if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
                else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
            }
        }, true);
`;
