export const conductorJs = String.raw`
let conductorVisible = false, conductorTimer = null, conductorEpoch = 0;
let conductorDraft = '', conductorMessageId = null, conductorSending = false;
let conductorData = {messages: [], tracks: []}, conductorTab = 'conversation';
let conductorSelectedSession = '', conductorSelectedEntry = '', conductorRecentFilter = 'all';
let conductorArchive = {entries: [], total: 0, next_offset: null}, conductorArchiveEpoch = 0;
let conductorSearchTimer = null, conductorHistory = false, conductorArchiveCriteria = '', conductorArchiveLoading = false;
const conductorExpanded = new Set();
function conductorEscape(value) { return String(value || '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c])); }
function conductorDate(value) { return value ? new Date(value).toLocaleString([], {month:'short',day:'numeric',hour:'numeric',minute:'2-digit'}) : ''; }
function conductorEnded(t) { return ['answered','failed','cancelled'].includes(t.state); }
function conductorStatus(t) { return ({sending:'Sending',forwarded:'Awaiting delivery',delivered:'Queued',working:'Working',waiting:'Needs attention',delivery_unknown:'Check delivery',answered:'Answered',failed:'Failed',cancelled:'Stopped'})[t.state] || 'Pending'; }
function conductorTone(t) { return t.state === 'answered' ? 'success' : t.state === 'failed' ? 'failure' : conductorEnded(t) ? 'muted' : 'active'; }
function conductorBadge(t) { return '<span class="conductor-badge '+conductorTone(t)+'">'+conductorStatus(t)+'</span>'; }
function conductorInline(value) {
    const text = String(value || ''), pattern = /(\x60[^\x60]+\x60|\*\*[^*]+\*\*|\[[^\]]+\]\(https?:\/\/[^)]+\))/g;
    let html = '', cursor = 0;
    for (const match of text.matchAll(pattern)) {
        html += conductorEscape(text.slice(cursor,match.index)); const token = match[0];
        if (token.startsWith('**')) html += '<strong>'+conductorEscape(token.slice(2,-2))+'</strong>';
        else if (token.charCodeAt(0) === 96) html += '<code>'+conductorEscape(token.slice(1,-1))+'</code>';
        else { const split = token.indexOf(']('); html += '<a target="_blank" rel="noopener noreferrer" href="'+conductorEscape(token.slice(split+2,-1))+'">'+conductorEscape(token.slice(1,split))+'</a>'; }
        cursor = match.index + token.length;
    }
    return html + conductorEscape(text.slice(cursor));
}
function conductorBody(blocks, fallback) {
    return (blocks && blocks.length ? blocks : [{type:'paragraph',text:fallback || ''}]).map(b => {
        if (b.type === 'table') return '<div class="conductor-table"><table><thead><tr>'+b.headers.map(h=>'<th>'+conductorInline(h)+'</th>').join('')+'</tr></thead><tbody>'+b.rows.map(r=>'<tr>'+r.map(c=>'<td>'+conductorInline(c)+'</td>').join('')+'</tr>').join('')+'</tbody></table></div>';
        if (b.type === 'code') return '<pre class="conductor-code">'+conductorEscape(b.text)+'</pre>';
        return '<'+(b.type === 'heading'?'h4':'p')+'>'+conductorInline(b.text)+'</'+(b.type === 'heading'?'h4':'p')+'>';
    }).join('');
}
function conductorDisclosure(id, title, body, initial = false) {
    return '<details data-disclosure="'+conductorEscape(id)+'"'+(conductorExpanded.has(id) || initial ? ' open' : '')+'><summary>'+title+'</summary>'+body+'</details>';
}
function showSessionList() { hideConductor(); if (prBoardVisible) togglePrBoard(); }
function hideConductor() {
    conductorVisible = false; conductorEpoch++; conductorArchiveEpoch++;
    clearTimeout(conductorTimer); clearTimeout(conductorSearchTimer);
    document.getElementById('conductor-view').hidden = true;
    document.getElementById('conductor-btn').classList.remove('active');
    document.getElementById('sessions').style.display = prBoardVisible ? 'none' : '';
    document.getElementById('session-list-btn').classList.toggle('active', !prBoardVisible);
}
function showConductor() {
    if (conductorVisible) return;
    if (prBoardVisible) togglePrBoard();
    conductorVisible = true; conductorEpoch++;
    document.getElementById('sessions').style.display = 'none';
    document.getElementById('session-list-btn').classList.remove('active');
    document.getElementById('conductor-btn').classList.add('active');
    const view = document.getElementById('conductor-view'); view.hidden = false;
    if (!view.dataset.ready) {
        view.dataset.ready = '1';
        view.innerHTML = '<header class="conductor-heading"><h2>Conductor</h2><span id="conductor-status">Connecting…</span></header><nav class="conductor-tabs" aria-label="Conductor views"><button data-action="tab" data-tab="conversation">Conversation</button><button data-action="tab" data-tab="sessions">Sessions <span id="conductor-active-count"></span></button><button data-action="tab" data-tab="archive">Archive</button></nav><div id="conductor-error" role="alert"></div><section id="conductor-conversation"><div id="conductor-messages"></div><form id="conductor-form"><span class="conductor-prompt" aria-hidden="true">›</span><textarea id="conductor-input" rows="2" maxlength="16000" placeholder="Ask Conductor to coordinate your sessions…" aria-label="Message Conductor"></textarea><button type="submit" id="conductor-send">Send</button></form></section><section id="conductor-sessions" hidden><p class="conductor-description">One row per actual session. Open a session to see the requests it handled.</p><div class="conductor-workspace"><aside id="conductor-session-list" aria-label="Sessions by project"></aside><div id="conductor-session-detail" class="conductor-detail"></div></div></section><section id="conductor-archive" hidden><div class="conductor-search"><input id="conductor-search" type="search" placeholder="Find a question, answer, project, or session…" aria-label="Search Conductor archive" maxlength="200"><select id="conductor-filter" aria-label="Request outcome"><option value="all">All outcomes</option><option value="active">Active</option><option value="answered">Answered</option><option value="failed">Failed</option><option value="stopped">Stopped</option></select></div><p id="conductor-search-status" class="conductor-description" role="status"></p><div class="conductor-workspace"><aside id="conductor-archive-list" aria-label="Archive by project and session"></aside><div id="conductor-archive-detail" class="conductor-detail"></div></div></section>';
        document.getElementById('conductor-input').value = conductorDraft;
        document.getElementById('conductor-input').addEventListener('input', e => { conductorDraft = e.target.value; conductorMessageId = null; });
        document.getElementById('conductor-form').addEventListener('submit', conductorSend);
        document.getElementById('conductor-search').addEventListener('input', () => { clearTimeout(conductorSearchTimer); conductorArchiveEpoch++; conductorSearchTimer = setTimeout(()=>loadConductorArchive(),250); });
        document.getElementById('conductor-filter').addEventListener('change',()=>loadConductorArchive());
        view.addEventListener('toggle', e => { const id = e.target.dataset.disclosure; if (id && e.target.isConnected) { if (e.target.open) conductorExpanded.add(id); else conductorExpanded.delete(id); } },true);
        view.addEventListener('click',async e=>{
            const button = e.target.closest('button[data-action]'); if (!button) return;
            switch(button.dataset.action) {
                case 'tab': conductorSetTab(button.dataset.tab); break;
                case 'inspect': conductorSelectedSession = button.dataset.id; conductorSetTab('sessions'); conductorRenderSessions(); break;
                case 'session': conductorSelectedSession = button.dataset.id; conductorRecentFilter = 'all'; conductorRenderSessions(); break;
                case 'entry': conductorSelectedEntry = button.dataset.id; conductorRenderArchive(); break;
                case 'history': conductorHistory = !conductorHistory; conductorRenderConversation(); break;
                case 'filter': conductorRecentFilter = button.dataset.filter; conductorRenderSessions(); break;
                case 'more': loadConductorArchive(true); break;
                case 'copy': {
                    const entry = [...conductorData.tracks,...conductorArchive.entries].find(t=>t.id===button.dataset.id);
                    try { await navigator.clipboard.writeText(conductorCopyText(entry)); button.textContent='Copied'; }
                    catch { button.textContent='Select the answer to copy'; } break;
                }
            }
        });
    }
    conductorSetTab(conductorTab); loadConductor(conductorEpoch);
}
function conductorSetTab(tab) {
    conductorTab = tab;
    for (const name of ['conversation','sessions','archive']) {
        document.getElementById('conductor-'+name).hidden = name !== tab;
        const button = document.querySelector('#conductor-view [data-tab="'+name+'"]');
        button.setAttribute('aria-pressed',String(name===tab));
    }
    if (tab==='archive') loadConductorArchive();
}
async function conductorRequest(path, body) {
    const response = await fetch('/api/conductor'+path,{method:body?'POST':'GET',headers:Object.assign({},getAuthHeaders(),body?{'Content-Type':'application/json'}:{}),...(body?{body:JSON.stringify(body)}:{})});
    if (!response.ok) { const data=await response.json().catch(()=>({})); throw new Error(response.status===404?'Conductor is not available on this server yet.':data.error||'Could not connect to Conductor.'); }
    return response.json();
}
function conductorPaint(id,html) { const el=document.getElementById(id); if (el.innerHTML!==html) el.innerHTML=html; }
function conductorRenderConversation() {
    const exchanges=[];
    for (const message of conductorData.messages) {
        if (message.role==='user' || !exchanges.length) exchanges.push([]);
        exchanges[exchanges.length-1].push(message);
    }
    const chosen=conductorHistory?exchanges:exchanges.slice(-1);
    let html=chosen.map(exchange=>{
        const question=exchange.find(m=>m.role==='user'), replies=exchange.filter(m=>m.role!=='user'), latest=replies.at(-1);
        return '<article class="conductor-exchange">'+(question?'<div class="conductor-question"><div class="conductor-message-label">You <time>'+conductorDate(question.at)+'</time></div><div class="conductor-prose">'+conductorBody(question.blocks,question.text)+'</div></div>':'')+
            (latest?'<div class="conductor-answer"><div class="conductor-message-label">Latest reply <time>'+conductorDate(latest.at)+'</time></div><div class="conductor-prose">'+conductorBody(latest.blocks,latest.text)+'</div>'+conductorHighlights(latest)+'</div>':'<p class="conductor-description">'+(conductorData.working?'Conductor is working…':'Waiting for the desktop coordinator…')+'</p>')+
            (question && !(latest?.highlights||[]).length ? conductorWork(question.id) : '')+(replies.length>1?conductorDisclosure('exchange-'+replies[0].id,'Earlier updates · '+(replies.length-1),replies.slice(0,-1).map(m=>'<div class="conductor-update"><time>'+conductorDate(m.at)+'</time><div class="conductor-prose">'+conductorBody(m.blocks,m.text)+'</div>'+conductorHighlights(m)+'</div>').join('')):'')+'</article>';
    }).join('');
    if (!html) html='<div class="conductor-empty"><h3>One conversation. Your existing sessions.</h3><p>Ask a question here. Conductor finds the right sessions and brings their answers back.</p><p>Try: “Ask NewsBlur and Turn Touch for their app store status.”</p></div>';
    if (exchanges.length>1) html+='<button class="conductor-link" data-action="history">'+(conductorHistory?'Show latest exchange':'Earlier conversation · '+(exchanges.length-1)+' exchanges')+'</button>';
    if (!conductorData.host_online) html+='<p class="conductor-description">Desktop offline. Start <code>crabigator conductor</code> on your computer.</p>';
    conductorPaint('conductor-messages',html);
}
function conductorWork(requestId) {
    const tracks=conductorData.tracks.filter(t=>t.request_id===requestId);
    return tracks.length?'<div class="conductor-highlights">'+tracks.map(t=>'<article class="conductor-highlight '+conductorTone(t)+'">'+conductorIdentity(t)+'<div class="conductor-entry-heading">'+conductorBadge(t)+'<time>'+conductorDate(t.updated_at)+'</time></div><p class="conductor-recap">'+conductorEscape(!conductorEnded(t) && t.events?.length ? t.events.at(-1).text.slice(0,180) : t.title)+'</p><button class="conductor-link" data-action="inspect" data-id="'+conductorEscape(t.session_id)+'">View request →</button></article>').join('')+'</div>':'';
}
function conductorSessionName(t) { return t.session_title || t.project || 'Session'; }
function conductorIdentity(t) {
    return '<span class="conductor-identity">'+sessionMarkChipHtml(t)+'<span><strong>'+conductorEscape(conductorSessionName(t))+'</strong><small>'+conductorEscape([t.project,t.platform].filter(Boolean).join(' · '))+'</small></span></span>';
}
function conductorContext(t) {
    return (t.recap?'<p class="conductor-recap">'+conductorEscape(t.recap)+'</p>':'')+'<div class="conductor-session-stats">'+(t.additions||t.deletions?'<span class="success">+'+Number(t.additions||0)+'</span> <span class="failure">−'+Number(t.deletions||0)+'</span> · ':'')+conductorEscape([t.session_state,t.prompts? t.prompts+' prompts':'',t.completions? t.completions+' completed':''].filter(Boolean).join(' · '))+'</div>';
}
function conductorCopyText(entry) { return [entry.result || entry.text || '', ...(entry.highlights||[]).map(h=>conductorSessionName(h)+'\n'+h.text)].filter(Boolean).join('\n\n'); }
function conductorHighlights(message) {
    return (message.highlights||[]).length?'<div class="conductor-highlights">'+message.highlights.map(h=>'<article class="conductor-highlight '+conductorTone(h)+'"><a class="conductor-identity-link" href="/dashboard?session='+encodeURIComponent(h.session_id)+'" target="_blank" rel="noopener">'+conductorIdentity(h)+'</a><div class="conductor-entry-heading">'+(h.state?conductorBadge(h):'')+'<time>'+conductorDate(h.updated_at)+'</time></div><div class="conductor-prose">'+conductorBody(h.blocks,h.text)+'</div></article>').join('')+'</div>':'';
}
function conductorGroups(entries) {
    const projects=new Map();
    for(const entry of entries) {
        const project=entry.project || 'Unknown project'; if (!projects.has(project)) projects.set(project,new Map());
        const sessions=projects.get(project), id=entry.session_id || 'conversation'; if (!sessions.has(id)) sessions.set(id,[]); sessions.get(id).push(entry);
    }
    return new Map([...projects].sort(([a],[b]) => Number(a==='Conductor')-Number(b==='Conductor') || a.localeCompare(b)));
}
function conductorEntryDetail(t, live=false) {
    const isMessage=t.kind==='message';
    const events=(t.events||[]).filter(e=>e.text!==t.result);
    return '<div class="conductor-entry-heading">'+(isMessage?'<span class="conductor-badge muted">'+(t.role==='user'?'Question':'Coordinator reply')+'</span>':conductorBadge(t))+'<time>'+conductorDate(t.updated_at||t.at)+'</time></div>'+
        (isMessage?'':'<h3>'+conductorEscape(t.title)+'</h3>'+conductorDisclosure('question-'+t.id,'Original question','<div class="conductor-prose">'+conductorBody(t.question_blocks,t.question||t.instruction)+'</div>'))+
        '<div class="conductor-prose conductor-result">'+conductorBody(t.blocks,t.result||t.text||(events.at(-1)||{}).text||'Waiting for a report from this session.')+'</div>'+
        conductorHighlights(t)+'<div class="conductor-actions">'+(t.result||t.text?'<button data-action="copy" data-id="'+conductorEscape(t.id)+'">Copy '+(t.state==='answered'?'answer':'text')+'</button>':'')+(t.session_id?'<a href="/dashboard?session='+encodeURIComponent(t.session_id)+'" target="_blank" rel="noopener">Open actual session ↗</a>':'')+'</div>'+
        (events.length?conductorDisclosure('progress-'+t.id,'Progress · '+events.length+' updates','<ol class="conductor-timeline">'+events.map(e=>'<li><time>'+conductorDate(e.at)+'</time><p>'+conductorEscape(e.text)+'</p></li>').join('')+'</ol>'):'')+
        (live?conductorDisclosure('terminal-'+t.id,'Live session activity','<p class="conductor-description">This is the session’s wider activity, including unrelated work.</p><pre class="conductor-code">'+ansiToHtml(t.activity||t.screen||'No live output available.')+'</pre>'):'');
}
function conductorRenderSessions() {
    const tracks=conductorData.tracks, sessions=conductorGroups(tracks);
    if (!tracks.some(t=>t.session_id===conductorSelectedSession)) conductorSelectedSession=tracks[0]?.session_id||'';
    let list='';
    for(const [project,groups] of sessions) {
        list+='<section class="conductor-project"><h3>'+conductorEscape(project)+'</h3>';
        for(const [id,items] of groups) {
            const t=items[0], active=items.filter(t=>!conductorEnded(t)).length, answered=items.filter(t=>t.state==='answered').length, failed=items.filter(t=>t.state==='failed').length;
            list+='<button class="conductor-session-row" data-action="session" data-id="'+conductorEscape(id)+'" aria-pressed="'+(id===conductorSelectedSession)+'">'+conductorIdentity(t)+'<small>'+active+' active · '+answered+' answered'+(failed?' · '+failed+' failed':'')+'</small><small class="conductor-connection">'+(t.connected?'● Online':'○ Offline')+'</small></button>';
        }
        list+='</section>';
    }
    conductorPaint('conductor-session-list',list||'<p class="conductor-description">Sessions appear here when Conductor sends them a request.</p>');
    const all=tracks.filter(t=>t.session_id===conductorSelectedSession), t=all[0];
    if(!t) { conductorPaint('conductor-session-detail','<div class="conductor-empty">No requests sent yet. Start in Conversation.</div>'); return; }
    const selected=all.filter(t=>conductorRecentFilter==='all'||(conductorRecentFilter==='active'?!conductorEnded(t):t.state===conductorRecentFilter));
    let detail='<header class="conductor-session-heading">'+conductorIdentity(t)+conductorContext(t)+'<p>'+conductorEscape(t.platform||'')+(t.device_name?' · '+conductorEscape(t.device_name):'')+' · '+(t.connected?'Online':'Offline')+'</p><p class="conductor-description">'+all.length+' recent requests. Older requests and answers are in Archive.</p></header><div class="conductor-outcomes">'+[['all','All recent'],['active','Active'],['answered','Answered'],['failed','Failed']].map(([key,label])=>'<button data-action="filter" data-filter="'+key+'" aria-pressed="'+(key===conductorRecentFilter)+'">'+label+'</button>').join('')+'</div>';
    detail+=selected.map(item=>'<article class="conductor-request">'+conductorDisclosure('request-'+item.id,conductorBadge(item)+' <span>'+conductorEscape(item.title)+'</span>',conductorEntryDetail(item,true))+'</article>').join('')||'<p class="conductor-description">No '+conductorRecentFilter+' requests for this session.</p>';
    conductorPaint('conductor-session-detail',detail);
}
async function loadConductorArchive(more=false) {
    const query=document.getElementById('conductor-search').value, status=document.getElementById('conductor-filter').value;
    const criteria=JSON.stringify([query,status]);
    if(more && (conductorArchiveLoading || conductorArchiveCriteria!==criteria)) return;
    const offset=more?conductorArchive.next_offset:0; if (offset===null) return;
    const epoch=++conductorArchiveEpoch; conductorArchiveLoading=true;
    document.getElementById('conductor-search-status').textContent='Searching saved questions and answers…';
    try {
        const data=await conductorRequest('/archive',{query,status,offset});
        if(epoch!==conductorArchiveEpoch||!conductorVisible) return;
        conductorArchiveCriteria=criteria; conductorArchive={...data,entries:more?[...conductorArchive.entries,...data.entries.filter(e=>!conductorArchive.entries.some(old=>old.id===e.id))]:data.entries};
        conductorRenderArchive();
    } catch(error) { if(epoch===conductorArchiveEpoch) document.getElementById('conductor-search-status').textContent=error.message; }
    finally { if(epoch===conductorArchiveEpoch) conductorArchiveLoading=false; }
}
function conductorRenderArchive() {
    const entries=conductorArchive.entries;
    if(!entries.some(e=>e.id===conductorSelectedEntry)) conductorSelectedEntry=entries[0]?.id||'';
    document.getElementById('conductor-search-status').textContent=conductorArchive.total+' saved matches · grouped by project and session';
    let list='';
    for(const [project,sessions] of conductorGroups(entries)) {
        list+='<section class="conductor-project"><h3>'+conductorEscape(project)+'</h3>';
        for(const [id,items] of sessions) {
            list+='<div class="conductor-archive-session"><h4>'+(id==='conversation'?'Coordinator conversation':conductorIdentity(items[0]))+'</h4>';
            for(const e of items) list+='<button class="conductor-archive-row" data-action="entry" data-id="'+conductorEscape(e.id)+'" aria-pressed="'+(e.id===conductorSelectedEntry)+'">'+(e.kind==='message'?'<small>'+(e.role==='user'?'Question':'Coordinator reply')+'</small>':conductorBadge(e))+'<strong>'+conductorEscape(e.title||(e.text||'').slice(0,90))+'</strong><span>'+conductorEscape(e.excerpt||'')+'</span><time>'+conductorDate(e.updated_at||e.at)+'</time></button>';
            list+='</div>';
        }
        list+='</section>';
    }
    if(conductorArchive.next_offset!==null) list+='<button class="conductor-link" data-action="more">Load more · '+entries.length+' of '+conductorArchive.total+'</button>';
    conductorPaint('conductor-archive-list',list||'<p class="conductor-description">No matches. Try another word or outcome.</p>');
    const selected=entries.find(e=>e.id===conductorSelectedEntry);
    conductorPaint('conductor-archive-detail',selected?'<p class="conductor-description">'+conductorEscape(selected.project||'Conductor')+(selected.session_id?' / '+conductorEscape(conductorSessionName(selected)):'')+'</p>'+conductorEntryDetail(selected):'<div class="conductor-empty">Search a project, a question, or a phrase from an answer.</div>');
}
async function loadConductor(epoch) {
    if(!conductorVisible||epoch!==conductorEpoch) return;
    try {
        const data=await conductorRequest(''); if(!conductorVisible||epoch!==conductorEpoch) return; conductorData=data;
        document.getElementById('conductor-status').textContent=data.host_online?(data.working?'● Working':'● Ready')+' · '+data.host_name:'○ Desktop offline';
        document.getElementById('conductor-error').textContent=data.error||'';
        const active=data.tracks.filter(t=>!conductorEnded(t)).length;
        document.getElementById('conductor-active-count').textContent=active?'('+active+' active)':'';
        conductorRenderConversation(); conductorRenderSessions();
    } catch(error) { if(epoch===conductorEpoch) document.getElementById('conductor-error').textContent=error.message; }
    finally { if(conductorVisible&&epoch===conductorEpoch) conductorTimer=setTimeout(()=>loadConductor(epoch),3000); }
}
async function conductorSend(event) {
    event.preventDefault(); if(conductorSending||!conductorDraft.trim()) return;
    conductorSending=true;
    const text=conductorDraft.trim(),id=conductorMessageId||(conductorMessageId=crypto.randomUUID()),button=document.getElementById('conductor-send'); button.disabled=true;
    try {
        await conductorRequest('/message',{id,text});
        if(conductorDraft.trim()===text) { conductorDraft=''; conductorMessageId=null; document.getElementById('conductor-input').value=''; }
        conductorHistory=false; clearTimeout(conductorTimer); conductorEpoch++; loadConductor(conductorEpoch);
    } catch(error) { document.getElementById('conductor-error').textContent=error.message; }
    finally { conductorSending=false; button.disabled=false; }
}
`;
