// The native apps' session modules: the open drawer (Recap, Git, Changes)
// beside a flow scene, then the summary bar. Matches SessionModules.swift and
// SessionSummaryBar on iOS. The hero passes a captured session's real values.

export interface SessionValues {
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
    /** A session waiting on a question shows the app's filled "?" badge. */
    asking?: boolean;
    /** Wide layouts (iPad) show these too, as the app does when they fit. */
    wide?: { completions: number; compactions: number; age: string; additions: number; deletions: number };
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

export function contextLabel({ used, limit }: SessionValues['context']): string {
    return `${tokens(used)} / ${tokens(limit)} · ${Math.round(used * 100 / limit)}%`;
}

export function markChip(mark: SessionValues['mark'], className: string): string {
    return `<span class="${className}" style="color:${mark.fg};background:${mark.bg}">${mark.glyph}</span>`;
}

export function appModulesHtml(session: SessionValues, flowSvg: string): string {
    const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;
    const wide = session.wide;
    const prompts = wide
        ? `<span><span class="c-orange">⟩</span><span class="c-title">${session.prompts}</span><span class="c-orange">${wide.age}</span></span>`
        : `<span class="c-orange">⟩ ${session.prompts}</span>`;
    const extra = wide ? `
            <span><span class="c-deep">⋖</span><span class="c-title">${wide.completions}</span><span class="c-deep">${wide.age}</span></span>` : '';
    const after = wide ? `
            <span class="c-purple">⊜ ${wide.compactions}</span>` : '';
    const preview = wide ? `
            <span class="app-preview">${plural(session.files, 'file')} · <span class="c-mint">+${wide.additions}</span> <span class="c-red">−${wide.deletions}</span></span>` : '';
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
            ${session.asking ? '<span class="app-ask">?</span>' : '<span class="app-done">✓</span>'}
            <span class="c-blue">◉ ${duration(session.workSeconds)}</span>
            <span class="c-mint">◐ ${duration(session.thinkingSeconds)}</span>
            ${prompts}${extra}
            <span class="c-orange">⚒ ${session.tools}</span>${after}
            <span><span class="c-title">◔</span><span class="c-title">${contextLabel(session.context)}</span></span>${preview}
            <span class="app-open">⌄</span>
        </div>
    </div>`;
}
