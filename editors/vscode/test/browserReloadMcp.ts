import { strict as assert } from 'node:assert';
import { setTimeout as delay } from 'node:timers/promises';
import * as vscode from 'vscode';
import type { ReloadRecord } from '../src/reloadHistory';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode, RuntimeViewSnapshot } from '../src/runtimeProtocol';
import { runtimeApp } from './browserMcp';
import { browserBuildTimeout, waitFor } from './browserTestSupport';
import { openEditorMcp } from './mcpFixture';

async function save(uri: vscode.Uri, text: string): Promise<void> {
    const document = await vscode.workspace.openTextDocument(uri);
    const edit = new vscode.WorkspaceEdit();
    edit.replace(document.uri, new vscode.Range(0, 0, document.lineCount, 0), text);
    assert.ok(await vscode.workspace.applyEdit(edit));
    assert.ok(await document.save());
}

export async function verifyBrowserReload(): Promise<void> {
    const buildTimeout = browserBuildTimeout();
    const initialTimeout = Math.max(360_000, buildTimeout);
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const request = { profile: 'dispatch-check', folder: folder.toString() };
    const [rust, xaml, host] = ['main.rs', 'Instructions.xaml', 'wwwroot/main.js'].map(file => vscode.Uri.joinPath(folder, file));
    const [originalRust, originalXaml, originalHost] = await Promise.all([rust, xaml, host]
        .map(async uri => (await vscode.workspace.openTextDocument(uri)).getText()));
    await vscode.window.showTextDocument(rust, { preview: false });
    const mcp = await openEditorMcp(38_000);
    const { invoke, content } = mcp;
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = config.inspect<boolean>('allowWrites')?.globalValue;
    const writes = (value: boolean | undefined) => config.update('allowWrites', value, vscode.ConfigurationTarget.Global);
    let runtime: string | undefined;
    try {
        await writes(false);
        assert.match((await invoke('ferroui_browser_run', request)).content[0]?.text ?? '', /allowWrites/);
        await writes(true);
        await waitFor(async () => vscode.workspace.getConfiguration('ferroui.mcp').get('allowWrites', false) ? true : undefined, 'the editor write permission');
        const queued = content<RuntimeSummary>(await invoke('ferroui_browser_run', request));
        runtime = queued.runtime;
        assert.match(queued.runtime, /^browser:/);
        assert.equal(queued.available, false);
        assert.equal(queued.generation, null);
        assert.equal(queued.browser?.development, true);
        assert.equal(content<RuntimeSummary>(await invoke('ferroui_browser_run', request)).runtime, runtime);
        content(await invoke('ferroui_runtime_open', { runtime }));
        const current = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes
            .find(item => item.runtime === runtime);
        const output = async () => content<{ output: string }>(await invoke('ferroui_browser_output', { runtime, lines: 150 })).output;
        const status = (label: string, timeout?: number) => waitFor(async () => {
            const session = await current();
            if (session?.status === label) return session;
        }, label, timeout);
        const ready = (older: RuntimeSummary | undefined, timeout = buildTimeout) => waitFor(async () => {
            const session = await current();
            if (session?.available && session.generation !== older?.generation) return session;
            if (!older && session?.status === 'Browser launch failed') throw new Error(await output());
        }, 'a new browser generation', timeout).catch(async error => {
            throw new Error(`${error.message}\n${await output()}`, { cause: error });
        });
        const history = async () => content<{ records: ReloadRecord[] }>(await invoke('ferroui_reload_history', { app_session: runtime })).records;
        const windows = async (app: ReturnType<typeof runtimeApp>) => (await app.result<RuntimeNode[]>('list_windows'))[0];
        const frame = async (app: ReturnType<typeof runtimeApp>) => app.frame((await windows(app)).node_id);

        const initial = await ready(undefined, initialTimeout);
        assert.ok(initial.browser?.url && initial.pid);
        const view = () => invoke('ferroui_live_app_view', { runtime });
        await waitFor(async () => {
            const reply = await view();
            return !reply.isError && content<RuntimeViewSnapshot>(reply).frame ? true : undefined;
        }, 'the pinned Live App frame');
        const live = runtimeApp(mcp, initial);
        assert.equal((await live.find('Heading')).text, 'Dispatch desk');
        assert.equal((await live.find('DispatchInstructions')).text, 'Confirm the orders ready for pickup');
        await live.click((await live.find('DispatchOrder')).node_id);
        assert.equal((await live.find('DispatchedCount')).text, 'Dispatched orders: 1');
        const accepted = await frame(live);

        const notes = vscode.Uri.joinPath(folder, 'notes/src/lib.rs');
        await save(notes, 'pub const STATUS: &str = "Updated notes";\n');
        await delay(700);
        assert.equal((await current())?.status, 'Watching browser saves');
        assert.ok(!(await history()).some(record => record.file.endsWith('notes/src/lib.rs')));

        await save(xaml, '<TextBlock');
        await status('Starting browser candidate', buildTimeout);
        assert.deepEqual(await frame(live), accepted, 'Startup retains the accepted frame');
        await live.click((await live.find('DispatchOrder')).node_id);
        await waitFor(async () => (await frame(live)).equals(accepted) ? undefined : true,
            'a new frame after input on the accepted page', 5_000);
        const startup = await status('Browser update failed');
        assert.equal(startup.generation, initial.generation);
        assert.equal((await live.find('DispatchedCount')).text, 'Dispatched orders: 2');
        assert.ok((await history()).some(record => record.file.endsWith('Instructions.xaml') && record.outcome === 'buildFailed'));

        await save(xaml, originalXaml.replace('Confirm the orders ready for pickup', 'Review the outgoing routes'));
        const xamlAccepted = await ready(initial);
        const xamlApp = runtimeApp(mcp, xamlAccepted);
        assert.equal(xamlAccepted.runtime, runtime);
        assert.equal(xamlAccepted.pid, initial.pid);
        assert.equal(new URL(xamlAccepted.browser!.url!).origin, new URL(initial.browser.url).origin);
        assert.equal((await xamlApp.find('DispatchInstructions')).text, 'Review the outgoing routes');
        assert.equal((await xamlApp.find('DispatchedCount')).text, 'Dispatched orders: 2');
        assert.notDeepEqual(await frame(xamlApp), accepted);
        await waitFor(async () => (await fetch(initial.browser!.url!)).status === 404 ? true : undefined, 'retirement of the previous site');
        assert.ok((await history()).some(record => record.file.endsWith('Instructions.xaml')
            && record.outcome === 'rebuilt' && record.frame?.status === 'rendered'));
        assert.match((await invoke('ferroui_runtime_query', { runtime, expected_generation: initial.generation, tool: 'list_windows' }))
            .content[0]?.text ?? '', /connection changed|generation/i);

        await save(rust, originalRust + '\nfn invalid_browser_save( {\n');
        const compile = await status('Browser update failed', buildTimeout);
        assert.equal(compile.generation, xamlAccepted.generation);
        assert.match(await output(), /error:|mismatched closing delimiter|unclosed delimiter/);
        assert.equal((await xamlApp.find('DispatchedCount')).text, 'Dispatched orders: 2');

        await Promise.all([
            save(host, 'await new Promise(resolve => setTimeout(resolve, 3500));\n' + originalHost),
            save(rust, originalRust.replace('text: "Dispatch desk"', 'text: "An intermediate page"')),
        ]);
        await status('Starting browser candidate', buildTimeout);
        assert.equal((await current())?.generation, xamlAccepted.generation);
        await Promise.all([
            save(host, originalHost),
            save(rust, originalRust.replace('text: "Dispatch desk"', 'text: "Routes ready"')),
        ]);
        const rustAccepted = await ready(xamlAccepted);
        const rustApp = runtimeApp(mcp, rustAccepted);
        assert.equal(rustAccepted.runtime, runtime);
        assert.equal(rustAccepted.pid, initial.pid);
        assert.equal((await rustApp.find('Heading')).text, 'Routes ready');
        assert.equal((await rustApp.find('DispatchedCount')).text, 'Dispatched orders: 2');
        assert.ok((await history()).some(record => record.file.endsWith('main.rs') && record.outcome === 'superseded'));
        const heading = await rustApp.find('Heading');
        const source = content<{ source?: { uri: string; canApply: boolean; version: number } }>(await invoke('ferroui_runtime_source',
            { runtime, node: heading.node_id }));
        assert.equal(source.source?.uri, rust.toString());
        assert.equal(source.source?.canApply, true);
        content(await invoke('ferroui_runtime_apply_source', {
            runtime, node: heading.node_id, property: 'Text', value: 'Source draft',
            expected_generation: rustAccepted.generation, expected_version: source.source?.version,
        }));
        const edited = (await vscode.window.showTextDocument(rust, { preview: false })).document;
        assert.match(edited.getText(), /text: "Source draft"/);
        await vscode.commands.executeCommand('undo');
        await waitFor(async () => edited.getText().includes('text: "Routes ready"') && !edited.isDirty ? true : undefined, 'the grouped source Undo');
        assert.equal((await current())?.generation, rustAccepted.generation);

        await vscode.commands.executeCommand('ferroui.appSessions.rebuild', runtime);
        const manual = await ready(rustAccepted);
        assert.equal(manual.pid, initial.pid);
        assert.equal((await runtimeApp(mcp, manual).find('Heading')).text, 'Routes ready');
        assert.match(content<RuntimeSummary>(await invoke('ferroui_browser_rebuild', { runtime })).status, /queued|building/i);
        content(await invoke('ferroui_browser_stop', { runtime }));
        assert.equal(await current(), undefined);
        await assert.rejects(fetch(manual.browser!.url!, { signal: AbortSignal.timeout(1_500) }));
        runtime = undefined;
        console.log('Managed Chromium XAML and Rust saves, failed build and startup retention, superseding, persistent state, manual rebuild, source, frame receipts and cleanup passed');
    } finally {
        try { if (runtime) await invoke('ferroui_browser_stop', { runtime }); }
        finally { await writes(previous); mcp.close(); }
    }
}
