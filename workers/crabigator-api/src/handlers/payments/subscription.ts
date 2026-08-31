import type { Env } from '../../types/env';
import { jsonResponse } from '../../router';
import { getStripeConfig } from './stripe-config';
import { getAppConfig } from '../../config';

interface SubscriptionRow {
    id: string;
    provider: string;
    provider_customer_id: string | null;
    provider_subscription_id: string;
    status: string;
    current_period_start: number | null;
    current_period_end: number | null;
    cancel_at_period_end: number;
    created_at: number;
}

/**
 * Get subscription status for a group
 */
export async function getSubscription(
    env: Env,
    groupId: string
): Promise<Response> {
    const row = await env.DB.prepare(`
        SELECT id, provider, provider_customer_id, provider_subscription_id, status, current_period_start, current_period_end, cancel_at_period_end, created_at
        FROM subscriptions
        WHERE group_id = ?
        ORDER BY created_at DESC
        LIMIT 1
    `).bind(groupId).first<SubscriptionRow>();

    if (!row) {
        return jsonResponse({
            has_subscription: false,
            is_pro: false,
        });
    }

    const isActive = row.status === 'active' || row.status === 'past_due';
    const isPeriodValid = !row.current_period_end || row.current_period_end > Math.floor(Date.now() / 1000);

    return jsonResponse({
        has_subscription: true,
        is_pro: isActive && isPeriodValid,
        subscription: {
            id: row.id,
            provider: row.provider,
            provider_customer_id: row.provider_customer_id,
            status: row.status,
            current_period_start: row.current_period_start,
            current_period_end: row.current_period_end,
            cancel_at_period_end: row.cancel_at_period_end === 1,
            created_at: row.created_at,
        },
    });
}

/**
 * Get subscription management portal URL
 */
export async function getSubscriptionPortal(
    env: Env,
    groupId: string,
    returnUrl: string
): Promise<Response> {
    // Get current subscription
    const row = await env.DB.prepare(`
        SELECT provider, provider_customer_id, provider_subscription_id, status
        FROM subscriptions
        WHERE group_id = ? AND status IN ('active', 'past_due')
        LIMIT 1
    `).bind(groupId).first<{ provider: string; provider_customer_id: string | null; provider_subscription_id: string; status: string }>();

    if (!row) {
        return jsonResponse(
            { error: 'No active subscription', code: 'NO_SUBSCRIPTION' },
            404
        );
    }

    if (row.provider === 'stripe') {
        if (!row.provider_customer_id) {
            return jsonResponse(
                { error: 'No customer ID found', code: 'NO_CUSTOMER' },
                400
            );
        }

        const portalUrl = await createStripePortalSession(env, row.provider_customer_id, returnUrl);
        if (!portalUrl) {
            return jsonResponse(
                { error: 'Failed to create portal session', code: 'PORTAL_ERROR' },
                500
            );
        }
        return jsonResponse({ portal_url: portalUrl, provider: 'stripe' });
    } else if (row.provider === 'paypal') {
        // PayPal users manage subscriptions directly on PayPal
        const paypalBase = getAppConfig(env).billing?.paypal_mode === 'live'
            ? 'https://www.paypal.com'
            : 'https://www.sandbox.paypal.com';
        const portalUrl = `${paypalBase}/myaccount/autopay/`;
        return jsonResponse({ portal_url: portalUrl, provider: 'paypal' });
    }

    return jsonResponse(
        { error: 'Unknown provider', code: 'UNKNOWN_PROVIDER' },
        400
    );
}

/**
 * Create Stripe billing portal session
 */
async function createStripePortalSession(
    env: Env,
    customerId: string,
    returnUrl: string
): Promise<string | null> {
    const config = getStripeConfig(env);
    if (!config) {
        return null;
    }

    try {
        const response = await fetch('https://api.stripe.com/v1/billing_portal/sessions', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${config.secretKey}`,
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: new URLSearchParams({
                customer: customerId,
                return_url: returnUrl,
            }),
        });

        if (!response.ok) {
            console.error('Stripe portal error:', await response.text());
            return null;
        }

        const data = await response.json() as { url: string };
        return data.url;
    } catch (error) {
        console.error('Error creating Stripe portal session:', error);
        return null;
    }
}

/**
 * Cancel subscription at period end
 */
export async function cancelSubscription(
    env: Env,
    groupId: string
): Promise<Response> {
    // Get current subscription
    const row = await env.DB.prepare(`
        SELECT provider, provider_subscription_id, status
        FROM subscriptions
        WHERE group_id = ? AND status IN ('active', 'past_due')
        LIMIT 1
    `).bind(groupId).first<{ provider: string; provider_subscription_id: string; status: string }>();

    if (!row) {
        return jsonResponse(
            { error: 'No active subscription', code: 'NO_SUBSCRIPTION' },
            404
        );
    }

    // Cancel with provider
    let providerCanceled = false;
    if (row.provider === 'stripe') {
        providerCanceled = await cancelStripeSubscription(env, row.provider_subscription_id);
    } else if (row.provider === 'paypal') {
        providerCanceled = await cancelPayPalSubscription(env, row.provider_subscription_id);
    }

    if (!providerCanceled) {
        // Still mark as canceling locally even if provider call fails
        console.error(`Failed to cancel ${row.provider} subscription ${row.provider_subscription_id}`);
    }

    // Mark as cancel_at_period_end locally
    await env.DB.prepare(`
        UPDATE subscriptions SET
            cancel_at_period_end = 1,
            updated_at = unixepoch()
        WHERE group_id = ? AND provider = ? AND provider_subscription_id = ?
    `).bind(groupId, row.provider, row.provider_subscription_id).run();

    return jsonResponse({ ok: true, canceled: true });
}

/**
 * Cancel Stripe subscription at period end
 */
async function cancelStripeSubscription(env: Env, subscriptionId: string): Promise<boolean> {
    const config = getStripeConfig(env);
    if (!config) {
        return false;
    }

    try {
        const response = await fetch(`https://api.stripe.com/v1/subscriptions/${subscriptionId}`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${config.secretKey}`,
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: 'cancel_at_period_end=true',
        });

        return response.ok;
    } catch (error) {
        console.error('Error canceling Stripe subscription:', error);
        return false;
    }
}

/**
 * Cancel PayPal subscription
 */
async function cancelPayPalSubscription(env: Env, subscriptionId: string): Promise<boolean> {
    if (!env.PAYPAL_CLIENT_ID || !env.PAYPAL_CLIENT_SECRET) {
        return false;
    }

    const apiBase = getAppConfig(env).billing?.paypal_mode === 'live'
        ? 'https://api-m.paypal.com'
        : 'https://api-m.sandbox.paypal.com';

    try {
        // Get access token
        const auth = btoa(`${env.PAYPAL_CLIENT_ID}:${env.PAYPAL_CLIENT_SECRET}`);
        const tokenResponse = await fetch(`${apiBase}/v1/oauth2/token`, {
            method: 'POST',
            headers: {
                'Authorization': `Basic ${auth}`,
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: 'grant_type=client_credentials',
        });

        if (!tokenResponse.ok) {
            return false;
        }

        const tokenData = await tokenResponse.json() as { access_token: string };

        // Cancel subscription
        const response = await fetch(`${apiBase}/v1/billing/subscriptions/${subscriptionId}/cancel`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${tokenData.access_token}`,
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ reason: 'Customer requested cancellation' }),
        });

        return response.ok || response.status === 204;
    } catch (error) {
        console.error('Error canceling PayPal subscription:', error);
        return false;
    }
}
