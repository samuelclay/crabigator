import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { embedded, functions, terminalUpdates } from './dashboard-source.mjs';

test('focusing a terminal keeps the board live, while editing a draft defers row changes', () => {
    let updates = 0, editing = false;
    const context = {
        document: { activeElement: { closest: () => true, matches: () => editing, isContentEditable: false } },
        preservePageScroll: update => update(), preserveBoardTerminals: update => update(),
        renderPrBoardContents: () => updates++,
    };
    const render = runInNewContext(functions(embedded('pr-board'), ['renderPrBoardBody']) + '\nrenderPrBoardBody', context);
    render();
    assert.equal(updates, 1, 'A focused terminal must still receive board updates');
    editing = true;
    render();
    assert.equal(updates, 1, 'Typing must not move the composer');
});

test('received history and the newest screen paint together, with one scroll restoration', () => {
    const callbacks = [], paints = [];
    let restorations = 0;
    const screen = { set innerHTML(value) { paints.push(['screen', value]); } };
    const context = {
        document: { getElementById: id => id === 'screen-s' ? screen : null },
        setTimeout: callback => callbacks.push(callback),
        preserveTerminalScroll: (_id, update) => { update(); restorations++; },
        renderScrollback: (_id, rows) => paints.push(['history', Array.from(rows)]),
        appendScrollback: (_id, text) => paints.push(['append', text]),
        ansiToHtml: text => text,
        scrollbackLines: text => text.trimEnd().split('\n'),
    };
    const schedule = runInNewContext(terminalUpdates + '\nscheduleTerminalUpdate', context);
    schedule('s', { type: 'scrollback', diff: 'already in snapshot\n' });
    schedule('s', { type: 'screen', content: 'old screen' });
    schedule('s', { type: 'scrollback_history', content: 'full history\n' });
    schedule('s', { type: 'scrollback', diff: 'new row\n' });
    schedule('s', { type: 'screen', content: 'latest screen' });
    assert.deepEqual(paints, [], 'Neither half of the terminal may paint early');
    assert.equal(callbacks.length, 1);
    callbacks[0]();
    assert.deepEqual(paints, [['history', ['full history']], ['append', 'new row\n'], ['screen', 'latest screen']]);
    assert.equal(restorations, 1);
});

for (const gesture of ['wheel', 'touch', 'keyboard', 'scrollbar']) {
    test(`${gesture} stops following before the next output, even just one pixel above the bottom`, () => {
        const listeners = new Map();
        const data = { pinned: true, lastScrollTop: 800 };
        const terminal = { scrollTop: 800, scrollHeight: 1000, clientHeight: 200,
            addEventListener: (name, handler) => listeners.set(name, handler) };
        const context = { sessions: new Map([['s', data]]), activeTerminalId: 's', isChangingFontSize: false,
            updatePinButton() {}, loadMoreScrollback() {}, activateTerminalScroll() {} };
        const bind = runInNewContext(functions(embedded('terminal'), ['bindTerminalScroll']) + '\nbindTerminalScroll', context);
        bind('s', terminal);
        if (gesture === 'wheel') listeners.get('wheel')({ deltaY: -1 });
        if (gesture === 'touch') {
            listeners.get('touchstart')({ touches: [{ clientY: 100 }] });
            listeners.get('touchmove')({ touches: [{ clientY: 101 }] });
        }
        if (gesture === 'keyboard') listeners.get('keydown')({ key: 'ArrowUp' });
        if (gesture !== 'scrollbar') assert.equal(data.pinned, false, 'Input intent must win before a scroll event or queued paint');
        terminal.scrollTop = 799;
        listeners.get('scroll')();
        assert.equal(data.pinned, false);
        terminal.scrollTop = 800;
        listeners.get('scroll')();
        assert.equal(data.pinned, true, 'Deliberately returning to the bottom resumes following');
        terminal.scrollTop = 810;
        listeners.get('scroll')();
        terminal.scrollTop = 800;
        listeners.get('scroll')();
        assert.equal(data.pinned, true, 'Elastic overscroll returning to the bottom is not an upward reading gesture');
    });
}


test('ended sessions never show a working spinner or demand an answer', () => {
    const { state, aggregate, icon } = runInNewContext(functions(embedded('pr-board'),
        ['prbSessionState', 'prbAggregateState', 'prbStateIcon']) + '\n({state:prbSessionState, aggregate:prbAggregateState, icon:prbStateIcon})',
        { PRB_C: { gray: '#888' } });
    for (const oldState of ['thinking', 'question', 'permission']) {
        const ended = { active: false, state: oldState };
        assert.equal(state(ended), 'ended');
        assert.match(icon(state(ended)), /Session disconnected or ended/);
        assert.equal(aggregate([ended, { active: true, state: 'complete' }]), 'complete');
    }
});


test('fullscreen preserves the reading position relative to the terminal viewport', () => {
    let viewportTop = 300;
    const data = { pinned: false };
    const terminal = { scrollTop: 500, scrollHeight: 2000, clientHeight: 400,
        getBoundingClientRect: () => ({ top: viewportTop }) };
    const anchor = { isConnected: true,
        getBoundingClientRect: () => ({ top: viewportTop + 510 - terminal.scrollTop,
            bottom: viewportTop + 530 - terminal.scrollTop }) };
    const preserve = runInNewContext(functions(embedded('terminal'), ['preserveTerminalScroll']) + '\npreserveTerminalScroll', {
        sessions: new Map([['s', data]]),
        document: { getElementById: id => id === 'terminal-s' ? terminal : { children: [anchor] } },
    });
    preserve('s', () => { viewportTop = 50; terminal.clientHeight = 800; });
    assert.equal(terminal.scrollTop, 500, 'Entering fullscreen keeps the same history row in view');
    preserve('s', () => { viewportTop = 300; terminal.clientHeight = 400; });
    assert.equal(terminal.scrollTop, 500, 'Exiting fullscreen keeps the same history row in view');
    data.pinned = true;
    preserve('s', () => { terminal.clientHeight = 300; });
    assert.equal(terminal.scrollTop, 1700, 'Pinned output stays at the bottom when modules reduce terminal height');
});
