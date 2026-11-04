// The native apps' session modules, drawn from a captured session's real values:
// the open drawer (Recap, Git, Changes) beside its flow scene, then the summary
// bar. Matches SessionModules.swift and SessionSummaryBar on iOS.

export interface CapturedSession {
    mark: { glyph: string; fg: string; bg: string };
    workSeconds: number;
    thinkingSeconds: number;
    prompts: number;
    tools: number;
    context: { used: number; limit: number };
    branch: string;
    files: number;
    symbols: number;
    recap: string;
}

/** A token count as every client shows it: 512, 9.5k, 186k, 1M. */
function tokens(count: number): string {
    if (count < 1000) return String(count);
    if (count < 9950) return (count / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
    const thousands = Math.round(count / 1000);
    return thousands < 1000 ? thousands + 'k' : (count / 1e6).toFixed(1).replace(/\.0$/, '') + 'M';
}

function duration(seconds: number): string {
    if (seconds < 60) return seconds + 's';
    if (seconds < 3600) return Math.floor(seconds / 60) + 'm';
    return Math.floor(seconds / 3600) + 'h ' + Math.floor(seconds % 3600 / 60) + 'm';
}

export function contextLabel({ used, limit }: CapturedSession['context']): string {
    return `${tokens(used)} / ${tokens(limit)} · ${Math.round(used * 100 / limit)}%`;
}

export function markChip(mark: CapturedSession['mark'], className: string): string {
    return `<span class="${className}" style="color:${mark.fg};background:${mark.bg}">${mark.glyph}</span>`;
}

export function appModulesHtml(session: CapturedSession, flowSvg: string): string {
    const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;
    return `
    <div class="app-modules">
        <div class="app-drawer">
            <div class="app-module-rows">
                <div class="app-module recap"><i>›</i><b>Recap</b><span>${session.recap}</span><small>now</small></div>
                <div class="app-module git"><i>›</i><b>Git</b><span>${session.branch}</span><small>${plural(session.files, 'file')}</small></div>
                ${session.symbols ? `<div class="app-module changes"><i>›</i><b>Changes</b><span>${plural(session.symbols, 'symbol')}</span></div>` : ''}
            </div>
            <div class="app-flow">${flowSvg}</div>
        </div>
        <div class="app-summary">
            ${markChip(session.mark, 'app-chip')}
            <span class="app-done">✓</span>
            <span class="c-blue">◉ ${duration(session.workSeconds)}</span>
            <span class="c-mint">◐ ${duration(session.thinkingSeconds)}</span>
            <span class="c-orange">⟩ ${session.prompts}</span>
            <span class="c-orange">⚒ ${session.tools}</span>
            <span><span class="c-title">◔</span><span class="c-title">${contextLabel(session.context)}</span></span>
            <span class="app-open">⌄</span>
        </div>
    </div>`;
}
