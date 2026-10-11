import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';

// Execute the dashboard's actual embedded browser code, not a second parser.
const source = readFileSync(new URL('../src/dashboard/js/ansi.ts', import.meta.url), 'utf8');
const script = runInNewContext(source.replace(/^export /m, '') + '\nansiJs');
const ansiToHtml = runInNewContext(script + '\nansiToHtml');

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
