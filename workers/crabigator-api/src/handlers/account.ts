import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { extractToken, verifyMobileToken, viewerCookie } from '../auth/middleware';
import {
    AccountError,
    accountErrorResponse,
    attachDesktopToAccount,
    loadAccountView,
    mintViewerToken,
    revokeViewerToken,
    touchAccountLogin,
    upsertAccountFromProfile,
    type SocialProvider,
} from '../auth/accounts';
import {
    anySocialConfigured,
    exchangeSocialCode,
    socialProviders,
    startSocialOAuth,
    takeOAuthState,
    type SocialIntent,
} from '../auth/social';

const PROVIDERS: SocialProvider[] = ['github', 'google'];

function isProvider(value: string): value is SocialProvider {
    return PROVIDERS.includes(value as SocialProvider);
}

function browserName(request: Request): string {
    const ua = request.headers.get('User-Agent') || '';
    if (/Mobile|Android|iPhone/i.test(ua)) return 'Mobile Browser';
    return 'Web Browser';
}

export async function startAccountOAuth(
    request: Request,
    env: Env,
    params: Record<string, string>,
): Promise<Response> {
    const provider = params.provider;
    if (!isProvider(provider)) {
        return jsonResponse({ error: 'Unknown provider', code: 'UNKNOWN_PROVIDER' }, 404);
    }
    const url = new URL(request.url);
    const intent = (url.searchParams.get('intent') === 'connect' ? 'connect' : 'login') as SocialIntent;
    const returnTo = safeReturnTo(url.searchParams.get('return_to'));

    let accountId: string | undefined;
    if (intent === 'connect') {
        const auth = await verifyMobileToken(request, env);
        if (!auth?.account_id) {
            return new Response(
                JSON.stringify({ error: 'Sign in first', code: 'MOBILE_AUTH_REQUIRED' }),
                { status: 401, headers: { 'Content-Type': 'application/json' } },
            );
        }
        accountId = auth.account_id;
    }

    return startSocialOAuth(request, env, provider, {
        intent,
        account_id: accountId,
        return_to: returnTo,
    });
}

function safeReturnTo(value: string | null): string {
    if (!value) return '/dashboard';
    if (!value.startsWith('/') || value.startsWith('//')) return '/dashboard';
    return value;
}

export async function handleSocialCallback(
    request: Request,
    env: Env,
    params: Record<string, string>,
): Promise<Response> {
    const provider = params.provider;
    if (!isProvider(provider)) {
        return oauthResultPage('Unknown login provider.', false, '/dashboard');
    }
    const url = new URL(request.url);
    const error = url.searchParams.get('error');
    if (error) {
        return oauthResultPage('Sign-in was cancelled.', false, '/dashboard');
    }
    const code = url.searchParams.get('code') || '';
    const stateId = url.searchParams.get('state') || '';
    const state = await takeOAuthState(env, stateId);
    if (!state || state.provider !== provider) {
        return oauthResultPage('This sign-in link expired. Try again.', false, '/dashboard');
    }

    try {
        const profile = await exchangeSocialCode(request, env, provider, code);
        const account = await upsertAccountFromProfile(
            env,
            profile,
            state.intent === 'connect' ? state.account_id : undefined,
        );
        await touchAccountLogin(env, account.id);

        if (state.intent === 'connect') {
            return oauthResultPage('Login connected.', true, state.return_to, null);
        }

        const minted = await mintViewerToken(env, account, browserName(request));
        return oauthResultPage('Signed in.', true, state.return_to, minted.token, request);
    } catch (err) {
        if (err instanceof AccountError) {
            return oauthResultPage(err.message, false, state.return_to);
        }
        console.error('Social callback failed', err);
        return oauthResultPage('Sign-in failed. Try again.', false, state.return_to);
    }
}

export async function getAccountStatus(request: Request, env: Env): Promise<Response> {
    const auth = await verifyMobileToken(request, env);
    if (!auth) {
        return new Response(
            JSON.stringify({ error: 'Mobile authentication required', code: 'MOBILE_AUTH_REQUIRED' }),
            { status: 401, headers: { 'Content-Type': 'application/json' } },
        );
    }
    if (!auth.account_id) {
        return jsonResponse({
            account: null,
            identities: [],
            group_id: auth.group_id || null,
            needs_desktop: !auth.group_id,
            social: socialProviders(env),
            social_configured: anySocialConfigured(env),
        });
    }
    const view = await loadAccountView(env, auth.account_id);
    if (!view) {
        return jsonResponse({ error: 'Account not found', code: 'ACCOUNT_NOT_FOUND' }, 404);
    }
    return jsonResponse({
        account: { id: view.account.id, group_id: view.account.group_id },
        identities: view.identities.map((identity) => ({
            provider: identity.provider,
            email: identity.email,
            name: identity.name,
            username: identity.username,
        })),
        group_id: view.account.group_id,
        needs_desktop: !view.account.group_id,
        social: socialProviders(env),
        social_configured: anySocialConfigured(env),
    });
}

export async function attachAccountDesktop(request: Request, env: Env): Promise<Response> {
    const auth = await verifyMobileToken(request, env);
    if (!auth?.account_id) {
        return new Response(
            JSON.stringify({ error: 'Sign in first', code: 'MOBILE_AUTH_REQUIRED' }),
            { status: 401, headers: { 'Content-Type': 'application/json' } },
        );
    }
    let body: { pairing_token?: string };
    try {
        body = await request.json();
    } catch {
        return jsonResponse({ error: 'Invalid JSON', code: 'INVALID_JSON' }, 400);
    }
    if (!body.pairing_token) {
        return jsonResponse({ error: 'Missing pairing code', code: 'MISSING_CODE' }, 400);
    }
    try {
        const account = await attachDesktopToAccount(
            env,
            auth.account_id,
            body.pairing_token.trim().toUpperCase(),
            browserName(request),
        );
        return jsonResponse({
            ok: true,
            group_id: account.group_id,
            needs_desktop: !account.group_id,
        });
    } catch (err) {
        return accountErrorResponse(err);
    }
}

export async function logoutAccount(request: Request, env: Env): Promise<Response> {
    const token = extractToken(request);
    if (token) {
        await revokeViewerToken(env, token);
    }
    return jsonResponse({ ok: true });
}

function oauthResultPage(
    message: string,
    ok: boolean,
    returnTo: string,
    token: string | null = null,
    request?: Request,
): Response {
    const safeReturn = JSON.stringify(returnTo);
    const safeToken = token ? JSON.stringify(token) : 'null';
    const safeMessage = JSON.stringify(message);
    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; background: #0a0a0a; color: #e5e5e5;
            min-height: 100vh; display: flex; align-items: center; justify-content: center; margin: 0; }
        .card { max-width: 28rem; padding: 2rem; text-align: center; }
        a { color: #22d3ee; }
    </style>
</head>
<body>
    <div class="card">
        <p id="msg"></p>
        <p><a href="/dashboard">Continue to dashboard</a></p>
    </div>
    <script>
        const ok = ${ok ? 'true' : 'false'};
        const token = ${safeToken};
        const returnTo = ${safeReturn};
        document.getElementById('msg').textContent = ${safeMessage};
        if (token) {
            localStorage.setItem('crabigator_mobile_token', token);
        }
        if (ok) {
            location.replace(returnTo);
        }
    </script>
</body>
</html>`;
    const headers = new Headers({ 'Content-Type': 'text/html; charset=utf-8' });
    if (ok && token && request) {
        const secure = new URL(request.url).protocol === 'https:';
        headers.set('Set-Cookie', viewerCookie(token, secure));
    }
    return new Response(html, {
        status: ok ? 200 : 400,
        headers,
    });
}
