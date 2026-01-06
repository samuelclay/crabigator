export interface Env {
    // D1 Database
    DB: D1Database;

    // KV Namespace for tokens
    TOKENS: KVNamespace;

    // Durable Object namespace for sessions
    SESSION: DurableObjectNamespace;

    // Durable Object for session list broadcasting
    SESSION_LIST: DurableObjectNamespace;

    // Environment variables
    API_VERSION: string;
}
