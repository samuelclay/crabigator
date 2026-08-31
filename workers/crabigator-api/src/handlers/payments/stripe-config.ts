import type { Env } from '../../types/env';
import { getAppConfig } from '../../config';

export interface StripeConfig {
    secretKey: string;
    webhookSecret: string;
    priceId: string;
    isTestMode: boolean;
}

/**
 * Get Stripe configuration based on APP_CONFIG.
 */
export function getStripeConfig(env: Env): StripeConfig | null {
    const isTestMode = getAppConfig(env).billing?.stripe_mode === 'test';

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
