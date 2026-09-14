/**
 * Per-session identity chip. Glyphs and palettes come from
 * src/session_mark.json, the same file the TUI includes.
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

/**
 * Assign chips the way the dashboard does: unique seeds, sorted, then claim
 * so two sessions in this set do not share a drawing until every drawing is used.
 */
export function assignSessionMarks<T extends object>(sessions: T[]): Map<string, SessionMark> {
    const marks = new Map<string, SessionMark>();
    const pending = [...new Set(sessions.map(sessionMarkSeed).filter(Boolean))]
        .sort((a, b) => a.localeCompare(b));
    for (const seed of pending) {
        marks.set(seed, sessionMarkClaim(seed, [...marks.values()]));
    }
    return marks;
}

export function withSessionMark<T extends object>(
    session: T,
    marks: Map<string, SessionMark>,
): T & { session_mark: SessionMark | null } {
    return {
        ...session,
        session_mark: marks.get(sessionMarkSeed(session)) ?? null,
    };
}
