// Dashboard JavaScript - stats-widget
export const statsWidgetJs = `
        function updateStatsWidget(sessionId, stats) {
            const widget = document.getElementById('stats-' + sessionId);
            if (!widget) return;

            const session = sessions.get(sessionId);
            const state = session?.state || 'ready';
            const stateIndicator = formatStateIndicator(state);

            // Store mode and permission in session data for tracking
            if (session && stats.mode) {
                session.mode = stats.mode;
            }
            if (session) {
                session.permission = stats.permission || null;
            }

            // Update info popover with model/platform if available
            if (stats.model) {
                const modelRow = document.getElementById('info-model-' + sessionId);
                const modelValue = document.getElementById('info-model-value-' + sessionId);
                if (modelRow && modelValue) {
                    modelRow.style.display = '';
                    modelValue.textContent = stats.model;
                }
            }
            if (stats.platform) {
                const platformRow = document.getElementById('info-platform-' + sessionId);
                const platformValue = document.getElementById('info-platform-value-' + sessionId);
                if (platformRow && platformValue) {
                    platformRow.style.display = '';
                    platformValue.textContent = stats.platform;
                }
            }

            const promptsElapsed = stats.prompts_changed_at ? formatElapsed(stats.prompts_changed_at) : '';
            const completionsElapsed = stats.completions_changed_at ? formatElapsed(stats.completions_changed_at) : '';
            const compressionsElapsed = stats.compressions_changed_at ? formatElapsed(stats.compressions_changed_at) : '';

            const compressionsRow = stats.compressions > 0
                ? \`<div class="widget-row"><span class="widget-label">⊜ Compactions \${stats.compressions}</span><span class="widget-value" style="color:#8b949e">\${compressionsElapsed}</span></div>\`
                : '';

            // Show idle time when in complete/question state and idle > 60s
            let idleRow = '';
            const isIdleState = ['complete', 'question', 'interrupted'].includes(session?.state);
            if (isIdleState && stats.idle_since) {
                const idleSecs = Math.floor(Date.now() / 1000 - stats.idle_since);
                if (idleSecs >= 60) {
                    idleRow = \`<div class="widget-row"><span class="widget-label">◇ Idle</span><span class="widget-value" style="color:#8b949e">\${formatDuration(idleSecs)}</span></div>\`;
                }
            }

            const newHtml = \`
                <div class="widget-title"><span style="color:#bc8cff">Stats</span> <span style="float:right">\${stateIndicator}</span></div>
                <div class="widget-row"><span class="widget-label">◆ Session</span><span class="widget-value" style="color:#58a6ff">\${formatDuration(stats.work_seconds || 0)}</span></div>
                <div class="widget-row"><span class="widget-label">◇ Thinking</span><span class="widget-value" style="color:#3fb950">\${stats.thinking_seconds ? formatDuration(stats.thinking_seconds) : '—'}</span></div>
                <div class="widget-row"><span class="widget-label">▸ Prompts \${stats.prompts || 0}</span><span class="widget-value" style="color:#8b949e">\${promptsElapsed}</span></div>
                <div class="widget-row"><span class="widget-label">◂ Completions \${stats.completions || 0}</span><span class="widget-value" style="color:#8b949e">\${completionsElapsed}</span></div>
                <div class="widget-row"><span class="widget-label">⚙ Tools</span><span class="widget-value">\${renderToolsSparkline(stats.tool_timestamps, stats.session_start, Date.now() / 1000)}</span></div>
                \${compressionsRow}
                \${idleRow}
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }

            // Update widgets summary
            const sessionEl = document.getElementById('widgets-session-' + sessionId);
            const thinkingEl = document.getElementById('widgets-thinking-' + sessionId);
            const idleEl = document.getElementById('widgets-idle-' + sessionId);
            if (sessionEl) sessionEl.textContent = formatDuration(stats.work_seconds || 0);
            if (thinkingEl) thinkingEl.textContent = stats.thinking_seconds ? formatDuration(stats.thinking_seconds) : '—';
            if (idleEl) {
                const isIdleState = ['complete', 'question', 'interrupted'].includes(session?.state);
                if (isIdleState && stats.idle_since) {
                    const idleSecs = Math.floor(Date.now() / 1000 - stats.idle_since);
                    idleEl.textContent = idleSecs >= 60 ? formatDuration(idleSecs) : '—';
                } else {
                    idleEl.textContent = '—';
                }
            }
        }

        // Create two-sided progress bar like CLI: ▓▓ (red/deletions) █████ (green/additions)
        function createProgressBar(additions, deletions) {
            if (additions === 0 && deletions === 0) {
                return '<span style="color:#6e7681">·</span>';
            }
            // Log-scale bar widths (like CLI)
            const delBar = deletions > 0 ? Math.floor(Math.log10(deletions)) + 1 : 0;
            const addBar = additions > 0 ? Math.floor(Math.log10(additions)) + 1 : 0;

            let result = '';
            if (delBar > 0) {
                result += '<span style="color:#f85149">' + '▓'.repeat(delBar) + '</span>';
            }
            if (addBar > 0) {
                result += '<span style="color:#3fb950">' + '█'.repeat(addBar) + '</span>';
            }
            return result;
        }

        // Unicode block characters for sparkline (8 levels + empty)
        const SPARKLINE_BLOCKS = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        const SPARKLINE_MAX = 10; // Fixed max: 10 tools = full height

        // Render tools sparkline histogram like CLI
        function renderToolsSparkline(timestamps, sessionStart, now) {
            if (!timestamps || !sessionStart || timestamps.length === 0) {
                return '<span style="color:#8b949e">—</span>';
            }

            const numBins = 20; // Number of histogram buckets
            const duration = now - sessionStart;
            if (duration <= 0) {
                return '<span style="color:#8b949e">—</span>';
            }

            const binSize = duration / numBins;
            const bins = new Array(numBins).fill(0);

            // Bin the timestamps
            for (const ts of timestamps) {
                if (ts >= sessionStart && ts <= now) {
                    const binIdx = Math.min(Math.floor((ts - sessionStart) / binSize), numBins - 1);
                    bins[binIdx]++;
                }
            }

            // Check if there's any activity
            const hasActivity = bins.some(c => c > 0);
            if (!hasActivity) {
                return '<span style="color:#8b949e">' + ' '.repeat(numBins) + '</span>';
            }

            // Build sparkline
            let sparkline = '';
            for (const count of bins) {
                if (count === 0) {
                    sparkline += SPARKLINE_BLOCKS[0];
                } else {
                    // Scale to 1-8 range using fixed max (absolute scale)
                    const scaled = Math.ceil((count / SPARKLINE_MAX) * 8);
                    const level = Math.max(1, Math.min(8, scaled));
                    sparkline += SPARKLINE_BLOCKS[level];
                }
            }

            return '<span style="color:#f0883e">' + sparkline + '</span>';
        }

`;
