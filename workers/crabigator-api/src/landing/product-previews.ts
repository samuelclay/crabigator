import {
    iconCrabigator, iconCrabigatorLogo, iconSearch, iconLines,
    iconMicrophone, iconKeyboard, iconWifi,
} from './icons';
import { appModulesHtml, contextLabel, contextPercent, type SessionValues } from './app-modules';
import { flowScenes } from './flow-scenes';
import { androidCapture } from './desktop-capture';

// Static product illustrations. Keep them out of the tab order: the actual
// dashboard link lives above the preview, and none of these controls are live.
const claudeReply = `
    <div class="preview-prompt"><span>❯</span> How does my desktop session reach my phone?</div>
    <div class="preview-answer"><span class="preview-bullet">●</span><div>
        <p>Claude runs normally in your desktop terminal.</p>
        <p>The live screen and new output stream online.</p>
        <p>Replies from your phone get typed into the terminal.</p>
        <p>Questions and permissions show the same choices.</p>
        <p>It reconnects and resends events saved offline.</p>
    </div></div>
    <div class="preview-done">✻ Worked for 7s</div>
`;

const sessions = [
    { mark: '╱◆╲', tone: 'amber', title: 'Crabigator phone session', branch: 'main', state: 'Complete' },
    { mark: '⢸⣿⡇', tone: 'sand', title: 'Native app notifications', branch: 'ios-rich-notifications', state: 'Working' },
    { mark: '⣀⣾⣀', tone: 'berry', title: 'Clean up the dashboard', branch: 'main', state: 'Working' },
    { mark: '◉ω◉', tone: 'violet', title: 'Flow scenes', branch: 'main', state: 'Complete' },
];

const sessionRows = sessions.map((session, index) => `
    <div class="preview-session ${index === 0 ? 'selected' : ''}">
        <span class="preview-mark ${session.tone}">${session.mark}</span>
        <div><strong>${session.title}</strong><span>${session.branch}</span></div>
        <small>${session.state === 'Working' ? '◌' : '✓'}</small>
    </div>
`).join('');

// The selected session's modules, matching its web dashboard card below.
const tabletSession: SessionValues = {
    mark: { glyph: '╱◆╲', fg: '#180e09', bg: '#a85b19' },
    workSeconds: 2880, thinkingSeconds: 1860, prompts: 5, tools: 6,
    context: { used: 186_000, limit: 1_000_000 },
    branch: 'main', files: 3, symbols: 4,
    recap: 'Desktop session, connected to your phone.',
    wide: { completions: 5, compactions: 1, age: '2m ago', additions: 24, deletions: 8 },
};

const composer = `<div class="preview-composer">${iconMicrophone}${iconKeyboard}<span>Message</span><b>↑</b></div>`;

export const tabletPreviewHtml = `
<div class="tablet-frame" role="img" aria-label="Crabigator on iPad: a native session sidebar beside the live Claude Code terminal, with voice and keyboard input.">
    <div class="tablet-screen" aria-hidden="true">
        <div class="tablet-status"><span>9:41 <span>Thu Oct 15</span></span><i>•••</i><span>${iconWifi}<b>100%</b><span class="tablet-battery"></span></span></div>
        <div class="tablet-toolbar"><div>${iconCrabigator}<span>${iconSearch}<span>⚙</span></span></div><div><span class="tablet-session-heading">Crabigator phone session<small>ClayBook · Live</small></span><span class="tablet-tools">↓ <span>ᴀA</span></span></div></div>
        <div class="tablet-workspace">
            <div class="tablet-sidebar">
                <div class="tablet-search">${iconSearch}<span>Search sessions</span></div>
                <div class="preview-machine">ClayBook <span>4 sessions</span></div>
                <div class="tablet-time">Last hour</div>
                <div class="tablet-repo">samuelclay/crabigator</div>
                ${sessionRows}
                <div class="tablet-view-switch"><span>${iconLines} List</span><span class="selected">▥ PR board</span></div>
            </div>
            <div class="tablet-detail">
                <div class="tablet-terminal">${claudeReply}<div class="preview-terminal-footer"><span>~/projects/crabigator</span><span>Opus · Claude Code</span></div><div class="preview-cursor">❯ <i></i></div></div>
                ${appModulesHtml(tabletSession, flowScenes.tablet)}
                ${composer}
            </div>
        </div>
        <div class="tablet-home"><span></span></div>
    </div>
</div>`;

// The dashboard colors the context window yellow from 70% and red from 85%.
function contextClass(context: SessionValues['context']): string {
    const percent = contextPercent(context);
    if (percent >= 85) return 'ws-red';
    if (percent >= 70) return 'ws-yellow';
    return 'ws-blue';
}

// The dashboard card's widgets panel: mark, title and state, then the stats
// with the context window, and a flow scene at the right.
function webWidgets(mark: string, tone: string, state: string, values: (string | number)[], context: SessionValues['context'], diff: number[], flow: string): string {
    const [session, thinking, prompts, completions, tools, compactions] = values;
    const [additions, deletions] = diff;
    return `<div class="web-widgets">
        <div class="web-widgets-rows">
            <div class="web-widgets-title"><span class="preview-mark ${tone}">${mark}</span><b>Session</b><span>${state}</span></div>
            <div class="web-widgets-stats"><span><i class="ws-blue">◉</i><span class="ws-session">${session}</span></span><span><i class="ws-green">◐</i><span class="ws-thinking">${thinking}</span></span><span><i class="ws-sand">⟩</i>${prompts}</span><span><i class="ws-orange">⋖</i>${completions}</span><span><i class="ws-tools">⚒</i>${tools}</span><span><i class="ws-pink">⊜</i>${compactions}</span><span><i>◔</i><span class="${contextClass(context)}">${contextLabel(context)}</span></span><span class="preview-add">+${additions}</span><span class="preview-del">−${deletions}</span></div>
        </div>
        <div class="web-widgets-flow">${flow}</div>
    </div>`;
}

export const webPreviewHtml = `
<div class="browser-frame web-product" role="img" aria-label="Crabigator web dashboard with a session sidebar, two terminal sessions, recaps, git changes, and message controls.">
    <div aria-hidden="true">
        <div class="browser-bar"><span class="window-dots"><i></i><i></i><i></i></span><span>drinkcrabigator.com/dashboard</span></div>
        <div class="web-preview-header">${iconCrabigatorLogo}<span>↻ Refresh</span><span>⑆ PRs</span><span>4 sessions</span><span>Style</span></div>
        <div class="web-preview-workspace">
            <div class="web-preview-sidebar"><div class="preview-machine">Sessions <span>⌕</span></div><div class="web-preview-project">crabigator</div>${sessionRows}</div>
            <div class="web-preview-main">
                <div class="web-preview-group"><strong>crabigator</strong><span>~/projects/crabigator</span><span>2 sessions</span></div>
                <div class="web-preview-grid">
                    <div class="web-session">
                        <div class="web-session-head"><span class="preview-mark amber">╱◆╲</span><strong>Crabigator phone session</strong><span class="preview-state">Complete</span><small>main</small></div>
                        <div class="web-terminal">${claudeReply}<div class="preview-cursor">❯ <i></i></div></div>
                        <div class="web-recap"><span>● Recap</span> Desktop session, connected to your phone.</div>
                        ${webWidgets('╱◆╲', 'amber', '✓ Complete', ['48m', '31m', 5, 5, 6, 1], { used: 186_000, limit: 1_000_000 }, [24, 8], flowScenes.webBalloon)}
                        ${composer}
                    </div>
                    <div class="web-session">
                        <div class="web-session-head"><span class="preview-mark berry">⣀⣾⣀</span><strong>Clean up the dashboard</strong><span class="preview-state working">Working</span><small>main</small></div>
                        <div class="web-terminal"><div class="preview-prompt"><span>❯</span> Give the sessions a little more breathing room.</div><div class="preview-code"><p>● Update dashboard.css</p><p class="preview-code-muted">&nbsp; .session-card {</p><p class="preview-del">− &nbsp; border: 1px solid var(--border-dim);</p><p class="preview-add">+ &nbsp; border: none;</p><p class="preview-code-muted">&nbsp; }</p><br><p>● Keep the terminal clear and the controls quiet.</p></div><div class="preview-done">◌ Checking the mobile layout…</div><div class="preview-cursor">❯ <i></i></div></div>
                        <div class="web-recap"><span>● Recap</span> Fewer borders. Same useful detail.</div>
                        ${webWidgets('⣀⣾⣀', 'berry', '◌ Working', ['12m', '9m', 3, 2, 8, 0], { used: 742_000, limit: 1_000_000 }, [20, 75], flowScenes.webSki)}
                        ${composer}
                    </div>
                </div>
                <div class="web-preview-changes"><span>⎇ main</span><span>6 files changed</span><span>header.css</span><span>layout.css</span><span>session-card.css</span><span class="preview-add">+20</span><span class="preview-del">−75</span></div>
            </div>
        </div>
    </div>
</div>`;

// Crabigator's Android app (SessionDetail.kt): the same session layout as on
// iPhone, in an Android frame with its status bar and gesture bar.
export const androidPreviewHtml = `
<div class="android-frame" role="img" aria-label="Crabigator's Android app showing a Claude Code session that summarized two pull requests: its terminal, then the session's recap, its pull requests, an engine scene, and its stats with the context window.">
    <div class="android-screen" aria-hidden="true">
        <div class="android-status"><span>9:41</span><i class="android-camera"></i><span class="phone-indicators"><span class="signal-bars"><i></i><i></i><i></i><i></i></span>${iconWifi}<span class="phone-battery"></span></span></div>
        <div class="android-app-bar"><span class="android-back">←</span><div><strong>Crabigator PRs 5 and 6</strong><span>ClayBook · Live</span></div><span class="android-control selected">↓</span><span class="android-control">Aa</span></div>
        <div class="phone-terminal">${androidCapture.phoneHtml}</div>
        ${appModulesHtml(androidCapture.session, flowScenes.android, { cols: 18, rows: 5 })}
        <div class="phone-composer">${iconMicrophone}${iconKeyboard}<span>Message</span></div>
        <div class="android-nav"><span></span></div>
    </div>
</div>`;
