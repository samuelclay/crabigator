import type { Env } from '../../types/env';

export interface StripeConfig {
    secretKey: string;
    webhookSecret: string;
    priceId: string;
    isTestMode: boolean;
}

/**
 * Get Stripe configuration based on STRIPE_MODE
 * Defaults to live mode if not specified
 */
export function getStripeConfig(env: Env): StripeConfig | null {
    const isTestMode = env.STRIPE_MODE === 'test';

    const secretKey = isTestMode ? env.STRIPE_SECRET_KEY_TEST : env.STRIPE_SECRET_KEY;
    const webhookSecret = isTestMode ? env.STRIPE_WEBHOOK_SECRET_TEST : env.STRIPE_WEBHOOK_SECRET;
    const priceId = isTestMode ? env.STRIPE_PRICE_ID_TEST : env.STRIPE_PRICE_ID;

    if (!secretKey || !webhookSecret || !priceId) {
        return null;
    }

    return {
        secretKey,
        webhookSecret,
        priceId,
        isTestMode,
    };
}
