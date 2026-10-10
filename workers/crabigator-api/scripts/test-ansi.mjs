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
