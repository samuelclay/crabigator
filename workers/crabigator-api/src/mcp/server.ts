import type { Env } from '../types/env';
import { getAppConfig, getPublicOrigin, getRuntimeConfig } from '../config';
import { extractToken, verifyMobileToken } from '../auth/middleware';
import { getPrBoard } from '../handlers/pr-board';
import { callTool, listToolDescriptors } from './tools';
import {
    assertSessionInGroup,
    authedApiRequest,
    listGroupSessions,
    sessionSnapshot,
    McpToolError,
    type McpAuth,
} from './session';
import { formatScreen } from './text';
import { handleMcpOAuth, isMcpOAuthPath } from './oauth';

interface JsonRpcRequest {
    jsonrpc?: string;
    id?: string | number | null;
    method?: string;
    params?: Record<string, unknown>;
}

export function isMcpPath(pathname: string): boolean {
    return pathname === '/mcp' || pathname.startsWith('/mcp/') || isMcpOAuthPath(pathname);
}

export async function handleMcp(
    request: Request,
    env: Env,
): Promise<Response> {
    const url = new URL(request.url);
    if (isMcpOAuthPath(url.pathname)) {
        return handleMcpOAuth(request, env);
    }
    if (url.pathname !== '/mcp') {
        return json({ error: 'Not found' }, 404);
    }
    if (request.method === 'OPTIONS') {
        return new Response(null, { status: 204, headers: corsHeaders() });
    }

    const runtime = getRuntimeConfig(request, env);
    if (!runtime.capabilities.mcp) {
        return json({ error: 'MCP is disabled', code: 'FEATURE_DISABLED' }, 404);
    }

    const auth = await mcpAuth(request, env);
    if (!auth && request.method !== 'GET') {
        return unauthorized(request, env);
    }

    if (request.method === 'GET') {
        if (!auth) return unauthorized(request, env);
        return attentionStream(env, auth);
    }
    if (request.method !== 'POST') {
        return json({ error: 'Method not allowed' }, 405);
    }

    let payload: JsonRpcRequest | JsonRpcRequest[];
    try {
        payload = await request.json();
    } catch {
        return json(jsonRpcError(null, -32700, 'Parse error'));
    }

    const origin = getPublicOrigin(request, getAppConfig(env));
    if (Array.isArray(payload)) {
        const results = [];
        for (const item of payload) {
            const result = await handleRpc(item, auth, env, origin);
            if (result) results.push(result);
        }
        return json(results);
    }
    const result = await handleRpc(payload, auth, env, origin);
    if (!result) return new Response(null, { status: 202, headers: corsHeaders() });
    return json(result);
}

async function mcpAuth(request: Request, env: Env): Promise<McpAuth | null> {
    const auth = await verifyMobileToken(request, env);
    const token = extractToken(request);
    if (!auth || !token) return null;
    return { ...auth, group_id: auth.group_id || '', token };
}

async function handleRpc(
    message: JsonRpcRequest,
    auth: McpAuth | null,
    env: Env,
    origin: string,
): Promise<unknown | null> {
    const id = message.id ?? null;
    const method = message.method || '';
    const params = message.params || {};

    if (message.id === undefined && method.startsWith('notifications/')) {
        return null;
    }

    switch (method) {
        case 'initialize':
            return jsonRpcResult(id, {
                protocolVersion: '2025-06-18',
                capabilities: {
                    tools: { listChanged: false },
                    resources: { subscribe: true, listChanged: true },
                },
                serverInfo: { name: 'crabigator', version: '0.1.0' },
                instructions:
                    'Crabigator MCP. List sessions, inspect screens, answer prompts, and drive the PR board. If no desktops are linked, tell the user to run crabigator pair.',
            });
        case 'ping':
            return jsonRpcResult(id, {});
        case 'tools/list':
            return jsonRpcResult(id, { tools: listToolDescriptors() });
        case 'tools/call': {
            const gated = requireLinked(auth, id);
            if (gated) return gated;
            const name = String(params.name || '');
            return jsonRpcResult(id, await callTool(name, params.arguments, auth!, env, origin));
        }
        case 'resources/list': {
            const gated = requireLinked(auth, id);
            if (gated) return gated;
            return jsonRpcResult(id, { resources: await listResources(env, auth!) });
        }
        case 'resources/templates/list':
            return jsonRpcResult(id, {
                resourceTemplates: [
                    {
                        uriTemplate: 'crabigator://sessions/{id}',
                        name: 'Session snapshot',
                    },
                    {
                        uriTemplate: 'crabigator://sessions/{id}/screen',
                        name: 'Session screen',
                    },
                    {
                        uriTemplate: 'crabigator://sessions/{id}/scrollback',
                        name: 'Session scrollback',
                    },
                    {
                        uriTemplate: 'crabigator://sessions/{id}/prompt',
                        name: 'Current prompt',
                    },
                ],
            });
        case 'resources/read': {
            const gated = requireLinked(auth, id);
            if (gated) return gated;
            const uri = String(params.uri || '');
            try {
                return jsonRpcResult(id, await readResource(env, auth!, origin, uri));
            } catch (error) {
                const message = error instanceof McpToolError ? error.message : 'Failed to read resource';
                return jsonRpcError(id, -32000, message);
            }
        }
        case 'resources/subscribe':
        case 'resources/unsubscribe':
            return jsonRpcResult(id, {});
        default:
            return jsonRpcError(id, -32601, `Method not found: ${method}`);
    }
}

function requireLinked(auth: McpAuth | null, id: string | number | null): unknown | null {
    if (!auth) {
        return jsonRpcError(id, -32001, 'Unauthorized');
    }
    if (!auth.group_id) {
        return jsonRpcError(
            id,
            -32002,
            'No desktops linked. Run crabigator pair and enter the code, then reconnect.',
        );
    }
    return null;
}

async function listResources(env: Env, auth: McpAuth) {
    const sessions = await listGroupSessions(env, auth.group_id);
    return [
        { uri: 'crabigator://sessions', name: 'Live sessions', mimeType: 'application/json' },
        { uri: 'crabigator://prs', name: 'PR board', mimeType: 'application/json' },
        ...sessions.map((session) => ({
            uri: `crabigator://sessions/${session.id}`,
            name: String(session.title || session.cwd || session.id),
            mimeType: 'application/json',
        })),
    ];
}

async function readResource(env: Env, auth: McpAuth, origin: string, uri: string) {
    if (uri === 'crabigator://sessions') {
        return resourceJson(uri, await listGroupSessions(env, auth.group_id));
    }
    if (uri === 'crabigator://prs') {
        const request = authedApiRequest(origin, '/api/prs/board', auth.token, 'GET');
        const response = await getPrBoard(request, env);
        return resourceJson(uri, await response.json());
    }
    const match = uri.match(/^crabigator:\/\/sessions\/([^/]+)(?:\/(screen|scrollback|prompt))?$/);
    if (match) {
        await assertSessionInGroup(env, auth.group_id, match[1]);
        const snap = await sessionSnapshot(env, match[1]);
        if (match[2] === 'screen') {
            return resourceText(uri, formatScreen(typeof snap.screen === 'string' ? snap.screen : '', 'text') || '');
        }
        if (match[2] === 'scrollback') {
            return resourceText(uri, formatScreen(typeof snap.scrollback === 'string' ? snap.scrollback : '', 'text') || '');
        }
        if (match[2] === 'prompt') {
            return resourceJson(uri, snap.prompt);
        }
        return resourceJson(uri, snap);
    }
    return resourceText(uri, `Unknown resource: ${uri}`);
}

function resourceJson(uri: string, data: unknown) {
    return {
        contents: [{ uri, mimeType: 'application/json', text: JSON.stringify(data, null, 2) }],
    };
}

function resourceText(uri: string, text: string) {
    return {
        contents: [{ uri, mimeType: 'text/plain', text }],
    };
}

function attentionStream(env: Env, auth: McpAuth): Response {
    const encoder = new TextEncoder();
    let closed = false;
    const stream = new ReadableStream({
        async start(controller) {
            const send = (payload: unknown) => {
                if (closed) return;
                controller.enqueue(encoder.encode(`event: message\ndata: ${JSON.stringify(payload)}\n\n`));
            };
            send({
                jsonrpc: '2.0',
                method: 'notifications/resources/updated',
                params: { uri: 'crabigator://sessions' },
            });
            const started = Date.now();
            let last = '';
            while (!closed && Date.now() - started < 25000) {
                try {
                    const sessions = await listGroupSessions(env, auth.group_id);
                    const fingerprint = sessions.map((session) => `${session.id}:${session.state}`).join(',');
                    if (fingerprint !== last) {
                        last = fingerprint;
                        send({
                            jsonrpc: '2.0',
                            method: 'notifications/resources/updated',
                            params: { uri: 'crabigator://sessions' },
                        });
                    }
                } catch (error) {
                    console.error('MCP attention stream', error);
                }
                await new Promise((resolve) => setTimeout(resolve, 2000));
            }
            try { controller.close(); } catch { /* already closed */ }
        },
        cancel() {
            closed = true;
        },
    });
    return new Response(stream, {
        headers: {
            'Content-Type': 'text/event-stream',
            'Cache-Control': 'no-cache',
            ...corsHeaders(),
        },
    });
}

function unauthorized(request: Request, env: Env): Response {
    const origin = getPublicOrigin(request, getAppConfig(env));
    return new Response(
        JSON.stringify({ error: 'Unauthorized', code: 'UNAUTHORIZED' }),
        {
            status: 401,
            headers: {
                'Content-Type': 'application/json',
                'WWW-Authenticate': `Bearer realm="crabigator", resource_metadata="${origin}/.well-known/oauth-protected-resource"`,
                ...corsHeaders(),
            },
        },
    );
}

function jsonRpcResult(id: string | number | null, result: unknown) {
    return { jsonrpc: '2.0', id, result };
}

function jsonRpcError(id: string | number | null, code: number, message: string) {
    return { jsonrpc: '2.0', id, error: { code, message } };
}

function json(data: unknown, status = 200): Response {
    return new Response(JSON.stringify(data), {
        status,
        headers: { 'Content-Type': 'application/json', ...corsHeaders() },
    });
}

function corsHeaders(): Record<string, string> {
    return {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization, MCP-Protocol-Version, Mcp-Session-Id',
        'Access-Control-Expose-Headers': 'WWW-Authenticate, Mcp-Session-Id',
    };
}
