import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';

function embedded(name) {
    const source = readFileSync(new URL(`../src/dashboard/js/${name}.ts`, import.meta.url), 'utf8');
    return runInNewContext(source.replace(/^export /m, '') + `\n${name}Js`);
}

function viewer() {
    const elements = new Map();
    for (const name of ['scrollback', 'separator', 'terminal']) {
        const classes = new Set();
        elements.set(`${name}-test`, {
            innerHTML: '', scrollTop: 0, scrollHeight: 0,
            classList: { add: key => classes.add(key), remove: key => classes.delete(key) },
            insertAdjacentHTML(where, html) {
                this.innerHTML = where === 'afterbegin' ? html + this.innerHTML : this.innerHTML + html;
            },
        });
    }
    const session = {};
    const context = {
        sessions: new Map([['test', session]]), activeTerminalId: 'test', SCROLLBACK_CHUNK_SIZE: 3,
        document: { getElementById: id => elements.get(id) },
    };
    const api = runInNewContext(embedded('ansi') + embedded('scrollback') + '\n({scrollbackLines, renderScrollback, appendScrollback, loadMoreScrollback})', context);
    return { api, session, html: () => elements.get('scrollback-test').innerHTML };
}

test('live history updates render the same rows as a full snapshot, including blank rows and colors', () => {
    const live = viewer();
    const chunks = ['\x1b[31mfirst\x1b[0m\n\n', '\x1b[32msecond\x1b[0m\n'];
    live.api.renderScrollback('test', []);
    for (const chunk of chunks) live.api.appendScrollback('test', chunk);
    const joined = viewer();
    joined.api.renderScrollback('test', joined.api.scrollbackLines(chunks.join('')));
    assert.equal(live.html(), joined.html());
    assert.equal(live.session.scrollbackBuffer.length, 3);
    assert.equal((live.html().match(/class="line"/g) || []).length, 3);
});

test('loading earlier rows and replacing history never duplicates a row or leaves stale content', () => {
    const v = viewer();
    const content = 'one\ntwo\nthree\nfour\nfive\n';
    v.api.renderScrollback('test', v.api.scrollbackLines(content));
    assert.equal(v.session.scrollbackRendered, 3);
    v.api.loadMoreScrollback('test');
    assert.equal(v.html(), ['one', 'two', 'three', 'four', 'five'].map(row => `<div class="line">${row}</div>`).join(''));
    v.api.renderScrollback('test', v.api.scrollbackLines(content));
    assert.equal(v.session.scrollbackRendered, 5, 'identical reconnect keeps the loaded reading position');
    v.api.renderScrollback('test', v.api.scrollbackLines('replacement\n'));
    assert.equal(v.html(), '<div class="line">replacement</div>');
    v.api.renderScrollback('test', []);
    assert.equal(v.html(), '');
    v.api.appendScrollback('test', 'new\n');
    assert.equal(v.html(), '<div class="line">new</div>');
});

test('an isolated blank terminal row is preserved', () => {
    const v = viewer();
    v.api.appendScrollback('test', '\n');
    assert.equal(v.html(), '<div class="line"></div>');
});
