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
