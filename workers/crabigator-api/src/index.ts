import { Router, jsonResponse } from './router';
import type { Env } from './types/env';
import { registerDevice, deviceHeartbeat } from './handlers/devices';
import { createSession, getSession, updateSession, deleteSession } from './handlers/sessions';
import { requireDeviceAuth } from './auth/middleware';
import { dashboardHtml } from './dashboard';

// Re-export Durable Objects
export { SessionDO } from './durable-objects/SessionDO';
export { SessionListDO } from './durable-objects/SessionListDO';

const router = new Router();

// ============================================
// Dashboard (no auth for now)
// ============================================

router.get('/dashboard', async () => {
    return new Response(dashboardHtml, {
        headers: { 'Content-Type': 'text/html; charset=utf-8' }
    });
});

// Redirect root to dashboard
router.get('/', async () => {
    return new Response(null, {
        status: 302,
        headers: { 'Location': '/dashboard' }
    });
});

// ============================================
// Device endpoints
// ============================================

router.post('/api/devices', registerDevice);
router.post('/api/devices/heartbeat', deviceHeartbeat);

// ============================================
// Session endpoints
// ============================================

router.post('/api/sessions', createSession);

// List all active sessions (no auth for dashboard)
router.get('/api/sessions', async (request, env) => {
    const url = new URL(request.url);
    const activeOnly = url.searchParams.get('active') !== 'false';
    const limit = Math.min(parseInt(url.searchParams.get('limit') || '50', 10), 100);
    const offset = parseInt(url.searchParams.get('offset') || '0', 10);

    let query = `
        SELECT id, client_session_id, cwd, platform, state, started_at, ended_at, is_active,
               prompts, completions, tool_calls, thinking_seconds
        FROM sessions
    `;

    if (activeOnly) {
        query += ' WHERE is_active = 1';
    }

    query += ' ORDER BY started_at DESC LIMIT ? OFFSET ?';

    const results = await env.DB.prepare(query).bind(limit, offset).all();

    const sessions = (results.results || []).map((row: Record<string, unknown>) => ({
        id: row.id,
        client_session_id: row.client_session_id,
        cwd: row.cwd,
        platform: row.platform,
        state: row.state,
        started_at: row.started_at,
        ended_at: row.ended_at,
        is_active: row.is_active === 1,
        stats: {
            prompts: row.prompts,
            completions: row.completions,
            tool_calls: row.tool_calls,
            thinking_seconds: row.thinking_seconds,
        },
    }));

    return jsonResponse({ sessions });
});

// SSE stream for real-time session list updates (no polling needed)
router.get('/api/sessions/stream', async (request, env) => {
    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const url = new URL(request.url);
    url.pathname = '/subscribe';
    return stub.fetch(new Request(url.toString(), request));
});

router.get('/api/sessions/:id', getSession);
router.patch('/api/sessions/:id', updateSession);
router.delete('/api/sessions/:id', deleteSession);

// ============================================
// Session events (via Durable Object)
// ============================================

// WebSocket connection from desktop
router.get('/api/sessions/:id/connect', async (request, env, params) => {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;
    const sessionId = params.id;

    // Verify session belongs to device
    const session = await env.DB.prepare(
        'SELECT id FROM sessions WHERE id = ? AND device_id = ?'
    ).bind(sessionId, device_id).first();

    if (!session) {
        return router.errorResponse('Session not found', 'NOT_FOUND', 404);
    }

    // Forward to Durable Object
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/connect';
    url.searchParams.set('sessionId', sessionId);
    return stub.fetch(new Request(url.toString(), request));
});

// SSE stream for mobile/web viewers (no auth required for dashboard)
// Note: Skips D1 lookup - DO handles non-existent sessions gracefully
router.get('/api/sessions/:id/events', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/events';
    return stub.fetch(new Request(url.toString(), request));
});

// Send answer from dashboard/mobile (no auth required for dashboard)
// Note: Skips D1 lookup - DO handles non-existent sessions gracefully
router.post('/api/sessions/:id/answer', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/answer';
    return stub.fetch(new Request(url.toString(), request));
});

// Send key command from dashboard (no auth required)
// Used for mode switching via Shift+Tab
router.post('/api/sessions/:id/key', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/key';
    return stub.fetch(new Request(url.toString(), request));
});

// Get session state (for debugging, no auth for dashboard)
// Note: Skips D1 lookup - DO handles non-existent sessions gracefully
router.get('/api/sessions/:id/state', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/state';
    return stub.fetch(new Request(url.toString(), request));
});

// ============================================
// Health check
// ============================================

router.get('/api/health', async () => {
    return jsonResponse({ status: 'ok', version: '0.1.0' });
});

// ============================================
// Worker entry point
// ============================================

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        return router.handle(request, env);
    },
} satisfies ExportedHandler<Env>;
