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
