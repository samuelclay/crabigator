/**
 * Release emails: each release's highlights, emailed through Mailgun to the
 * homepage's newsletter subscribers.
 *
 * `make announce-release TAG=vX.Y.Z` reads the release from GitHub and adds
 * it to `release_emails` (the /release command runs it once the notes are
 * final). The five-minute cron then writes the email once and sends it to
 * subscribers in batches, recording how far it got. Every email carries a
 * link, and the one-click header mail apps use, to unsubscribe.
 */
import type { Env } from './types/env';

/** The repository whose releases the emails announce. */
const RELEASES_REPO = 'samuelclay/crabigator';
/** Emails sent per cron run, every five minutes. */
const BATCH = 100;
/** A run that took an email this long ago and never let go has stopped. */
const CLAIM_SECONDS = 240;
/** Where each recipient's unsubscribe link goes in the stored email. */
const UNSUBSCRIBE = '{{unsubscribe_url}}';

export interface ReleaseMailer {
    /** The public origin, for unsubscribe links. */
    origin: string;
    mailgunDomain: string;
    from: string;
}

interface Release {
    tag_name: string;
    name: string | null;
    html_url: string;
    body: string | null;
    body_html: string | null;
}

interface ReleaseEmail {
    subject: string;
    html: string;
    text: string;
}

interface ReleaseEmailRow {
    id: number;
    tag: string;
    test_recipient: string | null;
    subject: string | null;
    html: string | null;
    text: string | null;
    release_json: string | null;
    last_signup_id: number;
    created_at: number;
}

function escapeHtml(value: string): string {
    return value.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);
}

/** A release's highlights: its notes up to "Complete Changes", without the "Highlights" heading. */
export function releaseHighlights(notes: string, kind: 'html' | 'markdown'): string {
    const [complete, heading] = kind === 'html'
        ? [/<h2[^>]*>\s*Complete Changes\s*<\/h2>/i, /<h2[^>]*>\s*Highlights\s*<\/h2>/i]
        : [/^##\s+Complete Changes\s*$/im, /^##\s+Highlights\s*$/im];
    const cut = notes.search(complete);
    return (cut >= 0 ? notes.slice(0, cut) : notes).replace(heading, '').trim();
}

/** The email for a release, with `{{unsubscribe_url}}` where each recipient's link goes. */
export function composeReleaseEmail(release: Release, origin: string): ReleaseEmail {
    const version = release.tag_name.replace(/^v/, '');
    // Release titles read "vX.Y.Z · One-liner".
    const title = (release.name || '').split(' · ').slice(1).join(' · ').trim();
    const subject = title ? `Crabigator ${version}: ${title}` : `Crabigator ${version}`;
    const host = new URL(origin).host;
    const font = `-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif`;
    const code = `font-family: SFMono-Regular, Menlo, Consolas, monospace; font-size: 13px; background: #f3f0ea; padding: 1px 4px; border-radius: 3px;`;
    const html = `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(subject)}</title>
<style>
.notes ul { padding-left: 20px; margin: 0; }
.notes li { margin: 0 0 10px; }
.notes code { ${code} }
.notes a { color: #b4541a; }
</style>
</head>
<body style="margin: 0; padding: 0; background: #f6f4ef;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background: #f6f4ef;">
<tr><td align="center" style="padding: 32px 16px;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width: 600px; background: #ffffff; border-radius: 8px;">
<tr><td style="padding: 32px; font-family: ${font}; font-size: 15px; line-height: 1.55; color: #1d1b18;">
<p style="margin: 0 0 6px; font-size: 13px; color: #8a8378;">Crabigator ${escapeHtml(version)}</p>
<h1 style="margin: 0 0 20px; font-size: 22px; line-height: 1.3;">${escapeHtml(title || 'A new release')}</h1>
<div class="notes">${releaseHighlights(release.body_html || '', 'html')}</div>
<p style="margin: 24px 0 0;"><a href="${escapeHtml(release.html_url)}" style="color: #b4541a;">Read every change on GitHub</a></p>
<p style="margin: 16px 0 0; color: #5c564d;">Crabigator offers the update the next time it starts, or run <code style="${code}">npm install -g crabigator</code>.</p>
</td></tr>
</table>
<p style="max-width: 600px; margin: 16px auto 0; font-family: ${font}; font-size: 12px; line-height: 1.5; color: #8a8378;">You’re getting this because you signed up for release news at ${escapeHtml(host)}. <a href="${UNSUBSCRIBE}" style="color: #8a8378;">Unsubscribe</a>.</p>
</td></tr>
</table>
</body>
</html>`;
    const text = [
        subject,
        '',
        releaseHighlights(release.body || '', 'markdown'),
        '',
        `Every change: ${release.html_url}`,
        '',
        'Crabigator offers the update the next time it starts, or run: npm install -g crabigator',
        '',
        '--',
        `You’re getting this because you signed up for release news at ${host}.`,
        `Unsubscribe: ${UNSUBSCRIBE}`,
    ].join('\n');
    return { subject, html, text };
}

/**
 * The release, or null when GitHub has no release with this tag. Only for a
 * row queued without one: GitHub often rate-limits the Worker's requests.
 */
async function fetchRelease(tag: string): Promise<Release | null> {
    const response = await fetch(`https://api.github.com/repos/${RELEASES_REPO}/releases/tags/${encodeURIComponent(tag)}`, {
        headers: { Accept: 'application/vnd.github.full+json', 'User-Agent': 'crabigator-release-emails' },
    });
    if (response.status === 404) return null;
    if (!response.ok) throw new Error(`GitHub answered ${response.status}: ${(await response.text()).slice(0, 120)}`);
    return response.json();
}

class MailgunError extends Error {
    constructor(readonly status: number, message: string) {
        super(message);
    }
}

async function sendOne(env: Env, mailer: ReleaseMailer, to: string, email: ReleaseEmail, unsubscribeUrl: string): Promise<void> {
    const form = new FormData();
    form.append('from', mailer.from);
    form.append('to', to);
    form.append('subject', email.subject);
    form.append('html', email.html.replaceAll(UNSUBSCRIBE, escapeHtml(unsubscribeUrl)));
    form.append('text', email.text.replaceAll(UNSUBSCRIBE, unsubscribeUrl));
    // One-click unsubscribe (RFC 8058): mail apps POST to the link themselves.
    form.append('h:List-Unsubscribe', `<${unsubscribeUrl}>`);
    form.append('h:List-Unsubscribe-Post', 'List-Unsubscribe=One-Click');
    form.append('o:tag', 'release');
    const response = await fetch(`https://api.mailgun.net/v3/${mailer.mailgunDomain}/messages`, {
        method: 'POST',
        headers: { Authorization: 'Basic ' + btoa('api:' + env.MAILGUN_API_KEY) },
        body: form,
    });
    if (!response.ok) {
        throw new MailgunError(response.status, `Mailgun answered ${response.status}: ${(await response.text()).slice(0, 200)}`);
    }
}

function unsubscribeUrl(origin: string, token: string): string {
    return `${origin}/unsubscribe?t=${encodeURIComponent(token)}`;
}

/** The subscriber's unsubscribe token, made now if they signed up before tokens existed. */
async function tokenFor(env: Env, signupId: number): Promise<string> {
    const row = await env.DB.prepare(
        'UPDATE email_signups SET unsubscribe_token = COALESCE(unsubscribe_token, ?) WHERE id = ? RETURNING unsubscribe_token',
    ).bind(crypto.randomUUID(), signupId).first<{ unsubscribe_token: string }>();
    return row!.unsubscribe_token;
}

/**
 * Send the oldest unfinished release email to its next batch of subscribers.
 * Everyone who signed up before the release was announced gets it, unless
 * they unsubscribed. A failure that isn't one address's fault (GitHub or
 * Mailgun down, a bad key) stops the batch, and the next run picks it up.
 */
export async function sendReleaseEmails(env: Env, mailer: ReleaseMailer, batch = BATCH): Promise<void> {
    const now = Math.floor(Date.now() / 1000);
    const row = await env.DB.prepare(`
        SELECT id, tag, test_recipient, subject, html, text, release_json, last_signup_id, created_at
        FROM release_emails
        WHERE finished_at IS NULL AND (claimed_at IS NULL OR claimed_at < ?)
        ORDER BY id LIMIT 1
    `).bind(now - CLAIM_SECONDS).first<ReleaseEmailRow>();
    if (!row) return;
    const claim = await env.DB.prepare(
        'UPDATE release_emails SET claimed_at = ? WHERE id = ? AND (claimed_at IS NULL OR claimed_at < ?)',
    ).bind(now, row.id, now - CLAIM_SECONDS).run();
    if (!claim.meta.changes) return;

    const finish = (error: string | null) => env.DB.prepare(
        'UPDATE release_emails SET finished_at = ?, error = COALESCE(?, error), claimed_at = NULL WHERE id = ?',
    ).bind(Math.floor(Date.now() / 1000), error, row.id).run();
    try {
        let email: ReleaseEmail;
        if (row.subject && row.html && row.text) {
            email = { subject: row.subject, html: row.html, text: row.text };
        } else {
            const release = row.release_json ? JSON.parse(row.release_json) as Release : await fetchRelease(row.tag);
            if (!release) {
                await finish(`GitHub has no release ${row.tag}`);
                return;
            }
            email = composeReleaseEmail(release, mailer.origin);
            await env.DB.prepare('UPDATE release_emails SET subject = ?, html = ?, text = ?, error = NULL WHERE id = ?')
                .bind(email.subject, email.html, email.text, row.id).run();
        }

        if (row.test_recipient) {
            const signup = await env.DB.prepare('SELECT id FROM email_signups WHERE email = ?')
                .bind(row.test_recipient.toLowerCase()).first<{ id: number }>();
            const token = signup ? await tokenFor(env, signup.id) : 'test';
            await sendOne(env, mailer, row.test_recipient, email, unsubscribeUrl(mailer.origin, token));
            await env.DB.prepare('UPDATE release_emails SET sent_count = 1 WHERE id = ?').bind(row.id).run();
            await finish(null);
            return;
        }

        const { results } = await env.DB.prepare(`
            SELECT id, email, unsubscribe_token FROM email_signups
            WHERE id > ? AND unsubscribed = 0 AND created_at <= ?
            ORDER BY id LIMIT ?
        `).bind(row.last_signup_id, row.created_at, batch).all<{ id: number; email: string; unsubscribe_token: string | null }>();
        for (const signup of results) {
            const token = signup.unsubscribe_token ?? await tokenFor(env, signup.id);
            let failed: string | null = null;
            try {
                await sendOne(env, mailer, signup.email, email, unsubscribeUrl(mailer.origin, token));
            } catch (error) {
                // Mailgun refuses a bad address with a 400; anything else isn't this address's fault.
                if (!(error instanceof MailgunError && error.status === 400)) throw error;
                failed = error.message;
            }
            await env.DB.prepare(`
                UPDATE release_emails
                SET last_signup_id = ?, sent_count = sent_count + ?, failed_count = failed_count + ?, error = COALESCE(?, error)
                WHERE id = ?
            `).bind(signup.id, failed ? 0 : 1, failed ? 1 : 0, failed, row.id).run();
        }
        if (results.length < batch) await finish(null);
        else await env.DB.prepare('UPDATE release_emails SET claimed_at = NULL WHERE id = ?').bind(row.id).run();
    } catch (error) {
        await env.DB.prepare('UPDATE release_emails SET error = ?, claimed_at = NULL WHERE id = ?')
            .bind(error instanceof Error ? error.message : String(error), row.id).run();
    }
}

function unsubscribePage(title: string, body: string, status = 200): Response {
    const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>${escapeHtml(title)} · Crabigator</title>
<style>
:root { color-scheme: light dark; --bg: #f6f4ef; --card: #ffffff; --ink: #1d1b18; --muted: #6f685d; --accent: #b4541a; }
@media (prefers-color-scheme: dark) { :root { --bg: #0b0f14; --card: #151b23; --ink: #e8e4dc; --muted: #9a9387; --accent: #e8915a; } }
body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: var(--bg); color: var(--ink); font: 16px/1.55 -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif; }
main { box-sizing: border-box; width: min(440px, calc(100% - 32px)); padding: 32px; background: var(--card); border-radius: 8px; }
h1 { margin: 0 0 12px; font-size: 22px; }
p { margin: 0 0 16px; color: var(--muted); }
button { font: inherit; padding: 10px 18px; border: 0; border-radius: 6px; background: var(--accent); color: #fff; cursor: pointer; }
a { color: var(--accent); }
</style>
</head>
<body><main><h1>${escapeHtml(title)}</h1>${body}</main></body>
</html>`;
    return new Response(html, {
        status,
        headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' },
    });
}

async function signupForToken(request: Request, env: Env) {
    const token = new URL(request.url).searchParams.get('t') || '';
    if (!token) return null;
    return env.DB.prepare('SELECT email, unsubscribed FROM email_signups WHERE unsubscribe_token = ?')
        .bind(token).first<{ email: string; unsubscribed: number }>();
}

const invalidLink = () => unsubscribePage('This link doesn’t work', '<p>It may have been copied only in part. Try the link in the email again.</p>', 404);

/** GET /unsubscribe?t=… asks first: link scanners open links, and must not unsubscribe anyone. */
export async function handleUnsubscribePage(request: Request, env: Env): Promise<Response> {
    const signup = await signupForToken(request, env);
    if (!signup) return invalidLink();
    if (signup.unsubscribed) {
        return unsubscribePage('You’re unsubscribed', `<p>${escapeHtml(signup.email)} gets no more release emails.</p>`);
    }
    return unsubscribePage(
        'Unsubscribe from release emails?',
        `<p>${escapeHtml(signup.email)} will stop getting an email for each Crabigator release.</p>`
        + `<form method="post"><button type="submit">Unsubscribe</button></form>`,
    );
}

/** POST /unsubscribe?t=…: the page's button, or a mail app's one-click unsubscribe. */
export async function handleUnsubscribe(request: Request, env: Env): Promise<Response> {
    const signup = await signupForToken(request, env);
    if (!signup) return invalidLink();
    const token = new URL(request.url).searchParams.get('t');
    await env.DB.prepare(
        'UPDATE email_signups SET unsubscribed = 1, unsubscribed_at = COALESCE(unsubscribed_at, unixepoch()) WHERE unsubscribe_token = ?',
    ).bind(token).run();
    return unsubscribePage('You’re unsubscribed', `<p>${escapeHtml(signup.email)} gets no more release emails.</p>`);
}
