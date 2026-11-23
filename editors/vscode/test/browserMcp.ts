import { strict as assert } from 'node:assert';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { browserSocket } from '../src/browserCdpTransport';
import { browserExecutable } from '../src/browserExecutable';
import { OwnedProcess } from '../src/ownedProcess';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode, RuntimeSource } from '../src/runtimeProtocol';
import { openEditorMcp, type McpFixture } from './mcpFixture';
import { until } from './wait';

const profile = 'dispatch-desk';
const label = 'Dispatch Desk';

export async function verifyBrowserMcp(url: string): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const source = vscode.Uri.joinPath(folder, 'src/Browser/FerroUI.Browser/examples/live_inspector/main.rs');
    const document = await vscode.workspace.openTextDocument(source);
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('ferroui.showAppSessions');
    const mcp = await openEditorMcp(40_000);
    const { invoke, content } = mcp;
    const request = { profile, folder: folder.toString() };
    const settings = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = settings.inspect<boolean>('allowWrites')?.globalValue;
    const writes = (value: boolean | undefined) => settings.update('allowWrites', value, vscode.ConfigurationTarget.Global);
    const sessions = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes;
    try {
        const targets = content<{ apps: { id: string; url: string }[] }>(await invoke('ferroui_browser_targets', { folder: request.folder }));
        assert.ok(targets.apps.some(app => app.id === profile && app.url === url));
        await writes(false);
        assert.match((await invoke('ferroui_browser_run', request)).content[0]?.text ?? '', /allowWrites/);
        await writes(true);
        const started = content<RuntimeSummary>(await invoke('ferroui_browser_run', request));
        assert.match(started.runtime, /^browser:/);
        assert.equal(started.browser?.launched, true);
        assert.equal(content<RuntimeSummary>(await invoke('ferroui_browser_run', request)).runtime, started.runtime);
        const app = runtimeApp(mcp, started);
        await checkBrowserApp(app, mcp, started, document);
        assert.match((await invoke('ferroui_runtime_detach', { runtime: started.runtime })).content[0]?.text ?? '', /browser session/);
        await vscode.commands.executeCommand('ferroui.appSessions.stop', started.runtime);
        assert.ok(!(await sessions()).some(session => session.runtime === started.runtime));
        await checkAttach(url, request, mcp, sessions);
        console.log('VS Code Browser Live App managed launch, Chromium PNG and input, checked Rust edits and Undo, independent attach and stale navigation passed');
    } finally {
        try {
            for (const session of await sessions()) {
                if (session.browser?.profile === profile) await invoke('ferroui_browser_stop', { runtime: session.runtime });
            }
        } finally {
            await writes(previous);
            mcp.close();
        }
    }
}

function runtimeApp({ invoke, content }: McpFixture, { runtime, generation }: RuntimeSummary) {
    assert.ok(generation);
    const query = (tool: string, args: object = {}) => invoke('ferroui_runtime_query', { runtime, expected_generation: generation, tool, arguments: args });
    const result = async <T>(tool: string, args?: object) => content<{ result: T }>(await query(tool, args)).result;
    const find = async (name: string) => (await result<RuntimeNode[]>('find_elements', { name }))[0];
    const click = async (node_id: number) => content(await invoke('ferroui_runtime_call', {
        runtime, expected_generation: generation, tool: 'click', arguments: { node_id },
    }));
    const frame = async (node_id: number) => {
        const reply = await query('screenshot', { node_id });
        assert.equal(content<{ generation: number }>(reply).generation, generation);
        const image = Buffer.from(reply.content.find(item => item.type === 'image')?.data ?? '', 'base64');
        assert.equal(image.subarray(1, 4).toString(), 'PNG');
        return image;
    };
    return { find, click, frame, result };
}

async function checkBrowserApp(app: ReturnType<typeof runtimeApp>, { invoke, content }: McpFixture,
    { runtime, generation, label }: RuntimeSummary, document: vscode.TextDocument) {
    const [window] = await app.result<RuntimeNode[]>('list_windows');
    assert.ok(window?.bounds);
    const [tree] = await app.result<RuntimeNode[]>('get_tree', { node_id: window.node_id, depth: 10 });
    assert.ok(tree.children?.length);
    const before = await app.frame(window.node_id);
    assert.deepEqual([before.readUInt32BE(16), before.readUInt32BE(20)], [960, 620]);
    const button = await app.find('DispatchOrder');
    assert.ok(button);
    assert.ok((await app.frame(button.node_id)).readUInt32BE(16) < before.readUInt32BE(16));
    const [automation] = await app.result<RuntimeNode[]>('get_tree', { node_id: window.node_id, tree: 'automation', depth: 10 });
    assert.ok(automation.children?.length);
    const denied = await invoke('ferroui_runtime_call', { runtime, expected_generation: generation, tool: 'set_property',
        arguments: { node_id: button.node_id, property: 'Content', value: 'New action' } });
    assert.ok(denied.isError);
    assert.match(denied.content[0]?.text ?? '', /writes|permission/i);
    const heading = await app.find('Heading');
    assert.equal(heading.text, 'Dispatch desk');
    const compiler = await app.result<RuntimeSource>('get_source', { node_id: heading.node_id });
    assert.ok(compiler.workspacePath?.endsWith('examples/live_inspector/main.rs'));
    const { source } = content<{ source: { uri: string; version: number; position: { line: number }; canApply: boolean } }>(
        await invoke('ferroui_runtime_source', { runtime, node: heading.node_id }));
    assert.ok(source);
    assert.equal(source.uri, document.uri.toString());
    assert.ok(document.lineAt(source.position.line).text.includes('Dispatch desk'));
    assert.equal(source.canApply, true);
    const original = document.getText();
    const edit = { runtime, node: heading.node_id, property: 'Text', value: 'Routing desk', expected_generation: generation, expected_version: source.version };
    try {
        content(await invoke('ferroui_runtime_apply_source', edit));
        assert.ok(document.getText().includes('text: "Routing desk"'));
        const rejected = await invoke('ferroui_runtime_apply_source', edit);
        assert.ok(rejected.isError);
        assert.match(rejected.content[0]?.text ?? '', /source|version|changed/i);
    } finally {
        if (document.getText() !== original) {
            await vscode.window.showTextDocument(document);
            await vscode.commands.executeCommand('undo');
            await until(() => document.getText() === original);
        }
    }
    content(await invoke('ferroui_runtime_open', { runtime }));
    await until(() => vscode.window.tabGroups.all.some(group => group.tabs.some(tab => tab.label === `Live App · ${label}`)));
    await app.click(button.node_id);
    assert.ok((await app.result<{ matched: boolean }>('wait_for', { name: 'DispatchedCount', text: 'Dispatched orders: 1' })).matched);
    assert.notDeepEqual(await app.frame(window.node_id), before);
}

async function checkAttach(url: string, request: { profile: string; folder: string }, { invoke, content }: McpFixture,
    sessions: () => Promise<RuntimeSummary[]>) {
    const executable = await browserExecutable();
    const directory = await mkdtemp(path.join(tmpdir(), 'ferroui-browser-test-'));
    const child = new OwnedProcess(spawn(executable, ['--headless', '--no-first-run', `--user-data-dir=${directory}`,
        '--no-default-browser-check', '--remote-debugging-port=0', '--remote-debugging-address=127.0.0.1', url],
    { detached: true, windowsHide: true, stdio: 'ignore' }), () => rm(directory, { recursive: true, force: true, maxRetries: 3 }));
    void child.closed.catch(() => {});
    const settings = vscode.workspace.getConfiguration('ferroui.browser');
    const previous = settings.inspect('apps')?.globalValue;
    try {
        const portRecord = path.join(directory, 'DevToolsActivePort');
        await until(async () => Boolean((await readFile(portRecord, 'utf8').catch(() => '')).split('\n')[1]));
        const debugPort = Number((await readFile(portRecord, 'utf8')).split('\n')[0]);
        assert.ok(debugPort > 0);
        await settings.update('apps', [{ id: profile, label, url, debugPort }], vscode.ConfigurationTarget.Global);
        const attached = content<RuntimeSummary>(await invoke('ferroui_browser_attach', request));
        assert.equal(attached.browser?.launched, false);
        const app = runtimeApp({ invoke, content }, attached);
        await app.click((await app.find('DispatchOrder')).node_id);
        content(await invoke('ferroui_browser_stop', { runtime: attached.runtime }));
        assert.ok(!child.hasExited, 'Disconnect leaves the independently started Chromium running');

        await vscode.commands.executeCommand('ferroui.browser.attach', request);
        const reopened = (await sessions()).find(session => session.browser?.profile === profile);
        assert.ok(reopened && reopened.runtime !== attached.runtime && reopened.generation !== attached.generation);
        assert.equal((await runtimeApp({ invoke, content }, reopened).find('DispatchedCount')).text, 'Dispatched orders: 1');
        const list = await fetch(`http://127.0.0.1:${debugPort}/json/list`, { signal: AbortSignal.timeout(3_000) });
        const targets = await list.json() as { url: string; webSocketDebuggerUrl: string }[];
        const address = targets.find(target => target.url === url)?.webSocketDebuggerUrl;
        assert.ok(address);
        const debuggerClient = await browserSocket(address, AbortSignal.timeout(3_000));
        try { await debuggerClient.request('Page.reload', {}, AbortSignal.timeout(3_000)); }
        finally { debuggerClient.close(); }
        await until(async () => !(await sessions()).some(session => session.runtime === reopened.runtime));
        const stale = await invoke('ferroui_runtime_query', {
            runtime: reopened.runtime, expected_generation: reopened.generation, tool: 'list_windows',
        });
        assert.ok(stale.isError);
        assert.match(stale.content[0]?.text ?? '', /inspector is unavailable/);
        const reloaded = content<RuntimeSummary>(await invoke('ferroui_browser_attach', request));
        assert.notEqual(reloaded.generation, reopened.generation);
        assert.equal((await runtimeApp({ invoke, content }, reloaded).find('DispatchedCount')).text, 'Dispatched orders: 0');
        await child.stop();
        await until(async () => !(await sessions()).some(session => session.runtime === reloaded.runtime));
    } finally {
        await child.stop();
        await settings.update('apps', previous, vscode.ConfigurationTarget.Global);
    }
}
