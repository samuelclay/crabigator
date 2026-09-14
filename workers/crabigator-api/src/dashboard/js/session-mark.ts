import { SESSION_MARK_GLYPHS, SESSION_MARK_PALETTES } from '../../session-mark';

// Glyphs and palettes are inlined from src/session_mark.json at build time.
export const sessionMarkJs = `
        const SESSION_MARK_GLYPHS = ${JSON.stringify(SESSION_MARK_GLYPHS)};
        const SESSION_MARK_PALETTES = ${JSON.stringify(SESSION_MARK_PALETTES)};

        function sessionMarkFnv1a(seed) {
            let hash = 0xcbf29ce484222325n;
            for (const byte of new TextEncoder().encode(String(seed || ''))) {
                hash ^= BigInt(byte);
                hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
            }
            return hash;
        }

        const sessionMarkAssigned = new Map();

        function sessionMarkColorKey(mark) {
            return mark.bg.join(',') + '/' + mark.fg.join(',');
        }

        function sessionMarkClaim(seed, taken) {
            const hash = sessionMarkFnv1a(seed);
            const preferredGlyph = Number(hash % BigInt(SESSION_MARK_GLYPHS.length));
            const preferredPalette = Number((hash >> 8n) % BigInt(SESSION_MARK_PALETTES.length));
            const usedGlyphs = new Set(taken.map(mark => mark.glyph));
            let glyph = SESSION_MARK_GLYPHS[preferredGlyph];
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
                taken.filter(mark => mark.glyph === glyph).map(sessionMarkColorKey)
            );
            let pair = SESSION_MARK_PALETTES[preferredPalette];
            for (let offset = 0; offset < SESSION_MARK_PALETTES.length; offset++) {
                const candidate = SESSION_MARK_PALETTES[(preferredPalette + offset) % SESSION_MARK_PALETTES.length];
                if (!usedColors.has(candidate[0].join(',') + '/' + candidate[1].join(','))) {
                    pair = candidate;
                    break;
                }
            }
            return { glyph, bg: pair[0], fg: pair[1] };
        }

        function sessionMarkSeed(session) {
            return (session && (session.client_session_id || session.session_id || session.id)) || '';
        }

        function sessionMarkFor(session) {
            const seed = sessionMarkSeed(session);
            if (sessionMarkAssigned.has(seed)) return sessionMarkAssigned.get(seed);
            const mark = sessionMarkClaim(seed, Array.from(sessionMarkAssigned.values()));
            sessionMarkAssigned.set(seed, mark);
            return mark;
        }

        function claimMarksFor(sessions) {
            const pending = (sessions || [])
                .filter(session => sessionMarkSeed(session) && !sessionMarkAssigned.has(sessionMarkSeed(session)))
                .sort((a, b) => sessionMarkSeed(a).localeCompare(sessionMarkSeed(b)));
            for (const session of pending) sessionMarkFor(session);
        }

        function sessionMarkChipHtml(session) {
            const mark = sessionMarkFor(session);
            return '<span class="session-mark" style="background:rgb(' + mark.bg.join(',')
                + ');color:rgb(' + mark.fg.join(',') + ')">' + mark.glyph + '</span>';
        }
`;
