// Home page interactions. Copy controls also serve the embedded MCP commands.
export const landingJs = `
document.querySelectorAll('[data-copy-target]').forEach(function(button) {
    const label = button.getAttribute('aria-label');
    let resetTimer;
    button.addEventListener('click', async function() {
        const target = document.getElementById(button.dataset.copyTarget);
        const status = document.getElementById('copy-status');
        if (!target) return;
        try {
            await navigator.clipboard.writeText(target.textContent.trim());
            status.textContent = 'Copied to clipboard.';
            button.setAttribute('aria-label', 'Copied');
            button.classList.add('copied');
            window.crabAnalytics?.trackClick(button.dataset.copyTarget === 'install-command' ? 'copy_install' : 'copy_mcp', button.dataset.copyTarget);
            clearTimeout(resetTimer);
            resetTimer = setTimeout(function() {
                button.setAttribute('aria-label', label);
                button.classList.remove('copied');
                status.textContent = '';
            }, 2000);
        } catch {
            status.textContent = 'Could not copy. Select and copy the command above.';
        }
    });
});
// Screen tabs: without JavaScript every panel shows, one after another.
const tablist = document.querySelector('.surface-tabs');
if (tablist) {
    const tabs = Array.from(tablist.querySelectorAll('[role="tab"]'));
    function select(tab, focus) {
        tabs.forEach(function(other) {
            const selected = other === tab;
            other.setAttribute('aria-selected', String(selected));
            other.tabIndex = selected ? 0 : -1;
            document.getElementById(other.getAttribute('aria-controls')).hidden = !selected;
        });
        if (focus) tab.focus();
    }
    tabs.forEach(function(tab, index) {
        tab.addEventListener('click', function() { select(tab, false); });
        tab.addEventListener('keydown', function(event) {
            const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
            if (event.key === 'Home') select(tabs[0], true);
            else if (event.key === 'End') select(tabs[tabs.length - 1], true);
            else if (step) select(tabs[(index + step + tabs.length) % tabs.length], true);
            else return;
            event.preventDefault();
        });
    });
    tablist.hidden = false;
    select(tabs[0], false);
}
const emailForm = document.getElementById('email-form');
if (emailForm) {
    emailForm.addEventListener('submit', function(event) {
        event.preventDefault();
        const input = emailForm.querySelector('input');
        if (!input.value.trim()) return;
        window.crabAnalytics?.trackSignup(input.value.trim());
        input.hidden = true;
        emailForm.querySelector('button').hidden = true;
        document.getElementById('email-success').hidden = false;
    });
}
`;
