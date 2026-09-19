export interface ToolExample {
    args: Record<string, unknown>;
    output: unknown;
    notes?: string;
}

interface ToolGroup {
    title: string;
    tools: string[];
}

export const SAMPLE_SESSION_ID = '7f3c1a90-2b4e-4d11-9c8a-1e6b0d4f2a18';

export const toolGroups: ToolGroup[] = [
    {
        title: 'Look around',
        tools: [
            'list_sessions',
            'get_session',
            'get_screen',
            'get_scrollback',
            'search_transcripts',
            'list_projects',
            'get_pr_board',
            'get_draft',
            'wait_for_attention',
            'get_mcp_logs',
        ],
    },
    {
        title: 'Take action',
        tools: [
            'send_input',
            'choose_option',
            'send_keys',
            'set_draft',
            'spawn_session',
            'hide_project',
            'transcribe_audio',
        ],
    },
    {
        title: 'Pull requests',
        tools: [
            'watch_pr',
            'unwatch_pr',
            'set_pr_disposition',
        ],
    },
];

const sessionMark = {
    glyph: '⊏◆⊐',
    fg: [94, 240, 255],
    bg: [0, 24, 72],
    fg_hex: '#5ef0ff',
    bg_hex: '#001848',
};

const sessionListItem = {
    id: SAMPLE_SESSION_ID,
    cwd: '/Users/sclay/projects/api',
    platform: 'claude',
    state: 'permission',
    started_at: 1740000000,
    last_activity_at: 1740000120,
    device_id: '44444444-4444-4444-8444-444444441111',
    device_name: 'MacBook Pro',
    group_id: 'grp_01h8k2',
    title: 'Add lodash and keep going',
    session_mark: sessionMark,
    prs: [
        {
            owner: 'samuelclay',
            repo: 'crabigator',
            number: 412,
            url: 'https://github.com/samuelclay/crabigator/pull/412',
            state: 'OPEN',
            primary: true,
            title: 'Rewrite the viewer socket handshake',
            branch: 'viewer-socket',
            additions: 214,
            deletions: 61,
        },
    ],
    stats: {
        prompts: 7,
        completions: 6,
        tool_calls: 42,
        thinking_seconds: 380,
    },
};

export const toolExamples: Record<string, ToolExample> = {
    list_sessions: {
        args: { needs_attention: true },
        output: { sessions: [sessionListItem] },
        notes: 'Omit filters to list every live session. needs_attention keeps only question and permission.',
    },
    get_session: {
        args: { session_id: SAMPLE_SESSION_ID },
        output: {
            id: SAMPLE_SESSION_ID,
            cwd: '/Users/sclay/projects/api',
            platform: 'claude',
            started_at: 1740000000,
            desktop_connected: true,
            state: 'permission',
            title: 'Add lodash and keep going',
            session_mark: sessionMark,
            prompt: {
                prompt_type: 'permission',
                tool_name: 'Bash',
                tool_input: { command: 'pnpm install lodash' },
                options: [
                    { label: 'Yes, allow this', value: '1' },
                    { label: "Yes, don't ask again", value: '2' },
                    { label: 'No, deny this', value: '3' },
                ],
                allows_tab_instructions: true,
                selected_option: 1,
            },
            recap: {
                status: 'ready',
                latest: {
                    prompt_count: 7,
                    generated_at: 1740000100,
                    variant: 'brief',
                    headline: 'Installing lodash after the auth refactor',
                    bullets: [
                        'Rewrote handleViewerSocket to drop stale sockets',
                        'Asked to install lodash for the new parser',
                    ],
                    next_prompt_notes: ['Approve the install, then run the tests'],
                    artifacts: [],
                    line_delta: { additions: 87, deletions: 42 },
                },
            },
            prs: [
                {
                    owner: 'samuelclay',
                    repo: 'crabigator',
                    number: 412,
                    url: 'https://github.com/samuelclay/crabigator/pull/412',
                    title: 'Rewrite the viewer socket handshake',
                    state: 'OPEN',
                    branch: 'viewer-socket',
                    primary: true,
                },
            ],
            git: {
                type: 'git',
                repo_owner: 'samuelclay',
                repo_name: 'crabigator',
                branch: 'viewer-socket',
                files: [
                    { path: 'src/app.rs', status: 'M ', additions: 87, deletions: 42 },
                    { path: 'src/ui.rs', status: 'M ', additions: 34, deletions: 12 },
                ],
            },
            stats: {
                type: 'stats',
                prompts: 7,
                completions: 6,
                tools: 42,
                compressions: 1,
                thinking_seconds: 380,
                work_seconds: 754,
                model: 'claude-opus-4-5-20251101',
            },
            draft: '',
            has_screen: true,
            scrollback_preview: 'Allow Bash to run `pnpm install lodash`?\n  1. Yes, allow this\n  2. Yes, don\'t ask again\n  3. No, deny this',
            hibernated_ephemeral: false,
        },
        notes: 'Drops the full screen and scrollback. Use get_screen or get_scrollback for those.',
    },
    get_screen: {
        args: { session_id: SAMPLE_SESSION_ID, format: 'text' },
        output: {
            session_id: SAMPLE_SESSION_ID,
            desktop_connected: true,
            hibernated_ephemeral: false,
            format: 'text',
            screen: [
                ' Claude Code',
                '',
                ' Allow Bash to run `pnpm install lodash`?',
                '  1. Yes, allow this',
                "  2. Yes, don't ask again",
                '  3. No, deny this',
                '',
                ' Tab to add instructions',
            ].join('\n'),
        },
        notes: 'Pass format=ansi to keep escape codes.',
    },
    get_scrollback: {
        args: { session_id: SAMPLE_SESSION_ID, tail: 8, query: 'lodash' },
        output: {
            session_id: SAMPLE_SESSION_ID,
            desktop_connected: true,
            hibernated_ephemeral: false,
            content: [
                '❯ add lodash for the parser',
                ' Allow Bash to run `pnpm install lodash`?',
                '  1. Yes, allow this',
            ].join('\n'),
        },
        notes: 'query keeps matching lines. tail keeps the last N lines after that filter.',
    },
    search_transcripts: {
        args: { query: 'lodash' },
        output: {
            results: [
                {
                    session_id: SAMPLE_SESSION_ID,
                    total: 3,
                    collapsed: 'Allow Bash to run `pnpm install lodash`?',
                    rows: [
                        { text: '❯ add lodash for the parser', is_match: true, gap_before: false },
                        { text: ' Allow Bash to run `pnpm install lodash`?', is_match: true, gap_before: false },
                    ],
                },
            ],
        },
        notes: 'The query must be at least 3 characters. Only live sessions hold scrollback.',
    },
    list_projects: {
        args: {},
        output: {
            projects: [
                {
                    cwd: '/Users/sclay/projects/crabigator',
                    last_active: 1740001000,
                    total_sessions: 12,
                },
                {
                    cwd: '/Users/sclay/projects/api',
                    last_active: 1739920000,
                    total_sessions: 4,
                },
            ],
        },
        notes: 'Projects from the last 14 days, minus any you hid.',
    },
    get_pr_board: {
        args: { days: 1 },
        output: {
            prs: [
                {
                    owner: 'samuelclay',
                    repo: 'crabigator',
                    number: 412,
                    updated_at: 1740001000,
                    pr: {
                        number: 412,
                        owner: 'samuelclay',
                        repo: 'crabigator',
                        url: 'https://github.com/samuelclay/crabigator/pull/412',
                        title: 'Rewrite the viewer socket handshake',
                        state: 'OPEN',
                        branch: 'viewer-socket',
                        is_draft: false,
                        additions: 214,
                        deletions: 61,
                        primary: true,
                    },
                    sessions: [
                        {
                            session_id: SAMPLE_SESSION_ID,
                            platform: 'claude',
                            dir_name: 'api',
                            branch: 'viewer-socket',
                            state: 'permission',
                            title: 'Add lodash and keep going',
                            session_mark: sessionMark,
                            recap: {
                                headline: 'Installing lodash after the auth refactor',
                                bullets: ['Asked to install lodash for the new parser'],
                            },
                        },
                    ],
                },
            ],
            sessions: [
                {
                    session_id: SAMPLE_SESSION_ID,
                    platform: 'claude',
                    dir_name: 'api',
                    state: 'permission',
                    title: 'Add lodash and keep going',
                },
            ],
            slack: [],
        },
        notes: 'Same payload the website PR board uses. days defaults to 1.',
    },
    get_mcp_logs: {
        args: { limit: 5, tool: 'send_input' },
        output: {
            calls: [
                {
                    ts: 1740000120000,
                    request_id: 'a1b2c3d4',
                    method: 'tools/call',
                    tool: 'send_input',
                    session_id: SAMPLE_SESSION_ID,
                    text_len: 41,
                    text_preview: 'approve the install, then run make test',
                    ms: 48,
                    ok: true,
                    spans: [
                        { name: 'auth', ms: 12 },
                        { name: 'session_fetch /answer', ms: 31 },
                        { name: 'tool:send_input', ms: 33 },
                    ],
                },
            ],
        },
        notes: 'Only this account’s calls. Hide SSE reconnects unless include_sse is true. Match request_id to the X-Mcp-Request-Id response header.',
    },
    get_draft: {
        args: { session_id: SAMPLE_SESSION_ID },
        output: { text: 'also run the tests after the install' },
    },
    wait_for_attention: {
        args: { timeout_seconds: 20 },
        output: { timed_out: false, sessions: [sessionListItem] },
        notes: 'Waits until a session is in question or permission. Default 20 seconds, max 25. timed_out is true when nothing needs you.',
    },
    send_input: {
        args: {
            session_id: SAMPLE_SESSION_ID,
            text: 'approve the install, then run make test',
        },
        output: { ok: true },
        notes: 'Types the text and presses Enter, same as the dashboard message box.',
    },
    choose_option: {
        args: { session_id: SAMPLE_SESSION_ID, value: '1' },
        output: { ok: true },
        notes: 'value is usually "1", "2", … from the prompt. Pass instructions to type Tab-instructions first.',
    },
    send_keys: {
        args: {
            session_id: SAMPLE_SESSION_ID,
            key: 'shift_tab',
        },
        output: { ok: true },
        notes: 'Named keys: shift_tab, escape, tab, enter, up, down, option_up, ctrl_c. Or pass steps for a sequence.',
    },
    set_draft: {
        args: {
            session_id: SAMPLE_SESSION_ID,
            text: 'also run the tests after the install',
        },
        output: { ok: true },
        notes: 'Saves unsent input without submitting it.',
    },
    spawn_session: {
        args: { cwd: '/Users/sclay/projects/api', platform: 'claude' },
        output: { ok: true },
        notes: 'Asks a live desktop to open a new session. Ghostty opens a tab in an existing window. If none is connected, returns { ok: false, fallback: "url_scheme", url: "crabigator://spawn?cwd=..." }.',
    },
    hide_project: {
        args: { cwd: '/Users/sclay/projects/old-app' },
        output: { ok: true },
        notes: 'Hides the directory from the dashboard project list for 30 days.',
    },
    transcribe_audio: {
        args: {
            audio_base64: 'UklGRiQAAABXQVZFZm10IBAAAAABAAEAESsAAAB3AQACABAAZGF0YQAAAAA=',
            mime_type: 'audio/webm',
            filename: 'prompt.webm',
        },
        output: { text: 'approve the install and keep going' },
        notes: 'Returns text only. Call send_input to submit it.',
    },
    watch_pr: {
        args: {
            owner: 'samuelclay',
            repo: 'crabigator',
            number: 412,
            url: 'https://github.com/samuelclay/crabigator/pull/412',
        },
        output: { ok: true },
        notes: 'Pass remove=true to unwatch, or use unwatch_pr.',
    },
    unwatch_pr: {
        args: { owner: 'samuelclay', repo: 'crabigator', number: 412 },
        output: { ok: true },
    },
    set_pr_disposition: {
        args: {
            owner: 'samuelclay',
            repo: 'crabigator',
            number: 412,
            disposition: 'primary',
            scope: `session:${SAMPLE_SESSION_ID}`,
        },
        output: { ok: true },
        notes: 'disposition is primary, secondary, dismissed, or auto. scope is session:<id> or path:<cwd>.',
    },
};
