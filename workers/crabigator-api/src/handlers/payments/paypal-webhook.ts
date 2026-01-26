import type { Env } from '../../types/env';
import { jsonResponse } from '../../router';

interface PayPalSubscription {
    id: string;
    plan_id: string;
    status: string;
    custom_id?: string;  // group_id
    subscriber?: {
        email_address?: string;
    };
    billing_info?: {
        next_billing_time?: string;
        last_payment?: {
            time?: string;
        };
    };
}

interface PayPalSale {
    id: string;
    billing_agreement_id: string;
    state: string;
}

interface PayPalWebhookEvent {
    id: string;
    event_type: string;
    resource: PayPalSubscription | PayPalSale;
}

/**
 * Get PayPal API base URL based on mode
 */
function getPayPalApiBase(env: Env): string {
    return env.PAYPAL_MODE === 'live'
        ? 'https://api-m.paypal.com'
        : 'https://api-m.sandbox.paypal.com';
}

/**
 * Verify PayPal webhook signature
 * PayPal uses a different verification approach - we call their API
 */
async function verifyPayPalWebhook(
    env: Env,
    request: Request,
    payload: string
): Promise<boolean> {
    if (!env.PAYPAL_CLIENT_ID || !env.PAYPAL_CLIENT_SECRET || !env.PAYPAL_WEBHOOK_ID) {
        return false;
    }

    // Get required headers
    const transmissionId = request.headers.get('paypal-transmission-id');
    const transmissionTime = request.headers.get('paypal-transmission-time');
    const certUrl = request.headers.get('paypal-cert-url');
    const authAlgo = request.headers.get('paypal-auth-algo');
    const transmissionSig = request.headers.get('paypal-transmission-sig');

    if (!transmissionId || !transmissionTime || !certUrl || !authAlgo || !transmissionSig) {
        console.error('Missing PayPal webhook headers');
        return false;
    }

    // Get access token
    const auth = btoa(`${env.PAYPAL_CLIENT_ID}:${env.PAYPAL_CLIENT_SECRET}`);
    const tokenResponse = await fetch(`${getPayPalApiBase(env)}/v1/oauth2/token`, {
        method: 'POST',
        headers: {
            'Authorization': `Basic ${auth}`,
            'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: 'grant_type=client_credentials',
    });

    if (!tokenResponse.ok) {
        console.error('Failed to get PayPal access token for verification');
        return false;
    }

    const tokenData = await tokenResponse.json() as { access_token: string };

    // Verify webhook
    const verifyPayload = {
        auth_algo: authAlgo,
        cert_url: certUrl,
        transmission_id: transmissionId,
        transmission_sig: transmissionSig,
        transmission_time: transmissionTime,
        webhook_id: env.PAYPAL_WEBHOOK_ID,
        webhook_event: JSON.parse(payload),
    };

    const verifyResponse = await fetch(`${getPayPalApiBase(env)}/v1/notifications/verify-webhook-signature`, {
        method: 'POST',
        headers: {
            'Authorization': `Bearer ${tokenData.access_token}`,
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(verifyPayload),
    });

    if (!verifyResponse.ok) {
        console.error('PayPal webhook verification failed:', await verifyResponse.text());
        return false;
    }

    const verifyResult = await verifyResponse.json() as { verification_status: string };
    return verifyResult.verification_status === 'SUCCESS';
}

/**
 * Handle PayPal webhook events
 */
export async function handlePayPalWebhook(
    request: Request,
    env: Env
): Promise<Response> {
    const payload = await request.text();

    // Verify signature
    const isValid = await verifyPayPalWebhook(env, request, payload);
    if (!isValid) {
        return jsonResponse({ error: 'Invalid signature' }, 401);
    }

    let event: PayPalWebhookEvent;
    try {
        event = JSON.parse(payload) as PayPalWebhookEvent;
    } catch {
        return jsonResponse({ error: 'Invalid JSON' }, 400);
    }

    // Check for duplicate event (idempotency)
    const existingEvent = await env.DB.prepare(
        'SELECT event_id FROM webhook_events WHERE provider = ? AND event_id = ?'
    ).bind('paypal', event.id).first();

    if (existingEvent) {
        return jsonResponse({ received: true, duplicate: true });
    }

    // Process the event
    try {
        switch (event.event_type) {
            case 'BILLING.SUBSCRIPTION.CREATED':
                await handleSubscriptionCreated(env, event.resource as PayPalSubscription);
                break;

            case 'BILLING.SUBSCRIPTION.ACTIVATED':
                await handleSubscriptionActivated(env, event.resource as PayPalSubscription);
                break;

            case 'BILLING.SUBSCRIPTION.UPDATED':
                await handleSubscriptionUpdated(env, event.resource as PayPalSubscription);
                break;

            case 'BILLING.SUBSCRIPTION.CANCELLED':
                await handleSubscriptionCancelled(env, event.resource as PayPalSubscription);
                break;

            case 'BILLING.SUBSCRIPTION.SUSPENDED':
                await handleSubscriptionSuspended(env, event.resource as PayPalSubscription);
                break;

            case 'PAYMENT.SALE.COMPLETED':
                await handlePaymentCompleted(env, event.resource as PayPalSale);
                break;

            default:
                console.log(`Unhandled PayPal event type: ${event.event_type}`);
        }

        // Record event as processed
        await env.DB.prepare(
            'INSERT INTO webhook_events (provider, event_id, event_type) VALUES (?, ?, ?)'
        ).bind('paypal', event.id, event.event_type).run();

        return jsonResponse({ received: true });
    } catch (error) {
        console.error('Error processing PayPal webhook:', error);
        return jsonResponse({ error: 'Processing failed' }, 500);
    }
}

/**
 * Handle BILLING.SUBSCRIPTION.CREATED - subscription created, pending approval
 */
async function handleSubscriptionCreated(
    env: Env,
    subscription: PayPalSubscription
): Promise<void> {
    const groupId = subscription.custom_id;
    if (!groupId) {
        console.error('No custom_id (group_id) in PayPal subscription');
        return;
    }

    // Create subscription record in pending state
    const subscriptionId = crypto.randomUUID();
    await env.DB.prepare(`
        INSERT INTO subscriptions (id, group_id, provider, provider_subscription_id, status, created_at, updated_at)
        VALUES (?, ?, 'paypal', ?, 'pending', unixepoch(), unixepoch())
        ON CONFLICT (provider, provider_subscription_id) DO NOTHING
    `).bind(subscriptionId, groupId, subscription.id).run();
}

/**
 * Handle BILLING.SUBSCRIPTION.ACTIVATED - create/activate subscription
 */
async function handleSubscriptionActivated(
    env: Env,
    subscription: PayPalSubscription
): Promise<void> {
    const groupId = subscription.custom_id;
    if (!groupId) {
        console.error('No custom_id (group_id) in PayPal subscription');
        return;
    }

    // Calculate period dates
    const now = Math.floor(Date.now() / 1000);
    let periodEnd: number | null = null;
    if (subscription.billing_info?.next_billing_time) {
        periodEnd = Math.floor(new Date(subscription.billing_info.next_billing_time).getTime() / 1000);
    }

    const subscriptionId = crypto.randomUUID();
    await env.DB.prepare(`
        INSERT INTO subscriptions (id, group_id, provider, provider_subscription_id, status, current_period_start, current_period_end, created_at, updated_at)
        VALUES (?, ?, 'paypal', ?, 'active', ?, ?, unixepoch(), unixepoch())
        ON CONFLICT (provider, provider_subscription_id) DO UPDATE SET
            status = 'active',
            current_period_start = excluded.current_period_start,
            current_period_end = excluded.current_period_end,
            updated_at = unixepoch()
    `).bind(subscriptionId, groupId, subscription.id, now, periodEnd).run();

    await syncUsageDO(env, groupId);
}

/**
 * Handle BILLING.SUBSCRIPTION.UPDATED - update subscription status
 */
async function handleSubscriptionUpdated(
    env: Env,
    subscription: PayPalSubscription
): Promise<void> {
    // Map PayPal status to our status
    let status = 'pending';
    if (subscription.status === 'ACTIVE') {
        status = 'active';
    } else if (subscription.status === 'SUSPENDED') {
        status = 'past_due';
    } else if (subscription.status === 'CANCELLED' || subscription.status === 'EXPIRED') {
        status = 'canceled';
    }

    let periodEnd: number | null = null;
    if (subscription.billing_info?.next_billing_time) {
        periodEnd = Math.floor(new Date(subscription.billing_info.next_billing_time).getTime() / 1000);
    }

    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = ?,
            current_period_end = COALESCE(?, current_period_end),
            updated_at = unixepoch()
        WHERE provider = 'paypal' AND provider_subscription_id = ?
    `).bind(status, periodEnd, subscription.id).run();

    // Get group_id and sync
    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('paypal', subscription.id).first<{ group_id: string }>();

    if (row) {
        await syncUsageDO(env, row.group_id);
    }
}

/**
 * Handle BILLING.SUBSCRIPTION.CANCELLED - mark as canceled
 */
async function handleSubscriptionCancelled(
    env: Env,
    subscription: PayPalSubscription
): Promise<void> {
    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = 'canceled',
            updated_at = unixepoch()
        WHERE provider = 'paypal' AND provider_subscription_id = ?
    `).bind(subscription.id).run();

    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('paypal', subscription.id).first<{ group_id: string }>();

    if (row) {
        await syncUsageDO(env, row.group_id);
    }
}

/**
 * Handle BILLING.SUBSCRIPTION.SUSPENDED - mark as past_due
 */
async function handleSubscriptionSuspended(
    env: Env,
    subscription: PayPalSubscription
): Promise<void> {
    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = 'past_due',
            updated_at = unixepoch()
        WHERE provider = 'paypal' AND provider_subscription_id = ?
    `).bind(subscription.id).run();

    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('paypal', subscription.id).first<{ group_id: string }>();

    if (row) {
        await syncUsageDO(env, row.group_id);
    }
}

/**
 * Handle PAYMENT.SALE.COMPLETED - confirm payment
 */
async function handlePaymentCompleted(
    env: Env,
    sale: PayPalSale
): Promise<void> {
    if (!sale.billing_agreement_id) {
        return;
    }

    // Update subscription to active
    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = 'active',
            updated_at = unixepoch()
        WHERE provider = 'paypal' AND provider_subscription_id = ?
    `).bind(sale.billing_agreement_id).run();

    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('paypal', sale.billing_agreement_id).first<{ group_id: string }>();

    if (row) {
        await syncUsageDO(env, row.group_id);
    }
}

/**
 * Notify UsageDO to sync subscription status
 */
async function syncUsageDO(env: Env, groupId: string): Promise<void> {
    try {
        const doId = env.USAGE.idFromName(groupId);
        const stub = env.USAGE.get(doId);
        await stub.fetch(new Request(`https://internal/sync?group_id=${groupId}`));
    } catch (error) {
        console.error('Error syncing UsageDO:', error);
    }
}
