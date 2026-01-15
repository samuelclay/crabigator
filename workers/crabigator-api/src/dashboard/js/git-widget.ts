// Dashboard JavaScript - git-widget
export const gitWidgetJs = `
        // Count digits in a number
        function digitCount(n) {
            if (n === 0) return 1;
            return Math.floor(Math.log10(n)) + 1;
        }

        // Create non-breaking spaces for HTML (regular spaces collapse)
        function nbsp(count) {
            return '&nbsp;'.repeat(Math.max(0, count));
        }

        // Compute column widths for git file diffs
        function computeGitColumnWidths(files) {
            let maxDel = 0;
            let maxAdd = 0;
            for (const f of files) {
                maxDel = Math.max(maxDel, f.deletions || 0);
                maxAdd = Math.max(maxAdd, f.additions || 0);
            }
            // Width for number columns: sign + digits
            const delNumWidth = maxDel > 0 ? 1 + digitCount(maxDel) : 0;
            const addNumWidth = maxAdd > 0 ? 1 + digitCount(maxAdd) : 0;
            // Bar width: symmetric based on max of both (log scale)
            const maxBar = Math.max(
                maxDel > 0 ? digitCount(maxDel) : 0,
                maxAdd > 0 ? digitCount(maxAdd) : 0
            );
            return { delNumWidth, addNumWidth, barWidth: maxBar };
        }

        // Compute column widths for semantic changes
        function computeChangesColumnWidths(byLanguage) {
            let maxDel = 0;
            let maxAdd = 0;
            for (const lang of byLanguage) {
                for (const c of (lang.changes || [])) {
                    maxDel = Math.max(maxDel, c.deletions || 0);
                    maxAdd = Math.max(maxAdd, c.additions || 0);
                }
            }
            // Width for number columns: sign + digits
            const delNumWidth = maxDel > 0 ? 1 + digitCount(maxDel) : 0;
            const addNumWidth = maxAdd > 0 ? 1 + digitCount(maxAdd) : 0;
            return { delNumWidth, addNumWidth };
        }

        function getStatusIcon(status) {
            // Map git status to CLI icons and colors
            const s = (status || '').trim();
            if (s === 'M' || status === 'M ' || status === ' M') {
                return { icon: '●', color: '#d29922' };  // yellow for modified
            }
            if (s === 'A') {
                return { icon: '+', color: '#3fb950' };  // green for added
            }
            if (s === 'D') {
                return { icon: '−', color: '#f85149' };  // red for deleted
            }
            if (s === '??' || s === '?') {
                return { icon: '?', color: '#39c5cf' };  // cyan for untracked
            }
            return { icon: '•', color: '#6e7681' };  // gray for other
        }

        function updateGitWidget(sessionId, git) {
            const widget = document.getElementById('git-' + sessionId);
            if (!widget) return;

            const files = git.files || [];
            const totalFiles = files.length;
            const branch = git.branch || 'unknown';

            // Update branch in header
            const branchEl = document.getElementById('branch-' + sessionId);
            if (branchEl) {
                branchEl.textContent = ' ' + branch;
            }

            // Compact display for clean repos - just header, no body
            if (totalFiles === 0) {
                const newHtml = \`<div class="widget-title"><span style="color:#7ee787">\${branch}</span> <span style="color:#3fb950">✓ Clean</span></div>\`;
                if (widget.innerHTML !== newHtml) {
                    widget.innerHTML = newHtml;
                }
                return;
            }

            // Header: branch on left, "N files" on right (like CLI)
            const filesLabel = totalFiles === 1 ? 'file' : 'files';
            const headerRight = '<span style="color:#d29922">' + totalFiles + ' ' + filesLabel + '</span>';

            // Compute column widths for alignment
            const { delNumWidth, addNumWidth, barWidth } = computeGitColumnWidths(files);

            let filesHtml = files.map(f => {
                const { icon, color } = getStatusIcon(f.status);
                const del = f.deletions || 0;
                const add = f.additions || 0;

                // 4-column layout: [del num] [bars] [add num]
                // Build deletion number (right-aligned)
                const delNumStr = del > 0 ? '−' + del : '';
                const delNumPad = delNumWidth - delNumStr.length;
                const delNumHtml = delNumWidth > 0
                    ? \`<span style="color:#f85149">\${nbsp(delNumPad)}\${delNumStr}</span>\`
                    : '';

                // Build combined bar (red left-padded, green right-padded, touching in middle)
                const delBarLen = del > 0 ? digitCount(del) : 0;
                const addBarLen = add > 0 ? digitCount(add) : 0;
                const delBarPad = barWidth - delBarLen;
                const addBarPad = barWidth - addBarLen;
                const redBars = '▓'.repeat(delBarLen);
                const greenBars = '█'.repeat(addBarLen);
                const barsHtml = barWidth > 0
                    ? \`\${nbsp(delBarPad)}<span style="display:inline-flex;gap:0"><span style="color:#f85149">\${redBars}</span><span style="color:#3fb950">\${greenBars}</span></span>\${nbsp(addBarPad)}\`
                    : '';

                // Build addition number (left-aligned)
                const addNumStr = add > 0 ? '+' + add : '';
                const addNumPad = addNumWidth - addNumStr.length;
                const addNumHtml = addNumWidth > 0
                    ? \`<span style="color:#3fb950">\${addNumStr}\${nbsp(addNumPad)}</span>\`
                    : '';

                return \`<div class="git-file">
                    <span style="color:\${color}">\${icon}</span>
                    <span class="path">\${f.path}</span>
                    <span class="diff">\${delNumHtml}\${barsHtml}\${addNumHtml}</span>
                </div>\`;
            }).join('');

            const newHtml = \`
                <div class="widget-title"><span style="color:#7ee787">\${branch}</span> <span style="float:right">\${headerRight}</span></div>
                <div class="git-files">\${filesHtml}</div>
            \`;

            // Only update if content changed (prevents flicker)
            if (widget.innerHTML !== newHtml) {
                widget.innerHTML = newHtml;
            }
        }

`;
