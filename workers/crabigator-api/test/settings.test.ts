import { SELF } from 'cloudflare:test';
import { expect, it } from 'vitest';
import { FONT_SCALES, TERMINAL_HEIGHTS } from '../src/dashboard/steps';

const URL = 'https://example.com/api/settings';

async function save(settings: Record<string, unknown>, cookie = ''): Promise<string> {
    const response = await SELF.fetch(URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...(cookie ? { Cookie: cookie } : {}) },
        body: JSON.stringify(settings),
    });
    expect(response.status).toBe(200);
    return cookie || response.headers.get('Set-Cookie')!.split(';')[0];
}

async function read(cookie: string): Promise<Record<string, unknown>> {
    return (await SELF.fetch(URL, { headers: { Cookie: cookie } })).json();
}

it('keeps every text size and terminal height the dashboard offers', async () => {
    // The page offers more text sizes than terminal heights. The largest of
    // each must come back after a reload, not a smaller one.
    const largest = { fontScaleIndex: FONT_SCALES.length - 1, terminalHeightIndex: TERMINAL_HEIGHTS.length - 1 };
    expect(largest.fontScaleIndex).toBeGreaterThan(largest.terminalHeightIndex);
    const cookie = await save(largest);
    expect(await read(cookie)).toMatchObject(largest);

    for (let index = 0; index < FONT_SCALES.length; index++) {
        await save({ fontScaleIndex: index }, cookie);
        expect((await read(cookie)).fontScaleIndex).toBe(index);
    }
    // A change to one setting leaves the others as they were.
    expect((await read(cookie)).terminalHeightIndex).toBe(largest.terminalHeightIndex);

    // An index outside the steps is held at the nearest step.
    await save({ fontScaleIndex: 99, terminalHeightIndex: 99 }, cookie);
    expect(await read(cookie)).toMatchObject(largest);
    await save({ fontScaleIndex: -4, terminalHeightIndex: -4 }, cookie);
    expect(await read(cookie)).toMatchObject({ fontScaleIndex: 0, terminalHeightIndex: 0 });
});
