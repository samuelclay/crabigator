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
            classList: { add: key => classes.add(key), remove: key => classes.delete(key), toggle: (key, on) => on ? classes.add(key) : classes.delete(key) },
            get firstElementChild() { return { remove: () => { this.innerHTML = this.innerHTML.replace(/^<div class="line">[\s\S]*?<\/div>/, ''); } }; },
            insertAdjacentHTML(where, html) {
                this.innerHTML = where === 'afterbegin' ? html + this.innerHTML : this.innerHTML + html;
            },
        });
    }
    const session = {};
    const context = {
        sessions: new Map([['test', session]]), activeTerminalId: 'test', SCROLLBACK_CHUNK_SIZE: 3,
        document: { getElementById: id => elements.get(id) },
        preserveTerminalScroll: (_id, update) => update(), // Geometry is covered by the browser suite.
    };
    const api = runInNewContext(embedded('ansi') + embedded('scrollback') + '\n({scrollbackLines, renderScrollback, appendScrollback, loadMoreScrollback, flushScrollback})', context);
    return { api, session, context, html: () => elements.get('scrollback-test').innerHTML };
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

test('a reconnect with new output retains older loaded rows and handles server trimming', () => {
    const v = viewer();
    const rows = ['one', 'two', 'three', 'four', 'five'];
    v.api.renderScrollback('test', rows);
    v.api.loadMoreScrollback('test');
    v.api.renderScrollback('test', [...rows, 'six']);
    assert.equal(v.session.scrollbackRendered, 6);
    assert.match(v.html(), /one/);
    assert.match(v.html(), /six/);
    v.api.renderScrollback('test', ['three', 'four', 'five', 'six', 'seven']);
    assert.equal(v.session.scrollbackRendered, 5);
    assert.equal(v.html(), ['three', 'four', 'five', 'six', 'seven'].map(row => `<div class="line">${row}</div>`).join(''));
});

test('returning to a terminal includes output received while another terminal was active', () => {
    const v = viewer();
    v.api.renderScrollback('test', ['one', 'two', 'three']);
    v.context.activeTerminalId = 'other';
    v.api.appendScrollback('test', 'four\nfive\n');
    v.context.activeTerminalId = 'test';
    v.api.flushScrollback('test');
    assert.equal(v.html(), ['one', 'two', 'three', 'four', 'five'].map(row => `<div class="line">${row}</div>`).join(''));
});

test('trimming history counts captured rows inside joined paragraphs', () => {
    const removeRows = runInNewContext(embedded('scrollback') + '\nremoveScrollbackRows');
    const element = { children: [], get firstElementChild() { return this.children[0]; } };
    function row(name, children = []) {
        const node = { name, children, dataset: children.length ? { terminalRows: String(children.length) } : {},
            remove() { element.children.splice(element.children.indexOf(node), 1); },
            replaceWith(...rows) { element.children.splice(element.children.indexOf(node), 1, ...rows); } };
        return node;
    }
    element.children = [row('paragraph', [row('one'), row('two'), row('three')]), row('four')];
    removeRows(element, 1);
    assert.deepEqual(element.children.map(row => row.name), ['two', 'three', 'four']);
    element.children = [row('paragraph', [row('one'), row('two'), row('three')]), row('four')];
    removeRows(element, 3);
    assert.deepEqual(element.children.map(row => row.name), ['four']);
});
