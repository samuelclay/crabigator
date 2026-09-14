import { AsyncLocalStorage } from 'node:async_hooks';
import type { Env } from '../types/env';

export const MCP_LOG_KEY = 'mcp-call-log';
const MCP_LOG_LIMIT = 300;
const MCP_LOG_TTL = 60 * 60 * 24 * 3;

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

export async function listMcpLogs(env: Env, limit = 100): Promise<McpCallLog[]> {
    const raw = await env.TOKENS.get(MCP_LOG_KEY);
    if (!raw) return [];
    try {
        const parsed = JSON.parse(raw) as McpCallLog[];
        if (!Array.isArray(parsed)) return [];
        return parsed.slice(0, Math.max(1, Math.min(limit, MCP_LOG_LIMIT)));
    } catch {
        return [];
    }
}

async function appendMcpLog(env: Env, entry: McpCallLog): Promise<void> {
    try {
        const existing = await listMcpLogs(env, MCP_LOG_LIMIT);
        existing.unshift(entry);
        await env.TOKENS.put(MCP_LOG_KEY, JSON.stringify(existing.slice(0, MCP_LOG_LIMIT)), {
            expirationTtl: MCP_LOG_TTL,
        });
    } catch (error) {
        console.error('MCP log persist failed', error);
    }
}

export function summarizeArgs(raw: unknown): { arg_keys?: string[]; session_id?: string } {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {};
    const args = raw as Record<string, unknown>;
    const sessionId = typeof args.session_id === 'string' ? args.session_id : undefined;
    return { arg_keys: Object.keys(args), session_id: sessionId };
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
