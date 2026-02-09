// Dashboard JavaScript - input
export const inputJs = `
        // Input preservation - save to localStorage and server
        const inputCache = new Map(); // sessionId -> text
        const localSaveTimers = new Map(); // sessionId -> timer

        function saveInputLocally(sessionId, text) {
            inputCache.set(sessionId, text);
            if (localSaveTimers.has(sessionId)) clearTimeout(localSaveTimers.get(sessionId));
            localSaveTimers.set(sessionId, setTimeout(() => {
                try {
                    const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                    stored[sessionId] = text;
                    localStorage.setItem('crabigator-inputs', JSON.stringify(stored));
                } catch {}
                localSaveTimers.delete(sessionId);
            }, 300));
        }

        function getLocalInput(sessionId) {
            try {
                const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                return stored[sessionId] || '';
            } catch { return ''; }
        }

        function clearLocalInput(sessionId) {
            inputCache.delete(sessionId);
            try {
                const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                delete stored[sessionId];
                localStorage.setItem('crabigator-inputs', JSON.stringify(stored));
            } catch {}
        }

        async function saveInputToServer(sessionId, text) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/draft', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({ text })
                });
                if (resp) handleAuthFailure(resp);
            } catch {}
        }

        async function getServerInput(sessionId) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/draft', {
                    headers: getAuthHeaders()
                });
                if (handleAuthFailure(resp)) return '';
                if (resp.ok) {
                    const data = await resp.json();
                    return data.text || '';
                }
            } catch {}
            return '';
        }

        async function restoreInput(sessionId) {
            // Try local first, then server
            let text = getLocalInput(sessionId);
            if (!text) {
                text = await getServerInput(sessionId);
            }
            if (text) {
                const input = document.getElementById('input-' + sessionId);
                if (input && !input.value) {
                    input.value = text;
                }
            }
            updateSendButton(sessionId);
        }

        // Track current suggestion per session
        const inputSuggestions = new Map(); // sessionId -> suggestion text

        function updateInputSuggestion(sessionId, suggestion) {
            const input = document.getElementById('input-' + sessionId);
            if (!input) return;

            inputSuggestions.set(sessionId, suggestion || '');

            if (suggestion) {
                // Show suggestion as placeholder (gray ghost text)
                input.placeholder = suggestion;
            } else {
                // Suggestion cleared — restore default placeholder
                input.placeholder = 'Send a message...';
            }
            updateSendButton(sessionId);
        }

        function handleInputKeydown(event, sessionId) {
            if (event.key === 'Enter') {
                sendAnswer(sessionId);
            } else if (event.key === 'Tab') {
                const suggestion = inputSuggestions.get(sessionId);
                if (suggestion && !event.target.value.trim()) {
                    event.preventDefault();
                    event.target.value = suggestion;
                    handleInputChange(sessionId, suggestion);
                }
            }
        }

        function updateSendButton(sessionId) {
            const input = document.getElementById('input-' + sessionId);
            const btn = document.getElementById('send-btn-' + sessionId);
            if (!input || !btn) return;
            const hasContent = input.value.trim().length > 0 || !!(inputSuggestions.get(sessionId));
            btn.disabled = !hasContent;
        }

        // Debounce server saves
        const inputSaveTimers = new Map();
        function handleInputChange(sessionId, text) {
            // Always save locally immediately
            saveInputLocally(sessionId, text);
            updateSendButton(sessionId);

            // Debounce server save (500ms)
            if (inputSaveTimers.has(sessionId)) {
                clearTimeout(inputSaveTimers.get(sessionId));
            }
            inputSaveTimers.set(sessionId, setTimeout(() => {
                saveInputToServer(sessionId, text);
                inputSaveTimers.delete(sessionId);
            }, 500));
        }

`;
