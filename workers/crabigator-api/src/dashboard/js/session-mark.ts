// Keep in step with src/session_mark.rs — same glyphs, palettes, FNV-1a seed,
// and occupancy walk so live sessions do not share a drawing until all 30 are used.
export const sessionMarkJs = `
        const SESSION_MARK_GLYPHS = [
            '▀▄▀', '▛█▜', '◢█◣', '▐█▌', '░█░', '⣏⣉⣹', '⢸⣿⡇', '⣀⣾⣀', '⠶⣿⠶', '⣹⠶⣏',
            '╭◈╮', '⟨※⟩', '╱◆╲', '◖◆◗', '⊏◆⊐', '⌈✦⌉', '◎◆◎', '◕‿◕', 'ᵔᴥᵔ', 'ᓚᘏᓗ',
            '◉ω◉', '¬‿¬', 'ᚼᛉᚼ', 'ᛏᛏ', '╠╬╣', '≈△≈', '◆◇◆', '▰▱▰', '⌬⌬', '⍟⍟'
        ];
        const SESSION_MARK_PALETTES = [
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
            [[216, 224, 112], [26, 40, 8]]
        ];

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
