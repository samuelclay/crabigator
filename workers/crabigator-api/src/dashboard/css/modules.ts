export const modulesCss = `
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
.module-body .session-recap-history { padding: 6px 0; }
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
