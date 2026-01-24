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
                <div class="widget-title"><span style="color:#db6d28">\${firstLang.language}</span> <span style="color:#8b949e">\${totalChanges} \${changeWord}</span></div>
                <div class="changes-list">\${changesHtml}</div>
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
        }

        function updateTitlesWidget(sessionId, titleHistory) {
            const widget = document.getElementById('titles-' + sessionId);
            if (!widget) return;

            // Hide widget if no titles
            if (!titleHistory || titleHistory.length === 0) {
                widget.style.display = 'none';
                return;
            }

            // Show widget
            widget.style.display = '';

            // Latest title is the widget title
            const latestTitle = titleHistory[titleHistory.length - 1];
            const escapedLatest = latestTitle.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

            // Previous titles are the history content (all except the last one)
            const previousTitles = titleHistory.slice(0, -1);

            let newHtml;
            if (previousTitles.length === 0) {
                // Just one title - show it as the module title, no content
                newHtml = \`
                    <div class="widget-title"><span style="color:#58a6ff">\${escapedLatest}</span></div>
                \`;
            } else {
                // Multiple titles - latest as title, previous as history (newest first)
                const historyHtml = previousTitles.slice().reverse().map(title => {
                    const escaped = title.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
                    return \`<div class="title-entry">\${escaped}</div>\`;
                }).join('');

                newHtml = \`
                    <div class="widget-title"><span style="color:#58a6ff">\${escapedLatest}</span></div>
                    <div class="titles-list">\${historyHtml}</div>
                \`;
            }

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }

            // Update widgets summary with latest title
            const titleSummaryEl = document.getElementById('widgets-title-' + sessionId);
            if (titleSummaryEl) {
                titleSummaryEl.textContent = latestTitle;
            }
        }

`;
