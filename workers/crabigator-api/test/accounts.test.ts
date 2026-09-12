import { env } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';
import {
    AccountError,
    attachDesktopToAccount,
    mintViewerToken,
    upsertAccountFromProfile,
} from '../src/auth/accounts';
import { verifyMobileToken } from '../src/auth/middleware';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

const testEnv = env as unknown as Env;

async function ensureSchema(): Promise<void> {
    await ensureAccountSchema(testEnv);
}

async function insertDevice(id: string): Promise<void> {
    await testEnv.DB.prepare(`
        INSERT INTO devices (id, secret_hash, name, created_at, last_seen_at)
        VALUES (?, ?, ?, unixepoch(), unixepoch())
    `).bind(id, 'a'.repeat(64), 'Test Mac').run();
}

async function putPairingCode(code: string, deviceId: string): Promise<void> {
    const token = `token-${code}`;
    await testEnv.TOKENS.put(`pairing_code:${code}`, token);
    await testEnv.TOKENS.put(`pairing:${token}`, JSON.stringify({
        device_id: deviceId,
        code,
        expires_at: Math.floor(Date.now() / 1000) + 3600,
        claimed: false,
    }));
}

describe('accounts', () => {
    it('creates an account from a social profile and attaches a desktop', async () => {
        await ensureSchema();
        const deviceId = '11111111-1111-4111-8111-111111111111';
        await insertDevice(deviceId);
        await putPairingCode('AAA-BBB-CCC', deviceId);

        const account = await upsertAccountFromProfile(testEnv, {
            provider: 'github',
            provider_user_id: '1001',
            email: 'dev@example.com',
            name: 'Dev',
            username: 'dev',
        });
        expect(account.group_id).toBeNull();

        const minted = await mintViewerToken(testEnv, account, 'Test');
        const request = new Request('https://example/api/account', {
            headers: { Authorization: `Bearer ${minted.token}` },
        });
        const before = await verifyMobileToken(request, testEnv);
        expect(before?.account_id).toBe(account.id);
        expect(before?.group_id).toBeUndefined();

        const attached = await attachDesktopToAccount(testEnv, account.id, 'AAA-BBB-CCC');
        expect(attached.group_id).toBeTruthy();

        const after = await verifyMobileToken(request, testEnv);
        expect(after?.group_id).toBe(attached.group_id);
    });

    it('refuses to attach a desktop that already belongs to another account', async () => {
        await ensureSchema();
        const deviceId = '22222222-2222-4222-8222-222222222222';
        await insertDevice(deviceId);
        await putPairingCode('DDD-EEE-FFF', deviceId);

        const first = await upsertAccountFromProfile(testEnv, {
            provider: 'github',
            provider_user_id: '2002',
            email: 'one@example.com',
            name: 'One',
            username: 'one',
        });
        await attachDesktopToAccount(testEnv, first.id, 'DDD-EEE-FFF');

        const second = await upsertAccountFromProfile(testEnv, {
            provider: 'google',
            provider_user_id: 'google-2',
            email: 'two@example.com',
            name: 'Two',
            username: 'two@example.com',
        });

        await putPairingCode('GGG-HHH-III', deviceId);
        await expect(attachDesktopToAccount(testEnv, second.id, 'GGG-HHH-III'))
            .rejects.toMatchObject({ code: 'GROUP_IN_USE' } satisfies Partial<AccountError>);
    });

    it('links a second provider to the same account and refuses a taken identity', async () => {
        await ensureSchema();
        const first = await upsertAccountFromProfile(testEnv, {
            provider: 'github',
            provider_user_id: '3003',
            email: 'merge@example.com',
            name: 'Merge',
            username: 'merge',
        });
        await upsertAccountFromProfile(testEnv, {
            provider: 'google',
            provider_user_id: 'google-3',
            email: 'merge@example.com',
            name: 'Merge',
            username: 'merge@example.com',
        }, first.id);

        const other = await upsertAccountFromProfile(testEnv, {
            provider: 'github',
            provider_user_id: '4004',
            email: 'other@example.com',
            name: 'Other',
            username: 'other',
        });
        const deviceId = '33333333-3333-4333-8333-333333333333';
        await insertDevice(deviceId);
        await putPairingCode('JJJ-KKK-LLL', deviceId);
        await attachDesktopToAccount(testEnv, other.id, 'JJJ-KKK-LLL');

        await expect(upsertAccountFromProfile(testEnv, {
            provider: 'github',
            provider_user_id: '4004',
            email: 'other@example.com',
            name: 'Other',
            username: 'other',
        }, first.id)).rejects.toMatchObject({ code: 'IDENTITY_IN_USE' });
    });
});
