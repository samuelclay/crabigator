import { presentConductorEntry } from './conductor-content';

type Entry = Record<string, any>;

/** History is independent of the coordinator's bounded model context. */
export class ConductorArchive {
    private previous = new Map<string, string>();
    constructor(private storage: DurableObjectStorage, entries: Entry[], seeded: boolean) {
        storage.sql.exec(`CREATE TABLE IF NOT EXISTS conductor_archive (
            id TEXT PRIMARY KEY, kind TEXT NOT NULL, state TEXT NOT NULL,
            project TEXT NOT NULL, session_id TEXT NOT NULL, updated_at REAL NOT NULL,
            search_text TEXT NOT NULL, payload TEXT NOT NULL)`);
        storage.sql.exec('CREATE INDEX IF NOT EXISTS conductor_archive_date ON conductor_archive(updated_at DESC, id)');
        if (seeded) for (const entry of entries) this.previous.set(this.key(entry), JSON.stringify(entry));
    }
    private key(entry: Entry) { return `${entry.kind}:${entry.id}`; }
    sync(entries: Entry[]) {
        for (const entry of entries) {
            const id = this.key(entry), payload = JSON.stringify(entry);
            if (this.previous.get(id) === payload) continue;
            const search = [entry.project, entry.session_title, entry.session_id, entry.device_name, entry.title, entry.question, entry.text, entry.result, ...(entry.highlights || []).map((h: Entry) => [h.project, h.session_title, h.text].join(' ')), ...(entry.events || []).map((e: Entry) => e.text)].join('\n').toLowerCase();
            this.storage.sql.exec(`INSERT INTO conductor_archive VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET state=excluded.state, project=excluded.project,
                session_id=excluded.session_id, updated_at=excluded.updated_at, search_text=excluded.search_text, payload=excluded.payload`,
            id, entry.kind, entry.state || 'conversation', entry.project || 'Conductor', entry.session_id || '', entry.updated_at || entry.at || 0, search, payload);
            this.previous.set(id, payload);
        }
    }
    message(id: string): Entry | undefined {
        const row = this.storage.sql.exec<{ payload: string }>('SELECT payload FROM conductor_archive WHERE id=?', `message:${id}`).toArray()[0];
        return row ? JSON.parse(row.payload) : undefined;
    }
    search(query: string, status: string, offset: number) {
        const states: Record<string, string[]> = {
            active: ['sending', 'forwarded', 'delivered', 'working', 'waiting', 'delivery_unknown'],
            answered: ['answered'], failed: ['failed'], stopped: ['cancelled'],
        };
        const selected = states[status];
        const where = 'instr(search_text, ?) > 0' + (selected ? ` AND state IN (${selected.map(() => '?').join(',')})` : '');
        const params = [query.toLowerCase(), ...(selected || [])];
        const total = this.storage.sql.exec<{ count: number }>(`SELECT count(*) AS count FROM conductor_archive WHERE ${where}`, ...params).one().count;
        const rows = this.storage.sql.exec<{ payload: string }>(`SELECT payload FROM conductor_archive WHERE ${where} ORDER BY updated_at DESC, id LIMIT 30 OFFSET ?`, ...params, offset).toArray();
        const entries: Entry[] = rows.map(row => {
            const entry = JSON.parse(row.payload);
            const source = [entry.result, entry.text, entry.question, entry.title, ...(entry.highlights || []).map((h: Entry) => h.text), ...(entry.events || []).map((e: Entry) => e.text)].filter(Boolean).join('\n');
            const start = Math.max(0, source.toLowerCase().indexOf(query.toLowerCase()) - 70);
            return { ...presentConductorEntry(entry), excerpt: (start ? '…' : '') + source.slice(start, start + 220) + (source.length > start + 220 ? '…' : '') };
        });
        return { entries, total, next_offset: offset + rows.length < total ? offset + rows.length : null };
    }
}
