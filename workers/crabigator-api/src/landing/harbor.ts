import { iconCrabigator, iconMicrophone, iconWifi } from './icons';
import { harborArtJs } from './harbor-art';

const question = 'What color should it glow?';
const beaconColors = [
    { reply: 'Warm amber.', rgb: [255, 184, 98] },
    { reply: 'Sea glass.', rgb: [108, 235, 199] },
    { reply: 'Glacier blue.', rgb: [126, 201, 255] },
    { reply: 'Soft violet.', rgb: [196, 160, 255] },
];
const answer = beaconColors[0].reply;

// The completed exchange is also the no-script and reduced-motion view.
export const harborHtml = `
<figure class="harbor" aria-label="A terminal and a phone share one session. The terminal asks what color to light the lighthouse. Each reply from the phone sets the color of the revolving beacon, starting with warm amber.">
    <div class="harbor-world" aria-hidden="true"><canvas class="harbor-art"></canvas><canvas class="harbor-water"></canvas><i class="harbor-lamp"></i></div>
    <svg class="harbor-route" aria-hidden="true"><path/></svg>
    <div class="harbor-courier" aria-hidden="true">${iconCrabigator}</div>
    <div class="harbor-term" aria-hidden="true">
        <div class="harbor-term-bar"><span class="window-dots"><i></i><i></i><i></i></span><span>crabigator — claude</span></div>
        <div class="harbor-log">
            <div class="harbor-welcome"><span class="harbor-clawd"> ▐▛███▜▌
▝▜█████▛▘
  ▘▘ ▝▝</span><div><strong>Claude Code</strong><small>Opus · ~/harbor</small></div></div>
            <p data-at=".4" class="harbor-prompt">Light the harbor beacon.</p>
            <p data-at="2.5" class="harbor-assistant">${question}</p>
            <p data-reply data-at="9.4" class="harbor-prompt">${answer}</p>
            <p data-at="10.4" class="harbor-assistant harbor-done">Beacon lit.</p>
        </div>
        <div class="harbor-input">❯ <i></i></div>
        <i class="harbor-port"></i>
    </div>
    <div class="harbor-phone" aria-hidden="true">
        <div class="harbor-phone-status">9:41 <i></i>${iconWifi}</div>
        <div class="harbor-app"><b>‹</b><div><strong>Harbor light</strong><small>ClayBook · Live</small></div><span>ᴀA</span></div>
        <div class="harbor-messages">
            <p data-at="5.4" class="harbor-assistant">${question}</p>
            <p data-reply data-at="7.9" class="harbor-prompt">${answer}</p>
            <p data-at="11.2" class="harbor-assistant harbor-done">Beacon lit.</p>
        </div>
        <div class="harbor-composer">${iconMicrophone}<span data-reply data-at="6.9" data-until="7.9"></span><b>↑</b></div>
        <div class="harbor-home"></div><i class="harbor-port"></i>
    </div>
</figure>`;

export const harborCss = `
.section .harbor-title > span { display:inline; color:inherit; transition:color .55s ease-out; }
[data-speaker="terminal"] .harbor-title span:first-child, [data-speaker="phone"] .harbor-title span:last-child { color:#ffcdab; }
.harbor { --beacon:rgb(${beaconColors[0].rgb.join(" ")}); position:relative; isolation:isolate; overflow:hidden; height:clamp(390px,36vw,470px); margin:44px auto 0; border-radius:16px; border:1px solid #485562; background:linear-gradient(#192a41,#73616b 64%,#28404b 76%,#112833 77%); }
.harbor-world,.harbor-world canvas,.harbor-route { position:absolute; inset:0; width:100%; height:100%; pointer-events:none; }
.harbor-art { image-rendering:pixelated; }
.harbor-lamp { position:absolute; width:7px; height:5px; transform:translate(-50%,-50%); background:color-mix(in srgb,var(--beacon),white 32%); box-shadow:0 0 9px 4px var(--beacon),0 0 38px 14px color-mix(in srgb,var(--beacon) 50%,transparent); }
.harbor-lamp::before { content:''; position:absolute; left:50%; top:50%; width:var(--beam-length,0px); height:var(--beam-width,0px); transform-origin:0 50%; transform:translateY(-50%) var(--sweep,scaleX(1)); clip-path:polygon(0 49%,100% 0,100% 100%,0 51%); background:linear-gradient(90deg,color-mix(in srgb,var(--beacon) 70%,transparent),transparent); }
.harbor-route path { stroke:#ffcd9c; stroke-width:1.25; stroke-dasharray:2 6; fill:none; opacity:.55; }
.harbor-courier { position:absolute; top:0; left:0; width:26px; height:26px; margin:-13px; opacity:0; filter:drop-shadow(0 0 7px #ffbe80); pointer-events:none; }
.harbor-courier svg { display:block; width:100%; }
.harbor-term,.harbor-phone { position:absolute; color:#ecede8; text-align:left; box-shadow:0 12px 30px #07101c65,0 1px 0 #ffffff20 inset; }
.harbor-term { left:5%; top:17%; width:39%; max-width:436px; border:1px solid #7d8188; background:#191918; border-radius:11px; font-family:'JetBrains Mono',monospace; }
.harbor-term-bar { position:relative; display:flex; justify-content:center; align-items:center; border-bottom:1px solid #454746; background:#303231; padding:7px 0; border-radius:10px 10px 0 0; color:#c2c4c0; font-size:11px; }
.harbor-term-bar .window-dots { position:absolute; left:12px; }
.harbor-log { padding:12px 14px 7px; font-size:clamp(12px,1.15vw,15px); line-height:1.45; }
.harbor-welcome { display:flex; align-items:center; gap:12px; margin-bottom:9px; }
.harbor-clawd { white-space:pre; font:11px/.98 monospace; color:#df9679; }
.harbor-welcome strong,.harbor-welcome small { display:block; }
.harbor-welcome small { color:#aaa9a4; font-size:.8em; }
.harbor-log p,.harbor-messages p { position:relative; margin:3px 0; min-height:calc(1lh + 4px); padding:2px 5px 2px 1.55em; }
.harbor-prompt { background:#343432; }
.harbor-prompt::before,.harbor-assistant::before { position:absolute; left:.4em; }
.harbor-prompt::before { content:'❯'; color:#d3d0c8; }
.harbor-assistant::before { content:'●'; color:#d3d0c8; font-size:.85em; top:.32em; }
.harbor-done::before,.harbor [data-reply]:not(:empty) { color:var(--beacon); }
.harbor-log p,.harbor-messages p { opacity:var(--transcript,1); }
.harbor p[data-full-line] { display:grid; }
.harbor p[data-full-line] > span,.harbor p[data-full-line]::after { grid-area:1 / 1; }
.harbor p[data-full-line]::after { content:attr(data-full-line); visibility:hidden; }
.harbor-input { color:#c9c9c3; margin:0 14px 10px; padding:3px 5px; border-block:1px solid #535351; font-size:12px; }
.harbor-input i { display:inline-block; width:.6em; height:1em; vertical-align:-.12em; background:#b7b6b0; }
.harbor-port { position:absolute; left:100%; top:56%; }
.harbor-phone { right:7%; top:10%; width:22%; max-width:212px; height:73%; max-height:305px; background:#101c26; border:4px solid #141b22; outline:1.5px solid #828888; border-radius:26px; display:flex; flex-direction:column; }
.harbor-phone .harbor-port { left:0; top:52%; }
.harbor-phone-status { display:flex; align-items:center; justify-content:space-between; padding:10px 14px 5px; font-size:10px; font-weight:650; }
.harbor-phone-status i { border-radius:8px; width:30%; height:10px; background:#070d12; }
.harbor-phone-status svg { width:14px; height:12px; }
.harbor-app { display:flex; align-items:center; gap:6px; padding:5px 10px 7px; border-bottom:1px solid #353c45; color:#67b4fd; }
.harbor-app b { font-size:20px; font-weight:500; }
.harbor-app strong,.harbor-app small { display:block; line-height:1.25; }
.harbor-app strong { font-size:13px; }
.harbor-app small { color:#91ce96; font-size:10px; }
.harbor-app > span { margin-left:auto; color:#aab3bc; font-size:12px; }
.harbor-messages { flex:1; padding:8px 6px; background:#101312; font:clamp(11px,1vw,13px)/1.4 'JetBrains Mono',monospace; }
.harbor-composer { display:flex; align-items:center; gap:5px; margin:6px 8px; font-size:12px; color:#e5e9e6; }
.harbor-composer svg { width:15px; flex:none; color:#a5aeb7; }
.harbor-composer span { flex:1; border:1px solid #41505a; border-radius:11px; padding:4px 6px; }
.harbor-composer span:empty::before { content:'Reply…'; color:#9aabb6; }
.harbor-composer b { display:grid; place-items:center; width:20px; height:20px; line-height:1; border-radius:50%; background:#35414a; color:#b6c4c9; }
.harbor.answering .harbor-composer span { border-color:var(--beacon); }
.harbor.answering .harbor-composer b { background:var(--beacon); color:#1c2934; }
.harbor-phone { transition:box-shadow .4s; }
.harbor.answering .harbor-phone { box-shadow:0 0 0 2px color-mix(in srgb,var(--beacon) 46%,transparent),0 0 22px color-mix(in srgb,var(--beacon) 18%,transparent),0 12px 30px #07101c65; }
.harbor-home { margin:0 auto 7px; width:32%; height:3px; border-radius:3px; background:#abb6b9; }
.harbor [data-at].unwritten { visibility:hidden; }
.harbor .typing > span::after,.harbor-composer .typing::after { position:absolute; content:'▍'; color:#ffbe88; }
@media(max-width:650px){
    .harbor { display:flex; flex-direction:column; justify-content:space-between; gap:24px; height:auto; min-height:540px; padding:16px 6% 22px; margin-top:30px; }
    .harbor-term { position:relative; inset:auto; width:94%; max-width:350px; }
    .harbor-log { padding:8px 10px 3px; }
    .harbor-welcome { margin-bottom:5px; }
    .harbor-input { margin:0 10px 6px; }
    .harbor-term .harbor-port { left:33%; top:100%; }
    .harbor-phone { position:relative; inset:auto; align-self:flex-end; width:59%; max-width:193px; height:252px; border-radius:20px; }
}
`;

export const harborJs = `
(() => {
    const scene = document.querySelector('.harbor');
    if (!scene) return;
    ${harborArtJs}
    const route = scene.querySelector('.harbor-route path');
    const courier = scene.querySelector('.harbor-courier');
    const colors = ${JSON.stringify(beaconColors)};
    const lines = Array.from(scene.querySelectorAll('[data-at]'), node => {
        const original = node.textContent;
        // Keep the full line's space while it types or dissolves, even if it wraps.
        const output = node.matches('p') ? document.createElement('span') : node;
        if (output !== node) {
            node.replaceChildren(output);
            node.dataset.fullLine = original;
        }
        return { node, output, text: original, start: Number(node.dataset.at), until: Number(node.dataset.until || Infinity) };
    });
    function setColor(index) {
        const { rgb, reply } = colors[index];
        scene.style.setProperty('--beacon', 'rgb(' + rgb.join(' ') + ')');
        for (const line of lines) {
            if (!line.node.hasAttribute('data-reply')) continue;
            line.text = reply;
            if (line.output !== line.node) line.node.dataset.fullLine = reply;
        }
    }
    const motion = matchMedia('(prefers-reduced-motion: reduce)');
    const period = 22, still = 14;
    let elapsed = motion.matches ? still : 0, time = elapsed, colorIndex = 0;
    let visible = false, frame = 0, previous = null, length = 0;
    setColor(0);
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
        scene.classList.toggle('answering', time >= 5.4 && time < 7.9);
        lines.forEach(({ node, output, text, start, until }) => {
            const count = Math.min(text.length, Math.max(0, Math.ceil((time - start) * 23)));
            const part = time < until ? text.slice(0, count) : '';
            if (output.textContent !== part) output.textContent = part;
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
        // Dissolve the conversation and the lamp into the empty opening scene.
        const fade = Math.min(1, Math.max(0, (period - time) / 2.2));
        const opacity = fade * fade * (3 - 2 * fade);
        scene.style.setProperty('--transcript', String(opacity));
        drawHarbor(elapsed, opacity * Math.min(1, Math.max(0, (time - 10.4) * 1.8)), colors[colorIndex].rgb);
    }
    function tick(now) {
        if (previous !== null) elapsed += (now - previous) / 1000;
        previous = now;
        time = elapsed % period;
        const index = Math.floor(elapsed / period) % colors.length;
        if (index !== colorIndex) { colorIndex = index; setColor(index); }
        paint();
        frame = requestAnimationFrame(tick);
    }
    function sync() {
        cancelAnimationFrame(frame); previous = null;
        if (visible && !document.hidden && !motion.matches) frame = requestAnimationFrame(tick);
    }
    motion.addEventListener('change', () => {
        elapsed = time = motion.matches ? still : 0;
        colorIndex = 0;
        setColor(0); paint(); sync();
    });
    document.addEventListener('visibilitychange', sync);
    const resize = new ResizeObserver(layout);
    resize.observe(scene);
    resize.observe(scene.querySelector('.harbor-term'));
    new IntersectionObserver(([entry]) => {
        visible = entry.isIntersecting && entry.intersectionRatio >= .12;
        if (visible) layout();
        sync();
    }, { threshold: .12 }).observe(scene);
    sync();
})();
`;
