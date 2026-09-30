import { CallbackEndpointError, McpEventError, type CallbackReason } from './events';

const WEBHOOK_BODY_LIMIT = 256 * 1024;
const RESPONSE_LIMIT = 64 * 1024;
const CONNECT_TIMEOUT_MS = 10_000;

export interface WebhookRequest {
    url: string;
    body: string;
    headers: Record<string, string>;
}

export type WebhookPoster = (request: WebhookRequest) => Promise<{ status: number; body: string }>;

let poster: WebhookPoster = postChecked;

/** Tests replace the network poster. Production checks DNS, then fetches the URL. */
export function setWebhookPoster(next: WebhookPoster | null): void {
    poster = next || postChecked;
}

export function webhookBodyTooLarge(body: string): boolean {
    return new TextEncoder().encode(body).length > WEBHOOK_BODY_LIMIT;
}

export function decodeWhsec(secret: string): Uint8Array | null {
    if (!secret.startsWith('whsec_')) return null;
    const encoded = secret.slice('whsec_'.length);
    if (!/^[A-Za-z0-9+/]+={0,2}$/.test(encoded) || encoded.length % 4 !== 0) return null;
    let binary: string;
    try {
        binary = atob(encoded);
    } catch {
        return null;
    }
    const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
    if (bytes.length < 24 || bytes.length > 64) return null;
    return bytes;
}

export async function signWebhook(
    webhookId: string,
    timestampSec: number,
    body: string,
    secrets: string[],
): Promise<string> {
    const signatures = [];
    for (const secret of secrets) {
        const keyBytes = decodeWhsec(secret);
        if (!keyBytes) throw new Error('Invalid webhook secret');
        const key = await crypto.subtle.importKey(
            'raw',
            keyBytes,
            { name: 'HMAC', hash: 'SHA-256' },
            false,
            ['sign'],
        );
        const material = new TextEncoder().encode(`${webhookId}.${timestampSec}.${body}`);
        const mac = await crypto.subtle.sign('HMAC', key, material);
        signatures.push(`v1,${base64Encode(new Uint8Array(mac))}`);
    }
    return signatures.join(' ');
}

export async function signedWebhookHeaders(
    webhookId: string,
    body: string,
    secrets: string[],
    subscriptionId: string,
): Promise<Record<string, string>> {
    const timestampSec = Math.floor(Date.now() / 1000);
    return {
        'Content-Type': 'application/json',
        'webhook-id': webhookId,
        'webhook-timestamp': String(timestampSec),
        'webhook-signature': await signWebhook(webhookId, timestampSec, body, secrets),
        'X-MCP-Subscription-Id': subscriptionId,
    };
}

export async function postWebhook(request: WebhookRequest): Promise<{ status: number; body: string }> {
    return poster(request);
}

/**
 * Refuse a name that currently resolves to a private address, then POST the
 * original URL. Pinning the socket to that IP and calling startTls omits the
 * server name on the production edge (workerd#6903), so the handshake fails.
 * Workers refuse a later rebind onto a private address.
 */
async function postChecked(request: WebhookRequest): Promise<{ status: number; body: string }> {
    const url = new URL(request.url);
    await assertPublicResolution(url.hostname);
    let response: Response;
    try {
        response = await fetch(request.url, {
            method: 'POST',
            headers: request.headers,
            body: request.body,
            redirect: 'error',
            signal: AbortSignal.timeout(CONNECT_TIMEOUT_MS),
        });
    } catch (error) {
        throw asCallbackError(error);
    }
    try {
        return { status: response.status, body: await readResponseText(response, RESPONSE_LIMIT) };
    } catch {
        return { status: response.status, body: '' };
    }
}

async function assertPublicResolution(hostname: string): Promise<void> {
    const host = hostname.replace(/^\[|\]$/g, '');
    if (isIpLiteral(host)) {
        if (!isPublicAddress(host)) {
            throw new CallbackEndpointError('connection_refused', 'Callback address is not public');
        }
        return;
    }
    const addresses = await resolvePublicDns(host);
    if (!addresses.length) {
        throw new CallbackEndpointError('connection_refused', 'Callback host did not resolve');
    }
    if (addresses.some((address) => !isPublicAddress(address))) {
        throw new CallbackEndpointError('connection_refused', 'Callback address is not public');
    }
}

async function resolvePublicDns(hostname: string): Promise<string[]> {
    try {
        const [v4, v6] = await Promise.all([
            lookupDns(hostname, 'A', 1),
            lookupDns(hostname, 'AAAA', 28),
        ]);
        return [...v4, ...v6];
    } catch (error) {
        throw asCallbackError(error);
    }
}

async function lookupDns(hostname: string, type: string, recordType: number): Promise<string[]> {
    const endpoint = `https://cloudflare-dns.com/dns-query?name=${encodeURIComponent(hostname)}&type=${type}`;
    const response = await fetch(endpoint, {
        headers: { Accept: 'application/dns-json' },
        redirect: 'error',
        signal: AbortSignal.timeout(5_000),
    });
    if (!response.ok) throw new CallbackEndpointError('timeout', 'Callback DNS lookup failed');
    const payload = await response.json() as {
        Status?: number;
        Answer?: Array<{ type?: number; data?: string }>;
    };
    if (payload.Status === 3) return [];
    if (payload.Status !== 0) throw new CallbackEndpointError('timeout', 'Callback DNS lookup failed');
    return (payload.Answer || [])
        .filter((record) => record.type === recordType && typeof record.data === 'string')
        .map((record) => record.data as string);
}

async function readResponseText(response: Response, limit: number): Promise<string> {
    if (!response.body) return '';
    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let total = 0;
    try {
        while (total < limit) {
            const { done, value } = await reader.read();
            if (done || !value) break;
            const take = Math.min(value.length, limit - total);
            const piece = new Uint8Array(take);
            piece.set(value.subarray(0, take));
            chunks.push(piece);
            total += take;
            if (take < value.length) break;
        }
    } finally {
        try { await reader.cancel(); } catch { /* already closed */ }
    }
    const bytes = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
    }
    return new TextDecoder().decode(bytes);
}

function asCallbackError(error: unknown): CallbackEndpointError {
    if (error instanceof CallbackEndpointError) return error;
    const message = error instanceof Error ? error.message.toLowerCase() : '';
    if (message.includes('timed out') || message.includes('timeout') || message.includes('aborted')) {
        return new CallbackEndpointError('timeout', 'Callback endpoint timed out');
    }
    if (message.includes('tls') || message.includes('certificate') || message.includes('ssl') || message.includes('handshake')) {
        return new CallbackEndpointError('tls_error', 'Callback TLS verification failed');
    }
    if (message.includes('redirect')) {
        return new CallbackEndpointError('connection_refused', 'Callback redirected');
    }
    return new CallbackEndpointError('connection_refused', 'Callback address is not reachable');
}

function base64Encode(bytes: Uint8Array): string {
    let binary = '';
    for (const byte of bytes) binary += String.fromCharCode(byte);
    return btoa(binary);
}

/** HTTPS only. Private and local targets fail verification instead of being fetched. */
export function canonicalCallbackUrl(raw: unknown): string {
    if (typeof raw !== 'string' || !raw.trim()) {
        throw new McpEventError(-32602, 'delivery.url must be an https URL');
    }
    let url: URL;
    try {
        url = new URL(raw);
    } catch {
        throw new McpEventError(-32602, 'delivery.url must be an https URL');
    }
    if (url.protocol !== 'https:' || url.username || url.password || /[\s\\]/.test(url.href)) {
        throw new McpEventError(-32602, 'delivery.url must be an https URL');
    }
    const hostname = url.hostname.replace(/\.$/, '').toLowerCase();
    if (!hostname || hostname.length > 253) {
        throw new McpEventError(-32602, 'delivery.url must be an https URL');
    }
    if (isBlockedHostname(hostname) || (isIpLiteral(hostname) && !isPublicAddress(hostname))) {
        throw new CallbackEndpointError('connection_refused', 'Callback address is not public');
    }
    url.hostname = hostname;
    url.hash = '';
    return url.href;
}

export function isPublicAddress(address: string): boolean {
    const trimmed = address.trim().toLowerCase().replace(/%.*$/, '').replace(/^\[|\]$/g, '');
    if (trimmed.includes(':')) {
        const groups = parseIpv6(trimmed);
        return Boolean(groups && isPublicIpv6(groups));
    }
    const v4 = ipv4ToInt(trimmed);
    return v4 != null && isPublicIpv4(v4);
}

export function isBlockedHostname(hostname: string): boolean {
    const host = hostname.toLowerCase().replace(/\.$/, '');
    if (host === 'localhost' || host.endsWith('.localhost')) return true;
    if (host.endsWith('.local') || host.endsWith('.internal')) return true;
    if (host === 'metadata.google.internal' || host === 'metadata.google.com') return true;
    return false;
}

export function isIpLiteral(hostname: string): boolean {
    const host = hostname.toLowerCase().replace(/^\[|\]$/g, '');
    if (host.includes(':')) return parseIpv6(host) != null;
    return ipv4ToInt(host) != null;
}

function ipv4ToInt(address: string): number | null {
    const parts = address.split('.');
    if (parts.length !== 4) return null;
    let value = 0;
    for (const part of parts) {
        if (!/^\d{1,3}$/.test(part)) return null;
        if (part.length > 1 && part.startsWith('0')) return null;
        const octet = Number(part);
        if (octet > 255) return null;
        value = (value * 256) + octet;
    }
    return value >>> 0;
}

function isPublicIpv4(value: number): boolean {
    const inPrefix = (base: number, bits: number) => (value >>> (32 - bits)) === (base >>> (32 - bits));
    if (inPrefix(0, 8)) return false;
    if (inPrefix(0x0a000000, 8)) return false;
    if (inPrefix(0x7f000000, 8)) return false;
    if (inPrefix(0xa9fe0000, 16)) return false;
    if (inPrefix(0xac100000, 12)) return false;
    if (inPrefix(0xc0a80000, 16)) return false;
    if (inPrefix(0x64400000, 10)) return false;
    if (inPrefix(0xc0000000, 24)) return false;
    if (inPrefix(0xc0000200, 24)) return false;
    if (inPrefix(0xc6336400, 24)) return false;
    if (inPrefix(0xcb007100, 24)) return false;
    if (inPrefix(0xc6120000, 15)) return false;
    if (value >= 0xe0000000) return false;
    return true;
}

function parseIpv6(address: string): number[] | null {
    let input = address.toLowerCase();
    if (input.includes('%')) return null;
    let embedded: number | null = null;
    if (input.includes('.')) {
        const lastColon = input.lastIndexOf(':');
        if (lastColon < 0) return null;
        embedded = ipv4ToInt(input.slice(lastColon + 1));
        if (embedded == null) return null;
        input = `${input.slice(0, lastColon + 1)}0:0`;
    }
    const halves = input.split('::');
    if (halves.length > 2) return null;
    const parseSide = (part: string): number[] | null => {
        if (!part) return [];
        const groups = part.split(':');
        if (groups.some((group) => !/^[0-9a-f]{1,4}$/.test(group))) return null;
        return groups.map((group) => Number.parseInt(group, 16));
    };
    const left = parseSide(halves[0]);
    const right = halves.length === 2 ? parseSide(halves[1]) : [];
    if (!left || !right) return null;
    if (halves.length === 1 && left.length !== 8) return null;
    const missing = 8 - left.length - right.length;
    if (halves.length === 2 && missing < 0) return null;
    const groups = halves.length === 2
        ? [...left, ...new Array<number>(missing).fill(0), ...right]
        : left;
    if (groups.length !== 8) return null;
    if (embedded != null) {
        groups[6] = (embedded >>> 16) & 0xffff;
        groups[7] = embedded & 0xffff;
    }
    return groups;
}

function isPublicIpv6(groups: number[]): boolean {
    if (groups.every((group) => group === 0)) return false;
    if (groups.slice(0, 7).every((group) => group === 0) && groups[7] === 1) return false;
    if ((groups[0] & 0xffc0) === 0xfe80) return false;
    if ((groups[0] & 0xfe00) === 0xfc00) return false;
    if ((groups[0] & 0xff00) === 0xff00) return false;
    if (groups[0] === 0x2001 && groups[1] === 0x0db8) return false;
    if (groups[0] === 0x2002) {
        return isPublicIpv4(((groups[1] << 16) | groups[2]) >>> 0);
    }
    if (groups[0] === 0x64 && groups[1] === 0xff9b && groups.slice(2, 6).every((group) => group === 0)) {
        return isPublicIpv4(((groups[6] << 16) | groups[7]) >>> 0);
    }
    if (groups.slice(0, 5).every((group) => group === 0) && groups[5] === 0xffff) {
        return isPublicIpv4(((groups[6] << 16) | groups[7]) >>> 0);
    }
    if (groups.slice(0, 6).every((group) => group === 0)) {
        return isPublicIpv4(((groups[6] << 16) | groups[7]) >>> 0);
    }
    return true;
}

export function statusReason(status: number): CallbackReason | null {
    if (status >= 200 && status < 300) return null;
    if (status >= 500) return 'http_5xx';
    if (status >= 400) return 'http_4xx';
    return 'connection_refused';
}
