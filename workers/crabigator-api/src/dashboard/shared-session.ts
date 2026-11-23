import { brandCrabSvg, brandLogoSvg } from './icons';
import { headerCss } from './css/header';
import { viewPanelCss } from './css/view-panel';
import { dashboardFonts } from './fonts';
import { modulesCss } from './css/modules';
import { widgetsCss } from './css/widgets';
import { recapCss } from './css/recap';
import { sessionCardCss } from './css/session-card';
import { modulesJs } from './js/modules';
import { changesWidgetJs } from './js/changes-widget';
import { gitWidgetJs } from './js/git-widget';
import { statsWidgetJs } from './js/stats-widget';
import { sessionMarkJs } from './js/session-mark';
import { modulePanelJs } from './js/module-panel';
import { metricFormatJs } from './js/utils';
import { sessionFormatJs } from './js/format';
import { spinnerJs } from './js/spinner';
import { baseCss } from './css/base';
import { terminalCss } from './css/terminal';
import { ansiJs } from './js/ansi';
import { terminalJs } from './js/terminal';
import { scrollbackJs } from './js/scrollback';
import { flowJs } from './js/flow';

export const sharedSessionHtml = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Shared session · Crabigator</title><link rel="icon" href="/assets/logo.svg">${dashboardFonts}<style>
${baseCss}
${terminalCss}
${headerCss}
${sessionCardCss}
${widgetsCss}
${recapCss}
${modulesCss}
${viewPanelCss}
@media (prefers-reduced-motion: no-preference) {
    @keyframes shared-session-enter { from { opacity: 0; transform: translateY(-8px); } to { opacity: 1; transform: none; } }
    .shared-dashboard .header, .shared-dashboard .session-header, #banner { animation: shared-session-enter .32s cubic-bezier(.2,1.3,.4,1) backwards; }
    .shared-dashboard .session-header { animation-delay: 45ms; }
    .shared-dashboard .terminal.has-output { animation: shared-session-enter .32s cubic-bezier(.22,1,.36,1); }
}
body.shared-dashboard {margin:0; padding:0; height:100vh; height:100dvh; display:flex; flex-direction:column; overflow:hidden; background:var(--bg-deep); color:var(--text-bright); font-family:var(--font-ui); -webkit-font-smoothing:antialiased}
[hidden] {display:none!important}
.shared-dashboard .header {position:static; flex:none}
.shared-dashboard .session-header {flex:none; border-radius:0; border-bottom:1px solid var(--border-dim)}
.shared-dashboard .session-info {max-height:18dvh; overflow:auto}
.shared-dashboard .session-context {display:flex}
.shared-dashboard .session-actions {align-self:center; flex-wrap:wrap}
.shared-dashboard .filter-chip[aria-pressed="true"] {border-color:var(--vp-accent); color:var(--vp-accent)}
.shared-dashboard .filter-chip:active {background:var(--vp-accent-tint)}
#share {font-family:'JetBrains Mono',monospace}
#banner {flex:none; background:#303135; color:var(--accent-peach); border-bottom:1px solid #685344; padding:6px 14px; font-size:12px}
#session-view {display:flex; flex:1; flex-direction:column; min-height:0}
.terminal.scroll-active {flex:1 1 0; min-height:0; overflow:auto; overscroll-behavior:contain; padding:12px clamp(12px,2vw,30px); border:0; font-size:13px}
#widgets-view {flex:none}
@media(max-width:720px){.shared-dashboard .session-header{flex-wrap:wrap}.shared-dashboard .session-info{flex:1 1 100%}.shared-dashboard .session-actions{gap:5px}}
</style></head><body class="container shared-dashboard">
<div id="banner" role="status" hidden></div>
<header class="header"><div class="header-main">
<h1><a class="dashboard-brand" href="/" aria-label="Crabigator home">${brandLogoSvg}${brandCrabSvg}</a><span class="dashboard-label">Shared session</span></h1>
<span class="status">Read-only</span><button class="view-btn" id="share" title="Copy public link" aria-label="Copy public session link" hidden></button>
</div></header>
<section class="session-header" aria-label="Session">
<div class="session-info" id="shared-heading"></div>
<div class="session-actions" role="toolbar" aria-label="Terminal display">
<button class="filter-chip" id="smaller" aria-label="Decrease font size">A−</button><button class="filter-chip" id="larger" aria-label="Increase font size">A+</button>
<button class="filter-chip" id="wrap" aria-pressed="true">Wrap</button><button class="filter-chip" id="pin" aria-pressed="true">Follow</button><span class="state" id="status" role="status">Connecting…</span></div>
</section>
<main id="session-view" aria-label="Session terminal and details">
<div class="terminal scroll-active" id="terminal-view" tabindex="0" role="region" aria-label="Terminal scrollback"><div class="terminal-scrollback" id="scrollback-view"></div><div class="terminal-screen" id="screen-view"></div></div>
<div id="recap-prs-view" class="session-prs" style="display:none"></div>
<div id="drawer-mount"></div></main>
<script src="/assets/shared-session.js"></script></body></html>`;

export const sharedSessionJs = [
    ansiJs, scrollbackJs, terminalJs, flowJs, sessionMarkJs, metricFormatJs, sessionFormatJs,
    modulePanelJs, modulesJs, spinnerJs, gitWidgetJs, statsWidgetJs, changesWidgetJs, `
const el = id => document.getElementById(id);
const activeTerminalId = 'view', SCROLLBACK_CHUNK_SIZE = 200;
const view = { id: 'view', element: el('session-view'), readOnly: true, pinned: true, scrollbackBuffer: [], scrollbackRendered: 0 };
const sessions = new Map([['view', view]]);
const visibleSections = { recap: true, prs: true, git: true, changes: true, commits: true };
el('shared-heading').innerHTML = sessionHeadingContentHtml('view');
el('drawer-mount').outerHTML = modulePanelHtml('view');
function toggleWidgets(id) {
    view.element.classList.toggle('widgets-collapsed');
    syncModuleDrawer(id);
}
initSessionModules('view');
const terminal = el('terminal-view');
let isChangingFontSize = false;
function activateTerminalScroll() { flushScrollback('view'); }
function updatePinButton(_, pinned) { el('pin').setAttribute('aria-pressed', pinned); }
bindTerminalScroll('view', terminal);
el('pin').onclick = () => {
    const session = sessions.get('view');
    session.pinned = !session.pinned;
    updatePinButton('view', session.pinned);
    if (session.pinned) terminal.scrollTop = terminal.scrollHeight;
};
el('wrap').onclick = () => preserveTerminalScroll('view', () => {
    el('wrap').setAttribute('aria-pressed', !terminal.classList.toggle('scroll-mode'));
});
for (const [id, step] of [['smaller', -1], ['larger', 1]]) el(id).onclick = () => {
    preserveTerminalScroll('view', () => { terminal.style.fontSize = Math.max(9, Math.min(28, parseFloat(getComputedStyle(terminal).fontSize) + step)) + 'px'; });
};
let shortId = '';
el('share').onclick = async () => {
    try { await navigator.clipboard.writeText(location.href); el('share').textContent = 'Copied'; }
    catch { el('share').textContent = 'Copy address bar URL'; }
    setTimeout(() => { el('share').textContent = shortId; }, 1800);
};

let moduleSnapshot;
function modules(d) {
    const next = { title: d.title, generatedTitle: d.title, state: d.state, endedAt: d.ended_at,
        cwd: d.project, session_mark: d.mark, git: d.git, changes: d.changes, stats: d.stats,
        prs: d.prs || [], recap: { latest: d.recap }, recapHistory: d.recaps || [],
        commitHistory: d.commits || [], slackThreads: d.threads || [], titles: d.titles || [] };
    const signature = JSON.stringify(next);
    if (signature === moduleSnapshot) return;
    Object.assign(view, next);
    updateTitlesWidget('view', view.titles);
    if (view.git) updateGitWidget('view', view.git);
    updateChangesWidget('view', view.changes);
    updateStatsWidget('view', view.stats || {});
    renderPrList('view', view.prs);
    refreshSessionModules('view');
    moduleSnapshot = signature;
}

let generation = '', cursor = 0, pendingHistory = '', needsHistoryReset = true;
let partialLine = '', lastScreen, lastFlow;
function updateHistory(h) {
    if (generation !== h.generation || cursor !== h.after) {
        pendingHistory = ''; partialLine = ''; needsHistoryReset = true;
    }
    cursor = h.cursor; generation = h.generation;
    pendingHistory += h.content;
    if (h.more || (!needsHistoryReset && !pendingHistory)) return;
    const data = sessions.get('view');
    if (needsHistoryReset || partialLine) {
        const previous = needsHistoryReset ? [] : data.scrollbackBuffer.slice(0, -1);
        renderScrollback('view', previous.concat(scrollbackLines(partialLine + pendingHistory)));
    } else if (pendingHistory) {
        appendScrollback('view', pendingHistory);
    }
    if (pendingHistory) {
        const lastBreak = pendingHistory.lastIndexOf('\\n');
        partialLine = lastBreak < 0 ? partialLine + pendingHistory : pendingHistory.slice(lastBreak + 1);
    }
    needsHistoryReset = false; pendingHistory = '';
    // Readers following the newest output can load earlier rows on demand.
    if (data.pinned && data.scrollbackRendered > SCROLLBACK_CHUNK_SIZE * 4) {
        preserveTerminalScroll('view', () => {
            removeScrollbackRows(el('scrollback-view'), data.scrollbackRendered - SCROLLBACK_CHUNK_SIZE);
            data.scrollbackRendered = SCROLLBACK_CHUNK_SIZE;
        });
    }
}
async function refresh() {
    let delay = 2000;
    try {
        const url = new URL(location.pathname + '/data', location.origin);
        url.searchParams.set('generation', generation); url.searchParams.set('cursor', cursor);
        const response = await fetch(url, { credentials:'omit' });
        if (response.status === 404) { el('status').textContent = 'Link unavailable'; return; }
        if (!response.ok) throw new Error();
        const d = await response.json();
        shortId = d.id;
        if (el('share').hidden) { el('share').textContent = shortId; el('share').hidden = false; }
        const heading = el('shared-heading');
        heading.querySelector('.row-computer').textContent = d.platform === 'codex' ? 'Codex' : d.platform;
        heading.querySelector('.cwd').textContent = d.project;
        document.title = (d.title || d.titles?.at(-1) || d.project || 'Shared session') + ' · Crabigator';
        el('banner').hidden = !d.ended;
        if (d.ended) el('banner').textContent = 'Session ended' + (d.ended_at ? ' · ' + new Date(d.ended_at * 1000).toLocaleString() : '');
        el('status').className = 'state ' + (d.ended ? 'complete' : d.state);
        el('status').textContent = d.ended ? 'Ended' : d.connected ? d.state : 'Reconnecting…';
        const h = d.history;
        updateHistory(h);
        const empty = cursor === 0 && d.ended;
        if (lastScreen !== d.screen || empty) {
            lastScreen = d.screen;
            preserveTerminalScroll('view', () => {
                el('screen-view').innerHTML = ansiToHtml(d.screen || (empty ? 'No terminal output was saved for this session.' : ''));
            });
        }
        if (view.scrollbackRendered || d.screen || empty) terminal.classList.add('has-output');
        modules(d);
        const flow = JSON.stringify(d.flow);
        if (flow !== lastFlow) { applyFlowEvent('view', d.flow); lastFlow = flow; }
        if (h.more) delay = 0;
        else if (d.ended && !d.connected) return;
    } catch { el('status').textContent = 'Reconnecting…'; delay = 5000; }
    setTimeout(refresh, document.hidden ? Math.max(delay, 15000) : delay);
}
refresh();
`].join('\n');
