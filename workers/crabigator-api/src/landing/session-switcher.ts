// Two excerpts from Crabigator PRs #4 and #2. The board, terminal splits and
// phone details use the same session descriptions so each cut is easy to follow.
import { heroCapture, trainFlowHtml } from './desktop-capture';
import { iconClaude, iconCodex, iconCrabigator, iconKeyboard, iconMicrophone, iconPullRequest, iconWifi } from './icons';

const sessions = [
    { id: 'refresh', pr: 4, title: 'Refresh idle PRs', agent: 'Codex', icon: iconCodex, mark: '◆◇◆', branch: 'hourly-pr-refresh',
      prompt: 'Refresh idle PRs once an hour.', file: 'src/pr.rs', line: 1246,
      diff: ['- self.refresh_active_prs();', '+ self.refresh_open_prs();', '  self.retry_unenriched();'],
      summary: 'Idle pull requests now refresh hourly.', adds: 36, deletes: 24, flow: heroCapture.flowHtml },
    { id: 'stats', pr: 2, title: 'Fit stats in short windows', agent: 'Claude Code', icon: iconClaude, mark: 'ᛏᛏ', branch: 'compact-stats-grid',
      prompt: 'Keep stats visible in short windows.', file: 'src/ui/stats.rs', line: 21,
      diff: ['+ const CORE_STATS_ROWS: u16 = 7;', '+ const COMPACT_STATS_ROWS: u16 = 4;'],
      summary: 'Short windows use the compact grid.', adds: 234, deletes: 129, flow: trainFlowHtml },
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
    return `<button class="switch-row ${s.id}" type="button" data-open="${s.id}" disabled>
        <span>${mark(s)} <b>#${s.pr}</b> ${tally(s)}</span><strong>${s.title}</strong><small>${s.icon} ${s.agent} <i>›</i></small>
    </button>`;
}
function phoneDetail(s: Session) {
    return `<div class="switch-page switch-detail ${s.id}" data-page="${s.id}" inert>
        <div class="phone-app-bar"><button type="button" data-open="board" aria-label="Back to PR board">‹</button><div><strong>${s.title}</strong><span>ClayBook · Live</span></div></div>
        ${transcript(s)}
        <div class="switch-phone-pr">${mark(s)} ${iconPullRequest} #${s.pr} ${tally(s)}</div>
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
                <div class="switch-board-head">${iconCrabigator}<strong>PR board</strong></div>
                <div class="switch-machine">ClayBook <small>2 sessions</small></div>
                <div class="switch-project">samuelclay/crabigator</div>
                ${sessions.map(boardRow).join('')}
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
.switch-board { padding:0 4cqw; }
.session-switcher:not([data-view=board]) .switch-board { transform:translateX(-22%) scale(.96); opacity:.38; }
.switch-detail[inert] { transform:translateX(103%); pointer-events:none; }
.switch-board-head,.switch-machine { display:flex; align-items:center; justify-content:space-between; }
.switch-board-head { justify-content:start; gap:2cqw; padding:2cqw 0 5cqw; border-bottom:1px solid #303741; font-size:7cqw; }
.switch-board-head svg { width:10cqw; height:10cqw; }
.switch-machine { margin-top:6cqw; font-size:5.7cqw; font-weight:600; color:#93c3e7; }.switch-machine small { font-size:4.4cqw; color:#a6b7c1; }
.switch-project { color:#e6c58a; margin:3cqw 0; font:4.4cqw monospace; }
.switch-row { display:block; text-align:left; padding:3.7cqw; margin:1.3cqw 0; background:#1b222a; color:#ecf0f3; border:1px solid #333d48; border-left:2px solid var(--tone); border-radius:6px; font-size:5cqw; }
.switch-row:enabled:hover,.switch-row:focus-visible { background:#29313c; }
.switch-row > span,.switch-row small { display:flex; align-items:center; gap:1.5cqw; }.switch-row .switch-tally,.switch-row small i { margin-left:auto; }
.switch-row:disabled { cursor:default; }
.switch-row strong { display:block; margin:2cqw 0; font-weight:550; }
.switch-row small { color:#a8b6c3; font-size:4.6cqw; }.switch-row small svg { width:1em; height:1em; }
.switch-row small i { color:var(--tone); font:6cqw/1 sans-serif; }
.switch-detail .phone-app-bar button { padding:1cqw; border:0; background:transparent; color:#67b3f7; font:11cqw/1 sans-serif; }
.switch-detail .switch-transcript { flex:1; overflow:auto; background:#111518; font-size:4.5cqw; }
.switch-phone-pr { display:flex; align-items:center; gap:2cqw; padding:2.5cqw 4cqw; color:#c2b0e8; background:#20242c; font-size:4.5cqw; }
.switch-phone-pr svg { width:1em; height:1em; }.switch-phone-pr .switch-tally { margin-left:auto; }
.session-switcher[data-next=refresh] .switch-row.refresh,.session-switcher[data-next=stats] .switch-row.stats { animation:switch-row-open .65s ease-in-out both; }
@keyframes switch-row-open { 45% { transform:scale(.98); } to { background:var(--mark); border-color:var(--tone); } }
@media(max-width:700px) {
    .session-switcher { grid-template-columns:minmax(0,1fr); gap:18px; margin-top:30px; }
    .switch-phone { width:clamp(160px,52%,210px); justify-self:center; }
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
