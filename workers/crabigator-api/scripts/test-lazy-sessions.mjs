import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import ts from 'typescript';

// Exercise the shipping connection lifecycle with a controllable network and clock.
const embedded = name => runInNewContext(readFileSync(new URL('../src/dashboard/js/' + name + '.ts', import.meta.url), 'utf8').replace(/^export /m, '') + '\n' + name + 'Js');
const source = ts.createSourceFile('dashboard.js', embedded('events') + embedded('prompt'), ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
const names = new Set(['connectToSession', 'closeSessionSocket', 'cancelSessionReconnect', 'scheduleSessionReconnect']);
const functions = source.statements.filter(s => ts.isFunctionDeclaration(s) && names.has(s.name?.text)).map(s => s.getText(source)).join('\n');

test('collapsed sessions never connect, and closing cancels queued reconnects and screen paints', () => {
    const sockets = [], timers = new Map(), pendingScreenUpdates = new Map(), screenUpdateTimers = new Map();
    class Socket {
        static OPEN = 1; static CONNECTING = 0;
        readyState = 0;
        constructor(url) { this.url = url; sockets.push(this); }
        close() { this.readyState = 3; this.onclose?.(); }
    }
    const sessions = new Map(Array.from({ length: 30 }, (_, i) => ['session-' + i, { sessionId: 'session-' + i }]));
    const expandedSessions = new Set();
    const context = { sessions, expandedSessions, WebSocket: Socket, sessionReconnectTimers: new Map(), pendingScreenUpdates, screenUpdateTimers,
        document: { visibilityState: 'visible' }, console: { log() {} }, getWebSocketUrl: p => p, isDeploying: false,
        setTimeout: fn => { const id = Symbol(); timers.set(id, fn); return id; }, clearTimeout: id => timers.delete(id) };
    const api = runInNewContext(functions + '\n({ connectToSession, closeSessionSocket })', context);
    for (const id of sessions.keys()) api.connectToSession(id);
    assert.equal(sockets.length, 0);
    for (const id of ['session-0', 'session-1']) { expandedSessions.add(id); api.connectToSession(id); }
    assert.equal(sockets.length, 2);
    sockets[0].onclose(); // A dropped connection queued a reconnect just before collapse.
    const paint = context.setTimeout(() => assert.fail('Closed terminal must not paint'));
    pendingScreenUpdates.set('session-0', 'late output'); screenUpdateTimers.set('session-0', paint);
    expandedSessions.delete('session-0'); api.closeSessionSocket(sessions.get('session-0'));
    assert.equal(timers.size, 0);
    assert.equal(pendingScreenUpdates.size, 0);
    for (const id of sessions.keys()) api.connectToSession(id); // tab visibility resume
    assert.equal(sockets.length, 3, 'Only the remaining expanded session reconnects');
    assert.equal(sockets[2].url, '/sessions/session-1/events');
});
