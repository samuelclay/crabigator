export const modulePanelJs = `
        // The stats strip on an open session's drawer handle. The View
        // panel draws its sample strip from the same markup.
        function statsStripHtml(id) {
            const stat = (stats, tooltip, color, glyph, inner) => '<span class="wh-stat' + (stats === 'tools' ? ' wh-tools' : '')
                + '" data-card-stats="' + stats + '" data-tooltip="' + tooltip + '"><span class="wh-icon" style="color:' + color + '">'
                + glyph + '</span>' + inner + '</span>';
            const value = (key, text, stats) => '<span class="wh-value"' + (stats ? ' data-card-stats="' + stats + '"' : '')
                + ' id="widgets-' + key + '-' + id + '">' + text + '</span>';
            const age = (key, stats) => '<span class="wh-elapsed"' + (stats ? ' data-card-stats="' + stats + '"' : '')
                + ' id="widgets-' + key + '-elapsed-' + id + '"></span>';
            return stat('sessionTime', 'Session', '#58a6ff', '◉', value('session', '--'))
                + stat('thinkingTime', 'Thinking', '#3fb950', '◐', value('thinking', '—'))
                + '<span class="wh-stat" id="widgets-idle-wrapper-' + id + '" style="display:none" data-tooltip="Idle">'
                + '<span class="wh-icon" style="color:#8b949e">◌</span>' + value('idle', '—') + '</span>'
                + stat('prompts promptRecency', 'Prompts', '#8b949e', '⟩', value('prompts', '0', 'prompts') + age('prompts', 'promptRecency'))
                + stat('completions completionRecency', 'Completions', '#8b949e', '⋖', value('completions', '0', 'completions') + age('completions', 'completionRecency'))
                + stat('tools', 'Tool calls', '#f0883e', '⚒', value('tools', '—'))
                + stat('compactions', 'Compactions', '#e879f9', '⊜', value('compactions', '0') + age('compactions'))
                + stat('context', 'Context', '#8b949e', '◔', value('context', '—'));
        }

        function sessionHeadingContentHtml(id) {
            return \`<span class="title-row"><span id="title-mark-\${id}"></span><span class="title" id="title-\${id}"></span></span>
                        <span class="generated-title" id="generated-title-\${id}"></span>
                        <span class="session-context"><span class="row-computer"></span><span class="cwd"></span><span class="branch" id="branch-\${id}"></span></span>
                        <span class="session-row-lines"></span>\`;
        }

        function modulePanelHtml(id) {
            return \`<div class="widgets-panel" id="widgets-\${id}">
                <div class="widgets-header">
                    <div class="widgets-header-row1">
                        <span id="widgets-mark-\${id}"></span>
                        <span class="widgets-title" id="widgets-title-\${id}">Session</span>
                        <span class="widgets-state" id="widgets-state-\${id}">○ Ready</span>
                        <span class="widgets-header-spacer"></span>
                        <button class="collapse-btn" id="widgets-btn-\${id}" title="Toggle Git & Changes widgets">▼</button>
                    </div>
                    <div class="widgets-header-row2">
                        \${statsStripHtml(id)}
                        <span class="wh-git" id="widgets-git-\${id}"></span>
                    </div>
                </div>
                <div class="widgets-content" id="widgets-content-\${id}">
                    <div class="widget title-history-widget" id="titles-\${id}" style="display:none">
                        <div class="widget-title"><span style="color:#58a6ff">—</span></div>
                        <div class="titles-list"></div>
                    </div>
                    <div class="widget" id="git-\${id}">
                        <div class="widget-title"><span style="color:#7ee787" id="git-branch-\${id}">...</span> <span style="float:right;color:#8b949e" id="git-filecount-\${id}">...</span></div>
                        <div class="git-files" style="color:#8b949e">Waiting for data...</div>
                    </div>
                    <div class="widget" id="changes-\${id}">
                        <div class="widget-title"><span style="color:#db6d28">Changes</span> <span style="float:right;color:#8b949e" id="changes-count-\${id}"></span></div>
                        <div class="changes-list" style="color:#8b949e">Waiting for data...</div>
                    </div>
                </div>
            </div>\`;
        }
`;
