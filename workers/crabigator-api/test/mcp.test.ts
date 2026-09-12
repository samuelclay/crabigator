import { env, SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';
import { mintViewerToken, upsertAccountFromProfile, attachDesktopToAccount, getAccount } from '../src/auth/accounts';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

const testEnv = env as unknown as Env;

async function linkedToken(): Promise<string> {
    await ensureAccountSchema(testEnv);
    const deviceId = '44444444-4444-4444-8444-444444444444';
    await testEnv.DB.prepare(`
        INSERT OR IGNORE INTO devices (id, secret_hash, name, created_at, last_seen_at)
        VALUES (?, ?, ?, unixepoch(), unixepoch())
    `).bind(deviceId, 'b'.repeat(64), 'MCP Mac').run();
    await testEnv.TOKENS.put('pairing_code:MCP-TOK-EN1', 'mcp-pair-token');
    await testEnv.TOKENS.put('pairing:mcp-pair-token', JSON.stringify({
        device_id: deviceId,
        code: 'MCP-TOK-EN1',
        expires_at: Math.floor(Date.now() / 1000) + 3600,
        claimed: false,
    }));
    const account = await upsertAccountFromProfile(testEnv, {
        provider: 'github',
        provider_user_id: 'mcp-user',
        email: 'mcp@example.com',
        name: 'MCP',
        username: 'mcp',
    });
    await attachDesktopToAccount(testEnv, account.id, 'MCP-TOK-EN1');
    const linked = await getAccount(testEnv, account.id);
    if (!linked) throw new Error('account missing');
    const minted = await mintViewerToken(testEnv, linked, 'MCP test');
    return minted.token;
}

describe('MCP server', () => {
    it('advertises OAuth metadata and rejects unauthenticated tools', async () => {
        const metadata = await SELF.fetch('https://self-host.example/.well-known/oauth-authorization-server');
        expect(metadata.status).toBe(200);
        const body = await metadata.json() as { authorization_endpoint: string; token_endpoint: string };
        expect(body.authorization_endpoint).toContain('/authorize');
        expect(body.token_endpoint).toContain('/token');

        const unauthorized = await SELF.fetch('https://self-host.example/mcp', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/list' }),
        });
        expect(unauthorized.status).toBe(401);
        expect(unauthorized.headers.get('WWW-Authenticate') || '').toContain('oauth-protected-resource');
    });

    it('lists tools for a signed-in account with a desktop', async () => {
        const token = await linkedToken();
        const response = await SELF.fetch('https://self-host.example/mcp', {
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
});
