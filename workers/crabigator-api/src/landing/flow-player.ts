// Plays the homepage's flow scenes from flow-data.ts, one canvas per scene, at
// each scene's own frame rate. Canvases with the same data key (a terminal and
// the phone mirroring it) read one clock, so they show the same frame. Each
// canvas sits over a still SVG of the first frame, which stays visible until
// the canvas has drawn, with reduced motion, and without JavaScript.
import { flowData } from './flow-data';

export const flowPlayerJs = `
(() => {
    const FLOW = ${JSON.stringify(flowData)};
    const canvases = document.querySelectorAll('canvas[data-flow]');
    if (!canvases.length || matchMedia('(prefers-reduced-motion: reduce)').matches) return;

    // flow's 2x4 masks (bit x + 2y), as in its desktop renderer.
    const QUAD = [0x20, 0x2598, 0x259D, 0x2580, 0x2596, 0x258C, 0x259E, 0x259B, 0x2597, 0x259A, 0x2590, 0x259C, 0x2584, 0x2599, 0x259F, 0x2588];
    const SHADE = { 0x2591: .3, 0x2592: .55, 0x2593: .8 };
    function coverage(cp) {
        if (cp === 0x20 || cp === 0xA0) return 0;
        const q = QUAD.indexOf(cp);
        if (q >= 0) return (q & 1 ? 5 : 0) | (q & 2 ? 10 : 0) | (q & 4 ? 80 : 0) | (q & 8 ? 160 : 0);
        if (cp > 0x2580 && cp < 0x2588) return (0xFF << ((4 - Math.floor((cp - 0x2580) / 2 + .5)) * 2)) & 0xFF;
        if (cp > 0x2588 && cp < 0x2590) { const c = Math.floor((0x2590 - cp) / 4 + .5); return c >= 2 ? 0xFF : c === 1 ? 0x55 : 0; }
        if (cp >= 0x2800 && cp <= 0x28FF) {
            const b = cp - 0x2800, dots = [[1, 2, 4, 64], [8, 16, 32, 128]];
            let mask = 0;
            for (let x = 0; x < 2; x++) for (let y = 0; y < 4; y++) if (b & dots[x][y]) mask |= 1 << (x + 2 * y);
            return mask;
        }
        return -1;
    }
    const hex = c => '#' + c.toString(16).padStart(6, '0');
    function mix(a, b, k) {
        const ch = (v, s) => (v >> s) & 255, m = s => Math.round(ch(a, s) + (ch(b, s) - ch(a, s)) * k);
        return (m(16) << 16) | (m(8) << 8) | m(0);
    }

    const scenes = {};
    for (const key in FLOW) {
        const d = FLOW[key];
        scenes[key] = { d, masks: d.g.map(coverage), cells: null, at: -1 };
    }
    // The scene's cells at frame n, replaying its changes from the last frame used.
    function cellsAt(scene, n) {
        const { d } = scene;
        if (!scene.cells || n < scene.at) {
            scene.cells = Int32Array.from(d.f[0]);
            scene.at = 0;
        }
        for (let i = scene.at + 1; i <= n; i++) {
            const f = d.f[i];
            for (let j = 0; j < f.length; j += 4) scene.cells.set([f[j + 1], f[j + 2], f[j + 3]], f[j] * 3);
        }
        scene.at = n;
        return scene.cells;
    }

    function draw(item, scene, cells) {
        const { canvas, ctx } = item, { d, masks } = scene;
        const w = Math.round(canvas.clientWidth * devicePixelRatio), h = Math.round(canvas.clientHeight * devicePixelRatio);
        if (!w || !h) return;
        if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
        const cols = d.c * 2, rows = d.r * 4, sx = w / cols, sy = h / rows;
        // Colour every sub-block, then fill runs of one colour along each sub-row.
        const sub = new Int32Array(cols * rows).fill(-1), text = [];
        for (let i = 0; i < d.c * d.r; i++) {
            const g = cells[i * 3], fg = d.k[cells[i * 3 + 1]], bg = d.k[cells[i * 3 + 2]];
            const cp = d.g[g], cx = (i % d.c) * 2, cy = Math.floor(i / d.c) * 4;
            let mask = masks[g];
            for (let y = 0; y < 4; y++) for (let x = 0; x < 2; x++) {
                let color = bg;
                if (cp in SHADE) color = fg < 0 ? bg : mix(bg < 0 ? 0x0a0f17 : bg, fg, SHADE[cp]);
                else if (mask > 0 && mask & (1 << (x + 2 * y))) color = fg < 0 ? bg : fg;
                sub[(cy + y) * cols + cx + x] = color;
            }
            if (mask < 0 && fg >= 0 && cp > 0x20) text.push([cx + 1, cy + 3, fg, cp]);
        }
        ctx.clearRect(0, 0, w, h);
        for (let y = 0; y < rows; y++) {
            const y0 = Math.round(y * sy), y1 = Math.round((y + 1) * sy);
            for (let x = 0; x < cols;) {
                const color = sub[y * cols + x];
                let end = x + 1;
                while (end < cols && sub[y * cols + end] === color) end++;
                if (color >= 0) {
                    ctx.fillStyle = hex(color);
                    const x0 = Math.round(x * sx);
                    ctx.fillRect(x0, y0, Math.round(end * sx) - x0, y1 - y0);
                }
                x = end;
            }
        }
        if (text.length) {
            ctx.textAlign = 'center';
            ctx.font = Math.round(sy * 3.1) + 'px JetBrains Mono, monospace';
            for (const [x, y, fg, cp] of text) { ctx.fillStyle = hex(fg); ctx.fillText(String.fromCodePoint(cp), x * sx, y * sy); }
        }
        if (!item.shown) {
            item.shown = true;
            const still = canvas.previousElementSibling;
            if (still && still.tagName.toLowerCase() === 'svg') still.style.visibility = 'hidden';
        }
    }

    const items = Array.from(canvases, canvas => ({ canvas, ctx: canvas.getContext('2d'), key: canvas.dataset.flow, visible: false, frame: -1, shown: false }))
        .filter(item => scenes[item.key]);
    const observer = new IntersectionObserver(entries => {
        for (const entry of entries) {
            const item = items.find(i => i.canvas === entry.target);
            if (item) item.visible = entry.isIntersecting;
        }
    });
    items.forEach(item => observer.observe(item.canvas));

    const start = performance.now();
    function tick(now) {
        if (!document.hidden) {
            const byKey = {};
            for (const item of items) {
                // A paused desktop and phone pair holds its frame.
                if (!item.visible || item.canvas.closest('.paused')) continue;
                const scene = scenes[item.key], d = scene.d;
                const n = Math.floor((now - start) / 1000 * d.fps) % d.f.length;
                if (n === item.frame && item.shown) continue;
                byKey[item.key] = byKey[item.key] || cellsAt(scene, n).slice();
                draw(item, scene, byKey[item.key]);
                item.frame = n;
            }
        }
        requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
})();
`;
