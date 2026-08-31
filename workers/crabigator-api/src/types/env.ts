import type { AppConfig } from '../config';

/**
 * Wrangler generates Cloudflare.Env from wrangler.example.jsonc. This extension
 * lists secrets, which never belong in the checked-in Wrangler configuration.
 */
export type Env = Omit<Cloudflare.Env, 'APP_CONFIG'> & {
    APP_CONFIG: AppConfig;

    STRIPE_SECRET_KEY?: string;
    STRIPE_WEBHOOK_SECRET?: string;
    STRIPE_PRICE_ID?: string;

    STRIPE_SECRET_KEY_TEST?: string;
    STRIPE_WEBHOOK_SECRET_TEST?: string;
    STRIPE_PRICE_ID_TEST?: string;

    PAYPAL_CLIENT_ID?: string;
    PAYPAL_CLIENT_SECRET?: string;
    PAYPAL_WEBHOOK_ID?: string;
    PAYPAL_PLAN_ID?: string;
    OPENAI_API_KEY?: string;
    MAILGUN_API_KEY?: string;
    STAFF_ACCESS_KEY?: string;
};
