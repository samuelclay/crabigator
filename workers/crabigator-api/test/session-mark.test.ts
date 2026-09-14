import { describe, expect, it } from 'vitest';
import {
    assignSessionMarks,
    sessionMarkClaim,
    sessionMarkFromSeed,
} from '../src/session-mark';

describe('session mark', () => {
    it('matches the desktop picker for a known seed', () => {
        const mark = sessionMarkFromSeed('crabigator-test');
        expect(mark.glyph).toBe('⊏◆⊐');
        expect(mark.bg).toEqual([122, 16, 36]);
        expect(mark.fg).toEqual([255, 210, 168]);
        expect(mark.bg_hex).toBe('#7a1024');
        expect(mark.fg_hex).toBe('#ffd2a8');
    });

    it('is stable for the same seed', () => {
        expect(sessionMarkFromSeed('crabigator-test')).toEqual(
            sessionMarkFromSeed('crabigator-test'),
        );
    });

    it('skips a taken preferred glyph', () => {
        const preferred = sessionMarkFromSeed('glyph-owner');
        const next = sessionMarkClaim('glyph-owner', [preferred]);
        expect(next.glyph).not.toBe(preferred.glyph);
    });

    it('assigns unique glyphs until the set is full', () => {
        const sessions = Array.from({ length: 8 }, (_, i) => ({
            client_session_id: `session-${i}`,
        }));
        const marks = assignSessionMarks(sessions);
        const glyphs = [...marks.values()].map((mark) => mark.glyph);
        expect(new Set(glyphs).size).toBe(glyphs.length);
    });
});
