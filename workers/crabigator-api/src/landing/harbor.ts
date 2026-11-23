import { iconCrabigator, iconWifi } from './icons';
import { harborArtJs } from './harbor-art';

const question = 'What color should it glow?';
const answer = 'Warm amber.';

// The completed exchange is also the no-script and reduced-motion view.
export const harborHtml = `
<figure class="harbor" aria-label="A terminal and a phone share one session. The terminal asks what color to light the lighthouse. A reply from the phone turns the beacon warm amber.">
    <div class="harbor-world" aria-hidden="true"><canvas class="harbor-art"></canvas><canvas class="harbor-water"></canvas><i class="harbor-lamp"></i></div>
    <svg class="harbor-route" aria-hidden="true"><path/></svg>
    <div class="harbor-courier" aria-hidden="true">${iconCrabigator}</div>
    <div class="harbor-term" aria-hidden="true">
        <div class="harbor-term-bar"><span class="window-dots"><i></i><i></i><i></i></span><span>~/harbor</span></div>
        <div class="harbor-log">
            <div class="harbor-command">$ crabigator codex</div>
            <p data-at=".4" class="harbor-prompt">Light the harbor beacon.</p>
            <p data-at="2.5">${question}</p>
            <p data-at="9.4" class="harbor-answer">${answer}</p>
            <p data-at="10.4" class="harbor-done">✓ Beacon lit.</p>
        </div>
        <i class="harbor-port"></i>
    </div>
    <div class="harbor-phone" aria-hidden="true">
        <div class="harbor-phone-status">9:41 <i></i>${iconWifi}</div>
        <div class="harbor-app">${iconCrabigator}<strong>Harbor light</strong></div>
        <div class="harbor-messages"><p data-at="5.4">${question}</p><p class="harbor-bubble">${answer}</p></div>
        <div class="harbor-composer"><span data-at="6.9" data-until="7.9">${answer}</span><b>↑</b></div>
        <div class="harbor-home"></div><i class="harbor-port"></i>
    </div>
</figure>`;

export const harborCss = `
.section .harbor-title > span { display:inline; color:inherit; transition:color .55s ease-out; }
[data-speaker="terminal"] .harbor-title span:first-child, [data-speaker="phone"] .harbor-title span:last-child { color:#ffcdab; }
.harbor { position:relative; isolation:isolate; overflow:hidden; height:clamp(390px,36vw,470px); margin:44px auto 0; border-radius:16px; border:1px solid #485562; background:linear-gradient(#192a41,#73616b 64%,#28404b 76%,#112833 77%); }
.harbor-world,.harbor-world canvas,.harbor-route { position:absolute; inset:0; width:100%; height:100%; pointer-events:none; }
.harbor-art { image-rendering:pixelated; }
.harbor-lamp { position:absolute; width:7px; height:5px; transform:translate(-50%,-50%); background:#fff0ae; box-shadow:0 0 9px 4px #ffc276,0 0 38px 14px #ffb65c80; }
.harbor-route path { stroke:#ffcd9c; stroke-width:1.25; stroke-dasharray:2 6; fill:none; opacity:.55; }
.harbor-courier { position:absolute; top:0; left:0; width:26px; height:26px; margin:-13px; opacity:0; filter:drop-shadow(0 0 7px #ffbe80); pointer-events:none; }
.harbor-courier svg { display:block; width:100%; }
.harbor-term,.harbor-phone { position:absolute; color:#ecede8; text-align:left; box-shadow:0 12px 30px #07101c65,0 1px 0 #ffffff20 inset; }
.harbor-term { left:5%; top:17%; width:39%; max-width:436px; border:1px solid #8b929980; background:#101c26f7; border-radius:11px; }
.harbor-term-bar { display:flex; align-items:center; gap:16px; border-bottom:1px solid #82939a38; background:#c3dad90d; padding:9px 14px; border-radius:11px 11px 0 0; color:#b9c7c9; font:11px 'JetBrains Mono',monospace; }
.harbor-log { padding:16px 18px; min-height:163px; font:clamp(12px,1.25vw,16px)/1.55 'JetBrains Mono',monospace; }
.harbor-log p { margin-top:.35em; }
.harbor-command { color:#8b9ca5; font-size:.85em; margin-bottom:12px; }
.harbor-prompt::before,.harbor-answer::before { content:'› '; color:var(--peach); }
.harbor-answer,.harbor-bubble { color:#ffce9d; }
.harbor-done { color:#acd6bf; }
.harbor-port { position:absolute; left:100%; top:56%; }
.harbor-phone { right:7%; top:10%; width:22%; max-width:212px; height:73%; max-height:305px; background:#101c26; border:4px solid #141b22; outline:1.5px solid #828888; border-radius:26px; display:flex; flex-direction:column; }
.harbor-phone .harbor-port { left:0; top:52%; }
.harbor-phone-status { display:flex; align-items:center; justify-content:space-between; padding:10px 14px 5px; font-size:10px; font-weight:650; }
.harbor-phone-status i { border-radius:8px; width:30%; height:10px; background:#070d12; }
.harbor-phone-status svg { width:14px; height:12px; }
.harbor-app { display:flex; align-items:center; gap:8px; padding:9px 12px; border-bottom:1px solid #3c4c54; font-size:14px; }
.harbor-app svg { width:23px; height:23px; }
.harbor-messages { padding:14px 12px; font-size:clamp(13px,1.2vw,16px); line-height:1.4; }
.harbor-bubble { width:fit-content; margin:12px 0 0 auto; padding:6px 10px; border-radius:11px 11px 3px 11px; background:#45362e; }
.harbor-bubble { transition:opacity .2s, transform .26s cubic-bezier(.2,1.25,.4,1); }
.harbor.awaiting-reply .harbor-bubble { opacity:0; transform:translateY(6px); }
.harbor-composer { display:flex; align-items:center; justify-content:space-between; gap:4px; margin:auto 8px 6px; padding:4px 5px 4px 9px; border:1px solid #41505a; border-radius:12px; font-size:12px; color:#e5e9e6; }
.harbor-composer span:empty::before { content:'Reply…'; color:#9aabb6; }
.harbor-composer b { display:grid; place-items:center; width:20px; height:20px; line-height:1; border-radius:50%; background:#35414a; color:#b6c4c9; }
.harbor.answering .harbor-composer { border-color:#ba906d; }
.harbor.answering .harbor-composer b { background:#ffbe88; color:#1c2934; }
.harbor-phone { transition:box-shadow .4s; }
.harbor.answering .harbor-phone { box-shadow:0 0 0 2px #ffc38a7a,0 0 22px #ffb87b30,0 12px 30px #07101c65; }
.harbor:not(.answering) .harbor-composer span:not(:empty) { visibility:hidden; }
.harbor-home { margin:0 auto 7px; width:32%; height:3px; border-radius:3px; background:#abb6b9; }
.harbor [data-at].unwritten { visibility:hidden; }
.harbor .typing::after { content:'▍'; color:#ffbe88; }
@media(max-width:650px){
    .harbor { height:480px; margin-top:30px; }
    .harbor-term { left:6%; top:5%; width:83%; max-width:350px; }
    .harbor-log { min-height:132px; padding:10px 13px; }
    .harbor-command { margin-bottom:5px; }
    .harbor-term .harbor-port { left:33%; top:100%; }
    .harbor-phone { top:47%; right:6%; width:48%; max-width:193px; height:44%; border-radius:20px; }
}
`;

export const harborJs = `
(() => {
    const scene = document.querySelector('.harbor');
    if (!scene) return;
    ${harborArtJs}
    const route = scene.querySelector('.harbor-route path');
    const courier = scene.querySelector('.harbor-courier');
    const lines = Array.from(scene.querySelectorAll('[data-at]'), node => ({
        node, text: node.textContent, start: Number(node.dataset.at), until: Number(node.dataset.until || Infinity)
    }));
    const motion = matchMedia('(prefers-reduced-motion: reduce)');
    const end = 13;
    let time = motion.matches ? end : 0, visible = false, frame = 0, previous = null, length = 0;
    function layout() {
        if (!visible) return;
        const box = scene.getBoundingClientRect();
        const [a, b] = Array.from(scene.querySelectorAll('.harbor-port'), port => {
            const p = port.getBoundingClientRect();
            return { x: p.left - box.left, y: p.top - box.top };
        });
        const dx = b.x - a.x, dy = b.y - a.y;
        route.setAttribute('d', 'M' + a.x + ' ' + a.y + ' C' + (a.x + dx * .4) + ' ' + (a.y + dy * .3 - 45) + ' ' + (a.x + dx * .6) + ' ' + (b.y + 28) + ' ' + b.x + ' ' + b.y);
        length = route.getTotalLength();
        sizeHarbor(scene.clientWidth, scene.clientHeight);
        paint();
    }
    function paint() {
        const speaker = time < 5.4 ? 'terminal' : time < 9.4 ? 'phone' : time < 11.5 ? 'terminal' : '';
        if (scene.parentElement.dataset.speaker !== speaker) scene.parentElement.dataset.speaker = speaker;
        scene.classList.toggle('awaiting-reply', time < 7.9);
        scene.classList.toggle('answering', time >= 5.4 && time < 7.9);
        lines.forEach(({ node, text, start, until }) => {
            const count = Math.min(text.length, Math.max(0, Math.ceil((time - start) * 23)));
            const part = time < until ? text.slice(0, count) : '';
            if (node.textContent !== part) node.textContent = part;
            node.classList.toggle('unwritten', until === Infinity && time < start);
            node.classList.toggle('typing', time >= start && count < text.length);
        });
        // The same messenger carries the question out and the answer back.
        const out = time >= 3.9 && time < 5.4, back = time >= 7.9 && time < 9.4;
        courier.style.opacity = out || back ? '1' : '0';
        if (out || back) {
            const t = (time - (out ? 3.9 : 7.9)) / 1.5;
            const u = t * t * (3 - 2 * t);
            const p = route.getPointAtLength((out ? u : 1 - u) * length);
            courier.style.transform = 'translate(' + p.x + 'px,' + p.y + 'px) rotate(' + ((out ? -1 : 1) * 8 * Math.sin(t * Math.PI * 2)) + 'deg)';
        }
        drawHarbor(time, Math.min(1, Math.max(0, (time - 10.4) * 1.8)));
    }
    function tick(now) {
        frame = 0;
        if (previous !== null) time = Math.min(end, time + (now - previous) / 1000);
        previous = now;
        paint();
        if (time < end) frame = requestAnimationFrame(tick);
        else sync();
    }
    function sync() {
        cancelAnimationFrame(frame); previous = null;
        if (visible && !document.hidden && !motion.matches && time < end) frame = requestAnimationFrame(tick);
    }
    motion.addEventListener('change', () => { time = motion.matches ? end : 0; paint(); sync(); });
    document.addEventListener('visibilitychange', sync);
    new ResizeObserver(layout).observe(scene);
    new IntersectionObserver(([entry]) => {
        visible = entry.isIntersecting && entry.intersectionRatio >= .12;
        if (!visible && !motion.matches && time >= end) time = 0;
        if (visible) layout();
        sync();
    }, { threshold: .12 }).observe(scene);
    sync();
})();
`;
