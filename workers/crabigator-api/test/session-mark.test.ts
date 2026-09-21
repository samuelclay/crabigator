import { describe, expect, it } from 'vitest';
import {
    assignSessionMarks,
    parseStoredSessionMark,
    sessionMarkClaim,
    sessionMarkFromSeed,
    sessionMarkJson,
    withSessionMark,
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

    it('keeps the mark stored on the session', () => {
        const stored = { glyph: '✶✶', fg: [9, 8, 7], bg: [6, 5, 4] };
        const sessions = [
            { client_session_id: 'local-mark-1', session_mark: stored },
            { client_session_id: 'other-session' },
        ];
        const marks = assignSessionMarks(sessions);
        expect(marks.get('local-mark-1')).toMatchObject(stored);
        expect(marks.get('other-session')?.glyph).not.toBe('✶✶');
        expect(withSessionMark(sessions[0], marks).session_mark?.glyph).toBe('✶✶');
    });

    it('round-trips a stored mark through JSON', () => {
        const json = sessionMarkJson({ glyph: '⊏◆⊐', fg: [255, 210, 168], bg: [122, 16, 36] });
        expect(parseStoredSessionMark(json)).toMatchObject({
            glyph: '⊏◆⊐',
            fg: [255, 210, 168],
            bg: [122, 16, 36],
            fg_hex: '#ffd2a8',
            bg_hex: '#7a1024',
        });
        expect(parseStoredSessionMark('{"glyph":"x"}')).toBeNull();
    });
});
