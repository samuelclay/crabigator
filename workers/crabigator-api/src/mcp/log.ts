import { AsyncLocalStorage } from 'node:async_hooks';
import type { Env } from '../types/env';

export const MCP_LOG_KEY = 'mcp-call-log';
const MCP_LOG_LIMIT = 300;
const MCP_LOG_TTL = 60 * 60 * 24 * 3;

function groupLogKey(groupId: string): string {
    return `mcp-call-log:group:${groupId}`;
}

export interface McpSpan {
    name: string;
    ms: number;
}

export interface McpCallLog {
    ts: number;
    request_id: string;
    http_method: string;
    method: string;
    tool?: string;
    resource?: string;
    rpc_id?: string | number | null;
    group_id?: string;
    account_id?: string;
    session_id?: string;
    ms: number;
    auth_ms?: number;
    ok: boolean;
    error?: string;
    result_bytes?: number;
    spans: McpSpan[];
    arg_keys?: string[];
    text_len?: number;
    text_preview?: string;
    value?: string;
    key?: string;
    sse?: boolean;
    cf_ray?: string;
}

export class McpTrace {
    spans: McpSpan[] = [];

    async span<T>(name: string, fn: () => Promise<T>): Promise<T> {
        const start = Date.now();
        try {
            return await fn();
        } finally {
            this.spans.push({ name, ms: Date.now() - start });
        }
    }
}

const traces = new AsyncLocalStorage<McpTrace>();

export function runMcpTrace<T>(trace: McpTrace, fn: () => T): T {
    return traces.run(trace, fn);
}

export async function mcpSpan<T>(name: string, fn: () => Promise<T>): Promise<T> {
    const trace = traces.getStore();
    if (!trace) return fn();
    return trace.span(name, fn);
}

export async function persistMcpLog(
    env: Env,
    entry: McpCallLog,
    ctx?: ExecutionContext,
): Promise<void> {
    console.log(JSON.stringify({ mcp_call: true, ...entry }));
    const write = appendMcpLog(env, entry);
    if (ctx) ctx.waitUntil(write);
    else await write;
}

export interface McpLogFilter {
    groupId?: string;
    includeSse?: boolean;
    sessionId?: string;
    tool?: string;
}

export async function listMcpLogs(
    env: Env,
    limit = 100,
    filter: McpLogFilter = {},
): Promise<McpCallLog[]> {
    const cap = Math.max(1, Math.min(limit, MCP_LOG_LIMIT));
    const key = filter.groupId ? groupLogKey(filter.groupId) : MCP_LOG_KEY;
    let rows = await readLogKey(env, key);
    if (filter.groupId && !rows.length) {
        rows = (await readLogKey(env, MCP_LOG_KEY)).filter((row) => row.group_id === filter.groupId);
    }
    if (!filter.includeSse) rows = rows.filter((row) => !row.sse);
    if (filter.sessionId) rows = rows.filter((row) => row.session_id === filter.sessionId);
    if (filter.tool) rows = rows.filter((row) => row.tool === filter.tool);
    if (filter.groupId) rows = rows.filter((row) => row.group_id === filter.groupId);
    return rows.slice(0, cap);
}

async function readLogKey(env: Env, key: string): Promise<McpCallLog[]> {
    const raw = await env.TOKENS.get(key);
    if (!raw) return [];
    try {
        const parsed = JSON.parse(raw) as McpCallLog[];
        return Array.isArray(parsed) ? parsed : [];
    } catch {
        return [];
    }
}

async function appendMcpLog(env: Env, entry: McpCallLog): Promise<void> {
    try {
        await writeLogKey(env, MCP_LOG_KEY, entry);
        if (entry.group_id) await writeLogKey(env, groupLogKey(entry.group_id), entry);
    } catch (error) {
        console.error('MCP log persist failed', error);
    }
}

async function writeLogKey(env: Env, key: string, entry: McpCallLog): Promise<void> {
    const existing = await readLogKey(env, key);
    existing.unshift(entry);
    await env.TOKENS.put(key, JSON.stringify(existing.slice(0, MCP_LOG_LIMIT)), {
        expirationTtl: MCP_LOG_TTL,
    });
}

export function summarizeArgs(raw: unknown): Pick<McpCallLog, 'arg_keys' | 'session_id' | 'text_len' | 'text_preview' | 'value' | 'key'> {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {};
    const args = raw as Record<string, unknown>;
    const summary: ReturnType<typeof summarizeArgs> = { arg_keys: Object.keys(args) };
    if (typeof args.session_id === 'string') summary.session_id = args.session_id;
    if (typeof args.text === 'string') {
        summary.text_len = args.text.length;
        summary.text_preview = args.text.slice(0, 80);
    }
    if (typeof args.value === 'string') summary.value = args.value;
    if (typeof args.key === 'string') summary.key = args.key;
    return summary;
}

export function serverTimingHeader(totalMs: number, spans: McpSpan[]): string {
    const parts = [`total;dur=${totalMs}`];
    for (const span of spans) {
        const name = span.name.replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 40);
        if (!name) continue;
        parts.push(`${name};dur=${span.ms}`);
    }
    return parts.join(', ');
}
