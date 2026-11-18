import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { embedded, functions } from './dashboard-source.mjs';

// Run the shipping preference code against a browser with these stored values.
function viewPrefs(stored = {}, oldStats = {}) {
    const storage = new Map(Object.entries(stored));
    const api = runInNewContext(embedded('view-prefs') + `
        ({ viewSets, prBoardViewPrefs, toggleViewSetItem, deriveSessionStats, groupByChoice, groupByPrefs,
           viewBandIndex, terminalHeightLabel, bands: VIEW_BANDS })`, {
        localStorage: { getItem: key => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value) },
        sidebarVisibleStats: oldStats,
    });
    return { ...api, storage };
}
const plain = value => JSON.parse(JSON.stringify(value));

// The old "Visible stats" switches as one browser had left them: three stats
// hidden, and Context never ticked.
const OLD_STATS = {
    sessionTime: true, thinkingTime: false, prompts: true, promptRecency: true, completions: false,
    completionRecency: true, tools: true, compactions: false, context: false, flow: true,
};
// The switches as a browser that never opened them had them: every stat
// ticked except Context.
const FRESH = {
    sessionTime: true, thinkingTime: true, prompts: true, promptRecency: true, completions: true,
    completionRecency: true, tools: true, compactions: true, context: false, flow: true,
};

test('sets that were never saved start from the old switches, so each screen looks the same', () => {
    const { viewSets } = viewPrefs({ crabigatorPrBoardView: JSON.stringify({ detail: 1 }) }, OLD_STATS);
    assert.deepEqual(plain(viewSets.list), {
        // The List always drew the recap; git and changes are new and start off.
        recap: true, sessionTime: true, thinkingTime: false, promptRecency: true, completionRecency: true,
        tools: true, compactions: false, context: false, git: false, changes: false,
        prompts: true, completions: false,
    });
    assert.deepEqual(plain(viewSets.board), {
        // PR board rows showed the two ages and, with the old switch on, recaps.
        recap: true, sessionTime: false, thinkingTime: false, promptRecency: true, completionRecency: true,
        tools: false, compactions: false, context: false, git: false, changes: false,
    });
    assert.deepEqual(plain(viewSets.session), {
        sessionTime: true, thinkingTime: false, prompts: true, completions: false, tools: true,
        compactions: false, context: false, promptRecency: true, completionRecency: true,
    });
    assert.equal(viewPrefs({}, OLD_STATS).viewSets.board.recap, false, 'Recaps stay off when the old switch was off');
});

test('Context starts on where no stat was hidden, off on the PR board, and a saved choice sticks', () => {
    const context = old => {
        const { viewSets } = viewPrefs({}, old);
        return [viewSets.list.context, viewSets.board.context, viewSets.session.context];
    };
    assert.deepEqual(context(FRESH), [true, false, true],
        'A fresh browser: on in List rows and the stats strip, off on PR board rows');
    const stats = ['sessionTime', 'thinkingTime', 'prompts', 'promptRecency', 'completions', 'completionRecency', 'tools', 'compactions'];
    for (const hidden of stats) {
        assert.deepEqual(context({ ...FRESH, [hidden]: false }), [false, false, false], hidden + ' hidden, Context never ticked');
        assert.deepEqual(context({ ...FRESH, [hidden]: false, context: true }), [true, false, true], hidden + ' hidden, Context ticked');
    }
    assert.deepEqual(context({ ...FRESH, flow: false }), [true, false, true], 'The flow scene is not a stat on a row or the strip');
    assert.deepEqual(context({ sessionTime: true, thinkingTime: true }), [true, false, true],
        'A stat that never had a switch counts as shown');

    // Turning it off saves the set, and the next visit keeps it off.
    const off = viewPrefs({}, FRESH);
    off.toggleViewSetItem('list', 'context');
    let again = viewPrefs(Object.fromEntries(off.storage), FRESH).viewSets;
    assert.equal(again.list.context, false, 'A saved "off" is kept');
    assert.equal(again.session.context, true, 'The stats strip was never saved, so it still follows the rule');
    // Turning it on where it started off sticks too, whatever the old switches say later.
    const on = viewPrefs({}, FRESH);
    on.toggleViewSetItem('board', 'context');
    again = viewPrefs(Object.fromEntries(on.storage), { ...FRESH, tools: false }).viewSets;
    assert.equal(again.board.context, true, 'A saved "on" is kept');
    assert.equal(again.list.context, false);
});

test('a count with no old switch starts on in the stats strip', () => {
    const { deriveSessionStats } = viewPrefs();
    const set = deriveSessionStats({ sessionTime: false, tools: true });
    assert.equal(set.prompts, true);
    assert.equal(set.completions, true);
    assert.equal(set.sessionTime, false);
    assert.equal(set.tools, true);
});

// The List's filter, run against a roster of sessions that are not streaming.
function listFilter(prefs, rows, open = []) {
    const hour = 3600, now = 1000 * hour;
    const max = [1, 3, 6, 9, 12, 24].map(hours => hours * hour);
    const context = {
        prBoardViewPrefs: { maxAgeHours: 24, computer: '', ...prefs },
        allSessions: rows.map(row => ({ ...row, prompts_changed_at: row.prompted ? now - row.prompted * hour : 0,
            started_at: now - (row.started ?? 100) * hour })),
        sessions: new Map(rows.map(row => [row.id, {}])), expandedSessions: new Set(open),
        isFocusedMode: () => false, getSessionStartedTime: session => session.started_at || 0,
        prbBucketIndex: age => { const index = max.findIndex(limit => age < limit); return index === -1 ? max.length : index; },
    };
    const inFilter = runInNewContext(
        functions(embedded('view-prefs'), ['viewBandIndex', 'listSessionFreshness', 'sessionInFilter'])
        + functions(embedded('pr-board'), ['prbDeviceLabel'])
        + '\nconst VIEW_BANDS = [1, 3, 6, 9, 12, 24, null].map(hours => ({ hours }));\nsessionInFilter', context);
    return rows.filter(row => inFilter(row.id, now)).map(row => row.id);
}

test('the List keeps a session that waits on you, or whose terminal is open, however old it is', () => {
    const rows = [
        { id: 'fresh', state: 'complete', prompted: 0.5 },
        { id: 'old', state: 'complete', prompted: 30 },
        { id: 'never-prompted', state: 'ready', started: 2 },
        { id: 'old-question', state: 'question', prompted: 72 },
        { id: 'old-permission', state: 'permission', prompted: 72 },
        { id: 'old-thinking', state: 'thinking', prompted: 72 },
        { id: 'old-open', state: 'complete', prompted: 72 },
    ];
    const waiting = ['old-question', 'old-permission', 'old-thinking', 'old-open'];
    assert.deepEqual(listFilter({ maxAgeHours: 1 }, rows, ['old-open']), ['fresh', ...waiting]);
    assert.deepEqual(listFilter({ maxAgeHours: 3 }, rows, ['old-open']), ['fresh', 'never-prompted', ...waiting],
        'A session with no prompt yet counts from its start');
    assert.deepEqual(listFilter({ maxAgeHours: null }, rows), rows.map(row => row.id), 'All ages hides nothing');
});

test('the computer filter matches a name however its spaces and .local are written', () => {
    const rows = [
        { id: 'studio', state: 'complete', prompted: 1, device_name: ' ClayMacStudio.local ' },
        { id: 'book', state: 'complete', prompted: 1, device_name: 'ClayBook' },
        { id: 'book-question', state: 'question', prompted: 90, device_name: 'ClayBook.local' },
        { id: 'unnamed', state: 'complete', prompted: 1 },
        { id: 'studio-open', state: 'complete', prompted: 90, device_name: 'ClayMacStudio' },
    ];
    assert.deepEqual(listFilter({ computer: 'ClayMacStudio' }, rows), ['studio']);
    assert.deepEqual(listFilter({ computer: 'ClayBook' }, rows, ['studio-open']), ['book', 'book-question', 'studio-open'],
        'An open terminal stays whichever computer is chosen');
    assert.deepEqual(listFilter({ computer: '' }, rows), ['studio', 'book', 'book-question', 'unnamed']);
});

test('a saved set wins outright, and one change saves only that set', () => {
    const { viewSets, toggleViewSetItem, storage } = viewPrefs({
        crabigatorPrBoardView: JSON.stringify({ detail: 1 }),
        'crabigator-board-row': JSON.stringify({ recap: false, git: true }),
    }, OLD_STATS);
    assert.equal(viewSets.board.recap, false, 'The old recap switch is no longer consulted');
    assert.equal(viewSets.board.git, true);
    assert.equal(viewSets.board.promptRecency, false, 'An item missing from a saved set is off');
    toggleViewSetItem('list', 'git');
    assert.equal(JSON.parse(storage.get('crabigator-list-row')).git, true);
    assert.equal(JSON.parse(storage.get('crabigator-list-row')).sessionTime, true, 'The derived choices are saved with it');
    assert.equal(storage.has('crabigator-session-stats'), false);
    assert.equal(JSON.parse(storage.get('crabigator-board-row')).recap, false);
});

test('Group by maps onto the two stored choices and back', () => {
    const { groupByChoice, groupByPrefs } = viewPrefs();
    const cases = [
        ['none', false, 'all'], ['computer', true, 'all'], ['project', false, 'project'], ['both', true, 'project'],
    ];
    for (const [choice, byComputer, projectMode] of cases) {
        assert.equal(groupByChoice(byComputer, projectMode), choice);
        assert.deepEqual(plain(groupByPrefs(choice)), { byComputer, projectMode });
    }
});

test('the time window keeps its stored hours, and anything else means every age', () => {
    const { viewBandIndex, bands } = viewPrefs();
    assert.deepEqual(Array.from(bands, band => band.label), ['1h', '3h', '6h', '9h', '12h', '24h', 'All']);
    assert.equal(viewBandIndex(1), 0);
    assert.equal(viewBandIndex(24), 5);
    for (const all of [null, 0, undefined, 48]) assert.equal(viewBandIndex(all), 6);
    assert.equal(viewPrefs().prBoardViewPrefs.maxAgeHours, 24, 'The board opens on 24 hours');
    const saved = viewPrefs({ crabigatorPrBoardView: JSON.stringify({ maxAgeHours: 0, liveOnly: true, view: 'prs' }) }).prBoardViewPrefs;
    assert.equal(saved.maxAgeHours, null);
    assert.equal(saved.liveOnly, true);
    assert.equal(saved.view, 'prs');
    assert.equal(saved.computer, '', 'Every computer until one is chosen');
});

test('each stored terminal height has a name beside its pixels', () => {
    const { terminalHeightLabel } = viewPrefs();
    assert.equal(terminalHeightLabel(350), 'Medium · 350 px');
    assert.equal(terminalHeightLabel(700), 'Tallest · 700 px');
    assert.equal(terminalHeightLabel(123), '123 px');
});

// The row lines, with the page's formatters stood in.
function rowLines() {
    return runInNewContext(embedded('session-row').split('        // A live session')[0] + '\n({ lines: sessionRowLinesHtml })', {
        escapeHtml: text => String(text).replace(/</g, '&lt;'),
        formatDuration: seconds => seconds + 's',
        formatElapsed: timestamp => 'at ' + timestamp,
    }).lines;
}
const ALL_ON = Object.fromEntries(['recap', 'sessionTime', 'thinkingTime', 'promptRecency', 'completionRecency',
    'tools', 'compactions', 'context', 'git', 'changes', 'prompts', 'completions'].map(key => [key, true]));
const FACTS = {
    recap: 'Mapped <every> setting', sessionSeconds: 8040, thinkingSeconds: 2280, prompts: 14, completions: 13,
    promptAt: 100, completionAt: 200, tools: 212, compactions: 3,
    context: { label: '186k / 1M · 19%', color: '#5fafff' },
    branch: 'main', uncommitted: 3, additions: 212, deletions: 48,
};
const text = html => html.replace(/<span class="row-sr">[^<]*<\/span>/g, '').replace(/<\/span><span class="row-line/g, '</span>\n<span class="row-line')
    .replace(/<[^>]+>/g, ' ').replace(/[ \t]+/g, ' ').split('\n').map(line => line.trim());

test('a row draws recap, times, counts, git and changes, in that order', () => {
    const html = rowLines()(FACTS, ALL_ON);
    assert.deepEqual(text(html), [
        'Mapped &lt;every> setting',
        '◉ 8040s ◐ 2280s ⟩ 14 at 100 ⋖ 13 at 200',
        '⚒︎ 212 ⊜ 3 ◔ 186k / 1M · 19%',
        '⎇ main 3 files not committed',
        'Δ +212 −48',
    ]);
    assert.match(html, /color:#5fafff/, 'Context wears the colour its fill level earns');
});

test('a row shows only the chosen items, and nothing for a stat with no data', () => {
    const lines = rowLines();
    assert.deepEqual(text(lines(FACTS, { sessionTime: true, context: true })), ['◉ 8040s', '◔ 186k / 1M · 19%']);
    assert.deepEqual(text(lines(FACTS, { promptRecency: true, completions: true })), ['⟩ at 100 ⋖ 13']);
    const empty = { ...FACTS, recap: '', sessionSeconds: null, thinkingSeconds: 0, prompts: null, completions: null,
        promptAt: 0, completionAt: 0, tools: 0, compactions: 0, context: null, branch: '', uncommitted: 0, additions: 0, deletions: 0 };
    assert.equal(lines(empty, ALL_ON), '', 'No dashes and no empty lines');
    assert.deepEqual(text(lines({ ...empty, uncommitted: 1 }, ALL_ON)), ['1 file not committed']);
    assert.deepEqual(text(lines({ ...empty, branch: 'main' }, ALL_ON)), ['⎇ main'], 'A clean branch names no files');
    assert.deepEqual(text(lines({ ...empty, deletions: 5 }, ALL_ON)), ['Δ +0 −5']);
});

test('a live session counts from its start, and an ended one keeps its reported total', () => {
    const seconds = runInNewContext(functions(embedded('session-row'), ['rowSessionSeconds']) + '\nrowSessionSeconds');
    assert.equal(seconds(true, 1000, 0, null, 1600), 600, 'The board has no total for a session that is still running');
    assert.equal(seconds(true, 1000, 0, 90, 1600), 600);
    assert.equal(seconds(false, 1000, 1500, 450, 1600), 450);
    assert.equal(seconds(false, 1000, 1500, null, 1600), 500, 'An ended session without a total runs to its end');
    assert.equal(seconds(true, 0, 0, null, 1600), null);
});
