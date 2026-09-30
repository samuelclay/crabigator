import type { Env } from '../types/env';
import { getAppConfig } from '../config';
import type { CloudPromptData, GitCommitInfo, RecapEvent, SessionPr } from '../types/session';

/**
 * JSON-RPC codes from the MCP events draft. `data` carries the discriminator
 * clients branch on (reason, kind, feature).
 */
export class McpEventError extends Error {
    readonly code: number;
    readonly data?: Record<string, unknown>;

    constructor(code: number, message: string, data?: Record<string, unknown>) {
        super(message);
        this.name = 'McpEventError';
        this.code = code;
        this.data = data;
    }
}

export type CallbackReason =
    | 'connection_refused'
    | 'timeout'
    | 'tls_error'
    | 'http_4xx'
    | 'http_5xx'
    | 'challenge_failed';

export class CallbackEndpointError extends McpEventError {
    constructor(reason: CallbackReason, message = 'Callback endpoint failed verification') {
        super(-32015, message, { reason });
        this.name = 'CallbackEndpointError';
    }
}

interface JsonSchema {
    type?: string;
    description?: string;
    enum?: string[];
    properties?: Record<string, JsonSchema>;
    items?: JsonSchema;
    required?: string[];
    additionalProperties?: boolean;
}

export interface McpEventDefinition {
    name: string;
    description: string;
    delivery: Array<'webhook'>;
    inputSchema: JsonSchema;
    payloadSchema: JsonSchema;
}

export const McpEventName = {
    stateChanged: 'session.state_changed',
    prompt: 'session.prompt',
    recap: 'session.recap',
    titleChanged: 'session.title_changed',
    prsChanged: 'session.prs_changed',
    commit: 'session.commit',
    connected: 'session.connected',
    ended: 'session.ended',
} as const;

const sessionId = {
    type: 'string',
    description: 'Cloud session id. Omit to include every session on the account.',
};
const platform = {
    type: 'string',
    description: 'Only sessions on this platform: claude, codex, grok, or opencode.',
};
const cwd = {
    type: 'string',
    description: 'Only sessions whose working directory is exactly this path.',
};

const commonPayload = {
    session_id: { type: 'string', description: 'Cloud session id.' },
    platform: { type: 'string', description: 'claude, codex, grok, or opencode.' },
    cwd: { type: 'string', description: 'Working directory.' },
    title: { type: 'string', description: 'Current session title.' },
    url: { type: 'string', description: 'Dashboard URL for this session.' },
};

const optionSchema: JsonSchema = {
    type: 'object',
    properties: {
        label: { type: 'string' },
        value: { type: 'string' },
    },
    required: ['label', 'value'],
    additionalProperties: false,
};

const prSchema: JsonSchema = {
    type: 'object',
    properties: {
        owner: { type: 'string' },
        repo: { type: 'string' },
        number: { type: 'number' },
        url: { type: 'string' },
        state: { type: 'string' },
        primary: { type: 'boolean' },
    },
    required: ['owner', 'repo', 'number', 'url', 'state', 'primary'],
    additionalProperties: false,
};

function filters(extra: Record<string, JsonSchema> = {}): JsonSchema {
    return {
        type: 'object',
        properties: { session_id: sessionId, platform, cwd, ...extra },
        additionalProperties: false,
    };
}

function payload(extra: Record<string, JsonSchema>, required: string[]): JsonSchema {
    return {
        type: 'object',
        properties: { ...commonPayload, ...extra },
        required: ['session_id', ...required],
        additionalProperties: false,
    };
}

const STATES = ['ready', 'thinking', 'permission', 'question', 'complete'];

export const eventDefinitions: McpEventDefinition[] = [
    {
        name: McpEventName.stateChanged,
        description: 'A session moved to a new state: ready, thinking, permission, question, or complete.',
        delivery: ['webhook'],
        inputSchema: filters({
            state: {
                type: 'string',
                enum: STATES,
                description: 'Only deliver when the session enters this state.',
            },
        }),
        payloadSchema: payload({
            previous_state: { type: 'string', enum: STATES, description: 'State before this change.' },
            state: { type: 'string', enum: STATES, description: 'State after this change.' },
            device_name: { type: 'string', description: 'Name of the computer running the session.' },
        }, ['previous_state', 'state']),
    },
    {
        name: McpEventName.prompt,
        description: 'A session is showing a question, a permission prompt, or an exit-plan prompt.',
        delivery: ['webhook'],
        inputSchema: filters({
            prompt_type: {
                type: 'string',
                enum: ['question', 'permission', 'exit_plan'],
                description: 'Only deliver this kind of prompt.',
            },
        }),
        payloadSchema: payload({
            prompt_type: { type: 'string', enum: ['question', 'permission', 'exit_plan'] },
            summary: { type: 'string', description: 'The question text, or the tool name for a permission prompt.' },
            tool_name: { type: 'string', description: 'Tool waiting for permission.' },
            options: { type: 'array', items: optionSchema },
            questions: {
                type: 'array',
                items: {
                    type: 'object',
                    properties: {
                        question: { type: 'string' },
                        header: { type: 'string' },
                        multi_select: { type: 'boolean' },
                        options: { type: 'array', items: optionSchema },
                    },
                    required: ['question', 'options'],
                    additionalProperties: false,
                },
            },
        }, ['prompt_type', 'summary']),
    },
    {
        name: McpEventName.recap,
        description: 'A session finished a turn and stored a recap.',
        delivery: ['webhook'],
        inputSchema: filters(),
        payloadSchema: payload({
            headline: { type: 'string' },
            bullets: { type: 'array', items: { type: 'string' } },
            additions: { type: 'number' },
            deletions: { type: 'number' },
        }, ['headline', 'bullets']),
    },
    {
        name: McpEventName.titleChanged,
        description: 'A session title changed.',
        delivery: ['webhook'],
        inputSchema: filters(),
        payloadSchema: payload({
            previous_title: { type: 'string' },
        }, ['title', 'previous_title']),
    },
    {
        name: McpEventName.prsChanged,
        description: 'The pull requests a session is tracking changed.',
        delivery: ['webhook'],
        inputSchema: filters(),
        payloadSchema: payload({
            prs: { type: 'array', items: prSchema },
        }, ['prs']),
    },
    {
        name: McpEventName.commit,
        description: 'A session recorded a new git commit.',
        delivery: ['webhook'],
        inputSchema: filters(),
        payloadSchema: payload({
            hash: { type: 'string' },
            short_hash: { type: 'string' },
            subject: { type: 'string' },
            committed_at: { type: 'string', description: 'When the commit was created, as ISO 8601.' },
        }, ['hash', 'short_hash', 'subject', 'committed_at']),
    },
    {
        name: McpEventName.connected,
        description: 'The desktop attached its live stream for a session. This includes reconnects after a dropped connection.',
        delivery: ['webhook'],
        inputSchema: filters(),
        payloadSchema: payload({
            state: { type: 'string', enum: STATES },
            device_name: { type: 'string' },
        }, ['state']),
    },
    {
        name: McpEventName.ended,
        description: 'A session ended after the desktop stayed disconnected.',
        delivery: ['webhook'],
        inputSchema: filters(),
        payloadSchema: payload({
            state: { type: 'string', enum: STATES },
            ended_at: { type: 'string', description: 'When the session ended, as ISO 8601.' },
            device_name: { type: 'string' },
        }, ['state', 'ended_at']),
    },
];

export function eventByName(name: string): McpEventDefinition | undefined {
    return eventDefinitions.find((event) => event.name === name);
}

export function listEvents(params: Record<string, unknown>): { events: McpEventDefinition[] } {
    if (typeof params.cursor === 'string' && params.cursor.length > 0) {
        return { events: [] };
    }
    return { events: eventDefinitions };
}

export function validateArguments(
    event: McpEventDefinition,
    raw: unknown,
): Record<string, unknown> {
    if (raw == null) return {};
    if (typeof raw !== 'object' || Array.isArray(raw)) {
        throw new McpEventError(-32602, 'arguments must be an object');
    }
    const input = raw as Record<string, unknown>;
    const properties = event.inputSchema.properties || {};
    const clean: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(input)) {
        const spec = properties[key];
        if (!spec) throw new McpEventError(-32602, `Unknown argument: ${key}`);
        if (value == null) continue;
        if (spec.type === 'string') {
            if (typeof value !== 'string' || !value) {
                throw new McpEventError(-32602, `${key} must be a string`);
            }
            if (spec.enum && !spec.enum.includes(value)) {
                throw new McpEventError(-32602, `${key} must be one of ${spec.enum.join(', ')}`);
            }
            clean[key] = value;
            continue;
        }
        throw new McpEventError(-32602, `${key} is not a supported argument type`);
    }
    for (const key of event.inputSchema.required || []) {
        if (clean[key] == null) throw new McpEventError(-32602, `${key} is required`);
    }
    return clean;
}

/** Every subscription filter has to match. Omitted filters match everything. */
export function eventMatches(args: Record<string, unknown>, data: Record<string, unknown>): boolean {
    return Object.entries(args).every(([key, expected]) => data[key] === expected);
}

export function canonicalJson(value: unknown): string {
    if (value === null || typeof value !== 'object') return JSON.stringify(value) ?? 'null';
    if (Array.isArray(value)) return `[${value.map((item) => canonicalJson(item)).join(',')}]`;
    const entries = Object.entries(value as Record<string, unknown>)
        .filter(([, item]) => item !== undefined)
        .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
    return `{${entries.map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`).join(',')}}`;
}

export async function subscriptionId(
    principal: string,
    callbackUrl: string,
    name: string,
    args: Record<string, unknown>,
): Promise<string> {
    const material = canonicalJson({ principal, url: callbackUrl, name, arguments: args });
    const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(material));
    const hex = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
    return `sub_${hex.slice(0, 32)}`;
}

export function sessionDashboardUrl(env: Env, sessionIdValue: string): string | undefined {
    const configured = getAppConfig(env).public_origin;
    if (!configured) return undefined;
    try {
        const origin = new URL(configured).origin;
        return `${origin}/dashboard?session=${encodeURIComponent(sessionIdValue)}`;
    } catch {
        return undefined;
    }
}

function clip(value: string, max: number): string {
    return value.length <= max ? value : value.slice(0, max);
}

function optionList(options: Array<{ label?: string; value?: string }> | undefined) {
    return (options || []).slice(0, 12).map((option) => ({
        label: clip(option.label || '', 200),
        value: clip(option.value || '', 80),
    }));
}

export function promptEventData(prompt: CloudPromptData): Record<string, unknown> {
    if (prompt.prompt_type === 'question') {
        const questions = (prompt.questions || []).slice(0, 6).map((question) => ({
            question: clip(question.question || '', 500),
            ...(question.header ? { header: clip(question.header, 80) } : {}),
            ...(question.multi_select ? { multi_select: true } : {}),
            options: optionList(question.options),
        }));
        return {
            prompt_type: 'question',
            summary: questions[0]?.question || '',
            questions,
        };
    }
    if (prompt.prompt_type === 'permission') {
        const toolName = clip(prompt.tool_name || '', 120);
        return {
            prompt_type: 'permission',
            summary: toolName,
            tool_name: toolName,
            options: optionList(prompt.options),
        };
    }
    return {
        prompt_type: 'exit_plan',
        summary: 'Exit plan',
        options: optionList(prompt.options),
    };
}

export function recapEventData(event: RecapEvent): Record<string, unknown> | null {
    const latest = event.latest;
    if (!latest?.headline) return null;
    return {
        headline: clip(latest.headline, 300),
        bullets: (latest.bullets || []).slice(0, 6).map((bullet) => clip(String(bullet), 240)),
        additions: latest.line_delta?.additions || 0,
        deletions: latest.line_delta?.deletions || 0,
    };
}

export function commitEventData(commit: GitCommitInfo): Record<string, unknown> {
    return {
        hash: commit.hash,
        short_hash: commit.short_hash || commit.hash.slice(0, 7),
        subject: clip(commit.subject || '', 200),
        committed_at: new Date((commit.timestamp || 0) * 1000).toISOString(),
    };
}

export function compactSessionPrs(prs: Array<Partial<SessionPr>> | null | undefined) {
    const compact = [];
    for (const pr of prs || []) {
        if (!pr?.owner || !pr.repo || typeof pr.number !== 'number' || !pr.url) continue;
        compact.push({
            owner: pr.owner,
            repo: pr.repo,
            number: pr.number,
            url: pr.url,
            state: pr.state || '',
            primary: Boolean(pr.primary),
        });
        if (compact.length >= 20) break;
    }
    return compact;
}
