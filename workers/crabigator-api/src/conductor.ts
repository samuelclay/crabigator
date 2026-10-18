import type { Env } from './types/env';
import { authenticate } from './auth/middleware';
import { ConductorArchive } from './conductor-archive';
import { parseStoredSessionMark, sessionMarkFromSeed } from './session-mark';
import { presentConductorEntry } from './conductor-content';

export interface ConductorMessage { id: string; role: 'user' | 'assistant'; text: string; at: number; highlights?: Record<string, any>[] }
export interface ConductorTrack {
    id: string; request_id: string; session_id: string; title: string; instruction: string;
    state: string; updated_at: number; result?: string;
    project?: string; session_title?: string; platform?: string; device_name?: string; question?: string;
    events: { state: string; text: string; at: number; highlights?: Record<string, any>[] }[];
}
interface State {
    messages: ConductorMessage[]; tracks: ConductorTrack[]; revision: number;
    host?: { id: string; device: string; name: string; expires: number };
    turn?: { id: string; revision: number; host: string; replied?: boolean };
    handled: number; error?: string; finished_turn?: string;
}
const json = (data: unknown, status = 200) => Response.json(data, { status });
const fail = (message: string, status = 400): never => { throw new ConductorError(message, status); };
class ConductorError extends Error { constructor(message: string, readonly status: number) { super(message); } }
const text = (value: unknown, limit = 16000): string => {
    if (typeof value !== 'string' || !value.trim() || value.length > limit) fail('Invalid or oversized text.');
    return (value as string).trim();
};
const ended = (track: ConductorTrack) => ['answered', 'failed', 'cancelled'].includes(track.state);
const visibleTracks = (s: State) => {
    const recent = new Set(s.tracks.filter(ended).slice(-24));
    return s.tracks.filter(track => !ended(track) || recent.has(track))
        .sort((a, b) => Number(ended(a)) - Number(ended(b)) || b.updated_at - a.updated_at);
};
function trimHistory(s: State): void {
    const requests = new Set(s.messages.filter(m => m.role === 'user').slice(-50).map(m => m.id));
    // Keep every unfinished request even when its conversation has scrolled out of history.
    for (const track of s.tracks) if (!ended(track)) requests.add(track.request_id);
    s.messages = s.messages.filter((m, i) => m.role === 'user' ? requests.has(m.id) : i >= s.messages.length - 100);
    s.tracks = s.tracks.filter(t => !ended(t) || requests.has(t.request_id));
}


/** Every public operation is bound to the caller's account before reaching storage. */
export async function conductorRoute(request: Request, env: Env): Promise<Response> {
    const auth = await authenticate(request, env);
    if (!auth || auth.type === 'share') return json({ error: 'Sign in to use Conductor.' }, 401);
    const device = auth.type === 'device' ? auth.device_id : '';
    const group = auth.type === 'mobile' ? auth.group_id : (await env.DB.prepare('SELECT group_id FROM devices WHERE id = ?').bind(device).first<{ group_id: string }>())?.group_id;
    if (!group) return json({ error: 'Pair a desktop first.' }, 403);
    const action = new URL(request.url).pathname.split('/').pop();
    if (['host', 'tool', 'report'].includes(action || '') && !device) return json({ error: 'Desktop authentication required.' }, 403);
    let body: Record<string, unknown> = {};
    if (request.method === 'POST') {
        if (Number(request.headers.get('content-length') || 0) > 65000) return json({ error: 'Request too large.' }, 413);
        const raw = await request.text();
        if (raw.length > 65000) return json({ error: 'Request too large.' }, 413);
        try { body = JSON.parse(raw); } catch { return json({ error: 'Invalid JSON.' }, 400); }
        if (!body || Array.isArray(body) || typeof body !== 'object') return json({ error: 'Expected an object.' }, 400);
    }
    const stub = env.SESSION_LIST.get(env.SESSION_LIST.idFromName(`conductor:${group}`));
    return stub.fetch(new Request('https://internal/conductor', { method: 'POST', body: JSON.stringify({ action, body, group, device }) }));
}

/** Stored in a separate, account-scoped instance of the existing SessionListDO namespace. */
export async function handleConductor(request: Request, env: Env, storage: DurableObjectStorage): Promise<Response> {
    try {
        const { action, body, group, device } = await request.json() as { action: string; body: Record<string, any>; group: string; device: string };
        let count = await storage.get<number>('conductor:chunks') || 0;
        const pieces = new Map<string, string>();
        for (let start = 0; start < count; start += 128) {
            const batch = await storage.get<string>(Array.from({ length: Math.min(128, count - start) }, (_, i) => `conductor:${start + i}`));
            for (const [key, value] of batch) pieces.set(key, value);
        }
        const stored = count ? JSON.parse(Array.from({ length: count }, (_, i) => pieces.get(`conductor:${i}`) || '').join('')) as State : null;
        const s = stored || { messages: [], tracks: [], revision: 0, handled: 0 };
        const now = Date.now();
        const archiveEntries = () => [
            ...s.messages.map(m => ({ ...m, kind: 'message', project: 'Conductor' })),
            ...s.tracks.map(t => ({ ...t, kind: 'track' })),
        ];
        const previousEntries = archiveEntries();
        const previousIdentity = JSON.stringify(previousEntries);
        const metadata = (row: Record<string, any>) => {
            let titles: string[] = [], recap: Record<string, any> = {};
            try { titles = JSON.parse(row.titles || '[]'); } catch {}
            try { recap = JSON.parse(row.recap || '{}') || {}; } catch {}
            return { project: [row.repo_owner, row.repo_name].filter(Boolean).join('/') || String(row.cwd || '').split('/').filter(Boolean).pop() || 'Unknown project',
                session_title: Array.isArray(titles) ? String(titles.at(-1) || '') : '', platform: row.platform || '', device_name: row.device_name || '',
                session_mark: parseStoredSessionMark(row.session_mark) || sessionMarkFromSeed(row.client_session_id || row.session_id || row.id || ''),
                recap: String(recap.headline || ''), additions: row.additions || 0, deletions: row.deletions || 0,
                session_state: row.state || '', prompts: row.prompts || 0, completions: row.completions || 0, thinking_seconds: row.thinking_seconds || 0 };
        };
        const identities = new Map<string, Promise<Record<string, any> | null>>();
        const identity = (id: string) => {
            if (!identities.has(id)) identities.set(id, env.DB.prepare(`SELECT s.*, d.name AS device_name FROM sessions s JOIN devices d ON d.id=s.device_id WHERE s.id=? AND d.group_id=?`).bind(id, group).first<Record<string, any>>().then(row => row ? metadata(row) : null));
            return identities.get(id)!;
        };
        // Refresh identities once per session; keep the saved identity if the session has ended or moved.
        for (const track of s.tracks) {
            Object.assign(track, await identity(track.session_id) || {}, { question: track.question || s.messages.find(m => m.id === track.request_id)?.text || track.instruction });
        }
        for (const message of s.messages) if (message.highlights?.length) {
            message.highlights = await Promise.all(message.highlights.map(async h => ({ ...h, ...await identity(h.session_id) || {} })));
        }
        const presentMessage = async (message: ConductorMessage) => ({ ...presentConductorEntry(message),
            highlights: await Promise.all((message.highlights || []).map(async h => ({ ...presentConductorEntry(h), ...await identity(h.session_id) || {} }))) });
        const seeded = await storage.get<boolean>('conductor:archive-ready') || false;
        const archive = new ConductorArchive(storage, previousEntries, seeded);
        if (!seeded) { archive.sync(archiveEntries()); await storage.put('conductor:archive-ready', true); }
        const save = async () => {
            archive.sync(archiveEntries());
            const encoded = JSON.stringify(s), chunks: Record<string, string | number> = {};
            // Each value stays below the Durable Object's 128 KiB value limit, including Unicode.
            for (let offset = 0, i = 0; offset < encoded.length; offset += 20000, i++) chunks[`conductor:${i}`] = encoded.slice(offset, offset + 20000);
            chunks['conductor:chunks'] = Object.keys(chunks).length;
            await storage.transaction(async txn => {
                const entries = Object.entries(chunks);
                for (let i = 0; i < entries.length; i += 128) await txn.put(Object.fromEntries(entries.slice(i, i + 128)));
                for (let i = Number(chunks['conductor:chunks']); i < count; i++) await txn.delete(`conductor:${i}`);
            });
            count = Number(chunks['conductor:chunks']);
        };
        if (JSON.stringify(archiveEntries()) !== previousIdentity) await save();
        const host = () => {
            if (!device || !s.host || s.host.device !== device || s.host.id !== body.host_id || s.host.expires < now) fail('Conductor host lease expired.', 409);
        };
        const directory = async () => (await env.DB.prepare(`SELECT s.*, s.id as session_id, d.name as device_name
            FROM sessions s JOIN devices d ON d.id = s.device_id WHERE d.group_id = ? AND s.is_active = 1 AND s.ended_at IS NULL ORDER BY s.started_at DESC LIMIT 100`).bind(group).all<Record<string, any>>()).results;
        const session = (id: string) => env.SESSION.get(env.SESSION.idFromName(id));
        const snapshot = async (id: string) => {
            const owner = await env.DB.prepare('SELECT d.group_id FROM sessions s JOIN devices d ON d.id = s.device_id WHERE s.id = ?').bind(id).first<{ group_id: string }>();
            if (owner?.group_id !== group) fail('Session not found.', 404);
            await session(id).fetch(new Request('https://internal/viewer-active', { method: 'POST' }));
            const response = await session(id).fetch(new Request('https://internal/snapshot'));
            if (!response.ok) fail('Could not read session.', 502);
            return response.json() as Promise<Record<string, any>>;
        };
        // A missing desktop acknowledgment is uncertain, never permission to replay input.
        for (const track of s.tracks) {
            if (['sending', 'forwarded'].includes(track.state) && now - track.updated_at > 60000) {
                track.state = 'delivery_unknown'; track.updated_at = now;
                track.events.push({ state: track.state, text: 'No desktop acknowledgment. Open the session before asking to retry.', at: now });
                s.revision++; await save();
            }
        }
        if (action === 'conductor') {
            const snapshots = new Map<string, Promise<Record<string, any>>>();
            const tracks = await Promise.all(visibleTracks(s).map(async track => {
                try {
                    if (!snapshots.has(track.session_id)) snapshots.set(track.session_id, snapshot(track.session_id));
                    const snap = await snapshots.get(track.session_id)!;
                    return { ...presentConductorEntry(track), connected: snap.desktop_connected, attention: snap.attention_pending,
                        screen: String(snap.screen || '').slice(-12000), activity: String(snap.scrollback || '').slice(-12000) };
                } catch { return { ...presentConductorEntry(track), connected: false }; }
            }));
            return json({ messages: await Promise.all(s.messages.map(presentMessage)), tracks, host_online: !!s.host && s.host.expires > now, host_name: s.host?.name || '', working: !!s.turn && !!s.host && s.host.expires > now, error: s.error || null });
        }
        if (action === 'archive') {
            const query = typeof body.query === 'string' ? body.query.trim() : '';
            const status = body.status || 'all', offset = body.offset || 0;
            if (query.length > 200 || !['all', 'active', 'answered', 'failed', 'stopped'].includes(status) || !Number.isSafeInteger(offset) || offset < 0 || offset > 1000000) fail('Invalid archive search.');
            const page = archive.search(query, status, offset);
            page.entries = await Promise.all(page.entries.map(async entry => ({ ...entry, ...(entry.session_id ? await identity(entry.session_id) || {} : {}), ...(entry.kind === 'message' ? await presentMessage(entry as unknown as ConductorMessage) : {}) })));
            return json(page);
        }
        if (action === 'message') {
            const id = text(body.id, 100), content = text(body.text);
            const existing = s.messages.find(m => m.id === id) || archive.message(id);
            if (existing) {
                if (existing.text !== content) fail('This message ID already has different text.', 409);
                return json({ ok: true });
            }
            s.messages.push({ id, text: content, role: 'user', at: now }); trimHistory(s); s.revision++; s.error = undefined;
            await save(); return json({ ok: true });
        }
        if (action === 'host') {
            const id = text(body.host_id, 100);
            if (s.host && s.host.expires > now && (s.host.id !== id || s.host.device !== device)) fail('Another desktop is running Conductor.', 409);
            if (s.host?.id !== id) s.turn = undefined;
            s.host = { id, device, name: text(body.name, 200), expires: now + 60000 };
            if (body.claim && !s.turn && s.revision > s.handled) s.turn = { id: crypto.randomUUID(), revision: s.revision, host: id };
            await save();
            return json({ turn: body.claim ? s.turn : null, messages: s.messages.slice(-30), tracks: visibleTracks(s) });
        }
        if (action === 'report') {
            const track = s.tracks.find(t => t.id === body.task_id);
            if (!track) fail('Task not found.', 404);
            const owner = await env.DB.prepare('SELECT device_id, client_session_id FROM sessions WHERE id = ?').bind(track!.session_id).first<{ device_id: string; client_session_id: string }>();
            if (!owner || owner.device_id !== device || owner.client_session_id !== body.session_id) fail('This task belongs to another session.', 403);
            const status = text(body.state, 40), content = text(body.text);
            if (!['delivered', 'working', 'waiting', 'answered', 'failed', 'delivery_unknown'].includes(status)) fail('Invalid task state.');
            if (status === 'delivered' && !['sending', 'forwarded', 'delivery_unknown'].includes(track!.state)) return json({ ok: true, state: track!.state });
            if (status === 'delivery_unknown' && ['working', 'waiting'].includes(track!.state)) return json({ ok: true, state: track!.state });
            if (ended(track!)) return json({ ok: true, state: track!.state });
            if (track!.events.some(e => e.state === status && e.text === content)) return json({ ok: true });
            track!.state = status; track!.updated_at = now;
            track!.events.push({ state: status, text: content, at: now }); track!.events = track!.events.slice(-40);
            if (status === 'answered' || status === 'failed') track!.result = content;
            s.revision++; await save(); return json({ ok: true });
        }
        if (action !== 'tool') fail('Unknown Conductor action.', 404);
        host();
        // A lost response must not force the host to repeat a completed model turn.
        if (body.name === 'finish' && body.turn_id === s.finished_turn) return json({ ok: true });
        if (!s.turn || body.turn_id !== s.turn.id) fail('This model turn is no longer active.', 409);
        const args = body.arguments || {};
        switch (body.name) {
            case 'list_sessions': return json({ sessions: await directory() });
            case 'read_tracks': return json({ tracks: s.tracks });
            case 'stop_tracking': {
                const track = s.tracks.find(t => t.id === args.task_id);
                if (!track) fail('Task not found.', 404);
                if (!ended(track!)) {
                    track!.state = 'cancelled'; track!.updated_at = now;
                    track!.events.push({ state: 'cancelled', text: 'Stopped tracking this request. The project session continues running.', at: now });
                    s.revision++; await save();
                }
                return json({ track });
            }
            case 'inspect_session': {
                const id = text(args.session_id, 100);
                if (!(await directory()).some(row => row.session_id === id)) fail('Session not found.', 404);
                const snap = await snapshot(id);
                return json({ title: snap.title, state: snap.state, prompt: snap.prompt, activity: String(snap.scrollback || '').slice(-20000) });
            }
            case 'dispatch': {
                if (!Array.isArray(args.tasks) || !args.tasks.length || args.tasks.length > 12) fail('Choose between 1 and 12 sessions.');
                const requestId = text(args.request_id, 100);
                if (!s.messages.some(m => m.id === requestId && m.role === 'user')) fail('Unknown user request.');
                const rows = await directory();
                const tasks = args.tasks.map((task: any) => ({ session_id: text(task.session_id, 100), instruction: text(task.instruction), title: text(task.title, 200) }));
                if (new Set(tasks.map((t: any) => t.session_id)).size !== tasks.length) fail('A session may appear only once per dispatch.');
                for (const task of tasks) if (!rows.some(r => r.session_id === task.session_id)) fail('A target session is no longer available.', 409);
                if (s.tracks.filter(t => !ended(t)).length + tasks.filter((task: any) => !s.tracks.some(t => t.request_id === requestId && t.session_id === task.session_id)).length > 100) fail('Conductor already has 100 unfinished tasks. Finish them before dispatching more.');
                const result: ConductorTrack[] = [];
                for (const task of tasks) {
                    const existing = s.tracks.find(t => t.request_id === requestId && t.session_id === task.session_id);
                    if (existing) { result.push(existing); continue; }
                    const track: ConductorTrack = { ...task, ...metadata(rows.find(r => r.session_id === task.session_id)!), question: s.messages.find(m => m.id === requestId)!.text,
                        id: crypto.randomUUID(), request_id: requestId, state: 'sending', updated_at: now, events: [] };
                    s.tracks.push(track); await save(); // Reserve before touching a terminal. Never replay uncertain input.
                    let attempted = false;
                    try {
                        const snap = await snapshot(task.session_id);
                        if (!snap.conductor_command) throw new Error('Restart this session with the new Crabigator build to enable Conductor.');
                        const command = `${String(snap.conductor_command)} conductor report ${track.id}`;
                        const prompt = `[Conductor task ${track.id}]\n${task.instruction}\n\nThis is a follow-up from the user. Keep current work intact. Do only the requested work; a status request does not authorize publishing or releasing. Before starting, run: ${command} working 'Started this request'. Send meaningful progress with the same command and state working or waiting. When finished, run it with state answered and your complete answer (shell-quote the text). Use state failed if unable to complete. Include sources and unknowns; do not invent results. Your report is visible to the user and coordinator.`;
                        attempted = true;
                        const response = await session(task.session_id).fetch(new Request('https://internal/conductor-input', { method: 'POST', body: JSON.stringify({ task_id: track.id, text: prompt }) }));
                        if (!response.ok) { attempted = false; const detail = await response.json() as { error?: string }; throw new Error(detail.error || 'The session could not accept the follow-up.'); }
                        track.state = 'forwarded';
                        track.events.push({ state: 'forwarded', text: 'Sent to the desktop; waiting for acknowledgment.', at: Date.now() });
                    } catch (error) {
                        track.state = attempted ? 'delivery_unknown' : 'failed'; track.result = error instanceof Error ? error.message : 'Delivery failed. Check the session before retrying.';
                        track.events.push({ state: track.state, text: track.result, at: Date.now() });
                    }
                    await save(); result.push(track);
                }
                return json({ tracks: result });
            }
            case 'reply': {
                if (args.automatic && s.turn!.replied) return json({ ok: true });
                const message = text(args.text);
                if (args.highlights !== undefined && (!Array.isArray(args.highlights) || args.highlights.length > 12)) fail('Use at most 12 session highlights.');
                if (new Set((args.highlights || []).map((h: any) => h.session_id)).size !== (args.highlights || []).length) fail('Use one highlight per session.');
                const highlights = [];
                for (const item of args.highlights || []) {
                    const id = text(item.session_id, 100), content = text(item.text, 1000), meta = await identity(id);
                    if (!meta) fail('Highlight session not found.', 404);
                    const track = args.highlights && item.task_id ? s.tracks.find(t => t.id === item.task_id && t.session_id === id) : undefined;
                    if (item.task_id && !track) fail('Highlight request not found.', 404);
                    highlights.push({ session_id: id, task_id: track?.id, text: content, ...meta, state: track?.state || '', updated_at: track?.updated_at || now });
                }
                if (highlights.length || args.final === true) s.turn!.replied = true;
                s.messages.push({ id: crypto.randomUUID(), role: 'assistant', text: message, at: now, highlights });
                trimHistory(s); await save(); return json({ ok: true });
            }
            case 'finish': {
                if (args.error) s.error = text(args.error, 2000);
                s.handled = s.turn!.revision; s.finished_turn = s.turn!.id; s.turn = undefined; await save(); return json({ ok: true });
            }
            default: return fail('Unknown Conductor tool.');
        }
    } catch (error) {
        return json({ error: error instanceof Error ? error.message : 'Conductor failed.' }, error instanceof ConductorError ? error.status : 500);
    }
}
