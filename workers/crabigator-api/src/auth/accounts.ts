import type { Env } from '../types/env';
import { generateToken, generateUUID, sha256 } from './tokens';
import { getOrCreateDeviceGroup } from './groups';

export const VIEWER_TOKEN_TTL = 60 * 60 * 24 * 365;
const PAIRING_CODE_RE = /^[A-Z0-9]{3}-[A-Z0-9]{3}-[A-Z0-9]{3}$/;

export type SocialProvider = 'github' | 'google';

export interface SocialProfile {
    provider: SocialProvider;
    provider_user_id: string;
    email: string | null;
    name: string | null;
    username: string | null;
}

export interface AccountRecord {
    id: string;
    group_id: string | null;
    created_at: number;
    last_login_at: number | null;
}

export interface AccountIdentity {
    provider: SocialProvider;
    provider_user_id: string;
    email: string | null;
    name: string | null;
    username: string | null;
}

export interface AccountView {
    account: AccountRecord;
    identities: AccountIdentity[];
}

export class AccountError extends Error {
    constructor(
        public readonly code: string,
        message: string,
        public readonly status = 400,
    ) {
        super(message);
        this.name = 'AccountError';
    }
}

export interface ViewerTokenData {
    account_id?: string;
    group_id?: string;
    desktop_id: string;
    mobile_id: string;
}

export async function getAccount(env: Env, accountId: string): Promise<AccountRecord | null> {
    return env.DB.prepare(
        'SELECT id, group_id, created_at, last_login_at FROM accounts WHERE id = ?',
    ).bind(accountId).first<AccountRecord>();
}

export async function loadAccountView(env: Env, accountId: string): Promise<AccountView | null> {
    const account = await getAccount(env, accountId);
    if (!account) return null;
    const rows = await env.DB.prepare(
        `SELECT provider, provider_user_id, email, name, username
         FROM account_identities WHERE account_id = ? ORDER BY provider`,
    ).bind(accountId).all<AccountIdentity>();
    return { account, identities: rows.results ?? [] };
}

export async function touchAccountLogin(env: Env, accountId: string): Promise<void> {
    await env.DB.prepare(
        'UPDATE accounts SET last_login_at = unixepoch() WHERE id = ?',
    ).bind(accountId).run();
}

/**
 * Find or create an account for a social profile.
 * When `linkToAccountId` is set, attach this identity to that account instead
 * of creating a new one — unless the identity already belongs to a different
 * account that has a desktop group.
 */
export async function upsertAccountFromProfile(
    env: Env,
    profile: SocialProfile,
    linkToAccountId?: string,
): Promise<AccountRecord> {
    const existing = await env.DB.prepare(
        `SELECT ai.account_id, a.group_id
         FROM account_identities ai
         JOIN accounts a ON a.id = ai.account_id
         WHERE ai.provider = ? AND ai.provider_user_id = ?`,
    ).bind(profile.provider, profile.provider_user_id).first<{
        account_id: string;
        group_id: string | null;
    }>();

    if (existing) {
        if (linkToAccountId && linkToAccountId !== existing.account_id) {
            if (existing.group_id) {
                throw new AccountError(
                    'IDENTITY_IN_USE',
                    'That login already belongs to another Crabigator account with desktops attached.',
                    409,
                );
            }
            await env.DB.prepare(
                'UPDATE account_identities SET account_id = ? WHERE provider = ? AND provider_user_id = ?',
            ).bind(linkToAccountId, profile.provider, profile.provider_user_id).run();
            await env.DB.prepare(
                `DELETE FROM accounts WHERE id = ? AND NOT EXISTS (
                    SELECT 1 FROM account_identities WHERE account_id = accounts.id
                )`,
            ).bind(existing.account_id).run();
            await updateIdentityProfile(env, linkToAccountId, profile);
            const linked = await getAccount(env, linkToAccountId);
            if (!linked) {
                throw new AccountError('ACCOUNT_NOT_FOUND', 'Account not found', 404);
            }
            return linked;
        }
        await updateIdentityProfile(env, existing.account_id, profile);
        const account = await getAccount(env, existing.account_id);
        if (!account) {
            throw new AccountError('ACCOUNT_NOT_FOUND', 'Account not found', 404);
        }
        return account;
    }

    const accountId = linkToAccountId || generateUUID();
    if (!linkToAccountId) {
        await env.DB.prepare(
            'INSERT INTO accounts (id, created_at, last_login_at) VALUES (?, unixepoch(), unixepoch())',
        ).bind(accountId).run();
    } else {
        const target = await getAccount(env, linkToAccountId);
        if (!target) {
            throw new AccountError('ACCOUNT_NOT_FOUND', 'Account not found', 404);
        }
    }

    await env.DB.prepare(
        `INSERT INTO account_identities
            (id, account_id, provider, provider_user_id, email, name, username, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch())`,
    ).bind(
        generateUUID(),
        accountId,
        profile.provider,
        profile.provider_user_id,
        profile.email,
        profile.name,
        profile.username,
    ).run();

    const account = await getAccount(env, accountId);
    if (!account) {
        throw new AccountError('ACCOUNT_NOT_FOUND', 'Account not found', 404);
    }
    return account;
}

async function updateIdentityProfile(
    env: Env,
    accountId: string,
    profile: SocialProfile,
): Promise<void> {
    await env.DB.prepare(
        `UPDATE account_identities
         SET email = COALESCE(?, email),
             name = COALESCE(?, name),
             username = COALESCE(?, username)
         WHERE account_id = ? AND provider = ? AND provider_user_id = ?`,
    ).bind(
        profile.email,
        profile.name,
        profile.username,
        accountId,
        profile.provider,
        profile.provider_user_id,
    ).run();
}

export async function firstDesktopInGroup(env: Env, groupId: string): Promise<string | null> {
    const row = await env.DB.prepare(
        'SELECT id FROM devices WHERE group_id = ? ORDER BY last_seen_at DESC LIMIT 1',
    ).bind(groupId).first<{ id: string }>();
    return row?.id ?? null;
}

export async function mintViewerToken(
    env: Env,
    account: AccountRecord,
    mobileName: string,
): Promise<{ token: string; mobile_id: string }> {
    const token = generateToken(32);
    const tokenHash = await sha256(token);
    const mobileId = `oauth-${account.id}`;
    const desktopId = account.group_id
        ? await firstDesktopInGroup(env, account.group_id)
        : null;

    const data: ViewerTokenData = {
        account_id: account.id,
        group_id: account.group_id || undefined,
        desktop_id: desktopId || '',
        mobile_id: mobileId,
    };
    await env.TOKENS.put(`mobile:${tokenHash}`, JSON.stringify(data), {
        expirationTtl: VIEWER_TOKEN_TTL,
    });

    if (desktopId) {
        await env.DB.prepare(`
            INSERT INTO linked_devices (id, desktop_id, mobile_id, mobile_name, mobile_token_hash, paired_at)
            VALUES (?, ?, ?, ?, ?, unixepoch())
            ON CONFLICT(desktop_id, mobile_id) DO UPDATE SET
                mobile_token_hash = excluded.mobile_token_hash,
                paired_at = excluded.paired_at,
                revoked_at = NULL,
                mobile_name = excluded.mobile_name
        `).bind(generateUUID(), desktopId, mobileId, mobileName, tokenHash).run();
    }

    return { token, mobile_id: mobileId };
}

export async function revokeViewerToken(env: Env, token: string): Promise<void> {
    const tokenHash = await sha256(token);
    await env.TOKENS.delete(`mobile:${tokenHash}`);
}

interface PairingTokenData {
    device_id: string;
    code: string;
    expires_at: number;
    claimed: boolean;
    mobile_id?: string;
    mobile_name?: string;
}

export async function resolvePairingDevice(
    env: Env,
    pairingTokenOrCode: string,
): Promise<{ token: string; data: PairingTokenData }> {
    let token: string | null = pairingTokenOrCode;
    if (PAIRING_CODE_RE.test(pairingTokenOrCode)) {
        token = await env.TOKENS.get(`pairing_code:${pairingTokenOrCode}`);
    }
    if (!token) {
        throw new AccountError('INVALID_CODE', 'Invalid or expired pairing code', 400);
    }
    const raw = await env.TOKENS.get(`pairing:${token}`);
    if (!raw) {
        throw new AccountError('INVALID_TOKEN', 'Invalid or expired pairing token', 400);
    }
    return { token, data: JSON.parse(raw) as PairingTokenData };
}

/**
 * Attach the desktop behind a pairing code to this account's device group.
 */
export async function attachDesktopToAccount(
    env: Env,
    accountId: string,
    pairingTokenOrCode: string,
    mobileName = 'Account',
): Promise<AccountRecord> {
    const account = await getAccount(env, accountId);
    if (!account) {
        throw new AccountError('ACCOUNT_NOT_FOUND', 'Account not found', 404);
    }

    const { token, data } = await resolvePairingDevice(env, pairingTokenOrCode);
    const deviceGroupId = await getOrCreateDeviceGroup(env, data.device_id);

    const groupOwner = await env.DB.prepare(
        'SELECT id FROM accounts WHERE group_id = ?',
    ).bind(deviceGroupId).first<{ id: string }>();
    if (groupOwner && groupOwner.id !== accountId) {
        throw new AccountError(
            'GROUP_IN_USE',
            'That desktop already belongs to another Crabigator account.',
            409,
        );
    }

    let groupId = account.group_id;
    if (!groupId) {
        await env.DB.prepare('UPDATE accounts SET group_id = ? WHERE id = ?')
            .bind(deviceGroupId, accountId).run();
        groupId = deviceGroupId;
    } else if (groupId !== deviceGroupId) {
        await env.DB.prepare(
            'UPDATE devices SET group_id = ? WHERE group_id = ?',
        ).bind(groupId, deviceGroupId).run();
    }

    data.claimed = true;
    data.mobile_id = `oauth-${accountId}`;
    data.mobile_name = mobileName;
    await env.TOKENS.put(`pairing:${token}`, JSON.stringify(data));

    const mobileId = `oauth-${accountId}`;
    const tokenPlaceholder = await sha256(`account-link:${accountId}:${data.device_id}`);
    await env.DB.prepare(`
        INSERT INTO linked_devices (id, desktop_id, mobile_id, mobile_name, mobile_token_hash, paired_at)
        VALUES (?, ?, ?, ?, ?, unixepoch())
        ON CONFLICT(desktop_id, mobile_id) DO UPDATE SET
            paired_at = excluded.paired_at,
            revoked_at = NULL,
            mobile_name = excluded.mobile_name
    `).bind(generateUUID(), data.device_id, mobileId, mobileName, tokenPlaceholder).run();

    const updated = await getAccount(env, accountId);
    if (!updated) {
        throw new AccountError('ACCOUNT_NOT_FOUND', 'Account not found', 404);
    }
    return updated;
}

export function accountErrorResponse(error: unknown): Response {
    if (error instanceof AccountError) {
        return new Response(
            JSON.stringify({ error: error.message, code: error.code }),
            {
                status: error.status,
                headers: {
                    'Content-Type': 'application/json',
                    'X-Error-Code': error.code,
                },
            },
        );
    }
    console.error('Account error', error);
    return new Response(
        JSON.stringify({ error: 'Internal server error', code: 'INTERNAL_ERROR' }),
        { status: 500, headers: { 'Content-Type': 'application/json' } },
    );
}
