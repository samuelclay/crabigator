// Dashboard JavaScript - the optional lines under a session's title.
// The List, the session list beside it, the PR board and the View panel's
// sample row all draw them with sessionRowLinesHtml, from one "facts" shape.
// No imports here: scripts/test-view-prefs.mjs runs these functions directly.
export const sessionRowJs = `
        // Glyph colours shared with the apps (xterm 39, 83, 179, 208).
        const ROW_TONES = {
            session: '#00afff', thinking: '#5fff5f', prompt: '#d7af5f', completion: '#ff8700',
            count: '#d7af5f', branch: '#7ee787', delta: '#ff87d7', added: '#5fff5f', removed: '#ff5f5f',
        };

        function rowStat(glyph, tone, name, valueHtml) {
            return '<span class="row-stat" style="color:' + tone + '" title="' + name + '">'
                + '<span class="row-glyph" aria-hidden="true">' + glyph + '</span>'
                + '<span class="row-sr">' + name + ' </span>' + valueHtml + '</span>';
        }
        // An age that keeps itself current (see refreshRecencyLabels).
        function rowAge(timestamp) {
            return '<span data-recency-timestamp="' + timestamp + '">' + formatElapsed(timestamp) + '</span>';
        }
        function recapHeadlineText(recap) {
            const headline = (recap && recap.headline) || '';
            return /completed the turn without captured transcript output/i.test(headline) ? '' : headline;
        }

        // Recap, times, counts, git, changes: the order every client uses.
        // A stat with no data is left out, and an empty line is not drawn.
        function sessionRowLinesHtml(facts, show) {
            const lines = [];
            if (show.recap && facts.recap) {
                lines.push('<span class="row-line row-recap">' + escapeHtml(facts.recap) + '</span>');
            }
            const times = [];
            if (show.sessionTime && facts.sessionSeconds != null) {
                times.push(rowStat('◉', ROW_TONES.session, 'Session time', formatDuration(facts.sessionSeconds)));
            }
            if (show.thinkingTime && facts.thinkingSeconds) {
                times.push(rowStat('◐', ROW_TONES.thinking, 'Thinking time', formatDuration(facts.thinkingSeconds)));
            }
            const prompt = [
                show.prompts && facts.prompts != null ? String(facts.prompts) : '',
                show.promptRecency && facts.promptAt ? rowAge(facts.promptAt) : '',
            ].filter(Boolean).join(' ');
            if (prompt) times.push(rowStat('⟩', ROW_TONES.prompt, 'Last prompt', prompt));
            const completion = [
                show.completions && facts.completions != null ? String(facts.completions) : '',
                show.completionRecency && facts.completionAt ? rowAge(facts.completionAt) : '',
            ].filter(Boolean).join(' ');
            if (completion) times.push(rowStat('⋖', ROW_TONES.completion, 'Last completion', completion));
            if (times.length) lines.push('<span class="row-line row-times">' + times.join('') + '</span>');

            const counts = [];
            // U+FE0E keeps the hammers a text glyph, not an emoji.
            if (show.tools && facts.tools) {
                counts.push(rowStat('⚒' + String.fromCharCode(0xFE0E), ROW_TONES.count, 'Tools', String(facts.tools)));
            }
            if (show.compactions && facts.compactions) {
                counts.push(rowStat('⊜', ROW_TONES.count, 'Compactions', String(facts.compactions)));
            }
            if (show.context && facts.context) {
                counts.push(rowStat('◔', facts.context.color, 'Context', facts.context.label));
            }
            if (counts.length) lines.push('<span class="row-line row-counts">' + counts.join('') + '</span>');

            if (show.git && (facts.branch || facts.uncommitted > 0)) {
                lines.push('<span class="row-line row-git">'
                    + (facts.branch ? '<span style="color:' + ROW_TONES.branch + '">⎇ ' + escapeHtml(facts.branch) + '</span>' : '')
                    + (facts.uncommitted > 0
                        ? '<span class="row-muted">' + facts.uncommitted + (facts.uncommitted === 1 ? ' file' : ' files') + ' not committed</span>'
                        : '')
                    + '</span>');
            }
            if (show.changes && (facts.additions || facts.deletions)) {
                lines.push('<span class="row-line row-changes" title="Lines not committed">'
                    + '<span style="color:' + ROW_TONES.delta + '">Δ</span>'
                    + '<span style="color:' + ROW_TONES.added + '">+' + (facts.additions || 0) + '</span>'
                    + '<span style="color:' + ROW_TONES.removed + '">−' + (facts.deletions || 0) + '</span>'
                    + '</span>');
            }
            return lines.join('');
        }

        // A live session's time runs from its start, as in the apps. An ended
        // one keeps the total its desktop reported.
        function rowSessionSeconds(active, startedAt, endedAt, workSeconds, now) {
            if (!active && typeof workSeconds === 'number') return workSeconds;
            const end = active ? now : endedAt;
            if (startedAt > 0 && end > 0) return Math.max(0, Math.floor(end - startedAt));
            return typeof workSeconds === 'number' ? workSeconds : null;
        }

        // Facts for a List row: what the open terminal last streamed, over
        // what the roster and the PR board report for a row with no stream.
        function listRowFacts(sessionId, now) {
            const live = sessions.get(sessionId) || {};
            const listed = allSessions.find(session => session.id === sessionId) || {};
            const stats = Object.assign({}, listed.stats, live.stats);
            const files = live.git && live.git.files;
            const sum = field => files.reduce((total, file) => total + (file[field] || 0), 0);
            const history = live.recapHistory || [];
            // An open terminal streams its own clock, which the stats strip
            // under it shows too. A closed row counts from the session's start.
            const streamed = expandedSessions.has(sessionId) && live.stats && typeof live.stats.work_seconds === 'number';
            return {
                recap: recapHeadlineText((live.recap && live.recap.latest) || history[history.length - 1] || listed.recap),
                sessionSeconds: streamed ? live.stats.work_seconds
                    : rowSessionSeconds(true, getSessionStartedTime({ started_at: live.startedAt || listed.started_at }), 0, stats.work_seconds, now),
                thinkingSeconds: stats.thinking_seconds || 0,
                prompts: stats.prompts != null ? stats.prompts : null,
                completions: stats.completions != null ? stats.completions : null,
                promptAt: stats.prompts_changed_at || listed.prompts_changed_at || 0,
                completionAt: stats.completions_changed_at || listed.completions_changed_at || 0,
                tools: stats.tools != null ? stats.tools : (stats.tool_calls || 0),
                compactions: stats.compressions || 0,
                context: contextUsage(stats),
                branch: (live.git && live.git.branch != null ? live.git.branch : listed.branch) || '',
                uncommitted: files ? files.length : (listed.uncommitted || 0),
                additions: files ? sum('additions') : (listed.additions || 0),
                deletions: files ? sum('deletions') : (listed.deletions || 0),
            };
        }

        // Facts for a session on the PR board, from GET /api/prs/board.
        function boardRowFacts(session, now) {
            const stats = session.stats || {};
            return {
                recap: recapHeadlineText(session.recap),
                sessionSeconds: rowSessionSeconds(!!session.active, session.started_at || 0,
                    session.ended_at || session.last_seen_at || 0, stats.work_seconds, now),
                thinkingSeconds: stats.thinking_seconds || 0,
                prompts: stats.prompts != null ? stats.prompts : null,
                completions: stats.completions != null ? stats.completions : null,
                promptAt: session.prompts_changed_at || 0,
                completionAt: session.completions_changed_at || 0,
                tools: stats.tools || 0,
                compactions: stats.compressions || 0,
                context: contextUsage(stats),
                branch: session.branch || '',
                uncommitted: session.uncommitted || 0,
                additions: session.additions || 0,
                deletions: session.deletions || 0,
            };
        }
`;
