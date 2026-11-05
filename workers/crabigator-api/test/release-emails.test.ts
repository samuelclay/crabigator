import { env } from 'cloudflare:test';
import { afterEach, beforeAll, beforeEach, expect, it, vi } from 'vitest';
import { handleAnalyticsEvent } from '../src/handlers/analytics';
import {
    composeReleaseEmail,
    handleUnsubscribe,
    handleUnsubscribePage,
    releaseHighlights,
    sendReleaseEmails,
} from '../src/release-emails';
import type { Env } from '../src/types/env';
import analyticsMigration from '../migrations/0007_analytics.sql?raw';
import releaseMigration from '../migrations/0026_release_emails.sql?raw';
import notesMigration from '../migrations/0027_release_email_notes.sql?raw';

const testEnv = env as unknown as Env;
const mailer = { origin: 'https://crab.example', mailgunDomain: 'email.crab.example', from: 'Crabigator <releases@crab.example>' };
const notesHtml = '<h2>Highlights</h2>\n<ul>\n<li><strong>Vertical flow.</strong> Tall <code>terminals</code> get it.</li>\n</ul>\n<h2>Complete Changes</h2>\n<ul>\n<li>Narrow the flow column (5385c46)</li>\n</ul>';
const notes = '## Highlights\n\n- **Vertical flow.** Tall `terminals` get it.\n\n## Complete Changes\n\n- Narrow the flow column (5385c46)\n';
const release = {
    tag_name: 'v0.18.0',
    name: 'v0.18.0 · Flow scenes stand up in tall terminals',
    html_url: 'https://github.com/samuelclay/crabigator/releases/tag/v0.18.0',
    body: notes,
    body_html: notesHtml,
};

beforeAll(async () => {
    // Comments may hold semicolons; drop them before splitting into statements.
    for (const migration of [analyticsMigration, releaseMigration, notesMigration]) {
        for (const sql of migration.replace(/--[^\n]*/g, '').split(';').map(s => s.trim()).filter(Boolean)) {
            await testEnv.DB.prepare(sql).run();
        }
    }
});

beforeEach(async () => {
    await testEnv.DB.prepare('DELETE FROM email_signups').run();
    await testEnv.DB.prepare('DELETE FROM release_emails').run();
});

afterEach(() => vi.restoreAllMocks());

/** Stub GitHub and Mailgun; returns the emails Mailgun was asked to send. */
function stubNetwork(mailgunStatus = 200) {
    const sent: FormData[] = [];
    vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
        const url = String(input instanceof Request ? input.url : input);
        if (url.startsWith('https://api.github.com/')) {
            return url.endsWith('/v0.18.0') ? Response.json(release) : new Response('Not Found', { status: 404 });
        }
        if (url === 'https://api.mailgun.net/v3/email.crab.example/messages') {
            if (mailgunStatus === 200) sent.push(init!.body as FormData);
            return new Response('{}', { status: mailgunStatus });
        }
        throw new Error(`unexpected fetch ${url}`);
    });
    return sent;
}

async function subscribe(email: string, unsubscribed = 0) {
    await testEnv.DB.prepare('INSERT INTO email_signups (email, unsubscribe_token, unsubscribed) VALUES (?, ?, ?)')
        .bind(email, `token-${email}`, unsubscribed).run();
}

const releaseRow = () => testEnv.DB.prepare('SELECT * FROM release_emails ORDER BY id DESC LIMIT 1').first<Record<string, unknown>>();

it('keeps only the highlights of the release notes', () => {
    expect(releaseHighlights(notesHtml, 'html')).toBe('<ul>\n<li><strong>Vertical flow.</strong> Tall <code>terminals</code> get it.</li>\n</ul>');
    expect(releaseHighlights(notes, 'markdown')).toBe('- **Vertical flow.** Tall `terminals` get it.');
    // Notes without the sections pass through whole.
    expect(releaseHighlights('<p>Small fix.</p>', 'html')).toBe('<p>Small fix.</p>');
});

it('writes the email from the release title and highlights', () => {
    const email = composeReleaseEmail(release, 'https://crab.example');
    expect(email.subject).toBe('Crabigator 0.18.0: Flow scenes stand up in tall terminals');
    expect(email.html).toContain('<strong>Vertical flow.</strong>');
    expect(email.html).not.toContain('Complete Changes');
    expect(email.html).toContain('href="{{unsubscribe_url}}"');
    expect(email.text).toContain('- **Vertical flow.** Tall `terminals` get it.');
    expect(email.text).toContain('Unsubscribe: {{unsubscribe_url}}');
    expect(email.text).toContain('signed up for release news at crab.example');
});

it('emails each subscriber once, with their own unsubscribe link, skipping the unsubscribed and later signups', async () => {
    await subscribe('a@example.com');
    await subscribe('gone@example.com', 1);
    await testEnv.DB.prepare('INSERT INTO email_signups (email) VALUES (?)').bind('old@example.com').run(); // signed up before tokens
    await testEnv.DB.prepare("INSERT INTO release_emails (tag, created_at) VALUES ('v0.18.0', unixepoch() + 60)").run();
    await testEnv.DB.prepare('INSERT INTO email_signups (email, created_at) VALUES (?, unixepoch() + 3600)').bind('later@example.com').run();
    const sent = stubNetwork();

    await sendReleaseEmails(testEnv, mailer);

    expect(sent.map(form => form.get('to'))).toEqual(['a@example.com', 'old@example.com']);
    expect(sent[0].get('subject')).toBe('Crabigator 0.18.0: Flow scenes stand up in tall terminals');
    expect(sent[0].get('h:List-Unsubscribe')).toBe('<https://crab.example/unsubscribe?t=token-a%40example.com>');
    expect(sent[0].get('h:List-Unsubscribe-Post')).toBe('List-Unsubscribe=One-Click');
    expect(String(sent[0].get('html'))).toContain('href="https://crab.example/unsubscribe?t=token-a%40example.com"');
    // A subscriber from before tokens gets one now.
    const old = await testEnv.DB.prepare('SELECT unsubscribe_token FROM email_signups WHERE email = ?').bind('old@example.com').first<{ unsubscribe_token: string }>();
    expect(old!.unsubscribe_token).toMatch(/^[0-9a-f-]{36}$/);
    expect(String(sent[1].get('text'))).toContain(`Unsubscribe: https://crab.example/unsubscribe?t=${old!.unsubscribe_token}`);
    expect(await releaseRow()).toMatchObject({ sent_count: 2, failed_count: 0, error: null });
    expect((await releaseRow())!.finished_at).not.toBeNull();

    // The next run finds nothing left to send.
    await sendReleaseEmails(testEnv, mailer);
    expect(sent).toHaveLength(2);
});

it('uses the release queued with the email instead of asking GitHub', async () => {
    await subscribe('a@example.com');
    await testEnv.DB.prepare("INSERT INTO release_emails (tag, release_json, created_at) VALUES ('v0.18.0', ?, unixepoch() + 60)")
        .bind(JSON.stringify({ ...release, name: 'v0.18.0 · From the queue' })).run();
    const sent = stubNetwork();
    await sendReleaseEmails(testEnv, mailer);
    expect(sent.map(form => form.get('subject'))).toEqual(['Crabigator 0.18.0: From the queue']);
    expect(vi.mocked(fetch).mock.calls.some(([url]) => String(url).startsWith('https://api.github.com/'))).toBe(false);
});

it('sends in batches and picks up where the last run stopped', async () => {
    for (const name of ['a', 'b', 'c']) await subscribe(`${name}@example.com`);
    await testEnv.DB.prepare("INSERT INTO release_emails (tag, created_at) VALUES ('v0.18.0', unixepoch() + 60)").run();
    const sent = stubNetwork();
    await sendReleaseEmails(testEnv, mailer, 2);
    expect(sent.map(form => form.get('to'))).toEqual(['a@example.com', 'b@example.com']);
    expect((await releaseRow())!.finished_at).toBeNull();
    await sendReleaseEmails(testEnv, mailer, 2);
    expect(sent.map(form => form.get('to'))).toEqual(['a@example.com', 'b@example.com', 'c@example.com']);
    expect((await releaseRow())!.finished_at).not.toBeNull();
});

it('stops without skipping anyone when Mailgun fails, and tries again next run', async () => {
    await subscribe('a@example.com');
    await testEnv.DB.prepare("INSERT INTO release_emails (tag, created_at) VALUES ('v0.18.0', unixepoch() + 60)").run();
    stubNetwork(500);
    await sendReleaseEmails(testEnv, mailer);
    expect(await releaseRow()).toMatchObject({ sent_count: 0, last_signup_id: 0, finished_at: null, claimed_at: null });
    expect(String((await releaseRow())!.error)).toContain('Mailgun answered 500');

    vi.restoreAllMocks();
    const sent = stubNetwork();
    await sendReleaseEmails(testEnv, mailer);
    expect(sent.map(form => form.get('to'))).toEqual(['a@example.com']);
});

it('sends a test to one address only, and finishes a release GitHub doesn’t have', async () => {
    await subscribe('a@example.com');
    await testEnv.DB.prepare("INSERT INTO release_emails (tag, test_recipient) VALUES ('v0.18.0', 'me@example.com')").run();
    const sent = stubNetwork();
    await sendReleaseEmails(testEnv, mailer);
    expect(sent.map(form => form.get('to'))).toEqual(['me@example.com']);

    await testEnv.DB.prepare("INSERT INTO release_emails (tag) VALUES ('v9.9.9')").run();
    await sendReleaseEmails(testEnv, mailer);
    expect(await releaseRow()).toMatchObject({ error: 'GitHub has no release v9.9.9', sent_count: 0 });
    expect((await releaseRow())!.finished_at).not.toBeNull();
    expect(sent).toHaveLength(1);
});

it('asks before unsubscribing, unsubscribes on the button or a one-click POST, and resubscribes on a new signup', async () => {
    await subscribe('a@example.com');
    const url = 'https://crab.example/unsubscribe?t=token-a%40example.com';
    const page = await handleUnsubscribePage(new Request(url), testEnv);
    expect(await page.text()).toContain('<form method="post">');
    const status = () => testEnv.DB.prepare('SELECT unsubscribed FROM email_signups WHERE email = ?').bind('a@example.com').first<{ unsubscribed: number }>();
    expect((await status())!.unsubscribed).toBe(0);

    const done = await handleUnsubscribe(new Request(url, { method: 'POST', body: 'List-Unsubscribe=One-Click' }), testEnv);
    expect(done.status).toBe(200);
    expect(await done.text()).toContain('You’re unsubscribed');
    expect((await status())!.unsubscribed).toBe(1);

    expect((await handleUnsubscribePage(new Request('https://crab.example/unsubscribe?t=nope'), testEnv)).status).toBe(404);

    const signup = new Request('https://crab.example/api/analytics/event', {
        method: 'POST',
        body: JSON.stringify({ visitor_id: 'v', session_id: 's', event_type: 'signup', event_value: 'A@example.com ' }),
    });
    await handleAnalyticsEvent(signup, testEnv);
    const row = await testEnv.DB.prepare('SELECT unsubscribed, unsubscribe_token FROM email_signups WHERE email = ?').bind('a@example.com').first();
    expect(row).toMatchObject({ unsubscribed: 0, unsubscribe_token: 'token-a@example.com' });
});

it('gives a new signup its own unsubscribe token', async () => {
    const signup = new Request('https://crab.example/api/analytics/event', {
        method: 'POST',
        body: JSON.stringify({ visitor_id: 'v', session_id: 's', event_type: 'signup', event_value: 'new@example.com' }),
    });
    await handleAnalyticsEvent(signup, testEnv);
    const row = await testEnv.DB.prepare('SELECT unsubscribe_token FROM email_signups WHERE email = ?').bind('new@example.com').first<{ unsubscribe_token: string }>();
    expect(row!.unsubscribe_token).toMatch(/^[0-9a-f-]{36}$/);
});
