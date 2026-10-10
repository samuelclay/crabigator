import type { Env } from '../types/env';
import { extractToken, requireMobileAuth, requireSessionAccess } from '../auth/middleware';
import { sha256 } from '../auth/tokens';
import { jsonResponse } from '../router';

export async function registerPush(request: Request, env: Env): Promise<Response> {
    const auth = await requireMobileAuth(request, env);
    if ('error' in auth) return auth.error;
    const body = await request.json().catch(() => null) as { token?: unknown; platform?: unknown } | null;
    if (!body || typeof body.token !== 'string' || body.token.length < 20 || body.token.length > 4096 || body.platform !== 'android') {
        return jsonResponse({ error: 'A valid Android push token is required' }, 400);
    }
    await env.DB.batch([
        // Reinstalling can keep the FCM token while pairing creates a new mobile id.
        env.DB.prepare('DELETE FROM mobile_push_tokens WHERE token = ? AND mobile_id != ?')
            .bind(body.token, auth.auth.mobile_id),
        env.DB.prepare(`INSERT INTO mobile_push_tokens (mobile_id, token, token_hash, platform, updated_at)
        VALUES (?, ?, ?, 'android', ?) ON CONFLICT(mobile_id) DO UPDATE SET token = excluded.token,
        token_hash = excluded.token_hash, updated_at = excluded.updated_at`)
        .bind(auth.auth.mobile_id, body.token, await sha256(extractToken(request)!), Math.floor(Date.now() / 1000)),
    ]);
    return jsonResponse({ ok: true, configured: Boolean(env.FIREBASE_SERVICE_ACCOUNT) });
}

export async function mobileSnapshot(request: Request, env: Env, params: Record<string, string>): Promise<Response> {
    const auth = await requireMobileAuth(request, env);
    if ('error' in auth) return auth.error;
    const access = await requireSessionAccess(request, env, params.id);
    if ('error' in access) return access.error;
    const stub = env.SESSION.get(env.SESSION.idFromName(params.id));
    const result = await stub.fetch('https://internal/snapshot');
    const snapshot = await result.json() as Record<string, unknown>;
    return jsonResponse({ id: snapshot.id, title: snapshot.title, state: snapshot.state,
        desktop_connected: snapshot.desktop_connected, prompt: snapshot.prompt,
        prompt_revision: snapshot.prompt_revision, attention_pending: snapshot.attention_pending });
}
