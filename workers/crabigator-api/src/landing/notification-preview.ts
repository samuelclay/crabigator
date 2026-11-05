// The notification section: an iPhone lock screen, looping every 12 seconds.
// A Crabigator notification arrives over two everyday ones; a long press opens
// the app's answer card (NotificationContentExtension/PromptCard.swift); a tap
// sends "Compact rows", the card shows Sending… then Sent, and closes, and the
// answered notification is gone. With reduced motion it holds on the open card.
import { iconCrabigator, iconWifi, iconLockFilled, iconFlashlight, iconCamera } from './icons';

const title = 'Native app notifications';
const options = [
    ['Compact rows', 'More sessions visible at once'],
    ['Expanded cards', 'More context for each session'],
    ['Adaptive layout', 'Match the available screen space'],
];

const crabIcon = `<span class="ln-icon ln-icon-crab">${iconCrabigator}</span>`;

export const notificationPreviewHtml = `
<div class="notification-phone phone-frame lock-demo" role="img" aria-label="An iPhone lock screen. A Crabigator notification asks how the session list should look. A long press opens it, a tap on Compact rows sends the answer, and the notification goes away.">
    <div class="phone-screen lock-screen" aria-hidden="true">
        <div class="phone-status"><span></span><span class="phone-island"></span><span class="phone-indicators"><span class="signal-bars"><i></i><i></i><i></i><i></i></span>${iconWifi}<span class="phone-battery"></span></span></div>
        <span class="lock-glyph">${iconLockFilled}</span>
        <div class="lock-date">Wednesday, November 4</div>
        <div class="lock-time">9:41</div>
        <div class="lock-note ln-crab">${crabIcon}<div><b>${title}</b><span>How should the session list look?</span><span>${options.map(([label]) => label).join(' · ')}</span></div><small>now</small><i class="ln-press"></i></div>
        <div class="ln-stack">
            <div class="lock-note"><span class="ln-icon ln-icon-calendar"><b>WED</b>4</span><div><b>Design review</b><span>Today at 2:00 PM</span></div><small>15m ago</small></div>
            <div class="lock-note"><span class="ln-icon ln-icon-reminders"><i></i><i></i><i></i></span><div><b>Water the plants</b><span>Today</span></div><small>1h ago</small></div>
        </div>
        <span class="lock-button lock-flashlight">${iconFlashlight}</span>
        <span class="lock-button lock-camera">${iconCamera}</span>
        <div class="phone-home"><span></span></div>
        <div class="ln-dim"></div>
        <div class="ln-card">
            <div class="ln-card-head">${crabIcon}<span>Crabigator</span><small>now</small></div>
            <div class="ln-card-body">
                <p class="ln-title">${title}</p>
                <p class="ln-header">LAYOUT</p>
                <p class="ln-question">How should the session list look?</p>
                ${options.map(([label, detail], index) => `<div class="ln-option${index === 0 ? ' ln-pick' : ''}"><b>${label}</b><span>${detail}</span>${index === 0 ? '<i class="ln-tap"></i>' : ''}</div>`).join('')}
                <div class="ln-status"><span class="ln-sending"><i></i>Sending “Compact rows”…</span><span class="ln-sent">✓ Sent “Compact rows”</span></div>
                <p class="ln-open">Open session</p>
            </div>
        </div>
    </div>
</div>`;

// Sizes are in container units of the phone (100cqw is its width).
export const notificationCss = `
.lock-screen { position: relative; font-family: -apple-system, BlinkMacSystemFont, 'DM Sans', sans-serif; color: #fff;
    background: radial-gradient(90% 55% at 82% 100%, #ffb98a40, transparent 70%), radial-gradient(130% 75% at 50% 0%, #4a3f78 0%, #232a4d 48%, #0f1525 100%); }
.lock-glyph { position: absolute; top: 15.5cqw; left: 50%; width: 4.4cqw; height: 4.4cqw; margin-left: -2.2cqw; color: #fffc; }
.lock-glyph svg { width: 100%; height: 100%; }
.lock-date { position: absolute; top: 24cqw; left: 0; right: 0; text-align: center; font-size: 4.6cqw; font-weight: 600; color: #ece8f6; }
.lock-time { position: absolute; top: 29cqw; left: 0; right: 0; text-align: center; font: 700 25cqw/1 -apple-system, 'DM Sans', sans-serif; letter-spacing: -.02em; color: #f3f1fa; }
.lock-note { position: absolute; left: 3.5cqw; right: 3.5cqw; display: flex; align-items: flex-start; gap: 2.8cqw; padding: 3.2cqw 3.4cqw; border-radius: 5.2cqw;
    background: #1c1f2e8c; backdrop-filter: blur(2.2cqw) saturate(1.4); -webkit-backdrop-filter: blur(2.2cqw) saturate(1.4); }
.lock-note > div { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: .4cqw; font-size: 3.5cqw; line-height: 1.3; }
.lock-note b { font-weight: 600; }
.lock-note span { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: #ffffffd9; }
.lock-note small { color: #ffffff8c; font-size: 3cqw; white-space: nowrap; }
.ln-icon { flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center; width: 8.4cqw; height: 8.4cqw; border-radius: 2cqw; }
.ln-icon-crab { background: #101820; }
.ln-icon-crab svg { width: 70%; height: 70%; }
.ln-icon-calendar { flex-direction: column; gap: 0; background: #fff; color: #111; font: 600 4cqw/1 -apple-system, 'DM Sans', sans-serif; }
.ln-icon-calendar b { color: #e5463b; font-size: 1.6cqw; letter-spacing: .05em; margin-bottom: .2cqw; }
.ln-icon-reminders { flex-direction: column; align-items: flex-start; gap: .7cqw; padding-left: 1.6cqw; background: #fff; }
.ln-icon-reminders i { width: 1.5cqw; height: 1.5cqw; border-radius: 50%; background: #2f7cf6; box-shadow: 2.2cqw 0 0 -.35cqw #c8ccd4; }
.ln-icon-reminders i:nth-child(2) { background: #f19a37; }.ln-icon-reminders i:nth-child(3) { background: #e5463b; }
.ln-crab { top: 106cqw; z-index: 1; }
.ln-stack { position: absolute; left: 0; right: 0; top: 106cqw; }
.ln-stack .lock-note + .lock-note { top: 19cqw; }
.lock-button { position: absolute; bottom: 11cqw; display: flex; align-items: center; justify-content: center; width: 12.5cqw; height: 12.5cqw; border-radius: 50%; background: #ffffff26; backdrop-filter: blur(2cqw); -webkit-backdrop-filter: blur(2cqw); color: #fff; }
.lock-button svg { width: 50%; height: 50%; }
.lock-flashlight { left: 11cqw; }.lock-camera { right: 11cqw; }
.lock-screen .phone-home { position: absolute; left: 0; right: 0; bottom: 0; }
.ln-press, .ln-tap { position: absolute; left: 50%; top: 50%; width: 11cqw; height: 11cqw; margin: -5.5cqw 0 0 -5.5cqw; border-radius: 50%; background: #ffffff8c; box-shadow: 0 0 0 1.6cqw #ffffff2e; opacity: 0; pointer-events: none; }
.ln-press { left: 58%; }
.ln-dim { position: absolute; inset: 0; z-index: 2; background: #080b1366; backdrop-filter: blur(1.8cqw); -webkit-backdrop-filter: blur(1.8cqw); }
.ln-card { position: absolute; left: 3.5cqw; right: 3.5cqw; top: 19cqw; z-index: 3; overflow: hidden; border-radius: 6cqw; background: #1b1f27f2; box-shadow: 0 3cqw 10cqw #0009; }
.ln-card-head { display: flex; align-items: center; gap: 2.4cqw; padding: 3cqw 4cqw; background: #ffffff0d; font-size: 3.5cqw; font-weight: 600; }
.ln-card-head .ln-icon { width: 6cqw; height: 6cqw; border-radius: 1.5cqw; }
.ln-card-head small { margin-left: auto; color: #ffffff80; font-size: 3cqw; font-weight: 400; }
.ln-card-body { padding: 4cqw; }
.ln-card-body p { margin: 0; }
.ln-title { color: #8a8a8a; font-size: 3.4cqw; font-weight: 600; }
.ln-header { margin-top: 3.4cqw !important; color: #d7af5f; font-size: 2.9cqw; font-weight: 700; letter-spacing: .04em; }
.ln-question { margin: 1.4cqw 0 3cqw !important; color: #e9eef4; font-size: 4.3cqw; font-weight: 600; }
.ln-option { position: relative; display: flex; flex-direction: column; gap: .7cqw; margin-top: 2cqw; padding: 2.6cqw 3.2cqw; border-radius: 2.7cqw; background: #ffffff0f; }
.ln-option b { color: #e9eef4; font-size: 4cqw; font-weight: 500; }
.ln-option span { color: #8a8a8a; font-size: 3.3cqw; }
.ln-status { display: grid; margin-top: 3.4cqw; font-size: 3.5cqw; }
.ln-status > span { grid-area: 1 / 1; opacity: 0; }
.ln-sending { display: flex; align-items: center; gap: 1.6cqw; color: #8a8a8a; }
.ln-sending i { width: 3cqw; height: 3cqw; border: .5cqw solid #ffffff40; border-top-color: #ffffffb0; border-radius: 50%; animation: ln-spin .8s linear infinite; }
.ln-sent { color: #5fff5f; font-weight: 600; }
.ln-open { margin-top: 2.6cqw !important; color: #d7af5f; font-size: 3.5cqw; font-weight: 500; }
.ln-crab { opacity: 0; }

.lock-demo .ln-crab { animation: ln-crab 12s infinite; }
.lock-demo .ln-stack { animation: ln-stack 12s infinite; }
.lock-demo .ln-press { animation: ln-press 12s infinite; }
.lock-demo .ln-dim { animation: ln-dim 12s infinite; }
.lock-demo .ln-card { animation: ln-card 12s infinite; }
.lock-demo .ln-tap { animation: ln-tap 12s infinite; }
.lock-demo .ln-pick { animation: ln-pick 12s infinite; }
.lock-demo .ln-sending { animation: ln-sending 12s infinite; }
.lock-demo .ln-sent { animation: ln-sent 12s infinite; }
.lock-demo.paused * { animation-play-state: paused !important; }
@keyframes ln-crab { 0%, 7% { opacity: 0; transform: translateY(-5cqw) scale(.96); } 11%, 23% { opacity: 1; transform: none; } 27%, 33% { opacity: 1; transform: scale(.965); } 36%, 100% { opacity: 0; transform: scale(1.03); } }
@keyframes ln-stack { 0%, 7% { transform: none; } 11%, 77% { transform: translateY(22.5cqw); } 82%, 100% { transform: none; } }
@keyframes ln-press { 0%, 22% { opacity: 0; transform: scale(.5); } 25% { opacity: 1; transform: scale(1); } 33% { opacity: 1; transform: scale(1.18); } 35%, 100% { opacity: 0; transform: scale(1.2); } }
@keyframes ln-dim { 0%, 33% { opacity: 0; } 37%, 74% { opacity: 1; } 78%, 100% { opacity: 0; } }
@keyframes ln-card { 0%, 34% { opacity: 0; transform: translateY(70cqw) scale(.86); } 39%, 74% { opacity: 1; transform: none; } 78%, 100% { opacity: 0; transform: scale(.93); } }
@keyframes ln-tap { 0%, 48% { opacity: 0; transform: scale(.5); } 50% { opacity: 1; transform: scale(1); } 53%, 100% { opacity: 0; transform: scale(1.35); } }
@keyframes ln-pick { 0%, 50% { box-shadow: none; background: #ffffff0f; } 51%, 100% { box-shadow: inset 0 0 0 .4cqw #d7af5fb3; background: #ffffff1a; } }
@keyframes ln-sending { 0%, 51% { opacity: 0; } 52%, 58% { opacity: 1; } 59%, 100% { opacity: 0; } }
@keyframes ln-sent { 0%, 58% { opacity: 0; } 59%, 78% { opacity: 1; } 79%, 100% { opacity: 0; } }
@keyframes ln-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) {
    .lock-demo * { animation: none !important; }
}
`;

// Start the loop from the top whenever the lock screen comes into view, and hold it while out of view.
export const notificationJs = `
document.querySelectorAll('.lock-demo').forEach(function(demo) {
    let visible = false;
    function update() {
        demo.classList.toggle('paused', !visible || document.hidden);
    }
    document.addEventListener('visibilitychange', update);
    new IntersectionObserver(function(entries) {
        const now = entries[0].isIntersecting;
        if (now && !visible && demo.getAnimations) {
            demo.getAnimations({ subtree: true }).forEach(function(animation) { animation.cancel(); animation.play(); });
        }
        visible = now;
        update();
    }, { threshold: .3 }).observe(demo);
    update();
});
`;
