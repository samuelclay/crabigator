// Dashboard JavaScript - leftover paid subscriptions
export const subscriptionJs = `
        // Crabigator is free for everyone. An account that still carries a paid
        // subscription gets a link to its billing portal so it can cancel.
        async function fetchSubscription() {
            try {
                const resp = await fetch(API_BASE + '/subscription', { headers: getAuthHeaders() });
                if (!resp.ok) return;
                const data = await resp.json();
                const paying = data.is_pro === true && !!(data.subscription && data.subscription.provider);
                const section = document.getElementById('subscription-section');
                const divider = document.getElementById('subscription-divider');
                if (section) section.hidden = !paying;
                if (divider) divider.hidden = !paying;
            } catch (err) {
                console.error('Error fetching subscription:', err);
            }
        }

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
                    btn.textContent = 'Manage subscription';
                }
            }
        }
`;
