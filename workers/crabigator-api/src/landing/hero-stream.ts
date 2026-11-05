// Replay captured output on each desktop and phone pair: rows marked `stream`
// arrive on the desktop, then on the phone just behind. Flow scenes with
// several frames (svg[style] carries each frame's --i) cycle a quarter second
// apart, as they were captured.
export const heroStreamCss = `
.stream-toggle { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; width: 30px; height: 30px; padding: 0; border: 0; border-radius: 50%; background: transparent; color: var(--muted); font-size: 14px; }
.stream-toggle:hover { color: var(--ink); background: var(--surface); }
.stream-toggle[hidden] { display: none; }
.connection-line { position: relative; flex: 1; overflow: hidden; height: 6px; background: linear-gradient(#52616b,#52616b) center / 100% 1px no-repeat; }
.stream-packet { position: absolute; left: 0; top: 0; width: 6px; height: 6px; border-radius: 50%; background: var(--peach); box-shadow: -8px 0 10px #ffb98a66; }
.bar-rows > svg[style], .app-flow > svg[style] { animation: flow-frame 3s step-end infinite; animation-delay: calc(var(--i) * .25s - 3s); }
.bar-rows > svg[style]:not(:first-of-type), .app-flow > svg[style]:not(:first-of-type) { opacity: 0; }
.paused svg[style] { animation-play-state: paused; }
@keyframes flow-frame { 0% { opacity: 1; } 8.333% { opacity: 0; } 100% { opacity: 0; } }
.app-working::after { content: '⠋'; color: #5fff5f; font-size: 1.2em; animation: working-dots 1s step-end infinite; }
@keyframes working-dots { 0% { content: '⠋'; } 10% { content: '⠙'; } 20% { content: '⠹'; } 30% { content: '⠸'; } 40% { content: '⠼'; } 50% { content: '⠴'; } 60% { content: '⠦'; } 70% { content: '⠧'; } 80% { content: '⠇'; } 90% { content: '⠏'; } }
@media (prefers-reduced-motion: reduce) {
    .bar-rows > svg[style], .app-flow > svg[style], .app-working::after { animation: none; }
}
`;

export const heroStreamJs = `
(() => {
    if (!Element.prototype.animate) return;
    const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
    document.querySelectorAll('.device-pair').forEach(figure => {
        const toggle = figure.querySelector('.stream-toggle');
        const devices = [figure.querySelectorAll('.term-capture .stream'), figure.querySelectorAll('.phone-terminal .stream')];
        const step = Number(figure.dataset.step || 600);
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

            // The hero also sends a packet along its connection line for each row.
            const packet = figure.querySelector('.stream-packet');
            const received = figure.querySelector('.phone-app-bar div > span');
            if (packet && received) {
                const packets = [{ left: '0%', opacity: 0, offset: 0 }];
                const colors = [{ color: '#8bd78d', offset: 0 }];
                devices[0].forEach((row, index) => {
                    const departure = (900 + index * step) / duration;
                    packets.push(
                        { left: '0%', opacity: 0, offset: departure },
                        { left: '4%', opacity: 1, offset: departure + .003 },
                        { left: 'calc(100% - 6px)', opacity: 1, offset: departure + .027 },
                        { left: 'calc(100% - 6px)', opacity: 0, offset: departure + .04 },
                    );
                    colors.push(
                        { color: '#8bd78d', offset: departure + .026 },
                        { color: '#ffb98a', offset: departure + .035 },
                        { color: '#8bd78d', offset: departure + .06 },
                    );
                });
                packets.push({ left: 'calc(100% - 6px)', opacity: 0, offset: 1 });
                colors.push({ color: '#8bd78d', offset: 1 });
                animate(packet, packets);
                animate(received, colors);
            }
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
