import type { PushFetch } from './notification-history';
import type { Env } from './types/env';

interface AppleCredentials { team_id: string; key_id: string; private_key: string; topic: string }
export interface AppleDevice { mobile_id: string; token: string; environment: string; sound: number }
let cached: { identity: string; token: string; expires: number } | undefined;
const encode = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes)).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
const json64 = (value: unknown) => encode(new TextEncoder().encode(JSON.stringify(value)));

async function providerToken(account: AppleCredentials): Promise<string> {
    const identity = `${account.team_id}:${account.key_id}`;
    if (cached?.identity === identity && cached.expires > Date.now()) return cached.token;
    const now = Math.floor(Date.now() / 1000);
    const unsigned = `${json64({ alg: 'ES256', kid: account.key_id })}.${json64({ iss: account.team_id, iat: now })}`;
    const der = Uint8Array.from(atob(account.private_key.replace(/-----[^-]+-----|\s/g, '')), c => c.charCodeAt(0));
    const key = await crypto.subtle.importKey('pkcs8', der, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign']);
    const signature = new Uint8Array(await crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, key, new TextEncoder().encode(unsigned)));
    const token = `${unsigned}.${encode(signature)}`;
    cached = { identity, token, expires: Date.now() + 45 * 60 * 1000 };
    return token;
}

/** Alert content is generic. The service extension authenticates to load the current prompt.
 * Resolution sends a background refresh, never a new visible alert. */
export async function sendApplePush(env: Env, device: AppleDevice, sessionId: string, pending: boolean, send: PushFetch = fetch): Promise<void> {
    if (!env.APNS_CREDENTIALS) return;
    const account = JSON.parse(env.APNS_CREDENTIALS) as AppleCredentials;
    const host = device.environment === 'sandbox' ? 'api.sandbox.push.apple.com' : 'api.push.apple.com';
    const aps = pending
        ? { alert: { title: 'Crabigator', body: 'A session needs your attention.' }, 'mutable-content': 1, 'content-available': 1,
            'thread-id': sessionId, ...(device.sound ? { sound: 'default' } : {}) }
        : { 'content-available': 1 };
    const response = await send(`https://${host}/3/device/${device.token}`, {
        method: 'POST', headers: {
            authorization: `bearer ${await providerToken(account)}`, 'apns-topic': account.topic,
            'apns-push-type': pending ? 'alert' : 'background', 'apns-priority': pending ? '10' : '5',
            'apns-collapse-id': sessionId, 'apns-expiration': String(Math.floor(Date.now() / 1000) + 300),
            'content-type': 'application/json',
        }, body: JSON.stringify({ aps, session_id: sessionId }), signal: AbortSignal.timeout(10000),
    });
    if (response.ok) return;
    const result = await response.json() as { reason?: string; timestamp?: number };
    if (response.status === 410 || result.reason === 'BadDeviceToken') {
        // Do not remove a newer registration after a delayed Unregistered response.
        const cutoff = result.timestamp ? Math.floor(result.timestamp / 1000) : Math.floor(Date.now() / 1000);
        await env.DB.prepare('DELETE FROM mobile_push_tokens WHERE mobile_id = ? AND token = ? AND updated_at <= ?')
            .bind(device.mobile_id, device.token, cutoff).run();
    } else throw new Error(`APNs delivery failed (${response.status}: ${result.reason || 'unknown'})`);
}
