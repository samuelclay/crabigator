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

        function formatSlackDate(postedAt) {
            if (!postedAt) return '';
            const date = new Date(postedAt * 1000);
            if (Number.isNaN(date.getTime())) return '';
            return new Intl.DateTimeFormat([], {
                month: 'short',
                day: 'numeric',
                hour: 'numeric',
                minute: '2-digit',
            }).format(date);
        }

        function renderSlackThreads(threads) {
            if (!threads || threads.length === 0) return '';
            const rows = threads.map(thread => {
                const date = formatSlackDate(thread.posted_at);
                const channelId = String(thread.url || '').match(/\\/archives\\/([^/]+)/)?.[1] || '';
                const channel = String(thread.channel || channelId).replace(/^#+/, '');
                const channelLabel = channel ? '#' + channel : '';
                const details = [date, channelLabel, thread.author].filter(Boolean).join(' · ');
                const label = details ? 'Slack · ' + details : 'Slack';
                const href = escapeHtml(thread.url);
                const link = '<a class="slack-thread" href="' + href + '" target="_blank" rel="noopener">'
                    + escapeHtml(label) + '</a>';
                const text = escapeHtml(String(thread.text || '').trim());
                const snippet = text
                    ? '<a class="slack-thread-text" href="' + href + '" target="_blank" rel="noopener" title="'
                        + text + '">' + text + '</a>'
                    : '';
                return link + snippet;
            }).join('');
            return '<div class="slack-threads">' + rows + '</div>';
        }

        function updateChangesWidget(sessionId, changes) {
            const widget = document.getElementById('changes-' + sessionId);
            const widgetsContent = document.getElementById('widgets-content-' + sessionId);
            if (!widget) return;

            // Hidden sections contribute no data: with both toggled off the
            // widget hides itself through the existing empty-state path below.
            const byLanguage = visibleSections.changes ? (changes?.by_language || []) : [];
            const sessionData = sessions.get(sessionId);
            const commitHistory = visibleSections.commits ? (sessionData?.commitHistory || []) : [];
            const slackThreads = sessionData?.slackThreads || [];
            const titleHierarchy = sessionTitleHierarchy(sessionData);
            const totalChanges = byLanguage.reduce((sum, lang) => sum + (lang.changes?.length || 0), 0);
            const hasChanges = totalChanges > 0;
            const hasCommits = commitHistory.length > 0;
            const hasSlackThreads = slackThreads.length > 0;
            const hasTitle = !!titleHierarchy.main;

            if (!hasChanges && !hasCommits && !hasSlackThreads && !hasTitle) {
                // Hide widget entirely when it has no title, links, changes, or commits.
                widget.classList.add('hidden-changes');
                widgetsContent?.classList.add('no-changes');
                return;
            }

            // Show the widget when it has a title, Slack link, change, or commit.
            widget.classList.remove('hidden-changes');
            widgetsContent?.classList.remove('no-changes');

            // Build header: "Language N changes" (like CLI)
            const firstLang = byLanguage.find(lang => (lang.changes?.length || 0) > 0) || byLanguage[0];
            const changeWord = totalChanges === 1 ? 'change' : 'changes';
            const headerLabel = hasChanges ? firstLang.language : hasSlackThreads ? 'Slack' : 'Commits';
            const headerCount = hasChanges
                ? totalChanges + ' ' + changeWord
                : hasSlackThreads
                    ? slackThreads.length + ' thread' + (slackThreads.length === 1 ? '' : 's')
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
                    // Names may carry a "parent › name" scope chain — mute the parents
                    const nameParts = String(c.name || '').split(' › ');
                    const shortName = nameParts.pop();
                    const scopeHtml = nameParts.length
                        ? \`<span class="name-scope">\${escapeHtml(nameParts.join(' › '))} › </span>\`
                        : '';
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
                        <span class="name">\${scopeHtml}\${escapeHtml(shortName)}</span>
                        <span class="stats" style="margin-left:auto">\${delNumHtml}&nbsp;\${addNumHtml}</span>
                    </div>\`;
                }
            }

            const slackThreadsHtml = renderSlackThreads(slackThreads);
            const commitHistoryHtml = renderCommitHistory(commitHistory, hasChanges || hasSlackThreads);
            const mainTitleClass = titleHierarchy.hasOfficial
                ? 'changes-pr-title'
                : 'changes-generated-title main';
            const generatedTitleHtml = titleHierarchy.generated
                ? '<div class="changes-generated-title">' + escapeHtml(titleHierarchy.generated) + '</div>'
                : '';
            const sessionTitlesHtml = titleHierarchy.main
                ? '<div class="changes-session-titles">'
                    + '<div class="' + mainTitleClass + '">' + escapeHtml(titleHierarchy.main) + '</div>'
                    + generatedTitleHtml
                    + '</div>'
                : '';
            const bodyHtml = slackThreadsHtml
                + (hasChanges ? \`<div class="changes-list\${hasSlackThreads ? ' with-divider' : ''}">\${changesHtml}</div>\` : '')
                + commitHistoryHtml;
            const contentHeaderHtml = hasChanges || hasSlackThreads || hasCommits
                ? '<div class="widget-title"><span style="color:#db6d28">' + escapeHtml(headerLabel)
                    + '</span> <span style="float:right;color:#8b949e">' + escapeHtml(headerCount) + '</span></div>'
                : '';
            const newHtml = sessionTitlesHtml + contentHeaderHtml + bodyHtml;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
        }

        function updateTitlesWidget(sessionId, titleHistory) {
            const widget = document.getElementById('titles-' + sessionId);
            const widgetsContent = document.getElementById('widgets-content-' + sessionId);
            if (!widget) return;

            const sessionData = sessions.get(sessionId);
            if (sessionData && titleHistory && titleHistory.length > 0) {
                const latest = titleHistory[titleHistory.length - 1];
                sessionData.title = latest;
                sessionData.generatedTitle = latest;
            }
            updateSessionTitleHierarchy(sessionId);

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

            const sessionData = sessions.get(sessionId);
            const history = sessionData?.recapHistory || [];
            // A new prompt intentionally clears recap.latest because the old
            // recap no longer describes the active turn. Keep that previous
            // recap available on the dashboard from its permanent history,
            // especially when someone already has the history expanded.
            const latest = recap.latest || history[history.length - 1] || null;
            const status = recap.status || 'waiting';
            const statusInfo = status === 'waiting' && latest
                ? { label: 'Previous recap', color: '#94a3b8' }
                : recapStatusLabel(status);
            card.classList.add(status.replace('_', '-'));
            card.classList.toggle('has-content', !!latest);

            statusEl.innerHTML = '<span class="rs-dot" style="background:' + statusInfo.color + '"></span>'
                + '<span class="rs-label" style="color:' + statusInfo.color + '">' + escapeHtml(statusInfo.label) + '</span>';

            if (latest && (status === 'ready' || status === 'waiting' || status === 'updating')) {
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
            widget.style.display = history.length > 0 && recapCard.classList.contains('expanded')
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
                if (sessionData?.recap) {
                    updateRecapCard(sessionId, sessionData.recap);
                }
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
            if (sessionData?.recap) {
                updateRecapCard(sessionId, sessionData.recap);
            } else {
                syncRecapHistoryVisibility(sessionId);
            }
        }

        function prStateInfo(pr) {
            if (pr.state === 'MERGED') return { label: 'merged', color: '#a371f7' };
            if (pr.state === 'CLOSED') return { label: 'closed', color: '#f85149' };
            if (pr.is_draft) return { label: 'draft', color: '#8b949e' };
            // Softer green matching the dir path (xterm 114 in the CLI).
            if (pr.state === 'OPEN') return { label: 'open', color: '#87d787' };
            // No state = never enriched. The desktop retries automatically;
            // say what's happening instead of leaving the row bare.
            if (pr.fetch_error) return { label: 'fetch failed', color: '#f85149', title: pr.fetch_error };
            return { label: 'fetching…', color: '#8b949e' };
        }

        // CI rollup badge: ✓ CI (all pass) / ✗N CI (failures) / ●N CI (pending).
        // Links to whatever the CLI resolved: the failing job when CI is red,
        // otherwise the PR's Checks tab.
        function prCiBadge(pr) {
            if (!pr.checks_total) return '';
            const cls = pr.checks_failed > 0 ? 'fail' : pr.checks_pending > 0 ? 'pending' : 'pass';
            const label = pr.checks_failed > 0
                ? '✗' + pr.checks_failed + ' CI'
                : pr.checks_pending > 0 ? '●' + pr.checks_pending + ' CI' : '✓ CI';
            return prExternalLink(pr.ci_url, 'pr-ci ' + cls, label);
        }

        // Unresolved review threads, linked to the first one. Blank when the
        // conversation is settled or the PR is no longer open.
        function prCommentsBadge(pr) {
            if (!pr.unresolved_comments) return '';
            return prExternalLink(pr.comments_url, 'pr-comments', '💬' + pr.unresolved_comments);
        }

        // Review approval state, open PRs only: ✓ approved, ✗ changes
        // requested, ⊘ a review dismissed by new commits, ◌ awaiting review.
        function prReviewBadge(pr) {
            if (pr.state !== 'OPEN') return '';
            let review = { glyph: '◌', cls: 'waiting', title: 'Awaiting review' };
            if (pr.review_decision === 'APPROVED') {
                review = { glyph: '✓', cls: 'approved', title: 'Approved' };
            } else if (pr.review_decision === 'CHANGES_REQUESTED') {
                review = { glyph: '✗', cls: 'changes', title: 'Changes requested' };
            } else if (pr.review_dismissed) {
                review = { glyph: '⊘', cls: 'dismissed', title: 'Approval dismissed by new commits' };
            }
            return '<span class="pr-review ' + review.cls + '" title="' + review.title + '">'
                + review.glyph + '</span>';
        }

        // A badge that opens a GitHub page when one is known, else plain text.
        function prExternalLink(url, className, innerHtml) {
            if (!url) return '<span class="' + className + '">' + innerHtml + '</span>';
            return '<a class="' + className + '" href="' + escapeHtml(url)
                + '" target="_blank" rel="noopener noreferrer">' + innerHtml + '</a>';
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

        // Canonical PR dispositions for the whole group, keyed owner/repo#number
        // and then by scope: '' for the group, 'session:<id>' for one session,
        // 'path:<cwd>' for every session in one worktree. The desktop already
        // applies these during classification, but loading them here lets a
        // fresh dashboard reflect toggles made elsewhere before the next prs
        // event arrives.
        const prOverrides = new Map();
        let prOverridesLoadStarted = false;
        async function loadPrOverrides() {
            try {
                const res = await fetch('/api/pr-overrides', { headers: getAuthHeaders() });
                if (!res.ok) return;
                const data = await res.json();
                prOverrides.clear();
                for (const o of (data.overrides || [])) {
                    const key = o.owner + '/' + o.repo + '#' + o.number;
                    if (!prOverrides.has(key)) prOverrides.set(key, new Map());
                    prOverrides.get(key).set(o.scope_key || '', o.disposition);
                }
                rerenderAllPrLists();
            } catch (e) { /* offline dashboards still render local state */ }
        }

        // The override one session sees: its own session scope beats its
        // worktree-path scope beats the group-wide row. With no session given
        // (board-level rows), only the group row counts.
        function prScopedOverride(pr, sessionData = null) {
            const scopes = prOverrides.get(prKey(pr));
            if (!scopes) return null;
            if (sessionData) {
                const bySession = sessionData.sessionId
                    ? scopes.get('session:' + sessionData.sessionId)
                    : null;
                if (bySession) return bySession;
                const byPath = sessionData.cwd ? scopes.get('path:' + sessionData.cwd) : null;
                if (byPath) return byPath;
                const byScope = sessionData.prScope ? scopes.get(sessionData.prScope) : null;
                if (byScope) return byScope;
            }
            return scopes.get('') || null;
        }

        function prKey(pr) {
            return pr.owner + '/' + pr.repo + '#' + pr.number;
        }

        // The same PR can sit in several session cards; a disposition change
        // must move it everywhere at once.
        function rerenderAllPrLists() {
            for (const [sid, sd] of sessions.entries()) {
                if (sd.prs && sd.prs.length) updatePrList(sid, sd.prs);
            }
        }

        function sessionRepositoryMatchesPr(sessionData, pr) {
            const git = sessionData?.git || {};
            const cwdDir = String(sessionData?.cwd || '').split('/').filter(Boolean).pop() || '';
            const repoName = git.repo_name || cwdDir;
            const ownerMatches = git.repo_owner
                ? String(git.repo_owner).toLowerCase() === String(pr.owner || '').toLowerCase()
                : !!pr.created_here;
            return ownerMatches
                && String(repoName).toLowerCase() === String(pr.repo || '').toLowerCase();
        }

        function prAttachedToSession(sessionData, pr) {
            const branch = String(pr.branch || '');
            if (!branch || ['main', 'master', 'develop'].includes(branch)) return false;
            if (!sessionRepositoryMatchesPr(sessionData, pr)) return false;
            if (branch === String(sessionData?.git?.branch || '')) return true;
            const worktree = String(sessionData?.cwd || '').split('/').filter(Boolean).pop() || '';
            return !!worktree && (branch === worktree || branch.endsWith('/' + worktree));
        }

        // Override wins. Current worktree ownership fills in automatic-primary
        // for sessions that started before this classification shipped.
        function prDisposition(pr, sessionData = null) {
            const override = prScopedOverride(pr, sessionData);
            if (override) return override;
            if (pr.dismissed) return 'dismissed';
            if (pr.primary) return 'primary';
            if (pr.primary_source === 'session' || pr.primary_source === 'override') {
                return 'secondary';
            }
            return sessionData && pr.state !== 'CLOSED' && prAttachedToSession(sessionData, pr)
                ? 'primary'
                : 'secondary';
        }

        function primaryPrForSession(sessionData) {
            const primaryPrs = (sessionData?.prs || []).filter(pr =>
                prDisposition(pr, sessionData) === 'primary' && String(pr.title || '').trim());
            primaryPrs.sort((a, b) =>
                (b.last_mentioned_at || 0) - (a.last_mentioned_at || 0)
                || (b.refreshed_at || 0) - (a.refreshed_at || 0)
                || (b.number || 0) - (a.number || 0));
            return primaryPrs[0] || null;
        }

        function stripGeneratedTitleMarker(title) {
            return String(title || '').replace(/^[ᛝ⟁↯✦▣]+\\s+/, '').trim();
        }

        function sessionTitleHierarchy(sessionData, fallbackTitle = '') {
            const generated = String(
                sessionData?.generatedTitle || sessionData?.title || fallbackTitle || ''
            ).trim();
            const primaryPr = primaryPrForSession(sessionData);
            const official = String(primaryPr?.title || '').trim();
            return {
                main: official || generated,
                generated: official && stripGeneratedTitleMarker(generated) !== official ? generated : '',
                hasOfficial: !!official,
            };
        }

        function updateSessionTitleHierarchy(sessionId) {
            const hierarchy = sessionTitleHierarchy(sessions.get(sessionId));
            const titleEl = document.getElementById('title-' + sessionId);
            if (titleEl) {
                titleEl.textContent = hierarchy.main;
                titleEl.classList.toggle('official', hierarchy.hasOfficial);
            }
            const generatedEl = document.getElementById('generated-title-' + sessionId);
            if (generatedEl) generatedEl.textContent = hierarchy.generated;
            const widgetsTitleEl = document.getElementById('widgets-title-' + sessionId);
            if (widgetsTitleEl) {
                widgetsTitleEl.textContent = hierarchy.main;
                widgetsTitleEl.classList.toggle('official', hierarchy.hasOfficial);
            }
        }

        async function postPrOverride(pr, disposition, scope = '') {
            const key = prKey(pr);
            if (!prOverrides.has(key)) prOverrides.set(key, new Map());
            prOverrides.get(key).set(scope, disposition);
            try {
                await fetch('/api/pr-overrides', {
                    method: 'POST',
                    headers: getAuthHeaders(),
                    body: JSON.stringify({
                        owner: pr.owner,
                        repo: pr.repo,
                        number: pr.number,
                        disposition: disposition,
                        scope: scope,
                    }),
                });
            } catch (e) { /* optimistic state stays; next load reconciles */ }
        }

        function updatePrList(sessionId, prs) {
            const sessionData = sessions.get(sessionId);
            if (sessionData) sessionData.prs = prs || [];
            updateSessionTitleHierarchy(sessionId);
            if (sessionData) {
                updateChangesWidget(sessionId, sessionData.changes || { by_language: [] });
            }
            scheduleSidebarUpdate();
            const widget = document.getElementById('recap-prs-' + sessionId);
            if (!widget) return;
            if (!prOverridesLoadStarted) {
                prOverridesLoadStarted = true;
                loadPrOverrides();
            }
            // Collapsed by default: only primary PRs. Clicking the header
            // reveals the secondary PRs too. Every row shows full detail
            // (PR title + branch) in both states.
            const expanded = !!(sessionData && sessionData.prsExpanded);

            // Dismissed PRs disappear; primaries render above secondaries.
            const disposition = pr => prDisposition(pr, sessionData);
            const visible = (prs || []).filter(pr => disposition(pr) !== 'dismissed');
            visible.sort((a, b) =>
                (disposition(b) === 'primary' ? 1 : 0) - (disposition(a) === 'primary' ? 1 : 0));

            if (visible.length === 0) {
                widget.style.display = 'none';
                widget.innerHTML = '';
                return;
            }

            const primaries = visible.filter(pr => disposition(pr) === 'primary');
            const secondaries = visible.filter(pr => disposition(pr) !== 'primary');
            const shown = expanded ? visible : primaries;

            const countLabel = n => n === 1 ? '1 PR' : n + ' PRs';
            // Collapsed with hidden secondaries: show what's hidden so the
            // chevron has a reason to be tapped.
            const label = expanded || !secondaries.length || !primaries.length
                ? countLabel(visible.length)
                : countLabel(primaries.length)
                    + ' <span class="pr-list-more">+' + secondaries.length + ' more</span>';
            const rows = shown.map(pr => {
                const stateInfo = prStateInfo(pr);
                const repoLabel = (pr.repo || 'PR') + ' #' + pr.number;
                const badge = stateInfo.label
                    ? '<span class="pr-badge" style="color:' + stateInfo.color
                        + ';border-color:' + stateInfo.color + '"'
                        + (stateInfo.title ? ' title="' + escapeHtml(stateInfo.title) + '"' : '')
                        + '>' + stateInfo.label + '</span>'
                    : '';
                const diffParts = [];
                if (pr.additions) diffParts.push('<span class="rd-add">+' + pr.additions + '</span>');
                if (pr.deletions) diffParts.push('<span class="rd-del">-' + pr.deletions + '</span>');
                const files = pr.changed_files
                    ? '<span class="pr-files">' + pr.changed_files + (pr.changed_files === 1 ? ' file' : ' files') + '</span>'
                    : '';
                // The diff cluster opens the PR's Files-changed tab.
                const diff = (diffParts.length || files)
                    ? prExternalLink(pr.url ? pr.url + '/files' : '', 'pr-diff',
                        diffParts.join(' ') + (diffParts.length && files ? ' ' : '') + files)
                    : '';
                const link = '<a class="pr-link" href="' + escapeHtml(pr.url) + '" target="_blank" rel="noopener noreferrer">' + escapeHtml(repoLabel) + '</a>';
                const isPrimary = disposition(pr) === 'primary';
                const star = '<span class="pr-primary-toggle ' + (isPrimary ? 'primary' : 'secondary')
                    + '" title="' + (isPrimary ? 'Primary — click to make secondary' : 'Secondary — click to make primary')
                    + '">' + (isPrimary ? '★' : '☆') + '</span>';
                // ↑ promotes a secondary PR, ↓ demotes a primary — the same
                // flip as the star, placed beside the dismiss.
                const flip = '<span class="pr-flip" title="' + (isPrimary ? 'Make secondary' : 'Make primary')
                    + '">' + (isPrimary ? '↓' : '↑') + '</span>';
                const dismissTitle = sessionData && String(sessionData.prScope || '').startsWith('path:')
                    ? 'Dismiss this PR from this worktree'
                    : 'Dismiss this PR from this session';
                const dismiss = '<span class="pr-dismiss" title="' + dismissTitle + '">✕</span>';
                // Right-hand status cluster: state, CI, merge cleanliness, then
                // the promote/demote and dismiss actions.
                const status = '<span class="pr-status">' + badge + prCiBadge(pr)
                    + prCommentsBadge(pr) + prReviewBadge(pr) + prMergeBadge(pr) + flip + dismiss + '</span>';

                const secondary = isPrimary ? '' : ' pr-secondary';
                const branch = pr.branch
                    ? '<span class="pr-branch" title="' + escapeHtml(pr.branch) + '">⎇ ' + escapeHtml(pr.branch) + '</span>'
                    : '';
                const title = pr.title ? '<div class="pr-title" title="' + escapeHtml(pr.title) + '">' + escapeHtml(pr.title) + '</div>' : '';
                return '<div class="pr-row' + secondary + '">' +
                    '<div class="pr-row-top">' + star + link + diff + status + '</div>' +
                    title +
                    (branch ? '<div class="pr-row-bottom">' + branch + '</div>' : '') +
                    '</div>';
            }).join('');

            const chevron = expanded ? '▾' : '▸';
            const titleAttr = expanded ? 'Click to collapse' : 'Click to expand';
            widget.innerHTML = '<div class="pr-list-title" title="' + titleAttr + '">'
                + '<span class="pr-list-chevron">' + chevron + '</span>Pull requests '
                + '<span class="pr-list-count">' + label + '</span></div>' + rows;
            // Attach handlers via JS (avoids escaping quotes inside the outer
            // template literal, which broke an inline onclick).
            // Every tap stops here: the session card behind this section has
            // its own tap targets and must not react to PR-section taps.
            widget.onclick = ev => ev.stopPropagation();
            const titleEl = widget.querySelector('.pr-list-title');
            if (titleEl) titleEl.onclick = ev => {
                ev.stopPropagation();
                togglePrs(sessionId);
            };
            // Actions taken on a session card apply in that session's scope,
            // so a dismissal here leaves the group's other sessions alone.
            const actionScope = (sessionData && sessionData.prScope)
                || ('session:' + sessionId);
            widget.querySelectorAll('.pr-row').forEach((rowEl, i) => {
                const pr = shown[i];
                if (!pr) return;
                const flipPr = ev => {
                    ev.stopPropagation();
                    postPrOverride(pr, disposition(pr) === 'primary' ? 'secondary' : 'primary', actionScope);
                    rerenderAllPrLists();
                };
                // The ★/☆ glyph and the ↑/↓ action both flip the PR.
                rowEl.querySelectorAll('.pr-primary-toggle, .pr-flip')
                    .forEach(el => { el.onclick = flipPr; });
                const dismissEl = rowEl.querySelector('.pr-dismiss');
                if (dismissEl) dismissEl.onclick = ev => {
                    ev.stopPropagation();
                    postPrOverride(pr, 'dismissed', actionScope);
                    rerenderAllPrLists();
                };
            });
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
