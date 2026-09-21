import type { Env } from '../types/env';
import type { MobileAuth } from '../types/api';
import { assignSessionMarks, parseStoredSessionMark, withSessionMark } from '../session-mark';
import { mcpSpan } from './log';

export interface McpAuth extends MobileAuth {
    group_id: string;
    token: string;
}

export async function listGroupSessions(env: Env, groupId: string): Promise<Array<Record<string, unknown>>> {
    return mcpSpan('list_group_sessions', async () => {
        const doId = env.SESSION_LIST.idFromName('global');
        const stub = env.SESSION_LIST.get(doId);
        const url = new URL('https://internal/sessions');
        url.searchParams.set('group_id', groupId);
        const response = await stub.fetch(new Request(url.toString()));
        const data = await response.json() as { sessions?: Array<Record<string, unknown>> };
        return data.sessions || [];
    });
}

interface SessionMetaRow {
    id: string;
    client_session_id: string | null;
    titles: string | null;
    titles_changed_at: number | null;
    recap: string | null;
    repo_owner: string | null;
    repo_name: string | null;
    branch: string | null;
    uncommitted_files: number | null;
    additions: number | null;
    deletions: number | null;
    pr_scope: string | null;
    session_mark: string | null;
}

interface SessionPrRow {
    session_id: string;
    owner: string;
    repo: string;
    number: number;
    url: string;
    state: string | null;
    is_primary: number | null;
    data: string;
}

function placeholders(count: number): string {
    return Array.from({ length: count }, () => '?').join(', ');
}

function currentTitle(raw: string | null): { title: string; title_history: string[] } {
    try {
        const parsed: unknown = JSON.parse(raw || '[]');
        const title_history = Array.isArray(parsed)
            ? parsed.filter((item): item is string => typeof item === 'string' && item.length > 0)
            : [];
        return { title: title_history[title_history.length - 1] || '', title_history };
    } catch {
        return { title: '', title_history: [] };
    }
}

function compactPr(row: SessionPrRow): Record<string, unknown> {
    let title = '';
    let branch = '';
    let additions: number | null = null;
    let deletions: number | null = null;
    try {
        const parsed = JSON.parse(row.data) as Record<string, unknown>;
        if (typeof parsed.title === 'string') title = parsed.title;
        if (typeof parsed.branch === 'string') branch = parsed.branch;
        if (typeof parsed.additions === 'number') additions = parsed.additions;
        if (typeof parsed.deletions === 'number') deletions = parsed.deletions;
    } catch { /* stored blob may be incomplete */ }
    return {
        owner: row.owner,
        repo: row.repo,
        number: row.number,
        url: row.url,
        state: row.state || '',
        primary: row.is_primary === 1,
        title,
        branch,
        additions,
        deletions,
    };
}

/**
 * Add the identity chip, titles, and PRs the PR board shows, so an MCP
 * client can name a session the same way the website does.
 */
export async function enrichSessions(
    env: Env,
    sessions: Array<Record<string, unknown>>,
    groupId?: string,
): Promise<Array<Record<string, unknown>>> {
    return mcpSpan('enrich_sessions', () => enrichSessionsInner(env, sessions, groupId));
}

async function enrichSessionsInner(
    env: Env,
    sessions: Array<Record<string, unknown>>,
    groupId?: string,
): Promise<Array<Record<string, unknown>>> {
    if (!sessions.length) return sessions;
    const ids = sessions
        .map((session) => String(session.id || session.session_id || ''))
        .filter(Boolean);
    if (!ids.length) return sessions;

    const meta = await env.DB.prepare(`
        SELECT id, client_session_id, titles, titles_changed_at, recap,
               repo_owner, repo_name, branch, uncommitted_files, additions, deletions, pr_scope,
               session_mark
        FROM sessions
        WHERE id IN (${placeholders(ids.length)})
    `).bind(...ids).all<SessionMetaRow>();
    const metaById = new Map((meta.results || []).map((row) => [row.id, row]));

    const prRows = await env.DB.prepare(`
        SELECT session_id, owner, repo, number, url, state, is_primary, data
        FROM session_prs
        WHERE session_id IN (${placeholders(ids.length)})
        ORDER BY is_primary DESC, updated_at DESC
    `).bind(...ids).all<SessionPrRow>();
    const prsById = new Map<string, Record<string, unknown>[]>();
    for (const row of prRows.results || []) {
        const list = prsById.get(row.session_id) || [];
        list.push(compactPr(row));
        prsById.set(row.session_id, list);
    }

    const merged = sessions.map((session) => {
        const id = String(session.id || session.session_id || '');
        const row = metaById.get(id);
        const titles = currentTitle(row?.titles ?? null);
        return {
            ...session,
            client_session_id: row?.client_session_id || session.client_session_id || '',
            title: titles.title || session.title || '',
            title_history: titles.title_history,
            title_set_at: row?.titles_changed_at || 0,
            recap: row?.recap ? safeJson(row.recap) : session.recap ?? null,
            repo_owner: row?.repo_owner || '',
            repo_name: row?.repo_name || '',
            branch: row?.branch || '',
            uncommitted: row?.uncommitted_files || 0,
            additions: row?.additions || 0,
            deletions: row?.deletions || 0,
            pr_scope: row?.pr_scope || session.pr_scope || '',
            prs: prsById.get(id) || [],
            session_mark: parseStoredSessionMark(row?.session_mark) ?? session.session_mark ?? null,
        };
    });
    const occupancy: object[] = [...merged];
    if (groupId) {
        const siblings = await env.DB.prepare(`
            SELECT sessions.id, sessions.client_session_id, sessions.session_mark
            FROM sessions
            JOIN devices ON devices.id = sessions.device_id
            WHERE devices.group_id = ? AND sessions.is_active = 1
        `).bind(groupId).all<{ id: string; client_session_id: string | null; session_mark: string | null }>();
        occupancy.push(...(siblings.results || []).map((row) => ({
            id: row.id,
            client_session_id: row.client_session_id,
            session_mark: parseStoredSessionMark(row.session_mark),
        })));
    }
    const marks = assignSessionMarks(occupancy);
    return merged.map((session) => withSessionMark(session, marks));
}

function safeJson(raw: string): unknown {
    try {
        return JSON.parse(raw);
    } catch {
        return raw;
    }
}

export async function assertSessionInGroup(
    env: Env,
    groupId: string,
    sessionId: string,
): Promise<void> {
    return mcpSpan('assert_session_in_group', async () => {
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
    });
}

export async function sessionFetch(
    env: Env,
    sessionId: string,
    path: string,
    init?: RequestInit,
): Promise<Response> {
    return mcpSpan(`session_fetch ${path}`, async () => {
        const stub = env.SESSION.get(env.SESSION.idFromName(sessionId));
        return stub.fetch(new Request(`https://internal${path}`, init));
    });
}

export async function sessionSnapshot(
    env: Env,
    sessionId: string,
): Promise<Record<string, unknown>> {
    return mcpSpan('session_snapshot', async () => {
        await sessionFetch(env, sessionId, '/viewer-active', { method: 'POST' }).catch(() => null);
        const response = await sessionFetch(env, sessionId, '/snapshot');
        if (!response.ok) {
            throw new McpToolError('Failed to read session', 'SNAPSHOT_FAILED');
        }
        return await response.json() as Record<string, unknown>;
    });
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
