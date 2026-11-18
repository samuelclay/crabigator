// Dashboard JavaScript - the filter bar, the View panel and the Account popover.
// The filter bar says which sessions are shown. The View panel says how the
// view on screen looks: the List, the PR board, or an open session. Account
// holds only account, device and agent items.
import {
    iconChevronDown,
    thumbGroupNone, thumbGroupComputer, thumbGroupProject, thumbGroupBoth,
    thumbLeadSession, thumbLeadPr, thumbWrap, thumbScroll, thumbSideLeft, thumbSideRight,
} from '../icons';

export const viewPanelJs = `
        const VP_CHEVRON = '${iconChevronDown}';
        const VP_THUMBS = {
            none: '${thumbGroupNone}', computer: '${thumbGroupComputer}',
            project: '${thumbGroupProject}', both: '${thumbGroupBoth}',
            sessions: '${thumbLeadSession}', prs: '${thumbLeadPr}',
            wrap: '${thumbWrap}', scroll: '${thumbScroll}',
            left: '${thumbSideLeft}', right: '${thumbSideRight}',
        };
        const VIEW_SCOPES = {
            list: { name: 'List', label: 'View options for the List' },
            board: { name: 'PR board', label: 'View options for the PR board' },
            session: { name: 'Session', label: 'View options for the session screen' },
        };

        // ── Which sessions ─────────────────────────────────────────────

        // The view the filter bar acts on, or null when it has no place:
        // before pairing, in Conductor, and in the single-session view.
        function boardScope() {
            if (!isPaired || conductorVisible || isFocusedMode()) return null;
            return prBoardVisible ? 'board' : 'list';
        }

        // Every computer the account's sessions run on, by name.
        function knownComputers() {
            const names = new Set();
            // One normaliser for the menu, the List filter and the PR board.
            const add = name => {
                const clean = prbDeviceLabel({ device_name: name });
                if (clean) names.add(clean);
            };
            for (const session of allSessions) add(session.device_name);
            for (const session of prBoardSessions) add(session.device_name);
            for (const entry of prBoardEntries) {
                for (const session of prbContributorSessions(entry)) add(session.device_name);
            }
            add(prBoardViewPrefs.computer);
            return [...names].sort((a, b) => a.localeCompare(b));
        }

        // Redraw the List only when a session crossed into or out of the filters.
        let lastListFilterSignature = null;
        function refreshListFilter() {
            if (!isPaired) return;
            if ([...listSessionsInFilter().keys()].join(' ') !== lastListFilterSignature) rerenderSessions();
        }
        setInterval(() => { if (!document.hidden) refreshListFilter(); }, 30000);

        // Sessions the filters hide never disappear silently: the List says
        // how many there are and offers them back.
        function updateFilterNote() {
            const note = document.getElementById('filter-note');
            if (!note) return;
            const shown = boardScope() === 'list' ? listSessionsInFilter().size : sessions.size;
            const hidden = sessions.size - shown;
            note.hidden = hidden <= 0;
            if (hidden <= 0) return;
            // With nothing left on the List, the note takes the List's place
            // at the top, large, so it cannot be missed. Otherwise it follows
            // the last row.
            const list = document.getElementById('sessions');
            note.classList.toggle('filter-note-empty', shown === 0);
            if (shown === 0 && list.previousElementSibling !== note) list.before(note);
            else if (shown > 0 && list.nextElementSibling !== note) list.after(note);
            const count = hidden + (hidden === 1 ? ' session is' : ' sessions are');
            const text = shown === 0
                ? 'No sessions match the filters above. ' + count + ' outside them.'
                : hidden + (hidden === 1 ? ' more session is' : ' more sessions are') + ' outside the filters above.';
            if (note.noteText === text) return;
            note.noteText = text;
            note.innerHTML = '<div class="filter-note-box"><span>' + text + '</span>'
                + '<button type="button" class="filter-note-btn" onclick="clearBoardFilters()">Show all sessions</button></div>';
        }

        function setBoardWindow(hours) {
            setBoardFilters({ maxAgeHours: hours });
        }
        function setBoardLiveOnly(liveOnly) {
            setBoardFilters({ liveOnly });
        }
        function setBoardComputer(name) {
            setBoardFilters({ computer: name });
        }
        function clearBoardFilters() {
            setBoardFilters({ maxAgeHours: null, computer: '' });
        }
        // Save the changed filters, then redraw everything that shows them.
        function setBoardFilters(changes) {
            Object.assign(prBoardViewPrefs, changes);
            savePrBoardView();
            refreshListFilter();
            renderFilterBar();
            updateSidebarContent();
            if (prBoardVisible) renderPrBoardBody();
            renderViewPanel();
        }

        // ── Filter bar ─────────────────────────────────────────────────

        let chipMenuName = null;

        function filterChipHtml(name, labelHtml, title) {
            return '<button type="button" class="filter-chip" id="chip-' + name + '" data-chip="' + name
                + '" aria-haspopup="menu" aria-controls="chip-menu" aria-expanded="' + (chipMenuName === name)
                + '" title="' + title + '">' + labelHtml + VP_CHEVRON + '</button>';
        }

        function renderFilterBar() {
            const bar = document.getElementById('filter-bar');
            if (!bar) return;
            const scope = boardScope();
            bar.hidden = !scope;
            updateFilterNote();
            if (!scope) {
                closeChipMenu();
                if (viewPanelScope && viewPanelScope !== 'session') closeViewPanel();
                return;
            }
            const onBoard = scope === 'board';
            const band = VIEW_BANDS[viewBandIndex(prBoardViewPrefs.maxAgeHours)];
            const computer = prBoardViewPrefs.computer;
            let html = filterChipHtml('time',
                '<span class="chip-dot" style="background:' + band.color + '"></span><span>' + band.label + '</span>',
                'Show sessions active in this time' + (onBoard ? ' (a)' : ''));
            // Only the PR board keeps ended sessions; the List holds live ones.
            if (onBoard) {
                html += filterChipHtml('sessions',
                    '<span>' + (prBoardViewPrefs.liveOnly ? 'Live' : 'Live + ended') + '</span>',
                    'Show live sessions only, or ended ones too (l)');
            }
            html += filterChipHtml('computer', computer
                ? '<span style="color:' + computerColor(computer) + '">' + escapeHtml(computer) + '</span>'
                : '<span>All computers</span>', 'Show sessions on this computer');
            const chips = document.getElementById('filter-chips');
            if (chips.barHtml === html) return;
            const focused = chips.contains(document.activeElement) ? document.activeElement.dataset.chip : null;
            chips.barHtml = html;
            chips.innerHTML = html;
            if (focused) document.getElementById('chip-' + focused)?.focus();
        }

        function chipMenuItems(name) {
            if (name === 'time') {
                return VIEW_BANDS.map(band => ({
                    value: band.hours === null ? 'all' : String(band.hours), label: band.menu,
                    on: band.hours === prBoardViewPrefs.maxAgeHours, dot: band.color,
                }));
            }
            if (name === 'sessions') {
                return [
                    { value: 'live', label: 'Live', on: prBoardViewPrefs.liveOnly },
                    { value: 'all', label: 'Live and ended', on: !prBoardViewPrefs.liveOnly },
                ];
            }
            return [{ value: '', label: 'All computers', on: !prBoardViewPrefs.computer }]
                .concat(knownComputers().map(computer => ({
                    value: computer, label: computer, on: computer === prBoardViewPrefs.computer,
                    color: computerColor(computer),
                })));
        }

        function openChipMenu(name) {
            const chip = document.getElementById('chip-' + name);
            const menu = document.getElementById('chip-menu');
            if (!chip || !menu) return;
            closeChipMenu();
            closeViewPanel();
            closeSettingsPopover();
            chipMenuName = name;
            menu.setAttribute('aria-label', { time: 'Time', sessions: 'Sessions', computer: 'Computer' }[name]);
            menu.innerHTML = chipMenuItems(name).map(item =>
                '<button type="button" class="chip-menu-item" role="menuitemradio" aria-checked="' + item.on
                + '" data-value="' + escapeHtml(item.value) + '">'
                + '<span class="chip-menu-check" aria-hidden="true">' + (item.on ? '✓' : '') + '</span>'
                + (item.dot ? '<span class="chip-dot" style="background:' + item.dot + '"></span>' : '')
                + '<span' + (item.color ? ' style="color:' + item.color + '"' : '') + '>' + escapeHtml(item.label) + '</span>'
                + '</button>').join('');
            menu.hidden = false;
            const rect = chip.getBoundingClientRect();
            menu.style.top = (rect.bottom + 6) + 'px';
            menu.style.left = Math.max(8, Math.min(rect.left, window.innerWidth - menu.offsetWidth - 8)) + 'px';
            chip.setAttribute('aria-expanded', 'true');
            (menu.querySelector('[aria-checked="true"]') || menu.firstElementChild)?.focus();
        }

        function closeChipMenu(returnFocus = false) {
            if (!chipMenuName) return;
            const chip = document.getElementById('chip-' + chipMenuName);
            chipMenuName = null;
            document.getElementById('chip-menu').hidden = true;
            if (!chip) return;
            chip.setAttribute('aria-expanded', 'false');
            if (returnFocus) chip.focus();
        }

        function pickChipMenuItem(value) {
            const name = chipMenuName;
            closeChipMenu();
            if (name === 'time') setBoardWindow(value === 'all' ? null : Number(value));
            else if (name === 'sessions') setBoardLiveOnly(value === 'live');
            else if (name === 'computer') setBoardComputer(value);
            document.getElementById('chip-' + name)?.focus();
        }

        function chipMenuKey(event) {
            const items = [...document.querySelectorAll('#chip-menu .chip-menu-item')];
            const at = items.indexOf(document.activeElement);
            let next = null;
            if (event.key === 'ArrowDown') next = items[(at + 1) % items.length];
            else if (event.key === 'ArrowUp') next = items[(at - 1 + items.length) % items.length];
            else if (event.key === 'Home') next = items[0];
            else if (event.key === 'End') next = items[items.length - 1];
            else if (event.key === 'Tab') { closeChipMenu(true); event.preventDefault(); return; }
            if (!next) return;
            event.preventDefault();
            next.focus();
        }

        // ── View panel: parts ──────────────────────────────────────────

        // data-do names the choice a control makes and data-arg its value.
        // data-fid lets focus return to the same control after a redraw.
        function vpAttrs(action, arg) {
            const value = escapeHtml(String(arg));
            return ' data-do="' + action + '" data-arg="' + value + '" data-fid="' + action + ':' + value + '"';
        }
        function vpSection(title, inner) {
            return '<section class="vp-section"><h3>' + title + '</h3>' + inner + '</section>';
        }
        function vpNote(text) {
            return '<p class="vp-note">' + text + '</p>';
        }
        function vpTile(action, arg, current, label, note = '') {
            const on = arg === current;
            return '<button type="button" class="vp-tile' + (on ? ' on' : '') + '" aria-pressed="' + on + '"'
                + vpAttrs(action, arg) + '>' + VP_THUMBS[arg]
                + '<span class="vp-tile-label">' + label + '</span>'
                + (note ? '<span class="vp-tile-note">' + note + '</span>' : '') + '</button>';
        }
        // options: [value, label, colour]. The group takes the name it is given.
        function vpSegments(action, options, current, name) {
            return '<div class="vp-seg" role="group" aria-label="' + name + '">'
                + options.map(([value, label, color]) => '<button type="button" aria-pressed="' + (value === current) + '"'
                    + vpAttrs(action, value) + (color ? ' style="color:' + color + '"' : '') + '>'
                    + escapeHtml(label) + '</button>').join('') + '</div>';
        }
        // items: [key, label]. tones adds a colour dot to the named chips.
        function vpChips(action, items, set, name, tones = {}) {
            return '<div class="vp-chips" role="group" aria-label="' + name + '">'
                + items.map(([key, label]) => '<button type="button" class="vp-chip" aria-pressed="' + !!set[key] + '"'
                    + vpAttrs(action, key) + '><span class="vp-check" aria-hidden="true"></span>'
                    + (tones[key] ? '<span class="vp-tone" style="background:' + tones[key] + '" aria-hidden="true"></span>' : '')
                    + '<span>' + label + '</span></button>').join('') + '</div>';
        }
        function vpSwitch(action, on, label) {
            const id = 'vp-switch-' + action;
            return '<div class="vp-row"><span id="' + id + '">' + label + '</span>'
                + '<button type="button" class="vp-switch" role="switch" aria-checked="' + on + '" aria-labelledby="' + id + '"'
                + vpAttrs(action, 'toggle') + '><span class="vp-switch-track"><span class="vp-switch-knob"></span></span></button></div>';
        }
        // down and up: [visible text, accessible name, enabled].
        function vpStepper(action, value, down, up, className = '') {
            const step = (delta, [text, name, enabled], extra) => '<button type="button" class="vp-step ' + extra + '" aria-label="' + name + '"'
                + vpAttrs(action, delta) + (enabled ? '' : ' disabled') + '>' + text + '</button>';
            return '<div class="vp-stepper ' + className + '">' + step(-1, down, 'vp-step-down')
                + '<span class="vp-step-value" aria-live="polite">' + value + '</span>' + step(1, up, 'vp-step-up') + '</div>';
        }
        function vpTextSizeStepper() {
            return vpStepper('text', Math.round(FONT_SCALES[currentFontScaleIndex] * 100) + '%',
                ['A', 'Smaller text', currentFontScaleIndex > 0],
                ['A', 'Larger text', currentFontScaleIndex < FONT_SCALES.length - 1], 'vp-text-size');
        }

        function vpWhichSessions(scope) {
            const selected = viewBandIndex(prBoardViewPrefs.maxAgeHours);
            const ruler = '<div class="vp-ruler" role="group" aria-label="Time">' + VIEW_BANDS.map((band, index) =>
                '<button type="button" class="vp-band' + (index === selected ? ' on' : '') + '" aria-pressed="' + (index === selected)
                + '" title="' + band.menu + '"' + vpAttrs('window', band.hours === null ? 'all' : band.hours) + '>'
                + '<span class="vp-band-bar" style="background:' + (index <= selected ? band.color : '#2A323D') + '"></span>'
                + '<span class="vp-band-label">' + band.label + '</span></button>').join('') + '</div>';
            const computers = [['', 'All computers']]
                .concat(knownComputers().map(computer => [computer, computer, computerColor(computer)]));
            const onBoard = scope === 'board';
            return vpSection('Which sessions', ruler
                // The List only ever holds live sessions, so the switch is the PR board's.
                + (onBoard ? vpSwitch('ended', !prBoardViewPrefs.liveOnly, 'Ended sessions') : '')
                + vpSegments('computer', computers, prBoardViewPrefs.computer, 'Computer')
                + vpNote('The same ' + (onBoard ? 'three' : 'two') + ' filters as the chips above. Change them in either place.'));
        }

        // ── View panel: the three scopes ───────────────────────────────

        function vpRowSection(scope) {
            const onBoard = scope === 'board';
            return vpSection(onBoard ? 'A PR board row shows' : 'A List row shows',
                '<div class="vp-sample vp-sample-' + scope + '" id="vp-row-sample" inert aria-hidden="true"></div>'
                + vpChips('row', onBoard ? ROW_ITEMS : LIST_ROW_ITEMS, viewSets[scope], 'Shown on a row')
                + vpNote(onBoard ? 'The List keeps its own set.' : 'The PR board keeps its own set.'));
        }

        function vpSizeSection(scope) {
            let html = '<div class="vp-size-row">' + vpTextSizeStepper()
                + vpSegments('spacing', ROW_SPACINGS, document.body.dataset.rowSpacing, 'Row spacing') + '</div>';
            if (scope === 'list') {
                // The session list beside the List: tablets and desktop only.
                html += '<div class="vp-wide-only"><span class="vp-label">Session list side</span>'
                    + '<div class="vp-tiles vp-tiles-2">' + vpTile('side', 'left', sidebarPosition, 'Left')
                    + vpTile('side', 'right', sidebarPosition, 'Right') + '</div>'
                    + vpSwitch('pin', sidebarPinned, 'Keep the session list open') + '</div>';
            }
            return vpSection('Size and spacing', html);
        }

        function vpListHtml() {
            const choice = groupByChoice(groupByComputer, groupingMode);
            const byProject = choice === 'project' || choice === 'both';
            return vpWhichSessions('list')
                + vpSection('Group by', '<div class="vp-tiles vp-tiles-4">'
                    + vpTile('group', 'none', choice, 'One list')
                    + vpTile('group', 'computer', choice, 'Computer')
                    + vpTile('group', 'project', choice, 'Project')
                    + vpTile('group', 'both', choice, 'Computer, then project') + '</div>'
                    + (byProject ? '<div class="vp-row"><span class="vp-label">Sort projects</span>'
                        + vpSegments('sort', [['recent', 'Most recent first'], ['alpha', 'A to Z']], projectOrderMode, 'Sort projects')
                        + '</div>' : ''))
                + vpRowSection('list')
                + vpSizeSection('list');
        }

        function vpBoardHtml() {
            return vpWhichSessions('board')
                + vpSection('Group by', '<div class="vp-tiles vp-tiles-4">'
                    + vpTile('group', 'none', groupByComputer ? 'computer' : 'none', 'One list')
                    + vpTile('group', 'computer', groupByComputer ? 'computer' : 'none', 'Computer') + '</div>')
                + vpSection('Each block starts with', '<div class="vp-tiles vp-tiles-2">'
                    + vpTile('lead', 'sessions', prBoardViewPrefs.view, 'The session')
                    + vpTile('lead', 'prs', prBoardViewPrefs.view, 'The pull request') + '</div>')
                + vpRowSection('board')
                + vpSizeSection('board');
        }

        // The drawer's parts, with the colour each wears in the drawing.
        const VP_DRAWER_ITEMS = [
            ['recap', 'Recap', '#5f87ff'], ['prs', 'Pull requests', '#af87ff'], ['commits', 'Commits', '#ff8700'],
            ['git', 'Git status', '#5fff5f'], ['changes', 'Changes', '#ff87d7'], ['flow', 'Flow scene', '#d7af5f'],
        ];
        function vpDrawerOn(key) {
            return key === 'flow' ? !!sidebarVisibleStats.flow : !!visibleSections[key];
        }

        // A small drawing of an open session that redraws for each choice:
        // header, terminal, the drawer's parts, the stats strip and the input.
        function vpSessionDrawingHtml() {
            const pixels = TERMINAL_HEIGHTS[currentHeightIndex];
            const lowest = TERMINAL_HEIGHTS[0], highest = TERMINAL_HEIGHTS[TERMINAL_HEIGHTS.length - 1];
            const terminal = Math.round(18 + (pixels - lowest) / (highest - lowest) * 62);
            const lines = ['80%', '55%', '90%', '42%', '70%', '86%', '36%', '66%', '80%', '52%', '74%', '48%'];
            const tones = { 1: '#5fff5f', 4: '#d7af5f', 7: '#5fff5f' };
            const drawer = widgetsExpanded
                ? VP_DRAWER_ITEMS.filter(([key]) => vpDrawerOn(key)).map(([key, , tone], index) =>
                    '<span style="width:' + [60, 76, 48, 66, 54, 70][index % 6] + '%;background:' + tone + '"></span>').join('')
                : '';
            return '<div class="vp-drawing" aria-hidden="true"><div class="vp-drawing-head"></div>'
                + '<div class="vp-drawing-terminal" style="height:' + terminal + 'px">'
                + lines.map((width, index) => '<span style="width:' + width + ';background:' + (tones[index] || '#6B7684') + '"></span>').join('')
                + '</div>' + (drawer ? '<div class="vp-drawing-drawer">' + drawer + '</div>' : '')
                + '<div class="vp-drawing-strip"></div><div class="vp-drawing-input"></div></div>';
        }

        function vpSessionHtml() {
            const pixels = TERMINAL_HEIGHTS[currentHeightIndex];
            const drawerSet = {}, drawerTones = {};
            for (const [key, , tone] of VP_DRAWER_ITEMS) { drawerSet[key] = vpDrawerOn(key); drawerTones[key] = tone; }
            return vpSection('Terminal text',
                    '<div class="vp-terminal-sample" style="font-size:' + (11 * FONT_SCALES[currentFontScaleIndex]).toFixed(1) + 'px" aria-hidden="true">'
                    + '<span style="color:#5fff5f">●</span> Read dashboard.ts<br>Mapped every setting on the screen.</div>'
                    + vpTextSizeStepper())
                + vpSection('Long lines', '<div class="vp-tiles vp-tiles-2">'
                    + vpTile('wrap', 'wrap', terminalWrapEnabled ? 'wrap' : 'scroll', 'Wrap', 'Continue on the next line')
                    + vpTile('wrap', 'scroll', terminalWrapEnabled ? 'wrap' : 'scroll', 'Scroll sideways', 'Keep each line whole') + '</div>')
                + vpSection('Terminal height', '<div class="vp-height-row">' + vpSessionDrawingHtml()
                    + '<div class="vp-height-controls">'
                    + vpStepper('height', terminalHeightLabel(pixels),
                        ['−', 'Shorter terminal', currentHeightIndex > 0],
                        ['+', 'Taller terminal', currentHeightIndex < TERMINAL_HEIGHTS.length - 1], 'vp-height')
                    + '<span class="vp-label">Details drawer under the terminal</span>'
                    + vpSegments('drawer', [['open', 'Open'], ['closed', 'Tucked away']], widgetsExpanded ? 'open' : 'closed',
                        'Details drawer under the terminal')
                    + vpNote('The small screen redraws as you choose a height and what the drawer holds.')
                    + '</div></div>')
                + vpSection('In the drawer', vpChips('section', VP_DRAWER_ITEMS, drawerSet, 'In the drawer', drawerTones))
                + vpSection('The stats strip shows',
                    '<div class="vp-sample vp-sample-strip" id="vp-strip-sample" inert aria-hidden="true"></div>'
                    + vpChips('stat', SESSION_STATS_ITEMS, viewSets.session, 'Shown in the stats strip'));
        }

        // ── View panel: the sample row ─────────────────────────────────

        // A made-up session, so each chip shows what it adds.
        function viewSampleSession(scope, now) {
            const onBoard = scope === 'board';
            return {
                session_mark: { glyph: onBoard ? '◆' : '◈', bg: onBoard ? [42, 36, 64] : [53, 43, 37], fg: onBoard ? [175, 135, 255] : [255, 170, 119] },
                platform: 'claude', dir_name: 'crabigator', device_name: 'ClayMac Studio',
                title: onBoard ? 'Live updates for mobile' : 'Redesign the settings screen',
                branch: onBoard ? 'mobile-realtime' : 'settings-redesign',
                state: 'complete', active: true, started_at: now - 8040,
                prompts_changed_at: now - (onBoard ? 1500 : 190), completions_changed_at: now - (onBoard ? 1440 : 70),
                stats: { thinking_seconds: 2280, tools: 212, compressions: 3, prompts: 14, completions: 13,
                    context_used: 186000, context_limit: 1000000 },
                uncommitted: 3, additions: 212, deletions: 48,
                recap: { headline: onBoard ? 'Streamed live updates to the board and added a stories pill.'
                    : 'Mapped every setting and drew four ways to arrange them.', bullets: [], generated_at: (now - 70) * 1000 },
            };
        }

        // The samples are drawn by the same code as the real rows: the
        // List's row header, the PR board's session block, and the strip
        // on an open session's drawer handle.
        function paintViewSamples(panel, scope) {
            const now = Math.floor(Date.now() / 1000);
            const row = panel.querySelector('#vp-row-sample');
            if (row && scope === 'board') {
                row.innerHTML = '<div class="pr-board"><div class="prb-body">'
                    + prbSampleRowHtml(viewSampleSession('board', now), now) + '</div></div>';
            } else if (row) {
                const sample = viewSampleSession('list', now);
                row.innerHTML = '<div class="container" data-layout="1"><div class="session-card session-row">'
                    + sessionHeaderHtml({ id: 'view-sample', state: sample.state }, false) + '</div></div>';
                const header = row.querySelector('.session-header');
                header.querySelector('.title').textContent = sample.title;
                header.querySelector('.title-row > span').innerHTML = sessionMarkChipHtml(sample);
                paintSessionRowContext(header, {
                    computer: groupByComputer ? '' : sample.device_name,
                    cwd: '/Users/sam/projects/crabigator',
                    branch: sample.branch,
                    facts: boardRowFacts(sample, now),
                }, viewSets.list);
            }
            const strip = panel.querySelector('#vp-strip-sample');
            if (strip) {
                strip.innerHTML = '<div class="session-card has-modules"><div class="widgets-panel"><div class="widgets-header">'
                    + '<div class="widgets-header-row2">' + statsStripHtml('view-sample') + '</div></div></div></div>';
                const values = {
                    session: '2h 14m', thinking: '38m', prompts: '14', 'prompts-elapsed': '3m ago', completions: '13',
                    'completions-elapsed': '1m ago', compactions: '3', context: '186k / 1M · 19%',
                };
                for (const [key, text] of Object.entries(values)) {
                    strip.querySelector('#widgets-' + key + '-view-sample').textContent = text;
                }
                strip.querySelector('#widgets-context-view-sample').style.color = '#5fafff';
                strip.querySelector('#widgets-tools-view-sample').innerHTML = '<span style="color:#f0883e">▂▅▃▁▆▂▁▃</span>';
                applySessionCardStatsVisibility(strip);
            }
        }

        // ── View panel: open, close, draw ──────────────────────────────

        let viewPanelScope = null;
        let viewPanelOpener = null;

        function toggleViewPanel(scope, opener) {
            if (viewPanelScope && viewPanelOpener === opener) {
                closeViewPanel(true);
                return;
            }
            openViewPanel(scope === 'auto' ? boardScope() : scope, opener);
        }

        function openViewPanel(scope, opener) {
            const panel = document.getElementById('view-panel');
            if (!scope || !panel) return;
            closeChipMenu();
            closeSettingsPopover();
            closeViewPanel();
            viewPanelScope = scope;
            viewPanelOpener = opener || null;
            panel.setAttribute('aria-label', VIEW_SCOPES[scope].label);
            renderViewPanel();
            // The top layer keeps the panel above a fullscreen session.
            if (panel.showPopover) { try { panel.showPopover(); } catch (e) {} }
            panel.classList.add('open');
            if (viewPanelOpener) viewPanelOpener.setAttribute('aria-expanded', 'true');
            positionViewPanel();
            panel.focus({ preventScroll: true });
        }

        function closeViewPanel(returnFocus = false) {
            if (!viewPanelScope) return;
            const panel = document.getElementById('view-panel');
            const opener = viewPanelOpener;
            viewPanelScope = null;
            viewPanelOpener = null;
            panel.classList.remove('open');
            if (panel.hidePopover) { try { panel.hidePopover(); } catch (e) {} }
            panel.innerHTML = '';
            if (opener) opener.setAttribute('aria-expanded', 'false');
            if (!returnFocus) return;
            const target = opener && opener.isConnected ? opener : document.getElementById('view-btn');
            if (target) target.focus({ preventScroll: true });
        }

        // A popover under its button on tablets and desktop. On a phone the
        // stylesheet turns it into a bottom sheet, so nothing is placed here.
        function positionViewPanel() {
            const panel = document.getElementById('view-panel');
            if (!viewPanelScope || !panel) return;
            panel.style.left = panel.style.top = panel.style.maxHeight = '';
            if (window.matchMedia('(max-width: 600px)').matches) return;
            const opener = viewPanelOpener && viewPanelOpener.isConnected
                ? viewPanelOpener.getBoundingClientRect() : { right: window.innerWidth - 12, bottom: 12 };
            const edge = 12;
            const width = panel.offsetWidth;
            const left = Math.max(edge, Math.min(opener.right - width, window.innerWidth - width - edge));
            // Open below the button; when little room is left there, rise to fit.
            const wanted = Math.min(panel.scrollHeight + 2, window.innerHeight - 2 * edge);
            const top = Math.max(edge, Math.min(opener.bottom + 8, window.innerHeight - edge - Math.min(wanted, 520)));
            panel.style.left = left + 'px';
            panel.style.top = top + 'px';
            panel.style.maxHeight = (window.innerHeight - top - edge) + 'px';
        }

        function viewPanelBodyHtml(scope) {
            if (scope === 'board') return vpBoardHtml();
            if (scope === 'session') return vpSessionHtml();
            return vpListHtml();
        }

        function renderViewPanel() {
            const panel = document.getElementById('view-panel');
            if (!viewPanelScope || !panel) return;
            const scope = viewPanelScope;
            const body = panel.querySelector('.vp-body');
            const scrollTop = body ? body.scrollTop : 0;
            const focused = panel.contains(document.activeElement) ? document.activeElement.dataset.fid : null;
            panel.dataset.scope = scope;
            panel.innerHTML = '<div class="vp-head"><h2 class="vp-title">View <span>' + VIEW_SCOPES[scope].name + '</span></h2>'
                + '<button type="button" class="vp-done" data-do="close" data-fid="close">Done</button></div>'
                + '<div class="vp-body">' + viewPanelBodyHtml(scope) + '</div>';
            paintViewSamples(panel, scope);
            panel.querySelector('.vp-body').scrollTop = scrollTop;
            if (focused) {
                const again = [...panel.querySelectorAll('[data-fid]')].find(control => control.dataset.fid === focused);
                (again && !again.disabled ? again : panel).focus({ preventScroll: true });
            }
        }

        // Every row on screen follows a change to its set at once.
        function applyRowSet(scope) {
            if (scope === 'board') {
                renderPrBoard();
                return;
            }
            sessions.forEach((_data, id) => updateSessionHeaderContext(id));
            updateSidebarContent();
        }

        function onViewPanelClick(event) {
            const control = event.target.closest('[data-do]');
            if (!control || control.disabled) return;
            const arg = control.dataset.arg;
            const scope = viewPanelScope;
            switch (control.dataset.do) {
                case 'close': closeViewPanel(true); return;
                // The filters redraw the panel themselves.
                case 'window': setBoardWindow(arg === 'all' ? null : Number(arg)); return;
                case 'ended': setBoardLiveOnly(!prBoardViewPrefs.liveOnly); return;
                case 'computer': setBoardComputer(arg); return;
                case 'lead': setBoardLead(arg); return;
                case 'pin': if (sidebarPinned) closeSidebar(); else pinSidebar(); return;
                case 'group': setGroupBy(arg, scope); break;
                case 'sort': setProjectOrder(arg); break;
                case 'row': toggleViewSetItem(scope, arg); applyRowSet(scope); break;
                case 'text': adjustFontSize(Number(arg)); break;
                case 'spacing': setRowSpacing(arg); break;
                case 'side': setSidebarPosition(arg); break;
                case 'wrap': setTerminalWrap(arg === 'wrap'); break;
                case 'height': adjustTerminalHeight(Number(arg)); break;
                case 'drawer': setWidgetsExpanded(arg === 'open'); break;
                case 'section': if (arg === 'flow') toggleFlowScene(); else toggleSection(arg); break;
                case 'stat':
                    toggleViewSetItem('session', arg);
                    applySessionCardStatsVisibility();
                    sessions.forEach((_data, id) => fitSummaryAges(id));
                    break;
                default: return;
            }
            renderViewPanel();
        }

        // ── Account ────────────────────────────────────────────────────

        function accountPopoverOpen() {
            const popover = document.getElementById('settings-popover');
            return !!popover && popover.classList.contains('visible');
        }

        function toggleSettingsPopover() {
            if (accountPopoverOpen()) {
                closeSettingsPopover(true);
                return;
            }
            const popover = document.getElementById('settings-popover');
            const button = document.getElementById('settings-btn');
            closeChipMenu();
            closeViewPanel();
            popover.classList.add('visible');
            button.classList.add('active');
            button.setAttribute('aria-expanded', 'true');
            describeThisDevice();
            popover.focus({ preventScroll: true });
        }

        function closeSettingsPopover(returnFocus = false) {
            if (!accountPopoverOpen()) return;
            const button = document.getElementById('settings-btn');
            document.getElementById('settings-popover').classList.remove('visible');
            button.classList.remove('active');
            button.setAttribute('aria-expanded', 'false');
            if (returnFocus) button.focus({ preventScroll: true });
        }

        // "This device": where this browser is paired, and the server's build.
        let accountBuild = null;
        async function describeThisDevice() {
            const meta = document.getElementById('account-device-meta');
            if (!meta) return;
            const write = () => { meta.textContent = window.location.host + (accountBuild ? ' · Version ' + accountBuild : ''); };
            write();
            if (accountBuild) return;
            try {
                const data = await (await fetch(API_BASE + '/health')).json();
                accountBuild = data.build || null;
                write();
            } catch (e) {
                // The server name alone is still true.
            }
        }

        function confirmUnpair() {
            if (window.confirm('Unpair this browser? You will need a new pairing code to connect it again.')) clearPairing();
        }

        // ── Wiring ─────────────────────────────────────────────────────

        document.getElementById('filter-chips')?.addEventListener('click', event => {
            const chip = event.target.closest('.filter-chip');
            if (!chip) return;
            if (chipMenuName === chip.dataset.chip) closeChipMenu(true);
            else openChipMenu(chip.dataset.chip);
        });
        document.getElementById('chip-menu')?.addEventListener('click', event => {
            const item = event.target.closest('.chip-menu-item');
            if (item) pickChipMenuItem(item.dataset.value);
        });
        document.getElementById('chip-menu')?.addEventListener('keydown', chipMenuKey);
        document.getElementById('view-panel')?.addEventListener('click', onViewPanelClick);

        // A press anywhere else closes whatever is open. The buttons that
        // open these close them too, so their own presses are left to them.
        document.addEventListener('pointerdown', event => {
            const target = event.target.nodeType === 1 ? event.target : event.target.parentElement;
            if (!target) return;
            if (chipMenuName && !target.closest('#chip-menu, .filter-chip')) closeChipMenu();
            if (viewPanelScope && !target.closest('#view-panel, .view-btn')) closeViewPanel();
            if (accountPopoverOpen() && !target.closest('.settings-container')) closeSettingsPopover();
        }, true);

        // Escape closes the open menu or panel and hands focus back to its
        // button, and does not also close what is behind it. The listener is
        // on the capture phase, so it runs before the PR board's. The
        // fullscreen session's listener is on the capture phase too, so this
        // one must be registered first: index.ts loads this script before it.
        document.addEventListener('keydown', event => {
            if (event.key === 'Escape') {
                let close = null;
                if (chipMenuName) close = closeChipMenu;
                else if (viewPanelScope) close = closeViewPanel;
                else if (accountPopoverOpen()) close = closeSettingsPopover;
                if (!close) return;
                event.preventDefault();
                event.stopImmediatePropagation();
                close(true);
                return;
            }
            if (event.key !== 'Tab' || !viewPanelScope) return;
            // Tab stays inside the open panel until it is closed.
            const panel = document.getElementById('view-panel');
            const stops = [...panel.querySelectorAll('button:not([disabled])')].filter(control => control.getClientRects().length);
            if (!stops.length) return;
            const first = stops[0], last = stops[stops.length - 1];
            const inside = panel.contains(document.activeElement) && document.activeElement !== panel;
            if (event.shiftKey && (!inside || document.activeElement === first)) {
                event.preventDefault(); event.stopImmediatePropagation(); last.focus();
            } else if (!event.shiftKey && (!inside || document.activeElement === last)) {
                event.preventDefault(); event.stopImmediatePropagation(); first.focus();
            }
        }, true);

        window.addEventListener('resize', () => { closeChipMenu(); positionViewPanel(); });

        // Conductor has no filter bar: it leaves and returns as that view opens and closes.
        if (document.getElementById('conductor-view')) {
            new MutationObserver(() => renderFilterBar())
                .observe(document.getElementById('conductor-view'), { attributes: true, attributeFilter: ['hidden'] });
        }

        // The filter bar comes and goes with the view, so the header's height
        // changes: everything placed under the header follows it. The content
        // moves with the header, so the reading position is taken again
        // rather than scrolled back to where it was.
        if (typeof ResizeObserver === 'function' && document.querySelector('.header')) {
            new ResizeObserver(() => {
                syncHeaderHeight();
                viewedPageAnchor = capturePageScrollAnchor();
            }).observe(document.querySelector('.header'));
        }
`;
