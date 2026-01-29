import type { Env } from '../../types/env';
import { jsonResponse } from '../../router';

interface StripeSubscription {
    id: string;
    customer: string;
    status: string;
    current_period_start: number;
    current_period_end: number;
    cancel_at_period_end: boolean;
    metadata?: { group_id?: string };
}

interface StripeInvoice {
    id: string;
    subscription: string;
    status: string;
}

interface StripeCheckoutSession {
    id: string;
    subscription: string;
    customer: string;
    metadata?: { group_id?: string };
}

interface StripeEvent {
    id: string;
    type: string;
    data: {
        object: StripeSubscription | StripeInvoice | StripeCheckoutSession;
    };
}

/**
 * Check if this event is from Crabigator (has group_id in metadata)
 * This filters out events from other products sharing the same Stripe account
 */
function isCrabigatorEvent(event: StripeEvent): boolean {
    const obj = event.data.object;

    // Check for group_id in metadata (checkout sessions and subscriptions)
    if ('metadata' in obj && obj.metadata?.group_id) {
        return true;
    }

    // For invoice events, we can't easily check without an API call
    // But invoices reference subscriptions, so we'll process them and
    // they'll just no-op if the subscription isn't in our database
    if (event.type.startsWith('invoice.')) {
        return true;
    }

    return false;
}

/**
 * Verify Stripe webhook signature
 */
async function verifyStripeSignature(
    payload: string,
    signature: string,
    secret: string
): Promise<boolean> {
    const parts = signature.split(',');
    const timestamp = parts.find(p => p.startsWith('t='))?.slice(2);
    const v1Sig = parts.find(p => p.startsWith('v1='))?.slice(3);

    if (!timestamp || !v1Sig) {
        return false;
    }

    // Check timestamp is within 5 minutes
    const timestampNum = parseInt(timestamp, 10);
    const now = Math.floor(Date.now() / 1000);
    if (Math.abs(now - timestampNum) > 300) {
        return false;
    }

    // Compute expected signature
    const signedPayload = `${timestamp}.${payload}`;
    const encoder = new TextEncoder();
    const key = await crypto.subtle.importKey(
        'raw',
        encoder.encode(secret),
        { name: 'HMAC', hash: 'SHA-256' },
        false,
        ['sign']
    );
    const signatureBytes = await crypto.subtle.sign('HMAC', key, encoder.encode(signedPayload));
    const expectedSig = Array.from(new Uint8Array(signatureBytes))
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

    return expectedSig === v1Sig;
}

/**
 * Handle Stripe webhook events
 */
export async function handleStripeWebhook(
    request: Request,
    env: Env
): Promise<Response> {
    // Try both live and test webhook secrets since we don't know
    // which mode the incoming webhook is from until after verification
    const liveWebhookSecret = env.STRIPE_WEBHOOK_SECRET;
    const testWebhookSecret = env.STRIPE_WEBHOOK_SECRET_TEST;

    if (!liveWebhookSecret && !testWebhookSecret) {
        return jsonResponse({ error: 'Stripe not configured' }, 500);
    }

    const signature = request.headers.get('stripe-signature');
    if (!signature) {
        return jsonResponse({ error: 'Missing signature' }, 400);
    }

    const payload = await request.text();

    // Try live secret first, then test secret
    let isValid = false;
    if (liveWebhookSecret) {
        isValid = await verifyStripeSignature(payload, signature, liveWebhookSecret);
    }
    if (!isValid && testWebhookSecret) {
        isValid = await verifyStripeSignature(payload, signature, testWebhookSecret);
    }
    if (!isValid) {
        return jsonResponse({ error: 'Invalid signature' }, 401);
    }

    let event: StripeEvent;
    try {
        event = JSON.parse(payload) as StripeEvent;
    } catch {
        return jsonResponse({ error: 'Invalid JSON' }, 400);
    }

    // Check if this is a Crabigator event (has group_id in metadata)
    // This filters out events from other products (e.g., NewsBlur)
    if (!isCrabigatorEvent(event)) {
        // Acknowledge but don't process - not our event
        return jsonResponse({ received: true, skipped: 'not_crabigator' });
    }

    // Check for duplicate event (idempotency)
    const existingEvent = await env.DB.prepare(
        'SELECT event_id FROM webhook_events WHERE provider = ? AND event_id = ?'
    ).bind('stripe', event.id).first();

    if (existingEvent) {
        // Already processed - return success
        return jsonResponse({ received: true, duplicate: true });
    }

    // Process the event
    try {
        switch (event.type) {
            case 'checkout.session.completed':
                await handleCheckoutCompleted(env, event.data.object as StripeCheckoutSession);
                break;

            case 'customer.subscription.created':
            case 'customer.subscription.updated':
                await handleSubscriptionUpdated(env, event.data.object as StripeSubscription);
                break;

            case 'customer.subscription.deleted':
                await handleSubscriptionDeleted(env, event.data.object as StripeSubscription);
                break;

            case 'invoice.payment_succeeded':
                await handleInvoicePaymentSucceeded(env, event.data.object as StripeInvoice);
                break;

            case 'invoice.payment_failed':
                await handleInvoicePaymentFailed(env, event.data.object as StripeInvoice);
                break;

            default:
                // Unhandled event type - log but don't fail
                console.log(`Unhandled Stripe event type: ${event.type}`);
        }

        // Record event as processed
        await env.DB.prepare(
            'INSERT INTO webhook_events (provider, event_id, event_type) VALUES (?, ?, ?)'
        ).bind('stripe', event.id, event.type).run();

        return jsonResponse({ received: true });
    } catch (error) {
        console.error('Error processing Stripe webhook:', error);
        return jsonResponse({ error: 'Processing failed' }, 500);
    }
}

/**
 * Handle checkout.session.completed - create subscription record
 */
async function handleCheckoutCompleted(
    env: Env,
    session: StripeCheckoutSession
): Promise<void> {
    const groupId = session.metadata?.group_id;
    if (!groupId) {
        console.error('No group_id in checkout session metadata');
        return;
    }

    // Create or update subscription record
    // Status will be updated by subscription.updated event
    const subscriptionId = crypto.randomUUID();
    await env.DB.prepare(`
        INSERT INTO subscriptions (id, group_id, provider, provider_customer_id, provider_subscription_id, status, created_at, updated_at)
        VALUES (?, ?, 'stripe', ?, ?, 'pending', unixepoch(), unixepoch())
        ON CONFLICT (provider, provider_subscription_id) DO UPDATE SET
            group_id = excluded.group_id,
            provider_customer_id = excluded.provider_customer_id,
            updated_at = unixepoch()
    `).bind(subscriptionId, groupId, session.customer, session.subscription).run();

    // Sync usage DO to pick up new subscription
    await syncUsageDO(env, groupId);
}

/**
 * Handle customer.subscription.updated - update subscription status
 */
async function handleSubscriptionUpdated(
    env: Env,
    subscription: StripeSubscription
): Promise<void> {
    // Map Stripe status to our status
    let status = 'pending';
    if (subscription.status === 'active' || subscription.status === 'trialing') {
        status = 'active';
    } else if (subscription.status === 'past_due') {
        status = 'past_due';
    } else if (subscription.status === 'canceled' || subscription.status === 'unpaid') {
        status = 'canceled';
    }

    const groupId = subscription.metadata?.group_id;

    // Use UPSERT to handle race condition where this arrives before checkout.session.completed
    if (groupId) {
        const subscriptionId = crypto.randomUUID();
        await env.DB.prepare(`
            INSERT INTO subscriptions (id, group_id, provider, provider_customer_id, provider_subscription_id, status, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at)
            VALUES (?, ?, 'stripe', ?, ?, ?, ?, ?, ?, unixepoch(), unixepoch())
            ON CONFLICT (provider, provider_subscription_id) DO UPDATE SET
                status = excluded.status,
                current_period_start = excluded.current_period_start,
                current_period_end = excluded.current_period_end,
                cancel_at_period_end = excluded.cancel_at_period_end,
                updated_at = unixepoch()
        `).bind(
            subscriptionId,
            groupId,
            subscription.customer,
            subscription.id,
            status,
            subscription.current_period_start,
            subscription.current_period_end,
            subscription.cancel_at_period_end ? 1 : 0
        ).run();

        await syncUsageDO(env, groupId);
    } else {
        // No group_id in metadata - just update existing record
        await env.DB.prepare(`
            UPDATE subscriptions SET
                status = ?,
                current_period_start = ?,
                current_period_end = ?,
                cancel_at_period_end = ?,
                updated_at = unixepoch()
            WHERE provider = 'stripe' AND provider_subscription_id = ?
        `).bind(
            status,
            subscription.current_period_start,
            subscription.current_period_end,
            subscription.cancel_at_period_end ? 1 : 0,
            subscription.id
        ).run();

        // Get group_id and sync usage DO
        const row = await env.DB.prepare(
            'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
        ).bind('stripe', subscription.id).first<{ group_id: string }>();

        if (row) {
            await syncUsageDO(env, row.group_id);
        }
    }
}

/**
 * Handle customer.subscription.deleted - mark subscription as canceled
 */
async function handleSubscriptionDeleted(
    env: Env,
    subscription: StripeSubscription
): Promise<void> {
    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = 'canceled',
            updated_at = unixepoch()
        WHERE provider = 'stripe' AND provider_subscription_id = ?
    `).bind(subscription.id).run();

    // Get group_id and sync usage DO
    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('stripe', subscription.id).first<{ group_id: string }>();

    if (row) {
        await syncUsageDO(env, row.group_id);
    }
}

/**
 * Handle invoice.payment_succeeded - confirm subscription is active
 */
async function handleInvoicePaymentSucceeded(
    env: Env,
    invoice: StripeInvoice
): Promise<void> {
    if (!invoice.subscription) {
        return;
    }

    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = 'active',
            updated_at = unixepoch()
        WHERE provider = 'stripe' AND provider_subscription_id = ?
    `).bind(invoice.subscription).run();

    // Get group_id and sync usage DO
    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('stripe', invoice.subscription).first<{ group_id: string }>();

    if (row) {
        await syncUsageDO(env, row.group_id);
    }
}

/**
 * Handle invoice.payment_failed - mark subscription as past_due
 */
async function handleInvoicePaymentFailed(
    env: Env,
    invoice: StripeInvoice
): Promise<void> {
    if (!invoice.subscription) {
        return;
    }

    await env.DB.prepare(`
        UPDATE subscriptions SET
            status = 'past_due',
            updated_at = unixepoch()
        WHERE provider = 'stripe' AND provider_subscription_id = ?
    `).bind(invoice.subscription).run();

    // Get group_id and sync usage DO
    const row = await env.DB.prepare(
        'SELECT group_id FROM subscriptions WHERE provider = ? AND provider_subscription_id = ?'
    ).bind('stripe', invoice.subscription).first<{ group_id: string }>();

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
