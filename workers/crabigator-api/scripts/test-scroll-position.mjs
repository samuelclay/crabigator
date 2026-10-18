import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import ts from 'typescript';

const embedded = runInNewContext(readFileSync(new URL('../src/dashboard/js/style.ts', import.meta.url), 'utf8').replace(/^export /m, '') + '\nstyleJs');
const source = ts.createSourceFile('style.js', embedded, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
const names = new Set(['pageScrollSurface', 'selectViewedSession', 'restoreViewedSessionPosition', 'noteViewedPageScroll', 'capturePageScrollAnchor', 'restorePageScrollAnchor', 'capturePageScrollRestorer', 'preservePageScroll']);
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
        const card = { closest: () => null, id: board ? '' : 'session-fixture', dataset: { key: 'session:fixture' },
            getBoundingClientRect: () => ({ top: cardPosition - scrollEl.scrollTop + (board ? 180 : 0), bottom: cardPosition - scrollEl.scrollTop + 800, height: 800 }) };
        const context = { viewedSessionId: null, viewedPageAnchor: null, pageScrollRevision: 0, restoredPageTop: null, window: { innerHeight: 900 }, prBoardVisible: board, CSS: { escape: s => s }, getScrollAnchorReferenceY: () => 79,
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
    const card = { id: 'session-fixture', closest: () => board,
        getBoundingClientRect: () => ({ top: 900 - board.scrollTop, bottom: 1400 - board.scrollTop }) };
    const context = { viewedSessionId: null, viewedAnchorPart: 'terminal', viewedPageAnchor: null, pageScrollRevision: 0, document: { scrollingElement: page }, window: { innerHeight: 900 }, getScrollAnchorReferenceY: () => 79 };
    const { scrollSessionCardIntoView } = runInNewContext(functions + '\n({ scrollSessionCardIntoView })', context);
    scrollSessionCardIntoView(card, true);
    assert.equal(board.scrollTop, 708);
    assert.equal(writes[0].behavior, 'smooth', 'Explicit navigation can animate');
    scrollSessionCardIntoView(card, false);
    assert.equal(writes[1].behavior, 'instant');
    assert.equal(page.scrollTop, 40);
});


for (const board of [false, true]) {
    test(`${board ? 'board' : 'page'} follows the selected terminal through later layout changes and yields to scrolling`, () => {
        let precedingHeight = 500, headerHeight = 60, groupCollapsed = false;
        const writes = [];
        const surfaceTop = board ? 180 : 0;
        const surface = { isConnected: true, scrollTop: 400, scrollLeft: 0, scrollHeight: 4000, clientHeight: 700,
            getBoundingClientRect: () => ({ top: surfaceTop, bottom: surfaceTop + 700 }),
            contains: () => true,
            scrollTo: ({top}) => { surface.scrollTop = top; writes.push(top); } };
        const other = { closest: () => null, id: 'session-other', getBoundingClientRect: () => ({ top: surfaceTop - surface.scrollTop, bottom: precedingHeight - surface.scrollTop + surfaceTop, height: precedingHeight }) };
        const watched = { closest: () => groupCollapsed ? {} : null, id: 'session-watched', getBoundingClientRect: () => ({ top: precedingHeight - surface.scrollTop + surfaceTop, bottom: precedingHeight + 1000 - surface.scrollTop + surfaceTop, height: 1000 }) };
        const terminal = { closest: () => groupCollapsed ? {} : null, id: 'terminal-watched', getBoundingClientRect: () => ({ top: watched.getBoundingClientRect().top + headerHeight, height: 500 }) };
        const nodes = { 'session-other': other, 'session-watched': watched, 'terminal-watched': terminal, 'prb-body': surface };
        surface.querySelectorAll = () => [other, watched];
        const page = board ? { scrollTop: 0 } : surface;
        const context = { viewedSessionId: null, viewedPageAnchor: null, viewedAnchorPart: 'terminal', pageScrollRevision: 0, restoredPageTop: null,
            prBoardVisible: board, CSS: { escape: s => s }, window: { innerHeight: 900 }, getScrollAnchorReferenceY: () => 79,
            document: { scrollingElement: page, documentElement: page, body: { classList: { contains: () => false } },
                getElementById: id => nodes[id], querySelector: selector => nodes[selector.slice(1)] } };
        const api = runInNewContext(functions + '\n({selectViewedSession,restoreViewedSessionPosition,noteViewedPageScroll,capturePageScrollRestorer})', context);
        api.selectViewedSession('watched');
        const initialY = terminal.getBoundingClientRect().top;
        // ResizeObserver/mutation phases occur after the original roster update.
        for (const growth of [80, 30, -45]) {
            precedingHeight += growth;
            api.restoreViewedSessionPosition();
            assert.equal(terminal.getBoundingClientRect().top, initialY);
        }
        headerHeight += 25;
        api.restoreViewedSessionPosition();
        assert.equal(terminal.getBoundingClientRect().top, initialY, 'A wrapping title must not move the terminal either');
        const count = writes.length;
        api.restoreViewedSessionPosition();
        assert.equal(writes.length, count, 'Unchanged frames do not issue scroll commands');
        const staleRestore = api.capturePageScrollRestorer();
        surface.scrollTop += 100;
        api.noteViewedPageScroll({target: surface});
        const userY = terminal.getBoundingClientRect().top;
        staleRestore();
        assert.equal(terminal.getBoundingClientRect().top, userY, 'An older restoration must not undo a new swipe');
        precedingHeight += 60;
        api.restoreViewedSessionPosition();
        assert.equal(terminal.getBoundingClientRect().top, userY, 'Later layout corrections use the new reading position');
        if (board) assert.equal(page.scrollTop, 0);
        groupCollapsed = true;
        const beforeCollapse = writes.length;
        api.restoreViewedSessionPosition();
        assert.equal(context.viewedSessionId, null, 'Collapsed groups release their hidden terminal');
        assert.equal(writes.length, beforeCollapse, 'Hiding a group must not scroll to its clipped terminal');
    });
}

test('keyboard shortcuts fit the visible space without scrolling the dashboard', () => {
    const code = runInNewContext(readFileSync(new URL('../src/dashboard/js/keyboard.ts', import.meta.url), 'utf8').replace(/^export /m, '') + '\nkeyboardJs');
    const ast = ts.createSourceFile('keyboard.js', code, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    const toggle = ast.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'toggleKeyboardPopover').getText(ast);
    const classes = new Set();
    const menu = { style: {}, classList: { contains: name => classes.has(name), add: name => classes.add(name), remove: name => classes.delete(name), toggle: (name, on) => on ? classes.add(name) : classes.delete(name) },
        scrollIntoView: () => assert.fail('Opening shortcuts must not scroll the page') };
    let bodyBounds = null;
    const container = { querySelector: () => menu, closest: selector => selector === '.session-body-inner' && bodyBounds ? { getBoundingClientRect: () => bodyBounds } : null, getBoundingClientRect: () => ({ top: 300, bottom: 336, left: 20 }) };
    const open = runInNewContext(toggle + '\ntoggleKeyboardPopover', {
        document: { querySelectorAll: () => [], querySelector: () => container },
        window: { innerHeight: 800, innerWidth: 1200 }, getScrollAnchorReferenceY: () => 79,
        requestAnimationFrame: () => assert.fail('Opening shortcuts must not schedule a later page scroll'),
    });
    open('s');
    assert.ok(classes.has('visible'));
    assert.ok(classes.has('drop-down'));
    assert.equal(menu.style.maxHeight, '452px', 'An oversized menu scrolls within the available space');
    open('s'); // Close before changing the clipping bounds.
    bodyBounds = { top: 100, bottom: 350 };
    open('s');
    assert.ok(!classes.has('drop-down'), 'A menu must not open below the composer into a clipped card edge');
    assert.equal(menu.style.maxHeight, '188px');
});
