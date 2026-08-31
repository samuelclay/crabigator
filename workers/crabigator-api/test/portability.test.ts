import { SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';
import { usePublicOrigin } from '../src/html-render';

describe('public origin rendering', () => {
    it('does not repeat a custom host that contains the official host', () => {
        const origin = 'https://preview.drinkcrabigator.com';
        expect(usePublicOrigin(
            'https://drinkcrabigator.com/dashboard drinkcrabigator.com',
            origin,
        )).toBe(`${origin}/dashboard preview.drinkcrabigator.com`);
    });
});

describe('self-hosted runtime configuration', () => {
    it('reports capabilities and uses the request origin', async () => {
        const health = await SELF.fetch('https://self-host.example/api/health');
        expect(health.status).toBe(200);
        expect(await health.json()).toMatchObject({
            service: 'crabigator-api',
            api_version: 'v1',
            status: 'ok',
            capabilities: {
                core: true,
                transcription: false,
                billing: false,
                staff: true,
            },
            missing_config: [],
        });

        const dashboard = await SELF.fetch('https://self-host.example/dashboard');
        const html = await dashboard.text();
        expect(html).toContain('"visible_session_limit":7');
        expect(html).toContain('Free accounts show the 7 most recently active sessions.');
        expect(html).toContain('<div class="paywall-price">€5</div>');
        expect(html).toContain('<div class="paywall-price-period">per week</div>');
        expect(html).toContain('.voice-btn');
        expect(html).not.toContain('https://drinkcrabigator.com');
        expect(html).not.toContain("fbq('init'");
    });

    it('returns 404 for disabled optional routes', async () => {
        const transcription = await SELF.fetch('https://self-host.example/api/transcribe', {
            method: 'POST',
        });
        expect(transcription.status).toBe(404);

        const analytics = await SELF.fetch('https://self-host.example/api/analytics/event', {
            method: 'POST',
        });
        expect(analytics.status).toBe(404);
    });
});

describe('staff access', () => {
    it('requires a login and creates a protected session', async () => {
        const anonymous = await SELF.fetch('https://self-host.example/staff', { redirect: 'manual' });
        expect(anonymous.status).toBe(302);
        expect(anonymous.headers.get('Location')).toBe('https://self-host.example/staff/login');

        const crossOrigin = await SELF.fetch('https://self-host.example/api/staff/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Origin: 'https://other.example' },
            body: JSON.stringify({ access_key: 'test-only-staff-access-key' }),
        });
        expect(crossOrigin.status).toBe(403);

        const login = await SELF.fetch('https://self-host.example/api/staff/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', Origin: 'https://self-host.example' },
            body: JSON.stringify({ access_key: 'test-only-staff-access-key' }),
        });
        expect(login.status).toBe(200);
        const cookie = login.headers.get('Set-Cookie');
        expect(cookie).toContain('Secure; HttpOnly; SameSite=Strict');

        const authenticated = await SELF.fetch('https://self-host.example/staff', {
            headers: { Cookie: cookie!.split(';', 1)[0] },
        });
        expect(authenticated.status).toBe(200);

        const missingOrigin = await SELF.fetch('https://self-host.example/api/staff/sync-usage', {
            method: 'POST',
            headers: { Cookie: cookie!.split(';', 1)[0] },
        });
        expect(missingOrigin.status).toBe(403);
    });
});
