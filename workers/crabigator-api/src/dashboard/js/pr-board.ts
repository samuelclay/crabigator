// Dashboard JavaScript - cross-session PR board
// Swaps the session grid for one deduped view of every PR the group's
// sessions have tracked (D1-backed, so it survives session end). Reuses the
// PR badge helpers and override plumbing from changes-widget.ts.
export const prBoardJs = `
        let prBoardVisible = false;
        let prBoardTimer = null;
        let prBoardEntries = [];
        let prBoardSessions = [];

        function togglePrBoard() {
            prBoardVisible = !prBoardVisible;
            const board = document.getElementById('pr-board-view');
            const sessionsEl = document.getElementById('sessions');
            const btn = document.getElementById('pr-board-btn');
            if (board) board.hidden = !prBoardVisible;
            if (sessionsEl) sessionsEl.style.display = prBoardVisible ? 'none' : '';
            if (btn) btn.classList.toggle('active', prBoardVisible);
            if (prBoardVisible) {
                loadPrBoard();
                prBoardTimer = setInterval(loadPrBoard, 20000);
            } else if (prBoardTimer) {
                clearInterval(prBoardTimer);
                prBoardTimer = null;
            }
        }

        async function loadPrBoard() {
            const board = document.getElementById('pr-board-view');
            if (!board) return;
            try {
                const res = await fetch('/api/prs/board', { headers: getAuthHeaders() });
                if (!res.ok) {
                    board.innerHTML = '<div class="pr-board-empty">Could not load the PR board ('
                        + res.status + '). Pair this browser from a session first.</div>';
                    return;
                }
                const data = await res.json();
                prBoardEntries = data.prs || [];
                prBoardSessions = data.sessions || [];
                renderPrBoard();
            } catch (e) {
                board.innerHTML = '<div class="pr-board-empty">Could not load the PR board.</div>';
            }
        }

        // Current pipeline status for the row's detail line.
        function prBoardStage(pr) {
            // No state = never enriched; the desktop is still fetching (and
            // retries failures automatically).
            if (!pr.state) return pr.fetch_error
                ? { label: 'fetch failed, retrying', cls: 'bad' }
                : { label: 'fetching', cls: 'dim' };
            if (pr.state === 'MERGED') return { label: 'merged', cls: 'merged' };
            if (pr.state !== 'OPEN') return { label: 'closed', cls: 'dim' };
            if (pr.mergeable === 'CONFLICTING') return { label: 'conflicts', cls: 'bad' };
            if ((pr.checks_failed || 0) > 0) return { label: 'CI failing', cls: 'bad' };
            if (pr.review_decision === 'CHANGES_REQUESTED')
                return { label: 'changes requested', cls: 'bad' };
            if (pr.is_draft) return { label: 'draft', cls: 'dim' };
            if ((pr.checks_pending || 0) > 0) return { label: 'CI running', cls: 'warn' };
            if (pr.review_decision !== 'APPROVED') return { label: 'awaiting review', cls: 'warn' };
            return { label: 'ready to merge', cls: 'good' };
        }

        // Five-dot progress strip: draft → open → CI → review → merged.
        function prBoardChecklist(pr) {
            const dot = (cls, ch) => '<span class="pb-dot ' + cls + '">' + ch + '</span>';
            const opened = pr.state === 'CLOSED' ? dot('bad', '✗') : dot('good', '●');
            const ci = !pr.checks_total ? dot('dim', '○')
                : (pr.checks_failed || 0) > 0 ? dot('bad', '✗')
                : (pr.checks_pending || 0) > 0 ? dot('warn', '◐')
                : dot('good', '●');
            const review = pr.review_decision === 'APPROVED' ? dot('good', '●')
                : pr.review_decision === 'CHANGES_REQUESTED' ? dot('bad', '✗')
                : dot('dim', '○');
            const merged = pr.state === 'MERGED' ? dot('merged', '●') : dot('dim', '○');
            return '<span class="pb-checklist">' + dot('good', '●') + opened + ci + review + merged + '</span>';
        }

        function prBoardAge(unixSeconds) {
            if (!unixSeconds) return '';
            const secs = Math.max(0, Math.floor(Date.now() / 1000) - unixSeconds);
            if (secs < 60) return secs + 's';
            if (secs < 3600) return Math.floor(secs / 60) + 'm';
            if (secs < 86400) return Math.floor(secs / 3600) + 'h';
            return Math.floor(secs / 86400) + 'd';
        }

        function prBoardActivityTime(sessions) {
            const completed = Math.max(0, ...(sessions || []).map(s => s.completions_changed_at || 0));
            if (completed) return completed;
            return Math.max(0, ...(sessions || []).map(s => s.prompts_changed_at || 0));
        }

        function prBoardActivityLabel(sessions) {
            const prompted = Math.max(0, ...(sessions || []).map(s => s.prompts_changed_at || 0));
            const completed = Math.max(0, ...(sessions || []).map(s => s.completions_changed_at || 0));
            return '⟩ ' + (prompted ? prBoardAge(prompted) : '—')
                + ' · ⋖ ' + (completed ? prBoardAge(completed) : '—');
        }

        function renderPrBoard() {
            const board = document.getElementById('pr-board-view');
            if (!board) return;

            // Dispositions already applied server-side, but a toggle made just
            // now should reshape the board before the next fetch.
            const entries = prBoardEntries
                .filter(e => prDisposition(e.pr) !== 'dismissed')
                .map(e => ({ ...e, primary: prDisposition(e.pr) === 'primary', stage: prBoardStage(e.pr) }));

            const representedSessions = new Set();
            for (const entry of entries) {
                for (const session of entry.sessions || []) {
                    if (session.session_id && prBoardSessionMatchesPr(session, entry.pr)) {
                        representedSessions.add(session.session_id);
                    }
                }
            }
            const sessionRows = [];
            for (const session of prBoardSessions) {
                if (!session.active || representedSessions.has(session.session_id)) continue;
                const repoName = session.repo_name || session.dir_name || 'Unknown repository';
                const repo = session.repo_owner ? session.repo_owner + '/' + repoName : repoName;
                const branch = session.branch || '(no branch)';
                const activity = session.completions_changed_at || session.prompts_changed_at || 0;
                sessionRows.push({ repo, branch, session, activity });
            }

            if (entries.length === 0 && sessionRows.length === 0) {
                board.innerHTML = '<div class="pr-board-empty">No live sessions or tracked PRs.</div>';
                return;
            }

            const groups = new Map();
            for (const entry of entries) {
                const repo = entry.owner + '/' + entry.repo;
                const key = repo.toLowerCase();
                if (!groups.has(key)) groups.set(key, { repo, items: [] });
                groups.get(key).items.push({ kind: 'pr', entry, activity: prBoardActivityTime(entry.sessions) });
            }
            for (const sessionRow of sessionRows) {
                const key = sessionRow.repo.toLowerCase();
                if (!groups.has(key)) groups.set(key, { repo: sessionRow.repo, items: [] });
                groups.get(key).items.push({ kind: 'session', sessionRow, activity: sessionRow.activity });
            }
            const repoGroups = [...groups.values()]
                .map(group => ({
                    repo: group.repo,
                    items: group.items.sort((a, b) => b.activity - a.activity),
                    activity: Math.max(0, ...group.items.map(item => item.activity))
                }))
                .sort((a, b) => b.activity - a.activity);

            let html = '<div class="pr-board-header">⑆ PR board <span class="pr-board-count">'
                + entries.length + ' PRs · ' + prBoardSessions.length + ' live sessions</span></div>';
            const rendered = [];
            for (const group of repoGroups) {
                const sessionIds = new Set();
                let prCount = 0;
                for (const item of group.items) {
                    if (item.kind === 'pr') {
                        prCount += 1;
                        for (const session of item.entry.sessions || []) {
                            sessionIds.add(session.session_id || session.dir_name);
                        }
                    } else {
                        const session = item.sessionRow.session;
                        sessionIds.add(session.session_id || session.dir_name);
                    }
                }
                html += '<div class="pr-board-repo">' + escapeHtml(group.repo)
                    + ' <span class="pr-board-count">' + prCount + ' PR' + (prCount === 1 ? '' : 's')
                    + ' · ' + sessionIds.size + ' session' + (sessionIds.size === 1 ? '' : 's') + '</span></div>';
                for (const item of group.items) {
                    if (item.kind === 'pr') {
                        const index = rendered.length;
                        rendered.push(item.entry);
                        html += prBoardRow(item.entry, index);
                    } else {
                        html += prBoardSessionRow(item.sessionRow);
                    }
                }
            }
            board.innerHTML = html;

            board.querySelectorAll('.pr-board-row[data-pr-index]').forEach(rowEl => {
                const entry = rendered[Number(rowEl.dataset.prIndex)];
                if (!entry) return;
                const star = rowEl.querySelector('.pr-primary-toggle');
                if (star) star.onclick = () => {
                    postPrOverride(entry.pr, prDisposition(entry.pr) === 'primary' ? 'secondary' : 'primary');
                    renderPrBoard();
                    rerenderAllPrLists();
                };
                const dismiss = rowEl.querySelector('.pr-dismiss');
                if (dismiss) dismiss.onclick = () => {
                    postPrOverride(entry.pr, 'dismissed');
                    renderPrBoard();
                    rerenderAllPrLists();
                };
            });
        }

        function prBoardSessionMatchesPr(session, pr) {
            return !!session.repo_name
                && session.repo_owner.toLowerCase() === (pr.owner || '').toLowerCase()
                && session.repo_name.toLowerCase() === (pr.repo || '').toLowerCase();
        }

        function prBoardSessionRow(sessionRow) {
            const session = sessionRow.session;
            const activity = prBoardActivityLabel([session]);
            const title = (session.title || '').replace(/^⟁\s+/, '')
                || session.dir_name || 'Untitled session';
            return '<div class="pr-board-row">'
                + '<div class="pr-row-top"><span class="pr-session-glyph">◇</span>'
                + '<span class="pr-board-identity"><span class="pr-link">' + escapeHtml(title) + '</span></span>'
                + '<span class="pr-diff"></span><span class="pr-files"></span>'
                + '<span class="pr-branch">⎇ ' + escapeHtml(sessionRow.branch) + '</span>'
                + '<span class="pr-activity">' + escapeHtml(activity) + '</span>'
                + '<span class="pr-status"></span></div></div>';
        }

        function prBoardRow(entry, index) {
            const pr = entry.pr;
            const isPrimary = entry.primary;
            const star = '<span class="pr-primary-toggle ' + (isPrimary ? 'primary' : 'secondary')
                + '" title="' + (isPrimary ? 'Primary — click to make secondary' : 'Secondary — click to make primary')
                + '">' + (isPrimary ? '★' : '☆') + '</span>';
            const link = '<a class="pr-link" href="' + escapeHtml(pr.url)
                + '" target="_blank" rel="noopener noreferrer">' + escapeHtml(pr.repo + ' #' + pr.number) + '</a>';
            const title = '<span class="pr-board-title"'
                + (pr.title ? ' title="' + escapeHtml(pr.title) + '"' : '')
                + '>' + escapeHtml(pr.title || '') + '</span>';
            const identity = '<span class="pr-board-identity">' + link + title + '</span>';
            const diffParts = [];
            if (pr.additions) diffParts.push('<span class="rd-add">+' + pr.additions + '</span>');
            if (pr.deletions) diffParts.push('<span class="rd-del">-' + pr.deletions + '</span>');
            const diff = diffParts.length
                ? prExternalLink(pr.url ? pr.url + '/files' : '', 'pr-diff', diffParts.join(' '))
                : '<span class="pr-diff"></span>';
            const files = '<span class="pr-files">'
                + ((pr.changed_files || 0) ? pr.changed_files + ' file' + (pr.changed_files === 1 ? '' : 's') : '')
                + '</span>';
            const branch = '<span class="pr-branch">'
                + (pr.branch ? '⎇ ' + escapeHtml(pr.branch) : '') + '</span>';
            const stateInfo = prStateInfo(pr);
            const badge = stateInfo.label
                ? '<span class="pr-badge" style="color:' + stateInfo.color
                    + ';border-color:' + stateInfo.color + '"'
                    + (stateInfo.title ? ' title="' + escapeHtml(stateInfo.title) + '"' : '')
                    + '>' + stateInfo.label + '</span>'
                : '';
            const dismiss = '<span class="pr-dismiss" title="Dismiss this PR everywhere">✕</span>';
            const status = '<span class="pr-status">' + badge + prCiBadge(pr)
                + prCommentsBadge(pr) + prMergeBadge(pr) + dismiss + '</span>';
            const activity = '<span class="pr-activity">'
                + escapeHtml(prBoardActivityLabel(entry.sessions)) + '</span>';

            const sessions = (entry.sessions || [])
                .map(s => escapeHtml(s.dir_name) + (s.active ? '' : ' <span class="pb-dim">(ended)</span>'))
                .join(', ');
            const mentionAge = pr.last_mentioned_at
                ? ' · spoken ' + prBoardAge(Math.floor(pr.last_mentioned_at / 1000)) + ' ago'
                : '';
            const mentions = (pr.mentions || 0) + ' mentions'
                + ((pr.user_mentions || 0) > 0 ? ' (' + pr.user_mentions + ' yours)' : '');
            const meta = prBoardChecklist(pr)
                + ' <span class="pb-stage ' + entry.stage.cls + '">' + entry.stage.label + '</span>'
                + ' <span class="pb-dim">· ' + sessions + mentionAge + ' · ' + mentions + '</span>';

            let extras = '';
            if (pr.ai_note) {
                const confidence = pr.ai_confidence
                    ? ' <span class="pb-dim">(' + escapeHtml(pr.ai_confidence) + ' confidence it\\'s done)</span>'
                    : '';
                extras += '<span class="pb-ai">✦ ' + escapeHtml(pr.ai_note) + confidence + '</span>';
            }
            if (pr.slack_origin_url) {
                extras += prExternalLink(pr.slack_origin_url, 'pb-slack', '⛓ slack origin');
            }
            for (let i = 0; i < (pr.slack_comment_urls || []).length; i++) {
                const label = pr.slack_comment_urls.length === 1 ? '⛓ slack' : '⛓ slack ' + (i + 1);
                extras += prExternalLink(pr.slack_comment_urls[i], 'pb-slack', label);
            }

            return '<div class="pr-board-row' + (isPrimary ? '' : ' pr-secondary')
                + '" data-pr-index="' + index + '">'
                + '<div class="pr-row-top">' + star + identity + diff + files + branch + activity + status + '</div>'
                + '<div class="pr-board-meta">' + meta + '</div>'
                + (extras ? '<div class="pr-board-extras">' + extras + '</div>' : '')
                + '</div>';
        }
`;
