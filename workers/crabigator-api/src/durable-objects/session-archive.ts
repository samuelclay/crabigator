// Keep every SQLite value below the row limit without splitting a Unicode character.
function* chunks(content: string) {
    for (let start = 0; start < content.length;) {
        let end = Math.min(start + 32_768, content.length);
        if (end < content.length && /[\uD800-\uDBFF]/.test(content[end - 1])) end--;
        yield content.slice(start, end);
        start = end;
    }
}

// History pages address complete chunks. A desktop resync starts a new generation,
// so a reader can never append rows from a previous version of the terminal.
export class SessionArchive {
    constructor(private storage: DurableObjectStorage) {
        storage.sql.exec('CREATE TABLE IF NOT EXISTS terminal_history (part INTEGER PRIMARY KEY, content TEXT NOT NULL)');
        storage.sql.exec('CREATE TABLE IF NOT EXISTS archive_fields (name TEXT, part INTEGER, value TEXT NOT NULL, PRIMARY KEY (name, part))');
    }

    history(content: string, replace: boolean) {
        const sql = this.storage.sql;
        // Joining viewers can prompt the desktop to replay identical history.
        // Keep existing readers' cursors and avoid rewriting the archive.
        if (replace) {
            let offset = 0;
            for (const row of sql.exec<{ content: string }>('SELECT content FROM terminal_history ORDER BY part')) {
                if (!content.startsWith(row.content, offset)) { offset = -1; break; }
                offset += row.content.length;
            }
            if (offset === content.length) return;
            if (offset >= 0) { content = content.slice(offset); replace = false; }
        }
        this.storage.transactionSync(() => {
            if (replace) {
                sql.exec('DELETE FROM terminal_history');
                this.set('generation', crypto.randomUUID());
            }
            for (const chunk of chunks(content)) {
                sql.exec('INSERT INTO terminal_history (content) VALUES (?)', chunk);
            }
        });
    }

    page(generation: string | null, cursor: number) {
        const version = this.get('generation') || 'initial';
        const after = generation === version && Number.isSafeInteger(cursor) && cursor > 0 ? cursor : 0;
        let content = '', next = after, more = false;
        for (const row of this.storage.sql.exec<{ part: number; content: string }>(
            'SELECT part, content FROM terminal_history WHERE part > ? ORDER BY part LIMIT 513', after
        )) {
            if (content.length >= 192 * 1024 || next - after >= 512) { more = true; break; }
            content += row.content;
            next = row.part;
        }
        return { generation: version, after, cursor: next, more, content };
    }

    set(name: string, value: string) {
        const sql = this.storage.sql;
        this.storage.transactionSync(() => {
            sql.exec('DELETE FROM archive_fields WHERE name = ?', name);
            let part = 0;
            for (const chunk of chunks(value)) {
                sql.exec('INSERT INTO archive_fields VALUES (?, ?, ?)', name, part++, chunk);
            }
        });
    }

    get(name: string): string | null {
        const rows = this.storage.sql.exec<{ value: string }>('SELECT value FROM archive_fields WHERE name = ? ORDER BY part', name).toArray();
        return rows.length ? rows.map(row => row.value).join('') : null;
    }
}
