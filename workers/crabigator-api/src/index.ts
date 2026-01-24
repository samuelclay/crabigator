import { Router, jsonResponse } from './router';
import type { Env } from './types/env';
import { registerDevice, deviceHeartbeat, getLinkedDevices, revokeLinkedDevice } from './handlers/devices';
import { createSession, getSession, updateSession, deleteSession } from './handlers/sessions';
import { generatePairingToken, claimPairingToken, getPairingStatus, getPairingCodePage, generateInviteCode } from './handlers/pairing';
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
router.get('/api/devices/linked', getLinkedDevices);
router.delete('/api/devices/linked/:mobile_id', revokeLinkedDevice);

// ============================================
// Pairing endpoints
// ============================================

router.post('/api/pairing/generate', generatePairingToken);
router.post('/api/pairing/claim', claimPairingToken);
router.post('/api/pairing/invite', generateInviteCode);
router.get('/api/pairing/:token/status', getPairingStatus);

// Public pairing code page (no auth)
router.get('/pair/:token', getPairingCodePage);

// ============================================
// Session endpoints
// ============================================

router.post('/api/sessions', createSession);

// List all active sessions (no auth for dashboard)
// Queries SessionListDO for currently-connected sessions to avoid stale data
router.get('/api/sessions', async (request, env) => {
    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const response = await stub.fetch(new Request('https://internal/sessions'));
    const data = await response.json() as { sessions: Array<{ id: string; cwd: string; platform: string; state: string; started_at: number }> };

    return jsonResponse({ sessions: data.sessions });
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

    // Verify session belongs to device and get session info
    const session = await env.DB.prepare(
        'SELECT id, cwd, platform, started_at FROM sessions WHERE id = ? AND device_id = ?'
    ).bind(sessionId, device_id).first<{ id: string; cwd: string; platform: string; started_at: number }>();

    if (!session) {
        return router.errorResponse('Session not found', 'NOT_FOUND', 404);
    }

    // Forward to Durable Object with session info
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/connect';
    url.searchParams.set('sessionId', sessionId);
    url.searchParams.set('cwd', session.cwd);
    url.searchParams.set('platform', session.platform);
    url.searchParams.set('started_at', String(session.started_at));
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

// Save draft input text (for input persistence across deploys)
router.post('/api/sessions/:id/draft', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/draft';
    return stub.fetch(new Request(url.toString(), request));
});

// Get draft input text
router.get('/api/sessions/:id/draft', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/draft';
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

// Notify session that a viewer is active (heartbeat from dashboard/phone)
// This allows the desktop to optimize streaming when no one is watching
router.post('/api/sessions/:id/viewer-active', async (request, env, params) => {
    const sessionId = params.id;
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/viewer-active';
    return stub.fetch(new Request(url.toString(), request));
});

// ============================================
// Dashboard settings (stored per client via cookie)
// ============================================

function getClientId(request: Request): string | null {
    const cookie = request.headers.get('Cookie') || '';
    const match = cookie.match(/crabigator_client=([a-zA-Z0-9-]+)/);
    return match ? match[1] : null;
}

function generateClientId(): string {
    return crypto.randomUUID();
}

interface DashboardSettings {
    fontScaleIndex?: number;
    terminalHeightIndex?: number;
    terminalWrapEnabled?: boolean;
}

const DEFAULT_SETTINGS: DashboardSettings = { fontScaleIndex: 3, terminalHeightIndex: 3, terminalWrapEnabled: true };

router.get('/api/settings', async (request, env) => {
    const clientId = getClientId(request);

    // If no client ID, return defaults (cookie will be set on next POST)
    if (!clientId) {
        return jsonResponse(DEFAULT_SETTINGS);
    }

    const settings = await env.TOKENS.get(`settings:${clientId}`, 'json') as DashboardSettings | null;
    return jsonResponse({ ...DEFAULT_SETTINGS, ...settings });
});

router.post('/api/settings', async (request, env) => {
    let clientId = getClientId(request);
    const isNew = !clientId;
    if (!clientId) {
        clientId = generateClientId();
    }

    let body: DashboardSettings;
    try {
        body = await request.json();
    } catch {
        return router.errorResponse('Invalid JSON', 'INVALID_JSON', 400);
    }

    // Load existing settings and merge
    const existing = await env.TOKENS.get(`settings:${clientId}`, 'json') as DashboardSettings | null;
    const settings: DashboardSettings = {
        fontScaleIndex: typeof body.fontScaleIndex === 'number'
            ? Math.max(0, Math.min(6, body.fontScaleIndex))
            : existing?.fontScaleIndex ?? 3,
        terminalHeightIndex: typeof body.terminalHeightIndex === 'number'
            ? Math.max(0, Math.min(6, body.terminalHeightIndex))
            : existing?.terminalHeightIndex ?? 3,
        terminalWrapEnabled: typeof body.terminalWrapEnabled === 'boolean'
            ? body.terminalWrapEnabled
            : existing?.terminalWrapEnabled ?? true
    };

    await env.TOKENS.put(`settings:${clientId}`, JSON.stringify(settings), {
        expirationTtl: 60 * 60 * 24 * 365 // 1 year
    });

    const headers: HeadersInit = { 'Content-Type': 'application/json' };
    if (isNew) {
        // Set cookie for new clients (1 year expiry)
        const expires = new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toUTCString();
        headers['Set-Cookie'] = `crabigator_client=${clientId}; Path=/; Expires=${expires}; SameSite=Lax`;
    }

    return new Response(JSON.stringify({ ok: true }), { headers });
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
