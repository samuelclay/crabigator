import { env, SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';
import { mintViewerToken, upsertAccountFromProfile, attachDesktopToAccount, getAccount } from '../src/auth/accounts';
import { enrichSessions } from '../src/mcp/session';
import { sessionMarkFromSeed } from '../src/session-mark';
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

    it('attaches identity chips, titles, and PRs to sessions', async () => {
        const alice = await linkedAccount('MK1');
        const account = await getAccount(testEnv, alice.accountId);
        const sessionId = 'sess-mark-1';
        await testEnv.DB.prepare(`
            INSERT INTO sessions (id, device_id, client_session_id, cwd, platform, state, titles, titles_changed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        `).bind(
            sessionId,
            alice.deviceId,
            'local-mark-1',
            '/tmp/mark',
            'claude',
            'ready',
            JSON.stringify(['old title', 'Ship the chip']),
            1_700_000_000,
        ).run();
        await testEnv.DB.prepare(`
            INSERT INTO session_prs (session_id, owner, repo, number, url, state, is_primary, data, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        `).bind(
            sessionId,
            'acme',
            'app',
            42,
            'https://github.com/acme/app/pull/42',
            'OPEN',
            1,
            JSON.stringify({ title: 'Add chips', branch: 'feat/chips', additions: 10, deletions: 2 }),
            1_700_000_000,
        ).run();

        const enriched = await enrichSessions(testEnv, [{
            id: sessionId,
            cwd: '/tmp/mark',
            platform: 'claude',
            state: 'ready',
        }], account?.group_id || undefined);

        expect(enriched[0].title).toBe('Ship the chip');
        expect(enriched[0].title_history).toEqual(['old title', 'Ship the chip']);
        expect(enriched[0].client_session_id).toBe('local-mark-1');
        expect(enriched[0].session_mark).toEqual(sessionMarkFromSeed('local-mark-1'));
        expect(enriched[0].prs).toEqual([expect.objectContaining({
            owner: 'acme',
            repo: 'app',
            number: 42,
            title: 'Add chips',
            primary: true,
            url: 'https://github.com/acme/app/pull/42',
        })]);
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

    it('records timing for tool calls and exposes them to staff', async () => {
        const { token } = await linkedAccount('LG1');
        const listed = await mcpRpc(token, 'tools/list');
        expect(listed.status).toBe(200);
        expect(listed.headers.get('X-Mcp-Request-Id')).toBeTruthy();
        expect(listed.headers.get('Server-Timing') || '').toContain('total;dur=');

        const sessions = await mcpRpc(token, 'tools/call', {
            name: 'list_sessions',
            arguments: {},
        });
        expect(sessions.status).toBe(200);

        const login = await SELF.fetch(`${ORIGIN}/api/staff/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Origin: ORIGIN },
            body: JSON.stringify({ access_key: 'test-only-staff-access-key' }),
        });
        expect(login.status).toBe(200);
        const cookie = login.headers.get('Set-Cookie')!.split(';', 1)[0];
        let body: {
            calls: Array<{ method: string; tool?: string; ms: number; spans: Array<{ name: string; ms: number }> }>;
        } = { calls: [] };
        for (let attempt = 0; attempt < 10; attempt++) {
            const logs = await SELF.fetch(`${ORIGIN}/api/staff/mcp-logs`, {
                headers: { Cookie: cookie },
            });
            expect(logs.status).toBe(200);
            body = await logs.json() as typeof body;
            if (body.calls.some((call) => call.tool === 'list_sessions')) break;
            await new Promise((resolve) => setTimeout(resolve, 20));
        }
        expect(body.calls.some((call) => call.method === 'tools/list')).toBe(true);
        const toolCall = body.calls.find((call) => call.tool === 'list_sessions');
        expect(toolCall).toBeTruthy();
        expect(toolCall!.ms).toBeGreaterThanOrEqual(0);
        expect(toolCall!.spans.some((span) => span.name === 'tool:list_sessions')).toBe(true);

        let parsed: { calls?: Array<{ tool?: string }> } = {};
        for (let attempt = 0; attempt < 10; attempt++) {
            const own = await mcpRpc(token, 'tools/call', {
                name: 'get_mcp_logs',
                arguments: { tool: 'list_sessions' },
            });
            const ownBody = await own.json() as { result?: { content?: Array<{ text: string }> } };
            parsed = JSON.parse(ownBody.result?.content?.[0]?.text || '{}') as typeof parsed;
            if (parsed.calls?.some((call) => call.tool === 'list_sessions')) break;
            await new Promise((resolve) => setTimeout(resolve, 20));
        }
        expect(parsed.calls?.some((call) => call.tool === 'list_sessions')).toBe(true);
    });
});

describe('MCP tools listing page', () => {
    it('covers every live tool with an example and a group', async () => {
        const { listToolDescriptors } = await import('../src/mcp/tools');
        const { toolExamples, toolGroups } = await import('../src/mcp/examples');
        const names = listToolDescriptors().map((tool) => tool.name);
        const grouped = toolGroups.flatMap((group) => group.tools);
        expect(new Set(grouped).size).toBe(grouped.length);
        expect(grouped.sort()).toEqual([...names].sort());
        for (const name of names) {
            expect(toolExamples[name], name).toBeTruthy();
            expect(toolExamples[name].output).toBeDefined();
        }
    });

    it('serves the public listing and the landing MCP section', async () => {
        const tools = await SELF.fetch(`${ORIGIN}/mcp-tools`);
        expect(tools.status).toBe(200);
        const toolsHtml = await tools.text();
        expect(toolsHtml).toContain('MCP tools');
        expect(toolsHtml).toContain('id="list_sessions"');
        expect(toolsHtml).toContain('id="choose_option"');
        expect(toolsHtml).toContain('id="get_pr_board"');
        expect(toolsHtml).toContain('Example output');
        expect(toolsHtml).toContain(`${ORIGIN}/mcp`);
        expect(toolsHtml).toContain(`claude mcp add --transport http crabigator ${ORIGIN}/mcp`);
        expect(toolsHtml).toContain(`codex mcp add crabigator --url ${ORIGIN}/mcp`);
        expect(toolsHtml).toContain(`opencode mcp add crabigator --url ${ORIGIN}/mcp`);
        expect(toolsHtml).toContain(`grok mcp add --transport http crabigator ${ORIGIN}/mcp`);
        expect(toolsHtml).not.toContain('https://drinkcrabigator.com');

        const landing = await SELF.fetch(`${ORIGIN}/`);
        const landingHtml = await landing.text();
        expect(landingHtml).toContain('id="mcp"');
        expect(landingHtml).toContain('/mcp-tools');
        expect(landingHtml).toContain('list_sessions');
        expect(landingHtml).toContain('Let another agent drive your sessions');
        expect(landingHtml).not.toContain('Look around');
        expect(landingHtml).not.toContain('Take action');
        expect(landingHtml).not.toContain('mcp-groups');
        expect(landingHtml).toContain(`claude mcp add --transport http crabigator ${ORIGIN}/mcp`);
        expect(landingHtml).toContain(`codex mcp add crabigator --url ${ORIGIN}/mcp`);
        expect(landingHtml).toContain(`opencode mcp add crabigator --url ${ORIGIN}/mcp`);
        expect(landingHtml).toContain(`grok mcp add --transport http crabigator ${ORIGIN}/mcp`);
    });
});

