export const modulesCss = `
/* Shared card heading: expansion reveals the terminal, not a different title. */
.container .session-card:not(.session-fullscreen) { margin-bottom: 12px; border: 1px solid var(--border-dim); border-radius: 10px; }
.container .session-card .session-header { padding: 12px 14px; gap: 10px; border-radius: 10px 10px 0 0; align-items: flex-start; }
.container .session-row .session-header { border-radius: 10px; }
.session-heading-button { background: none; border: 0; padding: 0; text-align: left; color: inherit; font: inherit; cursor: pointer; }
.session-heading-button:focus-visible { outline: 2px solid var(--accent-cyan); outline-offset: 4px; border-radius: 3px; }
.container .session-header .title-row { align-items: baseline; gap: 8px; }
.container .session-header .title { font-size: 14px; font-weight: 500; color: var(--text-bright); white-space: normal; overflow-wrap: anywhere; }
.session-header .session-context { display: flex; flex-wrap: wrap; gap: 4px 10px; font-size: 10px; color: var(--text-dim); }
.container .session-card .session-header .cwd { display: inline; font: inherit; margin: 0; }
.session-header .branch { color: #7ee787; font: inherit; }
.container .session-header .session-row-recap { margin: 0; max-width: none; font-size: 11px; color: var(--text-mid); }
.session-header .session-row-recap:empty, .session-header .branch:empty, .session-header .session-row-stats:empty { display: none; }
.container .session-header .generated-title { font-family: var(--font-ui); max-width: none; color: #8bbcf0; }
.container .session-header .session-actions { gap: 4px; }
.container .session-header .collapse-btn { min-width: 28px; min-height: 28px; font-size: 20px; }
@media (max-width: 720px) {
    .container .session-card .session-header { flex-wrap: wrap; padding: 12px; }
    .session-header .session-info { flex-basis: 100%; }
    .session-header .session-actions { margin-left: auto; }
}
/* A permanent one-line handle, with independently opened modules above it. */
.has-modules .widgets-panel { display: flex; flex-direction: column; min-height: 0; }
.has-modules .widgets-header { flex: none; flex-direction: row; align-items: center; padding: 0 0 0 6px; gap: 8px; height: 44px; font-size: 11px; border-top: 1px solid var(--border-dim); }
.has-modules .widgets-header > span { flex: none; }
.has-modules .widgets-header-row2 { flex: 1; min-width: 0; flex-wrap: nowrap; overflow-x: auto; scrollbar-width: none; white-space: nowrap; height: 44px; gap: 10px; }
.has-modules .wh-icon { font-size: inherit; }
.has-modules .wh-stat { flex: none; }
.has-modules .widgets-header .collapse-btn { align-self: stretch; width: 44px; padding: 0; background: #d7af5f0b; color: #d7af5f; font-size: 20px; transform: rotate(180deg); transition: transform .3s cubic-bezier(.2,1.5,.5,1); }
.has-modules.widgets-collapsed .widgets-header .collapse-btn { transform: rotate(0); }
.has-modules .wh-stat[data-card-stats="sessionTime"] .wh-value { color: #00afff; }
.has-modules .wh-stat[data-card-stats="thinkingTime"] .wh-value { color: #5fff5f; }
.has-modules .wh-stat[data-card-stats^="prompts"] .wh-icon, .has-modules .wh-stat[data-card-stats^="prompts"] .wh-elapsed { color: #d7af5f !important; }
.has-modules .wh-stat[data-card-stats^="completions"] .wh-icon, .has-modules .wh-stat[data-card-stats^="completions"] .wh-elapsed { color: #ff8700 !important; }
.has-modules .wh-elapsed { font-size: inherit; }
.has-modules .wh-git { display: none; }
.has-modules .widgets-content { display: block; background: var(--bg-card); max-height: min(45vh, 460px); overflow-y: auto; transition: max-height .3s cubic-bezier(.2,.8,.2,1), opacity .18s; }
.has-modules.widgets-collapsed .widgets-content { max-height: 0; opacity: 0; overflow: hidden; }
.session-module { border-bottom: 1px solid var(--border-dim); transform-origin: bottom; transition: transform .3s cubic-bezier(.2,1.4,.5,1), opacity .2s; transition-delay: calc((5 - var(--module-order)) * 25ms); }
.widgets-collapsed .session-module { opacity: 0; transform: translateY(12px) scale(.97); transition-duration: .14s; transition-delay: calc(var(--module-order) * 12ms); }
.session-module[hidden] { display: none; }
.session-module > summary { list-style: none; display: flex; align-items: center; gap: 8px; min-height: 44px; padding: 7px 10px; cursor: pointer; font-size: 11px; color: #d7af5f; }
.session-module > summary::-webkit-details-marker { display: none; }
.module-arrow { font-size: 18px; transition: transform .2s; }
.session-module[open] .module-arrow { transform: rotate(90deg); }
.module-label { font-weight: 600; flex: none; }
.module-preview { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-mid); font-weight: normal; }
.module-age { flex: none; color: var(--text-dim); font-size: 10px; white-space: nowrap; }
.module-recap > summary { color: #22d3ee; }
.module-prs > summary, .module-titles > summary { color: #bc8cff; }
.module-git > summary { color: #5fff5f; }
.module-commits > summary { color: var(--text-mid); }
.module-body { padding: 0 10px 10px; }
.module-body .widget, .module-body .session-recap, .module-body .session-prs { padding: 8px 0; background: transparent; border: 0; margin: 0; }
.module-body .session-recap { cursor: default; }
.module-body .pr-list-title, .module-body .session-recap-status, .module-body .changes-session-titles, .module-body .commit-history-title { display: none; }
.module-body .widget-title { margin-bottom: 8px; }
.module-recap .session-recap, .module-recap .session-recap-history { display: none !important; }
.module-recap-row { display: flex; align-items: baseline; gap: 8px; padding: 7px 0; font-size: 12px; line-height: 1.4; }
.module-recap-number { flex: none; color: #22d3ee; font-size: 10px; font-variant-numeric: tabular-nums; }
.module-recap-headline { flex: 1; min-width: 0; overflow-wrap: anywhere; color: var(--text-mid); }
.module-recap-row:first-child .module-recap-headline { color: var(--text-bright); }
.module-body .session-prs .pr-row { padding: 6px 0; background: transparent; border: 0; border-radius: 0; }
.has-modules .widgets-content.no-changes, .has-modules .widgets-content.no-titles { grid-template-columns: none; }
.fullscreen-session-btn { background: none; border: 0; color: var(--text-mid); min-width: 36px; min-height: 36px; font-size: 22px; cursor: pointer; }
.session-fullscreen-open { overflow: hidden; }
.session-card.session-fullscreen { position: fixed; inset: 0; margin: 0; width: 100%; height: 100dvh; max-width: none; max-height: none; border: 0; border-radius: 0; background: var(--bg-card); display: flex; flex-direction: column; padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left); }
.session-fullscreen::backdrop { background: var(--bg-deep); }
.session-fullscreen > .session-header { flex: none; }
.session-fullscreen > .session-body { display: flex; flex: 1; min-height: 0; }
.session-fullscreen .session-body-inner { display: flex; flex-direction: column; flex: 1; min-height: 0; width: 100%; }
.session-fullscreen .terminal { flex: 1; height: auto !important; min-height: 80px; overflow-y: auto; transition: none; }
.session-fullscreen .widgets-panel, .session-fullscreen .input-area, .session-fullscreen .prompt-panel { flex: none; }
.session-fullscreen .widgets-content { max-height: 40dvh; }
.session-fullscreen.widgets-collapsed .widgets-content { max-height: 0; }
.session-fullscreen .prompt-panel { max-height: 30dvh; overflow-y: auto; }
.session-fullscreen .session-actions > .collapse-btn { display: none; }
.session-fullscreen .session-header .cwd, .session-fullscreen .generated-title { max-width: 70vw; }
@media (prefers-reduced-motion: reduce) {
    .has-modules .widgets-content, .session-module, .module-arrow, .has-modules .widgets-header .collapse-btn { transition: none; }
}
`;
