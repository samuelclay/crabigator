import { baseCss } from './css/base';
import { terminalCss } from './css/terminal';
import { ansiJs } from './js/ansi';
import { terminalJs } from './js/terminal';
import { scrollbackJs } from './js/scrollback';
import { flowJs } from './js/flow';

export const sharedSessionHtml = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Shared session · Crabigator</title><link rel="icon" href="/assets/logo.svg"><style>
${baseCss}
${terminalCss}
body { margin:0; padding:0; height:100vh; height:100dvh; display:flex; flex-direction:column; overflow:hidden; background:var(--bg-deep); color:var(--text-bright); font-family:var(--font-ui) }
[hidden] {display:none!important}
header,.tools,.drawer>summary,.module>summary,.entry {display:flex; align-items:center; gap:.65rem}
header {padding:12px clamp(12px,2vw,30px); border-bottom:1px solid var(--border-dim); flex:none}
#mark {flex:none; display:grid; place-items:center; height:36px; width:36px; border-radius:7px; font:24px monospace; background:var(--bg-surface)}
.identity {flex:1; min-width:0} h1 {font-size:clamp(15px,1.5vw,22px); font-weight:650; overflow:hidden; text-overflow:ellipsis; white-space:nowrap}
#context,.hint,#metrics {color:var(--text-mid); font-size:12px} #context {overflow:hidden; white-space:nowrap; text-overflow:ellipsis; margin-top:3px}
button, summary {cursor:pointer; color:inherit} button {background:transparent; border:1px solid var(--border-dim); border-radius:5px; font:inherit; padding:4px 9px} button:hover {border-color:var(--accent-cyan)}
#share {color:var(--accent-cyan); font:13px monospace}
#banner {flex:none; background:#303135; color:var(--accent-peach); border-bottom:1px solid #685344; padding:8px clamp(12px,2vw,30px); font-size:13px}
.tools {flex:none; padding:6px clamp(12px,2vw,30px); background:var(--bg-card); font-size:12px; flex-wrap:wrap}
.tools strong {margin-right:auto} .tools button {border-color:transparent} .tools button:hover,.tools button[aria-pressed="true"] {color:var(--accent-cyan); background:var(--bg-surface)}
.terminal.scroll-active {flex:1 1 0; min-height:0; overflow:auto; overscroll-behavior:contain; padding:12px clamp(12px,2vw,30px); border:0; font-size:13px}
.drawer {flex:0 1 auto; min-height:0; max-height:42vh; max-height:42dvh; display:flex; flex-direction:column; overflow:auto; background:var(--bg-card); border-top:1px solid var(--border-dim); overscroll-behavior:contain}
.drawer>summary {position:sticky; top:0; z-index:1; padding:9px clamp(12px,2vw,30px); background:var(--bg-card); font-size:13px; font-weight:600}
summary {list-style:none} summary::-webkit-details-marker {display:none} summary:before {content:'›'; color:var(--text-dim); font:18px/1 monospace} details[open]>summary:before {transform:rotate(90deg)}
#metrics {margin-left:auto; text-align:right} .drawer-inner {display:flex; align-items:flex-start; gap:12px; padding:0 clamp(12px,2vw,30px) 8px}
#modules {flex:1; min-width:0} .module {border-top:1px solid var(--border-dim); font-size:13px}
.module>summary {min-height:28px} .module .name {font-weight:600; flex:none; min-width:92px} .hint {overflow:hidden; text-overflow:ellipsis; white-space:nowrap}
.entries {padding:2px 0 8px 22px} .entry {align-items:baseline; padding:3px 0; overflow-wrap:anywhere; font-size:13px}
.entry small {color:var(--text-dim); white-space:nowrap; font:11px monospace}.entry a {color:var(--accent-cyan)}
#module-flow-view {width:clamp(70px,14vw,200px); height:155px; background:#0b1218; border-radius:4px} canvas {width:100%;height:100%;display:block}
@media(max-width:550px){header{padding:8px 12px}.tools{gap:2px}.tools button{padding:4px 6px}.module .name{min-width:82px}.drawer-inner{gap:8px}#metrics{font-size:11px}#module-flow-view{width:60px}}
</style></head><body>
<div id="banner" role="status" hidden></div>
<header><span id="mark" aria-hidden="true">◈</span><div class="identity"><h1>Shared session</h1><div id="context">Crabigator · Read-only</div></div>
<button id="share" title="Copy public link" aria-label="Copy public session link" hidden></button></header>
<div class="tools"><strong>Terminal</strong><span id="status" role="status">Connecting…</span>
<button id="smaller" aria-label="Decrease font size">A−</button><button id="larger" aria-label="Increase font size">A+</button>
<button id="wrap" aria-pressed="true">Wrap</button><button id="pin" aria-pressed="true">Follow</button></div>
<div class="terminal scroll-active" id="terminal-view" tabindex="0" role="region" aria-label="Terminal scrollback"><div class="terminal-scrollback" id="scrollback-view"></div><div class="terminal-screen" id="screen-view"></div></div>
<details class="drawer" open><summary>Session details <span id="metrics"></span></summary><div class="drawer-inner"><div id="modules"></div><div id="module-flow-view" hidden><canvas aria-hidden="true"></canvas></div></div></details>
<script src="/assets/shared-session.js"></script></body></html>`;

export const sharedSessionJs = [
    ansiJs, scrollbackJs, terminalJs, flowJs, `
const el = id => document.getElementById(id);
const activeTerminalId = 'view', SCROLLBACK_CHUNK_SIZE = 200;
const sessions = new Map([['view', { pinned: true, scrollbackBuffer: [], scrollbackRendered: 0 }]]);
const terminal = el('terminal-view');
let isChangingFontSize = false;
function activateTerminalScroll() { flushScrollback('view'); }
function updatePinButton(_, pinned) { el('pin').setAttribute('aria-pressed', pinned); }
bindTerminalScroll('view', terminal);
new ResizeObserver(() => {
    if (sessions.get('view').pinned) terminal.scrollTop = terminal.scrollHeight;
}).observe(terminal);
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

function node(tag, text, className) {
    const item = document.createElement(tag);
    if (text != null) item.textContent = text;
    if (className) item.className = className;
    return item;
}
function entry(text, metric, url) {
    const item = node('div', null, 'entry');
    if (metric) item.append(node('small', metric));
    const link = typeof url === 'string' && /^https?:\\/\\//i.test(url);
    const label = node(link ? 'a' : 'span', text);
    if (link) { label.href = url; label.target = '_blank'; label.rel = 'noopener noreferrer'; }
    item.append(label);
    return item;
}
// Replace a module's rows only when its contents change. The native details
// element keeps its disclosure state while the session continues streaming.
const sections = new Map();
function moduleRows(name, hint, rows) {
    let section = sections.get(name);
    if (!section) {
        const details = node('details', null, 'module'), summary = node('summary');
        const preview = node('span', null, 'hint'), body = node('div', null, 'entries');
        summary.append(node('span', name, 'name'), preview); details.append(summary, body);
        el('modules').append(details); section = { preview, body }; sections.set(name, section);
    }
    const signature = JSON.stringify(rows);
    section.preview.textContent = hint || '—';
    if (section.signature !== signature) {
        section.body.replaceChildren(...(rows.length ? rows.map(args => entry(...args)) : [entry(hint || 'No entries')]));
        section.signature = signature;
    }
}
function modules(d) {
    const recaps = [...(d.recaps || [])].reverse();
    if (d.recap && !recaps.some(r => r.generated_at === d.recap.generated_at)) recaps.unshift(d.recap);
    moduleRows('Recap', recaps[0]?.headline || 'No recaps', recaps.flatMap(r => [[r.headline, r.title], ...(r.bullets || []).map(b => [b, '•'])]));
    const prs = (d.prs || []).filter(p => !p.dismissed);
    moduleRows('Pull requests', prs[0]?.title || 'No pull requests', prs.map(p => [p.title, p.owner + '/' + p.repo + '#' + p.number + ' · ' + p.state, p.url]));
    const files = d.git?.files || [];
    moduleRows('Git', d.git ? [d.git.branch, files.length ? files.length + (files.length === 1 ? ' changed file' : ' changed files') : 'Clean'].filter(Boolean).join(' · ') : 'No Git snapshot', files.map(f => [f.path, [f.status, '+' + f.additions, '−' + f.deletions].join(' ')]));
    const changes = (d.changes?.by_language || []).flatMap(g => (g.changes || []).map(c => [c.name + (c.file_path ? ' · ' + c.file_path : ''), [g.language, c.kind, c.change_type].join(' · ')]));
    moduleRows('Changes', changes.length ? changes.length + (changes.length === 1 ? ' change' : ' changes') : 'No changes', changes);
    const commits = [...(d.commits || [])].reverse();
    moduleRows('Commits', commits[0]?.subject || 'No commits', commits.map(c => [c.subject, c.short_hash]));
    const titles = [...(d.titles || [])].reverse();
    moduleRows('Title history', titles[0] || 'No previous titles', titles.map(t => [t]));
    if (d.threads?.length || sections.has('Slack threads')) moduleRows('Slack threads', d.threads?.length || 'No threads', (d.threads || []).map(t => [t.text || t.url, t.channel, t.url]));
    const stats = d.stats;
    if (stats) {
        const counts = [['Prompts', stats.prompts], ['Completions', stats.completions], ['Tool calls', stats.tools],
            ['Thinking', stats.thinking_seconds == null ? null : stats.thinking_seconds + 's'], ['Model', stats.model],
            ['Context', stats.context_limit && stats.context_used != null ? stats.context_used.toLocaleString() + ' / ' + stats.context_limit.toLocaleString() : null]];
        moduleRows('Stats', stats.model, counts.filter(([,value]) => value != null).map(([label,value]) => [String(value), label]));
    }
    el('metrics').textContent = stats ? [stats.model, stats.prompts + ' prompts', stats.tools + ' tools'].filter(Boolean).join(' · ') : '';
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
        el('mark').textContent = d.mark.glyph; el('mark').style.color = d.mark.fg_hex; el('mark').style.backgroundColor = d.mark.bg_hex;
        el('context').textContent = ['Read-only', d.project, d.platform === 'codex' ? 'Codex' : d.platform].filter(Boolean).join(' · ');
        const title = d.title || d.titles?.at(-1) || d.project || 'Shared session';
        el('context').title = el('context').textContent; document.title = title + ' · Crabigator'; document.querySelector('h1').textContent = title;
        el('banner').hidden = !d.ended;
        if (d.ended) el('banner').textContent = 'Session ended' + (d.ended_at ? ' · ' + new Date(d.ended_at * 1000).toLocaleString() : '');
        el('status').textContent = d.ended ? 'Saved' : d.connected ? 'Live · ' + d.state : 'Reconnecting…';
        const h = d.history;
        updateHistory(h);
        const empty = cursor === 0 && d.ended;
        if (lastScreen !== d.screen || empty) {
            lastScreen = d.screen;
            preserveTerminalScroll('view', () => {
                el('screen-view').innerHTML = ansiToHtml(d.screen || (empty ? 'No terminal output was saved for this session.' : ''));
            });
        }
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
