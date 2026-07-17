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

        function renderCommitHistory(commitHistory, withDivider) {
            if (!commitHistory || commitHistory.length === 0) return '';

            const commitWord = commitHistory.length === 1 ? 'commit' : 'commits';
            const entriesHtml = commitHistory.slice().reverse().map(commit => {
                const shortHash = commit.short_hash || (commit.hash || '').slice(0, 7);
                const age = commit.timestamp ? formatElapsed(commit.timestamp) : '';
                const date = commit.timestamp ? formatShortDate(commit.timestamp) : '';
                const subject = commit.subject || '(no commit message)';

                return \`<div class="commit-entry" title="\${escapeHtml(date)}">
                    <span class="commit-sha">\${escapeHtml(shortHash)}</span>
                    <span class="commit-subject">\${escapeHtml(subject)}</span>
                    <span class="commit-time">\${escapeHtml(age)}</span>
                </div>\`;
            }).join('');

            return \`<div class="commit-history\${withDivider ? ' with-divider' : ''}">
                <div class="commit-history-title">
                    <span>Commits</span>
                    <span>\${commitHistory.length} \${commitWord}</span>
                </div>
                <div class="commit-list">\${entriesHtml}</div>
            </div>\`;
        }

        function updateChangesWidget(sessionId, changes) {
            const widget = document.getElementById('changes-' + sessionId);
            const widgetsContent = document.getElementById('widgets-content-' + sessionId);
            if (!widget) return;

            const byLanguage = changes?.by_language || [];
            const sessionData = sessions.get(sessionId);
            const commitHistory = sessionData?.commitHistory || [];
            const totalChanges = byLanguage.reduce((sum, lang) => sum + (lang.changes?.length || 0), 0);
            const hasChanges = totalChanges > 0;
            const hasCommits = commitHistory.length > 0;

            if (!hasChanges && !hasCommits) {
                // Hide widget entirely when no changes or session commits exist.
                widget.classList.add('hidden-changes');
                widgetsContent?.classList.add('no-changes');
                return;
            }

            // Show widget when there are current changes or recorded commits.
            widget.classList.remove('hidden-changes');
            widgetsContent?.classList.remove('no-changes');

            // Build header: "Language N changes" (like CLI)
            const firstLang = byLanguage.find(lang => (lang.changes?.length || 0) > 0) || byLanguage[0];
            const changeWord = totalChanges === 1 ? 'change' : 'changes';
            const headerLabel = hasChanges ? firstLang.language : 'Commits';
            const headerCount = hasChanges
                ? totalChanges + ' ' + changeWord
                : commitHistory.length + ' ' + (commitHistory.length === 1 ? 'commit' : 'commits');

            // Compute column widths for alignment
            const { delNumWidth, addNumWidth } = computeChangesColumnWidths(byLanguage);

            let changesHtml = '';

            for (const lang of byLanguage) {
                if (!lang.changes || lang.changes.length === 0) continue;

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

            const commitHistoryHtml = renderCommitHistory(commitHistory, hasChanges);
            const bodyHtml = (hasChanges ? \`<div class="changes-list">\${changesHtml}</div>\` : '') + commitHistoryHtml;
            const newHtml = \`
                <div class="widget-title"><span style="color:#db6d28">\${headerLabel}</span> <span style="float:right;color:#8b949e">\${headerCount}</span></div>
                \${bodyHtml}
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
                case 'waiting': return { label: 'No recap', color: '#94a3b8' };
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
                const isExpanded = card.classList.contains('expanded');
                headlineEl.textContent = latest.headline || '';
                const bullets = isExpanded ? (latest.bullets || []) : (latest.bullets || []).slice(0, 2);
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
                headlineEl.textContent = 'Recaps appear after completed turns.';
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
            syncRecapHistoryVisibility(sessionId);
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
            const recapCard = document.getElementById('recap-' + sessionId);
            if (!recapCard) return;
            recapCard.classList.toggle('expanded');
            const sessionData = sessions.get(sessionId);
            if (sessionData?.recap) {
                updateRecapCard(sessionId, sessionData.recap);
            } else {
                syncRecapHistoryVisibility(sessionId);
            }
        }

        function syncRecapHistoryVisibility(sessionId) {
            const widget = document.getElementById('recaps-' + sessionId);
            const recapCard = document.getElementById('recap-' + sessionId);
            const sessionData = sessions.get(sessionId);
            if (!widget || !recapCard) return;

            const history = sessionData?.recapHistory || [];
            const recapHasContent = !recapCard.classList.contains('empty')
                && !recapCard.classList.contains('waiting')
                && !recapCard.classList.contains('disabled');
            widget.style.display = history.length > 0 && recapCard.classList.contains('expanded') && recapHasContent
                ? ''
                : 'none';
        }

        function updateRecapHistoryWidget(sessionId, history) {
            const widget = document.getElementById('recaps-' + sessionId);
            const countEl = document.getElementById('recap-count-' + sessionId);
            if (!widget) return;
            const sessionData = sessions.get(sessionId);
            if (sessionData) {
                sessionData.recapHistory = history || [];
            }
            if (!history || history.length === 0) {
                widget.style.display = 'none';
                if (countEl) countEl.textContent = '';
                return;
            }
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
            syncRecapHistoryVisibility(sessionId);
        }

        function prStateInfo(pr) {
            if (pr.state === 'MERGED') return { label: 'merged', color: '#a371f7' };
            if (pr.state === 'CLOSED') return { label: 'closed', color: '#f85149' };
            if (pr.is_draft) return { label: 'draft', color: '#8b949e' };
            // Softer green matching the dir path (xterm 114 in the CLI).
            if (pr.state === 'OPEN') return { label: 'open', color: '#87d787' };
            return { label: '', color: '#8b949e' };
        }

        // CI rollup badge: ✓ CI (all pass) / ✗N CI (failures) / ●N CI (pending).
        function prCiBadge(pr) {
            if (!pr.checks_total) return '';
            if (pr.checks_failed > 0) return '<span class="pr-ci fail">✗' + pr.checks_failed + ' CI</span>';
            if (pr.checks_pending > 0) return '<span class="pr-ci pending">●' + pr.checks_pending + ' CI</span>';
            return '<span class="pr-ci pass">✓ CI</span>';
        }

        // Merge cleanliness badge: conflicts / behind (needs update) / clean.
        function prMergeBadge(pr) {
            if (pr.mergeable === 'CONFLICTING') return '<span class="pr-merge conflict">conflicts</span>';
            if (pr.mergeable === 'MERGEABLE') {
                return pr.merge_state_status === 'BEHIND'
                    ? '<span class="pr-merge behind">behind</span>'
                    : '<span class="pr-merge clean">clean</span>';
            }
            return '';
        }

        function togglePrs(sessionId) {
            const sessionData = sessions.get(sessionId);
            if (!sessionData) return;
            sessionData.prsExpanded = !sessionData.prsExpanded;
            updatePrList(sessionId, sessionData.prs || []);
        }

        function updatePrList(sessionId, prs) {
            const widget = document.getElementById('recap-prs-' + sessionId);
            if (!widget) return;
            const sessionData = sessions.get(sessionId);
            if (sessionData) sessionData.prs = prs || [];
            // Collapsed by default: one line per PR. Click the header to expand
            // into the full view (PR title + branch).
            const expanded = !!(sessionData && sessionData.prsExpanded);

            if (!prs || prs.length === 0) {
                widget.style.display = 'none';
                widget.innerHTML = '';
                return;
            }

            const label = prs.length === 1 ? '1 PR' : prs.length + ' PRs';
            const rows = prs.map(pr => {
                const stateInfo = prStateInfo(pr);
                const repoLabel = (pr.repo || 'PR') + ' #' + pr.number;
                const badge = stateInfo.label
                    ? '<span class="pr-badge" style="color:' + stateInfo.color
                        + ';border-color:' + stateInfo.color + '">' + stateInfo.label + '</span>'
                    : '';
                const diffParts = [];
                if (pr.additions) diffParts.push('<span class="rd-add">+' + pr.additions + '</span>');
                if (pr.deletions) diffParts.push('<span class="rd-del">-' + pr.deletions + '</span>');
                const files = pr.changed_files
                    ? '<span class="pr-files">' + pr.changed_files + (pr.changed_files === 1 ? ' file' : ' files') + '</span>'
                    : '';
                const diff = (diffParts.length || files)
                    ? '<span class="pr-diff">' + diffParts.join(' ') + (diffParts.length && files ? ' ' : '') + files + '</span>'
                    : '';
                const link = '<a class="pr-link" href="' + escapeHtml(pr.url) + '" target="_blank" rel="noopener noreferrer">' + escapeHtml(repoLabel) + '</a>';
                // Right-hand status cluster: state, CI, merge cleanliness.
                const status = '<span class="pr-status">' + badge + prCiBadge(pr) + prMergeBadge(pr) + '</span>';

                if (!expanded) {
                    // One compact line: repo #num + diff on the left, status on the right.
                    return '<div class="pr-row pr-collapsed"><div class="pr-row-top">' + link + diff + status + '</div></div>';
                }
                const branch = pr.branch
                    ? '<span class="pr-branch" title="' + escapeHtml(pr.branch) + '">⎇ ' + escapeHtml(pr.branch) + '</span>'
                    : '';
                const title = pr.title ? '<div class="pr-title" title="' + escapeHtml(pr.title) + '">' + escapeHtml(pr.title) + '</div>' : '';
                return '<div class="pr-row">' +
                    '<div class="pr-row-top">' + link + diff + status + '</div>' +
                    title +
                    (branch ? '<div class="pr-row-bottom">' + branch + '</div>' : '') +
                    '</div>';
            }).join('');

            const chevron = expanded ? '▾' : '▸';
            const titleAttr = expanded ? 'Click to collapse' : 'Click to expand';
            widget.innerHTML = '<div class="pr-list-title" title="' + titleAttr + '">'
                + '<span class="pr-list-chevron">' + chevron + '</span>Pull requests '
                + '<span class="pr-list-count">' + label + '</span></div>' + rows;
            // Attach the toggle handler via JS (avoids escaping quotes inside the
            // outer template literal, which broke an inline onclick).
            const titleEl = widget.querySelector('.pr-list-title');
            if (titleEl) titleEl.onclick = () => togglePrs(sessionId);
            widget.style.display = '';
        }

        setInterval(() => {
            for (const [sessionId, sessionData] of sessions.entries()) {
                if (sessionData?.recap?.status === 'ready' && sessionData.recap.latest?.generated_at) {
                    updateRecapCard(sessionId, sessionData.recap);
                }
            }
        }, 30000);

`;
