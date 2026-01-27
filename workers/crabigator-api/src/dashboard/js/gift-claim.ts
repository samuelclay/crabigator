// Dashboard JavaScript - gift claim handling
export const giftClaimJs = `
        // Gift claim state
        let pendingGiftCookieId = localStorage.getItem('crabigator_pending_gift');

        // Handle ?gift=CODE URL parameter
        async function handleGiftParam() {
            const params = new URLSearchParams(window.location.search);
            const giftCode = params.get('gift');
            if (!giftCode) return false;

            // Validate code format (8 alphanumeric chars)
            if (!giftCode.match(/^[A-Z0-9]{8}$/i)) {
                showGiftError('Invalid gift code format');
                return true;
            }

            // Show loading state
            showGiftLoading();

            try {
                // First, get gift info
                const infoResponse = await fetch('/api/gifts/' + giftCode.toUpperCase());
                if (!infoResponse.ok) {
                    const data = await infoResponse.json();
                    throw new Error(data.error || 'Gift not found');
                }

                const giftInfo = await infoResponse.json();

                if (!giftInfo.is_claimable) {
                    showGiftError('This gift has already been claimed');
                    return true;
                }

                // Show the claim banner
                showGiftClaimBanner(giftInfo);

            } catch (err) {
                showGiftError(err.message);
            }
            return true;
        }

        function showGiftLoading() {
            const overlay = document.getElementById('gift-overlay');
            const content = document.getElementById('gift-content');
            const loading = document.getElementById('gift-loading');
            const success = document.getElementById('gift-success');
            const error = document.getElementById('gift-error');

            content.style.display = 'none';
            loading.classList.add('visible');
            success.classList.remove('visible');
            error.classList.remove('visible');
            overlay.classList.add('visible');
        }

        function showGiftClaimBanner(giftInfo) {
            const overlay = document.getElementById('gift-overlay');
            const content = document.getElementById('gift-content');
            const loading = document.getElementById('gift-loading');

            // Update content
            document.getElementById('gift-duration-text').textContent = giftInfo.duration_text;
            document.getElementById('gift-code-display').textContent = giftInfo.id;

            // Store gift code for claim
            overlay.dataset.giftCode = giftInfo.id;

            loading.classList.remove('visible');
            content.style.display = 'block';
            overlay.classList.add('visible');
        }

        function showGiftError(message) {
            const overlay = document.getElementById('gift-overlay');
            const content = document.getElementById('gift-content');
            const loading = document.getElementById('gift-loading');
            const error = document.getElementById('gift-error');
            const errorText = document.getElementById('gift-error-text');

            content.style.display = 'none';
            loading.classList.remove('visible');
            errorText.textContent = message;
            error.classList.add('visible');
            overlay.classList.add('visible');
        }

        function showGiftSuccess(message) {
            const content = document.getElementById('gift-content');
            const loading = document.getElementById('gift-loading');
            const success = document.getElementById('gift-success');
            const successText = document.getElementById('gift-success-text');

            content.style.display = 'none';
            loading.classList.remove('visible');
            successText.textContent = message;
            success.classList.add('visible');

            // Auto-close after 3 seconds
            setTimeout(() => {
                closeGiftOverlay();
                // Remove gift param from URL
                const url = new URL(window.location.href);
                url.searchParams.delete('gift');
                window.history.replaceState({}, '', url.pathname + url.search);
            }, 3000);
        }

        async function claimGift() {
            const overlay = document.getElementById('gift-overlay');
            const giftCode = overlay.dataset.giftCode;
            if (!giftCode) return;

            const claimBtn = document.getElementById('gift-claim-btn');
            claimBtn.disabled = true;
            claimBtn.textContent = 'Claiming...';

            try {
                // Build request body
                const body = {};

                // If not paired, generate a cookie ID
                if (!isPaired) {
                    let cookieId = pendingGiftCookieId;
                    if (!cookieId) {
                        cookieId = crypto.randomUUID();
                    }
                    body.cookie_id = cookieId;
                }

                const response = await fetch('/api/gifts/' + giftCode + '/claim', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify(body)
                });

                if (!response.ok) {
                    const data = await response.json();
                    throw new Error(data.error || 'Failed to claim gift');
                }

                const result = await response.json();

                // If pending (not paired), store the cookie ID
                if (result.status === 'pending' && result.cookie_id) {
                    localStorage.setItem('crabigator_pending_gift', result.cookie_id);
                    pendingGiftCookieId = result.cookie_id;
                }

                showGiftSuccess(result.message);

                // If activated, refresh usage/subscription status
                if (result.status === 'activated' || result.status === 'already_pro') {
                    // Trigger a usage refresh if the function exists
                    if (typeof fetchUsage === 'function') {
                        fetchUsage();
                    }
                }

            } catch (err) {
                showGiftError(err.message);
            }
        }

        function closeGiftOverlay() {
            const overlay = document.getElementById('gift-overlay');
            overlay.classList.remove('visible');

            // Remove gift param from URL
            const url = new URL(window.location.href);
            url.searchParams.delete('gift');
            window.history.replaceState({}, '', url.pathname + url.search);
        }

        function dismissGift() {
            closeGiftOverlay();
        }

        // Resolve pending gift after pairing
        async function resolvePendingGift() {
            if (!pendingGiftCookieId || !isPaired) return;

            try {
                const response = await fetch('/api/gifts/resolve-pending', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ cookie_id: pendingGiftCookieId })
                });

                if (response.ok) {
                    const result = await response.json();
                    if (result.resolved && result.status === 'activated') {
                        // Clear the pending gift
                        localStorage.removeItem('crabigator_pending_gift');
                        pendingGiftCookieId = null;

                        // Show a subtle notification that the gift was activated
                        console.log('Gift subscription activated:', result.message);
                    }
                }
            } catch (err) {
                console.error('Failed to resolve pending gift:', err);
            }
        }
`;
