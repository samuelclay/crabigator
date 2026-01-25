// Dashboard JavaScript - prompt
export const promptJs = `
            const panel = document.getElementById('prompt-' + sessionId);
            const headerEl = document.getElementById('prompt-header-' + sessionId);
            const questionEl = document.getElementById('prompt-question-' + sessionId);
            const optionsEl = document.getElementById('prompt-options-' + sessionId);
            const otherEl = document.getElementById('prompt-other-' + sessionId);

            if (!panel) return;

            if (!prompt) {
                // Clear/hide prompt panel
                panel.classList.remove('visible');
                return;
            }

            panel.classList.add('visible');

            // Helper to render options as styled divs with numbers
            function renderOptions(options) {
                return options.map((opt, i) => {
                    const num = i + 1;
                    const desc = opt.description
                        ? '<div class="prompt-option-desc">' + escapeHtml(opt.description) + '</div>'
                        : '';
                    return '<div class="prompt-option" onclick="sendPromptAnswer(\\'' + sessionId + '\\', \\'' + opt.value + '\\')">' +
                           '<span class="prompt-option-number">' + num + '.</span>' +
                           '<span class="prompt-option-label">' + escapeHtml(opt.label) + '</span>' +
                           desc +
                           '</div>';
                }).join('');
            }

            if (prompt.prompt_type === 'question') {
                // AskUserQuestion prompt
                const q = prompt.questions[0];
                headerEl.textContent = q.header || 'Question';
                questionEl.textContent = q.question;

                // Render options
                optionsEl.innerHTML = renderOptions(q.options);

                // Show "Other" input if allowed
                if (q.allows_other !== false) {
                    otherEl.style.display = 'flex';
                } else {
                    otherEl.style.display = 'none';
                }

            } else if (prompt.prompt_type === 'permission') {
                // Permission prompt
                headerEl.textContent = 'Permission: ' + prompt.tool_name;

                // Show tool info
                let desc = 'Allow this action?';
                if (prompt.tool_input?.command) desc = 'Command: ' + prompt.tool_input.command;
                else if (prompt.tool_input?.file_path) desc = 'File: ' + prompt.tool_input.file_path;
                else if (prompt.tool_input?.description) desc = prompt.tool_input.description;
                questionEl.textContent = desc;

                // Render options
                optionsEl.innerHTML = renderOptions(prompt.options);

                // Hide "Other" input for simple permission prompts
                // Show only if there's a "Tab to add additional instructions" hint (detected by allows_other)
                otherEl.style.display = prompt.allows_other ? 'flex' : 'none';

            } else if (prompt.prompt_type === 'exit_plan') {
                // ExitPlanMode prompt - options parsed from screen
                headerEl.textContent = 'Exit Plan Mode';
                questionEl.textContent = 'Choose how to proceed:';

                // Render options
                optionsEl.innerHTML = renderOptions(prompt.options);

                // Hide "Other" input for simple exit plan prompts
                otherEl.style.display = prompt.allows_other ? 'flex' : 'none';
            }
        }

        function scrollToSession(sessionId) {
            const card = document.getElementById('session-' + sessionId);
            if (card) {
                card.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        }

        async function sendPromptAnswer(sessionId, value) {
            console.log('[sendPromptAnswer] called for session:', sessionId, 'value:', value);
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text: value })
                });

                if (handleAuthFailure(resp)) return;
                if (resp.ok) {
                    console.log('[sendPromptAnswer] success');
                    // Hide prompt panel immediately for responsive feel
                    const panel = document.getElementById('prompt-' + sessionId);
                    if (panel) panel.classList.remove('visible');
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    console.error('Failed to send prompt answer:', err);
                }
            } catch (err) {
                console.error('Failed to send prompt answer:', err);
            }
        }

        async function sendOtherAnswer(sessionId) {
            console.log('[sendOtherAnswer] called for session:', sessionId);
            const input = document.getElementById('prompt-input-' + sessionId);
            console.log('[sendOtherAnswer] input element:', input);
            const text = input?.value?.trim();
            console.log('[sendOtherAnswer] text value:', text);
            if (!text) {
                console.log('[sendOtherAnswer] empty text, returning');
                return;
            }

            try {
                console.log('[sendOtherAnswer] sending:', text);
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text: text })
                });

                if (handleAuthFailure(resp)) return;
                if (resp.ok) {
                    console.log('[sendOtherAnswer] success');
                    input.value = '';
                    input.blur(); // Hide mobile keyboard
                    // Hide prompt panel
                    const panel = document.getElementById('prompt-' + sessionId);
                    if (panel) panel.classList.remove('visible');
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    console.error('Failed to send other answer:', err);
                }
            } catch (err) {
                console.error('Failed to send other answer:', err);
            }
        }

        async function sendAnswer(sessionId) {
            const input = document.getElementById('input-' + sessionId);
            const text = input.value.trim();
            if (!text) return;

            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text })
                });

                if (handleAuthFailure(resp)) return;
                if (resp.ok) {
                    input.value = '';
                    input.blur(); // Hide mobile keyboard
                    // Cancel any pending debounced save to prevent race condition
                    if (inputSaveTimers.has(sessionId)) {
                        clearTimeout(inputSaveTimers.get(sessionId));
                        inputSaveTimers.delete(sessionId);
                    }
                    clearLocalInput(sessionId);
                    saveInputToServer(sessionId, ''); // Clear server draft too
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    alert('Error: ' + (err.error || 'Failed to send'));
                }
            } catch (err) {
                console.error('Failed to send answer:', err);
                alert('Failed to send: ' + err.message);
            }
        }

        function escapeHtml(text) {
            return text
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#39;');
        }

        // SSE connection for real-time session list updates with polling fallback
`;
