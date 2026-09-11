// Dashboard JavaScript - prompt
export const promptJs = `
            const panel = document.getElementById('prompt-' + sessionId);
            const headerEl = document.getElementById('prompt-header-' + sessionId);
            const questionEl = document.getElementById('prompt-question-' + sessionId);
            const optionsEl = document.getElementById('prompt-options-' + sessionId);
            const otherEl = document.getElementById('prompt-other-' + sessionId);

            const shortId = sessionId.split('-')[0];

            if (!panel) return;

            if (!prompt) {
                // Clear/hide prompt panel
                console.log('%c[' + shortId + '] prompt', 'color:#f97316;font-weight:bold', 'cleared');
                panel.classList.remove('visible');
                return;
            }

            panel.classList.add('visible');

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
                // AskUserQuestion prompt - may have multiple questions.
                // The desktop mirrors the page Claude Code shows on the
                // terminal (current_question, checked, custom_text,
                // cursor_row, review). Older desktops send only the
                // questions, so we also keep our own page counter.
                const questions = prompt.questions || [];
                let qIdx = sessionQuestionIndex.get(sessionId) || 0;

                // If this is a new prompt (different questions), reset state
                const prevPrompt = sessionPromptData.get(sessionId);
                if (!prevPrompt || prevPrompt.prompt_type !== 'question' ||
                    prevPrompt.questions?.length !== questions.length ||
                    prevPrompt.questions?.[0]?.question !== questions[0]?.question) {
                    qIdx = 0;
                    sessionQuestionIndex.set(sessionId, 0);
                    forgetChecked(sessionId);
                }

                if (typeof prompt.current_question === 'number' &&
                    prompt.current_question >= 0 && prompt.current_question < questions.length) {
                    if (prompt.current_question !== qIdx) forgetChecked(sessionId);
                    qIdx = prompt.current_question;
                    sessionQuestionIndex.set(sessionId, qIdx);
                }

                // Clamp index to valid range
                if (qIdx >= questions.length) {
                    qIdx = 0;
                    sessionQuestionIndex.set(sessionId, 0);
                }

                const q = questions[qIdx] || { question: '', options: [] };
                const reviewing = Array.isArray(prompt.review);

                if (questions.length > 1 || reviewing) {
                    headerEl.innerHTML = renderQuestionTabs(questions, qIdx, reviewing);
                } else {
                    headerEl.textContent = q.header || 'Question';
                }

                const otherInput = document.getElementById('prompt-input-' + sessionId);
                const otherButton = otherEl?.querySelector('button');
                questionEl.textContent = reviewing ? 'Review your answers' : q.question;

                if (reviewing) {
                    optionsEl.innerHTML = renderReviewPage(sessionId, prompt.review);
                    otherEl.style.display = 'none';
                } else if (q.multi_select) {
                    const checked = syncCheckedFromDesktop(sessionId, prompt);
                    optionsEl.innerHTML = renderMultiSelectOptions(sessionId, q.options, checked);
                    if (otherInput) {
                        otherInput.placeholder = 'Add your own answer (optional)';
                        // Show what was typed on the terminal unless the viewer is editing
                        if (prompt.custom_text && !otherInput.value && document.activeElement !== otherInput) {
                            otherInput.value = prompt.custom_text;
                        }
                    }
                    if (otherButton) otherButton.textContent = 'Submit';
                    otherEl.style.display = 'flex';
                } else {
                    optionsEl.innerHTML = renderQuestionOptions(sessionId, qIdx, q.options);
                    if (otherInput) otherInput.placeholder = 'Type your response...';
                    if (otherButton) otherButton.textContent = 'Send';
                    otherEl.style.display = q.allows_other !== false ? 'flex' : 'none';
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
                const options = prompt.options || [];
                if (options.length === 0) {
                    optionsEl.innerHTML = '<div class="prompt-option-error">No options available - check desktop</div>';
                } else {
                    optionsEl.innerHTML = renderOptions(options, prompt);
                }

                // Hide "Other" input when we have inline tab inputs
                // Only show if no tab instructions available
                otherEl.style.display = 'none';

            } else if (prompt.prompt_type === 'exit_plan') {
                // ExitPlanMode prompt - options parsed from screen
                headerEl.textContent = 'Exit Plan Mode';
                questionEl.textContent = 'Choose how to proceed:';

                // Render options (no tab instructions for exit plan)
                const options = prompt.options || [];
                if (options.length === 0) {
                    optionsEl.innerHTML = '<div class="prompt-option-error">No options available - check desktop</div>';
                } else {
                    optionsEl.innerHTML = renderOptions(options, null);
                }

                // Hide "Other" input for simple exit plan prompts
                otherEl.style.display = 'none';
            }

            // Store prompt data AFTER new-prompt comparison (so next call
            // compares the incoming prompt against this one, not itself)
            sessionPromptData.set(sessionId, prompt);
        }

        // Store prompt data for each session (for key sequence navigation)
        const sessionPromptData = new Map();

        // Track which question index we're on for multi-question prompts
        const sessionQuestionIndex = new Map();

        // Ticked rows of the multi-select page being shown (Set of row numbers)
        const sessionCheckedLocal = new Map();
        // Until this time, keep our own ticks over the desktop's (a click
        // was just sent and the screen has not caught up yet)
        const sessionCheckedPendingUntil = new Map();

        // Whether the desktop is mirroring the page on screen. Older
        // desktops send only the questions; then we drive the pages blind.
        function isScreenDriven(prompt) {
            return !!prompt && (typeof prompt.current_question === 'number' || Array.isArray(prompt.review));
        }

        function isGrokCard(prompt) {
            return !!prompt && prompt.ui === 'grok_card';
        }

        // Grok picks answers with 1-9 then a-f.
        function grokOptionKey(num) {
            if (num >= 1 && num <= 9) return String(num);
            if (num >= 10 && num <= 15) return String.fromCharCode('a'.charCodeAt(0) + (num - 10));
            return String(num);
        }

        function currentQuestion(sessionId) {
            const prompt = sessionPromptData.get(sessionId);
            const qIdx = sessionQuestionIndex.get(sessionId) || 0;
            return { prompt, q: prompt?.questions?.[qIdx] || null };
        }

        // Drop our optimistic ticks: the page changed or its answer was sent
        function forgetChecked(sessionId) {
            sessionCheckedLocal.delete(sessionId);
            sessionCheckedPendingUntil.delete(sessionId);
        }

        function hidePromptPanel(sessionId) {
            const panel = document.getElementById('prompt-' + sessionId);
            if (panel) panel.classList.remove('visible');
        }

        // Adopt the desktop's ticks unless a click of ours is still in flight
        function syncCheckedFromDesktop(sessionId, prompt) {
            const local = sessionCheckedLocal.get(sessionId) || new Set();
            if (!isScreenDriven(prompt)) {
                sessionCheckedLocal.set(sessionId, local);
                return local;
            }
            const remote = new Set(prompt.checked || []);
            const pendingUntil = sessionCheckedPendingUntil.get(sessionId) || 0;
            const same = remote.size === local.size && [...remote].every(n => local.has(n));
            if (same || Date.now() > pendingUntil) {
                sessionCheckedLocal.set(sessionId, remote);
                sessionCheckedPendingUntil.delete(sessionId);
                return remote;
            }
            return local;
        }

        // Tab strip like the terminal's: ☒ answered, ☐ pending, ✔ Submit
        function renderQuestionTabs(questions, qIdx, reviewing) {
            const tabs = questions.map((quest, i) => {
                const answered = reviewing || i < qIdx;
                const isCurrent = !reviewing && i === qIdx;
                return '<span class="question-tab' + (isCurrent ? ' current' : '') + '">' +
                       (answered ? '☒' : '☐') + ' ' + escapeHtml(quest.header || ('Q' + (i + 1))) + '</span>';
            });
            tabs.push('<span class="question-tab' + (reviewing ? ' current' : '') + '">✔ Submit</span>');
            return tabs.join(' ');
        }

        // Render options for AskUserQuestion with question index tracking
        function renderQuestionOptions(sessionId, qIdx, options) {
            return options.map((opt, i) => {
                const num = i + 1;
                const desc = opt.description
                    ? '<div class="prompt-option-desc">' + escapeHtml(opt.description) + '</div>'
                    : '';

                return '<div class="prompt-option" onclick="sendQuestionAnswer(\\'' + sessionId + '\\', ' + qIdx + ', \\'' + opt.value + '\\')">' +
                       '<span class="prompt-option-number">' + num + '.</span>' +
                       '<span class="prompt-option-label">' + escapeHtml(opt.label) + '</span>' +
                       desc +
                       '</div>';
            }).join('');
        }

        // Render a multi-select page: each option is a checkbox that toggles
        // in the terminal as soon as it is clicked; Submit lives on the
        // input row below.
        function renderMultiSelectOptions(sessionId, options, checked) {
            const rows = options.map((opt, i) => {
                const num = i + 1;
                const isChecked = checked.has(num);
                const desc = opt.description
                    ? '<div class="prompt-option-desc">' + escapeHtml(opt.description) + '</div>'
                    : '';
                return '<div class="prompt-option prompt-check' + (isChecked ? ' checked' : '') + '" ' +
                       'role="checkbox" aria-checked="' + isChecked + '" ' +
                       'onclick="toggleQuestionOption(\\'' + sessionId + '\\', ' + num + ')">' +
                       '<span class="prompt-option-number">' + num + '.</span>' +
                       '<span class="prompt-checkbox">' + (isChecked ? '✔' : '') + '</span>' +
                       '<span class="prompt-option-label">' + escapeHtml(opt.label) + '</span>' +
                       desc +
                       '</div>';
            });
            rows.push('<div class="prompt-submit-hint">Pick any that apply, then Submit</div>');
            return rows.join('');
        }

        // Render the "Review your answers" page with its two choices
        function renderReviewPage(sessionId, answers) {
            const items = (answers || []).map(a =>
                '<div class="prompt-review-item">' +
                '<div class="prompt-review-q">● ' + escapeHtml(a.question || '') + '</div>' +
                '<div class="prompt-review-a">→ ' + escapeHtml(a.answer || '') + '</div>' +
                '</div>'
            ).join('');
            const choice = (num, label) =>
                '<div class="prompt-option" onclick="sendReviewChoice(\\'' + sessionId + '\\', ' + num + ')">' +
                '<span class="prompt-option-number">' + num + '.</span>' +
                '<span class="prompt-option-label">' + label + '</span>' +
                '</div>';
            return '<div class="prompt-review">' + items + '</div>' +
                   '<div class="prompt-review-ask">Ready to submit your answers?</div>' +
                   choice(1, 'Submit answers') + choice(2, 'Cancel');
        }

        async function postKeySequence(sessionId, steps) {
            const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/key-sequence', {
                method: 'POST',
                headers: getAuthHeaders(),
                body: JSON.stringify({ steps })
            });
            if (handleAuthFailure(resp)) return false;
            if (!resp.ok) {
                const err = await resp.json();
                console.error('Failed to send key sequence:', err);
                return false;
            }
            return true;
        }

        function moveSteps(from, to) {
            const steps = [];
            const key = to > from ? 'down' : 'up';
            for (let i = 0; i < Math.abs(to - from); i++) steps.push({ type: 'key', key });
            return steps;
        }

        // Backspace over the text the terminal already holds on a row
        function backspaceSteps(existing) {
            const steps = [];
            for (let i = 0; i < (existing || '').length; i++) steps.push({ type: 'key', key: 'backspace' });
            return steps;
        }

        // Digits type into the free-text row while the cursor sits on it,
        // so step off it before pressing one. Claude moves up; Grok leaves
        // edit mode with Escape. Records the move so later steps know
        // where the cursor ended up.
        function leaveTextRowSteps(prompt, q, grok) {
            const textRow = (q?.options?.length || 0) + 1;
            if (prompt?.cursor_row !== textRow) return [];
            if (grok) {
                prompt.cursor_row = Math.max(1, textRow - 1);
                return [{ type: 'key', key: 'escape' }];
            }
            prompt.cursor_row = textRow - 1;
            return [{ type: 'key', key: 'up' }];
        }

        // Tick or untick one row of a multi-select page. Digit keys toggle
        // the row without moving the cursor, so the click lands right away.
        async function toggleQuestionOption(sessionId, num) {
            const { prompt, q } = currentQuestion(sessionId);
            if (!q) return;
            const checked = sessionCheckedLocal.get(sessionId) || new Set();
            if (checked.has(num)) checked.delete(num); else checked.add(num);
            sessionCheckedLocal.set(sessionId, checked);
            sessionCheckedPendingUntil.set(sessionId, Date.now() + 1500);
            const optionsEl = document.getElementById('prompt-options-' + sessionId);
            if (optionsEl) optionsEl.innerHTML = renderMultiSelectOptions(sessionId, q.options, checked);

            const grok = isGrokCard(prompt);
            const steps = leaveTextRowSteps(prompt, q, grok);
            if (grok) {
                const cursor = typeof prompt.cursor_row === 'number' ? prompt.cursor_row : 1;
                steps.push(...moveSteps(cursor, num));
                prompt.cursor_row = num;
                steps.push({ type: 'key', key: 'space' });
            } else {
                steps.push({ type: 'text', text: String(num) });
            }
            try {
                await postKeySequence(sessionId, steps);
            } catch (err) {
                console.error('Failed to toggle option:', err);
            }
        }

        // Submit a multi-select page: put any custom text on the "Type
        // something" row, then move to the Submit row and press Enter. The
        // desktop then shows the next page or the review page.
        async function submitQuestionSelections(sessionId) {
            const { prompt, q } = currentQuestion(sessionId);
            if (!q) return;
            const input = document.getElementById('prompt-input-' + sessionId);
            const text = input?.value?.trim() || '';
            const numOptions = q.options?.length || 0;
            const textRow = numOptions + 1;
            const submitRow = numOptions + 2;
            const checked = sessionCheckedLocal.get(sessionId) || new Set();
            const existing = isScreenDriven(prompt) ? (prompt.custom_text || '') : '';
            let cursor = typeof prompt.cursor_row === 'number' ? prompt.cursor_row : 1;
            const grok = isGrokCard(prompt);

            const steps = [];
            if (grok) {
                steps.push(...leaveTextRowSteps(prompt, q, true));
                if (text && text !== existing) {
                    steps.push({ type: 'text', text: 'z' });
                    steps.push(...backspaceSteps(existing));
                    steps.push({ type: 'text', text });
                    steps.push({ type: 'key', key: 'escape' });
                }
                const qIdx = sessionQuestionIndex.get(sessionId) || 0;
                const last = qIdx + 1 >= (prompt.questions?.length || 0);
                steps.push({ type: 'key', key: last ? 'enter' : 'right' });
            } else {
                if (text) {
                    steps.push(...moveSteps(cursor, textRow));
                    cursor = textRow;
                    if (existing !== text) {
                        steps.push(...backspaceSteps(existing));
                        steps.push({ type: 'text', text });
                        steps.push({ type: 'delay', ms: 50 });
                    } else if (!checked.has(textRow)) {
                        steps.push({ type: 'key', key: 'enter' });
                    }
                } else if (existing && checked.has(textRow)) {
                    // The viewer cleared the text: untick what the terminal still holds
                    steps.push(...moveSteps(cursor, textRow));
                    cursor = textRow;
                    steps.push({ type: 'key', key: 'enter' });
                }
                steps.push(...moveSteps(cursor, submitRow));
                steps.push({ type: 'key', key: 'enter' });
            }

            try {
                if (!(await postKeySequence(sessionId, steps))) return;
                if (input) { input.value = ''; input.blur(); }
                forgetChecked(sessionId);
                if (!isScreenDriven(prompt)) advanceQuestionLocally(sessionId);
                scrollToSession(sessionId);
            } catch (err) {
                console.error('Failed to submit selections:', err);
            }
        }

        // "Submit answers" (1) or "Cancel" (2) on the review page; a digit
        // picks straight away on that menu.
        async function sendReviewChoice(sessionId, num) {
            try {
                if (!(await postKeySequence(sessionId, [{ type: 'text', text: String(num) }]))) return;
                hidePromptPanel(sessionId);
                sessionQuestionIndex.delete(sessionId);
                forgetChecked(sessionId);
                scrollToSession(sessionId);
            } catch (err) {
                console.error('Failed to send review choice:', err);
            }
        }

        // Without a desktop that mirrors the screen, count pages ourselves
        function advanceQuestionLocally(sessionId) {
            const prompt = sessionPromptData.get(sessionId);
            const qIdx = sessionQuestionIndex.get(sessionId) || 0;
            const total = prompt?.questions?.length || 0;
            if (qIdx + 1 < total) {
                sessionQuestionIndex.set(sessionId, qIdx + 1);
                updatePromptPanel(sessionId, prompt);
            } else {
                hidePromptPanel(sessionId);
                sessionQuestionIndex.delete(sessionId);
            }
        }

        // Pick one option of a single-select page. A digit selects it and
        // moves to the next page at once. Older desktops do not report the
        // cursor, so there we walk down from the first row and press Enter
        // (digits would leave a stray Enter for the next page).
        async function sendQuestionAnswer(sessionId, qIdx, value) {
            const optionIdx = parseInt(value);
            const prompt = sessionPromptData.get(sessionId);
            const q = prompt?.questions?.[qIdx];
            const screenDriven = isScreenDriven(prompt);
            const grok = isGrokCard(prompt);

            const steps = [];
            if (screenDriven || grok) {
                steps.push(...leaveTextRowSteps(prompt, q, grok));
                steps.push({ type: 'text', text: grok ? grokOptionKey(optionIdx) : String(optionIdx) });
            } else {
                for (let i = 1; i < optionIdx; i++) {
                    steps.push({ type: 'key', key: 'down' });
                }
                steps.push({ type: 'key', key: 'enter' });
            }

            try {
                if (!(await postKeySequence(sessionId, steps))) return;
                if (!screenDriven) advanceQuestionLocally(sessionId);
                scrollToSession(sessionId);
            } catch (err) {
                console.error('Failed to send question answer:', err);
            }
        }

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
            // Navigate to the target option, tab to open its instruction
            // input, type the instructions and submit. The delays give the
            // input time to appear and the text time to land.
            const steps = moveSteps(currentSelected, targetOption);
            steps.push({ type: 'key', key: 'tab' });
            steps.push({ type: 'delay', ms: 50 });
            steps.push({ type: 'text', text: instructions });
            steps.push({ type: 'delay', ms: 50 });
            steps.push({ type: 'key', key: 'enter' });

            try {
                if (!(await postKeySequence(sessionId, steps))) return;
                // Hide prompt panel immediately for responsive feel
                hidePromptPanel(sessionId);
                const inputEl = document.getElementById('tab-input-' + sessionId + '-' + targetOption);
                if (inputEl) inputEl.value = '';
                // Scroll to top of session so user can see what's happening
                scrollToSession(sessionId);
            } catch (err) {
                console.error('Failed to send key sequence:', err);
            }
        }

        async function sendPromptAnswer(sessionId, value) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text: value })
                });

                if (handleAuthFailure(resp)) return;
                if (resp.ok) {
                    // Hide prompt panel immediately for responsive feel
                    hidePromptPanel(sessionId);
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

        // Send a custom "Other" / "Type something" answer for AskUserQuestion.
        // On a multi-select page the input row is the Submit row, so hand
        // over to submitQuestionSelections. On a single-select page, put
        // the cursor on the "Type something" row, type the text and press
        // Enter; that answers the page and moves to the next one.
        async function sendOtherAnswer(sessionId) {
            const { prompt, q } = currentQuestion(sessionId);
            if (prompt?.prompt_type === 'question' && q?.multi_select) {
                return submitQuestionSelections(sessionId);
            }

            const input = document.getElementById('prompt-input-' + sessionId);
            const text = input?.value?.trim();
            if (!text) return;

            const numOptions = q?.options?.length || 3;
            const textRow = numOptions + 1;
            const screenDriven = isScreenDriven(prompt);
            const grok = isGrokCard(prompt);

            const steps = [];
            if (grok) {
                steps.push({ type: 'text', text: 'z' });
                steps.push(...backspaceSteps(prompt.custom_text));
            } else if (screenDriven) {
                // The row's digit moves the cursor onto it; typing there fills it in
                if (prompt.cursor_row !== textRow) steps.push({ type: 'text', text: String(textRow) });
                steps.push(...backspaceSteps(prompt.custom_text));
            } else {
                steps.push(...moveSteps(1, textRow));
            }
            steps.push({ type: 'text', text: text });
            steps.push({ type: 'delay', ms: 50 });
            steps.push({ type: 'key', key: 'enter' });

            try {
                if (!(await postKeySequence(sessionId, steps))) return;
                input.value = '';
                input.blur(); // Hide mobile keyboard
                if (!screenDriven) advanceQuestionLocally(sessionId);
                scrollToSession(sessionId);
            } catch (err) {
                console.error('Failed to send other answer:', err);
            }
        }

        async function sendAnswer(sessionId) {
            const input = document.getElementById('input-' + sessionId);
            // Use typed text, or fall back to suggestion placeholder
            const text = input.value.trim() || (inputSuggestions.get(sessionId) || '');
            if (!text) return;

            const restoreFailedSend = () => {
                input.value = text;
                resizeMessageInput(input);
                saveInputLocally(sessionId, text);
                updateSendButton(sessionId);
            };

            // Cancel any pending debounced save
            if (inputSaveTimers.has(sessionId)) {
                clearTimeout(inputSaveTimers.get(sessionId));
                inputSaveTimers.delete(sessionId);
            }

            // Clear input and cache BEFORE sending (optimistically)
            // This prevents stale text from being restored if page reloads mid-send
            input.value = '';
            resizeMessageInput(input);
            input.blur();
            clearLocalInput(sessionId);
            saveInputToServer(sessionId, '');
            updateSendButton(sessionId);

            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/answer', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text })
                });

                if (handleAuthFailure(resp)) {
                    // Auth failure - restore input so user can retry after re-auth
                    restoreFailedSend();
                    return;
                }
                if (resp.ok) {
                    // Scroll to top of session so user can see what's happening
                    scrollToSession(sessionId);
                } else {
                    // Server error - restore input so user can retry
                    restoreFailedSend();
                    const err = await resp.json();
                    alert('Error: ' + (err.error || 'Failed to send'));
                }
            } catch (err) {
                // Network error - restore input so user can retry
                restoreFailedSend();
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

        // The session list stream keeps polling as a fallback.
`;
