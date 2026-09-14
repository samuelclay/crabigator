/**
 * Per-session identity chip. Keep glyphs, palettes, and the FNV-1a seed in
 * step with src/session_mark.rs and dashboard/js/session-mark.ts.
 */

export interface SessionMark {
    glyph: string;
    fg: [number, number, number];
    bg: [number, number, number];
    fg_hex: string;
    bg_hex: string;
}

const GLYPHS = [
    '▀▄▀', '▛█▜', '◢█◣', '▐█▌', '░█░', '⣏⣉⣹', '⢸⣿⡇', '⣀⣾⣀', '⠶⣿⠶', '⣹⠶⣏',
    '╭◈╮', '⟨※⟩', '╱◆╲', '◖◆◗', '⊏◆⊐', '⌈✦⌉', '◎◆◎', '◕‿◕', 'ᵔᴥᵔ', 'ᓚᘏᓗ',
    '◉ω◉', '¬‿¬', 'ᚼᛉᚼ', 'ᛏᛏ', '╠╬╣', '≈△≈', '◆◇◆', '▰▱▰', '⌬⌬', '⍟⍟',
] as const;

type Rgb = [number, number, number];

const PALETTES: Array<[Rgb, Rgb]> = [
    [[122, 16, 36], [255, 210, 168]],
    [[58, 34, 8], [240, 192, 64]],
    [[0, 24, 72], [94, 240, 255]],
    [[42, 23, 96], [228, 212, 255]],
    [[23, 36, 76], [183, 212, 255]],
    [[16, 32, 16], [180, 240, 106]],
    [[26, 26, 26], [232, 220, 192]],
    [[59, 18, 102], [240, 216, 120]],
    [[92, 42, 0], [255, 232, 200]],
    [[74, 8, 40], [255, 192, 216]],
    [[10, 42, 50], [126, 224, 232]],
    [[106, 16, 56], [255, 240, 224]],
    [[32, 16, 64], [208, 176, 255]],
    [[196, 92, 18], [26, 18, 8]],
    [[0, 60, 80], [128, 240, 200]],
    [[200, 232, 120], [26, 40, 8]],
    [[8, 40, 56], [240, 192, 64]],
    [[240, 200, 160], [58, 24, 16]],
    [[18, 72, 48], [232, 220, 192]],
    [[42, 16, 64], [224, 192, 255]],
    [[26, 32, 48], [159, 216, 200]],
    [[74, 32, 128], [232, 208, 255]],
    [[20, 48, 24], [192, 232, 120]],
    [[216, 224, 112], [26, 40, 8]],
];

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
    const preferredGlyph = Number(hash % BigInt(GLYPHS.length));
    const preferredPalette = Number((hash >> 8n) % BigInt(PALETTES.length));
    const usedGlyphs = new Set(taken.map((mark) => mark.glyph));
    let glyph: string = GLYPHS[preferredGlyph];
    if (usedGlyphs.size < GLYPHS.length) {
        for (let offset = 0; offset < GLYPHS.length; offset++) {
            const candidate = GLYPHS[(preferredGlyph + offset) % GLYPHS.length];
            if (!usedGlyphs.has(candidate)) {
                glyph = candidate;
                break;
            }
        }
    }
    const usedColors = new Set(
        taken.filter((mark) => mark.glyph === glyph).map(colorKey),
    );
    let pair = PALETTES[preferredPalette];
    for (let offset = 0; offset < PALETTES.length; offset++) {
        const candidate = PALETTES[(preferredPalette + offset) % PALETTES.length];
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
