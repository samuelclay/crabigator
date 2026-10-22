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
            }
        }

        function bindTerminalScroll(sessionId, terminal) {
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
                else if (top > previous && terminal.scrollHeight - top - terminal.clientHeight < 5 && !data.pinned) {
                    data.pinned = true;
                    updatePinButton(sessionId, true);
                }
                if (!data.pinned && top < 100) loadMoreScrollback(sessionId);
            });
            terminal.addEventListener('click', () => activateTerminalScroll(sessionId));
            terminal.addEventListener('focus', () => activateTerminalScroll(sessionId));
        }
`;
