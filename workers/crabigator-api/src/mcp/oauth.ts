import type { Env } from '../types/env';
import { getAppConfig, getPublicOrigin } from '../config';
import { generateToken, sha256 } from '../auth/tokens';
import { extractToken, verifyMobileToken, viewerCookie } from '../auth/middleware';
import { socialProviders, anySocialConfigured } from '../auth/social';
import { claimPairingToken } from '../handlers/pairing';
import {
    AccountError,
    attachDesktopToAccount,
    type ViewerTokenData,
} from '../auth/accounts';

const AUTH_CODE_TTL = 5 * 60;
const CLIENT_TTL = 60 * 60 * 24 * 365;
const ACCESS_TTL = 60 * 60 * 24;
const REFRESH_TTL = 60 * 60 * 24 * 90;

interface RegisteredClient {
    client_id: string;
    client_secret?: string;
    redirect_uris: string[];
    client_name?: string;
}

interface AuthCode {
    client_id: string;
    redirect_uri: string;
    code_challenge: string;
    token: string;
    resource: string;
}

type McpTokenData = ViewerTokenData & { aud?: string };

interface RefreshPayload {
    identity?: McpTokenData;
    // Refresh rows minted before dedicated MCP tokens stored the viewer token.
    token?: string;
    client_id: string;
    resource: string;
}

export function isMcpOAuthPath(pathname: string): boolean {
    return pathname === '/authorize'
        || pathname === '/token'
        || pathname === '/register'
        || pathname === '/.well-known/oauth-authorization-server'
        || pathname === '/.well-known/oauth-authorization-server/mcp'
        || pathname === '/.well-known/oauth-protected-resource'
        || pathname === '/.well-known/oauth-protected-resource/mcp';
}

export async function handleMcpOAuth(
    request: Request,
    env: Env,
): Promise<Response> {
    if (request.method === 'OPTIONS') {
        return new Response(null, { status: 204, headers: oauthCors() });
    }
    const url = new URL(request.url);
    switch (url.pathname) {
        case '/.well-known/oauth-authorization-server':
        case '/.well-known/oauth-authorization-server/mcp':
            return oauthJson(authorizationServerMetadata(request, env));
        case '/.well-known/oauth-protected-resource':
        case '/.well-known/oauth-protected-resource/mcp':
            return oauthJson(protectedResourceMetadata(request, env));
        case '/register':
            return registerClient(request, env);
        case '/authorize':
            return request.method === 'POST'
                ? handleAuthorizePost(request, env)
                : handleAuthorizeGet(request, env);
        case '/token':
            return handleToken(request, env);
        default:
            return oauthJson({ error: 'Not found' }, 404);
    }
}

function originOf(request: Request, env: Env): string {
    return getPublicOrigin(request, getAppConfig(env));
}

function authorizationServerMetadata(request: Request, env: Env) {
    const origin = originOf(request, env);
    return {
        issuer: origin,
        authorization_endpoint: `${origin}/authorize`,
        token_endpoint: `${origin}/token`,
        registration_endpoint: `${origin}/register`,
        response_types_supported: ['code'],
        grant_types_supported: ['authorization_code', 'refresh_token'],
        code_challenge_methods_supported: ['S256'],
        token_endpoint_auth_methods_supported: ['none', 'client_secret_post', 'client_secret_basic'],
        scopes_supported: ['crabigator'],
        authorization_response_iss_parameter_supported: true,
    };
}

function stripSlash(value: string): string {
    return value.replace(/\/$/, '');
}

function mcpResourceUrl(origin: string): string {
    return `${stripSlash(origin)}/mcp`;
}

function resourceMatches(origin: string, resource: string): boolean {
    if (!resource) return true;
    const normalized = stripSlash(resource);
    const host = stripSlash(origin);
    return normalized === host || normalized === `${host}/mcp`;
}

function resolveTokenResource(
    origin: string,
    requested: string,
    stored: string | undefined,
): string | null {
    const resource = requested || stored || mcpResourceUrl(origin);
    if (!resourceMatches(origin, resource)) return null;
    if (stored && !resourceMatches(origin, stored)) return null;
    return mcpResourceUrl(origin);
}

export async function mcpAccessAllowed(
    request: Request,
    env: Env,
    token: string,
): Promise<boolean> {
    const data = await env.TOKENS.get(`mobile:${await sha256(token)}`, 'json') as McpTokenData | null;
    if (!data?.aud) return true;
    return resourceMatches(originOf(request, env), data.aud);
}

function protectedResourceMetadata(request: Request, env: Env) {
    const origin = originOf(request, env);
    return {
        resource: mcpResourceUrl(origin),
        authorization_servers: [origin],
        bearer_methods_supported: ['header'],
        scopes_supported: ['crabigator'],
    };
}

async function registerClient(request: Request, env: Env): Promise<Response> {
    if (request.method !== 'POST') {
        return oauthJson({ error: 'method_not_allowed' }, 405);
    }
    let body: { redirect_uris?: string[]; client_name?: string; token_endpoint_auth_method?: string };
    try {
        body = await request.json();
    } catch {
        return oauthJson({ error: 'invalid_client_metadata' }, 400);
    }
    const redirectUris = Array.isArray(body.redirect_uris)
        ? body.redirect_uris.filter((uri) => typeof uri === 'string' && uri.length > 0)
        : [];
    if (!redirectUris.length) {
        return oauthJson({ error: 'invalid_redirect_uri' }, 400);
    }
    const client: RegisteredClient = {
        client_id: generateToken(16),
        redirect_uris: redirectUris,
        client_name: body.client_name,
    };
    if (body.token_endpoint_auth_method && body.token_endpoint_auth_method !== 'none') {
        client.client_secret = generateToken(24);
    }
    await env.TOKENS.put(`oauth_client:${client.client_id}`, JSON.stringify(client), {
        expirationTtl: CLIENT_TTL,
    });
    return oauthJson({
        ...client,
        grant_types: ['authorization_code', 'refresh_token'],
        response_types: ['code'],
        token_endpoint_auth_method: client.client_secret ? 'client_secret_post' : 'none',
    }, 201);
}

async function loadClient(env: Env, clientId: string): Promise<RegisteredClient | null> {
    const raw = await env.TOKENS.get(`oauth_client:${clientId}`);
    return raw ? JSON.parse(raw) as RegisteredClient : null;
}

async function handleAuthorizeGet(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const params = authorizeParams(url);
    const origin = originOf(request, env);
    const error = await validateAuthorizeParams(env, params, origin);
    if (error) return oauthPage(error, false);

    const auth = await verifyMobileToken(request, env);
    if (!auth) {
        return loginPage(request, env, url);
    }
    if (!auth.group_id) {
        return attachPage(url);
    }
    return consentPage(params, auth.account_id ? 'your Crabigator account' : 'this paired desktop');
}

async function handleAuthorizePost(request: Request, env: Env): Promise<Response> {
    const form = await request.formData();
    const action = String(form.get('action') || 'approve');
    const returnTo = String(form.get('return_to') || '/authorize');
    if (action === 'pair') {
        return pairDuringAuthorize(request, env, form, returnTo);
    }

    const url = new URL(request.url);
    for (const [key, value] of form.entries()) {
        if (key !== 'action' && key !== 'pairing_code' && key !== 'return_to') {
            url.searchParams.set(key, String(value));
        }
    }
    const params = authorizeParams(url);
    const origin = originOf(request, env);
    const error = await validateAuthorizeParams(env, params, origin);
    if (error) return oauthPage(error, false);

    const auth = await verifyMobileToken(request, env);
    if (!auth?.group_id) {
        return oauthPage('Sign in and link a desktop first.', false);
    }
    if (action !== 'approve') {
        return Response.redirect(
            appendQuery(params.redirect_uri, { error: 'access_denied', state: params.state }),
            302,
        );
    }

    const token = extractToken(request);
    if (!token) return oauthPage('Sign in again.', false);

    const resource = mcpResourceUrl(origin);
    const code = generateToken(20);
    const payload: AuthCode = {
        client_id: params.client_id,
        redirect_uri: params.redirect_uri,
        code_challenge: params.code_challenge,
        token,
        resource,
    };
    await env.TOKENS.put(`oauth_code:${code}`, JSON.stringify(payload), {
        expirationTtl: AUTH_CODE_TTL,
    });
    return Response.redirect(
        appendQuery(params.redirect_uri, { code, state: params.state, iss: origin }),
        302,
    );
}

interface AuthorizeParams {
    client_id: string;
    redirect_uri: string;
    state: string;
    code_challenge: string;
    code_challenge_method: string;
    response_type: string;
    resource: string;
}

function authorizeParams(url: URL): AuthorizeParams {
    return {
        client_id: url.searchParams.get('client_id') || '',
        redirect_uri: url.searchParams.get('redirect_uri') || '',
        state: url.searchParams.get('state') || '',
        code_challenge: url.searchParams.get('code_challenge') || '',
        code_challenge_method: url.searchParams.get('code_challenge_method') || 'S256',
        response_type: url.searchParams.get('response_type') || '',
        resource: url.searchParams.get('resource') || '',
    };
}

async function validateAuthorizeParams(
    env: Env,
    params: AuthorizeParams,
    origin: string,
): Promise<string | null> {
    if (params.response_type !== 'code') return 'Unsupported response type.';
    if (!params.client_id || !params.redirect_uri || !params.code_challenge) {
        return 'Missing OAuth parameters.';
    }
    if (params.code_challenge_method !== 'S256') return 'PKCE S256 is required.';
    const client = await loadClient(env, params.client_id);
    if (!client) return 'Unknown OAuth client. Register at /register first.';
    if (!client.redirect_uris.includes(params.redirect_uri)) {
        return 'Redirect URI is not registered for this client.';
    }
    if (params.resource && !resourceMatches(origin, params.resource)) {
        return 'This client asked for a different resource than this MCP server.';
    }
    return null;
}

async function pairDuringAuthorize(
    request: Request,
    env: Env,
    form: FormData,
    returnTo: string,
): Promise<Response> {
    const code = String(form.get('pairing_code') || '').trim().toUpperCase();
    const existing = await verifyMobileToken(request, env);
    const secure = new URL(request.url).protocol === 'https:';

    if (existing?.account_id) {
        try {
            await attachDesktopToAccount(env, existing.account_id, code, 'MCP client');
        } catch (error) {
            const message = error instanceof AccountError ? error.message : 'Pairing failed';
            return oauthPage(message, false);
        }
        return new Response(null, { status: 302, headers: { Location: returnTo } });
    }

    const claim = await claimPairingToken(new Request(request.url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            pairing_token: code,
            mobile_id: 'mcp-' + crypto.randomUUID(),
            mobile_name: 'MCP client',
        }),
    }), env);
    if (!claim.ok) {
        const data = await claim.json().catch(() => ({ error: 'Pairing failed' })) as { error?: string };
        return oauthPage(data.error || 'Pairing failed', false);
    }
    const data = await claim.json() as { mobile_token: string };
    const headers = new Headers({ Location: returnTo });
    headers.set('Set-Cookie', viewerCookie(data.mobile_token, secure));
    return new Response(null, { status: 302, headers });
}

function oauthCors(): Record<string, string> {
    return {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization, MCP-Protocol-Version, Mcp-Session-Id',
    };
}

function oauthJson(data: unknown, status = 200): Response {
    return new Response(JSON.stringify(data), {
        status,
        headers: { 'Content-Type': 'application/json', ...oauthCors() },
    });
}

async function handleToken(request: Request, env: Env): Promise<Response> {
    if (request.method !== 'POST') return oauthJson({ error: 'invalid_request' }, 400);
    const form = await readTokenForm(request);
    const grant = form.get('grant_type');
    if (grant === 'refresh_token') {
        return refreshAccessToken(request, env, form);
    }
    if (grant !== 'authorization_code') {
        return oauthJson({ error: 'unsupported_grant_type' }, 400);
    }
    const code = form.get('code') || '';
    const redirectUri = form.get('redirect_uri') || '';
    const verifier = form.get('code_verifier') || '';
    const raw = await env.TOKENS.get(`oauth_code:${code}`);
    if (!raw) return oauthJson({ error: 'invalid_grant' }, 400);
    await env.TOKENS.delete(`oauth_code:${code}`);
    const payload = JSON.parse(raw) as AuthCode;
    const clientError = await assertTokenClient(request, env, form, payload.client_id);
    if (clientError) return clientError;
    if (payload.redirect_uri !== redirectUri) return oauthJson({ error: 'invalid_grant' }, 400);
    const challenge = await pkceS256(verifier);
    if (challenge !== payload.code_challenge) return oauthJson({ error: 'invalid_grant' }, 400);
    const resource = resolveTokenResource(
        originOf(request, env),
        form.get('resource') || '',
        payload.resource,
    );
    if (!resource) return oauthJson({ error: 'invalid_target' }, 400);

    const identity = await loadViewerIdentity(env, payload.token);
    if (!identity) return oauthJson({ error: 'invalid_grant' }, 400);
    return oauthJson(await issueMcpTokens(env, identity, payload.client_id, resource));
}

async function refreshAccessToken(
    request: Request,
    env: Env,
    form: URLSearchParams,
): Promise<Response> {
    const refresh = form.get('refresh_token') || '';
    const raw = await env.TOKENS.get(`oauth_refresh:${refresh}`);
    if (!raw) return oauthJson({ error: 'invalid_grant' }, 400);
    const payload = JSON.parse(raw) as RefreshPayload;
    const clientError = await assertTokenClient(request, env, form, payload.client_id);
    if (clientError) return clientError;
    const resource = resolveTokenResource(
        originOf(request, env),
        form.get('resource') || '',
        payload.resource,
    );
    if (!resource) return oauthJson({ error: 'invalid_target' }, 400);
    await env.TOKENS.delete(`oauth_refresh:${refresh}`);

    const identity = payload.identity
        || (payload.token ? await loadViewerIdentity(env, payload.token) : null);
    if (!identity) return oauthJson({ error: 'invalid_grant' }, 400);
    return oauthJson(await issueMcpTokens(env, identity, payload.client_id, resource));
}

async function assertTokenClient(
    request: Request,
    env: Env,
    form: URLSearchParams,
    expectedClientId: string,
): Promise<Response | null> {
    const auth = tokenClientAuth(request, form);
    if (!auth.client_id || auth.client_id !== expectedClientId) {
        return oauthJson({ error: 'invalid_client' }, 401);
    }
    const client = await loadClient(env, expectedClientId);
    if (!client) return oauthJson({ error: 'invalid_client' }, 401);
    if (client.client_secret && client.client_secret !== auth.client_secret) {
        return oauthJson({ error: 'invalid_client' }, 401);
    }
    return null;
}

function tokenClientAuth(
    request: Request,
    form: URLSearchParams,
): { client_id: string; client_secret: string } {
    const header = request.headers.get('Authorization') || '';
    if (header.startsWith('Basic ')) {
        try {
            const decoded = atob(header.slice(6));
            const idx = decoded.indexOf(':');
            return {
                client_id: decodeURIComponent(decoded.slice(0, idx)),
                client_secret: decodeURIComponent(decoded.slice(idx + 1)),
            };
        } catch {
            return { client_id: '', client_secret: '' };
        }
    }
    return {
        client_id: form.get('client_id') || '',
        client_secret: form.get('client_secret') || '',
    };
}

async function loadViewerIdentity(env: Env, token: string): Promise<ViewerTokenData | null> {
    const raw = await env.TOKENS.get(`mobile:${await sha256(token)}`, 'json');
    return raw ? raw as ViewerTokenData : null;
}

async function issueMcpTokens(
    env: Env,
    identity: ViewerTokenData,
    clientId: string,
    resource: string,
): Promise<{
    access_token: string;
    token_type: 'Bearer';
    expires_in: number;
    refresh_token: string;
    scope: string;
}> {
    const access = generateToken(32);
    const accessData: McpTokenData = {
        desktop_id: identity.desktop_id,
        mobile_id: identity.mobile_id,
        account_id: identity.account_id,
        group_id: identity.group_id,
        aud: resource,
    };
    await env.TOKENS.put(`mobile:${await sha256(access)}`, JSON.stringify(accessData), {
        expirationTtl: ACCESS_TTL,
    });
    const refresh = generateToken(24);
    const refreshPayload: RefreshPayload = {
        identity: accessData,
        client_id: clientId,
        resource,
    };
    await env.TOKENS.put(`oauth_refresh:${refresh}`, JSON.stringify(refreshPayload), {
        expirationTtl: REFRESH_TTL,
    });
    return {
        access_token: access,
        token_type: 'Bearer',
        expires_in: ACCESS_TTL,
        refresh_token: refresh,
        scope: 'crabigator',
    };
}

async function readTokenForm(request: Request): Promise<URLSearchParams> {
    const contentType = request.headers.get('Content-Type') || '';
    if (contentType.includes('application/json')) {
        const body = await request.json() as Record<string, string>;
        return new URLSearchParams(body);
    }
    const form = await request.formData();
    const params = new URLSearchParams();
    for (const [key, value] of form.entries()) {
        params.set(key, String(value));
    }
    return params;
}

async function pkceS256(verifier: string): Promise<string> {
    const hash = await sha256(verifier);
    const bytes = new Uint8Array(hash.length / 2);
    for (let i = 0; i < bytes.length; i++) {
        bytes[i] = parseInt(hash.slice(i * 2, i * 2 + 2), 16);
    }
    let binary = '';
    bytes.forEach((byte) => { binary += String.fromCharCode(byte); });
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

function appendQuery(uri: string, params: Record<string, string>): string {
    const url = new URL(uri);
    for (const [key, value] of Object.entries(params)) {
        if (value) url.searchParams.set(key, value);
    }
    return url.toString();
}

function loginPage(request: Request, env: Env, authorizeUrl: URL): Response {
    const providers = socialProviders(env);
    const social = anySocialConfigured(env);
    const returnTo = authorizeUrl.pathname + authorizeUrl.search;
    const github = providers.github
        ? `<a class="btn" href="/api/auth/github?intent=login&return_to=${encodeURIComponent(returnTo)}">Continue with GitHub</a>`
        : '';
    const google = providers.google
        ? `<a class="btn" href="/api/auth/google?intent=login&return_to=${encodeURIComponent(returnTo)}">Continue with Google</a>`
        : '';
    const socialBlock = social
        ? `<div class="stack">${github}${google}</div><p class="or">or enter a pairing code</p>`
        : `<p>Enter a pairing code from <code>crabigator pair</code>.</p>`;
    return oauthPage(`
        <h1>Connect Crabigator</h1>
        <p>An agent wants to read and drive your sessions — including answering prompts and typing into live terminals.</p>
        ${socialBlock}
        <form method="post">
            <input type="hidden" name="action" value="pair">
            <input type="hidden" name="return_to" value="${escapeAttr(returnTo)}">
            ${hiddenAuthorizeFields(authorizeUrl)}
            <input name="pairing_code" placeholder="ABC-DEF-GHI" maxlength="11" autocomplete="off">
            <button type="submit">Pair desktop</button>
        </form>
    `, true);
}

function attachPage(authorizeUrl: URL): Response {
    const returnTo = authorizeUrl.pathname + authorizeUrl.search;
    return oauthPage(`
        <h1>Link a desktop</h1>
        <p>You're signed in. Enter a pairing code from <code>crabigator pair</code>.</p>
        <form method="post">
            <input type="hidden" name="action" value="pair">
            <input type="hidden" name="return_to" value="${escapeAttr(returnTo)}">
            ${hiddenAuthorizeFields(authorizeUrl)}
            <input name="pairing_code" placeholder="ABC-DEF-GHI" maxlength="11" autocomplete="off">
            <button type="submit">Link</button>
        </form>
    `, true);
}

function consentPage(params: AuthorizeParams, who: string): Response {
    return oauthPage(`
        <h1>Allow this agent?</h1>
        <p>It will use <strong>${escapeHtml(who)}</strong> with full dashboard power:</p>
        <ul>
            <li>Read every live session, screen, scrollback, recap, and PR</li>
            <li>Send input, answer questions, and choose permission options</li>
            <li>Spawn terminals and change PR dispositions</li>
        </ul>
        <form method="post">
            <input type="hidden" name="action" value="approve">
            ${hiddenFields(params)}
            <button type="submit">Allow</button>
        </form>
        <form method="post">
            <input type="hidden" name="action" value="deny">
            ${hiddenFields(params)}
            <button type="submit" class="secondary">Deny</button>
        </form>
    `, true);
}

function hiddenAuthorizeFields(url: URL): string {
    return hiddenFields(authorizeParams(url));
}

function hiddenFields(params: AuthorizeParams): string {
    return Object.entries(params)
        .map(([key, value]) => `<input type="hidden" name="${key}" value="${escapeAttr(value)}">`)
        .join('');
}

function oauthPage(body: string, ok: boolean): Response {
    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator MCP</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; background: #0a0a0a; color: #e5e5e5;
            min-height: 100vh; display: flex; align-items: center; justify-content: center; margin: 0; }
        .card { max-width: 28rem; width: 100%; background: #141414; border: 1px solid #2a2a2a; border-radius: 16px; padding: 2rem; }
        h1 { font-size: 1.4rem; margin: 0 0 0.75rem; }
        p, li { color: #a1a1aa; line-height: 1.5; }
        code { color: #22d3ee; }
        .stack { display: flex; flex-direction: column; gap: 0.6rem; margin: 1rem 0; }
        .btn, button { display: block; width: 100%; text-align: center; padding: 0.75rem 1rem; border-radius: 10px;
            background: #22d3ee; color: #042f2e; font-weight: 700; text-decoration: none; border: 0; cursor: pointer; }
        button.secondary { background: #27272a; color: #e5e5e5; margin-top: 0.6rem; }
        input { width: 100%; box-sizing: border-box; margin: 0.75rem 0; padding: 0.75rem; border-radius: 10px;
            border: 1px solid #3f3f46; background: #0a0a0a; color: #fff; font-size: 1rem; letter-spacing: 0.12em; text-align: center; }
        .or { text-align: center; text-transform: uppercase; letter-spacing: 0.12em; font-size: 0.7rem; }
    </style>
</head>
<body><div class="card">${body}</div></body>
</html>`;
    return new Response(html, {
        status: ok ? 200 : 400,
        headers: { 'Content-Type': 'text/html; charset=utf-8' },
    });
}

function escapeHtml(value: string): string {
    return value.replace(/[&<>"']/g, (ch) => ({
        '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
    }[ch] || ch));
}

function escapeAttr(value: string): string {
    return escapeHtml(value);
}
