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
`;
