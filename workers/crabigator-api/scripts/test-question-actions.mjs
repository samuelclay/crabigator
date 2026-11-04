import { runInNewContext } from 'node:vm';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { embedded, functions } from './dashboard-source.mjs';

for (const preview of [true, false]) {
    test(`${preview ? 'preview' : 'ordinary'} question sends exactly one answer`, async () => {
        const q = { options: [{ label: 'List', ...(preview ? { preview: 'List drawing' } : {}) }, { label: 'Cards' }] };
        const prompt = { questions: [q], cursor_row: 1 };
        let sent;
        const answer = runInNewContext(functions(embedded('prompt'), ['sendQuestionAnswer', 'leaveTextRowSteps']) + '\nsendQuestionAnswer', {
            sessionPromptData: new Map([['s', prompt]]), isScreenDriven: () => true, isGrokCard: () => false,
            postKeySequence: async (_id, steps) => { sent = JSON.parse(JSON.stringify(steps)); return true; },
            scrollToSession() {}, console,
        });
        await answer('s', 0, '2');
        assert.deepEqual(sent, [{ type: 'text', text: '2' }, ...(preview ? [{ type: 'key', key: 'enter' }] : [])]);
    });
}
