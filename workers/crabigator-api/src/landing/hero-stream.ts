// Replay captured output on each desktop and phone pair: rows marked `stream`
// arrive on the desktop, then on the phone just behind. A pair out of view
// holds its rows and its flow scenes (see flow-player.ts).
export const heroStreamCss = `
.app-working::after { content: '⠋'; color: #5fff5f; font-size: 1.2em; animation: working-dots 1s step-end infinite; }
@keyframes working-dots { 0% { content: '⠋'; } 10% { content: '⠙'; } 20% { content: '⠹'; } 30% { content: '⠸'; } 40% { content: '⠼'; } 50% { content: '⠴'; } 60% { content: '⠦'; } 70% { content: '⠧'; } 80% { content: '⠇'; } 90% { content: '⠏'; } }
@media (prefers-reduced-motion: reduce) {
    .app-working::after { animation: none; }
}
`;

export const heroStreamJs = `
(() => {
    if (!Element.prototype.animate) return;
    const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
    document.querySelectorAll('.device-pair').forEach(figure => {
        const devices = [figure.querySelectorAll('.term-capture .stream'), figure.querySelectorAll('.phone-terminal .stream')];
        // Long runs of rows arrive faster, so the whole set lands within about five seconds.
        const step = Math.min(Number(figure.dataset.step || 600), 4800 / Math.max(1, devices[0].length));
        const duration = Math.max(9000, 900 + devices[0].length * step + 3500);
        let animations = [];
        let visible = false;

        function updatePlayback() {
            const running = visible && !document.hidden;
            animations.forEach(animation => running ? animation.play() : animation.pause());
            figure.classList.toggle('paused', !running);
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

            updatePlayback();
        }

        document.addEventListener('visibilitychange', updatePlayback);
        reducedMotion.addEventListener('change', () => {
            animations.forEach(animation => animation.cancel());
            animations = [];
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
