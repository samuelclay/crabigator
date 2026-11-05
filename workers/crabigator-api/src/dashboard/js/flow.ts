import { flowWasmPath } from '../../assets/flow-wasm';

// Each session's flow scene, beside its module drawer: the desktop's own scene
// code (crates/flow-render as WebAssembly), following the dials the desktop
// sends in "flow" events. The scene is the same; the frames are the browser's.
export const flowJs = `
        const FLOW_WASM_PATH = '${flowWasmPath}';
        const FLOW_TINTS = { normal: 0, smoke: 1, blue: 2 };
        const FLOW_CLEAR = 0x01000000;
        // A new size starts the scene over, and fire takes a couple of seconds to fill in.
        const FLOW_WARM_UP = 30;
        const flowViews = new Map();
        let flowWasm = null;
        let flowWasmLoading = null;
        let flowLooping = false;

        function loadFlowWasm() {
            flowWasmLoading = flowWasmLoading || fetch(FLOW_WASM_PATH)
                .then(response => response.arrayBuffer())
                .then(bytes => WebAssembly.instantiate(bytes, {}))
                .then(result => (flowWasm = result.instance.exports))
                .catch(error => { console.warn('Flow scenes are unavailable', error); flowWasmLoading = null; return null; });
            return flowWasmLoading;
        }

        function applyFlowEvent(sessionId, event) {
            const data = sessions.get(sessionId);
            if (data) data.flow = event;
            const box = document.getElementById('module-flow-' + sessionId);
            if (!box) return;
            let view = flowViews.get(sessionId);
            if (!view) {
                const canvas = box.querySelector('canvas');
                view = { box, canvas, ctx: canvas.getContext('2d'), handle: 0, key: '', event: null, next: 0 };
                flowViews.set(sessionId, view);
            }
            view.event = event;
            box.hidden = !event || !event.scene;
            if (box.hidden) { freeFlow(view); return; }
            loadFlowWasm().then(wasm => {
                if (!wasm || view.event !== event) return;
                const key = event.scene + '|' + event.seed + '|' + (typeof event.hue === 'number' ? event.hue : '');
                if (key !== view.key) {
                    freeFlow(view);
                    const name = new TextEncoder().encode(event.scene).slice(0, 32);
                    new Uint8Array(wasm.memory.buffer, wasm.flow_name_buffer(), 32).set(name);
                    view.handle = wasm.flow_new(wasm.flow_name_buffer(), name.length, event.seed >>> 0,
                        typeof event.hue === 'number' ? event.hue : NaN);
                    view.key = key;
                }
                if (!view.handle) return;
                wasm.flow_set(view.handle, event.level, event.waiting ? 1 : 0, event.night ? 1 : 0,
                    FLOW_TINTS[event.tint] || 0, event.boost || 0);
                if (!flowLooping) { flowLooping = true; requestAnimationFrame(flowTick); }
            });
        }

        function freeFlow(view) {
            if (view.handle && flowWasm) flowWasm.flow_free(view.handle);
            view.handle = 0;
            view.key = '';
            view.columns = 0;
            view.rows = 0;
        }

        // Step each scene on screen at its own pace (70ms busy, 125ms calm).
        function flowTick(now) {
            let live = false;
            flowViews.forEach((view, sessionId) => {
                if (!view.box.isConnected) { freeFlow(view); flowViews.delete(sessionId); return; }
                if (!view.handle) return;
                live = true;
                if (now < view.next || document.hidden) return;
                const width = view.box.clientWidth, height = view.box.clientHeight;
                if (!width || !height || view.box.offsetParent === null) { view.next = now + 250; return; }
                // Cells about 11px tall, shaped like a terminal's (0.6 wide to 1.45 tall).
                const rows = Math.max(3, Math.round(height / 11));
                const columns = Math.max(6, Math.round(width / (height / rows * 0.6 / 1.45)));
                // While the drawer opens or closes, the last frame stretches to fit; the
                // scene takes the new size once it holds still, so it starts over only once.
                let warm = 0;
                if (columns !== view.columns || rows !== view.rows) {
                    const size = columns + 'x' + rows;
                    if (!view.columns || (view.pending === size && now - view.pendingSince > 300)) {
                        view.columns = columns; view.rows = rows; warm = FLOW_WARM_UP;
                    } else if (view.pending !== size) { view.pending = size; view.pendingSince = now; }
                }
                for (let i = 0; i < warm; i++) flowWasm.flow_frame(view.handle, view.columns, view.rows);
                const pointer = flowWasm.flow_frame(view.handle, view.columns, view.rows);
                drawFlowCells(view, new Uint32Array(flowWasm.memory.buffer, pointer, view.columns * view.rows * 3), view.columns, view.rows);
                view.next = now + flowWasm.flow_pace(view.handle);
            });
            if (live) requestAnimationFrame(flowTick); else flowLooping = false;
        }

        // flow's 2x4 masks (bit x + 2y), as its desktop renderer draws blocks.
        const FLOW_QUAD = [0x20, 0x2598, 0x259D, 0x2580, 0x2596, 0x258C, 0x259E, 0x259B, 0x2597, 0x259A, 0x2590, 0x259C, 0x2584, 0x2599, 0x259F, 0x2588];
        const FLOW_SHADE = { 0x2591: 0.3, 0x2592: 0.55, 0x2593: 0.8 };
        const FLOW_BRAILLE = [[1, 2, 4, 64], [8, 16, 32, 128]];
        function flowMask(cp) {
            if (cp === 0x20 || cp === 0xA0) return 0;
            const quad = FLOW_QUAD.indexOf(cp);
            if (quad >= 0) return (quad & 1 ? 5 : 0) | (quad & 2 ? 10 : 0) | (quad & 4 ? 80 : 0) | (quad & 8 ? 160 : 0);
            if (cp > 0x2580 && cp < 0x2588) return (0xFF << ((4 - Math.floor((cp - 0x2580) / 2 + 0.5)) * 2)) & 0xFF;
            if (cp > 0x2588 && cp < 0x2590) { const c = Math.floor((0x2590 - cp) / 4 + 0.5); return c >= 2 ? 0xFF : c === 1 ? 0x55 : 0; }
            if (cp >= 0x2800 && cp <= 0x28FF) {
                let mask = 0;
                for (let x = 0; x < 2; x++) for (let y = 0; y < 4; y++) if ((cp - 0x2800) & FLOW_BRAILLE[x][y]) mask |= 1 << (x + 2 * y);
                return mask;
            }
            return -1;
        }
        function flowMix(a, b, k) {
            const channel = shift => Math.round(((a >> shift) & 255) + (((b >> shift) & 255) - ((a >> shift) & 255)) * k);
            return (channel(16) << 16) | (channel(8) << 8) | channel(0);
        }
        function drawFlowCells(view, words, columns, rows) {
            const { canvas, ctx } = view;
            const width = Math.round(canvas.clientWidth * devicePixelRatio), height = Math.round(canvas.clientHeight * devicePixelRatio);
            if (canvas.width !== width || canvas.height !== height) { canvas.width = width; canvas.height = height; }
            const subColumns = columns * 2, subRows = rows * 4, sx = width / subColumns, sy = height / subRows;
            const sub = new Int32Array(subColumns * subRows).fill(-1), text = [];
            for (let i = 0; i < columns * rows; i++) {
                const cp = words[i * 3], fg = words[i * 3 + 1] === FLOW_CLEAR ? -1 : words[i * 3 + 1], bg = words[i * 3 + 2] === FLOW_CLEAR ? -1 : words[i * 3 + 2];
                const cx = (i % columns) * 2, cy = Math.floor(i / columns) * 4, mask = flowMask(cp);
                for (let y = 0; y < 4; y++) for (let x = 0; x < 2; x++) {
                    let color = bg;
                    if (cp in FLOW_SHADE) color = fg < 0 ? bg : flowMix(bg < 0 ? 0x0b0f14 : bg, fg, FLOW_SHADE[cp]);
                    else if (mask > 0 && (mask & (1 << (x + 2 * y)))) color = fg < 0 ? bg : fg;
                    sub[(cy + y) * subColumns + cx + x] = color;
                }
                if (mask < 0 && fg >= 0 && cp > 0x20) text.push([cx + 1, cy + 3, fg, cp]);
            }
            ctx.clearRect(0, 0, width, height);
            for (let y = 0; y < subRows; y++) {
                const y0 = Math.round(y * sy), y1 = Math.round((y + 1) * sy);
                for (let x = 0; x < subColumns;) {
                    const color = sub[y * subColumns + x];
                    let end = x + 1;
                    while (end < subColumns && sub[y * subColumns + end] === color) end++;
                    if (color >= 0) {
                        ctx.fillStyle = '#' + color.toString(16).padStart(6, '0');
                        const x0 = Math.round(x * sx);
                        ctx.fillRect(x0, y0, Math.round(end * sx) - x0, y1 - y0);
                    }
                    x = end;
                }
            }
            if (text.length) {
                ctx.textAlign = 'center';
                ctx.font = Math.round(sy * 3.1) + 'px JetBrains Mono, monospace';
                text.forEach(([x, y, fg, cp]) => { ctx.fillStyle = '#' + fg.toString(16).padStart(6, '0'); ctx.fillText(String.fromCodePoint(cp), x * sx, y * sy); });
            }
        }
`;
