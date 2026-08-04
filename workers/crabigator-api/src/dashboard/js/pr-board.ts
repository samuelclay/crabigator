// Dashboard JavaScript - cross-session PR board
// Swaps the session grid for one deduped view of every PR the group's
// sessions have tracked (D1-backed, so it survives session end). Reuses the
// PR badge helpers and override plumbing from changes-widget.ts.
export const prBoardJs = `
        let prBoardVisible = false;
        let prBoardTimer = null;
        let prBoardEntries = [];

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
                renderPrBoard();
            } catch (e) {
                board.innerHTML = '<div class="pr-board-empty">Could not load the PR board.</div>';
            }
        }

        // Pipeline position, most-attention-needed first (mirrors the TUI).
        function prBoardStage(pr) {
            if (pr.state === 'OPEN' && ((pr.checks_failed || 0) > 0 || pr.mergeable === 'CONFLICTING'))
                return { rank: 0, label: pr.mergeable === 'CONFLICTING' ? 'conflicts' : 'CI failing', cls: 'bad' };
            if (pr.state === 'OPEN' && pr.review_decision === 'CHANGES_REQUESTED')
                return { rank: 1, label: 'changes requested', cls: 'bad' };
            if (pr.state === 'OPEN' && pr.is_draft) return { rank: 2, label: 'draft', cls: 'dim' };
            if (pr.state === 'OPEN' && (pr.checks_pending || 0) > 0) return { rank: 3, label: 'CI running', cls: 'warn' };
            if (pr.state === 'OPEN' && pr.review_decision !== 'APPROVED')
                return { rank: 4, label: 'awaiting review', cls: 'warn' };
            if (pr.state === 'OPEN') return { rank: 5, label: 'ready to merge', cls: 'good' };
            if (pr.state === 'MERGED') return { rank: 6, label: 'merged', cls: 'merged' };
            return { rank: 7, label: 'closed', cls: 'dim' };
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

        function renderPrBoard() {
            const board = document.getElementById('pr-board-view');
            if (!board) return;

            // Dispositions already applied server-side, but a toggle made just
            // now should reshape the board before the next fetch.
            const entries = prBoardEntries
                .filter(e => prDisposition(e.pr) !== 'dismissed')
                .map(e => ({ ...e, primary: prDisposition(e.pr) === 'primary', stage: prBoardStage(e.pr) }));
            entries.sort((a, b) =>
                a.stage.rank - b.stage.rank
                || (b.primary ? 1 : 0) - (a.primary ? 1 : 0)
                || (b.pr.last_mentioned_at || 0) - (a.pr.last_mentioned_at || 0));

            if (entries.length === 0) {
                board.innerHTML = '<div class="pr-board-empty">No PRs tracked yet. They appear as sessions mention them.</div>';
                return;
            }

            const repoOrder = [];
            const groups = new Map();
            for (const entry of entries) {
                const repo = entry.owner + '/' + entry.repo;
                if (!groups.has(repo)) { groups.set(repo, []); repoOrder.push(repo); }
                groups.get(repo).push(entry);
            }

            let html = '<div class="pr-board-header">⑆ PR board <span class="pr-board-count">'
                + entries.length + ' PRs</span></div>';
            const rendered = [];
            for (const repo of repoOrder) {
                const group = groups.get(repo);
                html += '<div class="pr-board-repo">' + escapeHtml(repo)
                    + ' <span class="pr-board-count">' + group.length + '</span></div>';
                for (const entry of group) {
                    html += prBoardRow(entry);
                    rendered.push(entry);
                }
            }
            board.innerHTML = html;

            board.querySelectorAll('.pr-board-row').forEach((rowEl, i) => {
                const entry = rendered[i];
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

        function prBoardRow(entry) {
            const pr = entry.pr;
            const isPrimary = entry.primary;
            const star = '<span class="pr-primary-toggle ' + (isPrimary ? 'primary' : 'secondary')
                + '" title="' + (isPrimary ? 'Primary — click to make secondary' : 'Secondary — click to make primary')
                + '">' + (isPrimary ? '★' : '⑂') + '</span>';
            const link = '<a class="pr-link" href="' + escapeHtml(pr.url)
                + '" target="_blank" rel="noopener noreferrer">' + escapeHtml(pr.repo + ' #' + pr.number) + '</a>';
            const title = pr.title
                ? '<span class="pr-board-title" title="' + escapeHtml(pr.title) + '">' + escapeHtml(pr.title) + '</span>'
                : '';
            const diffParts = [];
            if (pr.additions) diffParts.push('<span class="rd-add">+' + pr.additions + '</span>');
            if (pr.deletions) diffParts.push('<span class="rd-del">-' + pr.deletions + '</span>');
            const diff = diffParts.length
                ? prExternalLink(pr.url ? pr.url + '/files' : '', 'pr-diff', diffParts.join(' '))
                : '';
            const stateInfo = prStateInfo(pr);
            const badge = stateInfo.label
                ? '<span class="pr-badge" style="color:' + stateInfo.color
                    + ';border-color:' + stateInfo.color + '">' + stateInfo.label + '</span>'
                : '';
            const dismiss = '<span class="pr-dismiss" title="Dismiss this PR everywhere">✕</span>';
            const status = '<span class="pr-status">' + badge + prCiBadge(pr)
                + prCommentsBadge(pr) + prMergeBadge(pr) + dismiss + '</span>';

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

            return '<div class="pr-board-row">'
                + '<div class="pr-row-top">' + star + link + title + diff + status + '</div>'
                + '<div class="pr-board-meta">' + meta + '</div>'
                + (extras ? '<div class="pr-board-extras">' + extras + '</div>' : '')
                + '</div>';
        }
`;
