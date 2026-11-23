import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import * as vscode from 'vscode';

interface ToolResponse {
    content: { type: string; text?: string; data?: string }[];
    isError: boolean;
}

export interface McpFixture {
    invoke(name: string, args?: object): Promise<ToolResponse>;
    content<T>(result: ToolResponse): T;
}

export async function openEditorMcp(timeout = 15_000) {
    const definitions = await vscode.commands.executeCommand<vscode.McpServerDefinition[]>('ferroui.test.mcpDefinitions');
    const server = definitions.find(definition => definition instanceof vscode.McpStdioServerDefinition) as vscode.McpStdioServerDefinition | undefined;
    assert.ok(server, 'VS Code receives the bundled FerroUI MCP definition');
    const child = spawn(server.command, server.args, { stdio: 'pipe', windowsHide: true,
        env: { ...process.env, ...Object.fromEntries(Object.entries(server.env).filter(([, value]) => value !== null).map(([key, value]) => [key, String(value)])) } });
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
        }, timeout);
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
    const close = () => {
        lines.close();
        child.stdin.end();
        child.kill();
    };
    try {
        const hello = await request<{ protocolVersion: string }>('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'vscode-test', version: '1' } });
        assert.equal(hello.protocolVersion, '2025-11-25');
        child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
        return { request, invoke, content, close };
    } catch (error) {
        close();
        throw error;
    }
}
