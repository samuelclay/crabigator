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

            // Store prompt data for key sequence navigation
            sessionPromptData.set(sessionId, prompt);

            // Helper to render options as styled divs with numbers
            // For permissions with allows_tab_instructions, adds inline inputs on options 1 (Yes) and 3 (No)
            function renderOptions(options, promptData) {
                const allowsTab = promptData?.allows_tab_instructions === true;
                const selectedOption = promptData?.selected_option || 1;

                return options.map((opt, i) => {
                    const num = parseInt(opt.value) || (i + 1);
                    const desc = opt.description
                        ? '<div class="prompt-option-desc">' + escapeHtml(opt.description) + '</div>'
                        : '';

                    // Check if this option should have a tab input (Yes or No, not "allow all edits")
                    const isYesOrNo = (num === 1 || num === 3) && allowsTab;
                    const tabInputId = 'tab-input-' + sessionId + '-' + num;

                    if (isYesOrNo) {
                        // Wrap option + input + send button in a row
                        const sendBtnId = 'tab-send-' + sessionId + '-' + num;
                        return '<div class="prompt-option-row">' +
                               '<div class="prompt-option" onclick="handleOptionClick(\\'' + sessionId + '\\', ' + num + ', ' + selectedOption + ')">' +
                               '<span class="prompt-option-number">' + num + '.</span>' +
                               '<span class="prompt-option-label">' + escapeHtml(opt.label) + '</span>' +
                               desc +
                               '</div>' +
                               '<div class="prompt-tab-wrapper">' +
                               '<input type="text" class="prompt-tab-input" id="' + tabInputId + '" ' +
                               'placeholder="+ instructions" onclick="event.stopPropagation()" ' +
                               'oninput="toggleTabSendButton(\\'' + sessionId + '\\', ' + num + ')" ' +
                               'onkeydown="if(event.key===\\'Enter\\'){handleOptionClick(\\'' + sessionId + '\\', ' + num + ', ' + selectedOption + ');event.preventDefault();}">' +
                               '<button type="button" class="prompt-tab-send" id="' + sendBtnId + '" ' +
                               'onclick="event.stopPropagation();handleOptionClick(\\'' + sessionId + '\\', ' + num + ', ' + selectedOption + ')" ' +
                               'style="display:none">Send</button>' +
                               '</div>' +
                               '</div>';
                    }

                    // Regular option without tab input
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

                // Render options (no tab instructions for questions)
                optionsEl.innerHTML = renderOptions(q.options, null);

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

                // Render options with tab instruction inputs
                optionsEl.innerHTML = renderOptions(prompt.options, prompt);

                // Hide "Other" input when we have inline tab inputs
                // Only show if no tab instructions available
                otherEl.style.display = 'none';

            } else if (prompt.prompt_type === 'exit_plan') {
                // ExitPlanMode prompt - options parsed from screen
                headerEl.textContent = 'Exit Plan Mode';
                questionEl.textContent = 'Choose how to proceed:';

                // Render options (no tab instructions for exit plan)
                optionsEl.innerHTML = renderOptions(prompt.options, null);

                // Hide "Other" input for simple exit plan prompts
                otherEl.style.display = 'none';
            }
        }

        // Store prompt data for each session (for key sequence navigation)
        const sessionPromptData = new Map();

        // Toggle the visibility of the Send button based on input content
        function toggleTabSendButton(sessionId, optionNum) {
            const inputEl = document.getElementById('tab-input-' + sessionId + '-' + optionNum);
            const sendBtn = document.getElementById('tab-send-' + sessionId + '-' + optionNum);
            if (!inputEl || !sendBtn) return;

            const hasText = inputEl.value.trim().length > 0;
            sendBtn.style.display = hasText ? 'block' : 'none';
        }

        // Handle option click - check for tab instructions first
        async function handleOptionClick(sessionId, targetOption, currentSelected) {
            const inputEl = document.getElementById('tab-input-' + sessionId + '-' + targetOption);
            const instructions = inputEl?.value?.trim();

            if (!instructions) {
                // No instructions - just send the option value directly
                sendPromptAnswer(sessionId, String(targetOption));
                return;
            }

            // Has instructions - need to send key sequence
            await sendWithInstructions(sessionId, targetOption, currentSelected, instructions);
        }

        // Send option with additional instructions using key sequence
        async function sendWithInstructions(sessionId, targetOption, currentSelected, instructions) {
            console.log('[sendWithInstructions]', { sessionId, targetOption, currentSelected, instructions });

            const steps = [];

            // Navigate to target option if needed
            const delta = targetOption - currentSelected;
            if (delta > 0) {
                for (let i = 0; i < delta; i++) {
                    steps.push({ type: 'key', key: 'down' });
                }
            } else if (delta < 0) {
                for (let i = 0; i < Math.abs(delta); i++) {
                    steps.push({ type: 'key', key: 'up' });
                }
            }

            // Tab to open instruction input
            steps.push({ type: 'key', key: 'tab' });

            // Small delay for the input to appear
            steps.push({ type: 'delay', ms: 50 });

            // Type the instructions
            steps.push({ type: 'text', text: instructions });

            // Small delay before enter
            steps.push({ type: 'delay', ms: 50 });

            // Press enter to submit
            steps.push({ type: 'key', key: 'enter' });

            console.log('[sendWithInstructions] sending steps:', steps);

            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/key-sequence', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ steps })
                });

                if (handleAuthFailure(resp)) return;
                if (resp.ok) {
                    console.log('[sendWithInstructions] success');
                    // Hide prompt panel immediately for responsive feel
                    const panel = document.getElementById('prompt-' + sessionId);
                    if (panel) panel.classList.remove('visible');
                    // Clear the input
                    const inputEl = document.getElementById('tab-input-' + sessionId + '-' + targetOption);
                    if (inputEl) inputEl.value = '';
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    const err = await resp.json();
                    console.error('Failed to send key sequence:', err);
                }
            } catch (err) {
                console.error('Failed to send key sequence:', err);
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

            // Cancel any pending debounced save
            if (inputSaveTimers.has(sessionId)) {
                clearTimeout(inputSaveTimers.get(sessionId));
                inputSaveTimers.delete(sessionId);
            }

            // Clear input and cache BEFORE sending (optimistically)
            // This prevents stale text from being restored if page reloads mid-send
            input.value = '';
            input.blur();
            clearLocalInput(sessionId);
            saveInputToServer(sessionId, '');

            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text })
                });

                if (handleAuthFailure(resp)) {
                    // Auth failure - restore input so user can retry after re-auth
                    input.value = text;
                    saveInputLocally(sessionId, text);
                    return;
                }
                if (resp.ok) {
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    // Server error - restore input so user can retry
                    input.value = text;
                    saveInputLocally(sessionId, text);
                    const err = await resp.json();
                    alert('Error: ' + (err.error || 'Failed to send'));
                }
            } catch (err) {
                // Network error - restore input so user can retry
                input.value = text;
                saveInputLocally(sessionId, text);
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
