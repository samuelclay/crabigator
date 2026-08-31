import type { Env } from './types/env';

export interface AppConfig {
    public_origin?: string;
    features?: {
        transcription?: boolean;
        billing?: boolean;
        gifts?: boolean;
        outbound_email?: boolean;
        marketing_analytics?: boolean;
        traffic_alerts?: boolean;
        staff?: boolean;
    };
    billing?: {
        visible_session_limit?: number;
        price_display?: string;
        price_period?: string;
        stripe_mode?: 'test' | 'live';
        paypal_mode?: 'sandbox' | 'live';
    };
    email?: {
        mailgun_domain?: string;
        from?: string;
        traffic_alert_recipient?: string;
    };
    marketing?: { meta_pixel_id?: string };
    operations?: { billing_cycle_day?: number };
}

export interface Capabilities {
    core: true;
    transcription: boolean;
    billing: boolean;
    gifts: boolean;
    outbound_email: boolean;
    marketing_analytics: boolean;
    traffic_alerts: boolean;
    staff: boolean;
}

export interface RuntimeConfig {
    origin: string;
    visible_session_limit: number;
    billing_price: string;
    billing_period: string;
    capabilities: Capabilities;
    missing_config: string[];
}

function hasStripe(env: Env, config: AppConfig): boolean {
    const test = config.billing?.stripe_mode === 'test';
    return test
        ? Boolean(env.STRIPE_SECRET_KEY_TEST && env.STRIPE_WEBHOOK_SECRET_TEST && env.STRIPE_PRICE_ID_TEST)
        : Boolean(env.STRIPE_SECRET_KEY && env.STRIPE_WEBHOOK_SECRET && env.STRIPE_PRICE_ID);
}

function hasPayPal(env: Env): boolean {
    return Boolean(
        env.PAYPAL_CLIENT_ID
        && env.PAYPAL_CLIENT_SECRET
        && env.PAYPAL_WEBHOOK_ID
        && env.PAYPAL_PLAN_ID,
    );
}

export function getAppConfig(env: Env): AppConfig {
    const value: unknown = env.APP_CONFIG;
    if (typeof value === 'string') {
        try {
            return JSON.parse(value) as AppConfig;
        } catch {
            return {};
        }
    }
    return value && typeof value === 'object' ? value as AppConfig : {};
}

export function getPublicOrigin(request: Request, config: AppConfig): string {
    if (config.public_origin) {
        try {
            const configured = new URL(config.public_origin);
            if (configured.protocol === 'https:' || configured.hostname === 'localhost') {
                return configured.origin;
            }
        } catch {
            // Fall through to the request origin.
        }
    }
    return new URL(request.url).origin;
}

export function getRuntimeConfig(request: Request, env: Env): RuntimeConfig {
    const config = getAppConfig(env);
    const requested = config.features || {};
    const emailReady = Boolean(
        env.MAILGUN_API_KEY && config.email?.mailgun_domain && config.email?.from,
    );
    const billingReady = hasStripe(env, config) || hasPayPal(env);
    const capabilities: Capabilities = {
        core: true,
        transcription: Boolean(requested.transcription && env.OPENAI_API_KEY),
        billing: Boolean(requested.billing && billingReady),
        gifts: Boolean(requested.gifts && requested.billing && billingReady),
        outbound_email: Boolean(requested.outbound_email && emailReady),
        marketing_analytics: Boolean(requested.marketing_analytics),
        traffic_alerts: Boolean(
            requested.traffic_alerts
            && emailReady
            && config.public_origin
            && config.email?.traffic_alert_recipient,
        ),
        staff: Boolean(requested.staff && env.STAFF_ACCESS_KEY),
    };
    const missing = new Set<string>();
    if (requested.transcription && !env.OPENAI_API_KEY) missing.add('OPENAI_API_KEY');
    if (requested.billing && !billingReady) missing.add('billing_provider');
    if (requested.gifts && !requested.billing) missing.add('features.billing');
    if (requested.outbound_email && !emailReady) missing.add('mailgun');
    if (requested.traffic_alerts && !emailReady) missing.add('mailgun');
    if (requested.traffic_alerts && !config.public_origin) missing.add('public_origin');
    if (requested.traffic_alerts && !config.email?.traffic_alert_recipient) {
        missing.add('traffic_alert_recipient');
    }
    if (requested.staff && !env.STAFF_ACCESS_KEY) missing.add('STAFF_ACCESS_KEY');

    return {
        origin: getPublicOrigin(request, config),
        visible_session_limit: Math.max(1, config.billing?.visible_session_limit || 3),
        billing_price: config.billing?.price_display || '$3',
        billing_period: config.billing?.price_period || 'per month',
        capabilities,
        missing_config: [...missing],
    };
}

export function featureUnavailable(feature: keyof Omit<Capabilities, 'core'>): Response {
    return new Response(
        JSON.stringify({ error: `${feature} is not configured`, code: 'FEATURE_DISABLED' }),
        { status: 404, headers: { 'Content-Type': 'application/json' } },
    );
}
