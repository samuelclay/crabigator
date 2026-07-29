// Dashboard JavaScript - ansi
export const ansiJs = `
        // ANSI to HTML converter - processes escape sequences including cursor positioning
        function ansiToHtml(text) {
            if (!text) return '';

            // Trim trailing whitespace from each line to prevent background color bleeding
            // Terminal buffers often pad lines with spaces. Preserve trailing ANSI resets.
            function trimLineEndPreserveAnsi(line) {
                // Peel off any ANSI sequences at the end of the line.
                let end = line.length;
                let ansiSuffix = '';
                while (end > 0) {
                    const match = line.slice(0, end).match(/\\x1b\\[[0-9;?]*[A-Za-z]$/);
                    if (!match) break;
                    ansiSuffix = match[0] + ansiSuffix;
                    end -= match[0].length;
                }
                const trimmed = line.slice(0, end).trimEnd();
                return trimmed + ansiSuffix;
            }

            text = text.split('\\n').map(trimLineEndPreserveAnsi).join('\\n');

            const colors = {
                30: '#0d1117', 31: '#f85149', 32: '#3fb950', 33: '#d29922',
                34: '#58a6ff', 35: '#bc8cff', 36: '#39c5cf', 37: '#c9d1d9',
                90: '#6e7681', 91: '#ff7b72', 92: '#7ee787', 93: '#e3b341',
                94: '#79c0ff', 95: '#d2a8ff', 96: '#56d4dd', 97: '#ffffff'
            };
            const bgColors = {
                40: '#0d1117', 41: '#f85149', 42: '#3fb950', 43: '#d29922',
                44: '#58a6ff', 45: '#bc8cff', 46: '#39c5cf', 47: '#c9d1d9',
                100: '#6e7681', 101: '#ff7b72', 102: '#7ee787', 103: '#e3b341',
                104: '#79c0ff', 105: '#d2a8ff', 106: '#56d4dd', 107: '#ffffff'
            };

            // Parse extended color: 38;2;R;G;B (24-bit) or 38;5;N (256-color)
            // Returns { color, skip } where skip is additional codes to skip
            function parseExtendedColor(codes, idx) {
                const mode = codes[idx + 1];

                // 24-bit RGB: 38;2;R;G;B
                if (mode === 2 && codes[idx + 4] !== undefined) {
                    const r = codes[idx + 2];
                    const g = codes[idx + 3];
                    const b = codes[idx + 4];
                    return { color: 'rgb(' + r + ',' + g + ',' + b + ')', skip: 4 };
                }

                // 256-color palette: 38;5;N
                if (mode === 5 && codes[idx + 2] !== undefined) {
                    const colorNum = codes[idx + 2];
                    let color;
                    if (colorNum < 16) {
                        const basic = ['#0d1117','#cd3131','#0dbc79','#e5e510','#2472c8','#bc3fbc','#11a8cd','#e5e5e5',
                                      '#666666','#f14c4c','#23d18b','#f5f543','#3b8eea','#d670d6','#29b8db','#ffffff'];
                        color = basic[colorNum];
                    } else if (colorNum < 232) {
                        const n = colorNum - 16;
                        const ri = Math.floor(n/36);
                        const gi = Math.floor((n%36)/6);
                        const bi = n%6;
                        const r = ri === 0 ? 0 : ri * 40 + 55;
                        const g = gi === 0 ? 0 : gi * 40 + 55;
                        const b = bi === 0 ? 0 : bi * 40 + 55;
                        color = 'rgb(' + r + ',' + g + ',' + b + ')';
                    } else {
                        const gray = (colorNum - 232) * 10 + 8;
                        color = 'rgb(' + gray + ',' + gray + ',' + gray + ')';
                    }
                    return { color, skip: 2 };
                }
                return null;
            }

            let result = '';
            let inSpan = false;
            let i = 0;
            let currentStyle = '';
            let state = {
                fg: null,
                bg: null,
                bold: false,
                dim: false,
                italic: false,
                underline: false,
                inverse: false
            };

            function resetState() {
                state = {
                    fg: null,
                    bg: null,
                    bold: false,
                    dim: false,
                    italic: false,
                    underline: false,
                    inverse: false
                };
            }

            function buildStyle() {
                let fg = state.fg;
                let bg = state.bg;
                // For inverse, swap fg and bg
                if (state.inverse) {
                    const tmp = fg;
                    fg = bg || '#ffffff';
                    bg = tmp || '#0d1117';
                }

                const styles = [];
                if (fg) styles.push('color:' + fg);
                if (bg) styles.push('background-color:' + bg);
                if (state.bold) styles.push('font-weight:bold');
                if (state.dim) styles.push('opacity:0.5');
                if (state.italic) styles.push('font-style:italic');
                if (state.underline) styles.push('text-decoration:underline');
                return styles.join(';');
            }

            function applyStyle() {
                const nextStyle = buildStyle();
                if (nextStyle === currentStyle) return;
                if (inSpan) {
                    result += '</span>';
                    inSpan = false;
                }
                if (nextStyle) {
                    result += '<span style="' + nextStyle + '">';
                    inSpan = true;
                }
                currentStyle = nextStyle;
            }

            function escapeHtml(value) {
                return value
                    .replace(/&/g, '&amp;')
                    .replace(/</g, '&lt;')
                    .replace(/>/g, '&gt;')
                    .replace(/"/g, '&quot;');
            }

            function urlAt(index) {
                const match = text.slice(index).match(/^https?:\\/\\/[^\\s<>"'\\x1b]+/i);
                if (!match) return null;

                let url = match[0].replace(/[.,;:!?]+$/, '');
                const bracketPairs = [['(', ')'], ['[', ']'], ['{', '}']];
                for (const [open, close] of bracketPairs) {
                    while (url.endsWith(close)) {
                        const openCount = url.split(open).length - 1;
                        const closeCount = url.split(close).length - 1;
                        if (closeCount <= openCount) break;
                        url = url.slice(0, -1);
                    }
                }
                return url || null;
            }

            while (i < text.length) {
                // Check for ESC character (char code 27)
                if (text.charCodeAt(i) === 27 && text[i + 1] === '[') {
                    // Parse CSI sequence: ESC [ params command
                    let j = i + 2;
                    let params = '';
                    while (j < text.length && /[0-9;]/.test(text[j])) {
                        params += text[j];
                        j++;
                    }
                    const command = text[j];
                    j++;

                    if (command === 'm') {
                        // SGR - Select Graphic Rendition
                        const codes = params ? params.split(';').map(Number) : [0];
                        for (let k = 0; k < codes.length; k++) {
                            const code = codes[k];
                            if (code === 0) { resetState(); }
                            else if (code === 1) state.bold = true;
                            else if (code === 2) state.dim = true;
                            else if (code === 3) state.italic = true;
                            else if (code === 4) state.underline = true;
                            else if (code === 7) state.inverse = true;
                            else if (code === 22) { state.bold = false; state.dim = false; }
                            else if (code === 23) state.italic = false;
                            else if (code === 24) state.underline = false;
                            else if (code === 27) state.inverse = false;
                            else if (code === 39) state.fg = null;
                            else if (code === 49) state.bg = null;
                            else if (code === 38) {
                                const result = parseExtendedColor(codes, k);
                                if (result) { state.fg = result.color; k += result.skip; }
                            }
                            else if (code === 48) {
                                const result = parseExtendedColor(codes, k);
                                if (result) { state.bg = result.color; k += result.skip; }
                            }
                            else if (colors[code]) state.fg = colors[code];
                            else if (bgColors[code]) state.bg = bgColors[code];
                        }

                        applyStyle();
                    } else if (command === 'C') {
                        // CUF - Cursor Forward: ESC[C (1) or ESC[nC (n spaces)
                        // vt100 rows_formatted() emits these for empty cells instead of spaces
                        const count = params ? parseInt(params, 10) || 1 : 1;
                        for (let k = 0; k < count; k++) {
                            result += ' ';
                        }
                    }
                    // Skip other escape sequences (H, f, J, K, etc.) - they don't affect our line-based output
                    i = j;
                    continue;
                }

                // Track newlines in the content
                if (text[i] === '\\n' || text[i] === '\\r') {
                    if (text[i] === '\\n') {
                        // Close span before newline to prevent background color bleeding
                        if (inSpan) {
                            result += '</span>';
                        }
                        result += '\\n';
                        // Reopen span after newline if we had styling
                        if (inSpan) {
                            result += '<span style="' + currentStyle + '">';
                        }
                    }
                    // Skip carriage return - we only care about line feeds
                    i++;
                    continue;
                }

                const url = urlAt(i);
                if (url) {
                    const escapedUrl = escapeHtml(url);
                    result += '<a class="terminal-link" href="' + escapedUrl
                        + '" target="_blank" rel="noopener noreferrer">' + escapedUrl + '</a>';
                    i += url.length;
                    continue;
                }

                // Regular character - escape HTML
                const ch = text[i];
                if (ch === '<') result += '&lt;';
                else if (ch === '>') result += '&gt;';
                else if (ch === '&') result += '&amp;';
                else result += ch;
                i++;
            }

            if (inSpan) result += '</span>';

            // Wrap each line in a div, detecting special line types
            const boxDrawingChars = '─━═╌╍┄┅┈┉';

            // Helper: find HTML index corresponding to visible character position
            function findHtmlIndex(html, visiblePos) {
                let visible = 0;
                let inTag = false;
                for (let i = 0; i < html.length; i++) {
                    if (html[i] === '<') inTag = true;
                    else if (html[i] === '>') inTag = false;
                    else if (!inTag) {
                        if (visible === visiblePos) return i;
                        visible++;
                    }
                }
                return html.length;
            }

            // Helper: split HTML at visible character boundaries, handling open spans
            function splitHtmlForFlexbox(html, leftEnd, rightStart) {
                const leftIdx = findHtmlIndex(html, leftEnd);
                const rightIdx = findHtmlIndex(html, rightStart);

                let leftHtml = html.slice(0, leftIdx);
                let rightHtml = html.slice(rightIdx);

                // Check if we need to close/reopen a span at the split
                // Count open spans in left part
                const leftSpanOpens = (leftHtml.match(/<span[^>]*>/g) || []).length;
                const leftSpanCloses = (leftHtml.match(/<\\/span>/g) || []).length;
                const unclosedSpans = leftSpanOpens - leftSpanCloses;

                if (unclosedSpans > 0) {
                    // Find the last unclosed span's style
                    const spanMatches = leftHtml.match(/<span[^>]*>/g) || [];
                    const lastSpan = spanMatches[spanMatches.length - 1];
                    // Close it in left, reopen in right
                    leftHtml += '</span>';
                    rightHtml = lastSpan + rightHtml;
                }

                return [leftHtml, rightHtml];
            }

            const lines = result.split('\\n');
            result = lines.map(lineHtml => {
                // Strip HTML tags to analyze plain text content
                const plain = lineHtml.replace(/<[^>]+>/g, '').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');

                // Check for decorative rule line (>60% box chars)
                let boxCount = 0;
                for (const c of plain) {
                    if (boxDrawingChars.includes(c)) boxCount++;
                }
                if (plain.length > 10 && boxCount / plain.length > 0.6) {
                    return '<div class="rule-line">' + lineHtml + '</div>';
                }

                // Check for split line (text + 10+ spaces + text) that looks like two columns
                // Avoid matching indented code blocks or diff output with long right-hand text.
                const splitMatch = plain.match(/^(.+?)( {10,})(.+)$/);
                if (splitMatch) {
                    const leftLen = splitMatch[1].length;
                    const gapLen = splitMatch[2].length;
                    const rightLen = splitMatch[3].length;
                    const leftToken = splitMatch[1].trim();
                    const leftIsNumeric = /^[0-9]+$/.test(leftToken);
                    const looksLikeColumns = !leftIsNumeric && leftLen >= 4 && rightLen <= 30 && gapLen >= rightLen;

                    if (looksLikeColumns) {
                        const rightStart = leftLen + gapLen;
                        const [leftHtml, rightHtml] = splitHtmlForFlexbox(lineHtml, leftLen, rightStart);

                        return '<div class="line split-line"><span>' + leftHtml + '</span><span>' + rightHtml + '</span></div>';
                    }
                }

                return '<div class="line">' + lineHtml + '</div>';
            }).join('');

            return result;
        }
`;
