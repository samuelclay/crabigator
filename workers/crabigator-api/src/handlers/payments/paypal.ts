import type { Env } from '../../types/env';
import { jsonResponse } from '../../router';
import { getAppConfig, getPublicOrigin } from '../../config';

/**
 * Get PayPal API base URL based on mode
 */
function getPayPalApiBase(env: Env): string {
    return getAppConfig(env).billing?.paypal_mode === 'live'
        ? 'https://api-m.paypal.com'
        : 'https://api-m.sandbox.paypal.com';
}

/**
 * Get PayPal access token
 */
async function getPayPalAccessToken(env: Env): Promise<string | null> {
    if (!env.PAYPAL_CLIENT_ID || !env.PAYPAL_CLIENT_SECRET) {
        return null;
    }

    const auth = btoa(`${env.PAYPAL_CLIENT_ID}:${env.PAYPAL_CLIENT_SECRET}`);
    const response = await fetch(`${getPayPalApiBase(env)}/v1/oauth2/token`, {
        method: 'POST',
        headers: {
            'Authorization': `Basic ${auth}`,
            'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: 'grant_type=client_credentials',
    });

    if (!response.ok) {
        console.error('Failed to get PayPal access token:', await response.text());
        return null;
    }

    const data = await response.json() as { access_token: string };
    return data.access_token;
}

/**
 * Create a PayPal subscription
 * Returns a URL to redirect the user to PayPal's subscription approval page
 */
export async function createPayPalSubscription(
    request: Request,
    env: Env,
    groupId: string
): Promise<Response> {
    if (!env.PAYPAL_CLIENT_ID || !env.PAYPAL_CLIENT_SECRET || !env.PAYPAL_PLAN_ID) {
        return jsonResponse(
            { error: 'PayPal not configured', code: 'PAYPAL_NOT_CONFIGURED' },
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
    const successUrl = `${returnUrl}?payment=success&provider=paypal`;
    const cancelUrl = `${returnUrl}?payment=canceled`;

    // Get access token
    const accessToken = await getPayPalAccessToken(env);
    if (!accessToken) {
        return jsonResponse(
            { error: 'Failed to authenticate with PayPal', code: 'PAYPAL_AUTH_ERROR' },
            500
        );
    }

    // Create subscription
    const subscriptionPayload = {
        plan_id: env.PAYPAL_PLAN_ID,
        application_context: {
            brand_name: 'Crabigator',
            locale: 'en-US',
            shipping_preference: 'NO_SHIPPING',
            user_action: 'SUBSCRIBE_NOW',
            return_url: successUrl,
            cancel_url: cancelUrl,
        },
        custom_id: groupId,  // Store group_id for webhook lookup
    };

    try {
        const response = await fetch(`${getPayPalApiBase(env)}/v1/billing/subscriptions`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${accessToken}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(subscriptionPayload),
        });

        if (!response.ok) {
            const error = await response.text();
            console.error('PayPal subscription error:', error);
            return jsonResponse(
                { error: 'Failed to create subscription', code: 'PAYPAL_ERROR' },
                500
            );
        }

        const subscription = await response.json() as {
            id: string;
            status: string;
            links: Array<{ rel: string; href: string }>;
        };

        // Find approval URL
        const approvalLink = subscription.links.find(link => link.rel === 'approve');
        if (!approvalLink) {
            return jsonResponse(
                { error: 'No approval URL returned', code: 'PAYPAL_NO_APPROVAL_URL' },
                500
            );
        }

        return jsonResponse({
            subscription_url: approvalLink.href,
            subscription_id: subscription.id,
        });
    } catch (error) {
        console.error('PayPal API error:', error);
        return jsonResponse(
            { error: 'Failed to connect to PayPal', code: 'PAYPAL_CONNECTION_ERROR' },
            500
        );
    }
}
