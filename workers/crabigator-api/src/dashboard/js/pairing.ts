// Dashboard JavaScript - pairing gate
export const pairingJs = `
        // Pairing state
        let mobileToken = localStorage.getItem('crabigator_mobile_token');
        let isPaired = !!mobileToken;

        function checkPairingStatus() {
            mobileToken = localStorage.getItem('crabigator_mobile_token');
            isPaired = !!mobileToken;
            return isPaired;
        }

        function getAuthHeaders() {
            const headers = { 'Content-Type': 'application/json' };
            if (mobileToken) {
                headers['Authorization'] = 'Bearer ' + mobileToken;
            }
            return headers;
        }

        function getAuthQueryParam() {
            return mobileToken ? '?token=' + encodeURIComponent(mobileToken) : '';
        }

        function handleAuthFailure(resp) {
            if (resp.status === 401) {
                clearPairing();
                return true;
            }
            return false;
        }

        // Auto-setup via URL parameter (for Chrome MCP)
        async function handleSetupParam() {
            const params = new URLSearchParams(window.location.search);
            const setupCode = params.get('setup');
            if (!setupCode) return false;

            // Validate code format (ABC-DEF-GHI)
            if (!setupCode.match(/^[A-Z0-9]{3}-[A-Z0-9]{3}-[A-Z0-9]{3}$/i)) {
                showSetupError('Invalid pairing code format');
                return true;
            }

            showSetupProgress();

            try {
                let mobileId = localStorage.getItem('crabigator_mobile_id');
                if (!mobileId) {
                    mobileId = 'chrome-' + crypto.randomUUID();
                    localStorage.setItem('crabigator_mobile_id', mobileId);
                }

                const response = await fetch('/api/pairing/claim', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        pairing_token: setupCode.toUpperCase(),
                        mobile_id: mobileId,
                        mobile_name: 'Chrome MCP'
                    })
                });

                if (!response.ok) {
                    const data = await response.json();
                    throw new Error(data.error || 'Pairing failed');
                }

                const data = await response.json();
                localStorage.setItem('crabigator_mobile_token', data.mobile_token);

                showSetupSuccess();
                setTimeout(() => { window.location.href = '/dashboard'; }, 1000);
            } catch (err) {
                showSetupError(err.message);
            }
            return true;
        }

        function showSetupProgress() {
            const container = document.getElementById('sessions');
            container.innerHTML = '';
            container.dataset.layout = '1';
            container.style.display = 'flex';
            container.style.justifyContent = 'center';
            container.style.alignItems = 'center';
            container.style.minHeight = 'calc(100vh - 80px)';

            const gate = document.createElement('div');
            gate.className = 'pairing-gate';
            gate.innerHTML = \`
                <div class="pairing-card">
                    <div class="pairing-icon spinning">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
                        </svg>
                    </div>
                    <h2>Connecting...</h2>
                    <p class="pairing-description">
                        Setting up your session. Please wait.
                    </p>
                </div>
            \`;
            container.appendChild(gate);
        }

        function showSetupSuccess() {
            const container = document.getElementById('sessions');
            container.innerHTML = '';

            const gate = document.createElement('div');
            gate.className = 'pairing-gate';
            gate.innerHTML = \`
                <div class="pairing-card">
                    <div class="pairing-icon" style="color: #3fb950;">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M20 6L9 17l-5-5"/>
                        </svg>
                    </div>
                    <h2>Connected!</h2>
                    <p class="pairing-description">
                        Redirecting to dashboard...
                    </p>
                </div>
            \`;
            container.appendChild(gate);
        }

        function showSetupError(message) {
            const container = document.getElementById('sessions');
            container.innerHTML = '';

            const gate = document.createElement('div');
            gate.className = 'pairing-gate';
            gate.innerHTML = \`
                <div class="pairing-card">
                    <div class="pairing-icon" style="color: #f85149;">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="10"/>
                            <line x1="15" y1="9" x2="9" y2="15"/>
                            <line x1="9" y1="9" x2="15" y2="15"/>
                        </svg>
                    </div>
                    <h2>Setup Failed</h2>
                    <p class="pairing-description">
                        \${message}
                    </p>
                    <button onclick="location.href='/dashboard'" style="margin-top: 16px; padding: 8px 16px; background: #238636; color: white; border: none; border-radius: 6px; cursor: pointer;">
                        Try Manual Pairing
                    </button>
                </div>
            \`;
            container.appendChild(gate);
        }

        function showPairingGate() {
            const container = document.getElementById('sessions');
            container.innerHTML = '';
            container.dataset.layout = '1';
            container.style.display = 'flex';
            container.style.justifyContent = 'center';
            container.style.alignItems = 'center';
            container.style.minHeight = 'calc(100vh - 80px)';

            const gate = document.createElement('div');
            gate.className = 'pairing-gate';
            gate.innerHTML = \`
                <div class="pairing-card">
                    <div class="pairing-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <rect x="5" y="2" width="14" height="20" rx="2"/>
                            <line x1="12" y1="18" x2="12" y2="18"/>
                        </svg>
                    </div>
                    <h2>Pair with Desktop</h2>
                    <p class="pairing-description">
                        Enter the pairing code shown in your terminal to connect this device.
                    </p>
                    <div class="pairing-form">
                        <input
                            type="text"
                            id="pairing-code-input"
                            placeholder="ABC-DEF-GHI"
                            maxlength="11"
                            autocomplete="off"
                            autocorrect="off"
                            autocapitalize="characters"
                            spellcheck="false"
                        />
                        <button id="pairing-submit-btn" onclick="submitPairingCode()">
                            Connect
                        </button>
                    </div>
                    <div id="pairing-error" class="pairing-error"></div>
                    <p class="pairing-help">
                        Start <code>crabigator</code> on your desktop to see the pairing code.
                    </p>
                </div>
                <div class="install-card">
                    <div class="install-header">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="install-icon">
                            <circle cx="12" cy="12" r="10"/>
                            <line x1="12" y1="16" x2="12" y2="12"/>
                            <line x1="12" y1="8" x2="12" y2="8"/>
                        </svg>
                        <span>Don't have Crabigator installed?</span>
                    </div>
                    <div class="install-command">
                        <code>npm install -g crabigator</code>
                        <button class="install-copy-btn" onclick="copyInstallCommand(this)">
                            <svg viewBox="0 0 16 16" fill="currentColor" width="14" height="14">
                                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
                                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
                            </svg>
                        </button>
                    </div>
                    <p class="install-hint">Then run <code>crabigator</code> instead of <code>claude</code></p>
                    <a href="/" class="install-link">Learn more about Crabigator →</a>
                </div>
            \`;
            container.appendChild(gate);

            // Format input as user types
            const input = document.getElementById('pairing-code-input');
            input.addEventListener('input', formatPairingInput);
            input.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') submitPairingCode();
            });
            input.focus();
        }

        function formatPairingInput(e) {
            let value = e.target.value.toUpperCase().replace(/[^A-Z0-9]/g, '');
            // Insert dashes after every 3 characters
            if (value.length > 3) {
                value = value.slice(0, 3) + '-' + value.slice(3);
            }
            if (value.length > 7) {
                value = value.slice(0, 7) + '-' + value.slice(7);
            }
            e.target.value = value.slice(0, 11);
        }

        async function submitPairingCode() {
            const input = document.getElementById('pairing-code-input');
            const errorEl = document.getElementById('pairing-error');
            const submitBtn = document.getElementById('pairing-submit-btn');

            const code = input.value.trim().toUpperCase();
            if (!code.match(/^[A-Z0-9]{3}-[A-Z0-9]{3}-[A-Z0-9]{3}$/)) {
                errorEl.textContent = 'Please enter a valid code (e.g., ABC-DEF-GHI)';
                return;
            }

            // Disable input while processing
            input.disabled = true;
            submitBtn.disabled = true;
            submitBtn.textContent = 'Connecting...';
            errorEl.textContent = '';

            try {
                // Generate a unique mobile ID for this browser
                let mobileId = localStorage.getItem('crabigator_mobile_id');
                if (!mobileId) {
                    mobileId = 'web-' + crypto.randomUUID();
                    localStorage.setItem('crabigator_mobile_id', mobileId);
                }

                const response = await fetch('/api/pairing/claim', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        pairing_token: code,
                        mobile_id: mobileId,
                        mobile_name: navigator.userAgent.includes('Mobile') ? 'Mobile Browser' : 'Web Browser'
                    })
                });

                if (!response.ok) {
                    const data = await response.json();
                    throw new Error(data.error || 'Failed to pair');
                }

                const data = await response.json();

                // Store the mobile token
                localStorage.setItem('crabigator_mobile_token', data.mobile_token);
                mobileToken = data.mobile_token;
                isPaired = true;

                // Show success and reload
                errorEl.style.color = '#3fb950';
                errorEl.textContent = 'Paired successfully! Loading sessions...';

                setTimeout(() => {
                    location.reload();
                }, 1000);

            } catch (err) {
                errorEl.textContent = err.message;
                input.disabled = false;
                submitBtn.disabled = false;
                submitBtn.textContent = 'Connect';
            }
        }

        function clearPairing() {
            localStorage.removeItem('crabigator_mobile_token');
            localStorage.removeItem('crabigator_mobile_id');
            mobileToken = null;
            isPaired = false;
            location.reload();
        }

        function copyInstallCommand(btn) {
            navigator.clipboard.writeText('npm install -g crabigator').then(() => {
                btn.innerHTML = '<svg viewBox="0 0 16 16" fill="currentColor" width="14" height="14"><path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/></svg>';
                setTimeout(() => {
                    btn.innerHTML = '<svg viewBox="0 0 16 16" fill="currentColor" width="14" height="14"><path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/><path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/></svg>';
                }, 1500);
            });
        }
`;
