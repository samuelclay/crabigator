import type { Env } from '../../types/env';
import { jsonResponse } from '../../router';
import { getStripeConfig } from './stripe-config';
import { getAppConfig, getPublicOrigin } from '../../config';

/**
 * Create a Stripe checkout session for subscription
 * Returns a URL to redirect the user to Stripe's hosted checkout page
 */
export async function createStripeCheckout(
    request: Request,
    env: Env,
    groupId: string
): Promise<Response> {
    const config = getStripeConfig(env);
    if (!config) {
        return jsonResponse(
            { error: 'Stripe not configured', code: 'STRIPE_NOT_CONFIGURED' },
            500
        );
    }

    // Get the return URL from request body
    let body: { return_url?: string };
    try {
        body = await request.json();
    } catch {
        body = {};
    }

    const returnUrl = body.return_url || `${getPublicOrigin(request, getAppConfig(env))}/dashboard`;
    const successUrl = `${returnUrl}?payment=success&provider=stripe`;
    const cancelUrl = `${returnUrl}?payment=canceled`;

    // Create Stripe checkout session
    const params = new URLSearchParams();
    params.append('mode', 'subscription');
    params.append('payment_method_types[0]', 'card');
    params.append('line_items[0][price]', config.priceId);
    params.append('line_items[0][quantity]', '1');
    params.append('success_url', successUrl);
    params.append('cancel_url', cancelUrl);
    params.append('metadata[group_id]', groupId);
    params.append('subscription_data[metadata][group_id]', groupId);
    // Allow promotion codes for potential discounts
    params.append('allow_promotion_codes', 'true');

    try {
        const response = await fetch('https://api.stripe.com/v1/checkout/sessions', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${config.secretKey}`,
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: params.toString(),
        });

        if (!response.ok) {
            const error = await response.json() as { error?: { message?: string } };
            console.error('Stripe checkout error:', error);
            return jsonResponse(
                { error: 'Failed to create checkout session', code: 'STRIPE_ERROR' },
                500
            );
        }

        const session = await response.json() as { id: string; url: string };
        return jsonResponse({
            checkout_url: session.url,
            session_id: session.id,
        });
    } catch (error) {
        console.error('Stripe API error:', error);
        return jsonResponse(
            { error: 'Failed to connect to Stripe', code: 'STRIPE_CONNECTION_ERROR' },
            500
        );
    }
}
