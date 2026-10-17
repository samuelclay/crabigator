import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import ts from 'typescript';

const embedded = runInNewContext(readFileSync(new URL('../src/dashboard/js/style.ts', import.meta.url), 'utf8').replace(/^export /m, '') + '\nstyleJs');
const source = ts.createSourceFile('style.js', embedded, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
const names = new Set(['capturePageScrollAnchor', 'restorePageScrollAnchor', 'capturePageScrollRestorer', 'preservePageScroll']);
const functions = source.statements.filter(s => ts.isFunctionDeclaration(s) && names.has(s.name?.text)).map(s => s.getText(source)).join('\n');

for (const board of [false, true]) {
    test(`${board ? 'board' : 'list'} updates preserve the reading position without interrupting the next gesture`, () => {
        const writes = [], frames = [];
        const scrollEl = { isConnected: true, scrollTop: 500, scrollLeft: 0, scrollHeight: 2000, clientHeight: 600,
            getBoundingClientRect: () => ({ top: board ? 180 : -500 }),
            scrollTo: ({ top, left, behavior }) => { writes.push({ top, behavior }); scrollEl.scrollTop = top; scrollEl.scrollLeft = left; },
            querySelectorAll: () => [card] };
        const page = board ? { scrollTop: 0 } : scrollEl;
        let cardPosition = 480;
        const card = { id: board ? '' : 'session-fixture', dataset: { key: 'session:fixture' },
            getBoundingClientRect: () => ({ top: cardPosition - scrollEl.scrollTop + (board ? 180 : 0), bottom: cardPosition - scrollEl.scrollTop + 800, height: 800 }) };
        const context = { prBoardVisible: board, CSS: { escape: s => s }, getScrollAnchorReferenceY: () => 79,
            document: { scrollingElement: page, documentElement: page, getElementById: id => id === 'prb-body' ? scrollEl : null, querySelector: () => card },
            requestAnimationFrame: cb => frames.push(cb) };
        const { preservePageScroll } = runInNewContext(functions + '\n({ preservePageScroll })', context);
        preservePageScroll(() => {});
        assert.equal(writes.length, 0, 'Unchanged updates must not cancel touch momentum');
        preservePageScroll(() => { cardPosition += 60; });
        assert.equal(scrollEl.scrollTop, 560, 'The visible row stays at the same viewport offset');
        assert.equal(writes[0].behavior, 'instant', 'Restoration must never animate');
        if (board) assert.equal(page.scrollTop, 0, 'Board updates must not scroll the outer page');
        scrollEl.scrollTop = 610; // A swipe immediately after the update.
        for (const callback of frames) callback();
        assert.equal(scrollEl.scrollTop, 610, 'No deferred restore may undo the user swipe');
        assert.equal(frames.length, 0);
    });
}

test('opening a board session scrolls the board without moving the page', () => {
    const sidebar = runInNewContext(readFileSync(new URL('../src/dashboard/js/sidebar.ts', import.meta.url), 'utf8').replace(/^export /m, '') + '\nsidebarJs');
    const source = ts.createSourceFile('sidebar.js', sidebar, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    const names = new Set(['scrollSessionCardIntoView']);
    const functions = source.statements.filter(s => ts.isFunctionDeclaration(s) && names.has(s.name?.text)).map(s => s.getText(source)).join('\n');
    const writes = [];
    const board = { scrollTop: 300, getBoundingClientRect: () => ({ top: 180, bottom: 780 }),
        scrollTo: options => { writes.push(options); board.scrollTop = options.top; } };
    const page = { scrollTop: 40, scrollTo: () => assert.fail('Board navigation must not scroll the page') };
    const card = { closest: () => board,
        getBoundingClientRect: () => ({ top: 900 - board.scrollTop, bottom: 1400 - board.scrollTop }) };
    const context = { document: { scrollingElement: page }, window: { innerHeight: 900 }, getScrollAnchorReferenceY: () => 79 };
    const { scrollSessionCardIntoView } = runInNewContext(functions + '\n({ scrollSessionCardIntoView })', context);
    scrollSessionCardIntoView(card, true);
    assert.equal(board.scrollTop, 708);
    assert.equal(writes[0].behavior, 'smooth', 'Explicit navigation can animate');
    scrollSessionCardIntoView(card, false);
    assert.equal(writes[1].behavior, 'instant');
    assert.equal(page.scrollTop, 40);
});
