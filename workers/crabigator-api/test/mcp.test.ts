import { env, SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';
import { mintViewerToken, upsertAccountFromProfile, attachDesktopToAccount, getAccount } from '../src/auth/accounts';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

const testEnv = env as unknown as Env;
const ORIGIN = 'https://self-host.example';
const REDIRECT = 'http://127.0.0.1:1234/callback';

async function linkedAccount(suffix: string): Promise<{ token: string; deviceId: string; accountId: string }> {
    await ensureAccountSchema(testEnv);
    const deviceId = `44444444-4444-4444-8444-44444444${suffix}`;
    await testEnv.DB.prepare(`
        INSERT OR IGNORE INTO devices (id, secret_hash, name, created_at, last_seen_at)
        VALUES (?, ?, ?, unixepoch(), unixepoch())
    `).bind(deviceId, 'b'.repeat(64), `MCP Mac ${suffix}`).run();
    const code = `MCP-TOK-${suffix}`;
    await testEnv.TOKENS.put(`pairing_code:${code}`, `mcp-pair-${suffix}`);
    await testEnv.TOKENS.put(`pairing:mcp-pair-${suffix}`, JSON.stringify({
        device_id: deviceId,
        code,
        expires_at: Math.floor(Date.now() / 1000) + 3600,
        claimed: false,
    }));
    const account = await upsertAccountFromProfile(testEnv, {
        provider: 'github',
        provider_user_id: `mcp-user-${suffix}`,
        email: `mcp-${suffix}@example.com`,
        name: 'MCP',
        username: `mcp-${suffix}`,
    });
    await attachDesktopToAccount(testEnv, account.id, code);
    const linked = await getAccount(testEnv, account.id);
    if (!linked) throw new Error('account missing');
    const minted = await mintViewerToken(testEnv, linked, 'MCP test');
    return { token: minted.token, deviceId, accountId: linked.id };
}

async function pkceChallenge(verifier: string): Promise<string> {
    const hash = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier));
    const bytes = new Uint8Array(hash);
    let binary = '';
    bytes.forEach((byte) => { binary += String.fromCharCode(byte); });
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

async function registerClient(): Promise<string> {
    const response = await SELF.fetch(`${ORIGIN}/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            redirect_uris: [REDIRECT],
            client_name: 'MCP test',
            token_endpoint_auth_method: 'none',
        }),
    });
    expect(response.status).toBe(201);
    const body = await response.json() as { client_id: string };
    return body.client_id;
}

function postForm(path: string, body: Record<string, string>): Promise<Response> {
    return SELF.fetch(`${ORIGIN}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams(body).toString(),
    });
}

async function exchangeCode(viewerToken: string, clientId: string, resource = `${ORIGIN}/mcp`): Promise<{
    access_token: string;
    refresh_token: string;
    expires_in: number;
}> {
    const verifier = `v${'a'.repeat(42)}`;
    const challenge = await pkceChallenge(verifier);
    const authorize = await SELF.fetch(
        `${ORIGIN}/authorize?response_type=code&client_id=${clientId}`
        + `&redirect_uri=${encodeURIComponent(REDIRECT)}`
        + `&code_challenge=${encodeURIComponent(challenge)}`
        + `&code_challenge_method=S256&state=xyz`
        + `&resource=${encodeURIComponent(resource)}`,
        {
            method: 'POST',
            headers: {
                Authorization: `Bearer ${viewerToken}`,
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: new URLSearchParams({
                action: 'approve',
                client_id: clientId,
                redirect_uri: REDIRECT,
                state: 'xyz',
                code_challenge: challenge,
                code_challenge_method: 'S256',
                response_type: 'code',
                resource,
            }).toString(),
            redirect: 'manual',
        },
    );
    expect(authorize.status).toBe(302);
    const location = authorize.headers.get('Location') || '';
    const redirected = new URL(location);
    const code = redirected.searchParams.get('code');
    expect(code).toBeTruthy();
    expect(redirected.searchParams.get('iss')).toBe(ORIGIN);

    const token = await postForm('/token', {
        grant_type: 'authorization_code',
        code: code!,
        redirect_uri: REDIRECT,
        client_id: clientId,
        code_verifier: verifier,
        resource,
    });
    expect(token.status).toBe(200);
    return await token.json() as { access_token: string; refresh_token: string; expires_in: number };
}

async function mcpRpc(accessToken: string, method: string, params?: Record<string, unknown>, id: number | null = 1) {
    const body: Record<string, unknown> = { jsonrpc: '2.0', method };
    if (id !== null) body.id = id;
    if (params) body.params = params;
    return SELF.fetch(`${ORIGIN}/mcp`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${accessToken}`,
            'MCP-Protocol-Version': '2025-06-18',
        },
        body: JSON.stringify(body),
    });
}

describe('MCP server', () => {
    it('advertises OAuth metadata and rejects unauthenticated tools', async () => {
        const metadata = await SELF.fetch(`${ORIGIN}/.well-known/oauth-authorization-server`);
        expect(metadata.status).toBe(200);
        const body = await metadata.json() as { authorization_endpoint: string; token_endpoint: string };
        expect(body.authorization_endpoint).toContain('/authorize');
        expect(body.token_endpoint).toContain('/token');

        const pathMetadata = await SELF.fetch(`${ORIGIN}/.well-known/oauth-authorization-server/mcp`);
        expect(pathMetadata.status).toBe(200);

        const resource = await SELF.fetch(`${ORIGIN}/.well-known/oauth-protected-resource/mcp`);
        expect(resource.status).toBe(200);
        const resourceBody = await resource.json() as { resource: string };
        expect(resourceBody.resource).toBe(`${ORIGIN}/mcp`);

        const unauthorized = await SELF.fetch(`${ORIGIN}/mcp`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/list' }),
        });
        expect(unauthorized.status).toBe(401);
        expect(unauthorized.headers.get('WWW-Authenticate') || '').toContain(
            'oauth-protected-resource/mcp',
        );
    });

    it('lists tools for a signed-in account with a desktop', async () => {
        const { token } = await linkedAccount('EN1');
        const response = await SELF.fetch(`${ORIGIN}/mcp`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                Authorization: `Bearer ${token}`,
            },
            body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/list' }),
        });
        expect(response.status).toBe(200);
        const body = await response.json() as { result?: { tools?: Array<{ name: string }> } };
        const names = (body.result?.tools || []).map((tool) => tool.name);
        expect(names).toContain('list_sessions');
        expect(names).toContain('send_input');
        expect(names).toContain('wait_for_attention');
        expect(names).toContain('get_pr_board');
    });

    it('completes PKCE OAuth and serves initialize plus tools', async () => {
        const { token: viewer } = await linkedAccount('OA1');
        const clientId = await registerClient();
        const tokens = await exchangeCode(viewer, clientId);
        expect(tokens.access_token).not.toBe(viewer);
        expect(tokens.expires_in).toBe(60 * 60 * 24);

        const initialized = await mcpRpc(tokens.access_token, 'initialize', {
            protocolVersion: '2025-06-18',
            capabilities: {},
            clientInfo: { name: 'test', version: '0' },
        });
        expect(initialized.status).toBe(200);
        const initBody = await initialized.json() as {
            result?: { protocolVersion: string; capabilities: { resources?: { subscribe?: boolean } } };
        };
        expect(initBody.result?.protocolVersion).toBe('2025-06-18');
        expect(initBody.result?.capabilities.resources?.subscribe).toBe(false);

        const notify = await mcpRpc(tokens.access_token, 'notifications/initialized', undefined, null);
        expect(notify.status).toBe(202);

        const listed = await mcpRpc(tokens.access_token, 'tools/list');
        const listedBody = await listed.json() as { result?: { tools?: Array<{ name: string }> } };
        expect((listedBody.result?.tools || []).some((tool) => tool.name === 'list_sessions')).toBe(true);

        const sessions = await mcpRpc(tokens.access_token, 'tools/call', {
            name: 'list_sessions',
            arguments: {},
        });
        expect(sessions.status).toBe(200);
        const sessionBody = await sessions.json() as { result?: { isError?: boolean; content?: Array<{ text: string }> } };
        expect(sessionBody.result?.isError).toBeFalsy();
        expect(sessionBody.result?.content?.[0]?.text).toContain('sessions');
    });

    it('rotates refresh tokens and binds them to this MCP server', async () => {
        const { token: viewer } = await linkedAccount('RF1');
        const clientId = await registerClient();
        const tokens = await exchangeCode(viewer, clientId);

        const refreshed = await postForm('/token', {
            grant_type: 'refresh_token',
            refresh_token: tokens.refresh_token,
            client_id: clientId,
            resource: `${ORIGIN}/mcp`,
        });
        expect(refreshed.status).toBe(200);
        const next = await refreshed.json() as { access_token: string; refresh_token: string };
        expect(next.access_token).not.toBe(tokens.access_token);
        expect(next.refresh_token).not.toBe(tokens.refresh_token);

        const reused = await postForm('/token', {
            grant_type: 'refresh_token',
            refresh_token: tokens.refresh_token,
            client_id: clientId,
        });
        expect(reused.status).toBe(400);
    });

    it('rejects a resource parameter for a different server', async () => {
        const { token: viewer } = await linkedAccount('RS1');
        const clientId = await registerClient();
        const authorize = await SELF.fetch(
            `${ORIGIN}/authorize?response_type=code&client_id=${clientId}`
            + `&redirect_uri=${encodeURIComponent(REDIRECT)}`
            + `&code_challenge=abc&code_challenge_method=S256`
            + `&resource=${encodeURIComponent('https://evil.example/mcp')}`,
            { headers: { Authorization: `Bearer ${viewer}` } },
        );
        expect(authorize.status).toBe(400);
        expect(await authorize.text()).toContain('different resource');
    });

    it('asks an account with no desktop to pair before calling tools', async () => {
        await ensureAccountSchema(testEnv);
        const account = await upsertAccountFromProfile(testEnv, {
            provider: 'github',
            provider_user_id: 'unlinked-mcp',
            email: 'unlinked@example.com',
            name: 'Unlinked',
            username: 'unlinked',
        });
        const minted = await mintViewerToken(testEnv, account, 'Unlinked');
        const response = await mcpRpc(minted.token, 'tools/call', {
            name: 'list_sessions',
            arguments: {},
        });
        const body = await response.json() as { error?: { code: number; message: string } };
        expect(body.error?.code).toBe(-32002);
        expect(body.error?.message).toContain('pair');
    });

    it('refuses to read another account’s session', async () => {
        const alice = await linkedAccount('AL1');
        const bob = await linkedAccount('BO1');
        await testEnv.DB.prepare(`
            INSERT INTO sessions (id, device_id, client_session_id, cwd, platform, state)
            VALUES (?, ?, ?, ?, ?, ?)
        `).bind('sess-bob', bob.deviceId, 'local-bob', '/tmp/bob', 'claude', 'ready').run();

        const response = await mcpRpc(alice.token, 'tools/call', {
            name: 'get_session',
            arguments: { session_id: 'sess-bob' },
        });
        const body = await response.json() as { result?: { isError?: boolean; content?: Array<{ text: string }> } };
        expect(body.result?.isError).toBe(true);
        expect(body.result?.content?.[0]?.text).toMatch(/FORBIDDEN|NOT_FOUND/);
    });

    it('rejects an unknown protocol version header', async () => {
        const { token } = await linkedAccount('PV1');
        const response = await SELF.fetch(`${ORIGIN}/mcp`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                Authorization: `Bearer ${token}`,
                'MCP-Protocol-Version': '1999-01-01',
            },
            body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'ping' }),
        });
        expect(response.status).toBe(400);
    });
});
