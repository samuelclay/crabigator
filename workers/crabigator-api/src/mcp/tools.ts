import type { Env } from '../types/env';
import { getPrBoard, searchSessionScrollback } from '../handlers/pr-board';
import { setPrOverride } from '../handlers/pr-overrides';
import { setWatchedPr } from '../handlers/watched-prs';
import { handleTranscribe } from '../handlers/transcribe';
import {
    assertSessionInGroup,
    authedApiRequest,
    enrichSessions,
    listGroupSessions,
    needsAttention,
    sessionFetch,
    sessionSnapshot,
    McpToolError,
    type McpAuth,
} from './session';
import { formatScreen, tailLines, toolError, toolText } from './text';
import { mcpSpan } from './log';

type JsonSchema = Record<string, unknown>;

interface ToolDef {
    name: string;
    description: string;
    inputSchema: JsonSchema;
    handler: (args: Record<string, unknown>, auth: McpAuth, env: Env, origin: string) => Promise<unknown>;
}

const sessionIdProp = {
    session_id: { type: 'string', description: 'Cloud session id' },
};

const formatProp = {
    format: {
        type: 'string',
        enum: ['text', 'ansi'],
        description: 'Plain text (default) or raw ANSI',
    },
};

const prIdProps = {
    owner: { type: 'string', description: 'GitHub owner' },
    repo: { type: 'string', description: 'Repository name' },
    number: { type: 'number', description: 'Pull request number' },
};

export const toolDefs: ToolDef[] = [
    {
        name: 'list_sessions',
        description: 'List live Crabigator sessions for this account. Each session includes its identity chip (glyph + colors), title, and PRs — the same fields the PR board uses. Use needs_attention to find question or permission prompts.',
        inputSchema: {
            type: 'object',
            properties: {
                needs_attention: {
                    type: 'boolean',
                    description: 'Only sessions in question or permission',
                },
                state: {
                    type: 'string',
                    description: 'ready, thinking, permission, question, or complete',
                },
                cwd: { type: 'string', description: 'Exact working directory' },
                platform: { type: 'string', description: 'claude, codex, grok, or opencode' },
            },
        },
        handler: async (args, auth, env) => {
            let sessions = await listGroupSessions(env, auth.group_id);
            if (args.needs_attention) sessions = sessions.filter(needsAttention);
            if (typeof args.state === 'string') {
                sessions = sessions.filter((session) => session.state === args.state);
            }
            if (typeof args.cwd === 'string') {
                sessions = sessions.filter((session) => session.cwd === args.cwd);
            }
            if (typeof args.platform === 'string') {
                sessions = sessions.filter((session) => session.platform === args.platform);
            }
            return { sessions: await enrichSessions(env, sessions, auth.group_id) };
        },
    },
    {
        name: 'get_session',
        description: 'Snapshot of one session: identity chip (glyph + colors), title, PRs, recap, git, stats, prompt. Does not include the full screen.',
        inputSchema: {
            type: 'object',
            required: ['session_id'],
            properties: sessionIdProp,
        },
        handler: async (args, auth, env) => {
            const sessionId = requireString(args, 'session_id');
            await assertSessionInGroup(env, auth.group_id, sessionId);
            const snap = await sessionSnapshot(env, sessionId);
            const {
                screen: _screen,
                scrollback: _scrollback,
                ...rest
            } = snap;
            const [enriched] = await enrichSessions(env, [{ id: sessionId, ...rest }], auth.group_id);
            const livePrs = Array.isArray(rest.prs) ? rest.prs as unknown[] : [];
            return {
                ...enriched,
                title: rest.title || enriched.title,
                title_history: rest.title_history || enriched.title_history,
                prs: livePrs.length ? livePrs : enriched.prs,
                has_screen: Boolean(snap.screen),
                scrollback_preview: typeof snap.scrollback === 'string'
                    ? tailLines(String(snap.scrollback), 20)
                    : null,
            };
        },
    },
    {
        name: 'get_screen',
        description: 'Current terminal screen. Default format is plain text; pass format=ansi for escape codes.',
        inputSchema: {
            type: 'object',
            required: ['session_id'],
            properties: {
                ...sessionIdProp,
                ...formatProp,
            },
        },
        handler: async (args, auth, env) => {
            const sessionId = requireString(args, 'session_id');
            await assertSessionInGroup(env, auth.group_id, sessionId);
            const snap = await sessionSnapshot(env, sessionId);
            const format = args.format === 'ansi' ? 'ansi' : 'text';
            return {
                session_id: sessionId,
                desktop_connected: snap.desktop_connected,
                hibernated_ephemeral: snap.hibernated_ephemeral,
                format,
                screen: formatScreen(typeof snap.screen === 'string' ? snap.screen : null, format),
            };
        },
    },
    {
        name: 'get_scrollback',
        description: 'Accumulated session scrollback. Optional tail and substring query.',
        inputSchema: {
            type: 'object',
            required: ['session_id'],
            properties: {
                ...sessionIdProp,
                tail: { type: 'number', description: 'Keep only the last N lines' },
                query: { type: 'string', description: 'Keep lines that contain this text (min 3 characters)' },
                ...formatProp,
            },
        },
        handler: async (args, auth, env) => {
            const sessionId = requireString(args, 'session_id');
            await assertSessionInGroup(env, auth.group_id, sessionId);
            const snap = await sessionSnapshot(env, sessionId);
            const format = args.format === 'ansi' ? 'ansi' : 'text';
            let content = formatScreen(
                typeof snap.scrollback === 'string' ? snap.scrollback : '',
                format,
            ) || '';
            if (typeof args.query === 'string' && args.query.length >= 3) {
                const needle = args.query.toLowerCase();
                content = content.split('\n').filter((line) => line.toLowerCase().includes(needle)).join('\n');
            }
            if (typeof args.tail === 'number') content = tailLines(content, args.tail);
            return {
                session_id: sessionId,
                desktop_connected: snap.desktop_connected,
                hibernated_ephemeral: snap.hibernated_ephemeral,
                content,
            };
        },
    },
    {
        name: 'search_transcripts',
        description: 'Search live session scrollback across the account. Query must be at least 3 characters.',
        inputSchema: {
            type: 'object',
            required: ['query'],
            properties: { query: { type: 'string', description: 'Search text, at least 3 characters' } },
        },
        handler: async (args, auth, env, origin) => {
            const query = requireString(args, 'query');
            const request = authedApiRequest(
                origin,
                `/api/prs/search?q=${encodeURIComponent(query)}`,
                auth.token,
                'GET',
            );
            return jsonFromHandler(await searchSessionScrollback(request, env));
        },
    },
    {
        name: 'list_projects',
        description: 'Distinct project directories this account has used recently.',
        inputSchema: { type: 'object', properties: {} },
        handler: async (_args, auth, env) => {
            const cutoff = Math.floor(Date.now() / 1000) - 14 * 24 * 60 * 60;
            const result = await env.DB.prepare(`
                SELECT cwd,
                       MAX(COALESCE(sessions.ended_at, sessions.last_seen_at, sessions.started_at)) as last_active,
                       COUNT(*) as total_sessions
                FROM sessions
                JOIN devices ON devices.id = sessions.device_id
                WHERE devices.group_id = ?
                GROUP BY cwd
                HAVING last_active > ?
                ORDER BY last_active DESC
            `).bind(auth.group_id, cutoff).all<{ cwd: string; last_active: number; total_sessions: number }>();
            const hiddenJson = await env.TOKENS.get(`hidden-projects:${auth.group_id}`);
            const hidden = new Set<string>(hiddenJson ? JSON.parse(hiddenJson) : []);
            return {
                projects: (result.results || []).filter((row) => !hidden.has(row.cwd)),
            };
        },
    },
    {
        name: 'get_pr_board',
        description: 'Cross-session PR board for this account, same payload as the website: PRs, per-session titles, recaps, git, Slack, and identity chips (glyph + colors). Enough to recreate the board.',
        inputSchema: {
            type: 'object',
            properties: { days: { type: 'number', description: 'Look back this many days. Default 1.' } },
        },
        handler: async (args, auth, env, origin) => {
            const days = typeof args.days === 'number' ? args.days : 1;
            const request = authedApiRequest(origin, `/api/prs/board?days=${days}`, auth.token, 'GET');
            return jsonFromHandler(await getPrBoard(request, env));
        },
    },
    {
        name: 'get_draft',
        description: 'Unsent input text saved for a session.',
        inputSchema: {
            type: 'object',
            required: ['session_id'],
            properties: sessionIdProp,
        },
        handler: async (args, auth, env) => {
            const sessionId = requireString(args, 'session_id');
            await assertSessionInGroup(env, auth.group_id, sessionId);
            const response = await sessionFetch(env, sessionId, '/draft');
            return jsonFromHandler(response);
        },
    },
    {
        name: 'wait_for_attention',
        description: 'Wait until a session is in question or permission state, or until timeout_seconds (default 20, max 25).',
        inputSchema: {
            type: 'object',
            properties: {
                timeout_seconds: {
                    type: 'number',
                    description: 'Seconds to wait. Default 20, max 25.',
                },
            },
        },
        handler: async (args, auth, env) => {
            const timeoutMs = Math.min(Math.max(Number(args.timeout_seconds) || 20, 1), 25) * 1000;
            const started = Date.now();
            while (true) {
                const sessions = (await listGroupSessions(env, auth.group_id)).filter(needsAttention);
                if (sessions.length) return { timed_out: false, sessions };
                if (Date.now() - started >= timeoutMs) {
                    return { timed_out: true, sessions: [] };
                }
                await sleep(500);
            }
        },
    },
    {
        name: 'send_input',
        description: 'Type text into the session and press Enter. Same as the dashboard message box.',
        inputSchema: {
            type: 'object',
            required: ['session_id', 'text'],
            properties: {
                ...sessionIdProp,
                text: { type: 'string', description: 'Text to type, then Enter' },
            },
        },
        handler: async (args, auth, env) => {
            return postSession(env, auth, requireString(args, 'session_id'), '/answer', {
                text: requireString(args, 'text'),
            });
        },
    },
    {
        name: 'choose_option',
        description: 'Choose a permission or question option by its value (usually "1", "2", ...). Optional instructions use the Tab-instructions path.',
        inputSchema: {
            type: 'object',
            required: ['session_id', 'value'],
            properties: {
                ...sessionIdProp,
                value: { type: 'string', description: 'Option value, usually "1", "2", …' },
                instructions: { type: 'string', description: 'Optional Tab-instructions before confirming' },
            },
        },
        handler: async (args, auth, env) => {
            const sessionId = requireString(args, 'session_id');
            const value = requireString(args, 'value');
            const instructions = typeof args.instructions === 'string' ? args.instructions : '';
            if (instructions) {
                return postSession(env, auth, sessionId, '/key-sequence', {
                    steps: [
                        { type: 'key', key: 'tab' },
                        { type: 'text', text: instructions },
                        { type: 'key', key: 'enter' },
                    ],
                });
            }
            return postSession(env, auth, sessionId, '/answer', { text: value });
        },
    },
    {
        name: 'send_keys',
        description: 'Send a named key (shift_tab, escape, tab, enter, up, down, ctrl_c) or a key-sequence of steps.',
        inputSchema: {
            type: 'object',
            required: ['session_id'],
            properties: {
                ...sessionIdProp,
                key: {
                    type: 'string',
                    description: 'Named key: shift_tab, escape, tab, enter, up, down, ctrl_c',
                },
                steps: {
                    type: 'array',
                    description: 'Key sequence of { type: "key"|"text", key?, text? } steps',
                },
            },
        },
        handler: async (args, auth, env) => {
            const sessionId = requireString(args, 'session_id');
            if (Array.isArray(args.steps)) {
                return postSession(env, auth, sessionId, '/key-sequence', { steps: args.steps });
            }
            return postSession(env, auth, sessionId, '/key', {
                key: requireString(args, 'key'),
            });
        },
    },
    {
        name: 'set_draft',
        description: 'Save unsent input text for a session without submitting it.',
        inputSchema: {
            type: 'object',
            required: ['session_id', 'text'],
            properties: {
                ...sessionIdProp,
                text: { type: 'string', description: 'Unsent input to save' },
            },
        },
        handler: async (args, auth, env) => {
            return postSession(env, auth, requireString(args, 'session_id'), '/draft', {
                text: requireString(args, 'text'),
            });
        },
    },
    {
        name: 'spawn_session',
        description: 'Ask a live desktop to open a new Crabigator terminal in cwd.',
        inputSchema: {
            type: 'object',
            required: ['cwd'],
            properties: {
                cwd: { type: 'string', description: 'Working directory for the new session' },
                platform: { type: 'string', description: 'claude, codex, grok, or opencode' },
            },
        },
        handler: async (args, auth, env) => {
            const cwd = requireString(args, 'cwd');
            const platform = typeof args.platform === 'string' ? args.platform : undefined;
            const sessions = await listGroupSessions(env, auth.group_id);
            const fallbackUrl = `crabigator://spawn?cwd=${encodeURIComponent(cwd)}${platform ? `&platform=${encodeURIComponent(platform)}` : ''}`;
            if (!sessions.length) {
                return { ok: false, fallback: 'url_scheme', url: fallbackUrl };
            }
            for (const session of sessions) {
                const id = String(session.id || '');
                if (!id) continue;
                const response = await sessionFetch(env, id, '/spawn', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ cwd, platform }),
                });
                if (response.ok) return jsonFromHandler(response);
            }
            return { ok: false, fallback: 'url_scheme', url: fallbackUrl };
        },
    },
    {
        name: 'hide_project',
        description: 'Hide a project directory from the dashboard project list.',
        inputSchema: {
            type: 'object',
            required: ['cwd'],
            properties: { cwd: { type: 'string', description: 'Project directory to hide' } },
        },
        handler: async (args, auth, env) => {
            const cwd = requireString(args, 'cwd');
            const key = `hidden-projects:${auth.group_id}`;
            const existing = await env.TOKENS.get(key);
            const hidden: string[] = existing ? JSON.parse(existing) : [];
            if (!hidden.includes(cwd)) hidden.push(cwd);
            await env.TOKENS.put(key, JSON.stringify(hidden), { expirationTtl: 60 * 60 * 24 * 30 });
            return { ok: true };
        },
    },
    {
        name: 'transcribe_audio',
        description: 'Transcribe base64 audio with Whisper. Returns text; call send_input to submit it.',
        inputSchema: {
            type: 'object',
            required: ['audio_base64'],
            properties: {
                audio_base64: { type: 'string', description: 'Audio bytes, base64-encoded' },
                mime_type: { type: 'string', description: 'Audio MIME type. Default audio/webm.' },
                filename: { type: 'string', description: 'Original filename' },
            },
        },
        handler: async (args, auth, env, origin) => {
            const audio = requireString(args, 'audio_base64');
            const mime = typeof args.mime_type === 'string' ? args.mime_type : 'audio/webm';
            const filename = typeof args.filename === 'string' ? args.filename : 'audio.webm';
            const binary = Uint8Array.from(atob(audio), (ch) => ch.charCodeAt(0));
            const form = new FormData();
            form.append('file', new Blob([binary], { type: mime }), filename);
            const request = new Request(`${origin}/api/transcribe`, {
                method: 'POST',
                headers: { Authorization: `Bearer ${auth.token}` },
                body: form,
            });
            return jsonFromHandler(await handleTranscribe(request, env));
        },
    },
    {
        name: 'watch_pr',
        description: 'Add a PR to the account watch list. Pass remove=true to unwatch.',
        inputSchema: {
            type: 'object',
            required: ['owner', 'repo', 'number'],
            properties: {
                ...prIdProps,
                url: { type: 'string', description: 'Optional GitHub PR URL' },
                remove: { type: 'boolean', description: 'Set true to unwatch' },
            },
        },
        handler: async (args, auth, env, origin) => {
            const request = authedApiRequest(origin, '/api/prs/watched', auth.token, 'POST', {
                owner: requireString(args, 'owner'),
                repo: requireString(args, 'repo'),
                number: requireNumber(args, 'number'),
                url: typeof args.url === 'string' ? args.url : undefined,
                remove: args.remove === true,
            });
            return jsonFromHandler(await setWatchedPr(request, env));
        },
    },
    {
        name: 'unwatch_pr',
        description: 'Remove a PR from the account watch list.',
        inputSchema: {
            type: 'object',
            required: ['owner', 'repo', 'number'],
            properties: prIdProps,
        },
        handler: async (args, auth, env, origin) => {
            const request = authedApiRequest(origin, '/api/prs/watched', auth.token, 'POST', {
                owner: requireString(args, 'owner'),
                repo: requireString(args, 'repo'),
                number: requireNumber(args, 'number'),
                remove: true,
            });
            return jsonFromHandler(await setWatchedPr(request, env));
        },
    },
    {
        name: 'set_pr_disposition',
        description: 'Promote, demote, dismiss, or reset a PR. disposition is primary, secondary, dismissed, or auto. Optional scope: session:<id> or path:<cwd>.',
        inputSchema: {
            type: 'object',
            required: ['owner', 'repo', 'number', 'disposition'],
            properties: {
                ...prIdProps,
                disposition: {
                    type: 'string',
                    description: 'primary, secondary, dismissed, or auto',
                },
                scope: {
                    type: 'string',
                    description: 'session:<id> or path:<cwd>. Empty applies group-wide.',
                },
            },
        },
        handler: async (args, auth, env, origin) => {
            const request = authedApiRequest(origin, '/api/pr-overrides', auth.token, 'POST', {
                owner: requireString(args, 'owner'),
                repo: requireString(args, 'repo'),
                number: requireNumber(args, 'number'),
                disposition: requireString(args, 'disposition'),
                scope: typeof args.scope === 'string' ? args.scope : '',
            });
            return jsonFromHandler(await setPrOverride(request, env));
        },
    },
];

export function listToolDescriptors(): Array<{ name: string; description: string; inputSchema: JsonSchema }> {
    return toolDefs.map(({ name, description, inputSchema }) => ({ name, description, inputSchema }));
}

export async function callTool(
    name: string,
    rawArgs: unknown,
    auth: McpAuth,
    env: Env,
    origin: string,
): Promise<{ content: Array<{ type: 'text'; text: string }>; isError?: true }> {
    const tool = toolDefs.find((entry) => entry.name === name);
    if (!tool) return toolError(`Unknown tool: ${name}`);
    try {
        const args = (rawArgs && typeof rawArgs === 'object') ? rawArgs as Record<string, unknown> : {};
        const result = await mcpSpan(`tool:${name}`, () => tool.handler(args, auth, env, origin));
        return toolText(result);
    } catch (error) {
        if (error instanceof McpToolError) return toolError(`${error.code}: ${error.message}`);
        console.error('MCP tool failed', name, error);
        return toolError(error instanceof Error ? error.message : 'Tool failed');
    }
}

function requireString(args: Record<string, unknown>, key: string): string {
    const value = args[key];
    if (typeof value !== 'string' || !value) {
        throw new McpToolError(`Missing ${key}`, 'INVALID_ARGS');
    }
    return value;
}

function requireNumber(args: Record<string, unknown>, key: string): number {
    const value = args[key];
    if (typeof value !== 'number' || !Number.isFinite(value)) {
        throw new McpToolError(`Missing ${key}`, 'INVALID_ARGS');
    }
    return value;
}

async function jsonFromHandler(response: Response): Promise<unknown> {
    const data = await response.json().catch(() => ({ error: 'Invalid response' }));
    if (!response.ok) {
        const message = data && typeof data === 'object' && 'error' in data
            ? String((data as { error: unknown }).error)
            : `HTTP ${response.status}`;
        throw new McpToolError(message, 'UPSTREAM');
    }
    return data;
}

async function postSession(
    env: Env,
    auth: McpAuth,
    sessionId: string,
    path: string,
    body: unknown,
): Promise<unknown> {
    await assertSessionInGroup(env, auth.group_id, sessionId);
    const response = await sessionFetch(env, sessionId, path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    });
    return jsonFromHandler(response);
}

function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
