import type { Env } from '../types/env';
import type { MobileAuth } from '../types/api';

export interface McpAuth extends MobileAuth {
    group_id: string;
    token: string;
}

export async function listGroupSessions(env: Env, groupId: string): Promise<Array<Record<string, unknown>>> {
    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    const url = new URL('https://internal/sessions');
    url.searchParams.set('group_id', groupId);
    const response = await stub.fetch(new Request(url.toString()));
    const data = await response.json() as { sessions?: Array<Record<string, unknown>> };
    return data.sessions || [];
}

export async function assertSessionInGroup(
    env: Env,
    groupId: string,
    sessionId: string,
): Promise<void> {
    const session = await env.DB.prepare(`
        SELECT devices.group_id as group_id
        FROM sessions
        JOIN devices ON devices.id = sessions.device_id
        WHERE sessions.id = ?
    `).bind(sessionId).first<{ group_id: string | null }>();
    if (!session) {
        throw new McpToolError('Session not found', 'NOT_FOUND');
    }
    if (session.group_id !== groupId) {
        throw new McpToolError('Forbidden', 'FORBIDDEN');
    }
}

export async function sessionFetch(
    env: Env,
    sessionId: string,
    path: string,
    init?: RequestInit,
): Promise<Response> {
    const stub = env.SESSION.get(env.SESSION.idFromName(sessionId));
    return stub.fetch(new Request(`https://internal${path}`, init));
}

export async function sessionSnapshot(
    env: Env,
    sessionId: string,
): Promise<Record<string, unknown>> {
    await sessionFetch(env, sessionId, '/viewer-active', { method: 'POST' }).catch(() => null);
    const response = await sessionFetch(env, sessionId, '/snapshot');
    if (!response.ok) {
        throw new McpToolError('Failed to read session', 'SNAPSHOT_FAILED');
    }
    return await response.json() as Record<string, unknown>;
}

export class McpToolError extends Error {
    constructor(message: string, public readonly code: string) {
        super(message);
        this.name = 'McpToolError';
    }
}

export function needsAttention(session: Record<string, unknown>): boolean {
    const state = String(session.state || '');
    return state === 'question' || state === 'permission';
}

export function authedApiRequest(
    origin: string,
    path: string,
    token: string,
    method: string,
    body?: unknown,
): Request {
    return new Request(`${origin}${path}`, {
        method,
        headers: {
            Authorization: `Bearer ${token}`,
            'Content-Type': 'application/json',
        },
        body: body === undefined ? undefined : JSON.stringify(body),
    });
}
