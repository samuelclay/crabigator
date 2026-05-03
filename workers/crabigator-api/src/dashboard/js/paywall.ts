// Dashboard JavaScript - paywall and usage tracking
export const paywallJs = `
        // Account and plan state
        let usageState = {
            isPro: false,
            lastUpdate: Date.now(),
            subscriptionProvider: null  // 'stripe' or 'paypal'
        };
        let usageCountdownInterval = null;
        let paywallDismissed = false;

        // Update usage display in settings popover
        function updateUsageDisplay() {
            const usageDisplay = document.getElementById('usage-display');
            if (!usageDisplay) return;

            if (usageState.isPro) {
                // Pro user - show premium card with manage link
                const total = allSessions.length;
                usageDisplay.innerHTML =
                    '<div class="pro-status-card">' +
                        '<div class="pro-status-header">' +
                            '<div class="pro-status-icon">✓</div>' +
                            '<span class="pro-status-label">Pro Subscriber</span>' +
                        '</div>' +
                        '<div class="pro-status-sublabel">All ' + total + ' active session' + (total === 1 ? '' : 's') + ' visible</div>' +
                        '<button class="manage-subscription-link" onclick="openSubscriptionPortal()">' +
                            'Manage subscription' +
                            '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 3l6 5-6 5"/></svg>' +
                        '</button>' +
                    '</div>';
                return;
            }

            const renderableSessions = getRenderableSessions(allSessions);
            const visible = renderableSessions.length;
            const total = allSessions.length;
            const hidden = hiddenSessionCount;
            const percent = Math.min(100, (visible / FREE_VISIBLE_SESSION_LIMIT) * 100);
            const barClass = hidden > 0 ? ' critical' : (percent >= 100 ? ' warning' : '');
            const note = hidden > 0
                ? hidden + ' active session' + (hidden === 1 ? ' is' : 's are') + ' hidden on Free.'
                : 'Free accounts show the ' + FREE_VISIBLE_SESSION_LIMIT + ' most recently active sessions.';

            usageDisplay.innerHTML =
                '<div class="usage-header">' +
                    '<span class="usage-label">Visible Sessions</span>' +
                    '<span class="usage-time">' + visible + ' / ' + FREE_VISIBLE_SESSION_LIMIT + ' visible</span>' +
                '</div>' +
                '<div class="usage-bar-container">' +
                    '<div class="usage-bar' + barClass + '" style="width: ' + percent + '%"></div>' +
                '</div>' +
                '<div class="usage-note">' + note + (total > 0 ? ' ' + total + ' active total.' : '') + '</div>' +
                '<button class="upgrade-btn" id="upgrade-btn" onclick="showUpgradeModal()">Upgrade to Pro</button>';
        }

        // Time-based usage is no longer counted. Keep this no-op for older callbacks.
        function startUsageCountdown() {
            if (usageCountdownInterval) {
                clearInterval(usageCountdownInterval);
                usageCountdownInterval = null;
            }
        }

        // Fetch account status from server
        async function fetchUsage() {
            await fetchSubscription();
            updateUsageDisplay();
        }

        // Handle usage_update SSE event
        function handleUsageUpdate(data) {
            if (data.is_pro !== undefined) {
                usageState.isPro = data.is_pro;
                isProUser = data.is_pro;
            }
            usageState.lastUpdate = Date.now();
            syncRenderedSessions();
            updateUsageDisplay();
        }

        // Stop all sessions (close connections and clear UI)
        function stopAllSessions() {
            // Close all EventSource connections
            for (const [sessionId, session] of sessions) {
                if (session.eventSource) {
                    session.eventSource.close();
                }
            }
            sessions.clear();

            // Close session list stream too
            if (sessionListSource) {
                sessionListSource.close();
                sessionListSource = null;
            }

            // Clear the sessions container
            const container = document.getElementById('sessions');
            if (container) {
                container.innerHTML = '<div class="no-sessions">Session visibility limited</div>';
            }

            // Update status
            const status = document.getElementById('status');
            if (status) {
                status.textContent = 'Visibility limited';
            }

            // Stop viewer activity tracking
            stopViewerActivityTracking();
        }

        // Show paywall overlay (limit reached mode)
        function showPaywall() {
            showUpgradeModal();
        }

        // Show upgrade modal (voluntary upgrade mode)
        function showUpgradeModal() {
            const overlay = document.getElementById('paywall-overlay');
            if (!overlay) return;

            // Close settings popover
            const settingsPopover = document.getElementById('settings-popover');
            const settingsBtn = document.getElementById('settings-btn');
            if (settingsPopover) settingsPopover.classList.remove('visible');
            if (settingsBtn) settingsBtn.classList.remove('active');

            // Set upgrade messaging
            const title = document.getElementById('paywall-title');
            const subtitle = document.getElementById('paywall-subtitle');
            const usageSection = document.getElementById('paywall-usage-section');

            if (title) title.textContent = 'Upgrade to Pro';
            if (subtitle) subtitle.textContent = 'Show every active session at once. Free accounts show the ' + FREE_VISIBLE_SESSION_LIMIT + ' most recently active sessions.';
            if (usageSection) usageSection.style.display = 'none';

            showPaywallContent();
            overlay.classList.add('visible');
        }

        // Hide paywall overlay
        function hidePaywall() {
            const overlay = document.getElementById('paywall-overlay');
            if (overlay) {
                overlay.classList.remove('visible');
            }
        }

        // Dismiss paywall (user clicked "Maybe later")
        function dismissPaywall() {
            paywallDismissed = true;
            hidePaywall();
        }

        // Show main paywall content
        function showPaywallContent() {
            document.getElementById('paywall-content').style.display = 'block';
            document.getElementById('paywall-loading').classList.remove('visible');
            document.getElementById('paywall-success').classList.remove('visible');
            document.getElementById('paywall-error').classList.remove('visible');
        }

        // Show loading state
        function showPaywallLoading() {
            document.getElementById('paywall-content').style.display = 'none';
            document.getElementById('paywall-loading').classList.add('visible');
            document.getElementById('paywall-success').classList.remove('visible');
            document.getElementById('paywall-error').classList.remove('visible');
        }

        // Show success state
        function showPaywallSuccess() {
            document.getElementById('paywall-content').style.display = 'none';
            document.getElementById('paywall-loading').classList.remove('visible');
            document.getElementById('paywall-success').classList.add('visible');
            document.getElementById('paywall-error').classList.remove('visible');

            // Track conversion with Meta Pixel
            if (typeof fbq === 'function') {
                fbq('track', 'Purchase', {
                    value: 3.00,
                    currency: 'USD',
                    content_name: 'Crabigator Pro',
                    content_type: 'subscription'
                });
            }

            // Hide paywall after short delay
            setTimeout(() => {
                hidePaywall();
                // Refresh usage state
                usageState.isPro = true;
                isProUser = true;
                syncRenderedSessions();
                updateUsageDisplay();
            }, 2000);
        }

        // Show error state
        function showPaywallError(message) {
            document.getElementById('paywall-content').style.display = 'none';
            document.getElementById('paywall-loading').classList.remove('visible');
            document.getElementById('paywall-success').classList.remove('visible');
            document.getElementById('paywall-error').classList.add('visible');
            document.getElementById('paywall-error-text').textContent = message || 'Payment processing failed. Please try again.';
        }

        // Initiate Stripe payment
        async function initiateStripePayment() {
            const btn = document.getElementById('paywall-stripe-btn');
            btn.disabled = true;

            // Track checkout initiation with Meta Pixel
            if (typeof fbq === 'function') {
                fbq('track', 'InitiateCheckout', {
                    value: 3.00,
                    currency: 'USD',
                    content_name: 'Crabigator Pro',
                    payment_method: 'stripe'
                });
            }

            try {
                const resp = await fetch(API_BASE + '/payments/stripe/checkout', {
                    method: 'POST',
                    headers: {
                        ...getAuthHeaders(),
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({
                        return_url: window.location.origin + '/dashboard'
                    })
                });

                if (!resp.ok) {
                    const data = await resp.json();
                    throw new Error(data.error || 'Failed to create checkout session');
                }

                const data = await resp.json();
                if (data.checkout_url) {
                    window.location.href = data.checkout_url;
                } else {
                    throw new Error('No checkout URL returned');
                }
            } catch (err) {
                console.error('Stripe checkout error:', err);
                showPaywallError(err.message);
                btn.disabled = false;
            }
        }

        // Initiate PayPal payment
        async function initiatePayPalPayment() {
            const btn = document.getElementById('paywall-paypal-btn');
            btn.disabled = true;

            // Track checkout initiation with Meta Pixel
            if (typeof fbq === 'function') {
                fbq('track', 'InitiateCheckout', {
                    value: 3.00,
                    currency: 'USD',
                    content_name: 'Crabigator Pro',
                    payment_method: 'paypal'
                });
            }

            try {
                const resp = await fetch(API_BASE + '/payments/paypal/subscribe', {
                    method: 'POST',
                    headers: {
                        ...getAuthHeaders(),
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({
                        return_url: window.location.origin + '/dashboard'
                    })
                });

                if (!resp.ok) {
                    const data = await resp.json();
                    throw new Error(data.error || 'Failed to create subscription');
                }

                const data = await resp.json();
                if (data.subscription_url) {
                    window.location.href = data.subscription_url;
                } else {
                    throw new Error('No subscription URL returned');
                }
            } catch (err) {
                console.error('PayPal subscription error:', err);
                showPaywallError(err.message);
                btn.disabled = false;
            }
        }

        // Handle payment callback (check URL params)
        async function handlePaymentCallback() {
            const params = new URLSearchParams(window.location.search);
            const payment = params.get('payment');
            const provider = params.get('provider');

            if (payment === 'success') {
                // Clear URL params
                window.history.replaceState({}, '', '/dashboard');

                // Show loading and poll for subscription
                showPaywall();
                showPaywallLoading();
                await pollSubscriptionStatus();
            } else if (payment === 'canceled') {
                // Clear URL params
                window.history.replaceState({}, '', '/dashboard');
            }
        }

        // Poll for subscription status after payment redirect
        async function pollSubscriptionStatus() {
            const maxAttempts = 15;  // 30 seconds total
            let attempts = 0;

            const checkStatus = async () => {
                try {
                    const resp = await fetch(API_BASE + '/subscription', {
                        headers: getAuthHeaders()
                    });

                    if (!resp.ok) {
                        throw new Error('Failed to check subscription');
                    }

                    const data = await resp.json();
                    if (data.is_pro) {
                        showPaywallSuccess();
                        return;
                    }

                    attempts++;
                    if (attempts < maxAttempts) {
                        setTimeout(checkStatus, 2000);
                    } else {
                        showPaywallError('Payment is still processing. Your subscription will be activated shortly.');
                    }
                } catch (err) {
                    console.error('Subscription check error:', err);
                    attempts++;
                    if (attempts < maxAttempts) {
                        setTimeout(checkStatus, 2000);
                    } else {
                        showPaywallError('Could not verify payment. Please refresh the page.');
                    }
                }
            };

            await checkStatus();
        }

        // Open subscription management portal
        async function openSubscriptionPortal() {
            const btn = document.querySelector('.manage-subscription-link');
            if (btn) {
                btn.disabled = true;
                btn.textContent = 'Loading...';
            }

            try {
                const resp = await fetch(API_BASE + '/subscription/portal', {
                    method: 'POST',
                    headers: {
                        ...getAuthHeaders(),
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({
                        return_url: window.location.href
                    })
                });

                if (!resp.ok) {
                    const data = await resp.json();
                    throw new Error(data.error || 'Failed to open portal');
                }

                const data = await resp.json();
                if (data.portal_url) {
                    window.location.href = data.portal_url;
                } else {
                    throw new Error('No portal URL returned');
                }
            } catch (err) {
                console.error('Portal error:', err);
                alert('Unable to open subscription management. Please try again.');
                if (btn) {
                    btn.disabled = false;
                    btn.textContent = 'Manage Subscription';
                }
            }
        }

        // Fetch subscription status (for provider info)
        async function fetchSubscription() {
            try {
                const resp = await fetch(API_BASE + '/subscription', {
                    headers: getAuthHeaders()
                });
                if (!resp.ok) return;

                const data = await resp.json();
                const wasPro = usageState.isPro;
                usageState.isPro = data.is_pro || false;
                isProUser = usageState.isPro;
                usageState.subscriptionProvider = data.subscription?.provider || null;
                if (wasPro !== usageState.isPro) {
                    syncRenderedSessions();
                } else {
                    updateUsageDisplay();
                }
            } catch (err) {
                console.error('Error fetching subscription:', err);
            }
        }

        // Initialize usage tracking
        function initUsageTracking() {
            fetchUsage();
            handlePaymentCallback();
        }
`;
