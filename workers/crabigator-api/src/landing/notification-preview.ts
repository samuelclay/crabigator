// The notification section: a design question from a working session, opened
// from its notification over the iPhone app. The app behind shows Claude's own
// dialog (a real capture) and the session's modules, waiting on the answer.
import { iconWifi } from './icons';
import { appModulesHtml, markChip, type SessionValues } from './app-modules';
import { questionCapture } from './desktop-capture';
import { flowScenes } from './flow-scenes';

const session: SessionValues = {
    mark: { glyph: '⢸⣿⡇', fg: '#403023', bg: '#e4c399' },
    workSeconds: 1260, thinkingSeconds: 840, prompts: 4, tools: 18,
    context: { used: 312_000, limit: 1_000_000 },
    branch: 'ios-rich-notifications', files: 5, symbols: 7,
    recap: 'Answers questions from the notification.',
    state: 'asking',
};

const options = [
    ['Compact rows', 'More sessions visible at once'],
    ['Expanded cards', 'More context for each session'],
    ['Adaptive layout', 'Match the available screen space'],
];

export const notificationPreviewHtml = `
<div class="notification-phone phone-frame" role="img" aria-label="A Crabigator notification over the iPhone app asks how the session list should look, with Compact rows, Expanded cards, and Adaptive layout as answers. Behind it, the session shows the same question and a waiting starship scene.">
    <div class="phone-screen" aria-hidden="true">
        <div class="phone-status"><span>9:41</span><span class="phone-island"></span><span class="phone-indicators"><span class="signal-bars"><i></i><i></i><i></i><i></i></span>${iconWifi}<span class="phone-battery"></span></span></div>
        <div class="notification-app">
            <div class="phone-app-bar"><span class="phone-back">‹</span><div><strong>Native app notifications</strong><span>ClayBook · Live</span></div><span class="phone-type">ᴀA</span></div>
            <div class="phone-terminal">${questionCapture.phoneHtml}</div>
            ${appModulesHtml(session, flowScenes.notification)}
            <div class="phone-composer"><span>Message</span></div>
        </div>
        <div class="notification-card">
            <div class="notification-head">${markChip(session.mark, 'notification-mark')}Native app notifications</div>
            <div class="notification-label">LAYOUT</div>
            <div class="notification-question">How should the session list look?</div>
            ${options.map(([title, detail]) => `<div class="notification-option"><b>${title}</b><span>${detail}</span></div>`).join('')}
            <div class="notification-open">Open session</div>
        </div>
        <div class="notification-menu">${options.map(([title]) => `<span>${title}</span>`).join('')}<span>Reply</span></div>
    </div>
</div>`;
