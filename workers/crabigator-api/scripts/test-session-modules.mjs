import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { embedded, functions } from './dashboard-source.mjs';

for (const newest of ['prompts', 'completions']) {
    test(`summary keeps ago on ${newest} when only one label fits`, () => {
        const elements = Object.fromEntries(['prompts', 'completions'].map(key => [key, {
            dataset: { recencyTimestamp: key === newest ? '200' : '100' },
            textContent: '', getClientRects: () => [{}], setAttribute() {},
        }]));
        const row = { clientWidth: 140, get scrollWidth() {
            return 80 + Object.values(elements).reduce((sum, el) => sum + el.textContent.length * 6, 0);
        } };
        const fit = runInNewContext(functions(embedded('modules'), ['fitSummaryAges']) + '\nfitSummaryAges', {
            document: { querySelector: () => row, getElementById: id => elements[id.split('-')[1]] },
            formatElapsed: time => time === 200 ? '2m ago' : '3m ago',
        });
        fit('s');
        assert.equal(elements[newest].textContent, '2m ago');
        assert.equal(elements[newest === 'prompts' ? 'completions' : 'prompts'].textContent, '3m');
        row.clientWidth = 110;
        fit('s');
        assert.ok(Object.values(elements).every(el => !el.textContent.includes('ago')));
        row.clientWidth = 160;
        fit('s');
        assert.ok(Object.values(elements).every(el => el.textContent.includes('ago')), 'Resizing restores both labels');
    });
}

test('an explicitly opened drawer overrides a collapsed global default', () => {
    const storage = new Map();
    const context = { widgetsExpanded: false, localStorage: {
        getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value),
    } };
    const api = runInNewContext(functions(embedded('session-state'),
        ['getWidgetsCollapsedSessions', 'setWidgetsCollapsedSession', 'isWidgetsCollapsed'])
        + '\n({set:setWidgetsCollapsedSession, collapsed:isWidgetsCollapsed})', context);
    assert.equal(api.collapsed('s'), true);
    api.set('s', false);
    assert.equal(api.collapsed('s'), false);
    api.set('s', true);
    assert.equal(api.collapsed('s'), true);
});


test('roster refresh preserves live header metrics and explicit title changes', () => {
    const data = { title: 'Old title', generatedTitle: 'Old title', stats: { work_seconds: 7200, prompts: 3 }, git: { branch: 'old', files: ['a'] } };
    const update = runInNewContext(functions(embedded('session-state'), ['updateSessionHeader']) + '\nupdateSessionHeader', {
        sessions: new Map([['s', data]]),
        document: { getElementById: () => ({ querySelector: () => ({}) }) },
        updateSessionTitleHierarchy() {}, setSessionPlatform() {}, updateSessionSummary() {},
    });
    update({ id: 's', title: 'New title', stats: { prompts: 4 }, branch: '' });
    assert.equal(data.title, 'New title');
    assert.equal(data.generatedTitle, null, 'A cached generated title must not override a newer roster title');
    assert.equal(data.stats.work_seconds, 7200, 'Partial roster stats must not erase the live session duration');
    assert.equal(data.stats.prompts, 4);
    assert.equal(data.git.branch, '');
    assert.deepEqual(data.git.files, ['a']);
    update({ id: 's', title: 'Stale roster title', stats: { prompts: 2 } }, false);
    assert.equal(data.title, 'New title', 'Recreating the card must retain its current header');
    assert.equal(data.stats.prompts, 4);
});


test('a short roster recap keeps its turn number without duplicating history', () => {
    const entries = runInNewContext(functions(embedded('modules'), ['moduleRecapEntries']) + '\nmoduleRecapEntries');
    const history = [{ prompt_count: 9, generated_at: 1000, headline: 'Earlier' }, { prompt_count: 10, generated_at: 2000, headline: 'Latest' }];
    const rows = entries({ recapHistory: history, recap: { latest: { generated_at: 2000, headline: 'Latest' } } });
    assert.equal(rows.length, 2);
    assert.equal(rows[0].prompt_count, 10);
    const regenerated = entries({ recapHistory: history, recap: { latest: { prompt_count: 10, generated_at: 3000, headline: 'Updated' } } });
    assert.equal(regenerated.length, 2);
    assert.equal(regenerated[0].headline, 'Updated');
});

test('computer colors stay consistent across clients and hostname spellings', () => {
    const color = runInNewContext(functions(embedded('style'), ['computerColor']) + '\ncomputerColor', { TextEncoder });
    for (const [name, expected] of [['claybook', '#ffd700'], ['claybook-m4', '#ff8700'], ['claymac-studio', '#5f87ff'], ['claybook-tavus', '#87d7af'], [' ClayMac-Studio.LOCAL ', '#5f87ff'], ['ClayMac Studio', '#5f87ff'], ['ClayBook M4', '#ff8700'], ['ClayBook Tavus', '#87d7af']]) {
        assert.equal(color(name), expected, name);
    }
});

test('computer grouping combines a shared PR without losing either session', () => {
    const context = { groupByComputer: true };
    const group = runInNewContext(functions(embedded('pr-board'), ['prbDeviceLabel', 'prbSessionsByDevice']) + '\nprbSessionsByDevice', context);
    const sessions = [{ session_id: 'a', device_name: 'Studio.local' }, { session_id: 'b', device_name: 'Book' }];
    assert.equal(group(sessions).length, 2);
    context.groupByComputer = false;
    const combined = group(sessions);
    assert.equal(combined.length, 1);
    assert.equal(combined[0].machine, '');
    assert.deepEqual(combined[0].sessions, sessions);
    context.groupByComputer = true;
    assert.deepEqual(Array.from(group(sessions), row => row.machine), ['Studio', 'Book']);
});

test('focused sidebar keeps computer grouping and combines by recency when disabled', () => {
    const content = { innerHTML: '' };
    const rows = [{ id: 'a', deviceName: 'Studio', last_activity_at: 10 }, { id: 'b', deviceName: 'Book', last_activity_at: 20 }];
    const context = {
        allSessions: rows, sidebarLastRenderedHtml: null, groupByComputer: true, groupingMode: 'all',
        document: { getElementById: () => content }, isFocusedMode: () => true,
        getSidebarSessions: sessions => sessions, sidebarSessionView: s => s,
        renderSidebarSessionItem: s => `<row>${s.id}</row>`, updateSidebarActiveState() {},
        sortKeysByMostRecent: groups => [...groups.keys()].sort((a, b) => groups.get(b).mostRecentTime - groups.get(a).mostRecentTime),
        cleanDeviceName: s => s, escapeHtml: s => s, computerColor: () => '#5f87ff',
    };
    const render = runInNewContext(functions(embedded('sidebar'), ['updateSidebarContent']) + '\nupdateSidebarContent', context);
    render();
    assert.match(content.innerHTML, /sessions-device-section/);
    assert.ok(content.innerHTML.indexOf('Book') < content.innerHTML.indexOf('Studio'));
    context.groupByComputer = false;
    render();
    assert.equal(content.innerHTML, '<row>b</row><row>a</row>');
});
