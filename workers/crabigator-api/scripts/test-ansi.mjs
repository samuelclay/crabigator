import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';

// Execute the dashboard's actual embedded browser code, not a second parser.
const source = readFileSync(new URL('../src/dashboard/js/ansi.ts', import.meta.url), 'utf8');
const script = runInNewContext(source.replace(/^export /m, '') + '\nansiJs');
const ansiToHtml = runInNewContext(script + '\nansiToHtml');

test('repeated-character rules retain their full styled text in a clipped row', () => {
    for (const symbol of ['-', '─', '━', '=', 'x', '<', '&']) {
        const half = symbol.repeat(60);
        const html = ansiToHtml('  \x1b[33m' + half + '\x1b[31m' + half + '\x1b[0m   \nInput');
        assert.match(html, /^<div class="rule-line">/);
        assert.match(html, /<div class="line">Input<\/div>$/);
        const escaped = half.replaceAll('&', '&amp;').replaceAll('<', '&lt;');
        assert.ok(html.includes('color:#d29922">' + escaped));
        assert.ok(html.includes('color:#f85149">' + escaped));
    }
    for (const row of ['--', '----- title -----', 'aaab', '   ']) {
        assert.ok(!ansiToHtml(row).includes('rule-line'));
    }
    for (const symbol of ['🦀', '·', '✳']) {
        assert.match(ansiToHtml(symbol.repeat(120)), /^<div class="rule-line">/);
    }
});

test('embedded links survive SGR changes and newlines without leaking OSC controls', () => {
    const html = ansiToHtml('\x1b]8;id=pr;https://example.com/a;b=1&c=2\x1b\\Read \x1b[1mPR\x1b[0m\nnext\x1b]8;;\x07 plain');
    assert.match(html, /href="https:\/\/example.com\/a;b=1&amp;c=2"/);
    assert.match(html, />PR<\/a><\/span>/);
    assert.match(html, />next<\/a> plain/);
    assert.equal((html.match(/<a /g) || []).length, (html.match(/<\/a>/g) || []).length);
    assert.ok(!html.includes('\x1b'));
    assert.equal(ansiToHtml('\x1b]8;;javascript:alert(1)\x07unsafe\x1b]8;;\x07'), '<div class="line">unsafe</div>');
    assert.equal(ansiToHtml('\x1b]0;window title\x07plain'), '<div class="line">plain</div>');
});

test('Codex dim status and bold shortcut switch intensity without leaking styles', () => {
    assert.equal(ansiToHtml('\x1b[2mWorking (\x1b[1mesc\x1b[2m to interrupt)\x1b[22m normal'),
        '<div class="line"><span style="opacity:0.5">Working (</span><span style="font-weight:bold">esc</span><span style="opacity:0.5"> to interrupt)</span> normal</div>');
    assert.equal(ansiToHtml('\x1b[2mBUILD SUCCESSFUL\x1b[0m normal'),
        '<div class="line"><span style="opacity:0.5">BUILD SUCCESSFUL</span> normal</div>');
});

test('blinking bullets and all Claude spinner frames keep one cell and ANSI colors', () => {
    for (const symbol of '⏵⏸⏹⏺·✢✳✶✻✽') {
        const expected = '<div class="line"><span style="color:rgb(153,153,153)"><span class="terminal-symbol">'
            + symbol + '\uFE0E</span> Bash</span></div>';
        for (const selector of ['', '\uFE0E', '\uFE0F']) {
            assert.equal(ansiToHtml('\x1b[38;2;153;153;153m' + symbol + selector + ' Bash\x1b[0m'), expected);
        }
    }
    assert.equal(ansiToHtml('  Bash'), '<div class="line">  Bash</div>');
});

test('split columns reopen ANSI styles instead of closed symbol spans', () => {
    assert.equal(ansiToHtml('\x1b[33m⏵⏵ auto mode on                    Context 40%\x1b[0m'),
        '<div class="line split-line"><span><span style="color:#d29922">'
        + '<span class="terminal-symbol">⏵\uFE0E</span><span class="terminal-symbol">⏵\uFE0E</span>'
        + ' auto mode on</span></span><span><span style="color:#d29922">Context 40%</span></span></div>');
    assert.equal(ansiToHtml('\x1b[33mLeft label          \x1b[32m          Right\x1b[0m'),
        '<div class="line split-line"><span><span style="color:#d29922">Left label</span></span>'
        + '<span><span style="color:#3fb950">Right</span></span></div>');
});

const paragraphJoins = runInNewContext(script + '\nterminalParagraphJoins');
const paragraphCases = JSON.parse(readFileSync(new URL('./fixtures/terminal-paragraphs.json', import.meta.url), 'utf8'));
for (const {name, source, expected} of paragraphCases) {
    test('paragraph reflow: ' + name, () => {
        const rows = source.split('\n'), joins = paragraphJoins(rows), output = [];
        rows.forEach((row, i) => {
            row = row.replace(/ +$/, '');
            if (joins.has(i - 1)) output[output.length - 1] += ' ' + row.trimStart();
            else output.push(row);
        });
        assert.equal(output.join('\n'), expected);
        const html = ansiToHtml(source);
        assert.equal((html.match(/<div class="(?:line[^"\n]*|rule-line)">/g) || []).length, rows.length, 'Every captured row remains available in scroll mode');
        assert.equal(html.includes('terminal-paragraph'), joins.size > 0);
    });
}

test('joined HTML keeps leading spaces, styled runs, links and escaped text', () => {
    const first = '  This paragraph describes the updated reading experience';
    const html = ansiToHtml('\x1b[2C\x1b[1m' + first.slice(2).replaceAll(' ', '\x1b[C') + '\x1b[0m\n\x1b[2C\x1b[31mcontinuing with \x1b]8;;https://example.com\x07linked text\x1b]8;;\x07.\n' + '─'.repeat(first.length + 2));
    assert.match(html, /data-terminal-rows="2"/);
    assert.equal((html.match(/class="reflow-indent">  <\/span>/g) || []).length, 2);
    assert.match(html, /color:#f85149/);
    assert.match(html, /href="https:\/\/example.com"/);
    assert.equal((html.match(/<span /g) || []).length, (html.match(/<\/span>/g) || []).length);
    assert.equal((html.match(/<a /g) || []).length, (html.match(/<\/a>/g) || []).length);
});
