// Styles for the landing MCP section and the /mcp-tools listing.
export const mcpCss = `
/* === MCP SECTION (landing) === */
.mcp-section {
    border-top: 1px solid var(--border-dim);
    border-bottom: 1px solid var(--border-dim);
    scroll-margin-top: 96px;
    width: 100%;
    overflow-x: hidden;
}
.mcp-section::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='280' height='200'%3E%3Cstyle%3Etext%7Bfill:rgba(34,211,238,0.16);font-family:monospace;font-size:10px%7D%3C/style%3E%3Ctext x='16' y='24'%3Etools/call%3C/text%3E%3Ctext x='160' y='24'%3E%7B%7D%3C/text%3E%3Ctext x='40' y='64'%3Emcp%3C/text%3E%3Ctext x='200' y='64'%3E%3C/%3E%3C/text%3E%3Ctext x='80' y='104'%3Elist_sessions%3C/text%3E%3Ctext x='12' y='144'%3Echoose_option%3C/text%3E%3Ctext x='180' y='144'%3Eok%3C/text%3E%3Ctext x='48' y='184'%3Ewait_for_attention%3C/text%3E%3C/svg%3E");
    background-size: 280px 200px;
    pointer-events: none;
}
.mcp-section > * { position: relative; z-index: 1; }

.mcp-intro {
    max-width: 1200px;
    margin: 0 auto 48px;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 420px);
    gap: 64px;
    align-items: start;
    min-width: 0;
}
.mcp-intro-copy .section-subtitle {
    margin: 0 0 28px;
    max-width: 540px;
}

.mcp-connect {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 28px;
    width: 100%;
    min-width: 0;
}
.mcp-connect-row {
    display: flex;
    align-items: stretch;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    overflow: hidden;
    min-width: 0;
    max-width: 100%;
}
.mcp-connect-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--accent-magenta);
    padding: 12px 14px;
    border-right: 1px solid var(--border-dim);
    display: flex;
    align-items: center;
    white-space: nowrap;
    background: var(--bg-surface);
    min-width: 8.4em;
}
.mcp-connect-value {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--accent-cyan);
    padding: 12px 14px;
    flex: 1;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
}
.mcp-connect-copy {
    background: transparent;
    border: none;
    border-left: 1px solid var(--border-dim);
    color: var(--text-dim);
    padding: 0 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
}
.mcp-connect-copy:hover { color: var(--accent-cyan); }
.mcp-connect-copy.copied { color: var(--accent-green); }
.mcp-connect-copy svg { width: 14px; height: 14px; }

.mcp-intro-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
}
.mcp-intro-note {
    font-size: 13px;
    color: var(--text-dim);
    max-width: 36em;
    overflow-wrap: anywhere;
}
.mcp-intro-note code,
.mcp-docs-hero code {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-cyan);
    background: var(--bg-abyss);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.95em;
}

.mcp-demo {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    overflow: hidden;
    min-width: 0;
    max-width: 100%;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.35);
}
.mcp-demo-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-dim);
}
.mcp-demo-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-dim);
}
.mcp-demo-dot.red { background: #f87171; }
.mcp-demo-dot.yellow { background: #fbbf24; }
.mcp-demo-dot.green { background: #4ade80; }
.mcp-demo-title {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    margin-left: 4px;
}
.mcp-demo-body { padding: 16px 18px 20px; }
.mcp-demo-step { margin-bottom: 18px; }
.mcp-demo-step:last-child { margin-bottom: 0; }
.mcp-demo-call {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-cyan);
    margin-bottom: 8px;
    overflow-wrap: anywhere;
    white-space: normal;
}
.mcp-demo-call::before {
    content: '→ ';
    color: var(--accent-magenta);
}
.mcp-demo-json {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-mid);
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 6px;
    padding: 10px 12px;
    overflow-x: auto;
    max-width: 100%;
    white-space: pre;
}

.mcp-groups {
    max-width: 1200px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
}
.mcp-group {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 24px;
}
.mcp-group-title {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 8px;
}
.mcp-group-blurb {
    font-size: 14px;
    color: var(--text-mid);
    margin-bottom: 16px;
    line-height: 1.5;
}
.mcp-group-tools {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
}
.mcp-group-tools a {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    color: var(--accent-cyan);
    padding: 6px 10px;
    border: 1px solid transparent;
    border-radius: 6px;
    display: block;
}
.mcp-group-tools a:hover {
    border-color: var(--border-dim);
    background: rgba(34, 211, 238, 0.06);
    text-decoration: none;
}

/* === MCP TOOLS PAGE === */
.mcp-docs-hero {
    padding: 140px 32px 48px;
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    overflow-x: hidden;
}
.mcp-docs-hero .section-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--accent-magenta);
    text-transform: uppercase;
    letter-spacing: 3px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
}
.mcp-docs-hero .section-label svg {
    width: 16px;
    height: 16px;
    fill: currentColor;
}
.mcp-docs-hero h1 {
    font-size: clamp(32px, 5vw, 52px);
    letter-spacing: -1px;
    line-height: 1.1;
    margin-bottom: 16px;
    overflow-wrap: anywhere;
}
.mcp-docs-hero p {
    font-size: 18px;
    color: var(--text-mid);
    max-width: 640px;
    margin-bottom: 24px;
    overflow-wrap: anywhere;
}
.mcp-docs-layout {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: 48px;
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 32px 96px;
    align-items: start;
    width: 100%;
    box-sizing: border-box;
}
.mcp-toc {
    position: sticky;
    top: 96px;
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 16px;
    max-height: calc(100vh - 120px);
    overflow: auto;
}
.mcp-toc-label {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    letter-spacing: 2px;
    text-transform: uppercase;
    color: var(--accent-magenta);
    margin-bottom: 12px;
}
.mcp-filter {
    width: 100%;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    color: var(--text-bright);
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    padding: 8px 10px;
    border-radius: 6px;
    margin-bottom: 14px;
}
.mcp-filter:focus {
    outline: none;
    border-color: var(--accent-cyan);
}
.mcp-toc-group { margin-bottom: 14px; }
.mcp-toc-group-title {
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 6px;
}
.mcp-toc a {
    display: block;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--text-mid);
    padding: 4px 0;
}
.mcp-toc a:hover, .mcp-toc a:focus { color: var(--accent-cyan); }

.mcp-tool {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 28px;
    margin-bottom: 20px;
    scroll-margin-top: 96px;
    min-width: 0;
    max-width: 100%;
}
.mcp-tool-name {
    font-family: 'JetBrains Mono', monospace;
    font-size: 20px;
    color: var(--accent-cyan);
    margin-bottom: 8px;
    word-break: break-word;
}
.mcp-tool-desc {
    font-size: 15px;
    color: var(--text-mid);
    margin-bottom: 16px;
    line-height: 1.55;
    overflow-wrap: anywhere;
}
.mcp-tool-notes {
    font-size: 13px;
    color: var(--text-dim);
    margin: 0 0 20px;
    line-height: 1.5;
}
.mcp-subhead {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--accent-magenta);
    margin: 0 0 10px;
}
.mcp-args {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 20px;
    font-size: 13px;
}
.mcp-args th, .mcp-args td {
    text-align: left;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-dim);
    vertical-align: top;
}
.mcp-args th {
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    color: var(--text-dim);
    font-weight: 500;
}
.mcp-args code {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-cyan);
    font-size: 12px;
}
.mcp-req {
    font-family: 'JetBrains Mono', monospace;
    font-size: 10px;
    color: var(--accent-orange);
    margin-left: 6px;
    letter-spacing: 0.5px;
    text-transform: uppercase;
}
.mcp-empty {
    font-size: 13px;
    color: var(--text-dim);
    margin-bottom: 20px;
}
.mcp-code {
    position: relative;
    background: var(--bg-abyss);
    border: 1px solid var(--border-dim);
    border-radius: 8px;
    margin-bottom: 16px;
    max-width: 100%;
    overflow: hidden;
}
.mcp-code pre {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    line-height: 1.55;
    color: var(--text-mid);
    padding: 14px 16px;
    overflow-x: auto;
    max-width: 100%;
    white-space: pre;
}
.mcp-code-copy {
    position: absolute;
    top: 8px;
    right: 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border-dim);
    color: var(--text-dim);
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
}
.mcp-code-copy:hover { color: var(--accent-cyan); border-color: var(--accent-cyan); }
.mcp-code-copy.copied { color: var(--accent-green); border-color: var(--accent-green); }
.mcp-jk { color: var(--accent-cyan); }
.mcp-js { color: var(--accent-green); }
.mcp-jn { color: var(--accent-orange); }
.mcp-jb { color: var(--accent-magenta); }

.mcp-guide {
    background: var(--bg-deep);
    border: 1px solid var(--border-dim);
    border-radius: 12px;
    padding: 28px;
    margin-bottom: 20px;
    scroll-margin-top: 96px;
}
.mcp-guide h2 {
    font-size: 22px;
    margin-bottom: 12px;
}
.mcp-guide p, .mcp-guide li {
    font-size: 15px;
    color: var(--text-mid);
    line-height: 1.55;
    margin-bottom: 10px;
}
.mcp-guide ol, .mcp-guide ul { padding-left: 20px; margin-bottom: 16px; }
.mcp-guide code {
    font-family: 'JetBrains Mono', monospace;
    color: var(--accent-cyan);
    font-size: 13px;
}
.mcp-guide table { width: 100%; border-collapse: collapse; margin: 12px 0 20px; font-size: 14px; }
.mcp-guide th, .mcp-guide td {
    text-align: left;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-dim);
    vertical-align: top;
    color: var(--text-mid);
}
.mcp-guide th { color: var(--text-dim); font-family: 'JetBrains Mono', monospace; font-size: 11px; }

.mcp-hidden { display: none !important; }

@media (max-width: 1100px) {
    .nav-link { padding: 8px 10px; font-size: 12px; }
}

@media (max-width: 1024px) {
    .mcp-intro {
        grid-template-columns: 1fr;
        gap: 36px;
    }
    .mcp-intro-copy { text-align: center; }
    .mcp-intro-copy .section-subtitle { margin-left: auto; margin-right: auto; }
    .mcp-intro-copy .section-title {
        overflow-wrap: anywhere;
        white-space: normal;
        font-size: 28px;
        max-width: 100%;
    }
    .mcp-connect { text-align: left; }
    .mcp-intro-actions { align-items: center; }
    .mcp-intro-note { margin-left: auto; margin-right: auto; text-align: center; }
    .mcp-groups { grid-template-columns: 1fr; }
    .mcp-docs-layout {
        grid-template-columns: 1fr;
        gap: 24px;
        padding: 0 16px 80px;
    }
    .mcp-toc {
        position: static;
        max-height: none;
    }
    .mcp-toc-links {
        display: flex;
        flex-wrap: wrap;
        gap: 8px 16px;
    }
    .mcp-toc-group { margin-bottom: 8px; }
}

@media (max-width: 768px) {
    .mcp-docs-hero { padding: 112px 16px 32px; }
    .mcp-docs-hero h1 { font-size: 28px; white-space: normal; max-width: 100%; }
    .mcp-docs-hero p { font-size: 15px; max-width: 100%; word-break: break-word; }
    .mcp-tool { padding: 18px; }
    .mcp-connect-row {
        display: grid;
        grid-template-columns: 1fr auto;
        grid-template-areas: "label copy" "value value";
    }
    .mcp-connect-label { grid-area: label; border-right: none; border-bottom: 1px solid var(--border-dim); }
    .mcp-connect-copy { grid-area: copy; border-left: none; border-bottom: 1px solid var(--border-dim); }
    .mcp-connect-value {
        grid-area: value;
        font-size: 11px;
        white-space: normal;
        overflow-wrap: anywhere;
        word-break: break-word;
        width: 100%;
    }
    .mcp-toc-links { display: block; }
    .mcp-demo-json { font-size: 10px; white-space: pre-wrap; overflow-wrap: anywhere; }
    .mcp-args { display: block; }
    .mcp-args thead { display: none; }
    .mcp-args tr {
        display: block;
        padding: 10px 0;
        border-bottom: 1px solid var(--border-dim);
    }
    .mcp-args td {
        display: block;
        padding: 2px 0;
        border: none;
    }
}
`;
