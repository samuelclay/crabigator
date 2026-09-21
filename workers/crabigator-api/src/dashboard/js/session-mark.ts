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

        function storedSessionMark(session) {
            const mark = session && session.session_mark;
            if (!mark || typeof mark.glyph !== 'string' || !mark.glyph) return null;
            if (!Array.isArray(mark.bg) || !Array.isArray(mark.fg)) return null;
            if (mark.bg.length !== 3 || mark.fg.length !== 3) return null;
            return { glyph: mark.glyph, bg: mark.bg, fg: mark.fg };
        }

        function sessionForMark(session) {
            if (storedSessionMark(session)) return session;
            const id = session && (session.id || session.sessionId || session.session_id);
            if (!id || typeof allSessions === 'undefined') return session;
            const listed = allSessions.find(function (item) {
                return item && (item.id === id || item.session_id === id);
            });
            return listed || session;
        }

        function sessionMarkFor(session) {
            const record = sessionForMark(session);
            const stored = storedSessionMark(record);
            const seed = sessionMarkSeed(record);
            if (stored) {
                if (seed) sessionMarkAssigned.set(seed, stored);
                return stored;
            }
            if (sessionMarkAssigned.has(seed)) return sessionMarkAssigned.get(seed);
            const mark = sessionMarkClaim(seed, Array.from(sessionMarkAssigned.values()));
            sessionMarkAssigned.set(seed, mark);
            return mark;
        }

        function claimMarksFor(sessions) {
            const list = sessions || [];
            for (const session of list) {
                const stored = storedSessionMark(session);
                const seed = sessionMarkSeed(session);
                if (stored && seed) sessionMarkAssigned.set(seed, stored);
            }
            const pending = list
                .filter(session => sessionMarkSeed(session) && !sessionMarkAssigned.has(sessionMarkSeed(session)))
                .sort((a, b) => sessionMarkSeed(a).localeCompare(sessionMarkSeed(b)));
            for (const session of pending) sessionMarkFor(session);
        }

        function sessionMarkChipHtml(session) {
            const mark = sessionMarkFor(session);
            if (!mark) return '';
            return '<span class="session-mark" style="background:rgb(' + mark.bg.join(',')
                + ');color:rgb(' + mark.fg.join(',') + ')">' + escapeHtml(mark.glyph) + '</span>';
        }
`;
