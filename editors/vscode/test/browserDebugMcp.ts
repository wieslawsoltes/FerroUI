import { strict as assert } from 'node:assert';
import * as vscode from 'vscode';
import type { RuntimeSummary } from '../src/runtimeConnections';
import { fileKey } from '../src/workspacePaths';
import { runtimeApp } from './browserMcp';
import { openEditorMcp } from './mcpFixture';
import { until } from './wait';

export async function verifyBrowserDebug(): Promise<void> {
    const workspace = vscode.workspace.workspaceFolders![0].uri;
    const script = vscode.Uri.joinPath(workspace, 'wwwroot', 'probe.js');
    const failMarker = vscode.Uri.joinPath(workspace, 'fail-build');
    const editor = await vscode.window.showTextDocument(script);
    const point = new vscode.SourceBreakpoint(new vscode.Location(script, new vscode.Position(2, 4)));
    const hits: { session: vscode.DebugSession; thread: number }[] = [];
    const terminated = new Set<string>();
    const tracker = vscode.debug.registerDebugAdapterTrackerFactory('*', {
        createDebugAdapterTracker(session) {
            return { onDidSendMessage(message) {
                if (message.event === 'stopped' && message.body?.reason === 'breakpoint') hits.push({ session, thread: message.body.threadId });
            } };
        },
    });
    const ending = vscode.debug.onDidTerminateDebugSession(session => terminated.add(session.id));
    vscode.debug.addBreakpoints([point]);
    const mcp = await openEditorMcp(38_000);
    const { invoke, content } = mcp;
    const config = vscode.workspace.getConfiguration('ferroui.mcp');
    const previous = config.inspect<boolean>('allowWrites')?.globalValue;
    let runtime: string | undefined;
    const current = async () => content<{ runtimes: RuntimeSummary[] }>(await invoke('ferroui_app_sessions')).runtimes
        .find(session => session.runtime === runtime);
    const accepted = async (previous?: number | null) => {
        let result: RuntimeSummary | undefined;
        await until(async () => {
            result = await current();
            return Boolean(result?.available && result.generation !== previous);
        });
        return result!;
    };
    const breakpoint = async (index: number, expected: RuntimeSummary) => {
        await until(() => Boolean(hits[index]));
        const { session, thread } = hits[index];
        const { stackFrames: [frame] } = await session.customRequest('stackTrace', { threadId: thread });
        assert.equal(frame.line, 3);
        assert.equal(fileKey(frame.source.path), fileKey(script.fsPath));
        const evaluation = await session.customRequest('evaluate', { frameId: frame.id,
            expression: `location.href === ${JSON.stringify(expected.browser?.url)}`, context: 'watch' });
        assert.equal(evaluation.result, 'true');
        return session.parentSession ?? session;
    };
    try {
        await config.update('allowWrites', true, vscode.ConfigurationTarget.Global);
        const queued = content<RuntimeSummary>(await invoke('ferroui_browser_run', { profile: 'debug-dispatch', folder: workspace.toString(), debug: true }));
        runtime = queued.runtime;
        assert.equal(queued.browser?.debuggable, true);
        const initial = await accepted();
        const old = await breakpoint(0, initial);
        assert.equal((await current())?.browser?.paused, true, 'The accepted Chromium page reports its debugger pause.');
        const blocked = await invoke('ferroui_runtime_query', { runtime, expected_generation: initial.generation, tool: 'list_windows' });
        assert.ok(blocked.isError);
        assert.match(blocked.content[0]?.text ?? '', /paused in the debugger/);
        const duplicate = content<RuntimeSummary>(await invoke('ferroui_browser_run', { profile: 'debug-dispatch', folder: workspace.toString(), debug: true }));
        assert.equal(duplicate.generation, initial.generation);
        assert.ok(!terminated.has(old.id));

        await vscode.workspace.fs.writeFile(failMarker, Buffer.from('controlled'));
        content(await invoke('ferroui_browser_rebuild', { runtime }));
        await until(async () => (await current())?.status === 'Browser update failed');
        assert.equal((await current())?.generation, initial.generation);
        assert.ok(!terminated.has(old.id));
        await vscode.workspace.fs.delete(failMarker);

        const line = editor.document.lineAt(2);
        assert.ok(await editor.edit(edit => edit.replace(line.range, line.text.replace('+= 1', '+= 2'))));
        assert.ok(await editor.document.save());
        const replacement = await accepted(initial.generation);
        assert.equal(replacement.runtime, initial.runtime);
        assert.equal(replacement.pid, initial.pid);
        const next = await breakpoint(1, replacement);
        assert.notEqual(next.id, old.id);
        await until(() => terminated.has(old.id));
        assert.equal(next.configuration.port, old.configuration.port);
        assert.match((await invoke('ferroui_browser_debug', { runtime, expected_generation: initial.generation }))
            .content[0]?.text ?? '', /accepted browser generation/);
        const detach = (generation: RuntimeSummary['generation']) => invoke('ferroui_browser_debug', {
            runtime, expected_generation: generation, action: 'detach',
        });
        assert.match((await detach(initial.generation)).content[0]?.text ?? '', /accepted browser generation/);
        assert.ok(!terminated.has(next.id));
        await config.update('allowWrites', false, vscode.ConfigurationTarget.Global);
        try {
            assert.ok((await detach(replacement.generation)).isError);
            assert.ok(!terminated.has(next.id));
        } finally { await config.update('allowWrites', true, vscode.ConfigurationTarget.Global); }

        const detached = content<RuntimeSummary & { debuggerAttached: boolean }>(await detach(replacement.generation));
        assert.equal(detached.debuggerAttached, false);
        await until(() => terminated.has(next.id));
        assert.equal((await current())?.generation, replacement.generation);
        assert.ok(!(await current())?.browser?.paused);
        const live = runtimeApp(mcp, replacement);
        await live.click((await live.find('DispatchOrder')).node_id);
        assert.equal((await live.find('DispatchedCount')).text, 'Dispatched orders: 1');
        content(await invoke('ferroui_browser_rebuild', { runtime }));
        const undebugged = await accepted(replacement.generation);
        assert.equal(undebugged.runtime, replacement.runtime);
        assert.equal(undebugged.pid, replacement.pid);
        await runtimeApp(mcp, undebugged).find('DispatchedCount');
        assert.equal(hits.length, 2, 'The retired debugger does not follow the replacement.');
        content(await invoke('ferroui_browser_debug', { runtime, expected_generation: undebugged.generation }));
        const reattached = await breakpoint(2, undebugged);
        assert.notEqual(reattached.id, next.id);
        await vscode.commands.executeCommand('ferroui.appSessions.detachDebug', runtime);
        await until(() => terminated.has(reattached.id));
        assert.equal((await current())?.generation, undebugged.generation);
        content(await invoke('ferroui_browser_debug', { runtime, expected_generation: undebugged.generation }));
        const finalDebugger = await breakpoint(3, undebugged);
        content(await invoke('ferroui_browser_stop', { runtime }));
        await until(() => terminated.has(finalDebugger.id));
        assert.equal(await current(), undefined);
        await assert.rejects(fetch(`http://127.0.0.1:${finalDebugger.configuration.port}/json/version`, { signal: AbortSignal.timeout(2_000) }));
        runtime = undefined;
        console.log('Managed Chromium verified debugger detach through MCP and App Sessions, generation guards, reload without debugger follow, and scoped cleanup');
    } finally {
        vscode.debug.removeBreakpoints([point]);
        tracker.dispose();
        ending.dispose();
        try { if (runtime) await invoke('ferroui_browser_stop', { runtime }); }
        finally { await config.update('allowWrites', previous, vscode.ConfigurationTarget.Global); mcp.close(); }
    }
}
