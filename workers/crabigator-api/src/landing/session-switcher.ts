// Two excerpts from Crabigator PRs #4 and #2. The board, terminal splits and
// phone details use the same session descriptions so each cut is easy to follow.
import { heroCapture, trainFlowHtml } from './desktop-capture';
import { iconClaude, iconCodex, iconCrabigator, iconKeyboard, iconMicrophone, iconSearch, iconDesktop, iconLines, iconWifi } from './icons';
import { iconPerson, iconSliders } from '../dashboard/icons';
import { appModulesHtml } from './app-modules';

const sessions = [
    { id: 'refresh', pr: 4, title: 'Refresh idle PRs', agent: 'Codex', icon: iconCodex, mark: '◆◇◆', branch: 'hourly-pr-refresh',
      prompt: 'Refresh idle PRs once an hour.', file: 'src/pr.rs', line: 1246,
      diff: ['- self.refresh_active_prs();', '+ self.refresh_open_prs();', '  self.retry_unenriched();'],
      summary: 'Idle pull requests now refresh hourly.', prTitle: 'Refresh idle pull requests hourly', files: 1, adds: 36, deletes: 24, flow: heroCapture.flowHtml },
    { id: 'stats', pr: 2, title: 'Fit stats in short windows', agent: 'Claude Code', icon: iconClaude, mark: 'ᛏᛏ', branch: 'compact-stats-grid',
      prompt: 'Keep stats visible in short windows.', file: 'src/ui/stats.rs', line: 21,
      diff: ['+ const CORE_STATS_ROWS: u16 = 7;', '+ const COMPACT_STATS_ROWS: u16 = 4;'],
      summary: 'Short windows use the compact grid.', prTitle: 'Keep all stats visible in short windows', files: 3, adds: 234, deletes: 129, flow: trainFlowHtml },
];
type Session = typeof sessions[number];

const mark = (s: Session) => `<span class="switch-mark">${s.mark}</span>`;
const tally = (s: Session) => `<span class="switch-tally"><b>+${s.adds}</b> <i>−${s.deletes}</i></span>`;
function transcript(s: Session) {
    return `<div class="switch-transcript">
        <p class="switch-prompt">${s.id === 'refresh' ? '›' : '❯'} ${s.prompt}</p>
        <p class="switch-file">${s.id === 'refresh' ? '• Edited' : '● Update'} <b>${s.file}</b></p>
        <div class="switch-patch">${s.diff.map((line, i) => `<div class="${line.startsWith('+') ? 'added' : line.startsWith('-') ? 'removed' : ''}"><small>${s.line + i}</small><span>${line}</span></div>`).join('')}</div>
        <p class="switch-result">● ${s.summary}</p>
    </div>`;
}
function desktopPane(s: Session) {
    return `<div class="switch-pane ${s.id}">
        <div class="switch-agent">${s.icon} ${s.agent}</div>
        ${transcript(s)}
        <div class="switch-footer"><div>${mark(s)} <b>#${s.pr}</b> ${s.title}<small>${s.branch} · ${tally(s)}</small></div><div class="switch-flow">${s.flow}</div></div>
    </div>`;
}
function boardRow(s: Session) {
    return `<button class="switch-row ${s.id}" type="button" data-open="${s.id}" aria-label="Open ${s.title}, PR ${s.pr}" disabled>
        ${mark(s)}
        <span class="switch-row-body">
            <strong>${s.title}</strong>
            <span class="switch-recap">${s.summary}</span>
            <span class="switch-branch">${s.branch}<span>✓</span></span>
            <span class="switch-pr-title">#${s.pr} &nbsp;${s.prTitle}</span>
            <span class="switch-pr-state"><span>Merged</span> ${tally(s)}</span>
        </span>
    </button>`;
}
function phoneDetail(s: Session) {
    return `<div class="switch-page switch-detail ${s.id}" data-page="${s.id}" inert>
        <div class="phone-app-bar"><button type="button" data-open="board" aria-label="Back to PR board">‹</button><div><strong>${s.title}</strong><span>ClayBook · Live</span></div><span class="phone-type">ᴀA</span></div>
        ${transcript(s)}
        ${appModulesHtml({
            mark: { glyph: s.mark, fg: 'var(--tone)', bg: 'var(--mark)' },
            recap: s.summary, prs: { count: 1, summary: 'merged' }, branch: s.branch, files: s.files, symbols: 0,
            thinkingSeconds: s.id === 'refresh' ? 19 : 36, prompts: 1, context: { used: 34000, limit: 258000 },
        }, s.flow)}
        <div class="phone-composer" aria-hidden="true">${iconMicrophone}${iconKeyboard}<span>Message</span></div>
    </div>`;
}
export const sessionSwitcherHtml = `
<div class="session-switcher" data-view="board" role="group" aria-label="Two desktop sessions. Open either one from the phone’s PR board.">
    <div class="desktop-frame switch-desktop">
        <div class="desktop-bar"><span class="window-dots"><i></i><i></i><i></i></span><span>crabigator — split panes</span></div>
        <div class="switch-panes">${sessions.map(desktopPane).join('')}</div>
    </div>
    <div class="phone-frame switch-phone"><div class="phone-screen">
        <div class="phone-status" aria-hidden="true"><span>9:41</span><span class="phone-island"></span><span class="phone-indicators">${iconWifi}<span class="phone-battery"></span></span></div>
        <div class="switch-stack">
            <div class="switch-page switch-board" data-page="board">
                <div class="switch-board-head" aria-hidden="true">${iconCrabigator}<span>＋</span>${iconSearch}${iconPerson}</div>
                <div class="switch-filters" aria-hidden="true"><span>24h ⌄</span><span>Live ⌄</span><span>All computers ⌄</span><b>${iconSliders} View</b></div>
                <div class="switch-roster">
                    <div class="switch-machine">${iconDesktop} ClayBook <small>2 sessions ﹀</small></div>
                    <div class="switch-age">Last hour</div>
                    <div class="switch-project">samuelclay/crabigator</div>
                    ${sessions.map(boardRow).join('')}
                </div>
                <div class="switch-modes" aria-hidden="true"><span>${iconLines} List</span><b>▥ PR board</b><span>∴ Conductor</span></div>
            </div>
            ${sessions.map(phoneDetail).join('')}
        </div>
        <div class="phone-home"><span></span></div>
    </div></div>
</div>`;

export const sessionSwitcherCss = `
.session-switcher { display:grid; grid-template-columns:minmax(0,1fr) minmax(0,24%); gap:3%; align-items:center; margin-top:48px; text-align:left; }
.switch-panes { display:grid; grid-template-columns:1fr 1fr; min-height:38cqw; }
.session-switcher .refresh { --tone:#efba86; --mark:#55351e; }
.session-switcher .stats { --tone:#c4adff; --mark:#372952; }
.switch-pane { display:flex; flex-direction:column; min-width:0; background:#131518; box-shadow:inset 0 2px transparent; transition:background .5s,box-shadow .5s; }
.switch-pane + .switch-pane { border-left:1px solid #5b6269; }
.session-switcher[data-view=refresh] .switch-pane.refresh,.session-switcher[data-view=stats] .switch-pane.stats { background:#1a1b1e; box-shadow:inset 0 2px var(--tone); }
.switch-agent { display:flex; align-items:center; gap:.6em; color:var(--tone); font-size:clamp(10px,1.8cqw,16px); font-weight:600; padding:1em 1em .6em; }
.switch-agent svg { width:1.2em; height:1.2em; }
.switch-transcript { color:#e3e3de; font:clamp(10px,1.65cqw,15px)/1.55 'JetBrains Mono',monospace; padding:.6em .85em 1em; overflow-wrap:anywhere; }
.switch-prompt { padding:.25em .4em; background:#343535; }
.switch-file { margin:1.3em 0 .45em; color:#a9b2b6; }
.switch-file b { color:#e2e3dd; font-weight:500; }
.switch-patch { font-size:.91em; }
.switch-patch > div { display:flex; gap:.5em; padding:.14em .35em; }
.switch-patch span { min-width:0; }
.switch-patch small { flex:none; color:#a0aaa6; font-size:.85em; user-select:none; }
.switch-patch .added { background:#20362b; color:#ace5b9; }
.switch-patch .removed { background:#3c292c; color:#f2ada8; }
.switch-result { margin-top:1em; }
.switch-footer { display:flex; align-items:stretch; margin-top:auto; border-top:1px solid #414547; background:#1c2024; font:clamp(9px,1.46cqw,14px)/1.45 'JetBrains Mono',monospace; }
.switch-footer > div:first-child { flex:1; min-width:0; padding:.6em; }
.switch-footer small { display:block; margin-top:.35em; color:#99a6af; overflow-wrap:anywhere; }
.switch-mark { display:inline-block; padding:0 .25em; background:var(--mark); color:var(--tone); border-radius:3px; font:600 .95em/1.5 monospace; }
.switch-flow { position:relative; align-self:stretch; width:14%; overflow:hidden; }
.switch-flow > * { position:absolute; inset:0; width:100%; height:100%; }
.switch-tally { white-space:nowrap; font:inherit; }
.switch-tally b,.switch-tally i { font-weight:500; font-style:normal; }.switch-tally b { color:#9dddac; }.switch-tally i { color:#eba4a4; }
.switch-phone { width:100%; }
.switch-stack { position:relative; flex:1; min-height:0; overflow:hidden; }
.switch-page { position:absolute; inset:0; display:flex; flex-direction:column; background:#0d131b; transition:transform .6s cubic-bezier(.22,1,.36,1),opacity .45s; }
.switch-board { padding:0 3cqw; background:#101419; color:#e6edf3; font-family:-apple-system,BlinkMacSystemFont,'DM Sans',sans-serif; }
.session-switcher:not([data-view=board]) .switch-board { transform:translateX(-22%); opacity:.38; }
.switch-detail[inert] { transform:translateX(103%); pointer-events:none; }
.switch-board-head,.switch-filters,.switch-machine,.switch-modes { display:flex; align-items:center; flex:none; }
.switch-board-head { padding:1cqw 1.6cqw 3cqw; gap:4cqw; color:#a4aab0; }
.switch-board-head svg { width:5.2cqw; height:5.2cqw; fill:currentColor; }
.switch-board-head svg:first-child { width:9cqw; height:9cqw; margin-right:auto; }
.switch-board-head > span { font:7cqw/1 sans-serif; }
.switch-filters { gap:1cqw; font-size:3.4cqw; white-space:nowrap; }
.switch-filters > * { padding:1.3cqw 2cqw; background:#1b2129; border-radius:3cqw; }
.switch-filters b { margin-left:auto; }
.switch-filters svg,.switch-modes svg { width:1em; height:1em; }
.switch-roster { flex:1; min-height:0; overflow:auto; overscroll-behavior:contain; }
.switch-machine { gap:2cqw; padding:4cqw 2cqw 2.8cqw; color:#ffda49; font-size:4.3cqw; font-weight:550; }
.switch-machine svg { width:1.15em; height:1.15em; fill:currentColor; }
.switch-machine small { margin-left:auto; font-size:3.5cqw; color:#a0a5ac; }
.switch-age { padding:1.1cqw 2cqw; color:#70dedf; background:#103035; font-size:3.65cqw; }
.switch-project { color:#ffda49; margin:2cqw; font-size:3.7cqw; }
.switch-row { appearance:none; display:grid; grid-template-columns:10cqw minmax(0,1fr); gap:2.5cqw; width:100%; padding:2.2cqw 2cqw; text-align:left; background:transparent; border:0; border-radius:0; font:inherit; }
.switch-row + .switch-row { margin-top:1.8cqw; }
.switch-row:enabled:hover,.switch-row:focus-visible { background:#262b2d; }
.switch-row:focus-visible { outline-offset:-2px; }
.switch-row:disabled { cursor:default; }
.switch-row .switch-mark { display:grid; place-items:center; align-self:start; height:6.6cqw; font-size:3.7cqw; }
.switch-row-body { display:grid; gap:.95cqw; }
.switch-row strong { color:#5fafff; font-size:4.75cqw; font-weight:550; line-height:1.3; }
.switch-recap { color:#829dff; font-size:3.85cqw; line-height:1.3; }
.switch-branch,.switch-pr-state { display:flex; align-items:center; gap:2cqw; font-size:3.6cqw; }
.switch-branch { justify-content:space-between; color:#a5a7ab; }
.switch-branch > span,.switch-pr-title { color:#af87ff; }
.switch-pr-title { margin-top:.8cqw; font-size:4.1cqw; line-height:1.3; }
.switch-pr-state > span:first-child { color:#c4a4ff; background:#261e35; padding:0 1cqw; }
.switch-modes { align-self:center; margin:2cqw 0; padding:1cqw; gap:.6cqw; color:#afb2b4; background:#24282c; border-radius:6cqw; font-size:3.6cqw; }
.switch-modes > * { padding:1.2cqw 2cqw; }
.switch-modes b { background:#443b29; color:#d7af5f; border-radius:4cqw; font-weight:550; }
.switch-detail .phone-app-bar button { padding:1cqw; border:0; background:transparent; color:#67b3f7; font:11cqw/1 sans-serif; }
.switch-detail .switch-transcript { flex:1; overflow:auto; background:#111518; font-size:4.5cqw; }
.session-switcher[data-next=refresh] .switch-row.refresh,.session-switcher[data-next=stats] .switch-row.stats { animation:switch-row-open .65s ease-in-out both; }
@keyframes switch-row-open { from { background:transparent; } to { background:#d7af5f25; } }
@media(max-width:700px) {
    .session-switcher { grid-template-columns:minmax(0,1fr); gap:18px; margin-top:30px; }
    .switch-phone { width:clamp(205px,68%,260px); justify-self:center; }
    .switch-footer .switch-flow { display:none; }
}
@media(prefers-reduced-motion:reduce) { .switch-page,.switch-pane { transition:none; } }
`;

export const sessionSwitcherJs = `
(() => {
    const demo = document.querySelector('.session-switcher');
    if (!demo) return;
    const motion = matchMedia('(prefers-reduced-motion: reduce)');
    const pages = Array.from(demo.querySelectorAll('[data-page]'));
    const order = ['board','refresh','board','stats'];
    let step = 0, timer, visible = false;
    function schedule() {
        clearTimeout(timer);
        delete demo.dataset.next;
        if (visible && !document.hidden && !motion.matches && !demo.contains(document.activeElement)) {
            const next = (step + 1) % order.length;
            if (order[step] === 'board') {
                timer = setTimeout(() => {
                    demo.dataset.next = order[next];
                    timer = setTimeout(() => show(next), 650);
                }, 2850);
            } else {
                timer = setTimeout(() => show(next), 5000);
            }
        }
    }
    function show(next, manual = false) {
        const from = order[step], to = order[next];
        step = next;
        demo.dataset.view = to;
        pages.forEach(page => page.inert = page.dataset.page !== to);
        if (manual) {
            const selector = to === 'board' ? '[data-open="' + from + '"]' : '[data-page="' + to + '"] [data-open]';
            demo.querySelector(selector).focus({ preventScroll:true });
        }
        schedule();
    }
    demo.addEventListener('click', event => {
        const target = event.target.closest('[data-open]')?.dataset.open;
        if (target) show(target === 'board' ? (step + 1) % order.length : order.indexOf(target), true);
    });
    demo.querySelectorAll('.switch-row').forEach(button => button.disabled = false);
    demo.addEventListener('focusin', schedule);
    demo.addEventListener('focusout', () => queueMicrotask(schedule));
    document.addEventListener('visibilitychange', schedule);
    motion.addEventListener('change', schedule);
    new IntersectionObserver(([entry]) => {
        visible = entry.isIntersecting && entry.intersectionRatio >= .45;
        schedule();
    }, { threshold:.45 }).observe(demo.querySelector('.switch-phone'));
})();
`;
