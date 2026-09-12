import type { Env } from '../types/env';
import { generateToken } from './tokens';
import { getPublicOrigin, getAppConfig } from '../config';
import type { SocialProfile, SocialProvider } from './accounts';

const STATE_TTL = 10 * 60;
const GITHUB_AUTHORIZE = 'https://github.com/login/oauth/authorize';
const GITHUB_TOKEN = 'https://github.com/login/oauth/access_token';
const GITHUB_USER = 'https://api.github.com/user';
const GITHUB_EMAILS = 'https://api.github.com/user/emails';
const GOOGLE_AUTHORIZE = 'https://accounts.google.com/o/oauth2/v2/auth';
const GOOGLE_TOKEN = 'https://oauth2.googleapis.com/token';
const GOOGLE_USERINFO = 'https://www.googleapis.com/oauth2/v3/userinfo';

export type SocialIntent = 'login' | 'connect';

export interface OAuthState {
    provider: SocialProvider;
    intent: SocialIntent;
    account_id?: string;
    return_to: string;
}

export function githubConfigured(env: Env): boolean {
    return Boolean(env.GITHUB_CLIENT_ID && env.GITHUB_CLIENT_SECRET);
}

export function googleConfigured(env: Env): boolean {
    return Boolean(env.GOOGLE_CLIENT_ID && env.GOOGLE_CLIENT_SECRET);
}

export function socialProviders(env: Env): { github: boolean; google: boolean } {
    return { github: githubConfigured(env), google: googleConfigured(env) };
}

export function anySocialConfigured(env: Env): boolean {
    const providers = socialProviders(env);
    return providers.github || providers.google;
}

function callbackUrl(request: Request, env: Env, provider: SocialProvider): string {
    const origin = getPublicOrigin(request, getAppConfig(env));
    return `${origin}/oauth/${provider}/callback`;
}

export async function startSocialOAuth(
    request: Request,
    env: Env,
    provider: SocialProvider,
    state: Omit<OAuthState, 'provider'>,
): Promise<Response> {
    if (provider === 'github' && !githubConfigured(env)) {
        return missingProvider('github');
    }
    if (provider === 'google' && !googleConfigured(env)) {
        return missingProvider('google');
    }

    const stateId = generateToken(16);
    const payload: OAuthState = { ...state, provider };
    await env.TOKENS.put(`oauth_state:${stateId}`, JSON.stringify(payload), {
        expirationTtl: STATE_TTL,
    });

    const redirectUri = callbackUrl(request, env, provider);
    const url = provider === 'github'
        ? githubAuthorizeUrl(env, redirectUri, stateId)
        : googleAuthorizeUrl(env, redirectUri, stateId);
    return Response.redirect(url, 302);
}

function missingProvider(provider: string): Response {
    return new Response(
        JSON.stringify({ error: `${provider} login is not configured`, code: 'SOCIAL_NOT_CONFIGURED' }),
        { status: 404, headers: { 'Content-Type': 'application/json' } },
    );
}

function githubAuthorizeUrl(env: Env, redirectUri: string, state: string): string {
    const url = new URL(GITHUB_AUTHORIZE);
    url.searchParams.set('client_id', env.GITHUB_CLIENT_ID!);
    url.searchParams.set('redirect_uri', redirectUri);
    url.searchParams.set('scope', 'read:user user:email');
    url.searchParams.set('state', state);
    return url.toString();
}

function googleAuthorizeUrl(env: Env, redirectUri: string, state: string): string {
    const url = new URL(GOOGLE_AUTHORIZE);
    url.searchParams.set('client_id', env.GOOGLE_CLIENT_ID!);
    url.searchParams.set('redirect_uri', redirectUri);
    url.searchParams.set('response_type', 'code');
    url.searchParams.set('scope', 'openid email profile');
    url.searchParams.set('state', state);
    url.searchParams.set('prompt', 'select_account');
    return url.toString();
}

export async function takeOAuthState(env: Env, stateId: string): Promise<OAuthState | null> {
    if (!stateId) return null;
    const raw = await env.TOKENS.get(`oauth_state:${stateId}`);
    if (!raw) return null;
    await env.TOKENS.delete(`oauth_state:${stateId}`);
    return JSON.parse(raw) as OAuthState;
}

export async function exchangeSocialCode(
    request: Request,
    env: Env,
    provider: SocialProvider,
    code: string,
): Promise<SocialProfile> {
    if (provider === 'github') {
        return exchangeGitHub(request, env, code);
    }
    return exchangeGoogle(request, env, code);
}

async function exchangeGitHub(
    request: Request,
    env: Env,
    code: string,
): Promise<SocialProfile> {
    const tokenResp = await fetch(GITHUB_TOKEN, {
        method: 'POST',
        headers: {
            Accept: 'application/json',
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            client_id: env.GITHUB_CLIENT_ID,
            client_secret: env.GITHUB_CLIENT_SECRET,
            code,
            redirect_uri: callbackUrl(request, env, 'github'),
        }),
    });
    if (!tokenResp.ok) {
        throw new Error(`GitHub token exchange failed: ${tokenResp.status}`);
    }
    const tokenBody = await tokenResp.json() as { access_token?: string; error?: string };
    if (!tokenBody.access_token) {
        throw new Error(tokenBody.error || 'GitHub did not return an access token');
    }

    const userResp = await fetch(GITHUB_USER, {
        headers: {
            Accept: 'application/vnd.github+json',
            Authorization: `Bearer ${tokenBody.access_token}`,
            'User-Agent': 'crabigator',
        },
    });
    if (!userResp.ok) {
        throw new Error(`GitHub user lookup failed: ${userResp.status}`);
    }
    const user = await userResp.json() as {
        id: number;
        login: string;
        name: string | null;
        email: string | null;
    };

    let email = user.email;
    if (!email) {
        const emailsResp = await fetch(GITHUB_EMAILS, {
            headers: {
                Accept: 'application/vnd.github+json',
                Authorization: `Bearer ${tokenBody.access_token}`,
                'User-Agent': 'crabigator',
            },
        });
        if (emailsResp.ok) {
            const emails = await emailsResp.json() as Array<{
                email: string;
                primary: boolean;
                verified: boolean;
            }>;
            const primary = emails.find((row) => row.primary && row.verified) || emails.find((row) => row.verified);
            email = primary?.email ?? null;
        }
    }

    return {
        provider: 'github',
        provider_user_id: String(user.id),
        email,
        name: user.name,
        username: user.login,
    };
}

async function exchangeGoogle(
    request: Request,
    env: Env,
    code: string,
): Promise<SocialProfile> {
    const body = new URLSearchParams({
        client_id: env.GOOGLE_CLIENT_ID!,
        client_secret: env.GOOGLE_CLIENT_SECRET!,
        code,
        grant_type: 'authorization_code',
        redirect_uri: callbackUrl(request, env, 'google'),
    });
    const tokenResp = await fetch(GOOGLE_TOKEN, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body,
    });
    if (!tokenResp.ok) {
        throw new Error(`Google token exchange failed: ${tokenResp.status}`);
    }
    const tokenBody = await tokenResp.json() as { access_token?: string; error?: string };
    if (!tokenBody.access_token) {
        throw new Error(tokenBody.error || 'Google did not return an access token');
    }

    const userResp = await fetch(GOOGLE_USERINFO, {
        headers: { Authorization: `Bearer ${tokenBody.access_token}` },
    });
    if (!userResp.ok) {
        throw new Error(`Google user lookup failed: ${userResp.status}`);
    }
    const user = await userResp.json() as {
        sub: string;
        email?: string;
        name?: string;
    };
    return {
        provider: 'google',
        provider_user_id: user.sub,
        email: user.email ?? null,
        name: user.name ?? null,
        username: user.email ?? null,
    };
}
