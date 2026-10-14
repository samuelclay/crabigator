import type { Env } from '../types/env';
import { extractToken, requireMobileAuth, requireSessionAccess } from '../auth/middleware';
import { sha256 } from '../auth/tokens';
import { jsonResponse } from '../router';

export async function registerPush(request: Request, env: Env): Promise<Response> {
    const auth = await requireMobileAuth(request, env);
    if ('error' in auth) return auth.error;
    const body = await request.json().catch(() => null) as { token?: unknown; platform?: unknown; environment?: unknown; sound?: unknown } | null;
    if (!body || typeof body.token !== 'string' || body.token.length < 20 || body.token.length > 4096 || !['android', 'ios'].includes(String(body.platform))) {
        return jsonResponse({ error: 'A valid mobile push token is required' }, 400);
    }
    const apple = body.platform === 'ios';
    if (apple && (!/^[a-f0-9]{64,200}$/i.test(body.token) || !['sandbox', 'production'].includes(String(body.environment))
        || (body.sound !== undefined && typeof body.sound !== 'boolean'))) {
        return jsonResponse({ error: 'A valid Apple token and push environment are required' }, 400);
    }
    await env.DB.batch([
        // Reinstalling can keep the FCM token while pairing creates a new mobile id.
        env.DB.prepare('DELETE FROM mobile_push_tokens WHERE token = ? AND mobile_id != ?')
            .bind(body.token, auth.auth.mobile_id),
        env.DB.prepare(`INSERT INTO mobile_push_tokens (mobile_id, token, token_hash, platform, environment, sound, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(mobile_id) DO UPDATE SET token = excluded.token,
        token_hash = excluded.token_hash, platform = excluded.platform, environment = excluded.environment,
        sound = excluded.sound, updated_at = excluded.updated_at`)
        .bind(auth.auth.mobile_id, body.token, await sha256(extractToken(request)!), body.platform,
            apple ? body.environment : 'production', apple && body.sound ? 1 : 0, Math.floor(Date.now() / 1000)),
    ]);
    return jsonResponse({ ok: true, configured: Boolean(apple ? env.APNS_CREDENTIALS : env.FIREBASE_SERVICE_ACCOUNT) });
}

export async function mobileSnapshot(request: Request, env: Env, params: Record<string, string>): Promise<Response> {
    const auth = await requireMobileAuth(request, env);
    if ('error' in auth) return auth.error;
    const access = await requireSessionAccess(request, env, params.id);
    if ('error' in access) return access.error;
    const stub = env.SESSION.get(env.SESSION.idFromName(params.id));
    const result = await stub.fetch('https://internal/snapshot');
    const snapshot = await result.json() as Record<string, unknown>;
    const terminal = new URL(request.url).searchParams.get('terminal') === '1';
    return jsonResponse({ id: snapshot.id, title: snapshot.title, state: snapshot.state,
        desktop_connected: snapshot.desktop_connected, prompt: snapshot.prompt,
        prompt_revision: snapshot.prompt_revision, attention_pending: snapshot.attention_pending,
        ...(terminal ? { screen: snapshot.screen, scrollback: snapshot.screen !== null || snapshot.scrollback
            ? snapshot.scrollback : null } : {}) });
}

/** Relay a new session through the desktop the mobile user selected. */
export async function spawnOnSessionDesktop(request: Request, env: Env, sessionId: string): Promise<Response> {
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) return access.error;
    const body = await request.json().catch(() => null) as { cwd?: unknown; platform?: unknown } | null;
    if (!body || typeof body.cwd !== 'string' || !body.cwd.startsWith('/') || body.cwd.includes('\0')
        || typeof body.platform !== 'string' || !['claude', 'codex', 'opencode', 'grok'].includes(body.platform)) {
        return jsonResponse({ error: 'Choose an assistant and enter an absolute folder path.' }, 400);
    }
    const stub = env.SESSION.get(env.SESSION.idFromName(sessionId));
    return stub.fetch(new Request('https://internal/spawn', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ cwd: body.cwd, platform: body.platform }),
    }));
}
