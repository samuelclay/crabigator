import { Router, jsonResponse } from './router';
import type { Env } from './types/env';
import type { SessionInfo } from './types/session';
import { registerDevice, deviceHeartbeat, getLinkedDevices, revokeLinkedDevice } from './handlers/devices';
import { getPrOverrides, setPrOverride, getPrActionPage } from './handlers/pr-overrides';
import { getPrBoard, backfillSessionPrs, searchSessionScrollback } from './handlers/pr-board';
import { getWatchedPrs, setWatchedPr, relayWatchedPrStats } from './handlers/watched-prs';
import { createSession, getSession, updateSession, deleteSession } from './handlers/sessions';
import { generatePairingToken, claimPairingToken, getPairingStatus, getPairingCodePage, generateInviteCode } from './handlers/pairing';
import { requireAuth, requireDeviceAuth, requireMobileAuth, requireSessionAccess } from './auth/middleware';
import { dashboardHtml, renderDashboardHtml } from './dashboard';
import { renderLandingHtml } from './landing';
import { featureUnavailable, getAppConfig, getRuntimeConfig, type Capabilities } from './config';
import { clearStaffSession, createStaffSession, requireStaffSession, staffLoginPage } from './auth/staff';
import { createStripeCheckout } from './handlers/payments/stripe';
import { handleStripeWebhook } from './handlers/payments/stripe-webhook';
import { createPayPalSubscription } from './handlers/payments/paypal';
import { handlePayPalWebhook } from './handlers/payments/paypal-webhook';
import { getSubscription, cancelSubscription, getSubscriptionPortal } from './handlers/payments/subscription';
import { handleUpdateCheck, handleStaffDashboard, handleStaffTelemetry, handleStaffSyncUsage, handleStaffAnalytics, handleStaffSessionAnalytics } from './handlers/telemetry';
import { handleCreateGift, handleListGifts, handleSendGiftEmail, handleGetGift, handleClaimGift, handleResolvePendingGift } from './handlers/gifts';
import { handleAnalyticsBeacon, handleAnalyticsEvent } from './handlers/analytics';
import { fetchNpmStats, checkTrafficAnomalies } from './handlers/npm-stats';
import { cleanupZombieSessions } from './handlers/cleanup';
import { handleTranscribe } from './handlers/transcribe';

const PROJECT_HISTORY_RETENTION_DAYS = 14;
const SECONDS_PER_DAY = 24 * 60 * 60;

// Build version - deterministic hash of dashboard content
// This ensures all worker instances return the same version for the same code
let buildVersion: string | null = null;
async function getBuildVersion(): Promise<string> {
    if (!buildVersion) {
        // Hash the dashboard HTML to get a deterministic version across all instances
        const encoder = new TextEncoder();
        const data = encoder.encode(dashboardHtml);
        const hashBuffer = await crypto.subtle.digest('SHA-256', data);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        buildVersion = hashArray.slice(0, 4).map(b => b.toString(16).padStart(2, '0')).join('');
    }
    return buildVersion;
}

// Re-export Durable Objects
export { SessionDO } from './durable-objects/SessionDO';
export { SessionListDO } from './durable-objects/SessionListDO';
export { UsageDO } from './durable-objects/UsageDO';

const router = new Router();
type RouteHandler = (request: Request, env: Env, params: Record<string, string>) => Promise<Response>;

function withFeature(
    feature: keyof Omit<Capabilities, 'core'>,
    handler: RouteHandler,
): RouteHandler {
    return async (request, env, params) => {
        if (!getRuntimeConfig(request, env).capabilities[feature]) return featureUnavailable(feature);
        return handler(request, env, params);
    };
}

function withStaffSession(handler: RouteHandler): RouteHandler {
    return withFeature('staff', async (request, env, params) => {
        const error = await requireStaffSession(request, env);
        return error || handler(request, env, params);
    });
}

// ============================================
// Dashboard (no auth for now)
// ============================================

router.get('/dashboard', async (request, env) => {
    const runtime = getRuntimeConfig(request, env);
    const appConfig = getAppConfig(env);
    return new Response(renderDashboardHtml(runtime, appConfig.marketing?.meta_pixel_id), {
        headers: {
            'Content-Type': 'text/html; charset=utf-8',
            'Cache-Control': 'no-cache, must-revalidate',
            'ETag': await getBuildVersion()
        }
    });
});

// Landing page
router.get('/', async (request, env) => {
    const runtime = getRuntimeConfig(request, env);
    const appConfig = getAppConfig(env);
    return new Response(renderLandingHtml(runtime, appConfig.marketing?.meta_pixel_id), {
        headers: {
            'Content-Type': 'text/html; charset=utf-8',
            'Cache-Control': 'public, max-age=3600',
            'ETag': await getBuildVersion()
        }
    });
});

// ============================================
// Static Assets (for emails, etc.)
// ============================================

import { iconCrabigator, iconPhone, iconCloud, iconBolt } from './landing/icons';

// Crab logo
router.get('/assets/logo.svg', async () => {
    // Add xmlns for standalone SVG
    const svg = iconCrabigator.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" ');
    return new Response(svg, {
        headers: {
            'Content-Type': 'image/svg+xml',
            'Cache-Control': 'public, max-age=31536000',
        }
    });
});

// Feature icons for email
router.get('/assets/icon-phone.svg', async () => {
    const svg = iconPhone.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" fill="#58a6ff" ').replace('class="icon" ', '');
    return new Response(svg, {
        headers: { 'Content-Type': 'image/svg+xml', 'Cache-Control': 'public, max-age=31536000' }
    });
});

router.get('/assets/icon-cloud.svg', async () => {
    const svg = iconCloud.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" fill="#a371f7" ');
    return new Response(svg, {
        headers: { 'Content-Type': 'image/svg+xml', 'Cache-Control': 'public, max-age=31536000' }
    });
});

router.get('/assets/icon-bolt.svg', async () => {
    const svg = iconBolt.replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" fill="#3fb950" ');
    return new Response(svg, {
        headers: { 'Content-Type': 'image/svg+xml', 'Cache-Control': 'public, max-age=31536000' }
    });
});

// Open Graph preview images
import { ogLandingBase64, ogDashboardBase64 } from './assets/og-images';

router.get('/assets/og-landing.png', async () => {
    const imageData = Uint8Array.from(atob(ogLandingBase64), c => c.charCodeAt(0));
    return new Response(imageData, {
        headers: {
            'Content-Type': 'image/png',
            'Cache-Control': 'public, max-age=31536000',
        }
    });
});

router.get('/assets/og-dashboard.png', async () => {
    const imageData = Uint8Array.from(atob(ogDashboardBase64), c => c.charCodeAt(0));
    return new Response(imageData, {
        headers: {
            'Content-Type': 'image/png',
            'Cache-Control': 'public, max-age=31536000',
        }
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
// PR disposition overrides (primary/secondary/dismissed)
// ============================================

router.get('/api/pr-overrides', getPrOverrides);
router.post('/api/pr-overrides', setPrOverride);
// Self-closing action page behind the TUI's ★/✕ links
router.get('/pr-action', getPrActionPage);
// Cross-session PR board (durable, D1-backed)
router.get('/api/prs/board', getPrBoard);
router.get('/api/prs/search', searchSessionScrollback);
// Explicitly watched PRs: add/remove, list, and board-relayed GitHub stats
router.get('/api/prs/watched', getWatchedPrs);
router.post('/api/prs/watched', setWatchedPr);
router.post('/api/prs/watched/stats', relayWatchedPrStats);
// One-shot: pull stored PR lists out of pre-write-through SessionDOs into D1
router.post('/api/prs/backfill', backfillSessionPrs);

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
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const url = new URL('https://internal/sessions');
    url.searchParams.set('group_id', group_id);
    const response = await stub.fetch(new Request(url.toString()));
    const data = await response.json() as { sessions: Array<Pick<SessionInfo, 'id' | 'cwd' | 'platform' | 'state' | 'started_at' | 'last_activity_at' | 'stats'>> };

    return jsonResponse({ sessions: data.sessions });
});

// Debug view of SessionListDO state (scoped to device or group)
router.get('/api/sessions/debug', async (request, env) => {
    const authResult = await requireAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const auth = authResult.auth;
    if (auth.type === 'share') {
        return router.errorResponse('Forbidden', 'FORBIDDEN', 403);
    }

    const url = new URL(request.url);
    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const debugUrl = new URL('https://internal/debug');

    const full = url.searchParams.get('full');
    if (full) {
        debugUrl.searchParams.set('full', full);
    }

    if (auth.type === 'device') {
        debugUrl.searchParams.set('device_id', auth.device_id);
    } else if (auth.type === 'mobile') {
        if (auth.group_id) {
            debugUrl.searchParams.set('group_id', auth.group_id);
        } else {
            debugUrl.searchParams.set('device_id', auth.desktop_id);
        }
    }

    return stub.fetch(new Request(debugUrl.toString(), request));
});

// Hibernatable WebSocket for real-time session list updates.
router.get('/api/sessions/stream', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const url = new URL(request.url);
    url.pathname = '/subscribe';
    url.searchParams.set('version', await getBuildVersion());
    url.searchParams.set('group_id', group_id);
    return stub.fetch(new Request(url.toString(), request));
});

router.get('/api/sessions/:id', getSession);
router.patch('/api/sessions/:id', updateSession);
router.delete('/api/sessions/:id', deleteSession);

// List all known projects (distinct cwds from session history)
router.get('/api/projects', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    // Keep recently-used projects visible even when they have no active sessions.
    const projectHistoryCutoff = Math.floor(Date.now() / 1000) - PROJECT_HISTORY_RETENTION_DAYS * SECONDS_PER_DAY;

    const result = await env.DB.prepare(`
        SELECT cwd,
               MAX(COALESCE(sessions.ended_at, sessions.last_seen_at, sessions.started_at)) as last_active,
               COUNT(*) as total_sessions
        FROM sessions
        JOIN devices ON devices.id = sessions.device_id
        WHERE devices.group_id = ?
        GROUP BY cwd
        HAVING last_active > ?
        ORDER BY last_active DESC
    `).bind(group_id, projectHistoryCutoff).all<{ cwd: string; last_active: number; total_sessions: number }>();

    // Filter out manually hidden projects
    const hiddenJson = await env.TOKENS.get(`hidden-projects:${group_id}`);
    const hiddenSet = new Set<string>(hiddenJson ? JSON.parse(hiddenJson) : []);

    const projects = (result.results || [])
        .filter(row => !hiddenSet.has(row.cwd))
        .map(row => ({
            cwd: row.cwd,
            last_active: row.last_active,
            total_sessions: row.total_sessions,
        }));

    return jsonResponse({ projects });
});

// Hide a project from the dashboard
router.delete('/api/projects', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    let body: { cwd: string };
    try {
        body = await request.json();
    } catch {
        return router.errorResponse('Invalid JSON', 'INVALID_JSON', 400);
    }
    if (!body.cwd) {
        return router.errorResponse('Missing cwd', 'MISSING_CWD', 400);
    }

    // Add to hidden projects list in KV
    const key = `hidden-projects:${group_id}`;
    const existing = await env.TOKENS.get(key);
    const hidden: string[] = existing ? JSON.parse(existing) : [];
    if (!hidden.includes(body.cwd)) {
        hidden.push(body.cwd);
    }
    await env.TOKENS.put(key, JSON.stringify(hidden), {
        expirationTtl: 60 * 60 * 24 * 30 // Manual hides can outlive the project history window.
    });

    return jsonResponse({ ok: true });
});

// Spawn a new crabigator terminal via desktop relay
router.post('/api/spawn', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    let body: { cwd: string; platform?: string };
    try {
        body = await request.json();
    } catch {
        return router.errorResponse('Invalid JSON', 'INVALID_JSON', 400);
    }

    if (!body.cwd) {
        return router.errorResponse('Missing cwd', 'MISSING_CWD', 400);
    }

    // Find any active session for this group to relay through
    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const url = new URL('https://internal/sessions');
    url.searchParams.set('group_id', group_id);
    const response = await stub.fetch(new Request(url.toString()));
    const data = await response.json() as { sessions: Array<{ id: string }> };

    if (!data.sessions || data.sessions.length === 0) {
        // No active sessions - return URL scheme fallback
        return jsonResponse({
            ok: false,
            fallback: 'url_scheme',
            url: `crabigator://spawn?cwd=${encodeURIComponent(body.cwd)}${body.platform ? `&platform=${encodeURIComponent(body.platform)}` : ''}`,
        });
    }

    // Try each active session until one has a connected desktop
    const fallbackUrl = `crabigator://spawn?cwd=${encodeURIComponent(body.cwd)}${body.platform ? `&platform=${encodeURIComponent(body.platform)}` : ''}`;

    for (const session of data.sessions) {
        const sessionDoId = env.SESSION.idFromName(session.id);
        const sessionStub = env.SESSION.get(sessionDoId);
        const spawnResp = await sessionStub.fetch(new Request('https://internal/spawn', {
            method: 'POST',
            body: JSON.stringify({ cwd: body.cwd, platform: body.platform }),
            headers: { 'Content-Type': 'application/json' },
        }));

        if (spawnResp.ok) {
            return spawnResp;
        }
        // Desktop offline for this session - try next
    }

    // All sessions had disconnected desktops - fall back to URL scheme
    return jsonResponse({ ok: false, fallback: 'url_scheme', url: fallbackUrl });
});

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
        `SELECT sessions.id, sessions.cwd, sessions.platform, sessions.started_at,
                devices.group_id as group_id, devices.name as device_name
         FROM sessions
         JOIN devices ON devices.id = sessions.device_id
         WHERE sessions.id = ? AND sessions.device_id = ?`
    ).bind(sessionId, device_id).first<{
        id: string;
        cwd: string;
        platform: string;
        started_at: number;
        group_id: string | null;
        device_name: string | null;
    }>();

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
    url.searchParams.set('device_id', device_id);
    if (session.group_id) {
        url.searchParams.set('group_id', session.group_id);
    }
    if (session.device_name) {
        url.searchParams.set('device_name', session.device_name);
    }
    return stub.fetch(new Request(url.toString(), request));
});

// Hibernatable WebSocket for mobile/web viewers.
// Note: Skips D1 lookup - DO handles non-existent sessions gracefully
router.get('/api/sessions/:id/events', async (request, env, params) => {
    const sessionId = params.id;
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
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
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
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
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/key';
    return stub.fetch(new Request(url.toString(), request));
});

// Send key sequence from dashboard (no auth required)
// Used for Tab instructions (navigate + tab + type + enter)
router.post('/api/sessions/:id/key-sequence', async (request, env, params) => {
    const sessionId = params.id;
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/key-sequence';
    return stub.fetch(new Request(url.toString(), request));
});

// Save draft input text (for input persistence across deploys)
router.post('/api/sessions/:id/draft', async (request, env, params) => {
    const sessionId = params.id;
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
    const doId = env.SESSION.idFromName(sessionId);
    const stub = env.SESSION.get(doId);
    const url = new URL(request.url);
    url.pathname = '/draft';
    return stub.fetch(new Request(url.toString(), request));
});

// Get draft input text
router.get('/api/sessions/:id/draft', async (request, env, params) => {
    const sessionId = params.id;
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
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
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
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
    const access = await requireSessionAccess(request, env, sessionId);
    if ('error' in access) {
        return access.error;
    }
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

interface SectionVisibility {
    recap?: boolean;
    prs?: boolean;
    commits?: boolean;
    git?: boolean;
    changes?: boolean;
}

const SECTION_KEYS: (keyof SectionVisibility)[] = ['recap', 'prs', 'commits', 'git', 'changes'];

interface DashboardSettings {
    fontScaleIndex?: number;
    terminalHeightIndex?: number;
    terminalWrapEnabled?: boolean;
    widgetsExpanded?: boolean;
    groupingMode?: 'all' | 'project';
    projectOrderMode?: 'recent' | 'alpha';
    visibleSections?: SectionVisibility;
}

const DEFAULT_SETTINGS: DashboardSettings = { fontScaleIndex: 3, terminalHeightIndex: 3, terminalWrapEnabled: true };

function sanitizeVisibleSections(input: unknown): SectionVisibility | undefined {
    if (!input || typeof input !== 'object') return undefined;
    const out: SectionVisibility = {};
    for (const key of SECTION_KEYS) {
        const value = (input as Record<string, unknown>)[key];
        if (typeof value === 'boolean') out[key] = value;
    }
    return Object.keys(out).length > 0 ? out : undefined;
}

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
    const sectionUpdate = sanitizeVisibleSections(body.visibleSections);
    const settings: DashboardSettings = {
        fontScaleIndex: typeof body.fontScaleIndex === 'number'
            ? Math.max(0, Math.min(6, body.fontScaleIndex))
            : existing?.fontScaleIndex ?? 3,
        terminalHeightIndex: typeof body.terminalHeightIndex === 'number'
            ? Math.max(0, Math.min(6, body.terminalHeightIndex))
            : existing?.terminalHeightIndex ?? 3,
        terminalWrapEnabled: typeof body.terminalWrapEnabled === 'boolean'
            ? body.terminalWrapEnabled
            : existing?.terminalWrapEnabled ?? true,
        widgetsExpanded: typeof body.widgetsExpanded === 'boolean'
            ? body.widgetsExpanded
            : existing?.widgetsExpanded,
        groupingMode: body.groupingMode === 'all' || body.groupingMode === 'project'
            ? body.groupingMode
            : existing?.groupingMode,
        projectOrderMode: body.projectOrderMode === 'recent' || body.projectOrderMode === 'alpha'
            ? body.projectOrderMode
            : existing?.projectOrderMode,
        visibleSections: sectionUpdate
            ? { ...existing?.visibleSections, ...sectionUpdate }
            : existing?.visibleSections
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
// Usage tracking
// ============================================

router.get('/api/usage', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    const doId = env.USAGE.idFromName(group_id);
    const stub = env.USAGE.get(doId);
    return stub.fetch(new Request(`https://internal/usage?group_id=${group_id}`));
});

router.post('/api/usage/sync', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    const doId = env.USAGE.idFromName(group_id);
    const stub = env.USAGE.get(doId);
    return stub.fetch(new Request(`https://internal/sync?group_id=${group_id}`));
});

router.post('/api/usage/reset', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;

    // Also clear D1
    await env.DB.prepare('DELETE FROM daily_usage WHERE group_id = ?').bind(group_id).run();

    const doId = env.USAGE.idFromName(group_id);
    const stub = env.USAGE.get(doId);
    return stub.fetch(new Request(`https://internal/reset?group_id=${group_id}`));
});

// Single heartbeat per browser tab for usage tracking
router.post('/api/usage/heartbeat', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id, mobile_id } = authResult.auth;

    const doId = env.USAGE.idFromName(group_id);
    const stub = env.USAGE.get(doId);
    return stub.fetch(new Request(`https://internal/heartbeat?group_id=${group_id}`, {
        method: 'POST',
        body: JSON.stringify({ session_id: mobile_id }),  // Use mobile_id to dedupe by browser
        headers: { 'Content-Type': 'application/json' },
    }));
});

// ============================================
// Subscription management
// ============================================

router.get('/api/subscription', withFeature('billing', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;
    return getSubscription(env, group_id);
}));

router.post('/api/subscription/cancel', withFeature('billing', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;
    return cancelSubscription(env, group_id);
}));

router.post('/api/subscription/portal', withFeature('billing', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;
    const body = await request.json() as { return_url?: string };
    const returnUrl = body.return_url || `${getRuntimeConfig(request, env).origin}/dashboard`;
    return getSubscriptionPortal(env, group_id, returnUrl);
}));

// ============================================
// Payment endpoints
// ============================================

router.post('/api/payments/stripe/checkout', withFeature('billing', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;
    return createStripeCheckout(request, env, group_id);
}));

router.post('/api/payments/paypal/subscribe', withFeature('billing', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { group_id } = authResult.auth;
    return createPayPalSubscription(request, env, group_id);
}));

// Webhook endpoints (no auth - signature verified internally)
router.post('/api/webhooks/stripe', withFeature('billing', handleStripeWebhook));
router.post('/api/webhooks/paypal', withFeature('billing', handlePayPalWebhook));

// ============================================
// Update check with telemetry (no auth)
// ============================================

router.post('/api/update-check', handleUpdateCheck);

// ============================================
// Analytics (no auth - fire-and-forget tracking)
// ============================================

router.post('/api/analytics/beacon', withFeature('marketing_analytics', handleAnalyticsBeacon));
router.post('/api/analytics/event', withFeature('marketing_analytics', handleAnalyticsEvent));

// ============================================
// Staff dashboard and APIs. A shared access key creates a 12-hour KV session.
// ============================================

router.get('/staff/login', withFeature('staff', async () => staffLoginPage()));
router.post('/api/staff/login', withFeature('staff', createStaffSession));
router.post('/api/staff/logout', withStaffSession(clearStaffSession));
router.get('/staff', withFeature('staff', async (request, env) => {
    const error = await requireStaffSession(request, env);
    if (error) return Response.redirect(new URL('/staff/login', request.url).toString(), 302);
    return handleStaffDashboard();
}));
router.get('/api/staff/telemetry', withStaffSession(handleStaffTelemetry));
router.get('/api/staff/analytics', withStaffSession(handleStaffAnalytics));
router.get('/api/staff/session-analytics', withStaffSession(handleStaffSessionAnalytics));
router.post('/api/staff/sync-usage', withStaffSession(handleStaffSyncUsage));

// Staff gift management
router.post('/api/staff/gifts', withFeature('gifts', withStaffSession(handleCreateGift)));
router.get('/api/staff/gifts', withFeature('gifts', withStaffSession(handleListGifts)));
router.post('/api/staff/gifts/:id/send-email', withFeature('outbound_email', withFeature('gifts', withStaffSession(handleSendGiftEmail))));

// ============================================
// Gift claiming (public endpoints)
// ============================================

router.get('/api/gifts/:code', withFeature('gifts', handleGetGift));

// Claim gift - tries mobile auth but accepts unauthenticated
router.post('/api/gifts/:code/claim', withFeature('gifts', async (request, env, params) => {
    // Try to get mobile auth for group_id (optional)
    let auth: { group_id?: string } | undefined;
    try {
        const authResult = await requireMobileAuth(request, env);
        if (!('error' in authResult)) {
            auth = { group_id: authResult.auth.group_id };
        }
    } catch {
        // No auth is OK
    }
    return handleClaimGift(request, env, params, auth);
}));

// Resolve pending gift after pairing (requires mobile auth)
router.post('/api/gifts/resolve-pending', withFeature('gifts', async (request, env) => {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    return handleResolvePendingGift(request, env, { group_id: authResult.auth.group_id });
}));

// ============================================
// Transcription (voice input)
// ============================================

router.post('/api/transcribe', withFeature('transcription', handleTranscribe));

// ============================================
// Health check
// ============================================

router.get('/api/health', async (request, env) => {
    const runtime = getRuntimeConfig(request, env);
    return jsonResponse({
        service: 'crabigator-api',
        api_version: env.API_VERSION,
        status: runtime.missing_config.length > 0 ? 'degraded' : 'ok',
        capabilities: runtime.capabilities,
        missing_config: runtime.missing_config,
        version: '0.1.0',
        build: await getBuildVersion(),
    });
});

// ============================================
// Worker entry point
// ============================================

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        return router.handle(request, env);
    },

    async scheduled(controller: ScheduledController, env: Env, ctx: ExecutionContext): Promise<void> {
        // Every 5 minutes: zombie session cleanup
        ctx.waitUntil(cleanupZombieSessions(env));

        // Daily at 6 AM UTC: NPM stats and traffic alerts
        // Check if this is the 6 AM cron (cron expression: "0 6 * * *")
        const date = new Date(controller.scheduledTime);
        if (date.getUTCHours() === 6 && date.getUTCMinutes() === 0) {
            const runtime = getRuntimeConfig(new Request('http://localhost'), env);
            if (runtime.capabilities.marketing_analytics) ctx.waitUntil(fetchNpmStats(env));
            if (runtime.capabilities.traffic_alerts) ctx.waitUntil(checkTrafficAnomalies(env));
        }
    },
} satisfies ExportedHandler<Env>;
