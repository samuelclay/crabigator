// Dashboard JavaScript - changes-widget
export const changesWidgetJs = `
        // Get icon and color for change type modifier (like CLI)
        function getModifierStyle(changeType) {
            switch (changeType) {
                case 'added': return { modifier: '+', color: '#3fb950' };
                case 'deleted': return { modifier: '-', color: '#f85149' };
                default: return { modifier: '~', color: '#d29922' };  // modified
            }
        }

        // Get icon and color for node kind (like CLI)
        function getKindIcon(kind) {
            switch (kind?.toLowerCase()) {
                case 'function':
                case 'method':
                    return { icon: 'ƒ', color: '#58a6ff' };
                case 'class':
                    return { icon: '◆', color: '#bc8cff' };
                case 'struct':
                    return { icon: '◇', color: '#39c5cf' };
                case 'enum':
                    return { icon: '▣', color: '#d29922' };
                case 'trait':
                    return { icon: '◈', color: '#bc8cff' };
                case 'impl':
                    return { icon: '◊', color: '#39c5cf' };
                case 'module':
                    return { icon: '□', color: '#8b949e' };
                case 'const':
                    return { icon: '•', color: '#8b949e' };
                default:
                    return { icon: '·', color: '#6e7681' };
            }
        }

        function updateChangesWidget(sessionId, changes) {
            const widget = document.getElementById('changes-' + sessionId);
            const widgetsContent = document.getElementById('widgets-content-' + sessionId);
            if (!widget) return;

            const byLanguage = changes.by_language || [];

            if (byLanguage.length === 0) {
                // Hide widget entirely when no changes
                widget.classList.add('hidden-changes');
                widgetsContent?.classList.add('no-changes');
                return;
            }

            // Show widget when there are changes
            widget.classList.remove('hidden-changes');
            widgetsContent?.classList.remove('no-changes');

            // Build header: "Language N changes" (like CLI)
            const firstLang = byLanguage[0];
            const totalChanges = byLanguage.reduce((sum, lang) => sum + (lang.changes?.length || 0), 0);
            const changeWord = totalChanges === 1 ? 'change' : 'changes';

            // Compute column widths for alignment
            const { delNumWidth, addNumWidth } = computeChangesColumnWidths(byLanguage);

            let changesHtml = '';

            for (const lang of byLanguage) {
                // Add language header if multiple languages
                if (byLanguage.length > 1) {
                    const langCount = lang.changes?.length || 0;
                    const langWord = langCount === 1 ? 'change' : 'changes';
                    changesHtml += \`<div style="color:#db6d28;margin-top:4px;font-size:11px">\${lang.language} <span style="color:#8b949e">\${langCount} \${langWord}</span></div>\`;
                }

                for (const c of (lang.changes || [])) {
                    const { modifier, color: modColor } = getModifierStyle(c.change_type);
                    const { icon, color: iconColor } = getKindIcon(c.kind);
                    const del = c.deletions || 0;
                    const add = c.additions || 0;

                    // Build deletion number (right-aligned)
                    const delNumStr = del > 0 ? '−' + del : '';
                    const delNumPad = delNumWidth - delNumStr.length;
                    const delNumHtml = delNumWidth > 0
                        ? \`<span style="color:#f85149">\${nbsp(delNumPad)}\${delNumStr}</span>\`
                        : '';

                    // Build addition number (left-aligned)
                    const addNumStr = add > 0 ? '+' + add : '';
                    const addNumPad = addNumWidth - addNumStr.length;
                    const addNumHtml = addNumWidth > 0
                        ? \`<span style="color:#3fb950">\${addNumStr}\${nbsp(addNumPad)}</span>\`
                        : '';

                    changesHtml += \`<div class="change-item">
                        <span style="color:\${modColor}">\${modifier}</span><span style="color:\${iconColor}">\${icon}</span>
                        <span class="name">\${c.name}</span>
                        <span class="stats" style="margin-left:auto">\${delNumHtml}&nbsp;\${addNumHtml}</span>
                    </div>\`;
                }
            }

            const newHtml = \`
                <div class="widget-title"><span style="color:#db6d28">\${firstLang.language}</span> <span style="float:right;color:#8b949e">\${totalChanges} \${changeWord}</span></div>
                <div class="changes-list">\${changesHtml}</div>
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
        }

        function updateTitlesWidget(sessionId, titleHistory) {
            const widget = document.getElementById('titles-' + sessionId);
            const widgetsContent = document.getElementById('widgets-content-' + sessionId);
            if (!widget) return;

            // Update widgets header with latest title
            const titleSummaryEl = document.getElementById('widgets-title-' + sessionId);
            if (titleSummaryEl && titleHistory && titleHistory.length > 0) {
                titleSummaryEl.textContent = titleHistory[titleHistory.length - 1];
            }

            // Hide title history widget if no titles or only one title
            if (!titleHistory || titleHistory.length <= 1) {
                widget.style.display = 'none';
                widgetsContent?.classList.add('no-titles');
                return;
            }

            // Show widget only when there are multiple titles (history)
            widget.style.display = '';
            widgetsContent?.classList.remove('no-titles');

            // Latest title is already in the header, show previous titles as history
            const latestTitle = titleHistory[titleHistory.length - 1];
            const escapedLatest = latestTitle.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
            const previousTitles = titleHistory.slice(0, -1);

            // Multiple titles - latest as title, previous as history (newest first)
            const historyHtml = previousTitles.slice().reverse().map(title => {
                const escaped = title.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
                return \`<div class="title-entry">\${escaped}</div>\`;
            }).join('');

            const newHtml = \`
                <div class="widget-title"><span style="color:#58a6ff">\${escapedLatest}</span> <span style="color:#8b949e">\${titleHistory.length} titles</span></div>
                <div class="titles-list">\${historyHtml}</div>
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
        }

        function escapeHtml(text) {
            return String(text || '')
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;');
        }

        function recapStatusLabel(status) {
            switch (status) {
                case 'ready': return { label: 'Recap', color: '#22d3ee' };
                case 'updating': return { label: 'Generating recap…', color: '#fbbf24' };
                case 'failed': return { label: 'Recap unavailable', color: '#fbbf24' };
                case 'missing_key': return { label: 'Recaps off', color: '#94a3b8' };
                case 'waiting': return { label: 'Awaiting first turn…', color: '#94a3b8' };
                case 'disabled': return { label: 'Recaps disabled', color: '#64748b' };
                default: return { label: 'Recap', color: '#22d3ee' };
            }
        }

        function formatLineDelta(delta) {
            if (!delta || (delta.additions === 0 && delta.deletions === 0)) return '';
            const parts = [];
            if (delta.additions) parts.push('<span class="rd-add">+' + delta.additions + '</span>');
            if (delta.deletions) parts.push('<span class="rd-del">-' + delta.deletions + '</span>');
            return '<span class="rd-delta">Δ ' + parts.join(' ') + '</span>';
        }

        function formatRecapMeta(recap, latest) {
            const generatedAtMs = latest?.generated_at;
            const age = generatedAtMs ? formatElapsed(generatedAtMs / 1000) : '';
            const delta = formatLineDelta(recap.line_delta || latest?.line_delta);

            if (age && delta) {
                return '<span class="rd-age">' + escapeHtml(age) + '</span>&nbsp;&nbsp;' + delta;
            }
            if (age) return '<span class="rd-age">' + escapeHtml(age) + '</span>';
            return delta;
        }

        function updateRecapCard(sessionId, recap) {
            const card = document.getElementById('recap-' + sessionId);
            if (!card) return;
            const statusEl = document.getElementById('recap-status-' + sessionId);
            const headlineEl = document.getElementById('recap-headline-' + sessionId);
            const bulletsEl = document.getElementById('recap-bullets-' + sessionId);
            const metaEl = document.getElementById('recap-meta-' + sessionId);
            if (!statusEl || !headlineEl || !bulletsEl || !metaEl) return;

            // Reset state classes — set the freshest one below.
            card.classList.remove('empty', 'ready', 'updating', 'failed', 'missing-key', 'waiting', 'disabled');

            const status = recap.status || 'waiting';
            const statusInfo = recapStatusLabel(status);
            card.classList.add(status.replace('_', '-'));

            statusEl.innerHTML = '<span class="rs-dot" style="background:' + statusInfo.color + '"></span>'
                + '<span class="rs-label" style="color:' + statusInfo.color + '">' + escapeHtml(statusInfo.label) + '</span>';

            if (status === 'ready' && recap.latest) {
                const latest = recap.latest;
                headlineEl.textContent = latest.headline || '';
                const bullets = (latest.bullets || []).slice(0, 2);
                bulletsEl.innerHTML = bullets.map(b => '<div class="rb-line">' + escapeHtml(b) + '</div>').join('');
                metaEl.innerHTML = formatRecapMeta(recap, latest);
            } else if (status === 'failed') {
                headlineEl.textContent = recap.error
                    ? extractFriendlyRecapError(recap.error)
                    : 'Recap call did not return.';
                bulletsEl.innerHTML = '<div class="rb-line rb-hint">Clears on next prompt — \`crabigator recap status\` for details.</div>';
                metaEl.innerHTML = '';
            } else if (status === 'updating') {
                headlineEl.textContent = 'Summarizing the latest turn…';
                bulletsEl.innerHTML = '';
                metaEl.innerHTML = '';
            } else if (status === 'missing_key') {
                headlineEl.textContent = 'Set ANTHROPIC_API_KEY (or run "crabigator key <key>") to enable per-turn recaps.';
                bulletsEl.innerHTML = '';
                metaEl.innerHTML = '';
            } else if (status === 'waiting') {
                headlineEl.textContent = 'Recaps will appear here after each completed turn.';
                bulletsEl.innerHTML = '';
                metaEl.innerHTML = '';
            } else if (status === 'disabled') {
                headlineEl.textContent = 'Per-turn recaps are disabled for this session.';
                bulletsEl.innerHTML = '';
                metaEl.innerHTML = '';
            } else {
                headlineEl.textContent = '';
                bulletsEl.innerHTML = '';
                metaEl.innerHTML = '';
                card.classList.add('empty');
            }
        }

        function extractFriendlyRecapError(raw) {
            if (typeof raw !== 'string') return String(raw);
            const cleaned = raw.replace(/\\n/g, ' ').trim();
            const start = cleaned.indexOf('{');
            if (start >= 0) {
                try {
                    const parsed = JSON.parse(cleaned.slice(start));
                    if (parsed?.error?.message) return parsed.error.message;
                    if (parsed?.message) return parsed.message;
                } catch (e) { /* fall through */ }
            }
            return cleaned;
        }

        function onRecapClick(sessionId, ev) {
            const card = document.getElementById('session-' + sessionId);
            if (!card) return;
            // Shift+Click jumps focus to the recap-history widget (full timeline);
            // a normal click just expands the card so the user sees terminal + bullets.
            const recapsWidget = document.getElementById('recaps-' + sessionId);
            const widgetsCard = document.querySelector('#widgets-' + sessionId);
            if (ev?.shiftKey && recapsWidget && recapsWidget.style.display !== 'none') {
                card.classList.remove('collapsed');
                if (widgetsCard) widgetsCard.classList.remove('collapsed');
                recapsWidget.scrollIntoView({ behavior: 'smooth', block: 'center' });
                return;
            }
            // Treat the recap card like the existing collapse toggle target.
            if (typeof toggleCollapse === 'function') {
                toggleCollapse(sessionId);
            } else {
                card.classList.toggle('collapsed');
            }
        }

        function updateRecapHistoryWidget(sessionId, history) {
            const widget = document.getElementById('recaps-' + sessionId);
            const countEl = document.getElementById('recap-count-' + sessionId);
            if (!widget) return;
            if (!history || history.length === 0) {
                widget.style.display = 'none';
                if (countEl) countEl.textContent = '';
                return;
            }
            widget.style.display = '';
            if (countEl) {
                countEl.textContent = history.length + ' recap' + (history.length === 1 ? '' : 's');
            }

            // Newest first. Each row mirrors the inline card layout but is more compact.
            const rowsHtml = history.slice().reverse().map(entry => {
                const headline = escapeHtml(entry.headline || '');
                const bullets = (entry.bullets || []).slice(0, 2)
                    .map(b => '<div class="rh-bullet">' + escapeHtml(b) + '</div>')
                    .join('');
                const delta = formatLineDelta(entry.line_delta);
                return '<div class="recap-entry">' +
                    '<div class="rh-headline">' + headline + '</div>' +
                    bullets +
                    (delta ? '<div class="rh-delta">' + delta + '</div>' : '') +
                    '</div>';
            }).join('');

            const list = widget.querySelector('.recaps-list');
            if (list) list.innerHTML = rowsHtml;
        }

        setInterval(() => {
            for (const [sessionId, sessionData] of sessions.entries()) {
                if (sessionData?.recap?.status === 'ready' && sessionData.recap.latest?.generated_at) {
                    updateRecapCard(sessionId, sessionData.recap);
                }
            }
        }, 30000);

`;
