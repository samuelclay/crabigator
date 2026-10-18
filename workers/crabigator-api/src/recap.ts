import type { RecapEvent, SessionEvent } from './types/session';

/** Older desktops published these placeholders as if they were real recaps. */
export function hasRecapContent(recap: { headline?: unknown } | null | undefined): boolean {
    const headline = typeof recap?.headline === 'string' ? recap.headline.trim() : '';
    return headline.length > 0
        && headline !== 'The agent completed the turn without captured transcript output.'
        && headline !== 'The agent completed the turn. The recap response could not be parsed, so check the live output for details.';
}

export function cleanRecap(event: RecapEvent): RecapEvent {
    if (!event.latest || hasRecapContent(event.latest)) return event;
    return { ...event, latest: undefined, status: event.status === 'ready' ? 'waiting' : event.status };
}

export function cleanRecapEvent(event: SessionEvent): SessionEvent {
    if (event.type === 'recap') return cleanRecap(event);
    if (event.type === 'recap_history') return { ...event, history: event.history.filter(hasRecapContent) };
    return event;
}
