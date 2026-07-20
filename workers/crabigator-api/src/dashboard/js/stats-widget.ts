// Dashboard JavaScript - stats-widget
export const statsWidgetJs = `
        function updateStatsWidget(sessionId, stats) {
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

            // Update header stats (always visible)
            const stateEl = document.getElementById('widgets-state-' + sessionId);
            const sessionEl = document.getElementById('widgets-session-' + sessionId);
            const thinkingEl = document.getElementById('widgets-thinking-' + sessionId);
            const idleEl = document.getElementById('widgets-idle-' + sessionId);
            const idleWrapper = document.getElementById('widgets-idle-wrapper-' + sessionId);
            const promptsEl = document.getElementById('widgets-prompts-' + sessionId);
            const completionsEl = document.getElementById('widgets-completions-' + sessionId);
            const toolsEl = document.getElementById('widgets-tools-' + sessionId);
            const compactionsEl = document.getElementById('widgets-compactions-' + sessionId);

            if (stateEl) stateEl.innerHTML = stateIndicator;
            if (sessionEl) sessionEl.textContent = formatDuration(stats.work_seconds || 0);
            if (thinkingEl) thinkingEl.textContent = stats.thinking_seconds ? formatDuration(stats.thinking_seconds) : '—';

            // Show/hide idle based on state
            const isIdleState = ['complete', 'question', 'interrupted'].includes(session?.state);
            if (idleWrapper && idleEl) {
                if (isIdleState && stats.idle_since) {
                    const idleSecs = Math.floor(Date.now() / 1000 - stats.idle_since);
                    if (idleSecs >= 60) {
                        idleWrapper.style.display = '';
                        idleEl.textContent = formatDuration(idleSecs);
                    } else {
                        idleWrapper.style.display = 'none';
                    }
                } else {
                    idleWrapper.style.display = 'none';
                }
            }

            if (promptsEl) promptsEl.textContent = stats.prompts || 0;
            if (completionsEl) completionsEl.textContent = stats.completions || 0;
            if (toolsEl) {
                const compactTools = typeof window !== 'undefined'
                    && window.matchMedia
                    && window.matchMedia('(max-width: 768px)').matches;
                toolsEl.innerHTML = renderToolsSparkline(
                    stats.tool_timestamps,
                    stats.session_start,
                    Date.now() / 1000,
                    compactTools ? 8 : 20
                );
            }
            if (compactionsEl) compactionsEl.textContent = stats.compressions || 0;

            // Update elapsed times
            const promptsElapsedEl = document.getElementById('widgets-prompts-elapsed-' + sessionId);
            const completionsElapsedEl = document.getElementById('widgets-completions-elapsed-' + sessionId);
            const compactionsElapsedEl = document.getElementById('widgets-compactions-elapsed-' + sessionId);

            updateRecencyElement(promptsElapsedEl, stats.prompts_changed_at);
            updateRecencyElement(completionsElapsedEl, stats.completions_changed_at);
            updateRecencyElement(compactionsElapsedEl, stats.compressions_changed_at);
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
        function renderToolsSparkline(timestamps, sessionStart, now, numBins = 20) {
            if (!timestamps || !sessionStart || timestamps.length === 0) {
                return '<span style="color:#8b949e">—</span>';
            }

            numBins = Math.max(4, Math.min(20, numBins));
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
