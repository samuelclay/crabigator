// Dashboard JavaScript - style
export const styleJs = `
        function adjustFontSize(delta) {
            const newIndex = Math.max(0, Math.min(FONT_SCALES.length - 1, currentFontScaleIndex + delta));
            if (newIndex !== currentFontScaleIndex) {
                currentFontScaleIndex = newIndex;
                applyFontScale();
                saveSettingsToServer();
            }
        }

        function applyFontScale() {
            const scale = FONT_SCALES[currentFontScaleIndex];

            // Set flag to prevent scroll events from unpinning during zoom change
            isChangingFontSize = true;

            document.documentElement.style.setProperty('--font-scale', scale);
            const label = document.getElementById('font-label');
            if (label) label.textContent = Math.round(scale * 100) + '%';

            // Scroll pinned sessions to bottom after zoom
            requestAnimationFrame(() => {
                for (const [id, sessionData] of sessions) {
                    if (sessionData.pinned) {
                        const terminal = document.getElementById('terminal-' + id);
                        if (terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                }
                // Clear flag after scroll positions are restored
                setTimeout(() => { isChangingFontSize = false; }, 100);
            });
        }

        function adjustTerminalHeight(delta) {
            const newIndex = Math.max(0, Math.min(TERMINAL_HEIGHTS.length - 1, currentHeightIndex + delta));
            if (newIndex !== currentHeightIndex) {
                currentHeightIndex = newIndex;
                applyTerminalHeight();
                saveSettingsToServer();
            }
        }

        function applyTerminalHeight() {
            const height = TERMINAL_HEIGHTS[currentHeightIndex];
            const label = document.getElementById('height-label');
            if (label) label.textContent = height + 'px';

            // Apply height directly to all terminal elements
            document.querySelectorAll('.terminal').forEach(terminal => {
                terminal.style.height = height + 'px';
            });

            // Scroll pinned sessions to bottom after height change
            requestAnimationFrame(() => {
                for (const [id, sessionData] of sessions) {
                    if (sessionData.pinned) {
                        const terminal = document.getElementById('terminal-' + id);
                        if (terminal) {
                            terminal.scrollTop = terminal.scrollHeight;
                            sessionData.lastScrollTop = terminal.scrollTop;
                        }
                    }
                }
            });
        }

        async function saveSettingsToServer() {
            try {
                await fetch(API_BASE + '/settings', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        fontScaleIndex: currentFontScaleIndex,
                        terminalHeightIndex: currentHeightIndex
                    }),
                    credentials: 'same-origin'
                });
            } catch {}
        }

        async function loadSettingsFromServer() {
            try {
                const resp = await fetch(API_BASE + '/settings', {
                    credentials: 'same-origin'
                });
                if (resp.ok) {
                    const data = await resp.json();
                    if (typeof data.fontScaleIndex === 'number') {
                        currentFontScaleIndex = Math.max(0, Math.min(FONT_SCALES.length - 1, data.fontScaleIndex));
                    }
                    if (typeof data.terminalHeightIndex === 'number') {
                        currentHeightIndex = Math.max(0, Math.min(TERMINAL_HEIGHTS.length - 1, data.terminalHeightIndex));
                    }
                }
            } catch {}
            applyFontScale();
            applyTerminalHeight();
        }

        // Style popover
        function toggleStylePopover() {
            const popover = document.getElementById('style-popover');
            const btn = document.getElementById('style-btn');
            const isVisible = popover.classList.toggle('visible');
            btn.classList.toggle('active', isVisible);
        }

        function closeStylePopover() {
            const popover = document.getElementById('style-popover');
            const btn = document.getElementById('style-btn');
            popover.classList.remove('visible');
            btn.classList.remove('active');
        }

        // Close popover when clicking outside
        document.addEventListener('click', (e) => {
            const stylePopover = document.getElementById('style-popover');
            const styleBtn = document.getElementById('style-btn');
            const settingsPopover = document.getElementById('settings-popover');
            const settingsBtn = document.getElementById('settings-btn');

            if (stylePopover && styleBtn && !stylePopover.contains(e.target) && !styleBtn.contains(e.target)) {
                closeStylePopover();
            }
            if (settingsPopover && settingsBtn && !settingsPopover.contains(e.target) && !settingsBtn.contains(e.target)) {
                closeSettingsPopover();
            }
        });

        // Settings popover
        function toggleSettingsPopover() {
            const popover = document.getElementById('settings-popover');
            const btn = document.getElementById('settings-btn');
            const isVisible = popover.classList.toggle('visible');
            btn.classList.toggle('active', isVisible);
            // Close style popover if open
            if (isVisible) closeStylePopover();
        }

        function closeSettingsPopover() {
            const popover = document.getElementById('settings-popover');
            const btn = document.getElementById('settings-btn');
            if (popover) popover.classList.remove('visible');
            if (btn) btn.classList.remove('active');
        }

        async function generateInviteCode() {
            const btn = document.getElementById('generate-invite-btn');
            const result = document.getElementById('invite-result');

            if (!mobileToken) {
                result.innerHTML = '<div style="color: #f85149; font-size: 12px;">Not paired yet</div>';
                result.classList.add('visible');
                return;
            }

            btn.disabled = true;
            btn.textContent = 'Generating…';

            try {
                const response = await fetch('/api/pairing/invite', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'Authorization': 'Bearer ' + mobileToken
                    }
                });

                if (!response.ok) {
                    throw new Error('Failed to generate invite code');
                }

                const data = await response.json();
                const pairUrl = window.location.origin + '/pair/' + data.token;

                result.innerHTML = \`
                    <div class="invite-code">\${data.code}</div>
                    <div class="invite-hint">Enter this code on your other device</div>
                    <div class="invite-link">
                        <a href="\${pairUrl}" target="_blank">Or open this link →</a>
                    </div>
                \`;
                result.classList.add('visible');

            } catch (err) {
                result.innerHTML = '<div style="color: #f85149; font-size: 12px;">' + err.message + '</div>';
                result.classList.add('visible');
            } finally {
                btn.disabled = false;
                btn.textContent = 'Generate pairing code';
            }
        }

`;
