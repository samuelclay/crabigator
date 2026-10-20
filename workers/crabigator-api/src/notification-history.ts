import type { Env } from './types/env';

export type PushFetch = (url: string, init: RequestInit) => Promise<Response>;
export class PushOutcomeUnknownError extends Error {}

/** Record before sending. A lost response is unknown, never proof of rejection. */
export function auditedPushFetch(env: Env, deliveryId: string): PushFetch {
    return async (url, init) => {
        const id = crypto.randomUUID();
        const payload = JSON.parse(String(init.body));
        // FCM addresses the device in its body; APNs uses the URL. Neither is retained.
        if (payload.message) delete payload.message.token;
        await env.DB.prepare(`INSERT INTO notification_attempts (id, delivery_id, started_at, payload)
            VALUES (?, ?, ?, ?)`).bind(id, deliveryId, Date.now(), JSON.stringify(payload)).run();
        let response: Response;
        try {
            response = await fetch(url, init);
        } catch (error) {
            await env.DB.prepare(`UPDATE notification_attempts SET status = 'unknown', completed_at = ? WHERE id = ?`)
                .bind(Date.now(), id).run().catch(() => {
                    console.error('Could not record unknown push outcome', { attempt_id: id });
                });
            throw new PushOutcomeUnknownError('Push response was lost', { cause: error });
        }
        const body = await response.clone().json().catch(() => null) as {
            name?: string; reason?: string; error?: { status?: string };
        } | null;
        try {
            await env.DB.prepare(`UPDATE notification_attempts SET status = ?, completed_at = ?, http_status = ?,
                provider_id = ?, provider_reason = ? WHERE id = ?`).bind(response.ok ? 'accepted' : 'rejected', Date.now(),
                response.status, response.headers.get('apns-id') || body?.name || null,
                body?.reason || body?.error?.status || null, id).run();
        } catch (error) {
            throw new PushOutcomeUnknownError('Push response could not be recorded', { cause: error });
        }
        return response;
    };
}
