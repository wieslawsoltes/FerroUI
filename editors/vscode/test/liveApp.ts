import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import * as vscode from 'vscode';
import { revision } from '../src/reloadProcess';
import { RuntimeClient } from '../src/runtimeClient';
import { mcpEndpointPrefix, parseMcpEndpoint, type RuntimeMcpEndpoint } from '../src/runtimeMcp';
import type { RuntimeNode, RuntimeSource } from '../src/runtimeProtocol';
import { sourcePosition } from '../src/runtimeSource';

export async function verifyLiveApp(): Promise<void> {
    const app = process.env.FERROUI_TEST_HOSTED_APP;
    if (!app) return;
    const child = spawn(app, ['--demo', '--mcp', '--mcp-input'], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true,
        env: { ...process.env, FERROUI_EDITOR_RELOAD: '1' } });
    let client: RuntimeClient | undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const lines = createInterface({ input: child.stderr });
    try {
        const endpoint = await new Promise<RuntimeMcpEndpoint>((resolve, reject) => {
            timeout = setTimeout(() => reject(new Error('The native Todo app did not publish its inspector endpoint')), 10_000);
            lines.on('line', text => {
                if (!text.startsWith(mcpEndpointPrefix)) return;
                const endpoint = parseMcpEndpoint(text.slice(mcpEndpointPrefix.length));
                if (endpoint) resolve(endpoint);
            });
            child.once('error', reject);
            child.once('exit', code => reject(new Error(`The native Todo app exited during startup (${code})`)));
        });
        client = new RuntimeClient(endpoint);
        assert.ok(await client.supports('get_element_at'));
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { name: 'NewTask' })).matched);
        const [window] = await client.call<RuntimeNode[]>('list_windows');
        const [entry] = await client.call<RuntimeNode[]>('find_elements', { name: 'NewTask' });
        const [add] = await client.call<RuntimeNode[]>('find_elements', { name: 'AddTask' });
        const png = await client.call<string>('screenshot', { node_id: window.node_id });
        assert.ok(entry && add && png.length);
        const source = await client.call<RuntimeSource>('get_source', { node_id: entry.node_id });
        assert.equal(source?.language, 'rust');
        assert.equal(source.columnEncoding, 'unicodeScalar');
        assert.ok(source.invocation && source.invocation.line < source.line);
        const rust = await readFile(new URL(source.uri), 'utf8');
        assert.equal(source.revision, revision(Buffer.from(rust)));
        const line = rust.split(/\r?\n/)[source.line - 1];
        assert.match(Array.from(line).slice(source.column - 1).join(''), /^TextBox\s*\{\s*name: "NewTask"/);
        const unicode = await vscode.workspace.openTextDocument({ language: 'rust', content: 'rsx! { "👋" TextBox {} }\r\n' });
        const position = sourcePosition(unicode, { ...source, line: 1, column: 12 });
        assert.equal(position?.character, unicode.lineAt(0).text.indexOf('TextBox'));
        assert.equal(sourcePosition(unicode, { ...source, line: 1, column: 999 }), undefined);
        await client.call('type_text', { node_id: entry.node_id, text: 'QA task from VS Code' });
        await client.call('click', { node_id: add.node_id });
        assert.ok((await client.call<{ matched: boolean }>('wait_for', { text: 'QA task from VS Code' })).matched);
        const updated = await client.call<string>('screenshot', { node_id: window.node_id });
        assert.notEqual(updated, png);
        console.log('Live App found the authored RSX control, entered text, clicked a bound action and captured the updated native window');
    } finally {
        clearTimeout(timeout);
        client?.dispose();
        lines.close();
        child.kill();
    }
}
