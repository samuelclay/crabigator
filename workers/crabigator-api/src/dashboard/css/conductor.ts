export const conductorCss = `
#conductor-view { width: 100%; min-width: 0; max-width: 1600px; height: calc(100dvh - var(--header-height, 67px)); margin: 0 auto; padding: 20px 24px 12px; display: flex; flex-direction: column; font: 15px/1.7 'JetBrains Mono', monospace; color: #d7e0e8; }
#conductor-view [hidden], #conductor-view[hidden] { display: none !important; }
#conductor-view button, #conductor-view input, #conductor-view select, #conductor-view textarea { font: inherit; }
#conductor-view button, #conductor-view a, #conductor-view summary { touch-action: manipulation; }
#conductor-view button { cursor: pointer; }
#conductor-view button:focus-visible, #conductor-view summary:focus-visible, #conductor-view input:focus-visible, #conductor-view textarea:focus-visible { outline: 2px solid #edb583; outline-offset: 2px; }
.conductor-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.conductor-heading h2 { margin: 0; font-size: 18px; font-weight: 600; }
#conductor-status { color: #9baab8; font-size: 11px; }
.conductor-tabs { display: flex; gap: 24px; border-bottom: 1px solid #303c47; margin: 12px 0 16px; }
.conductor-tabs button { background: none; color: #9baab8; border: 0; border-bottom: 2px solid transparent; padding: 10px 0; font-size: 13px !important; }
.conductor-tabs button[aria-pressed=true] { color: #edb583; border-color: #edb583; }
#conductor-error:empty { display: none; }
#conductor-error { padding: 10px; color: #f29a98; background: #352329; margin-bottom: 10px; }
#conductor-conversation, #conductor-sessions, #conductor-archive { flex: 1; min-height: 0; display: flex; flex-direction: column; }
#conductor-messages { flex: 1; min-height: 0; overflow-y: auto; padding: 0 max(0px, calc((100% - 104ch)/2)); scrollbar-gutter: stable; }
.conductor-exchange { padding: 8px 8px 24px; border-bottom: 1px solid #27323c; margin-bottom: 20px; }
.conductor-question { border-left: 2px solid #80b5ce; padding-left: 18px; margin-bottom: 28px; }
.conductor-message-label { font-size: 11px; color: #edb583; margin-bottom: 10px; display: flex; gap: 16px; flex-wrap: wrap; }
.conductor-question .conductor-message-label { color: #9ac9df; }
#conductor-view time { color: #8798a7; font-size: 11px; }
.conductor-prose { overflow-wrap: anywhere; max-width: 88ch; }
.conductor-prose p { white-space: pre-wrap; margin: 0 0 18px; }
.conductor-prose h4 { color: #f1f4f7; font-size: 15px; margin: 22px 0 12px; }
.conductor-prose strong { color: #f4f6f8; font-weight: 700; }
.conductor-prose code { color: #b4d8dd; background: #223039; padding: 1px 4px; border-radius: 3px; }
.conductor-prose a, .conductor-actions a { color: #9ac9df; text-decoration: underline; text-underline-offset: 3px; }
.conductor-table { overflow-x: auto; margin: 18px 0; border: 1px solid #34424d; border-radius: 5px; }
.conductor-table table { width: 100%; border-collapse: collapse; font-size: 12px; }
.conductor-table th, .conductor-table td { text-align: left; padding: 10px 12px; min-width: 120px; border-bottom: 1px solid #303b45; vertical-align: top; }
.conductor-table th { color: #a5c7d1; background: #1c2932; font-weight: 500; }
.conductor-table tr:last-child td { border-bottom: 0; }
#conductor-view details { margin: 14px 0; }
#conductor-view summary { cursor: pointer; color: #9baab8; font-size: 12px; padding: 8px 0; }
.conductor-update { border-left: 1px solid #36414a; padding: 10px 16px; margin: 8px 0; }
.conductor-update .conductor-prose { color: #aebbc8; font-size: 12px; }
.conductor-empty { margin: 30px auto; max-width: 66ch; color: #9baab8; }
.conductor-empty h3 { color: #e7edf2; font-size: 16px; }
.conductor-description { color: #9baab8; font-size: 12px; margin: 0 0 14px; }
#conductor-form { display: flex; gap: 12px; align-items: end; border-top: 1px solid #34414d; padding-top: 14px; margin-top: 10px; }
.conductor-prompt { color: #edb583; padding-bottom: 12px; font-size: 24px; }
#conductor-input { flex: 1; min-width: 0; resize: vertical; max-height: 160px; color: inherit; background: #18222b; border: 1px solid #34414d; border-radius: 5px; padding: 12px; }
#conductor-send { background: #edb583; color: #101419; border: 0; border-radius: 5px; padding: 12px 18px; }
#conductor-send:disabled { opacity: .5; }
.conductor-workspace { display: grid; grid-template-columns: 310px minmax(0,1fr); gap: 28px; flex: 1; min-height: 0; }
.conductor-workspace > aside { overflow-y: auto; border-right: 1px solid #2c3944; padding-right: 16px; }
.conductor-detail { min-width: 0; overflow-y: auto; padding: 4px 14px 24px 0; }
.conductor-project { margin-bottom: 24px; }
.conductor-project h3 { margin: 0 0 10px; color: #b9ccd6; font-size: 12px; overflow-wrap: anywhere; }
.conductor-session-row, .conductor-archive-row { display: flex; flex-direction: column; gap: 6px; text-align: left; width: 100%; padding: 12px; margin-bottom: 8px; background: #17212a; color: #c9d4dd; border: 1px solid #2c3944; border-radius: 5px; }
.conductor-session-row[aria-pressed=true], .conductor-archive-row[aria-pressed=true] { border-color: #ba956d; background: #252923; }
.conductor-session-row strong, .conductor-archive-row strong { font-size: 12px; font-weight: 500; overflow-wrap: anywhere; }
.conductor-session-row span, .conductor-archive-row span:not(.conductor-badge) { font-size: 11px; color: #9baab8; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; overflow-wrap: anywhere; }
.conductor-session-row small { font-size: 10px; color: #c0cbd3; }
.conductor-session-row .conductor-connection { color: #8498a5; }
.conductor-session-heading h3 { margin: 0 0 6px; font-size: 17px; }
.conductor-session-heading p { font-size: 12px; color: #aebbc8; margin: 0 0 12px; }
.conductor-outcomes { display: flex; flex-wrap: wrap; gap: 6px; margin: 16px 0; }
.conductor-outcomes button, .conductor-actions button, .conductor-link { border: 1px solid #394650; background: transparent; border-radius: 4px; padding: 6px 10px; color: #aebbc8; font-size: 12px !important; }
.conductor-outcomes button[aria-pressed=true] { color: #edb583; border-color: #927451; }
.conductor-request { border-bottom: 1px solid #2c3944; }
.conductor-request summary { display: flex; gap: 10px; align-items: baseline; color: #d7e0e8 !important; }
.conductor-request summary::before { content: '▸'; }
.conductor-request details[open] > summary::before { content: '▾'; }
.conductor-entry-heading { display: flex; align-items: center; gap: 14px; margin-bottom: 14px; }
.conductor-detail h3 { font-size: 15px; overflow-wrap: anywhere; }
.conductor-badge { display: inline-block; width: fit-content; font-size: 10px; line-height: 1.5; padding: 3px 7px; border-radius: 3px; white-space: nowrap; }
.conductor-badge.success { color: #a3d8bf; background: #1e382e; }
.conductor-badge.failure { color: #f1afab; background: #412b30; }
.conductor-badge.active { color: #f0ce92; background: #3b3425; }
.conductor-badge.muted { color: #a9b5bf; background: #28323c; }
.conductor-actions { display: flex; gap: 18px; align-items: center; margin: 18px 0; font-size: 12px; }
.conductor-timeline { list-style: none; padding-left: 14px; border-left: 1px solid #36414a; }
.conductor-timeline li { padding: 6px 0; }
.conductor-timeline p { font-size: 12px; white-space: pre-wrap; color: #afbec8; }
.conductor-code { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 360px; overflow: auto; font: 12px/1.6 'JetBrains Mono', monospace; background: #0c141b; padding: 14px; border: 1px solid #2b3945; border-radius: 4px; }
.conductor-search { display: flex; gap: 10px; margin-bottom: 12px; }
.conductor-search input { flex: 1; min-width: 0; }
.conductor-search input, .conductor-search select { color: inherit; border: 1px solid #394650; background: #17212a; padding: 10px; border-radius: 4px; font-size: 12px !important; }
.conductor-archive-session h4 { color: #839ba9; font-size: 10px; margin: 10px 0; }
@media(max-width: 900px) { .conductor-workspace { grid-template-columns: 250px minmax(0,1fr); gap: 16px; } }
@media(max-width: 600px) {
 #conductor-view { padding: 12px; font-size: 13px; }
 .conductor-heading { align-items: start; } #conductor-status { max-width: 48%; text-align: right; font-size: 10px; }
 .conductor-tabs { gap: 18px; margin-bottom: 12px; } .conductor-tabs button { font-size: 11px !important; }
 .conductor-workspace { display: flex; flex-direction: column; gap: 16px; overflow-y: auto; }
 .conductor-workspace > aside { max-height: 32vh; flex-shrink: 0; overflow-y: auto; border-right: 0; border-bottom: 1px solid #34414d; padding: 0 0 8px; }
 .conductor-detail { overflow: visible; padding-right: 0; }
 .conductor-search { flex-wrap: wrap; } .conductor-search input { flex-basis: 100%; }
 .conductor-search select { width: 100%; } .conductor-prompt { display: none; }
 .conductor-prose { line-height: 1.75; } #conductor-form { gap: 8px; }
}

.conductor-session-row .conductor-identity, .conductor-identity { display:flex; align-items:center; gap:12px; min-width:0; text-align:left; }
.conductor-session-row .conductor-identity > span:last-child, .conductor-identity > span:last-child { display:flex; flex-direction:column; gap:5px; min-width:0; }
.conductor-identity strong { font-size:14px; line-height:1.5; overflow-wrap:anywhere; }
.conductor-identity small { color:#9baab8; font-size:11px; font-weight:normal; }
.conductor-identity .session-mark-chip { flex-shrink:0; }
.conductor-identity-link { color:inherit; text-decoration:none; }
.conductor-identity-link:hover strong { text-decoration:underline; }
.conductor-highlights { display:grid; grid-template-columns:repeat(auto-fit,minmax(min(290px,100%),1fr)); gap:18px; margin-top:24px; }
.conductor-highlight { padding:20px; border:1px solid #2c3944; border-top:3px solid #e9ae77; border-radius:6px; background:#17212a; min-width:0; }
.conductor-highlight.success { border-top-color:#8bcdb2; }
.conductor-highlight.failure { border-top-color:#e59b99; }
.conductor-highlight .conductor-entry-heading { margin:16px 0; flex-wrap:wrap; gap:8px; }
.conductor-highlight .conductor-prose { font-size:14px; line-height:1.8; }
.conductor-prose strong { color:#f5c99e; font-weight:700; }
.conductor-recap { color:#9baab8; font-size:12px; line-height:1.7; margin:14px 0 8px; }
.conductor-session-stats { font-size:11px; color:#9baab8; margin:12px 0; }
.conductor-session-stats .success { color:#8bcdb2; }
.conductor-session-stats .failure { color:#e59b99; }
`;
