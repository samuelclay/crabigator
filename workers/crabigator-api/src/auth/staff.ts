import type { Env } from '../types/env';

const COOKIE_NAME = 'crabigator_staff';
const SESSION_SECONDS = 12 * 60 * 60;

async function digestBytes(value: string): Promise<ArrayBuffer> {
    const bytes = new TextEncoder().encode(value);
    return crypto.subtle.digest('SHA-256', bytes);
}

async function digest(value: string): Promise<string> {
    const result = await digestBytes(value);
    return Array.from(new Uint8Array(result), byte => byte.toString(16).padStart(2, '0')).join('');
}

function readCookie(request: Request): string | null {
    const cookie = request.headers.get('Cookie') || '';
    for (const part of cookie.split(';')) {
        const [name, ...value] = part.trim().split('=');
        if (name === COOKIE_NAME) return value.join('=');
    }
    return null;
}

async function accessKeyMatches(provided: string, expected: string): Promise<boolean> {
    const [left, right] = await Promise.all([digestBytes(provided), digestBytes(expected)]);
    return crypto.subtle.timingSafeEqual(left, right);
}

function loginHtml(): string {
    return `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    <title>Crabigator staff login</title><style>
    body{margin:0;min-height:100vh;display:grid;place-items:center;background:#0d1117;color:#e6edf3;font:16px system-ui}
    form{width:min(360px,calc(100vw - 48px));padding:28px;border:1px solid #30363d;border-radius:12px;background:#161b22}
    h1{font-size:20px;margin:0 0 18px}label{display:block;margin-bottom:8px;color:#8b949e}
    input,button{box-sizing:border-box;width:100%;padding:11px;border-radius:7px;font:inherit}
    input{border:1px solid #30363d;background:#0d1117;color:#e6edf3}button{margin-top:14px;border:0;background:#238636;color:white;font-weight:600}
    </style></head><body><form id="login"><h1>Crabigator staff</h1>
    <label for="key">Access key</label><input id="key" type="password" autocomplete="current-password" required>
    <button type="submit">Sign in</button></form><script>
    document.getElementById('login').addEventListener('submit',async function(event){event.preventDefault();
    const response=await fetch('/api/staff/login',{method:'POST',headers:{'Content-Type':'application/json'},
    body:JSON.stringify({access_key:document.getElementById('key').value})});
    if(response.ok){location.href='/staff'}else{location.reload()}});</script></body></html>`;
}

export function staffLoginPage(): Response {
    return new Response(loginHtml(), {
        headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' },
    });
}

export async function createStaffSession(request: Request, env: Env): Promise<Response> {
    if (!env.STAFF_ACCESS_KEY) return new Response('Not found', { status: 404 });
    if (request.headers.get('Origin') !== new URL(request.url).origin) {
        return new Response('Forbidden', { status: 403 });
    }
    let accessKey = '';
    try {
        const body = await request.json<{ access_key?: string }>();
        accessKey = body.access_key || '';
    } catch {
        return new Response(JSON.stringify({ error: 'Invalid JSON' }), { status: 400 });
    }
    if (!await accessKeyMatches(accessKey, env.STAFF_ACCESS_KEY)) {
        return new Response(JSON.stringify({ error: 'Invalid access key' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' },
        });
    }

    const tokenBytes = new Uint8Array(32);
    crypto.getRandomValues(tokenBytes);
    const token = btoa(String.fromCharCode(...tokenBytes))
        .replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
    await env.TOKENS.put(`staff-session:${await digest(token)}`, await digest(env.STAFF_ACCESS_KEY), {
        expirationTtl: SESSION_SECONDS,
    });
    return new Response(JSON.stringify({ ok: true }), {
        headers: {
            'Content-Type': 'application/json',
            'Set-Cookie': `${COOKIE_NAME}=${token}; Path=/; Max-Age=${SESSION_SECONDS}; Secure; HttpOnly; SameSite=Strict`,
            'Cache-Control': 'no-store',
        },
    });
}

export async function clearStaffSession(request: Request, env: Env): Promise<Response> {
    const token = readCookie(request);
    if (token) await env.TOKENS.delete(`staff-session:${await digest(token)}`);
    return new Response(JSON.stringify({ ok: true }), {
        headers: {
            'Content-Type': 'application/json',
            'Set-Cookie': `${COOKIE_NAME}=; Path=/; Max-Age=0; Secure; HttpOnly; SameSite=Strict`,
            'Cache-Control': 'no-store',
        },
    });
}

export async function requireStaffSession(request: Request, env: Env): Promise<Response | null> {
    if (!env.STAFF_ACCESS_KEY) return new Response('Not found', { status: 404 });
    const token = readCookie(request);
    if (!token) return new Response('Unauthorized', { status: 401 });
    const fingerprint = await env.TOKENS.get(`staff-session:${await digest(token)}`);
    const expectedFingerprint = await digest(env.STAFF_ACCESS_KEY);
    if (!fingerprint || !await accessKeyMatches(fingerprint, expectedFingerprint)) {
        return new Response('Unauthorized', { status: 401 });
    }
    if (request.method !== 'GET' && request.method !== 'HEAD') {
        const origin = request.headers.get('Origin');
        if (!origin || origin !== new URL(request.url).origin) {
            return new Response('Forbidden', { status: 403 });
        }
    }
    return null;
}
