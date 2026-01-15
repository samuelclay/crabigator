// Dashboard JavaScript - input
export const inputJs = `
        // Input preservation - save to localStorage and server
        const inputCache = new Map(); // sessionId -> text

        function saveInputLocally(sessionId, text) {
            inputCache.set(sessionId, text);
            try {
                const stored = JSON.parse(localStorage.getItem('crabigator-inputs') || '{}');
                stored[sessionId] = text;
                localStorage.setItem('crabigator-inputs', JSON.stringify(stored));
            } catch {}
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
                await fetch(API_BASE + '/sessions/' + sessionId + '/draft', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ text })
                });
            } catch {}
        }

        async function getServerInput(sessionId) {
            try {
                const resp = await fetch(API_BASE + '/sessions/' + sessionId + '/draft');
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
        }

        // Debounce server saves
        const inputSaveTimers = new Map();
        function handleInputChange(sessionId, text) {
            // Always save locally immediately
            saveInputLocally(sessionId, text);

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
