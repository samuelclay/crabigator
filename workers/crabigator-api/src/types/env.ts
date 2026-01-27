export interface Env {
    // D1 Database
    DB: D1Database;

    // KV Namespace for tokens
    TOKENS: KVNamespace;

    // Durable Object namespace for sessions
    SESSION: DurableObjectNamespace;

    // Durable Object for session list broadcasting
    SESSION_LIST: DurableObjectNamespace;

    // Durable Object for usage tracking
    USAGE: DurableObjectNamespace;

    // Environment variables
    API_VERSION: string;

    // Stripe payment secrets (live)
    STRIPE_SECRET_KEY?: string;
    STRIPE_WEBHOOK_SECRET?: string;
    STRIPE_PRICE_ID?: string;

    // Stripe payment secrets (test)
    STRIPE_SECRET_KEY_TEST?: string;
    STRIPE_WEBHOOK_SECRET_TEST?: string;
    STRIPE_PRICE_ID_TEST?: string;

    // Stripe mode: 'test' or 'live' (defaults to 'live')
    STRIPE_MODE?: 'test' | 'live';

    // PayPal payment secrets
    PAYPAL_CLIENT_ID?: string;
    PAYPAL_CLIENT_SECRET?: string;
    PAYPAL_WEBHOOK_ID?: string;
    PAYPAL_PLAN_ID?: string;
    PAYPAL_MODE?: 'sandbox' | 'live';

    // Mailgun email settings
    MAILGUN_API_KEY?: string;
    MAILGUN_DOMAIN?: string;
    MAILGUN_FROM?: string;
}
