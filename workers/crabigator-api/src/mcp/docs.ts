import type { RuntimeConfig } from '../config';
import { escapeHtml, metaPixelHtml, usePublicOrigin } from '../html-render';
import { landingCss } from '../landing/css';
import { mcpCss } from '../landing/mcp-css';
import { analyticsJs } from '../landing/analytics';
import {
    iconCopy,
    iconCrabigator,
    iconCrabigatorEncoded,
    iconGithubSmall,
    iconPlug,
} from '../landing/icons';
import { SAMPLE_SESSION_ID, toolExamples, toolGroups, type ToolExample, type ToolGroup } from './examples';
import { listToolDescriptors } from './tools';

export const MCP_TOOLS_PATH = '/mcp-tools';

type JsonSchema = Record<string, unknown>;

function mcpConnectRow(label: string, id: string, value: string, copyLabel: string): string {
    return `
                    <div class="mcp-connect-row">
                        <span class="mcp-connect-label">${label}</span>
                        <code class="mcp-connect-value" id="${id}">${value}</code>
                        <button type="button" class="mcp-connect-copy" data-copy-target="${id}" aria-label="${copyLabel}">
                            ${iconCopy}
                        </button>
                    </div>`;
}

function mcpConnectHtml(): string {
    const url = 'https://drinkcrabigator.com/mcp';
    const rows = [
        mcpConnectRow('URL', 'mcp-url-text', url, 'Copy MCP URL'),
        mcpConnectRow('Claude', 'mcp-claude-text', `claude mcp add --transport http crabigator ${url}`, 'Copy Claude command'),
        mcpConnectRow('Codex', 'mcp-codex-text', `codex mcp add crabigator --url ${url}`, 'Copy Codex command'),
        mcpConnectRow('opencode', 'mcp-opencode-text', `opencode mcp add crabigator --url ${url}`, 'Copy opencode command'),
        mcpConnectRow('Grok', 'mcp-grok-text', `grok mcp add --transport http crabigator ${url}`, 'Copy Grok command'),
    ].join('');
    return `
                <div class="mcp-connect">${rows}
                </div>`;
}

export function mcpLandingSectionHtml(): string {
    const count = listToolDescriptors().length;
    return `
    <section class="section mcp-section" id="mcp">
        <div class="mcp-intro">
            <div class="mcp-intro-copy">
                <h2 class="section-title">Let another agent drive your sessions</h2>
                <p class="section-subtitle">
                    Point Claude, Cursor, Grok, or any MCP client at your live Crabigator sessions.
                    Sign in with GitHub or Google. The agent can list sessions, read screens,
                    answer prompts, and manage PRs. Same as the dashboard.
                </p>
                ${mcpConnectHtml()}
                <div class="mcp-intro-actions">
                    <a href="${MCP_TOOLS_PATH}" class="btn-primary" data-track="mcp_tools" data-label="landing">
                        See all ${count} tools
                    </a>
                    <p class="mcp-intro-note">
                        If this account has no desktop yet, enter a pairing code from <code>crabigator pair</code>.
                    </p>
                </div>
            </div>
            ${mcpDemoHtml()}
        </div>
        <div class="mcp-groups">
            ${toolGroups.map(groupHtml).join('')}
        </div>
    </section>`;
}

function groupHtml(group: ToolGroup): string {
    return `
            <div class="mcp-group">
                <h3 class="mcp-group-title">${escapeHtml(group.title)}</h3>
                <p class="mcp-group-blurb">${escapeHtml(group.blurb)}</p>
                <ul class="mcp-group-tools">
                    ${group.tools.map((name) => (
                        `<li><a href="${MCP_TOOLS_PATH}#${escapeHtml(name)}">${escapeHtml(name)}</a></li>`
                    )).join('')}
                </ul>
            </div>`;
}

function mcpDemoHtml(): string {
    const listed = {
        sessions: [{
            id: SAMPLE_SESSION_ID,
            cwd: '/Users/sclay/projects/api',
            state: 'permission',
            title: 'Add lodash and keep going',
            session_mark: { glyph: '⊏◆⊐' },
        }],
    };
    const chosen = toolExamples.choose_option;
    return `
            <div class="mcp-demo" aria-hidden="true">
                <div class="mcp-demo-bar">
                    <span class="mcp-demo-dot red"></span>
                    <span class="mcp-demo-dot yellow"></span>
                    <span class="mcp-demo-dot green"></span>
                    <span class="mcp-demo-title">MCP · crabigator</span>
                </div>
                <div class="mcp-demo-body">
                    <div class="mcp-demo-step">
                        <div class="mcp-demo-call">list_sessions ${inlineArgs(toolExamples.list_sessions.args)}</div>
                        <div class="mcp-demo-json">${highlightJson(listed)}</div>
                    </div>
                    <div class="mcp-demo-step">
                        <div class="mcp-demo-call">choose_option ${inlineArgs(chosen.args)}</div>
                        <div class="mcp-demo-json">${highlightJson(chosen.output)}</div>
                    </div>
                </div>
            </div>`;
}

function mcpLogsGuideHtml(): string {
    return `
            <article class="mcp-guide" id="logs" data-tool="logs" data-search="logs diagnose send_input request_id">
                <h2>Read the logs from another repo</h2>
                <p>
                    If a client sent <code>send_input</code> (or any tool) and the Crabigator session did not change,
                    compare the client's send log with what this server stored. Public recipe:
                    <code>https://drinkcrabigator.com/mcp-tools#logs</code>
                </p>
                <h3 class="mcp-subhead">1. What the client should log</h3>
                <p>On every <code>POST https://drinkcrabigator.com/mcp</code> JSON-RPC call, record:</p>
                <ul>
                    <li>JSON-RPC <code>id</code>, <code>method</code> (usually <code>tools/call</code>), <code>params.name</code>, and <code>params.arguments.session_id</code></li>
                    <li>For <code>send_input</code>: <code>text.length</code> and the first 80 characters</li>
                    <li>HTTP status</li>
                    <li>Response headers <code>X-Mcp-Request-Id</code>, <code>Server-Timing</code>, and <code>cf-ray</code></li>
                </ul>
                <h3 class="mcp-subhead">2. What the server stored</h3>
                <p>
                    Call the <a href="#get_mcp_logs"><code>get_mcp_logs</code></a> tool on this same MCP server.
                    Each row is one request this account made. <code>request_id</code> is the
                    <code>X-Mcp-Request-Id</code> header. For typed input you also get <code>text_len</code>
                    and <code>text_preview</code>.
                </p>
                <p>
                    <code>GET /mcp</code> is the notification stream. It reconnects about every 25 seconds and is
                    not a tool call. <code>get_mcp_logs</code> hides those rows unless you pass <code>include_sse: true</code>.
                </p>
                <h3 class="mcp-subhead">3. How to read a mismatch</h3>
                <table>
                    <thead><tr><th>Client</th><th>Server log</th><th>Meaning</th></tr></thead>
                    <tbody>
                        <tr>
                            <td>Sent <code>tools/call</code></td>
                            <td>No matching <code>request_id</code></td>
                            <td>The POST never reached Crabigator (wrong URL, auth failed, or the client never sent it).</td>
                        </tr>
                        <tr>
                            <td>Sent <code>send_input</code></td>
                            <td><code>ok: false</code></td>
                            <td>The server got it and rejected it. Read <code>error</code> (often desktop offline or bad session id).</td>
                        </tr>
                        <tr>
                            <td>Sent <code>send_input</code></td>
                            <td><code>ok: true</code></td>
                            <td>The server accepted it. If the terminal did not type, the desktop is not attached to that session.</td>
                        </tr>
                        <tr>
                            <td>Nothing sent</td>
                            <td>Only SSE <code>GET</code> rows</td>
                            <td>The client is idle on the notification stream. It is not calling tools.</td>
                        </tr>
                    </tbody>
                </table>
            </article>`;
}

export function renderMcpToolsHtml(runtime: RuntimeConfig, metaPixelId = ''): string {
    const pixel = runtime.capabilities.marketing_analytics ? metaPixelHtml(metaPixelId) : '';
    const tools = listToolDescriptors();
    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crabigator MCP tools</title>
    <meta name="description" content="Every Crabigator MCP tool, with arguments and example outputs. List sessions, read screens, answer prompts, and drive the PR board.">
    <meta property="og:title" content="Crabigator MCP tools">
    <meta property="og:description" content="Every Crabigator MCP tool, with arguments and example outputs.">
    <meta property="og:image" content="https://drinkcrabigator.com/assets/og-landing.png">
    <meta property="og:url" content="https://drinkcrabigator.com${MCP_TOOLS_PATH}">
    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Crabigator">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="Crabigator MCP tools">
    <meta name="twitter:description" content="Every Crabigator MCP tool, with arguments and example outputs.">
    <meta name="twitter:image" content="https://drinkcrabigator.com/assets/og-landing.png">
    <link rel="icon" href="data:image/svg+xml,${iconCrabigatorEncoded}">
    <style>${landingCss}${mcpCss}${
        !runtime.capabilities.billing ? '.nav-link[href="/#pricing"]{display:none!important}' : ''
    }</style>
    ${pixel}
</head>
<body>
    <nav class="nav">
        <a href="/" class="nav-logo">
            ${iconCrabigator}
            Crabigator
        </a>
        <div class="nav-links">
            <a href="/#features" class="nav-link">Features</a>
            <a href="${MCP_TOOLS_PATH}" class="nav-link active">MCP</a>
            <a href="/#security" class="nav-link">Security</a>
            <a href="/#pricing" class="nav-link">Pricing</a>
            <a href="/#install" class="nav-link">Install</a>
            <a href="/dashboard" class="nav-btn" data-track="dashboard_nav" data-label="mcp-tools">Open Dashboard</a>
            <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="nav-github">
                ${iconGithubSmall}
            </a>
        </div>
    </nav>

    <header class="mcp-docs-hero">
        <p class="section-label">${iconPlug} MCP</p>
        <h1>MCP tools</h1>
        <p>
            Point an MCP client at the URL below. After GitHub or Google sign-in, it can call these ${tools.length} tools.
            Each response is JSON returned as MCP text. The samples below are representative, not live data.
        </p>
        ${mcpConnectHtml()}
    </header>

    <div class="mcp-docs-layout">
        <aside class="mcp-toc">
            <div class="mcp-toc-label">Tools</div>
            <input type="search" class="mcp-filter" id="mcp-filter" placeholder="Filter tools" aria-label="Filter tools">
            <div class="mcp-toc-links">
                <div class="mcp-toc-group">
                    <div class="mcp-toc-group-title">Diagnose</div>
                    <a href="#logs" data-tool="logs">Read the logs</a>
                </div>
                ${toolGroups.map((group) => `
                <div class="mcp-toc-group">
                    <div class="mcp-toc-group-title">${escapeHtml(group.title)}</div>
                    ${group.tools.map((name) => `<a href="#${escapeHtml(name)}" data-tool="${escapeHtml(name)}">${escapeHtml(name)}</a>`).join('')}
                </div>`).join('')}
            </div>
        </aside>
        <main>
            ${mcpLogsGuideHtml()}
            ${tools.map(toolCardHtml).join('')}
        </main>
    </div>

    <footer class="footer">
        <div class="footer-content">
            <div class="footer-cta">
                <span class="footer-cta-text">Install Crabigator</span>
                <a href="/#install" class="btn-primary" data-track="install_cta" data-label="mcp-tools">Install</a>
            </div>
            <div class="footer-links">
                <a href="/dashboard" class="footer-link">Dashboard</a>
                <a href="${MCP_TOOLS_PATH}" class="footer-link">MCP</a>
                <a href="https://github.com/samuelclay/crabigator" target="_blank" rel="noopener" class="footer-link">GitHub</a>
                <a href="https://github.com/samuelclay/crabigator#readme" target="_blank" rel="noopener" class="footer-link">Documentation</a>
            </div>
            <div class="footer-meta">
                MIT License
            </div>
        </div>
        <div class="footer-bottom">
            <p class="footer-bottom-text">Built by <a href="https://samuelclay.com" target="_blank" rel="noopener">Samuel Clay</a> in San Francisco. Talk to <a href="https://x.com/samuelclay" target="_blank" rel="noopener">@samuelclay on X</a>.</p>
        </div>
    </footer>

    ${runtime.capabilities.marketing_analytics ? `<script>${analyticsJs}</script>` : ''}
    <script>${mcpDocsJs}</script>
</body>
</html>`;
    return usePublicOrigin(html, runtime.origin);
}

function toolCardHtml(tool: { name: string; description: string; inputSchema: JsonSchema }): string {
    const example = toolExamples[tool.name];
    const args = argumentRows(tool.inputSchema);
    return `
            <article class="mcp-tool" id="${escapeHtml(tool.name)}" data-search="${escapeHtml(`${tool.name} ${tool.description}`.toLowerCase())}">
                <h2 class="mcp-tool-name">${escapeHtml(tool.name)}</h2>
                <p class="mcp-tool-desc">${escapeHtml(tool.description)}</p>
                ${example?.notes ? `<p class="mcp-tool-notes">${escapeHtml(example.notes)}</p>` : ''}
                <h3 class="mcp-subhead">Arguments</h3>
                ${args || '<p class="mcp-empty">No arguments.</p>'}
                ${exampleHtml(tool.name, example)}
            </article>`;
}

function argumentRows(schema: JsonSchema): string {
    const properties = (schema.properties || {}) as Record<string, JsonSchema>;
    const entries = Object.entries(properties);
    if (!entries.length) return '';
    const required = new Set(Array.isArray(schema.required) ? schema.required as string[] : []);
    const rows = entries.map(([name, spec]) => {
        const desc = typeof spec.description === 'string' ? spec.description : '';
        return `<tr>
                    <td><code>${escapeHtml(name)}</code>${required.has(name) ? '<span class="mcp-req">required</span>' : ''}</td>
                    <td><code>${escapeHtml(schemaType(spec))}</code></td>
                    <td>${escapeHtml(desc)}</td>
                </tr>`;
    }).join('');
    return `<table class="mcp-args">
                    <thead><tr><th>Name</th><th>Type</th><th>Description</th></tr></thead>
                    <tbody>${rows}</tbody>
                </table>`;
}

function schemaType(spec: JsonSchema): string {
    if (Array.isArray(spec.enum)) return spec.enum.map((value) => JSON.stringify(value)).join(' | ');
    if (typeof spec.type === 'string') return spec.type;
    return 'any';
}

function exampleHtml(name: string, example: ToolExample | undefined): string {
    if (!example) {
        return '<p class="mcp-empty">No sample output yet.</p>';
    }
    return `
                <h3 class="mcp-subhead">Example call</h3>
                ${jsonBlock({ name, arguments: example.args })}
                <h3 class="mcp-subhead">Example output</h3>
                ${jsonBlock(example.output)}`;
}

function jsonBlock(value: unknown): string {
    return `<div class="mcp-code">
                    <button type="button" class="mcp-code-copy">Copy</button>
                    <pre>${highlightJson(value)}</pre>
                </div>`;
}

function inlineArgs(args: Record<string, unknown>): string {
    const keys = Object.keys(args);
    if (!keys.length) return '';
    return escapeHtml(JSON.stringify(args));
}

function highlightJson(value: unknown): string {
    const json = JSON.stringify(value, null, 2);
    return json.replace(
        /("(?:\\.|[^"\\])*")(\s*:)?|\b(-?\d+(?:\.\d+)?)\b|\b(true|false|null)\b/g,
        (_match, str: string | undefined, colon: string | undefined, num: string | undefined, kw: string | undefined) => {
            if (str) {
                const escaped = escapeHtml(str);
                return colon
                    ? `<span class="mcp-jk">${escaped}</span>${colon}`
                    : `<span class="mcp-js">${escaped}</span>`;
            }
            if (num) return `<span class="mcp-jn">${escapeHtml(num)}</span>`;
            return `<span class="mcp-jb">${escapeHtml(kw || '')}</span>`;
        },
    );
}

const mcpDocsJs = `
    function bindCopyButtons() {
        document.querySelectorAll('[data-copy-target]').forEach((button) => {
            button.addEventListener('click', () => {
                const id = button.getAttribute('data-copy-target');
                const source = id ? document.getElementById(id) : null;
                copyText(source ? source.textContent.trim() : '', button);
            });
        });
        document.querySelectorAll('.mcp-code-copy').forEach((button) => {
            button.addEventListener('click', () => {
                const pre = button.parentElement && button.parentElement.querySelector('pre');
                copyText(pre ? pre.textContent : '', button);
            });
        });
    }

    function copyText(text, button) {
        if (!text) return;
        const restoreLabel = button.classList.contains('mcp-code-copy') ? button.textContent : null;
        navigator.clipboard.writeText(text).then(() => {
            button.classList.add('copied');
            if (restoreLabel !== null) button.textContent = 'Copied';
            setTimeout(() => {
                button.classList.remove('copied');
                if (restoreLabel !== null) button.textContent = restoreLabel;
            }, 1600);
        }).catch(() => {});
    }

    function bindFilter() {
        const input = document.getElementById('mcp-filter');
        if (!input) return;
        const cards = Array.from(document.querySelectorAll('.mcp-tool'));
        const links = Array.from(document.querySelectorAll('.mcp-toc [data-tool]'));
        const groups = Array.from(document.querySelectorAll('.mcp-toc-group'));
        input.addEventListener('input', () => {
            const query = input.value.trim().toLowerCase();
            cards.forEach((card) => {
                const haystack = card.getAttribute('data-search') || '';
                card.classList.toggle('mcp-hidden', Boolean(query) && !haystack.includes(query));
            });
            links.forEach((link) => {
                const name = (link.getAttribute('data-tool') || '').toLowerCase();
                link.classList.toggle('mcp-hidden', Boolean(query) && !name.includes(query));
            });
            groups.forEach((group) => {
                const visible = group.querySelectorAll('a:not(.mcp-hidden)').length;
                group.classList.toggle('mcp-hidden', Boolean(query) && visible === 0);
            });
        });
    }

    bindCopyButtons();
    bindFilter();
`;
