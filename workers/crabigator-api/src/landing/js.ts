// Landing page JavaScript
export const landingJs = `
    // Copy button functionality
    function copyInstallCommand() {
        const command = 'npm install -g crabigator';
        const btn = document.getElementById('copy-btn');

        navigator.clipboard.writeText(command).then(() => {
            btn.innerHTML = '<svg viewBox="0 0 16 16" fill="currentColor"><path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/></svg> Copied!';
            btn.classList.add('copied');

            setTimeout(() => {
                btn.innerHTML = '<svg viewBox="0 0 16 16" fill="currentColor"><path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/><path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/></svg> Copy';
                btn.classList.remove('copied');
            }, 2000);
        }).catch(err => {
            console.error('Failed to copy:', err);
        });
    }

    // Email signup handler (works for both hero and bottom forms)
    function handleEmailSignup(formId, successId, privacySelector) {
        return function(event) {
            event.preventDefault();
            const form = event.target;
            const email = form.querySelector('input[type="email"]').value;
            const btn = form.querySelector('button');
            const successEl = document.getElementById(successId);
            const formEl = document.getElementById(formId);

            if (!email) return;

            btn.disabled = true;
            btn.textContent = 'Signing up...';

            // Simulate API call (in production, this would POST to an actual endpoint)
            setTimeout(() => {
                formEl.style.display = 'none';
                successEl.classList.add('visible');
                if (privacySelector) {
                    const privacy = document.querySelector(privacySelector);
                    if (privacy) privacy.style.display = 'none';
                }
            }, 800);
        };
    }

    // Scroll spy for navigation
    function initScrollSpy() {
        const sections = document.querySelectorAll('section[id]');
        const navLinks = document.querySelectorAll('.nav-link');

        function updateActiveLink() {
            const scrollPos = window.scrollY + 100; // offset for nav height

            let currentSection = '';
            sections.forEach(section => {
                const sectionTop = section.offsetTop;
                const sectionHeight = section.offsetHeight;
                if (scrollPos >= sectionTop && scrollPos < sectionTop + sectionHeight) {
                    currentSection = section.getAttribute('id');
                }
            });

            navLinks.forEach(link => {
                link.classList.remove('active');
                const href = link.getAttribute('href');
                if (href === '#' + currentSection) {
                    link.classList.add('active');
                }
            });
        }

        window.addEventListener('scroll', updateActiveLink, { passive: true });
        updateActiveLink(); // Initial check
    }

    // Hero devices scaling for mobile
    function initHeroDevicesScaling() {
        const devices = document.querySelector('.hero-devices');
        const phone = document.querySelector('.hero-phone');
        const terminal = document.querySelector('.hero-terminal');

        if (!devices || !phone || !terminal) return;

        const OVERLAP = 20; // pixels of overlap
        const PHONE_WIDTH = 220; // natural phone width
        const TERMINAL_WIDTH = 580; // natural terminal width
        const MOBILE_BREAKPOINT = 1024; // Apply scaling for mobile and tablet

        function scaleDevices() {
            const viewportWidth = window.innerWidth;

            // Only apply scaling on mobile
            if (viewportWidth >= MOBILE_BREAKPOINT) {
                // Reset to default desktop styles
                devices.style.transform = '';
                devices.style.width = '';
                devices.style.marginLeft = '';
                devices.style.marginRight = '';
                devices.style.position = '';
                devices.style.left = '';
                devices.style.flexDirection = '';
                devices.style.justifyContent = '';
                devices.style.alignItems = '';
                devices.style.height = '';
                phone.style.position = '';
                phone.style.left = '';
                phone.style.marginRight = '';
                phone.style.width = '';
                phone.style.order = '';
                phone.style.zIndex = '';
                terminal.style.width = '';
                terminal.style.maxWidth = '';
                terminal.style.order = '';
                return;
            }

            // Calculate scale factor based on viewport width
            const totalNaturalWidth = PHONE_WIDTH + TERMINAL_WIDTH - OVERLAP;
            const scale = viewportWidth / totalNaturalWidth;

            // Container: set to natural width, then scaled
            // Use transform-origin: top left and position from left edge
            devices.style.width = totalNaturalWidth + 'px';
            devices.style.flexDirection = 'row';
            devices.style.justifyContent = 'flex-start';
            devices.style.alignItems = 'flex-start';
            devices.style.position = 'relative';
            devices.style.left = '0';
            // Offset to break out of hero-content padding to viewport edge
            devices.style.marginLeft = '-16px';
            devices.style.marginRight = '-16px';
            // Reset height and transform to measure natural height
            devices.style.height = 'auto';
            devices.style.transform = 'none';

            // Measure natural height before scaling
            const naturalHeight = devices.scrollHeight;

            // Apply transform
            devices.style.transform = 'scale(' + scale + ')';
            devices.style.transformOrigin = 'top left';

            // Set container height to match scaled height (prevents empty space)
            devices.style.height = (naturalHeight * scale) + 'px';

            // Phone: on left, overlapping terminal by OVERLAP pixels
            phone.style.order = '-1';
            phone.style.position = 'relative';
            phone.style.left = '0';
            phone.style.marginRight = '-' + OVERLAP + 'px';
            phone.style.width = PHONE_WIDTH + 'px';
            phone.style.zIndex = '10';

            // Terminal: on the right
            terminal.style.width = TERMINAL_WIDTH + 'px';
            terminal.style.maxWidth = TERMINAL_WIDTH + 'px';
        }

        // Run on load and resize
        scaleDevices();
        window.addEventListener('resize', scaleDevices, { passive: true });
    }

    // Initialize
    document.addEventListener('DOMContentLoaded', function() {
        // Hero email form
        const heroEmailForm = document.getElementById('hero-email-form');
        if (heroEmailForm) {
            heroEmailForm.addEventListener('submit', handleEmailSignup('hero-email-form', 'hero-email-success', null));
        }

        // Bottom email form
        const emailForm = document.getElementById('email-form');
        if (emailForm) {
            emailForm.addEventListener('submit', handleEmailSignup('email-form', 'email-success', '.email-privacy'));
        }

        // Initialize scroll spy
        initScrollSpy();

        // Initialize hero devices scaling
        initHeroDevicesScaling();
    });
`;
