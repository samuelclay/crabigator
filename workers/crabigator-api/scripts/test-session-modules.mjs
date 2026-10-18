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
