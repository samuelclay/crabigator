export function stripAnsi(text: string): string {
    return text
        .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g, '')
        .replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, '')
        .replace(/\x1b[@-Z\\-_]/g, '')
        .replace(/\r/g, '');
}

export function formatScreen(content: string | null, format: 'text' | 'ansi'): string | null {
    if (content == null) return null;
    return format === 'ansi' ? content : stripAnsi(content);
}

export function tailLines(content: string, tail?: number): string {
    if (!tail || tail <= 0) return content;
    const lines = content.split('\n');
    return lines.slice(-tail).join('\n');
}

export function toolText(data: unknown): { content: Array<{ type: 'text'; text: string }> } {
    const text = typeof data === 'string' ? data : JSON.stringify(data, null, 2);
    return { content: [{ type: 'text', text }] };
}

export function toolError(message: string): {
    content: Array<{ type: 'text'; text: string }>;
    isError: true;
} {
    return { content: [{ type: 'text', text: message }], isError: true };
}
