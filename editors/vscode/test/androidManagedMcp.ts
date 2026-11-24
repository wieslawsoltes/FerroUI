import { strict as assert } from 'node:assert';
import { readFileSync, writeFileSync, unlinkSync, readdirSync } from 'node:fs';
import { createServer } from 'node:http';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { openEditorMcp } from './mcpFixture';
import { waitFor } from './browserTestSupport';

const stage = () => process.env.FERROUI_MANAGED_TEST_DIR!;
const deviceFile = () => join(stage(), 'device.json');
const device = () => JSON.parse(readFileSync(deviceFile(), 'utf8'));
const root = { node_id: 1, name: 'FerroUI Android fixture', type: 'Window', bounds: { x: 0, y: 0, width: 128, height: 96 } };
const image = 'iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAE0lEQVR4nGNMjb3OAANMcBZeDgBQdAGhOsFZtwAAAABJRU5ErkJggg==';

export async function verifyAndroidDevelopment() {
    const server = createServer(async (request, response) => {
        const receipt = device().receipt;
        if (request.headers.authorization !== 'Bearer ' + receipt?.token || request.headers.host !== '127.0.0.1:' + receipt.port) {
            response.writeHead(403).end();
            return;
        }
        if (request.method === 'DELETE') { response.writeHead(204).end(); return; }
        let body = '';
        for await (const chunk of request) body += chunk;
        const { method, id, params } = JSON.parse(body);
        let result;
        if (method === 'initialize') result = { protocolVersion: '2025-11-25', capabilities: { tools: {} }, serverInfo: { name: 'test', version: '1' } };
        else if (method === 'notifications/initialized') { response.writeHead(202).end(); return; }
        else if (method === 'tools/list') result = { tools: ['list_windows', 'get_tree', 'screenshot'].map(name => ({ name, inputSchema: { type: 'object' }, annotations: { readOnlyHint: true } })) };
        else if (method === 'tools/call' && params.name === 'screenshot') result = { content: [{ type: 'image', mimeType: 'image/png', data: image }] };
        else result = { content: [{ type: 'text', text: JSON.stringify([root]) }] };
        response.writeHead(200, { 'Content-Type': 'application/json', 'Mcp-Session-Id': 'test-session' }).end(JSON.stringify({ jsonrpc: '2.0', id, result }));
    });
    await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
    const address = server.address();
    assert.ok(address && typeof address === 'object');
    writeFileSync(deviceFile(), JSON.stringify({ pid: null, receipt: null, next: 4000, hostPort: address.port,
        installed: null, installations: [], forwards: { 38099: 'tcp:5999' } }));
    const mcp = await openEditorMcp(25_000);
    const { invoke, content } = mcp;
    let runtime: string | undefined;
    const folder = vscode.workspace.workspaceFolders![0];
    const current = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes.find(item => item.runtime === runtime);
    const status = (label: string) => waitFor(async () => {
        const session = await current();
        if (session?.status === label) return session;
    }, label, 15_000);
    const accepted = (older?: number | null) => waitFor(async () => {
        const session = await current();
        if (session?.available && session.generation !== older) return session;
        if (session?.status === 'Android build failed' || session?.status === 'Android deployment failed') {
            throw Error(content<{ output: string }>(await invoke('ferroui_android_output', { runtime })).output);
        }
    }, 'an accepted Android generation', 15_000);
    const save = async (file: string, next: string) => {
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder.uri, file));
        const edit = new vscode.WorkspaceEdit();
        edit.replace(doc.uri, new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)), next);
        assert.ok(await vscode.workspace.applyEdit(edit));
        assert.ok(await doc.save());
    };
    const rust = 'app/lib.rs';
    try {
        const request = { device: 'emulator-5450', profile: 'managed', folder: folder.uri.toString() };
        const mcpConfig = vscode.workspace.getConfiguration('ferroui.mcp');
        assert.ok((await invoke('ferroui_android_run', request)).isError, 'managed launches require agent write permission');
        await mcpConfig.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        const starting = content<RuntimeSummary>(await invoke('ferroui_android_run', request));
        runtime = starting.runtime;
        assert.equal(starting.available, false);
        assert.equal(starting.android?.development, true);
        const first = await accepted();
        assert.equal(first.pid, 4001);
        assert.equal(device().installations.length, 1);
        await save('notes/lib.rs', '// an unrelated Cargo package\n');
        await delay(650);
        assert.equal((await current())?.pid, first.pid);
        assert.equal(device().installations.length, 1);
        await save(rust, '// fail-build\n');
        await status('Android build failed');
        assert.equal((await current())?.generation, first.generation);
        assert.equal(device().installations.length, 1);
        assert.match(content<{ output: string }>(await invoke('ferroui_android_output', { runtime })).output, /controlled compiler error/);

        writeFileSync(join(stage(), 'hold'), '');
        await save(rust, '// first candidate\n');
        await status('Building Android update');
        assert.equal((await current())?.generation, first.generation);
        await save(rust, '// newest candidate\n');
        unlinkSync(join(stage(), 'hold'));
        const second = await accepted(first.generation);
        assert.equal(second.pid, 4002);
        assert.equal(device().installations.length, 2);
        assert.equal(runtime, second.runtime);
        assert.ok((await invoke('ferroui_runtime_query', { runtime, expected_generation: first.generation, tool: 'list_windows' })).isError);

        await save('app/View.xaml', '<TextBlock Text="Saved in XAML"/>\n');
        const third = await accepted(second.generation);
        const history = content<{ records: { file: string; outcome: string; frame?: { status: string } }[] }>(
            await invoke('ferroui_reload_history', { app_session: runtime })).records;
        assert.ok(history.some(record => record.file.endsWith('View.xaml') && record.outcome === 'rebuilt' && record.frame?.status === 'rendered'));
        await save(rust, '// bad-startup\n');
        const restored = await status('Android deployment failed · restored previous build');
        assert.ok(restored.available && restored.generation !== third.generation);
        assert.deepEqual(device().installations.slice(-2), ['bad', 'good']);
        assert.equal(restored.runtime, runtime);

        await save(rust, '// repaired source\n');
        const recovered = await accepted(restored.generation);
        await mcpConfig.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        assert.ok((await invoke('ferroui_android_rebuild', { runtime })).isError, 'redeployment requires agent write permission');
        assert.match(content<{ output: string }>(await invoke('ferroui_android_output', { runtime })).output, /staged Android package/);
        assert.equal((await current())?.generation, recovered.generation);
        await mcpConfig.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        content(await invoke('ferroui_android_rebuild', { runtime }));
        const rebuilt = await accepted(recovered.generation);
        assert.ok(rebuilt.available);
        const installations = device().installations.length;
        writeFileSync(join(stage(), 'hold'), '');
        await save(rust, '// stop while building\n');
        await status('Building Android update');
        assert.equal((await current())?.generation, rebuilt.generation);
        content(await invoke('ferroui_android_stop', { runtime }));
        runtime = undefined;
        assert.equal(device().pid, null);
        assert.equal(device().installations.length, installations);
        assert.deepEqual(device().forwards, { 38099: 'tcp:5999' });
        assert.ok(!readdirSync(join(folder.uri.fsPath, 'target')).some(name => name.startsWith('.ferroui-android-')));
        console.log('VS Code Android managed lifecycle passed scoped saves, supersession, compile retention, rollback, generation checks, write permissions and cancellation cleanup.');
    } finally {
        if (runtime) await invoke('ferroui_android_stop', { runtime }).catch(() => {});
        mcp.close();
        server.closeAllConnections();
        await new Promise<void>(resolve => server.close(() => resolve()));
    }
}
