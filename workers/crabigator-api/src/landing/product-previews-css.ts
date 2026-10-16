// Container units scale the entire illustration together, including its type.
export const productPreviewsCss = `
.tablet-frame { container-type: inline-size; }
.tablet-screen { aspect-ratio: 1.52; border-radius: 17px; background: #0e141a; overflow: hidden; display: flex; flex-direction: column; font-family: -apple-system, BlinkMacSystemFont, 'DM Sans', sans-serif; }
.tablet-screen svg:not([fill="none"]), .web-product svg:not([fill="none"]) { fill: currentColor; }
.tablet-screen svg, .web-product svg { width: 1.8cqw; height: 1.8cqw; flex-shrink: 0; }
.tablet-status { display: flex; justify-content: space-between; align-items: center; padding: 1.1cqw 2cqw .8cqw; font-size: 1.05cqw; font-weight: 600; }
.tablet-status > span { display: flex; align-items: center; gap: .7cqw; }
.tablet-status > i { font-style: normal; letter-spacing: .25cqw; color: #77818d; }
.tablet-status svg { width: 1.4cqw; height: 1.4cqw; }
.tablet-status b { font-size: .9cqw; font-weight: 500; }
.tablet-battery { width: 2cqw; height: .9cqw; background: #c6d3dc; border-radius: .2cqw; }
.tablet-toolbar { display: grid; grid-template-columns: 30% 70%; min-height: 6cqw; align-items: center; }
.tablet-toolbar > div { display: flex; align-items: center; justify-content: space-between; padding: .8cqw 2cqw; }
.tablet-toolbar > div:first-child > svg { width: 2.8cqw; height: 2.8cqw; }
.tablet-toolbar > div:first-child > span { display: flex; align-items: center; gap: 1.7cqw; color: #b9a05c; font-size: 2.1cqw; }
.tablet-session-heading { color: #65b1f7; font-size: 1.65cqw; font-weight: 550; }
.tablet-session-heading small { display: block; margin-top: .25cqw; color: #8bca9d; font-size: 1.05cqw; font-weight: 400; }
.tablet-tools { display: flex; gap: 2cqw; align-items: center; color: #c6b06b; font-size: 2.4cqw; }
.tablet-tools > span { color: #a3b0ba; font-size: 1.5cqw; }
.tablet-workspace { display: grid; grid-template-columns: 30% 70%; flex: 1; min-height: 0; }
.tablet-sidebar { position: relative; padding: .6cqw 1.3cqw; }
.tablet-search { display: flex; align-items: center; gap: .6cqw; padding: .7cqw 1cqw; border-radius: .6cqw; background: #080c11; font-size: 1.15cqw; color: #818f9b; }
.preview-machine { display: flex; align-items: center; justify-content: space-between; color: #a3b0ba; font-size: 1cqw; padding: 1.6cqw .6cqw 1.1cqw; }
.preview-machine > span { font-size: .85em; color: #7d8c98; }
.tablet-time { padding: .25cqw .7cqw; border-radius: .3cqw; background: #11323a; color: #64d9df; font-size: 1cqw; font-weight: 600; }
.tablet-repo { color: #d6bb70; font-size: 1cqw; margin: .6cqw .7cqw; }
.preview-session { display: flex; align-items: start; gap: .8cqw; padding: 1.1cqw .7cqw; border-radius: .5cqw; min-width: 0; }
.preview-session.selected { background: #24313a; box-shadow: inset 0 0 0 1.5px #bb9e5a; }
.preview-mark { display: inline-flex; justify-content: center; align-items: center; flex-shrink: 0; padding: .15em .4em; font: 500 1.1cqw/1.4 'JetBrains Mono', monospace; border-radius: .25cqw; }
.preview-mark.amber { background: #a85b19; color: #180e09; }.preview-mark.sand { background: #e4c399; color: #403023; }.preview-mark.berry { background: #5d1c40; color: #eac9e0; }.preview-mark.violet { background: #593179; color: #dfcdec; }
.preview-session > div { min-width: 0; flex: 1; }
.preview-session strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #65b1f7; font-size: 1.15cqw; font-weight: 500; }
.preview-session div > span { display: block; color: #83919e; font-size: .9cqw; margin-top: .5cqw; }
.preview-session > small { color: #ac90d2; font-size: 1cqw; }
.tablet-view-switch { position: absolute; bottom: 1.2cqw; left: 4cqw; display: flex; align-items: center; padding: .4cqw; border-radius: 3cqw; background: #252b2e; font-size: 1.1cqw; color: #929a9f; }
.tablet-view-switch > span { padding: .65cqw 1cqw; display: flex; gap: .5cqw; align-items: center; }.tablet-view-switch .selected { color: #d9bb6b; background: #443f2f; border-radius: 2cqw; }
.tablet-detail { display: flex; flex-direction: column; min-height: 0; }
.tablet-terminal { flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 2cqw; background: #080f15; font: 1.45cqw/1.8 'JetBrains Mono', monospace; overflow: hidden; }
.preview-prompt { display: flex; align-items: baseline; gap: .7em; color: #b7c0c9; margin-bottom: 1.6em; }
.preview-prompt > span { color: #d9e2e8; }
.preview-answer { display: flex; gap: .85em; color: #e0e5e9; }
.preview-bullet { color: #b6cfbc; font-size: .8em; padding-top: .25em; }
.preview-answer p { margin-bottom: .65em; }
.preview-done { color: #7f8d98; font-size: .85em; margin-top: 1.5em; }
.preview-terminal-footer { margin-top: auto; display: flex; justify-content: space-between; gap: 1em; padding-top: 2em; font-size: .72em; color: #889ba9; }.preview-terminal-footer > span:first-child { color: #94b99b; }
.preview-cursor { padding-top: .9em; margin-top: 1em; color: #d2dce3; border-top: 1px solid #ffffff15; }
.preview-cursor i { display: inline-block; vertical-align: middle; width: .55em; height: 1.1em; background: #adbdc8; opacity: .65; margin-left: .3em; }
.preview-composer { display: flex; align-items: center; gap: 1.1cqw; padding: 1cqw 1.4cqw; color: #95a3af; }
.preview-composer > span { flex: 1; background: #101820; border-radius: .7cqw; padding: .75cqw 1cqw; font-size: 1.15cqw; color: #7d8c98; }
.preview-composer > b { font-size: 1.5cqw; color: #a0b8c9; font-weight: 400; }
.tablet-detail .preview-composer > span { box-shadow: inset 0 0 0 1px #ffffff22; }
.tablet-home { display: flex; justify-content: center; padding: .7cqw 0 .5cqw; }.tablet-home > span { width: 22%; height: .35cqw; background: #b5bdc4; border-radius: 1cqw; }
.web-product { container-type: inline-size; }
.web-product .browser-bar { font-size: 1cqw; padding: 1cqw 1.4cqw; }.web-product .browser-bar > span:last-child { font-size: .9cqw; padding: .35cqw 2cqw; }.web-product .window-dots i { width: .6cqw; height: .6cqw; }
.web-preview-header { display: flex; align-items: center; gap: 2cqw; background: #101820; padding: 1.5cqw 2cqw; font-size: 1cqw; color: #a3b0ba; }
.web-preview-header .brand-logo { width: 19cqw; height: auto; margin-right: auto; }
.web-preview-workspace { display: grid; grid-template-columns: 22% 78%; background: #101820; }
.web-preview-sidebar { background: #131e27; padding: 0 .8cqw; }.web-preview-sidebar .preview-machine { font-size: 1.1cqw; padding: 1cqw; }.web-preview-project { font-size: .85cqw; color: #91a1ad; padding: .7cqw 1cqw; }
.web-preview-sidebar .preview-session { padding: 1cqw .6cqw; gap: .65cqw; }.web-preview-sidebar .preview-session.selected { box-shadow: none; }.web-preview-sidebar .preview-session strong { font-size: .9cqw; color: #b7c9d4; }.web-preview-sidebar .preview-session div > span { font-size: .75cqw; }.web-preview-sidebar .preview-mark { font-size: .9cqw; }
.web-preview-main { padding: 0 1.4cqw 1.4cqw; min-width: 0; }
.web-preview-group { display: flex; align-items: baseline; gap: 1.2cqw; padding: 1.2cqw .4cqw 1.5cqw; font-size: .9cqw; color: #8295a3; }.web-preview-group strong { color: #afc3d0; font-weight: 500; }.web-preview-group > span:last-child { margin-left: auto; }
.web-preview-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.2cqw; }
.web-session { min-width: 0; background: #1b2731; border-radius: .6cqw; overflow: hidden; }
.web-session-head { display: flex; align-items: center; flex-wrap: wrap; gap: .5cqw; padding: 1cqw; }.web-session-head > strong { font-size: 1cqw; font-weight: 500; color: #b7c9d4; }.web-session-head > small { flex-basis: 100%; font-size: .8cqw; color: #94b99b; }
.web-session-head .preview-mark { font-size: .85cqw; }
.preview-state { margin-left: auto; font-size: .65cqw; padding: .2cqw .5cqw; background: #ffffff08; border-radius: .3cqw; color: #a3b0ba; }.preview-state.working { color: #92b7dd; background: #29405a; }
.web-terminal { background: #0b1218; padding: 1.2cqw 1cqw; height: 27cqw; font: .93cqw/1.7 'JetBrains Mono', monospace; display: flex; flex-direction: column; overflow: hidden; }.web-terminal .preview-cursor { margin-top: auto; }.web-terminal .preview-answer p { margin-bottom: .6em; }.web-terminal .preview-done { margin-top: 1em; }
.preview-code p { margin-bottom: .2em; }.preview-code-muted { color: #91a1ad; }.preview-add { color: #a7cfb6; }.preview-del { color: #f29a98; }
.web-recap { padding: 1cqw 1cqw .6cqw; font-size: .85cqw; color: #c7d2da; }.web-recap > span { color: #9ac9df; font-size: .7cqw; margin-right: .5cqw; }
.web-stats { display: flex; align-items: center; gap: .7cqw; padding: .5cqw 1cqw; font: .7cqw 'JetBrains Mono', monospace; color: #91a1ad; }.web-stats .preview-add { margin-left: auto; }
.web-session .preview-composer { padding: .8cqw; gap: .7cqw; }.web-session .preview-composer svg { width: 1.2cqw; height: 1.2cqw; }.web-session .preview-composer > span { font-size: .9cqw; padding: .7cqw; }.web-session .preview-composer > b { font-size: 1.1cqw; }
.web-preview-changes { display: flex; flex-wrap: wrap; gap: 1.2cqw; padding: 1.5cqw .3cqw .2cqw; color: #8295a3; font: .75cqw 'JetBrains Mono', monospace; }.web-preview-changes > span:first-child { color: #a7cfb6; }.web-preview-changes .preview-add { margin-left: auto; }
@media (max-width: 700px) {
    .tablet-screen { border-radius: 6px; }
    .tablet-status { font-size: 1.5cqw; }.tablet-status b { font-size: 1.3cqw; }
    .tablet-terminal { font-size: 1.65cqw; line-height: 1.7; }.tablet-terminal .preview-prompt { margin-bottom: 1em; }.tablet-terminal .preview-done { margin-top: .8em; }
    .tablet-view-switch { font-size: 1.3cqw; left: 2cqw; }
    .web-preview-sidebar { display: none; }.web-preview-workspace { grid-template-columns: 1fr; }
    .web-preview-header { font-size: 1.7cqw; }.web-preview-header .brand-logo { width: 28cqw; }
    .web-preview-header > span:first-of-type { display: none; }
    .web-preview-group { font-size: 1.7cqw; padding: 2cqw 1cqw; }
    .web-preview-grid { gap: 2cqw; }
    .web-session-head > strong { font-size: 1.6cqw; }.web-session-head > small { font-size: 1.3cqw; }.web-session-head .preview-mark { font-size: 1.4cqw; }
    .preview-state { display: none; }
    .web-terminal { height: 52cqw; font-size: 1.55cqw; padding: 2cqw; }
    .web-recap { font-size: 1.4cqw; padding: 1.5cqw; }.web-recap > span { font-size: 1.2cqw; }
    .web-stats { font-size: 1.1cqw; padding: 1cqw 1.5cqw; }.web-stats > span:nth-child(3) { display: none; }
    .web-session .preview-composer > span { font-size: 1.6cqw; }.web-session .preview-composer svg { width: 2cqw; height: 2cqw; }
    .web-preview-changes { font-size: 1.3cqw; padding: 2cqw 1cqw; }.web-preview-changes > span:nth-child(n+3):nth-child(-n+5) { display: none; }
}
`;
