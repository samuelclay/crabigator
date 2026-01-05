/**
 * Session state matching crabigator's Rust SessionState enum
 */
export type SessionState = 'ready' | 'thinking' | 'permission' | 'question' | 'complete';

/**
 * Scrollback event - append-only diff of newly added lines
 */
export interface ScrollbackEvent {
    type: 'scrollback';
    diff: string;           // Newly appended lines since last update
    total_lines: number;    // Total line count (for verification)
}

/**
 * State change event
 */
export interface StateEvent {
    type: 'state';
    state: SessionState;
    timestamp: number;      // Unix timestamp (ms)
}

/**
 * Git file status
 */
export interface GitFile {
    path: string;
    status: string;         // Git porcelain format: "M ", "??", "A ", etc.
    additions: number;
    deletions: number;
}

/**
 * Git status event
 */
export interface GitEvent {
    type: 'git';
    branch: string;
    files: GitFile[];
}

/**
 * Code change (function, method, class modification)
 */
export interface CodeChange {
    kind: string;           // "Function", "Method", "Class", etc.
    name: string;           // Symbol name
    change_type: string;    // "added", "modified", "deleted"
    additions: number;
    deletions: number;
    file_path?: string;
    line_number?: number;
}

/**
 * Changes grouped by language
 */
export interface LanguageChanges {
    language: string;
    changes: CodeChange[];
}

/**
 * Code changes event
 */
export interface ChangesEvent {
    type: 'changes';
    by_language: LanguageChanges[];
}

/**
 * Session statistics event
 */
export interface StatsEvent {
    type: 'stats';
    prompts: number;
    completions: number;
    tools: number;
    thinking_seconds: number;
    work_seconds: number;
}

/**
 * ANSI screen snapshot (for late joiners)
 */
export interface ScreenEvent {
    type: 'screen';
    content: string;        // ANSI-escaped screen content
}

/**
 * Desktop connection status event (for dashboard)
 */
export interface DesktopStatusEvent {
    type: 'desktop_status';
    connected: boolean;
    timestamp: number;
}

/**
 * Union of all session event types
 */
export type SessionEvent =
    | ScrollbackEvent
    | StateEvent
    | GitEvent
    | ChangesEvent
    | StatsEvent
    | ScreenEvent
    | DesktopStatusEvent;

/**
 * Message from cloud to desktop (via WebSocket)
 */
export interface AnswerMessage {
    type: 'answer';
    text: string;           // Text to inject into PTY
}

export interface PingMessage {
    type: 'ping';
}

export type CloudToDesktopMessage = AnswerMessage | PingMessage;

/**
 * Session info for listing
 */
export interface SessionInfo {
    id: string;
    client_session_id: string;
    cwd: string;
    platform: 'claude' | 'codex';
    state: SessionState;
    started_at: number;
    ended_at: number | null;
    is_active: boolean;
    stats: {
        prompts: number;
        completions: number;
        tool_calls: number;
        thinking_seconds: number;
    };
}
