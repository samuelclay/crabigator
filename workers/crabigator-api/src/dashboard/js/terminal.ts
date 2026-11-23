// Keep terminal updates and user scrolling independent of page scrolling.
export const terminalJs = `
        function preserveTerminalScroll(sessionId, update) {
            const data = sessions.get(sessionId);
            const terminal = document.getElementById('terminal-' + sessionId);
            if (!data || !terminal || data.updatingTerminal) return update();
            const top = terminal.scrollTop;
            const viewportTop = terminal.getBoundingClientRect().top;
            const history = document.getElementById('scrollback-' + sessionId);
            let anchor = null;
            if (!data.pinned && history) {
                // Histories can contain thousands of rows. Find the first
                // visible row without measuring every preceding row per frame.
                let low = 0, high = history.children.length;
                while (low < high) {
                    const mid = Math.floor((low + high) / 2);
                    if (history.children[mid].getBoundingClientRect().bottom <= viewportTop) low = mid + 1;
                    else high = mid;
                }
                anchor = history.children[low] || null;
                if (anchor?.dataset?.terminalRows) {
                    // A paragraph wrapper may disappear when old source rows
                    // are trimmed. Anchor a retained visible row inside it.
                    anchor = Array.from(anchor.children).find(row => row.getBoundingClientRect().bottom > viewportTop) || anchor;
                }
            }
            const offset = anchor ? anchor.getBoundingClientRect().top - viewportTop : 0;
            data.updatingTerminal = true;
            try {
                update();
            } finally {
                data.updatingTerminal = false;
                const target = data.pinned ? terminal.scrollHeight
                    : anchor?.isConnected ? terminal.scrollTop + anchor.getBoundingClientRect().top - terminal.getBoundingClientRect().top - offset
                    : top;
                const clamped = Math.max(0, Math.min(target, terminal.scrollHeight - terminal.clientHeight));
                if (Math.abs(terminal.scrollTop - clamped) > 0.5) terminal.scrollTop = clamped;
                data.lastScrollTop = terminal.scrollTop;
                data.refreshPromptNavigation?.();
            }
        }

        function bindPromptNavigation(sessionId, terminal) {
            const data = sessions.get(sessionId);
            const frame = document.createElement('div');
            frame.className = 'terminal-frame';
            terminal.before(frame);
            frame.append(terminal);
            const history = document.getElementById('scrollback-' + sessionId);
            const screen = document.getElementById('screen-' + sessionId);
            let indexedBuffer, indexedRows = 0, historyPrompts = [], targets = [];
            let pending = false;

            function makeButton(direction) {
                const button = document.createElement('button');
                button.type = 'button';
                button.hidden = true;
                button.className = 'prompt-jump ' + direction;
                button.setAttribute('aria-controls', terminal.id);
                const label = direction === 'previous' ? 'Previous prompt' : 'Next prompt';
                const heading = document.createElement('strong');
                heading.textContent = label;
                const count = document.createElement('span');
                count.className = 'prompt-count';
                const preview = document.createElement('span');
                preview.className = 'prompt-excerpt';
                button.append(heading, count, preview);
                frame.append(button);
                button.onclick = () => {
                    const target = targets[Number(button.dataset.index)];
                    if (!target) return;
                    activateTerminalScroll(sessionId);
                    data.pinned = false;
                    data.promptSelected = target;
                    updatePinButton(sessionId, false);
                    if (target.row !== undefined) {
                        const missing = indexedRows - (data.scrollbackRendered || 0) - target.row;
                        if (missing > 0) loadMoreScrollback(sessionId, Math.max(missing, SCROLLBACK_CHUNK_SIZE));
                    }
                    const node = target.node || history.querySelectorAll('.prompt-row')[target.ordinal - firstMounted()];
                    if (node) {
                        terminal.scrollLeft = 0;
                        terminal.scrollTop += node.getBoundingClientRect().top - terminal.getBoundingClientRect().top - 6;
                        data.lastScrollTop = terminal.scrollTop;
                        if (!matchMedia('(prefers-reduced-motion: reduce)').matches)
                            node.animate([{backgroundColor:'rgba(108,193,221,.30)'}, {backgroundColor:'transparent'}], 650);
                    }
                    refresh();
                    if (button.hidden) (direction === 'previous' ? next : previous).button.focus({preventScroll:true});
                };
                return {button, update(index) {
                    const target = targets[index];
                    button.hidden = !target;
                    if (!target) return;
                    button.dataset.index = index;
                    count.textContent = (index + 1) + '/' + targets.length;
                    preview.textContent = target.text;
                    button.title = label + ' ' + (index + 1) + ' of ' + targets.length + ' — ' + target.text;
                }};
            }
            const previous = makeButton('previous'), next = makeButton('next');

            function firstMounted() {
                const firstRow = indexedRows - (data.scrollbackRendered || 0);
                const i = historyPrompts.findIndex(p => p.row >= firstRow);
                return i < 0 ? historyPrompts.length : i;
            }
            function refresh() {
                pending = false;
                const buffer = data.scrollbackBuffer;
                if (buffer !== indexedBuffer) {
                    historyPrompts = []; indexedRows = 0; indexedBuffer = buffer;
                }
                while (indexedRows < (buffer?.length || 0)) {
                    const row = indexedRows++;
                    const text = terminalPromptExcerpt(buffer[row]);
                    if (text) historyPrompts.push({row, text, ordinal:historyPrompts.length});
                }
                targets = historyPrompts.concat(Array.from(screen.querySelectorAll('.prompt-row'), (node, i) => ({
                    text:Array.from(node.textContent.slice(1).trim()).slice(0, 80).join(''), node, ordinal:historyPrompts.length + i
                })));
                let before = targets.length - 1, after = targets.length;
                if (data.pinned) data.promptSelected = null;
                if (!data.pinned) {
                    const selected = data.promptSelected;
                    const current = selected && targets[selected.ordinal]?.text === selected.text ? selected.ordinal : -1;
                    if (current >= 0) { before = current - 1; after = current + 1; }
                    else {
                        data.promptSelected = null;
                        const top = terminal.getBoundingClientRect().top + 2;
                        const first = firstMounted();
                        const mounted = history.querySelectorAll('.prompt-row');
                        after = targets.findIndex((p, i) => i >= first &&
                            (p.node || mounted[i - first])?.getBoundingClientRect().top >= top);
                        if (after < 0) after = targets.length;
                        before = after - 1;
                    }
                }
                previous.update(before); next.update(after);
            }
            function schedule() {
                if (!pending) { pending = true; requestAnimationFrame(refresh); }
            }
            data.refreshPromptNavigation = schedule;
            terminal.addEventListener('scroll', schedule, {passive:true});
            for (const name of ['wheel','touchmove','pointerdown','keydown']) {
                terminal.addEventListener(name, () => { data.promptSelected = null; schedule(); }, {passive:true});
            }
            schedule();
        }

        function bindTerminalScroll(sessionId, terminal) {
            bindPromptNavigation(sessionId, terminal);
            function stopFollowing() {
                const data = sessions.get(sessionId);
                if (activeTerminalId !== sessionId || !data?.pinned || terminal.scrollHeight <= terminal.clientHeight) return;
                data.pinned = false;
                updatePinButton(sessionId, false);
            }
            // Stop before the browser scrolls, so an arriving frame cannot win
            // the race with a small wheel or touch gesture.
            terminal.addEventListener('wheel', event => {
                if (event.deltaY < 0) stopFollowing();
            }, { passive: true });
            let touchY = null;
            terminal.addEventListener('touchstart', event => {
                touchY = event.touches[0]?.clientY ?? null;
            }, { passive: true });
            terminal.addEventListener('touchmove', event => {
                const nextY = event.touches[0]?.clientY;
                if (touchY !== null && nextY > touchY) stopFollowing();
                touchY = nextY ?? null;
            }, { passive: true });
            terminal.addEventListener('keydown', event => {
                if (['ArrowUp', 'PageUp', 'Home'].includes(event.key)) stopFollowing();
            });
            terminal.addEventListener('scroll', () => {
                const data = sessions.get(sessionId);
                if (!data || isChangingFontSize || data.updatingTerminal || activeTerminalId !== sessionId) return;
                // Clamp elastic overscroll before comparing positions.
                const top = Math.max(0, Math.min(terminal.scrollTop, terminal.scrollHeight - terminal.clientHeight));
                const previous = data.lastScrollTop || 0;
                data.lastScrollTop = top;
                if (top < previous) stopFollowing();
                else if (!data.promptSelected && top > previous && terminal.scrollHeight - top - terminal.clientHeight < 5 && !data.pinned) {
                    data.pinned = true;
                    updatePinButton(sessionId, true);
                }
                if (!data.pinned && top < 100) loadMoreScrollback(sessionId);
            });
            terminal.addEventListener('click', () => activateTerminalScroll(sessionId));
            terminal.addEventListener('focus', () => activateTerminalScroll(sessionId));
        }
`;
