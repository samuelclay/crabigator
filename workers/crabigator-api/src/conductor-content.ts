/** Small, shared presentation model. Clients render text, never model-supplied HTML. */
export function conductorBlocks(text: string): Record<string, unknown>[] {
    const lines = text.replace(/\r\n/g, '\n').split('\n');
    const blocks: Record<string, unknown>[] = [];
    const cells = (line: string) => line.trim().replace(/^\|/, '').replace(/\|$/, '').split(/(?<!\\)\|/).map(s => s.trim().replace(/\\\|/g, '|'));
    for (let i = 0; i < lines.length;) {
        const line = lines[i];
        if (!line.trim()) { i++; continue; }
        if (/^\s*```/.test(line)) {
            const code: string[] = []; i++;
            while (i < lines.length && !/^\s*```/.test(lines[i])) code.push(lines[i++]);
            i++; blocks.push({ type: 'code', text: code.join('\n') }); continue;
        }
        if (line.includes('|') && i + 1 < lines.length && cells(lines[i + 1]).every(s => /^:?-{3,}:?$/.test(s))) {
            const headers = cells(line), rows: string[][] = []; i += 2;
            while (i < lines.length && lines[i].includes('|') && lines[i].trim()) rows.push(cells(lines[i++]));
            blocks.push({ type: 'table', headers, rows }); continue;
        }
        if (/^#{1,6}\s/.test(line)) { blocks.push({ type: 'heading', text: line.replace(/^#{1,6}\s+/, '') }); i++; continue; }
        const paragraph = [line]; i++;
        while (i < lines.length && lines[i].trim() && !/^\s*```|^#{1,6}\s/.test(lines[i]) && !(lines[i].includes('|') && i + 1 < lines.length && cells(lines[i + 1]).every(s => /^:?-{3,}:?$/.test(s)))) paragraph.push(lines[i++]);
        blocks.push({ type: 'paragraph', text: paragraph.join('\n') });
    }
    return blocks;
}

export function presentConductorEntry(entry: Record<string, any>): Record<string, any> {
    return { ...entry, blocks: conductorBlocks(entry.text || entry.result || ''), question_blocks: conductorBlocks(entry.question || '') };
}
