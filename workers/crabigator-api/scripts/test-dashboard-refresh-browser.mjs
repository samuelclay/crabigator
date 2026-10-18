// Run with node scripts/test-dashboard-refresh-browser.mjs, then open the printed
// URL. Uses the shipping renderers and CSS in a real browser, without an account.
import { createServer } from 'node:http';
import { embedded, functions, terminalUpdates } from './dashboard-source.mjs';

const code = [embedded('ansi'), embedded('terminal'), embedded('scrollback'), terminalUpdates,
    functions(embedded('style'), ['getScrollAnchorReferenceY', 'capturePageScrollAnchor', 'restorePageScrollAnchor', 'capturePageScrollRestorer', 'preservePageScroll']),
    functions(embedded('pr-board'), ['reconcilePrBoard', 'preserveBoardTerminals', 'mountBoardTerminals', 'prbSessionState', 'prbAggregateState', 'prbStateIcon'])].join('\n');
const css = embedded('terminal', 'css') + embedded('pr-board', 'css');
const html = `<!doctype html><meta charset="utf-8"><title>Dashboard refresh checks</title>
<style>${css}
:root{--header-height:0px}*{box-sizing:border-box}body{background:#0b1218;color:#d0d7de;font:14px system-ui;margin:20px}
#results{white-space:pre-wrap}.pr-board{height:480px;padding:0;border:1px solid #444}.prb-body{display:block}
.prb-row{display:block;padding:10px}.session-card{background:#161b22}.terminal{height:180px;width:100%;font-size:12px}
#sessions{display:none}.line{line-height:20px}input{width:100%}
</style><h1>Dashboard refresh checks</h1><button id="run">Run checks</button><pre id="results">Ready</pre>
<div id="sessions"></div><div id="pr-board-view" class="pr-board"><div id="prb-body" class="prb-body"></div></div>
<script>
const sessions = new Map(); const expandedSessions = new Set(['test']); let prBoardRendered = []; let activeTerminalId = 'test', isChangingFontSize = false, prBoardVisible = true;
const SCROLLBACK_CHUNK_SIZE = 30; const PRB_C = {gray:"#888"};
function prbRowSessions(item) {return item.sessions;} function updatePinButton() {} function activateTerminalScroll() {}
${code}
const frame = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
const results = document.getElementById('results');
const check = (condition, message) => { if (!condition) throw Error(message); };
const near = (a,b) => Math.abs(a-b) < 1;
const row = (key, label) => '<div class="prb-row" data-key="'+key+'"><div class="prb-l1">'+label+'</div></div>';
async function run() {
    results.textContent = ''; const passed = [];
    const test = async (name, fn) => { await fn(); passed.push(name); results.textContent = passed.map(n=>'PASS '+n).join('\\n'); };
    const body = document.getElementById('prb-body'); body.innerHTML = '';
    const data = {pinned:false, lastScrollTop:0, scrollbackBuffer:[], scrollbackRendered:0}; sessions.set('test',data);
    reconcilePrBoard(body, row('a','Watching session') + row('b','Another session'));
    const card = document.createElement('div'); card.id='session-test';card.className='session-card';
    card.innerHTML='<div class="terminal scroll-active" tabindex="0" id="terminal-test"><div class="terminal-scrollback" id="scrollback-test"></div><div class="terminal-screen" id="screen-test"></div></div><input id="input-test" placeholder="Draft stays here">';
    body.firstElementChild.append(card);
    const terminal = document.getElementById('terminal-test'), history=document.getElementById('scrollback-test');
    data.element=card; const screen=document.getElementById('screen-test'); screen.innerHTML=ansiToHtml('live screen');
    bindTerminalScroll('test',terminal);
    let rows = Array.from({length:120}, (_,i)=>'history row '+i);
    renderScrollback('test', rows); loadMoreScrollback('test'); loadMoreScrollback('test'); loadMoreScrollback('test');
    terminal.scrollTop=1200;data.lastScrollTop=1200; await frame();
    let anchor=history.children[60], offset=()=>anchor.getBoundingClientRect().top-terminal.getBoundingClientRect().top;
    const originalOffset=offset();
    await test('Reconnect keeps loaded history and the exact visible row', async()=>{
        rows=rows.concat('new reconnect row');renderScrollback('test',rows);await frame();
        check(history.children.length===121,'Loaded history was discarded');
        check(history.children[60]===anchor && near(offset(),originalOffset),'Visible row moved on reconnect');
        rows=rows.slice(10);renderScrollback('test',rows);await frame();
        check(history.children[50]===anchor && near(offset(),originalOffset),'Server trimming moved the retained row');
    });
    await test('Output while inactive appears without moving the reading position', async()=>{
        activeTerminalId='other';appendScrollback('test','received while away\\n');activeTerminalId='test';flushScrollback('test');await frame();
        check(history.lastElementChild.textContent==='received while away','Missing buffered output');
        check(near(offset(),originalOffset),'Returning changed the reading position');
    });
    await test('History and screen share one paint and preserve an unpinned reader', async()=>{
        const before=history.children.length;
        scheduleTerminalUpdate('test',{type:'scrollback',diff:'batched history\\n'});
        scheduleTerminalUpdate('test',{type:'screen',content:'new screen'});
        check(history.children.length===before && screen.textContent==='live screen','Half a terminal painted early');
        await new Promise(r=>setTimeout(r,120)); await frame();
        check(history.lastElementChild.textContent==='batched history' && screen.textContent==='new screen','Batch did not paint');
        check(near(offset(),originalOffset),'Batch moved the reader');
    });
    await test('A one-pixel upward gesture wins against a queued output frame', async()=>{
        data.pinned=true;terminal.scrollTop=terminal.scrollHeight;data.lastScrollTop=terminal.scrollTop;
        scheduleTerminalUpdate('test',{type:'screen',content:'one\\ntwo\\nthree\\nfour'});
        terminal.dispatchEvent(new WheelEvent('wheel',{deltaY:-1}));terminal.scrollTop-=1;data.lastScrollTop=terminal.scrollTop;
        const top=terminal.scrollTop;await new Promise(r=>setTimeout(r,120));await frame();
        check(!data.pinned && near(top,terminal.scrollTop),'Small upward scroll was pulled back to the bottom');
    });
    for (const fallback of [false,true]) await test('Board refresh preserves terminal nodes and scroll'+(fallback?' without moveBefore':''),async()=>{
        if(fallback)body.moveBefore=undefined;
        terminal.scrollTop=700;data.lastScrollTop=700;data.pinned=false;
        preservePageScroll(()=>preserveBoardTerminals(()=>reconcilePrBoard(body,row('a','Watching session')+row('b','Another session'))));
        const existingRow=card.parentElement, draft=document.getElementById('input-test');draft.value='unfinished draft';
        const observed=[];const observer=new MutationObserver(changes=>observed.push(...changes));observer.observe(body,{childList:true,subtree:true});
        preservePageScroll(()=>preserveBoardTerminals(()=>reconcilePrBoard(body,row('a','Updated state')+row('b','Another session'))));await frame();
        check(card.parentElement===existingRow && terminal.scrollTop===700,'Metadata refresh reset terminal');
        check(!observed.some(m=>[...m.removedNodes].some(n=>n===card||n.contains?.(card))),'Metadata update detached an expanded terminal');
        observer.disconnect();
        preservePageScroll(()=>preserveBoardTerminals(()=>reconcilePrBoard(body,row('b','Another session')+row('a','Updated again'))));await frame();
        check(card.parentElement===existingRow && terminal.scrollTop===700,'Reordering reset terminal');
        check(draft.value==='unfinished draft','Draft was lost');
        const header='<div class="prb-bucket" data-board-key="bucket:0">Today</div>';
        reconcilePrBoard(body,header+row('b','Another session')+row('a','Updated again'));
        reconcilePrBoard(body,header+row('b','Another session')+row('a','Updated again'));
        check(body.firstElementChild.textContent==='Today','Heading was duplicated');
    });
    await test('Moving an expanded session to a different PR preserves its scroll and focus',async()=>{
        terminal.scrollTop=700;data.lastScrollTop=700;terminal.focus({preventScroll:true});
        prBoardRendered=[{key:'c',sessions:[{session_id:'test'}]}];
        preservePageScroll(()=>preserveBoardTerminals(()=>{
            reconcilePrBoard(body,row('c','New primary PR'));
            mountBoardTerminals();
        }));await frame();
        check(card.parentElement.dataset.key==='c' && terminal.scrollTop===700,'New PR mounting reset the terminal');
        check(document.activeElement===terminal,'Moving the session lost keyboard focus');
    });
    await test('Disconnected sessions show a quiet ended badge, not a working spinner', async()=>{
        const badge=document.createElement('div');body.appendChild(badge);
        badge.innerHTML=prbStateIcon(prbSessionState({active:false,state:'thinking'}));
        check(!badge.querySelector('.prb-throb') && badge.textContent==='○','Ended session still looks busy');
        check(badge.firstElementChild.title==='Session disconnected or ended','Status has no explanation');
    });
    results.textContent += '\\nAll '+passed.length+' browser checks passed.';
}
document.getElementById('run').onclick=()=>run().catch(e=>{results.textContent+='\\nFAIL '+e.stack;});
</script>`;
createServer((_req, res) => { res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }); res.end(html); })
    .listen(8799, '127.0.0.1', () => console.log('Open http://127.0.0.1:8799 and click Run checks.'));
