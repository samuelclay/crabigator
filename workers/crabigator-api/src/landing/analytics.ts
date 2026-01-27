// Client-side analytics tracking script
// Lightweight (~2KB), privacy-focused, first-party analytics
export const analyticsJs = `
(function() {
    'use strict';

    // Configuration
    var BEACON_ENDPOINT = '/api/analytics/beacon';
    var EVENT_ENDPOINT = '/api/analytics/event';
    var SESSION_TIMEOUT = 30 * 60 * 1000; // 30 minutes

    // Generate UUID v4
    function uuid() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            var r = Math.random() * 16 | 0;
            return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
        });
    }

    // Get or create visitor ID (persistent across sessions)
    function getVisitorId() {
        var key = 'crab_vid';
        var id = null;
        try {
            id = localStorage.getItem(key);
            if (!id) {
                id = uuid();
                localStorage.setItem(key, id);
            }
        } catch (e) {
            // localStorage not available, use session-only
            id = uuid();
        }
        return id;
    }

    // Get or create session ID (expires after 30 min inactivity)
    function getSessionId() {
        var key = 'crab_session';
        var now = Date.now();
        var session = null;

        try {
            var stored = sessionStorage.getItem(key);
            if (stored) {
                session = JSON.parse(stored);
            }

            if (!session || (now - session.lastActive) > SESSION_TIMEOUT) {
                session = { id: uuid(), lastActive: now };
            } else {
                session.lastActive = now;
            }

            sessionStorage.setItem(key, JSON.stringify(session));
        } catch (e) {
            // sessionStorage not available
            session = { id: uuid(), lastActive: now };
        }

        return session.id;
    }

    // Parse URL parameters
    function getParam(name) {
        var params = new URLSearchParams(window.location.search);
        return params.get(name);
    }

    // Get UTM and promo parameters
    function getTrackingParams() {
        return {
            utm_source: getParam('utm_source'),
            utm_medium: getParam('utm_medium'),
            utm_campaign: getParam('utm_campaign'),
            utm_content: getParam('utm_content'),
            utm_term: getParam('utm_term'),
            promo_code: getParam('promo') || getParam('offer') || getParam('code')
        };
    }

    // Store first-touch attribution
    function storeAttribution() {
        var key = 'crab_attr';
        try {
            var existing = localStorage.getItem(key);
            if (existing) return; // Only store first touch

            var params = getTrackingParams();
            var hasUtm = params.utm_source || params.utm_campaign;
            var hasReferrer = document.referrer && !document.referrer.includes(window.location.hostname);

            if (hasUtm || hasReferrer) {
                localStorage.setItem(key, JSON.stringify({
                    utm_source: params.utm_source,
                    utm_medium: params.utm_medium,
                    utm_campaign: params.utm_campaign,
                    utm_content: params.utm_content,
                    utm_term: params.utm_term,
                    promo_code: params.promo_code,
                    referrer: document.referrer,
                    timestamp: Date.now()
                }));
            }
        } catch (e) {
            // localStorage not available
        }
    }

    // Get stored attribution (for email signups)
    function getAttribution() {
        try {
            var stored = localStorage.getItem('crab_attr');
            return stored ? JSON.parse(stored) : {};
        } catch (e) {
            return {};
        }
    }

    // Track max scroll depth
    var maxScrollDepth = 0;
    function trackScroll() {
        var docHeight = document.documentElement.scrollHeight - window.innerHeight;
        if (docHeight <= 0) return;
        var scrolled = window.scrollY || window.pageYOffset;
        var depth = Math.round((scrolled / docHeight) * 100);
        if (depth > maxScrollDepth) {
            maxScrollDepth = Math.min(depth, 100);
        }
    }

    // Track time on page
    var pageLoadTime = Date.now();
    function getTimeOnPage() {
        return Math.round((Date.now() - pageLoadTime) / 1000);
    }

    // Send beacon (fire-and-forget)
    function sendBeacon(url, data) {
        var payload = JSON.stringify(data);
        if (navigator.sendBeacon) {
            navigator.sendBeacon(url, payload);
        } else {
            // Fallback for older browsers
            var xhr = new XMLHttpRequest();
            xhr.open('POST', url, true);
            xhr.setRequestHeader('Content-Type', 'application/json');
            xhr.send(payload);
        }
    }

    // Send page view
    function sendPageView() {
        var params = getTrackingParams();
        var referrer = document.referrer;
        // Don't count internal navigation as referrer
        if (referrer && referrer.includes(window.location.hostname)) {
            referrer = null;
        }

        sendBeacon(BEACON_ENDPOINT, {
            visitor_id: getVisitorId(),
            session_id: getSessionId(),
            page: window.location.pathname,
            referrer: referrer,
            utm_source: params.utm_source,
            utm_medium: params.utm_medium,
            utm_campaign: params.utm_campaign,
            utm_content: params.utm_content,
            utm_term: params.utm_term,
            promo_code: params.promo_code
        });
    }

    // Send engagement update (on page unload)
    function sendEngagement() {
        sendBeacon(BEACON_ENDPOINT + '?type=engagement', {
            visitor_id: getVisitorId(),
            session_id: getSessionId(),
            scroll_depth: maxScrollDepth,
            time_on_page: getTimeOnPage()
        });
    }

    // Track click event
    function trackClick(target, label) {
        sendBeacon(EVENT_ENDPOINT, {
            visitor_id: getVisitorId(),
            session_id: getSessionId(),
            event_type: 'click',
            event_target: target,
            event_value: label || null,
            page: window.location.pathname
        });
    }

    // Track email signup
    function trackSignup(email) {
        var params = getTrackingParams();
        var attr = getAttribution();

        sendBeacon(EVENT_ENDPOINT, {
            visitor_id: getVisitorId(),
            session_id: getSessionId(),
            event_type: 'signup',
            event_target: 'email_form',
            event_value: email,
            page: window.location.pathname,
            utm_source: params.utm_source || attr.utm_source,
            utm_medium: params.utm_medium || attr.utm_medium,
            utm_campaign: params.utm_campaign || attr.utm_campaign,
            promo_code: params.promo_code || attr.promo_code,
            referrer: document.referrer || attr.referrer
        });
    }

    // Initialize tracking
    function init() {
        // Store attribution on first visit
        storeAttribution();

        // Send page view
        sendPageView();

        // Throttled scroll tracking
        var scrollTimeout = null;
        window.addEventListener('scroll', function() {
            if (scrollTimeout) return;
            scrollTimeout = setTimeout(function() {
                trackScroll();
                scrollTimeout = null;
            }, 100);
        }, { passive: true });

        // Send engagement on page unload
        document.addEventListener('visibilitychange', function() {
            if (document.visibilityState === 'hidden') {
                sendEngagement();
            }
        });
        window.addEventListener('pagehide', sendEngagement);

        // Track clicks on elements with data-track attribute
        document.addEventListener('click', function(e) {
            var el = e.target;
            // Walk up to find data-track
            while (el && el !== document) {
                if (el.dataset && el.dataset.track) {
                    trackClick(el.dataset.track, el.dataset.label || null);
                    return;
                }
                el = el.parentElement;
            }
        });
    }

    // Expose API for manual tracking
    window.crabAnalytics = {
        trackClick: trackClick,
        trackSignup: trackSignup,
        getVisitorId: getVisitorId,
        getSessionId: getSessionId,
        getAttribution: getAttribution
    };

    // Initialize on load
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
`;
