// Replay the captured output on both devices, with the phone just behind the desktop.
export const heroStreamCss = `
.hero-stream-toggle { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; width: 30px; height: 30px; padding: 0; border: 0; border-radius: 50%; background: transparent; color: var(--muted); font-size: 14px; }
.hero-stream-toggle:hover { color: var(--ink); background: var(--surface); }
.hero-stream-toggle[hidden] { display: none; }
.connection-line { position: relative; flex: 1; overflow: hidden; height: 6px; background: linear-gradient(#52616b,#52616b) center / 100% 1px no-repeat; }
.stream-packet { position: absolute; left: 0; top: 0; width: 6px; height: 6px; border-radius: 50%; background: var(--peach); box-shadow: -8px 0 10px #ffb98a66; }
`;

export const heroStreamJs = `
(() => {
    const hero = document.querySelector('.hero-screens');
    if (!hero || !Element.prototype.animate) return;
    const toggle = hero.querySelector('.hero-stream-toggle');
    const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
    const duration = 9000;
    let animations = [];
    let visible = false;
    let paused = false;

    function updatePlayback() {
        const running = visible && !document.hidden && !paused;
        animations.forEach(animation => running ? animation.play() : animation.pause());
        toggle.textContent = paused ? '▶' : 'Ⅱ';
        toggle.setAttribute('aria-label', paused ? 'Play terminal animation' : 'Pause terminal animation');
    }

    function start() {
        if (animations.length || reducedMotion.matches) return;
        const startTime = document.timeline.currentTime;
        function animate(element, frames) {
            const animation = element.animate(frames, { duration, iterations: Infinity });
            animation.startTime = startTime;
            animations.push(animation);
        }

        ['.desktop-capture', '.phone-terminal'].forEach((selector, device) => {
            const lines = hero.querySelector(selector).children;
            // These rows are the five captured response lines and the completion timestamp.
            [2, 3, 4, 5, 6, 8].forEach((row, index) => {
                const arrival = (900 + index * 600 + device * 240) / duration;
                const element = lines[row];
                animate(element, [
                    { opacity: 0, backgroundColor: 'transparent', offset: 0 },
                    { opacity: 0, backgroundColor: '#ffb98a24', offset: arrival },
                    { opacity: 1, backgroundColor: '#ffb98a24', offset: arrival + .025 },
                    { opacity: 1, backgroundColor: 'transparent', offset: arrival + .075 },
                    { opacity: 1, backgroundColor: 'transparent', offset: .94 },
                    { opacity: 0, backgroundColor: 'transparent', offset: 1 },
                ]);
            });
        });

        const packets = [{ left: '0%', opacity: 0, offset: 0 }];
        const received = [{ color: '#8bd78d', offset: 0 }];
        for (let index = 0; index < 6; index++) {
            const departure = (900 + index * 600) / duration;
            packets.push(
                { left: '0%', opacity: 0, offset: departure },
                { left: '4%', opacity: 1, offset: departure + .003 },
                { left: 'calc(100% - 6px)', opacity: 1, offset: departure + .027 },
                { left: 'calc(100% - 6px)', opacity: 0, offset: departure + .04 },
            );
            received.push(
                { color: '#8bd78d', offset: departure + .026 },
                { color: '#ffb98a', offset: departure + .035 },
                { color: '#8bd78d', offset: departure + .06 },
            );
        }
        packets.push({ left: 'calc(100% - 6px)', opacity: 0, offset: 1 });
        received.push({ color: '#8bd78d', offset: 1 });
        animate(hero.querySelector('.stream-packet'), packets);
        animate(hero.querySelector('.phone-app-bar div > span'), received);
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
    }, { threshold: .15 }).observe(hero);
})();
`;
