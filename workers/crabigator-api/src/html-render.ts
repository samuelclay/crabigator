const OFFICIAL_ORIGIN = 'https://drinkcrabigator.com';
const OFFICIAL_HOST = 'drinkcrabigator.com';
const ORIGIN_PLACEHOLDER = '__CRABIGATOR_PUBLIC_ORIGIN__';

export function escapeHtml(value: string): string {
    return value.replace(/[&<>"']/g, (character) => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;',
    })[character] || character);
}

export function usePublicOrigin(html: string, origin: string): string {
    return html
        .replaceAll(OFFICIAL_ORIGIN, ORIGIN_PLACEHOLDER)
        .replaceAll(OFFICIAL_HOST, new URL(origin).host)
        .replaceAll(ORIGIN_PLACEHOLDER, origin);
}

export function metaPixelHtml(pixelId: string): string {
    const safeId = pixelId.replace(/[^0-9]/g, '');
    if (!safeId) return '';

    return `<script>
    !function(f,b,e,v,n,t,s){if(f.fbq)return;n=f.fbq=function(){n.callMethod?
    n.callMethod.apply(n,arguments):n.queue.push(arguments)};if(!f._fbq)f._fbq=n;
    n.push=n;n.loaded=!0;n.version='2.0';n.queue=[];t=b.createElement(e);t.async=!0;
    t.src=v;s=b.getElementsByTagName(e)[0];s.parentNode.insertBefore(t,s)}
    (window,document,'script','https://connect.facebook.net/en_US/fbevents.js');
    fbq('init','${safeId}');fbq('track','PageView');
    </script><noscript><img height="1" width="1" style="display:none"
    src="https://www.facebook.com/tr?id=${safeId}&ev=PageView&noscript=1"></noscript>`;
}
