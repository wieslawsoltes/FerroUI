import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import * as vscode from 'vscode';

interface ToolResponse {
    content: { type: string; text?: string; data?: string }[];
    isError: boolean;
}

export async function verifyMcp(document: vscode.TextDocument): Promise<void> {
    const definitions = await vscode.commands.executeCommand<vscode.McpServerDefinition[]>('ferroui.test.mcpDefinitions');
    const server = definitions.find(definition => definition instanceof vscode.McpStdioServerDefinition) as vscode.McpStdioServerDefinition | undefined;
    assert.ok(server, 'VS Code receives the bundled FerroUI MCP definition');
    const child = spawn(server.command, server.args, { stdio: 'pipe', windowsHide: true,
        env: { ...process.env, ...Object.fromEntries(Object.entries(server.env).filter(([,value]) => value !== null).map(([key, value]) => [key, String(value)])) } });
    let output = '';
    child.stderr.on('data', chunk => { output += chunk.toString(); });
    let id = 0;
    const readers = new Map<number, (value: unknown) => void>();
    const lines = createInterface({ input: child.stdout });
    lines.on('line', line => {
        const message = JSON.parse(line);
        readers.get(message.id)?.(message);
    });
    child.on('error', error => { output = error.message; });
    const request = <T>(method: string, params = {}) => new Promise<T>((resolve, reject) => {
        const key = ++id;
        const timer = setTimeout(() => {
            readers.delete(key);
            reject(new Error(`MCP ${method} timed out. ${output}`));
        }, 15_000);
        readers.set(key, raw => {
            clearTimeout(timer);
            readers.delete(key);
            const message = raw as { error?: { message: string }; result?: T };
            if (message.error) reject(new Error(message.error.message));
            else resolve(message.result!);
        });
        child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: key, method, params }) + '\n');
    });
    const invoke = (name: string, args = {}) => request<ToolResponse>('tools/call', { name, arguments: args });
    const content = <T>(result: ToolResponse): T => {
        assert.equal(result.isError, false, result.content[0]?.text);
        return JSON.parse(result.content.find(item => item.type === 'text')!.text!);
    };
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const original = document.getText();
    try {
        const hello = await request<{ protocolVersion: string }>('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'vscode-test', version: '1' } });
        assert.equal(hello.protocolVersion, '2025-11-25');
        child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
        const { tools } = await request<{ tools: { name: string }[] }>('tools/list');
        assert.ok(tools.some(tool => tool.name === 'designer_place'));
        const { designs } = content<{ designs: { uri: string; session: number }[] }>(await invoke('ferroui_designs'));
        const session = designs.find(design => design.uri === document.uri.toString())!.session;
        const { revision, nodes } = content<{ revision: number; nodes: { name: string | null; id: number }[] }>(await invoke('designer_outline', { session }));
        const button = nodes.find(node => node.name === 'saveButton');
        assert.ok(button);
        const studio = content<{ tools: { name: string }[] }>(await invoke('designer_studio_tools', { session }));
        assert.ok(studio.tools.some(tool => tool.name === 'designer_ast'));
        assert.ok(studio.tools.some(tool => tool.name === 'designer_events'));
        content(await invoke('designer_studio_query', { session, tool: 'designer_ast', arguments: { depth: 2 } }));
        const combined = content<{ nodes: { visual: boolean; logical: boolean }[] }>(
            await invoke('designer_studio_query', { session, tool: 'designer_tree', arguments: { tree: 'combined' } }));
        assert.ok(combined.nodes.some(row => row.visual && row.logical));
        const inspector = content<{ properties: { name: string; group: string }[]; events: { name: string }[] }>(
            await invoke('designer_inspect', { session, node: button.id }));
        assert.ok(inspector.properties.some(field => field.name === 'Width' && field.group === 'Layout'));
        assert.ok(inspector.events.some(field => field.name === 'Click'));
        const assets = content<{ assets: { id: string }[] }>(await invoke('designer_assets', { session }));
        assert.ok(assets.assets.some(asset => asset.id === 'Assets/Preview Image.png'));
        const image = await invoke('designer_preview_image', { session });
        assert.equal(image.isError, false);
        assert.equal(Buffer.from(image.content.find(item => item.type === 'image')!.data!, 'base64').subarray(1, 4).toString(), 'PNG');
        const edit = { session, expected_revision: revision, label: 'Test agent edit', edits: [{ SetAttribute: { node: button.id, name: 'Content', value: 'MCP edited this buffer' } }] };
        const denied = await invoke('designer_apply', edit);
        assert.ok(denied.isError && denied.content[0]?.text?.includes('allowWrites'));
        await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        content(await invoke('designer_apply', edit));
        assert.ok(document.isDirty && document.getText().includes('Content="MCP edited this buffer"'));
        assert.equal((await invoke('designer_apply', edit)).isError, true);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        const latest = content<{ revision: number }>(await invoke('designer_inspect', { session, node: button.id }));
        content(await invoke('designer_set_event', { session, node: button.id, expected_revision: latest.revision,
            event: 'Click', action: 'set', handler: 'OnSave' }));
        assert.match(document.getText(), /Click="OnSave"/);
        content(await invoke('designer_history', { session, expected_version: document.version, action: 'undo' }));
        assert.equal(document.getText(), original);
        console.log('VS Code MCP discovery, shared Studio queries, PNG content, write permissions, stale revisions and editor undo passed');
    } finally {
        await config.update('allowWrites', undefined, vscode.ConfigurationTarget.Global);
        lines.close();
        child.stdin.end();
        child.kill();
    }
}
