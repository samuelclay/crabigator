// Run with node scripts/test-dashboard-refresh-browser.mjs, then open the printed
// URL. Uses the shipping renderers and CSS in a real browser, without an account.
import { createServer } from 'node:http';
import { embedded, functions, terminalUpdates } from './dashboard-source.mjs';

const code = [embedded('ansi'), embedded('terminal'), embedded('scrollback'), terminalUpdates,
    functions(embedded('style'), ['pageScrollSurface', 'getScrollAnchorReferenceY', 'capturePageScrollAnchor', 'restorePageScrollAnchor', 'capturePageScrollRestorer', 'preservePageScroll']),
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
let viewedPageAnchor=null, viewedSessionId=null, pageScrollRevision=0, restoredPageTop=null;
const sessions = new Map(); const expandedSessions = new Set(['test']); let prBoardRendered = []; let activeTerminalId = 'test', isChangingFontSize = false, prBoardVisible = true;
const SCROLLBACK_CHUNK_SIZE = 30; const PRB_C = {gray:"#888"};
function restoreSessionFullscreen() {}
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
    await test('Paragraphs reflow with two-cell indentation at every viewport width', async()=>{
        const first='  This paragraph describes the updated reading experience';
        const continuation='  continuing with details on the following row and additional words to wrap.';
        const source=first+'\\n'+continuation+'\\n'+'─'.repeat(first.length+2);
        const sample=document.createElement('div');sample.className='terminal';sample.style.height='auto';
        sample.innerHTML=ansiToHtml(source);document.body.appendChild(sample);
        const paragraph=sample.querySelector('.terminal-paragraph');
        check(paragraph && paragraph.children.length===2,'Expected two retained source rows');
        const originalText=sample.textContent;
        const selection=window.getSelection();selection.selectAllChildren(paragraph);
        check(selection.toString().includes('experience continuing'),'Copying lost the joined word boundary');selection.removeAllRanges();
        let savedSpace=false;
        for(const width of [240,320,500,740,240]) {
            sample.style.width=width+'px';await frame();
            const starts=new Map(),walker=document.createTreeWalker(paragraph,NodeFilter.SHOW_TEXT);
            while(walker.nextNode()) {
                const node=walker.currentNode;
                if(node.parentElement.closest('.reflow-indent'))continue;
                for(let i=0;i<node.length;i++) {
                    if(node.data[i]===' ')continue;
                    const range=document.createRange();range.setStart(node,i);range.setEnd(node,i+1);
                    const rect=range.getBoundingClientRect();
                    starts.set(rect.top,Math.min(starts.get(rect.top)??Infinity,rect.left));
                }
            }
            const indent=parseFloat(getComputedStyle(paragraph).paddingLeft),left=paragraph.getBoundingClientRect().left;
            check(indent>0 && starts.size>1,'Paragraph did not wrap with indentation');
            for(const x of starts.values())check(near(x,left+indent),'Wrapped prose lost its two-space indent');
            const before=document.createElement('div');before.className='terminal';before.style.cssText=sample.style.cssText;
            before.innerHTML=ansiToHtml(source);document.body.appendChild(before);
            const oldParagraph=before.querySelector('.terminal-paragraph');oldParagraph.replaceWith(...oldParagraph.children);
            savedSpace ||= sample.getBoundingClientRect().height<before.getBoundingClientRect().height;
            before.remove();
            check(sample.textContent===originalText,'Resizing changed captured text');
        }
        check(savedSpace,'Joining did not save any space');
        sample.classList.add('scroll-mode');await frame();
        for(const row of paragraph.children) {
            check(getComputedStyle(row).display==='block','Scroll mode lost its source rows');
            check(row.textContent.startsWith('  '),'Scroll mode lost original prefix spaces');
        }
        check(sample.textContent===originalText,'Scroll mode changed source text');sample.remove();
        const quicklook=document.createElement('pre');quicklook.innerHTML=ansiToHtml(source);document.body.appendChild(quicklook);
        for(const gap of quicklook.querySelectorAll('.reflow-gap'))check(getComputedStyle(gap).display==='none','Quick look gained a leading space');
        check(quicklook.innerText===source,'Quick look changed source rows');quicklook.remove();
    });
    await test('History trimming retains the correct rows within a paragraph', async()=>{
        const first='  This paragraph describes the updated reading experience';
        const next='  continuing with details on the following row.';
        const source=[first,next,'─'.repeat(first.length+2),'retained row'];
        renderScrollback('test',source);loadMoreScrollback('test');
        check(history.querySelector('.terminal-paragraph'),'History did not join its paragraph');
        renderScrollback('test',source.slice(1));
        check(history.children.length===3 && history.firstElementChild.textContent===next,'Partial group trim lost or duplicated rows');
        check(data.scrollbackRendered===3,'Rendered count no longer matches captured rows');
        check(getComputedStyle(history.firstElementChild).display==='block','Retained row is still incorrectly inline');
        appendScrollback('test','after trimming\\n');
        check(history.lastElementChild.textContent==='after trimming','Appending after a partial trim lost content');
    });
    await test('Trimming a paragraph keeps its visible captured row anchored', async()=>{
        const sample=document.createElement('div');sample.className='terminal scroll-active';sample.id='terminal-reflow';sample.style.cssText='width:240px;height:80px';
        sample.innerHTML='<div class="terminal-scrollback has-content" id="scrollback-reflow"></div>';document.body.appendChild(sample);
        const history=sample.firstElementChild;
        const first='  This paragraph describes the updated reading experience';
        const next='  continuing with details on the following row and additional words to wrap.';
        history.innerHTML=ansiToHtml([first,next,'─'.repeat(first.length+2),...Array(20).fill('later row')].join('\\n'));
        sessions.set('reflow',{pinned:false,lastScrollTop:0});await frame();
        const retained=history.querySelector('.terminal-paragraph').children[1];
        sample.scrollTop=retained.getBoundingClientRect().bottom-sample.getBoundingClientRect().top-20;await frame();
        const offset=retained.getBoundingClientRect().top-sample.getBoundingClientRect().top;
        preserveTerminalScroll('reflow',()=>removeScrollbackRows(history,1));await frame();
        check(retained.isConnected && near(retained.getBoundingClientRect().top-sample.getBoundingClientRect().top,offset),'Trimming moved the surviving row');
        sample.remove();sessions.delete('reflow');
    });
    results.textContent += '\\nAll '+passed.length+' browser checks passed.';
}
document.getElementById('run').onclick=()=>run().catch(e=>{results.textContent+='\\nFAIL '+e.stack;});
</script>`;
createServer((_req, res) => { res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }); res.end(html); })
    .listen(8799, '127.0.0.1', () => console.log('Open http://127.0.0.1:8799 and click Run checks.'));
