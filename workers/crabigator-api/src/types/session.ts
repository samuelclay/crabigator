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
 * Scrollback history event - full accumulated scrollback for late joiners
 */
export interface ScrollbackHistoryEvent {
    type: 'scrollback_history';
    content: string;        // Full accumulated scrollback content
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
 * Claude Code operating mode
 */
export type ClaudeMode = 'normal' | 'auto_accept' | 'plan';

/**
 * Permission option extracted from screen content
 */
export interface PermissionOption {
    /** Option number (1, 2, 3, etc.) */
    number: number;
    /** Full text of the option */
    text: string;
    /** Whether this option is currently selected */
    selected: boolean;
}

/**
 * Permission suggestion metadata
 */
export interface PermissionSuggestion {
    type: string;
    mode?: string;
    behavior?: string;
}

/**
 * Permission details when in permission state
 */
export interface PermissionInfo {
    tool: string;
    suggestions: PermissionSuggestion[];
    /** Options extracted from screen content (the actual menu items shown to user) */
    options?: PermissionOption[];
    /** The question being asked (e.g., "Do you want to create test-file.txt?") */
    question?: string;
}

/**
 * Session statistics event
 */
export interface StatsEvent {
    type: 'stats';
    prompts: number;
    completions: number;
    tools: number;
    compressions: number;
    thinking_seconds: number;
    work_seconds: number;
    mode?: ClaudeMode;
    /** Permission details when in permission state */
    permission?: PermissionInfo;
    /** Model name (e.g., "claude-opus-4-5-20251101") */
    model?: string;
    /** Unix timestamp when prompts count last changed */
    prompts_changed_at?: number;
    /** Unix timestamp when completions count last changed */
    completions_changed_at?: number;
    /** Unix timestamp when compressions count last changed */
    compressions_changed_at?: number;
    /** Unix timestamps of tool invocations for sparkline */
    tool_timestamps: number[];
    /** Unix timestamp when session started */
    session_start: number;
    /** Unix timestamp when session became idle (for idle time display) */
    idle_since?: number;
    /** Autocomplete suggestion from Claude Code's input line */
    suggestion?: string;
}

/**
 * ANSI screen snapshot (for late joiners)
 */
export interface ScreenEvent {
    type: 'screen';
    content: string;        // ANSI-escaped screen content
}

/**
 * Terminal title event (from OSC sequences)
 */
export interface TitleEvent {
    type: 'title';
    title: string;          // Terminal title extracted from OSC sequences
}

/**
 * Title history event - all titles from the session
 */
export interface TitleHistoryEvent {
    type: 'title_history';
    history: string[];      // All terminal titles from this session
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
 * A selectable option in a prompt
 */
export interface PromptOption {
    label: string;
    value: string;           // What to send back when selected (e.g., "1", "y", "n")
    description?: string;
}

/**
 * A question in an AskUserQuestion prompt
 */
export interface QuestionData {
    question: string;
    header?: string;
    options: PromptOption[];
    multi_select?: boolean;
    allows_other?: boolean;  // Whether free-text "Other" input is allowed
}

/**
 * Question prompt data (AskUserQuestion)
 */
export interface QuestionPrompt {
    prompt_type: 'question';
    questions: QuestionData[];
}

/**
 * Permission prompt data
 */
export interface PermissionPrompt {
    prompt_type: 'permission';
    tool_name: string;
    tool_input?: unknown;
    options: PromptOption[];
    /** Whether "Tab to add additional instructions" is available */
    allows_tab_instructions?: boolean;
    /** Currently selected option number (1-indexed) */
    selected_option?: number;
}

/**
 * ExitPlanMode prompt data
 */
export interface ExitPlanPrompt {
    prompt_type: 'exit_plan';
    options: PromptOption[];
}

/**
 * Union of prompt data types
 */
export type CloudPromptData = QuestionPrompt | PermissionPrompt | ExitPlanPrompt;

/**
 * Prompt event - sent when entering/leaving interactive states
 */
export interface PromptEvent {
    type: 'prompt';
    prompt: CloudPromptData | null;  // null to clear
}

/**
 * Union of all session event types
 */
export type SessionEvent =
    | ScrollbackEvent
    | ScrollbackHistoryEvent
    | StateEvent
    | GitEvent
    | ChangesEvent
    | StatsEvent
    | ScreenEvent
    | TitleEvent
    | TitleHistoryEvent
    | DesktopStatusEvent
    | PromptEvent;

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

/**
 * Key command to send to desktop (e.g., Shift+Tab for mode switching)
 */
export interface KeyMessage {
    type: 'key';
    key: string;            // Key identifier: "shift_tab", etc.
}

/**
 * A single step in a key sequence
 */
export type KeyStep =
    | { type: 'key'; key: string }      // Named key: "up", "down", "tab", "enter"
    | { type: 'text'; text: string }    // Raw text to type
    | { type: 'delay'; ms: number };    // Wait in milliseconds

/**
 * Multi-step key sequence to send to desktop (for Tab instructions)
 */
export interface KeySequenceMessage {
    type: 'key_sequence';
    steps: KeyStep[];
}

/**
 * Viewer status notification to desktop
 * Sent when dashboard/phone viewer becomes active/inactive
 * Desktop can use this to adjust streaming frequency
 */
export interface ViewerStatusMessage {
    type: 'viewer_status';
    active: boolean;        // True if viewers are actively watching
}

export type CloudToDesktopMessage = AnswerMessage | PingMessage | KeyMessage | KeySequenceMessage | ViewerStatusMessage;

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
