// Replay captured output on each desktop and phone pair: rows marked `stream`
// arrive on the desktop, then on the phone just behind. Every flow scene has
// twelve frames (each svg[style]), shown a quarter second apart in a 3s loop.
const FLOW_PARENTS = ['.bar-rows', '.app-flow', '.web-widgets-flow', '.scene-frames'];
const flowFrame = (rest: string) => FLOW_PARENTS.map(parent => parent + ' > svg' + rest).join(', ');
const frameDelays = Array.from({ length: 12 }, (_, i) =>
    flowFrame(':nth-of-type(' + (i + 1) + ')') + ' { animation-delay: ' + (i * .25 - 3) + 's; }').join('\n');
export const heroStreamCss = `
.stream-toggle { display: inline-flex; align-items: center; justify-content: center; width: 30px; height: 30px; padding: 0; border: 0; border-radius: 50%; background: transparent; color: var(--muted); font-size: 14px; }
.stream-toggle:hover { color: var(--ink); background: var(--surface); }
.stream-toggle[hidden] { display: none; }
${flowFrame('[style]')} { animation: flow-frame 3s step-end infinite; }
${flowFrame('[style]:not(:first-of-type)')} { opacity: 0; }
${frameDelays}
.paused svg[style] { animation-play-state: paused; }
@keyframes flow-frame { 0% { opacity: 1; } 8.333% { opacity: 0; } 100% { opacity: 0; } }
.app-working::after { content: '⠋'; color: #5fff5f; font-size: 1.2em; animation: working-dots 1s step-end infinite; }
@keyframes working-dots { 0% { content: '⠋'; } 10% { content: '⠙'; } 20% { content: '⠹'; } 30% { content: '⠸'; } 40% { content: '⠼'; } 50% { content: '⠴'; } 60% { content: '⠦'; } 70% { content: '⠧'; } 80% { content: '⠇'; } 90% { content: '⠏'; } }
@media (prefers-reduced-motion: reduce) {
    ${flowFrame('[style]')}, .app-working::after { animation: none; }
}
`;

export const heroStreamJs = `
(() => {
    if (!Element.prototype.animate) return;
    const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
    document.querySelectorAll('.device-pair').forEach(figure => {
        const toggle = figure.querySelector('.stream-toggle');
        const devices = [figure.querySelectorAll('.term-capture .stream'), figure.querySelectorAll('.phone-terminal .stream')];
        // Long runs of rows arrive faster, so the whole set lands within about five seconds.
        const step = Math.min(Number(figure.dataset.step || 600), 4800 / Math.max(1, devices[0].length));
        const duration = Math.max(9000, 900 + devices[0].length * step + 3500);
        let animations = [];
        let visible = false;
        let paused = false;

        function updatePlayback() {
            const running = visible && !document.hidden && !paused;
            animations.forEach(animation => running ? animation.play() : animation.pause());
            figure.classList.toggle('paused', !running);
            toggle.textContent = paused ? '▶' : 'Ⅱ';
            toggle.setAttribute('aria-label', paused ? 'Play animation' : 'Pause animation');
        }

        function start() {
            if (animations.length || reducedMotion.matches) return;
            const startTime = document.timeline.currentTime;
            function animate(element, frames) {
                const animation = element.animate(frames, { duration, iterations: Infinity });
                animation.startTime = startTime;
                animations.push(animation);
            }
            devices.forEach((rows, device) => rows.forEach((row, index) => {
                const arrival = (900 + index * step + device * 240) / duration;
                animate(row, [
                    { opacity: 0, backgroundColor: 'transparent', offset: 0 },
                    { opacity: 0, backgroundColor: '#ffb98a24', offset: arrival },
                    { opacity: 1, backgroundColor: '#ffb98a24', offset: arrival + .025 },
                    { opacity: 1, backgroundColor: 'transparent', offset: arrival + .075 },
                    { opacity: 1, backgroundColor: 'transparent', offset: .94 },
                    { opacity: 0, backgroundColor: 'transparent', offset: 1 },
                ]);
            }));

            toggle.hidden = false;
            updatePlayback();
        }

        toggle.addEventListener('click', () => {
            paused = !paused;
            updatePlayback();
        });
        document.addEventListener('visibilitychange', updatePlayback);
        reducedMotion.addEventListener('change', () => {
            animations.forEach(animation => animation.cancel());
            animations = [];
            toggle.hidden = true;
            if (visible) start();
        });
        new IntersectionObserver(([entry]) => {
            visible = entry.isIntersecting && entry.intersectionRatio >= .15;
            if (visible) start();
            updatePlayback();
        }, { threshold: .15 }).observe(figure);
    });
})();
`;
