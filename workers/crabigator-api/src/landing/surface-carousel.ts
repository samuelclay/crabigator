// The active tab's underline is the clock. Pausing it preserves the dwell time.
export const surfaceCarouselJs = `
(() => {
    const section = document.getElementById('apps');
    const tablist = section.querySelector('.surface-tabs');
    const deck = section.querySelector('.surface-deck');
    const tabs = Array.from(tablist.querySelectorAll('[role="tab"]'));
    const panels = tabs.map(tab => document.getElementById(tab.getAttribute('aria-controls')));
    const reduced = matchMedia('(prefers-reduced-motion: reduce)');
    let current = 0, visible = false, clock, slides = [];
    function playClock() {
        if (!clock) return;
        const reading = panels[current].contains(document.activeElement) || tablist.querySelector(':focus-visible');
        if (visible && !document.hidden && !reading && !reduced.matches) clock.play();
        else clock.pause();
    }
    function startClock() {
        clock?.cancel();
        clock = tabs[current].querySelector('.surface-progress').animate(
            [{ transform: 'scaleX(0)' }, { transform: 'scaleX(1)' }],
            { duration: current === 0 ? 12500 : 8000, fill: 'forwards' });
        clock.onfinish = () => select((current + 1) % tabs.length, 1);
        playClock();
    }
    function select(index, direction, focus = false) {
        if (index === current) return;
        const moving = !reduced.matches;
        // Retarget the visible panels from their actual positions on a rapid tap.
        const positions = panels.map(panel => {
            if (panel.hidden || !moving) return null;
            const style = getComputedStyle(panel);
            return { transform: style.transform, opacity: style.opacity };
        });
        slides.forEach(slide => { slide.onfinish = null; slide.cancel(); });
        slides = [];
        current = index;
        panels.forEach((panel, i) => {
            const selected = i === index;
            tabs[i].setAttribute('aria-selected', String(selected));
            tabs[i].tabIndex = selected ? 0 : -1;
            panel.inert = !selected;
            panel.hidden = !selected && !positions[i];
            if (panel.hidden || !moving) return;
            const start = positions[i] || { transform: 'translateX(' + 42 * direction + '%)', opacity: 0 };
            const end = selected ? { transform: 'translateX(0)', opacity: 1 }
                : { transform: 'translateX(' + (-28 * direction) + '%)', opacity: 0 };
            const slide = panel.animate([start, end], { duration: 650, easing: 'cubic-bezier(.22,1,.36,1)' });
            if (!selected) slide.onfinish = () => { panel.hidden = true; };
            slides.push(slide);
        });
        if (focus) tabs[index].focus({ preventScroll: true });
        startClock();
    }
    tabs.forEach((tab, index) => {
        const line = document.createElement('i');
        line.className = 'surface-progress';
        line.setAttribute('aria-hidden', 'true');
        tab.append(line);
        panels[index].hidden = panels[index].inert = index !== current;
        tab.onclick = () => select(index, Math.sign(index - current));
        tab.onkeydown = event => {
            const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
            if (step) select((index + step + tabs.length) % tabs.length, step, true);
            else if (event.key === 'Home') select(0, -1, true);
            else if (event.key === 'End') select(tabs.length - 1, 1, true);
            else return;
            event.preventDefault();
        };
    });
    let touch;
    deck.addEventListener('pointerdown', event => {
        if (event.pointerType === 'touch' && event.isPrimary) touch = { x: event.clientX, y: event.clientY };
    });
    deck.addEventListener('pointercancel', () => { touch = null; });
    deck.addEventListener('pointerup', event => {
        if (!touch || !event.isPrimary) return;
        const dx = event.clientX - touch.x, dy = event.clientY - touch.y;
        touch = null;
        if (Math.abs(dx) < 45 || Math.abs(dx) < 1.5 * Math.abs(dy)) return;
        const step = -Math.sign(dx);
        select((current + step + tabs.length) % tabs.length, step);
    });
    section.addEventListener('focusin', playClock);
    section.addEventListener('focusout', () => queueMicrotask(playClock));
    document.addEventListener('visibilitychange', playClock);
    reduced.addEventListener('change', () => {
        if (reduced.matches) slides.forEach(slide => slide.finish());
        playClock();
    });
    tablist.hidden = false;
    new IntersectionObserver(([entry]) => {
        visible = entry.intersectionRatio >= .35;
        playClock();
    }, { threshold: .35 }).observe(deck);
    startClock();
})();
`;

export const surfaceCarouselCss = `
.surface-tabs button { position:relative; }
.surface-progress { position:absolute; bottom:3px; height:2px; left:16%; right:16%; background:#4a626d; border-radius:2px; transform:scaleX(0); transform-origin:left; }
.surface-tabs:not([hidden]) + .surface-deck { display:grid; overflow:clip; touch-action:pan-y pinch-zoom; }
.surface-deck .surface-panel { min-width:0; grid-area:1/1; }
.surface-deck figure { --preview-height:clamp(440px,74svh,680px); height:var(--preview-height); display:grid; grid-template-columns:minmax(0,1fr); grid-template-rows:minmax(0,1fr); place-items:center; padding:16px; background:#1b2731; border-radius:12px; }
.surface-deck .notification-phone,.surface-deck .android-frame { width:min(100%,calc((var(--preview-height) - 40px)/2.2)); }
.surface-deck .tablet-frame { width:min(100%,calc((var(--preview-height) - 50px)*1.45)); }
.surface-deck .web-product { width:min(100%,calc((var(--preview-height) - 32px)*1.68)); }
@media(prefers-reduced-motion:reduce){ .surface-progress { display:none; } }
`;
