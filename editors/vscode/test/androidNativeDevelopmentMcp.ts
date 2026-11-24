import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import type { RuntimeNode } from '../src/runtimeProtocol';
import { fileKey } from '../src/workspacePaths';
import { waitFor } from './browserTestSupport';
import { openEditorMcp } from './mcpFixture';
import { trackNativeDebugger } from './nativeDebugFixture';

export async function verifyNativeAndroidDevelopment(device: string): Promise<void> {
    const folder = vscode.workspace.workspaceFolders![0].uri;
    const rust = await vscode.workspace.openTextDocument(vscode.Uri.joinPath(folder, 'lib.rs'));
    const xaml = vscode.Uri.joinPath(folder, 'Message.xaml');
    const mark = rust.getText().indexOf('*count += 1');
    assert.ok(mark >= 0, 'Find the Service Review callback.');
    const point = new vscode.SourceBreakpoint(new vscode.Location(rust.uri, rust.positionAt(mark)));
    const name = 'Attach to Service Review';
    const tracker = trackNativeDebugger(name);
    const { content, invoke, close } = await openEditorMcp(40_000);
    let runtime: string | undefined;
    const current = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes
        .find(item => item.runtime === runtime);
    const output = async () => content<{ output: string }>(await invoke('ferroui_android_output', { runtime })).output;
    const accept = (prior?: number) => waitFor(async () => {
        const session = await current();
        if (session?.status === 'Android build failed' || session?.status === 'Android deployment failed') throw Error(await output());
        if (session?.available && session.generation && session.generation !== prior) return session;
    }, 'a source-built Android frame', 15 * 60_000);
    const identity = (session: RuntimeSummary) => ({ runtime, expected_generation: session.generation });
    const find = async (session: RuntimeSummary, match: { name: string } | { text: string }) =>
        content<{ result: RuntimeNode[] }>(await invoke('ferroui_runtime_query', {
            ...identity(session), tool: 'find_elements', arguments: match,
        })).result[0];
    const save = async (uri: vscode.Uri, before: string, after: string) => {
        const document = await vscode.workspace.openTextDocument(uri);
        const offset = document.getText().indexOf(before);
        assert.ok(offset >= 0);
        const edit = new vscode.WorkspaceEdit();
        edit.replace(document.uri, new vscode.Range(document.positionAt(offset), document.positionAt(offset + before.length)), after);
        assert.ok(await vscode.workspace.applyEdit(edit) && await document.save());
    };
    const callback = async (app: RuntimeSummary, debug: vscode.DebugSession) => {
        const button = await find(app, { name: 'ReviewCase' });
        assert.ok(button);
        const click = invoke('ferroui_runtime_call', { ...identity(app), tool: 'click', arguments: { node_id: button.node_id } })
            .then(response => { content(response); }).catch((error: unknown) => error);
        const threadId = await tracker.breakpoint(debug);
        try {
            const { stackFrames: [frame] } = await debug.customRequest('stackTrace', { threadId, startFrame: 0, levels: 1 });
            assert.match(frame.name, /complete_review/);
            assert.equal(fileKey(frame.source.path), fileKey(rust.uri.fsPath));
            assert.equal(frame.line, point.location.range.start.line + 1);
        } finally { await debug.customRequest('continue', { threadId }); }
        assert.equal(await click, undefined);
        await waitFor(async () => (await find(app, { name: 'ReviewedCount' }))?.text === 'Reviewed cases: 1' ? true : undefined,
            'the native callback to finish');
    };
    try {
        await vscode.workspace.getConfiguration('ferroui.mcp').update('allowWrites', true, vscode.ConfigurationTarget.Global);
        await vscode.window.showTextDocument(rust);
        const pending = content<RuntimeSummary>(await invoke('ferroui_android_run', { device, profile: 'service-review', folder: folder.toString() }));
        runtime = pending.runtime;
        assert.ok(pending.android?.development && !pending.available);
        content(await invoke('ferroui_runtime_open', { runtime }));
        const first = await accept();
        assert.ok(first.pid && first.generation && first.android?.debuggable);
        assert.ok(await find(first, { text: 'XAML inbox' }));
        vscode.debug.addBreakpoints([point]);
        content(await invoke('ferroui_android_debug', { ...identity(first), debug_configuration: name }));
        const firstDebugger = await tracker.accept();
        await callback(first, firstDebugger);

        await save(rust.uri, '"Service review"', '"Service review updated"');
        const second = await accept(first.generation);
        assert.notEqual(second.pid, first.pid);
        assert.ok((await invoke('ferroui_runtime_query', { ...identity(first), tool: 'list_windows' })).isError);
        const secondDebugger = await tracker.accept(firstDebugger.id);
        assert.ok(!tracker.sessions.has(firstDebugger.id));
        assert.ok(await find(second, { text: 'Service review updated' }));
        await callback(second, secondDebugger);

        await save(xaml, 'XAML inbox', 'XAML inbox updated');
        const third = await accept(second.generation ?? undefined);
        assert.notEqual(third.pid, second.pid);
        const thirdDebugger = await tracker.accept(secondDebugger.id);
        assert.ok(await find(third, { text: 'XAML inbox updated' }));
        await callback(third, thirdDebugger);

        await save(rust.uri, '*count += 1;', 'compile_error!("managed Android source fixture");');
        await waitFor(async () => (await current())?.status === 'Android build failed' ? true : undefined, 'a rejected Rust save', 120_000);
        assert.equal((await current())?.generation, third.generation);
        assert.ok(tracker.sessions.has(thirdDebugger.id));
        assert.match(await output(), /managed Android source fixture/);
        content(await invoke('ferroui_android_debug', { ...identity(third), action: 'detach' }));
        assert.equal((await current())?.pid, third.pid);
        assert.ok(await find(third, { text: 'XAML inbox updated' }));
        content(await invoke('ferroui_android_stop', { runtime }));
        assert.equal(await current(), undefined);
        runtime = undefined;
        console.log('Managed Android native Rust and XAML saves, debugger handoff, failed build retention, and detach passed');
    } finally {
        if (runtime) await invoke('ferroui_android_stop', { runtime }).catch(() => {});
        vscode.debug.removeBreakpoints([point]);
        await tracker.close();
        close();
    }
}
