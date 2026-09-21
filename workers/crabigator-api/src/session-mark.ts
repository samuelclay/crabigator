/**
 * Per-session identity chip. The desktop chooses the glyph and colors and
 * stores them on the session. Callers draw that stored mark. The seed picker
 * below is only for sessions that have not published one yet.
 *
 * Glyphs and palettes come from src/session_mark.json, the same file the TUI includes.
 */

import markData from '../../../src/session_mark.json' with { type: 'json' };

export interface SessionMark {
    glyph: string;
    fg: [number, number, number];
    bg: [number, number, number];
    fg_hex: string;
    bg_hex: string;
}

type Rgb = [number, number, number];

export const SESSION_MARK_GLYPHS: string[] = markData.glyphs;
export const SESSION_MARK_PALETTES: Array<[Rgb, Rgb]> = markData.palettes as Array<[Rgb, Rgb]>;

function rgbHex(rgb: Rgb): string {
    return `#${rgb.map((n) => n.toString(16).padStart(2, '0')).join('')}`;
}

function pack(glyph: string, bg: Rgb, fg: Rgb): SessionMark {
    return { glyph, fg, bg, fg_hex: rgbHex(fg), bg_hex: rgbHex(bg) };
}

function fnv1a64(seed: string): bigint {
    let hash = 0xcbf29ce484222325n;
    for (const byte of new TextEncoder().encode(seed)) {
        hash ^= BigInt(byte);
        hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
    }
    return hash;
}

function colorKey(mark: { bg: Rgb; fg: Rgb }): string {
    return `${mark.bg.join(',')}/${mark.fg.join(',')}`;
}

/** Occupancy-aware pick. Glyphs stay unique until every drawing is taken. */
export function sessionMarkClaim(seed: string, taken: SessionMark[] = []): SessionMark {
    const hash = fnv1a64(seed);
    const preferredGlyph = Number(hash % BigInt(SESSION_MARK_GLYPHS.length));
    const preferredPalette = Number((hash >> 8n) % BigInt(SESSION_MARK_PALETTES.length));
    const usedGlyphs = new Set(taken.map((mark) => mark.glyph));
    let glyph: string = SESSION_MARK_GLYPHS[preferredGlyph];
    if (usedGlyphs.size < SESSION_MARK_GLYPHS.length) {
        for (let offset = 0; offset < SESSION_MARK_GLYPHS.length; offset++) {
            const candidate = SESSION_MARK_GLYPHS[(preferredGlyph + offset) % SESSION_MARK_GLYPHS.length];
            if (!usedGlyphs.has(candidate)) {
                glyph = candidate;
                break;
            }
        }
    }
    const usedColors = new Set(
        taken.filter((mark) => mark.glyph === glyph).map(colorKey),
    );
    let pair = SESSION_MARK_PALETTES[preferredPalette];
    for (let offset = 0; offset < SESSION_MARK_PALETTES.length; offset++) {
        const candidate = SESSION_MARK_PALETTES[(preferredPalette + offset) % SESSION_MARK_PALETTES.length];
        if (!usedColors.has(colorKey({ bg: candidate[0], fg: candidate[1] }))) {
            pair = candidate;
            break;
        }
    }
    return pack(glyph, pair[0], pair[1]);
}

export function sessionMarkFromSeed(seed: string): SessionMark {
    return sessionMarkClaim(seed, []);
}

function rgb(value: unknown): Rgb | null {
    if (!Array.isArray(value) || value.length !== 3) return null;
    const nums = value.map((item) => (typeof item === 'number' ? item : Number.NaN));
    if (nums.some((item) => !Number.isInteger(item) || item < 0 || item > 255)) return null;
    return [nums[0], nums[1], nums[2]];
}

/** A mark the desktop stored: a short glyph plus two RGB colors. */
export function parseStoredSessionMark(value: unknown): SessionMark | null {
    let raw = value;
    if (typeof raw === 'string') {
        if (!raw) return null;
        try {
            raw = JSON.parse(raw);
        } catch {
            return null;
        }
    }
    if (!raw || typeof raw !== 'object') return null;
    const obj = raw as { glyph?: unknown; fg?: unknown; bg?: unknown };
    const glyph = typeof obj.glyph === 'string' ? obj.glyph : '';
    const fg = rgb(obj.fg);
    const bg = rgb(obj.bg);
    if (!glyph || glyph.length > 16 || !fg || !bg) return null;
    return pack(glyph, bg, fg);
}

/** JSON for the sessions.session_mark column, or null when the value is not a mark. */
export function sessionMarkJson(value: unknown): string | null {
    const mark = parseStoredSessionMark(value);
    if (!mark) return null;
    return JSON.stringify({ glyph: mark.glyph, fg: mark.fg, bg: mark.bg });
}

/** Local crabigator id when present; otherwise the cloud session id. */
export function sessionMarkSeed(session: {
    client_session_id?: unknown;
    session_id?: unknown;
    id?: unknown;
}): string {
    const client = typeof session.client_session_id === 'string' ? session.client_session_id : '';
    const sessionId = typeof session.session_id === 'string' ? session.session_id : '';
    const id = typeof session.id === 'string' ? session.id : '';
    return client || sessionId || id;
}

function storedOn<T extends object>(session: T): SessionMark | null {
    return parseStoredSessionMark((session as { session_mark?: unknown }).session_mark);
}

/**
 * Marks for a set of sessions. A mark stored on the session wins. Sessions
 * without one still get a unique fallback drawing.
 */
export function assignSessionMarks<T extends object>(sessions: T[]): Map<string, SessionMark> {
    const marks = new Map<string, SessionMark>();
    const taken: SessionMark[] = [];
    const seeds = new Set<string>();
    for (const session of sessions) {
        const seed = sessionMarkSeed(session);
        if (!seed) continue;
        seeds.add(seed);
        const stored = storedOn(session);
        if (stored && !marks.has(seed)) {
            marks.set(seed, stored);
            taken.push(stored);
        }
    }
    const pending = [...seeds].filter((seed) => !marks.has(seed)).sort((a, b) => a.localeCompare(b));
    for (const seed of pending) {
        const mark = sessionMarkClaim(seed, taken);
        marks.set(seed, mark);
        taken.push(mark);
    }
    return marks;
}

export function withSessionMark<T extends object>(
    session: T,
    marks: Map<string, SessionMark>,
): T & { session_mark: SessionMark | null } {
    return {
        ...session,
        session_mark: storedOn(session) ?? marks.get(sessionMarkSeed(session)) ?? null,
    };
}
