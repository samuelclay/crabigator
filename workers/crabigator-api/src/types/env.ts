export interface Env {
    // D1 Database
    DB: D1Database;

    // KV Namespace for tokens
    TOKENS: KVNamespace;

    // Durable Object namespace for sessions
    SESSION: DurableObjectNamespace;

    // Environment variables
    API_VERSION: string;
}
